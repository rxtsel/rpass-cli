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
