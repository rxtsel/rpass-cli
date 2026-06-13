mod support;

use std::fs;
use std::path::Path;

use predicates::prelude::*;
use serde_json::Value;

use support::rpass;

#[test]
fn init_creates_missing_store_and_writes_gpg_id() {
    let parent = tempfile::TempDir::new().expect("temp dir");
    let store = parent.path().join(".password-store");

    rpass()
        .args([
            "--store-dir",
            store.to_str().expect("store path"),
            "init",
            "alice@example.invalid",
        ])
        .assert()
        .success()
        .stdout("Password store initialized for alice@example.invalid\n")
        .stderr("");

    assert_eq!(
        fs::read_to_string(store.join(".gpg-id")).expect("gpg id"),
        "alice@example.invalid\n"
    );
}

#[test]
fn init_accepts_multiple_gpg_ids() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "alice@example.invalid",
            "bob@example.invalid",
        ])
        .assert()
        .success()
        .stdout("Password store initialized for alice@example.invalid, bob@example.invalid\n")
        .stderr("");

    assert_eq!(
        fs::read_to_string(store.path().join(".gpg-id")).expect("gpg id"),
        "alice@example.invalid\nbob@example.invalid\n"
    );
}

#[test]
fn init_path_writes_gpg_id_for_subfolder() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "--path",
            "team/work",
            "team@example.invalid",
        ])
        .assert()
        .success()
        .stdout("Password store initialized for team@example.invalid (team/work)\n")
        .stderr("");

    assert_eq!(
        fs::read_to_string(store.path().join("team/work/.gpg-id")).expect("gpg id"),
        "team@example.invalid\n"
    );
}

#[test]
fn init_short_path_flag_writes_gpg_id_for_subfolder() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "-p",
            "team",
            "team@example.invalid",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(store.path().join("team/.gpg-id")).expect("gpg id"),
        "team@example.invalid\n"
    );
}

#[test]
fn init_with_empty_gpg_id_removes_existing_gpg_id_for_path() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join("team/.gpg-id"), "team@example.invalid\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "--path",
            "team",
            "",
        ])
        .assert()
        .success()
        .stdout("Password store recipients removed (team)\n")
        .stderr("");

    assert!(!store.path().join("team/.gpg-id").exists());
}

#[test]
fn init_reports_success_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");

    let assert = rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "--json",
            "alice@example.invalid",
            "bob@example.invalid",
        ])
        .assert()
        .success()
        .stderr("");
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");

    assert_eq!(output["path"], ".gpg-id");
    assert_eq!(output["recipients"][0], "alice@example.invalid");
    assert_eq!(output["recipients"][1], "bob@example.invalid");
    assert_eq!(output["removed"], false);
}

#[test]
fn init_rejects_path_traversal_subfolder_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "--json",
            "--path",
            "../outside",
            "alice@example.invalid",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"invalid_init_path\""));
}

#[test]
fn init_auto_commits_when_store_is_git_repository() {
    let store = tempfile::TempDir::new().expect("temp dir");
    git(store.path(), ["init"]);
    git(store.path(), ["config", "user.name", "rpass tests"]);
    git(
        store.path(),
        ["config", "user.email", "rpass-tests@example.invalid"],
    );

    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "alice@example.invalid",
        ])
        .assert()
        .success();

    assert_eq!(
        git_output(store.path(), ["log", "-1", "--pretty=%s"]).trim_end_matches(['\r', '\n']),
        "Set GPG id to alice@example.invalid."
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
