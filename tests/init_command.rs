mod support;

use std::fs;
use std::path::Path;

use predicates::prelude::*;
use serde_json::Value;

use support::{reencrypting_gpg_script, rpass};

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

#[test]
fn init_re_encrypts_existing_entries_with_new_recipients() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "old@example.invalid\n");
    write_file(store.path().join("entry.gpg"), "secret\n");
    write_file(store.path().join("subdir/nested.gpg"), "nested-secret\n");

    let (gpg, log_file) = reencrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", &gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "new@example.invalid",
        ])
        .assert()
        .success()
        .stdout("Password store initialized for new@example.invalid\n")
        .stderr("");

    let log = fs::read_to_string(&log_file).expect("log file");
    assert!(
        log.contains("recipient:new@example.invalid"),
        "expected new recipient in log, got: {log}"
    );
    let encrypt_count = log.lines().filter(|l| *l == "encrypt").count();
    assert_eq!(
        encrypt_count, 2,
        "expected 2 entries re-encrypted, got: {encrypt_count}"
    );

    assert_eq!(
        fs::read_to_string(store.path().join("entry.gpg")).expect("entry"),
        "secret\n",
        "content should be preserved after re-encryption"
    );
    assert_eq!(
        fs::read_to_string(store.path().join("subdir/nested.gpg")).expect("nested entry"),
        "nested-secret\n",
        "nested content should be preserved after re-encryption"
    );
}

#[test]
fn init_skips_entries_in_subdirectory_with_own_gpg_id() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "old@example.invalid\n");
    write_file(store.path().join("team/.gpg-id"), "team@example.invalid\n");
    write_file(store.path().join("entry.gpg"), "root-secret\n");
    write_file(store.path().join("team/entry.gpg"), "team-secret\n");

    let (gpg, log_file) = reencrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", &gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "new@example.invalid",
        ])
        .assert()
        .success();

    let log = fs::read_to_string(&log_file).expect("log file");
    let encrypt_count = log.lines().filter(|l| *l == "encrypt").count();
    assert_eq!(
        encrypt_count, 1,
        "only the root entry should be re-encrypted, not the team entry with its own .gpg-id; log: {log}"
    );
}

#[test]
fn init_re_encrypts_only_entries_in_target_subfolder() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "root@example.invalid\n");
    write_file(
        store.path().join("team/.gpg-id"),
        "old-team@example.invalid\n",
    );
    write_file(store.path().join("root-entry.gpg"), "root-secret\n");
    write_file(store.path().join("team/entry.gpg"), "team-secret\n");

    let (gpg, log_file) = reencrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", &gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "--path",
            "team",
            "new-team@example.invalid",
        ])
        .assert()
        .success();

    let log = fs::read_to_string(&log_file).expect("log file");
    let encrypt_count = log.lines().filter(|l| *l == "encrypt").count();
    assert_eq!(
        encrypt_count, 1,
        "only team/entry.gpg should be re-encrypted; log: {log}"
    );
    assert!(
        log.contains("recipient:new-team@example.invalid"),
        "new team recipient should be used; log: {log}"
    );
}

#[test]
fn init_skips_re_encryption_when_store_has_no_entries() {
    let store = tempfile::TempDir::new().expect("temp dir");

    let (gpg, log_file) = reencrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", &gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "init",
            "alice@example.invalid",
        ])
        .assert()
        .success()
        .stdout("Password store initialized for alice@example.invalid\n");

    let log = fs::read_to_string(&log_file).unwrap_or_default();
    assert!(
        log.is_empty(),
        "no re-encryption should happen with empty store; log: {log}"
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
