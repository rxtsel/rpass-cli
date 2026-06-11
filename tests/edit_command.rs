mod support;

use std::fs;

use predicates::prelude::*;

use support::{editing_gpg_script, editing_script, failing_editing_gpg_script, rpass};

#[test]
fn edits_existing_entry_with_editor_and_reencrypts() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.com\n").expect("gpg id");
    fs::create_dir_all(store.path().join("email")).expect("entry dir");
    fs::write(store.path().join("email/work.gpg"), "encrypted\n").expect("entry");
    let gpg = editing_gpg_script(store.path(), "old\nusername: alice\n");
    let editor = editing_script(store.path(), "new\nusername: bob\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("EDITOR", editor)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "email/work",
        ])
        .assert()
        .success()
        .stdout("Entry 'email/work' updated\n")
        .stderr("");

    let encrypted = fs::read_to_string(store.path().join("email/work.gpg")).expect("entry");
    assert_eq!(encrypted, "new\nusername: bob\n");
}

#[test]
fn creates_missing_entry_with_editor() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.com\n").expect("gpg id");
    let gpg = editing_gpg_script(store.path(), "");
    let editor = editing_script(store.path(), "secret\nurl: https://example.com\n");

    rpass()
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

    let encrypted = fs::read_to_string(store.path().join("email/work.gpg")).expect("entry");
    assert_eq!(encrypted, "secret\nurl: https://example.com\n");
}

#[test]
fn leaves_existing_entry_unchanged_when_editor_makes_no_changes() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.com\n").expect("gpg id");
    fs::create_dir_all(store.path().join("email")).expect("entry dir");
    fs::write(store.path().join("email/work.gpg"), "original encrypted\n").expect("entry");
    let gpg = editing_gpg_script(store.path(), "old\nusername: alice\n");
    let editor = editing_script(store.path(), "old\nusername: alice\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("EDITOR", editor)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "email/work",
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let encrypted = fs::read_to_string(store.path().join("email/work.gpg")).expect("entry");
    assert_eq!(encrypted, "original encrypted\n");
}

#[test]
fn unchanged_edit_with_json_emits_no_success_body() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.com\n").expect("gpg id");
    fs::create_dir_all(store.path().join("email")).expect("entry dir");
    fs::write(store.path().join("email/work.gpg"), "original encrypted\n").expect("entry");
    let gpg = editing_gpg_script(store.path(), "old\nusername: alice\n");
    let editor = editing_script(store.path(), "old\nusername: alice\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("EDITOR", editor)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "email/work",
            "--json",
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");
}

#[test]
fn failed_reencrypt_preserves_existing_entry() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.com\n").expect("gpg id");
    fs::create_dir_all(store.path().join("email")).expect("entry dir");
    fs::write(store.path().join("email/work.gpg"), "original encrypted\n").expect("entry");
    let gpg = failing_editing_gpg_script(store.path(), "old\nusername: alice\n");
    let editor = editing_script(store.path(), "new\nusername: bob\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("EDITOR", editor)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "email/work",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("gpg failed to encrypt entry"));

    let encrypted = fs::read_to_string(store.path().join("email/work.gpg")).expect("entry");
    assert_eq!(encrypted, "original encrypted\n");
}

#[test]
fn reports_editor_failure_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.com\n").expect("gpg id");

    rpass()
        .env("EDITOR", "missing-editor")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "email/work",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"editor_failed\""));
}
