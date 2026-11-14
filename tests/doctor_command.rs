use std::fs;

use predicates::prelude::*;
use tempfile::TempDir;

mod support;

use support::{empty_success_gpg_script, rpass, successful_gpg_script};

#[test]
fn reports_ready_environment_as_text() {
    let store = password_store_with_gpg_id();
    let gpg = successful_gpg_script(store.path(), "gpg (GnuPG) test\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "doctor",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("[ok] store_directory"))
        .stdout(predicate::str::contains("[ok] gpg_id"))
        .stdout(predicate::str::contains("[ok] gpg"))
        .stdout(predicate::str::contains("rpass is ready"));
}

#[test]
fn reports_ready_environment_as_json() {
    let store = password_store_with_gpg_id();
    let gpg = successful_gpg_script(store.path(), "gpg (GnuPG) test\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "doctor",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"ok\": true"))
        .stdout(predicate::str::contains("\"name\": \"store_directory\""))
        .stdout(predicate::str::contains("\"name\": \"gpg\""));
}

#[test]
fn rejects_empty_or_comment_only_recipient_file() {
    let store = TempDir::new().unwrap();
    let gpg = successful_gpg_script(store.path(), "gpg (GnuPG) test\n");
    for contents in ["\n", "# no recipients\n"] {
        fs::write(store.path().join(".gpg-id"), contents).unwrap();
        rpass()
            .env("PASSWORD_STORE_GPG", &gpg)
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "doctor",
                "--json",
            ])
            .assert()
            .failure()
            .stdout(predicate::str::contains("\"ok\": false"))
            .stdout(predicate::str::contains("\"name\": \"gpg_id\""));
    }
}

#[test]
fn rejects_gpg_command_with_empty_version_output() {
    let store = password_store_with_gpg_id();
    rpass()
        .env("PASSWORD_STORE_GPG", empty_success_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "doctor",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"ok\": false"))
        .stdout(predicate::str::contains("\"name\": \"gpg\""))
        .stderr(predicate::str::contains("doctor_checks_failed"));
}

#[test]
fn exits_with_failure_when_gpg_id_is_missing() {
    let store = TempDir::new().expect("temp dir");
    let gpg = successful_gpg_script(store.path(), "gpg (GnuPG) test\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "doctor",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("[fail] gpg_id"))
        .stderr(predicate::str::contains("doctor checks failed"));
}

fn password_store_with_gpg_id() -> TempDir {
    let store = TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "KEY").expect("gpg id");
    store
}
