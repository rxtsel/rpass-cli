mod support;

use predicates::prelude::*;

use support::{passphrase_gpg_script, password_store_with_entry, rpass, successful_gpg_script};

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
fn generates_otp_code_as_json_with_passphrase_stdin() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = passphrase_gpg_script(store.path(), "correct horse", entry_with_otp_uri());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .write_stdin("correct horse\n")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "otp",
            "--json",
            "--passphrase-stdin",
            "email/work",
        ])
        .assert()
        .success()
        .stderr("")
        .stdout(predicate::str::contains("\"name\": \"email/work\""))
        .stdout(predicate::str::is_match(r#"\"code\": \"\d{6}\""#).expect("regex"))
        .stdout(predicate::str::contains("\"remaining_seconds\""))
        .stdout(predicate::str::contains("\"period\": 30"));
}

#[test]
fn accepts_lowercase_otp_secret() {
    let store = password_store_with_entry("finance/stripe.gpg");
    let gpg = successful_gpg_script(
        store.path(),
        "\
secret
otpauth://totp/Stripe:test?secret=eq2gkb3bljy7hansqf2kmqb7
",
    );

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "otp",
            "finance/stripe",
        ])
        .assert()
        .success()
        .stdout(predicate::str::is_match(r"^\d{6}\n$").expect("regex"));
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
fn reports_entry_without_otp_uri_as_json() {
    let store = password_store_with_entry("email/work.gpg");
    let gpg = successful_gpg_script(store.path(), "secret\nusername: alice\n");

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
        .failure()
        .stdout("")
        .stderr(
            "{\n  \"error\": {\n    \"code\": \"otp_not_found\",\n    \"message\": \"entry does not contain an otpauth URI\"\n  }\n}\n",
        );
}

#[test]
fn reports_invalid_otp_uri() {
    let store = password_store_with_entry("email/work.gpg");
    let leaked_secret = "not-valid-secret";
    let gpg = successful_gpg_script(
        store.path(),
        "\
secret
otpauth://totp/example?secret=not-valid-secret
",
    );

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
        ))
        .stderr(predicate::str::contains(leaked_secret).not());
}

fn entry_with_otp_uri() -> &'static str {
    "\
secret
otpauth://totp/GitHub:test?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ
"
}
