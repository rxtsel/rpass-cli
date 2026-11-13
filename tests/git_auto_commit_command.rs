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
fn move_into_git_metadata_is_rejected_without_changing_history() {
    let store = git_store();
    write_file(store.path().join("old.gpg"), "password");
    git(store.path(), ["add", "old.gpg"]);
    git(store.path(), ["commit", "-m", "seed entry"]);
    let before = commit_count(store.path());
    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "old",
            ".git/hidden",
        ])
        .assert()
        .failure();
    assert_eq!(commit_count(store.path()), before);
    assert!(store.path().join("old.gpg").exists());
    assert!(!store.path().join(".git/hidden.gpg").exists());
}

#[test]
fn move_file_alongside_same_named_directory_commits_the_file() {
    let store = git_store();
    write_file(store.path().join("foo.gpg"), "password");
    write_file(store.path().join("foo/child.gpg"), "child");
    git(store.path(), ["add", "foo.gpg", "foo/child.gpg"]);
    git(store.path(), ["commit", "-m", "seed entries"]);
    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "foo",
            "renamed",
        ])
        .assert()
        .success();
    let files = git_output(store.path(), ["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(files.lines().any(|file| file == "renamed.gpg"));
    assert!(files.lines().any(|file| file == "foo/child.gpg"));
    assert!(!files.lines().any(|file| file == "foo.gpg"));
}

#[test]
fn case_only_move_commits_new_name_without_other_staged_files() {
    let store = git_store();
    git(store.path(), ["config", "core.ignorecase", "true"]);
    write_file(store.path().join("old.gpg"), "password");
    git(store.path(), ["add", "old.gpg"]);
    git(store.path(), ["commit", "-m", "seed entry"]);
    write_file(store.path().join("staged-notes.txt"), "unrelated");
    git(store.path(), ["add", "staged-notes.txt"]);
    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "old",
            "OLD",
        ])
        .assert()
        .success();
    let files = git_output(store.path(), ["ls-tree", "--name-only", "HEAD"]);
    assert!(files.lines().any(|file| file == "OLD.gpg"));
    assert!(!files.lines().any(|file| file == "old.gpg"));
    assert!(!files.lines().any(|file| file == "staged-notes.txt"));
    assert_eq!(
        git_output(store.path(), ["status", "--porcelain"]).trim(),
        "A  staged-notes.txt"
    );
}

#[test]
fn case_only_directory_move_commits_nested_entries() {
    let store = git_store();
    git(store.path(), ["config", "core.ignorecase", "true"]);
    write_file(store.path().join("old/entry.gpg"), "password");
    git(store.path(), ["add", "old/entry.gpg"]);
    git(store.path(), ["commit", "-m", "seed entry"]);
    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "old",
            "OLD",
        ])
        .assert()
        .success();
    let files = git_output(store.path(), ["ls-tree", "-r", "--name-only", "HEAD"]);
    assert!(files.lines().any(|file| file == "OLD/entry.gpg"));
    assert!(!files.lines().any(|file| file == "old/entry.gpg"));
}

#[test]
fn case_only_move_works_before_the_first_commit() {
    let store = tempfile::TempDir::new().unwrap();
    git(store.path(), ["init"]);
    git(store.path(), ["config", "user.name", "rpass tests"]);
    git(
        store.path(),
        ["config", "user.email", "rpass-tests@example.invalid"],
    );
    git(store.path(), ["config", "core.ignorecase", "true"]);
    write_file(store.path().join("old.gpg"), "password");
    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "old",
            "OLD",
        ])
        .assert()
        .success();
    assert_eq!(
        git_output(store.path(), ["ls-tree", "--name-only", "HEAD"]).trim(),
        "OLD.gpg"
    );
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

#[cfg(windows)]
#[test]
fn auto_commit_matches_tracked_file_with_different_case() {
    let store = git_store();
    write_file(store.path().join("Foo.gpg"), "original");
    git(store.path(), ["add", "Foo.gpg"]);
    git(store.path(), ["commit", "-m", "seed Foo"]);

    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .write_stdin("updated\n")
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "insert",
            "--force",
            "foo",
        ])
        .assert()
        .success();
    assert_latest_commit_message(store.path(), "Added given password for foo to store.");
    assert!(
        git_output(store.path(), ["show", "--pretty=", "--name-only", "HEAD"]).contains("Foo.gpg")
    );
}

#[test]
fn auto_commit_never_stages_plaintext_or_commits_unrelated_staged_files() {
    let store = git_store();
    write_file(
        store.path().join("bitwarden-export.json"),
        "unencrypted vault",
    );
    write_file(store.path().join("staged-notes.txt"), "unencrypted notes");
    git(store.path(), ["add", "staged-notes.txt"]);
    let gpg = encrypting_gpg_script(store.path());
    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", gpg)
        .write_stdin("secret\n")
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "insert",
            "example/login",
        ])
        .assert()
        .success();
    let committed = git_output(store.path(), ["show", "--pretty=", "--name-only", "HEAD"]);
    assert!(committed.contains("example/login.gpg"));
    assert!(!committed.contains("bitwarden-export.json"));
    assert!(!committed.contains("staged-notes.txt"));
    assert!(git_output(store.path(), ["status", "--porcelain"]).contains("bitwarden-export.json"));
}

#[test]
fn auto_commit_does_not_include_unrelated_encrypted_changes() {
    let store = git_store();
    write_file(store.path().join("other.gpg"), "original");
    git(store.path(), ["add", "other.gpg"]);
    git(store.path(), ["commit", "-m", "seed other"]);
    write_file(store.path().join("other.gpg"), "changed");

    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "init",
            "bob@example.invalid",
        ])
        .assert()
        .success();

    let files = git_output(store.path(), ["show", "--pretty=", "--name-only", "HEAD"]);
    assert!(files.contains(".gpg-id"));
    assert!(!files.contains("other.gpg"));
    assert!(git_output(store.path(), ["status", "--porcelain"]).contains("other.gpg"));
}

#[test]
fn auto_commit_uses_literal_entry_name_with_git_pattern_characters() {
    let store = git_store();
    write_file(store.path().join("item1.gpg"), "original");
    git(store.path(), ["add", "item1.gpg"]);
    git(store.path(), ["commit", "-m", "seed item"]);
    write_file(store.path().join("item1.gpg"), "unrelated change");
    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .write_stdin("secret\n")
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "insert",
            "item[1]",
        ])
        .assert()
        .success();
    let files = git_output(store.path(), ["show", "--pretty=", "--name-only", "HEAD"]);
    assert!(files.contains("item[1].gpg"));
    assert!(!files.lines().any(|line| line == "item1.gpg"));
}

#[test]
fn directory_move_commits_only_its_old_and_new_paths() {
    let store = git_store();
    write_file(store.path().join("old/entry.gpg"), "encrypted");
    write_file(store.path().join("other.gpg"), "original");
    git(store.path(), ["add", "-A"]);
    git(store.path(), ["commit", "-m", "seed entries"]);
    write_file(store.path().join("other.gpg"), "unrelated change");
    rpass_with_git_identity()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "old",
            "new",
        ])
        .assert()
        .success();
    let files = git_output(
        store.path(),
        ["diff", "--name-only", "--no-renames", "HEAD^", "HEAD"],
    );
    assert!(files.contains("old/entry.gpg"));
    assert!(files.contains("new/entry.gpg"));
    assert!(!files.lines().any(|line| line == "other.gpg"));
}

#[test]
fn partial_import_commits_only_entries_written_in_that_run() {
    let store = git_store();
    write_file(store.path().join("other.gpg"), "original");
    git(store.path(), ["add", "other.gpg"]);
    git(store.path(), ["commit", "-m", "seed other"]);
    write_file(store.path().join("other.gpg"), "unrelated change");
    write_file(store.path().join("team/.gpg-id"), "");
    write_file(
        store.path().join("export.json"),
        r#"{"folders":[{"id":"f","name":"team"}],"items":[{"name":"valid","login":{"password":"secret"}},{"name":"lost","folderId":"f"}]}"#,
    );
    rpass_with_git_identity()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            store.path().join("export.json").to_str().unwrap(),
        ])
        .assert()
        .failure();
    let files = git_output(store.path(), ["show", "--pretty=", "--name-only", "HEAD"]);
    assert!(files.contains("valid.gpg"));
    assert!(!files.lines().any(|line| line == "other.gpg"));
    assert!(!files.contains("export.json"));
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
