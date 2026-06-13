mod support;

use std::fs;
use std::path::Path;

use support::{editing_gpg_script, editing_script, encrypting_gpg_script, rpass};

#[test]
fn insert_auto_commits_when_store_is_git_repository() {
    let store = git_store();
    let gpg = encrypting_gpg_script(store.path());

    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", gpg)
        .write_stdin("secret\n")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "insert",
            "--echo",
            "example/login",
        ])
        .assert()
        .success();

    assert_latest_commit_message(
        store.path(),
        "Added given password for example/login to store.",
    );
}

#[test]
fn generate_auto_commits_when_password_is_saved() {
    let store = git_store();
    let gpg = encrypting_gpg_script(store.path());

    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/generated",
            "--length",
            "18",
        ])
        .assert()
        .success();

    assert_latest_commit_message(
        store.path(),
        "Added generated password for example/generated to store.",
    );
}

#[test]
fn generate_dry_run_does_not_auto_commit() {
    let store = git_store();
    let before = commit_count(store.path());

    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "--dry-run",
            "--length",
            "18",
        ])
        .assert()
        .success();

    assert_eq!(commit_count(store.path()), before);
}

#[test]
fn edit_auto_commits_only_when_content_changes() {
    let store = git_store();
    write_file(store.path().join("email/work.gpg"), "encrypted\n");
    git(store.path(), ["add", "-A"]);
    git(store.path(), ["commit", "-m", "seed entry"]);
    let gpg = editing_gpg_script(store.path(), "old\nusername: alice\n");
    let editor = editing_script(store.path(), "new\nusername: bob\n");

    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("EDITOR", editor)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "email/work",
        ])
        .assert()
        .success();

    assert_latest_commit_message(store.path(), "Edited password for email/work.");
}

#[test]
fn unchanged_edit_does_not_auto_commit() {
    let store = git_store();
    write_file(store.path().join("email/work.gpg"), "encrypted\n");
    git(store.path(), ["add", "-A"]);
    git(store.path(), ["commit", "-m", "seed entry"]);
    let before = commit_count(store.path());
    let gpg = editing_gpg_script(store.path(), "old\nusername: alice\n");
    let editor = editing_script(store.path(), "old\nusername: alice\n");

    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("EDITOR", editor)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "email/work",
        ])
        .assert()
        .success();

    assert_eq!(commit_count(store.path()), before);
}

#[test]
fn rm_auto_commits_when_store_is_git_repository() {
    let store = git_store();
    write_file(store.path().join("example/login.gpg"), "encrypted\n");
    git(store.path(), ["add", "-A"]);
    git(store.path(), ["commit", "-m", "seed entry"]);

    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "rm",
            "--force",
            "example/login",
        ])
        .assert()
        .success();

    assert_latest_commit_message(store.path(), "Removed example/login from store.");
}

#[test]
fn mv_auto_commits_when_store_is_git_repository() {
    let store = git_store();
    write_file(store.path().join("old.gpg"), "encrypted\n");
    git(store.path(), ["add", "-A"]);
    git(store.path(), ["commit", "-m", "seed entry"]);

    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "old",
            "new",
        ])
        .assert()
        .success();

    assert_latest_commit_message(store.path(), "Renamed old to new.");
}

#[test]
fn write_commands_do_not_require_git_when_store_is_not_repository() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    let gpg = encrypting_gpg_script(store.path());

    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", gpg)
        .write_stdin("secret\n")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "insert",
            "--echo",
            "example/login",
        ])
        .assert()
        .success();
}

fn git_store() -> tempfile::TempDir {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    git(store.path(), ["init"]);
    git(store.path(), ["config", "user.name", "rpass tests"]);
    git(
        store.path(),
        ["config", "user.email", "rpass-tests@example.invalid"],
    );
    git(store.path(), ["add", "-A"]);
    git(store.path(), ["commit", "-m", "initial store"]);
    store
}

fn rpass_with_git_identity() -> assert_cmd::Command {
    let mut command = rpass();
    command
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid");
    command
}

fn assert_latest_commit_message(store: &Path, expected: &str) {
    assert_eq!(latest_commit_message(store), expected);
}

fn latest_commit_message(store: &Path) -> String {
    git_output(store, ["log", "-1", "--pretty=%s"])
        .trim_end_matches(['\r', '\n'])
        .to_owned()
}

fn commit_count(store: &Path) -> usize {
    git_output(store, ["rev-list", "--count", "HEAD"])
        .trim()
        .parse()
        .expect("commit count")
}

fn git<const N: usize>(path: &Path, args: [&str; N]) {
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .status()
        .expect("git command");
    assert!(status.success());
}

fn git_output<const N: usize>(path: &Path, args: [&str; N]) -> String {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(path)
        .args(args)
        .output()
        .expect("git command");
    assert!(output.status.success());
    String::from_utf8(output.stdout).expect("git stdout")
}

fn write_file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("file");
}
