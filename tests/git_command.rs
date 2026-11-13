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
fn git_init_honors_initial_branch_option() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "alice\n");
    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "git",
            "init",
            "-b",
            "named-branch",
        ])
        .assert()
        .success();
    let branch = std::process::Command::new("git")
        .arg("-C")
        .arg(store.path())
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    assert!(branch.status.success());
    assert_eq!(
        String::from_utf8(branch.stdout).unwrap().trim(),
        "named-branch"
    );
}

#[test]
fn git_init_rejects_bare_repositories_before_initializing() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "alice\n");
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "git",
            "init",
            "--bare",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("bare Git repositories"));
    assert!(!store.path().join("HEAD").exists());
    assert!(!store.path().join(".git").exists());
}

#[test]
fn git_init_rejects_a_different_target_directory() {
    let store = tempfile::TempDir::new().unwrap();
    let other = store.path().join("other");
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "git",
            "init",
            other.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unsupported git init argument"));
    assert!(!other.exists());
    assert!(!store.path().join(".git").exists());
}

#[test]
fn git_init_does_not_commit_plaintext_export() {
    let store = tempfile::TempDir::new().expect("store");
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(store.path().join("export.json"), "unencrypted vault");
    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args(["--store-dir", store.path().to_str().unwrap(), "git", "init"])
        .assert()
        .success();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(store.path())
        .args(["show", "--pretty=", "--name-only", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    let files = String::from_utf8(output.stdout).unwrap();
    assert!(files.contains(".gpg-id"));
    assert!(!files.contains("export.json"));
}

#[test]
fn git_init_excludes_abandoned_move_backups() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(
        store.path().join(".rpass-backup-example/previous/old.gpg"),
        "old ciphertext",
    );
    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args(["--store-dir", store.path().to_str().unwrap(), "git", "init"])
        .assert()
        .success();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(store.path())
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), ".gpg-id");
}

#[test]
fn git_init_handles_more_paths_than_a_windows_command_line() {
    let store = tempfile::TempDir::new().expect("store");
    write_file(store.path().join(".gpg-id"), "alice\n");
    for index in 0..400 {
        write_file(
            store
                .path()
                .join(format!("{index:04}-{}.gpg", "x".repeat(90))),
            "ciphertext",
        );
    }
    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args(["--store-dir", store.path().to_str().unwrap(), "git", "init"])
        .assert()
        .success();
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(store.path())
        .args(["ls-tree", "-r", "--name-only", "HEAD"])
        .output()
        .expect("git ls-tree");
    assert!(output.status.success());
    assert_eq!(output.stdout.split(|byte| *byte == b'\n').count() - 1, 401);
}

#[cfg(unix)]
#[test]
fn git_init_preserves_non_utf8_filenames() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(
        store.path().join(OsStr::from_bytes(b"odd-\xff.gpg")),
        "ciphertext",
    );
    rpass()
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args(["--store-dir", store.path().to_str().unwrap(), "git", "init"])
        .assert()
        .success();
    let tree = std::process::Command::new("git")
        .arg("-C")
        .arg(store.path())
        .args(["ls-tree", "-r", "-z", "--name-only", "HEAD"])
        .output()
        .unwrap();
    assert!(tree.status.success());
    assert!(
        tree.stdout
            .windows(b"odd-\xff.gpg".len())
            .any(|bytes| bytes == b"odd-\xff.gpg")
    );
}

#[test]
fn git_init_succeeds_for_empty_store_without_initial_commit() {
    let store = tempfile::TempDir::new().expect("temp dir");

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
        .success();

    assert!(store.path().join(".git").is_dir());

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
        .stdout("");
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
