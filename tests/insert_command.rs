mod support;

use std::fs;

use predicates::prelude::*;

use support::{encrypting_gpg_script, rpass};

#[test]
fn inserts_entry_by_encrypting_stdin_with_store_recipients() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.com\n").expect("gpg id");
    let gpg = encrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .write_stdin("secret\nusername: alice\n")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "insert",
            "email/work",
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let encrypted = fs::read_to_string(store.path().join("email/work.gpg")).expect("entry");
    assert_eq!(encrypted, "secret\nusername: alice\n");

    let recipients =
        fs::read_to_string(store.path().join("gpg-recipients.txt")).expect("recipients");
    assert_eq!(recipients, "alice@example.com\n");
}

#[test]
fn inserts_entry_with_nearest_directory_recipients() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "root@example.com\n").expect("root gpg id");
    fs::create_dir_all(store.path().join("teams")).expect("teams dir");
    fs::write(store.path().join("teams/.gpg-id"), "team@example.com\n").expect("team gpg id");
    let gpg = encrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .write_stdin("secret\n")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "insert",
            "teams/service",
        ])
        .assert()
        .success();

    let recipients =
        fs::read_to_string(store.path().join("gpg-recipients.txt")).expect("recipients");
    assert_eq!(recipients, "team@example.com\n");
}

#[test]
fn reports_missing_gpg_id_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");

    rpass()
        .write_stdin("secret\n")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "insert",
            "email/work",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"gpg_id_not_found\""));
}
