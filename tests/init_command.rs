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
fn init_rejects_recipient_with_line_break_without_changing_recipients() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "original\n").unwrap();
    for recipient in ["alice\nbob", "alice\rbob", "# ignored", " alice"] {
        rpass()
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "init",
                recipient,
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("invalid recipient"));
        assert_eq!(
            fs::read(store.path().join(".gpg-id")).unwrap(),
            b"original\n"
        );
    }
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
fn init_rejects_hidden_or_multiline_recipient_path() {
    let store = tempfile::TempDir::new().unwrap();
    for path in [".git", "foo\nbar"] {
        rpass()
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "init",
                "--path",
                path,
                "alice",
            ])
            .assert()
            .failure();
        assert!(!store.path().join(path).exists());
    }
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
fn init_removing_subfolder_reencrypts_with_parent() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "root@example.invalid\n");
    write_file(store.path().join("team/.gpg-id"), "team@example.invalid\n");
    write_file(store.path().join("team/entry.gpg"), "secret\n");
    let (gpg, log) = reencrypting_gpg_script(store.path());
    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "init",
            "--path",
            "team",
            "",
        ])
        .assert()
        .success();
    assert!(!store.path().join("team/.gpg-id").exists());
    assert!(
        fs::read_to_string(log)
            .unwrap()
            .contains("recipient:root@example.invalid")
    );
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

#[cfg(unix)]
#[test]
fn init_rejects_symlinked_recipient_file() {
    let store = tempfile::TempDir::new().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    fs::write(outside.path(), "original\n").unwrap();
    std::os::unix::fs::symlink(outside.path(), store.path().join(".gpg-id")).unwrap();
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "init",
            "alice",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("outside the password store"));
    assert_eq!(fs::read_to_string(outside.path()).unwrap(), "original\n");
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

#[cfg(unix)]
#[test]
fn failed_second_reencryption_restores_recipients_and_ciphertexts() {
    use std::os::unix::fs::PermissionsExt;

    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "old@example.invalid\n");
    write_file(store.path().join("first.gpg"), "first ciphertext\n");
    write_file(store.path().join("second.gpg"), "second ciphertext\n");
    let script = store.path().join("gpg-fail-second");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nmode=encrypt\noutput=\ninput=\nwhile [ $# -gt 0 ]; do\n  case \"$1\" in\n    --decrypt) mode=decrypt; shift;;\n    --output) output=$2; shift 2;;\n    --recipient) shift 2;;\n    -*) shift;;\n    *) input=$1; shift;;\n  esac\ndone\nif [ \"$mode\" = decrypt ]; then cat \"$input\"; exit; fi\ncount=$(cat '{counter}' 2>/dev/null || echo 0)\nif [ \"$count\" = 1 ]; then exit 1; fi\necho 1 > '{counter}'\ncat > \"$output\"\n",
            counter = store.path().join("count").display()
        ),
    )
    .unwrap();
    let mut permissions = fs::metadata(&script).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&script, permissions).unwrap();

    rpass()
        .env("PASSWORD_STORE_GPG", script)
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "init",
            "new@example.invalid",
        ])
        .assert()
        .failure();
    assert_eq!(
        fs::read_to_string(store.path().join("count")).unwrap(),
        "1\n"
    );
    assert_eq!(
        fs::read_to_string(store.path().join(".gpg-id")).unwrap(),
        "old@example.invalid\n"
    );
    assert_eq!(
        fs::read_to_string(store.path().join("first.gpg")).unwrap(),
        "first ciphertext\n"
    );
    assert_eq!(
        fs::read_to_string(store.path().join("second.gpg")).unwrap(),
        "second ciphertext\n"
    );
}

#[test]
fn init_commits_reencrypted_entries_with_recipient_changes() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "old@example.invalid\n");
    write_file(store.path().join("entry.gpg"), "old ciphertext\n");
    git(store.path(), ["init"]);
    git(store.path(), ["config", "user.name", "rpass tests"]);
    git(
        store.path(),
        ["config", "user.email", "rpass-tests@example.invalid"],
    );
    git(store.path(), ["add", ".gpg-id", "entry.gpg"]);
    git(store.path(), ["commit", "-m", "initial store"]);
    write_file(store.path().join("entry.gpg"), "new ciphertext\n");
    let (gpg, _) = reencrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("GIT_AUTHOR_NAME", "rpass tests")
        .env("GIT_AUTHOR_EMAIL", "rpass-tests@example.invalid")
        .env("GIT_COMMITTER_NAME", "rpass tests")
        .env("GIT_COMMITTER_EMAIL", "rpass-tests@example.invalid")
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "init",
            "new@example.invalid",
        ])
        .assert()
        .success();

    let committed = git_output(store.path(), ["show", "--pretty=", "--name-only", "HEAD"]);
    assert!(committed.lines().any(|path| path == ".gpg-id"));
    assert!(committed.lines().any(|path| path == "entry.gpg"));
}

#[test]
fn init_skips_git_and_recovery_directories_when_reencrypting() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "old@example.invalid\n");
    write_file(store.path().join("entry.gpg"), "secret\n");
    git(store.path(), ["init"]);
    git(store.path(), ["config", "user.name", "rpass tests"]);
    git(
        store.path(),
        ["config", "user.email", "rpass-tests@example.invalid"],
    );
    write_file(store.path().join(".git/ignored.gpg"), "metadata\n");
    write_file(
        store.path().join(".rpass-backup-old/ignored.gpg"),
        "backup\n",
    );
    let (gpg, log_file) = reencrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "init",
            "new@example.invalid",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(store.path().join(".git/ignored.gpg")).unwrap(),
        "metadata\n"
    );
    assert_eq!(
        fs::read_to_string(store.path().join(".rpass-backup-old/ignored.gpg")).unwrap(),
        "backup\n"
    );
    assert_eq!(
        fs::read_to_string(log_file)
            .unwrap()
            .lines()
            .filter(|line| *line == "encrypt")
            .count(),
        1
    );
}

#[cfg(unix)]
#[test]
fn init_does_not_reencrypt_entries_through_symlinks() {
    let store = tempfile::TempDir::new().unwrap();
    let outside = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "old@example.invalid\n");
    write_file(outside.path().join("entry.gpg"), "outside ciphertext\n");
    std::os::unix::fs::symlink(outside.path(), store.path().join("linked")).unwrap();
    let (gpg, log_file) = reencrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "init",
            "new@example.invalid",
        ])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(outside.path().join("entry.gpg")).unwrap(),
        "outside ciphertext\n"
    );
    assert!(fs::read_to_string(log_file).unwrap_or_default().is_empty());
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
