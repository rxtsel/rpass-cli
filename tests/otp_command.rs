mod support;

use predicates::prelude::*;

use support::{password_store_with_entry, rpass, successful_gpg_script};

#[test]
fn generates_otp_code_as_text() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = successful_gpg_script(store.path(), entry_with_otp_uri());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "otp",
            "email/work",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^\d{6}\n$").expect("regex"));
}

#[test]
fn generates_otp_code_as_json() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = successful_gpg_script(store.path(), entry_with_otp_uri());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "otp",
            "email/work",
            "--json",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\": \"email/work\""))
        .stdout(predicate::str::is_match(r#""code": "\d{6}""#).expect("regex"))
        .stdout(predicate::str::contains("\"remaining_seconds\""))
        .stdout(predicate::str::contains("\"period\": 30"));
}

#[test]
fn reports_entry_without_otp_uri() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = successful_gpg_script(store.path(), "secret\nusername: alice\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "otp",
            "email/work",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "entry does not contain an otpauth URI",
        ));
}

#[test]
fn reports_invalid_otp_uri() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = successful_gpg_script(store.path(), "secret\notpauth://totp/example\n");

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "otp",
            "email/work",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "entry contains an invalid otpauth URI",
        ));
}

fn entry_with_otp_uri() -> &'static str {
    "\
secret
otpauth://totp/GitHub:test?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ
"
}
