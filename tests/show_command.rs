mod support;

use predicates::prelude::*;
use tempfile::TempDir;

use support::{
    failing_gpg_script, missing_executable_path, password_store_with_entry, rpass,
    successful_gpg_script,
};

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
        .stdout(
            "{\n  \"name\": \"email/work\",\n  \"password\": \"secret\",\n  \"fields\": [\n    {\n      \"name\": \"username\",\n      \"value\": \"alice\"\n    },\n    {\n      \"name\": \"url\",\n      \"value\": \"https://example.com\"\n    }\n  ],\n  \"otp_uri\": \"otpauth://totp/example\",\n  \"extra_lines\": [\n    \"note\"\n  ]\n}\n",
        );
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
fn reports_invalid_entry_name_as_json() {
    let store = TempDir::new().expect("temp dir");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "show",
            "../outside",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(
            "{\n  \"error\": {\n    \"code\": \"invalid_entry_name\",\n    \"message\": \"invalid entry name '../outside': entry name cannot contain '.' or '..' path segments\"\n  }\n}\n",
        );
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
