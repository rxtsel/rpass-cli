mod support;

use std::fs;
use std::path::Path;

use predicates::prelude::*;
use serde_json::Value;

use support::rpass;

#[test]
fn lists_root_recipients() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(
        store.path().join(".gpg-id"),
        "alice@example.invalid\n# comment\n\nbob@example.invalid\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
        ])
        .assert()
        .success()
        .stdout("alice@example.invalid\nbob@example.invalid\n")
        .stderr("");
}

#[test]
fn lists_recipients_for_subfolder() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join("team/.gpg-id"), "team@example.invalid\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "--path",
            "team",
        ])
        .assert()
        .success()
        .stdout("team@example.invalid\n")
        .stderr("");
}

#[test]
fn lists_recipients_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");

    let assert = rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "--json",
        ])
        .assert()
        .success()
        .stderr("");
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");

    assert_eq!(output["path"], ".gpg-id");
    assert_eq!(output["recipients"][0], "alice@example.invalid");
}

#[test]
fn adds_recipient_without_duplicates() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "add",
            "bob@example.invalid",
        ])
        .assert()
        .success()
        .stdout("Recipient 'bob@example.invalid' added\n")
        .stderr("");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "add",
            "bob@example.invalid",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(store.path().join(".gpg-id")).expect("gpg id"),
        "alice@example.invalid\nbob@example.invalid\n"
    );
}

#[test]
fn adds_recipient_to_subfolder() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join("team/.gpg-id"), "alice@example.invalid\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "--path",
            "team",
            "add",
            "bob@example.invalid",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(store.path().join("team/.gpg-id")).expect("gpg id"),
        "alice@example.invalid\nbob@example.invalid\n"
    );
}

#[test]
fn removes_recipient() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(
        store.path().join(".gpg-id"),
        "alice@example.invalid\nbob@example.invalid\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "remove",
            "bob@example.invalid",
        ])
        .assert()
        .success()
        .stdout("Recipient 'bob@example.invalid' removed\n")
        .stderr("");

    assert_eq!(
        fs::read_to_string(store.path().join(".gpg-id")).expect("gpg id"),
        "alice@example.invalid\n"
    );
}

#[test]
fn remove_missing_recipient_returns_json_error() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "--json",
            "remove",
            "bob@example.invalid",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "\"code\": \"recipient_not_found\"",
        ));
}

#[test]
fn missing_gpg_id_returns_json_error() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"gpg_id_not_found\""));
}

#[test]
fn rejects_invalid_path_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "--json",
            "--path",
            "../outside",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"invalid_init_path\""));
}

#[test]
fn add_auto_commits_when_store_is_git_repository() {
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

    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "recipients",
            "add",
            "bob@example.invalid",
        ])
        .assert()
        .success();

    assert_eq!(
        git_output(store.path(), ["log", "-1", "--pretty=%s"]).trim_end_matches(['\r', '\n']),
        "Added GPG id bob@example.invalid."
    );
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
