use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn shows_decrypted_entry_as_text() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = successful_gpg_script(store.path(), "secret\nusername: alice\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "show",
            "email/work",
        ])
        .assert()
        .success()
        .stdout("secret\nusername: alice\n");
}

#[test]
fn shows_decrypted_entry_as_json() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = successful_gpg_script(
        store.path(),
        "secret\nusername: alice\nurl: https://example.com\notpauth://totp/example\nnote\n",
    );

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "show",
            "email/work",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"email/work\""))
        .stdout(predicate::str::contains("\"password\": \"secret\""))
        .stdout(predicate::str::contains("\"name\": \"username\""))
        .stdout(predicate::str::contains("\"value\": \"alice\""))
        .stdout(predicate::str::contains(
            "\"otp_uri\": \"otpauth://totp/example\"",
        ))
        .stdout(predicate::str::contains("\"note\""));
}

#[test]
fn reports_missing_entry_without_running_gpg() {
    let store = TempDir::new().expect("temp dir");

    rpass()
        .env("PASSWORD_STORE_GPG", "missing-gpg")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "show",
            "missing",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("entry does not exist: missing"));
}

#[test]
fn reports_invalid_entry_name() {
    let store = TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "show",
            "../outside",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid entry name '../outside'"));
}

#[test]
fn reports_missing_gpg_executable() {
    let store = password_store_with_entry("email/work.gpg");

    rpass()
        .env("PASSWORD_STORE_GPG", missing_executable_path(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "show",
            "email/work",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("gpg executable was not found"));
}

#[test]
fn reports_gpg_decrypt_failure() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = failing_gpg_script(store.path(), "gpg: decryption failed");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "show",
            "email/work",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "gpg failed to decrypt entry: gpg: decryption failed",
        ));
}

fn rpass() -> Command {
    Command::cargo_bin("rpass").expect("rpass binary")
}

fn password_store_with_entry(entry: &str) -> TempDir {
    let store = TempDir::new().expect("temp dir");
    create_file(store.path().join(entry));
    store
}

fn create_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, "").expect("file");
}

fn missing_executable_path(directory: &Path) -> PathBuf {
    directory.join("missing-gpg")
}

#[cfg(windows)]
fn successful_gpg_script(directory: &Path, output: &str) -> PathBuf {
    let script = directory.join("gpg.cmd");
    let output_file = directory.join("gpg-output.txt");

    fs::write(&output_file, output).expect("output file");
    fs::write(
        &script,
        format!("@echo off\r\ntype \"{}\"\r\n", output_file.display()),
    )
    .expect("script");
    script
}

#[cfg(not(windows))]
fn successful_gpg_script(directory: &Path, output: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = directory.join("gpg");

    fs::write(&script, format!("#!/bin/sh\nprintf '{}'\n", output)).expect("script");
    make_executable(&script);
    script
}

#[cfg(windows)]
fn failing_gpg_script(directory: &Path, message: &str) -> PathBuf {
    let script = directory.join("gpg-fail.cmd");

    fs::write(
        &script,
        format!("@echo off\r\necho {message} 1>&2\r\nexit /b 2\r\n"),
    )
    .expect("script");
    script
}

#[cfg(not(windows))]
fn failing_gpg_script(directory: &Path, message: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;

    let script = directory.join("gpg-fail");

    fs::write(
        &script,
        format!("#!/bin/sh\nprintf '{}' >&2\nexit 2\n", message),
    )
    .expect("script");
    make_executable(&script);
    script
}

#[cfg(not(windows))]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("permissions");
}
