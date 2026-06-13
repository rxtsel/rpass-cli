mod support;

use std::fs;
use std::path::Path;

use predicates::prelude::*;
use serde_json::Value;

use support::{missing_executable_path, rpass};

#[test]
fn git_init_initializes_repository_and_commits_current_store() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");

    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "git",
            "init",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Added current contents of password store.",
        ));

    assert!(store.path().join(".git").is_dir());

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "git",
            "log",
            "--oneline",
            "--",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Added current contents of password store.",
        ));
}

#[test]
fn git_status_passes_arguments_to_git_in_store() {
    let store = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(store.path());
    write_file(store.path().join("example/login.gpg"), "encrypted\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "git",
            "status",
            "--short",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("?? example/"));
}

#[test]
fn git_json_wraps_stdout_stderr_and_exit_code() {
    let store = tempfile::TempDir::new().expect("temp dir");
    init_git_repo(store.path());
    write_file(store.path().join("example/login.gpg"), "encrypted\n");

    let assert = rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "git",
            "--json",
            "status",
            "--short",
        ])
        .assert()
        .success()
        .stderr("");
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");

    assert_eq!(output["exit_code"], 0);
    assert_eq!(output["stderr"], "");
    assert!(
        output["stdout"]
            .as_str()
            .expect("stdout")
            .contains("?? example/")
    );
}

#[test]
fn missing_git_returns_json_error() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .env("PASSWORD_STORE_GIT", missing_executable_path(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "git",
            "--json",
            "status",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"git_not_found\""));
}

#[test]
fn git_status_in_non_repository_returns_structured_json_error() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "git",
            "--json",
            "status",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "\"code\": \"git_repository_not_found\"",
        ));
}

fn init_git_repo(path: &Path) {
    git(path, ["init"]);
    git(path, ["config", "user.name", "rpass tests"]);
    git(
        path,
        ["config", "user.email", "rpass-tests@example.invalid"],
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

fn write_file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("file");
}
