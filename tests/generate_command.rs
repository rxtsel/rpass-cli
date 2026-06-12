mod support;

use std::fs;

use predicates::prelude::*;
use serde_json::Value;

use support::{encrypting_gpg_script, rpass};

#[test]
fn generates_default_password_and_inserts_entry() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.invalid\n").expect("gpg id");
    let gpg = encrypting_gpg_script(store.path());

    let assert = rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/login",
        ])
        .assert()
        .success()
        .stderr("");
    let generated = stdout_line(&assert);

    assert_eq!(generated.chars().count(), 14);
    assert!(
        generated
            .chars()
            .any(|character| character.is_ascii_lowercase())
    );
    assert!(
        generated
            .chars()
            .any(|character| character.is_ascii_uppercase())
    );
    assert!(
        generated
            .chars()
            .any(|character| character.is_ascii_digit())
    );
    assert!(
        generated
            .chars()
            .any(|character| !character.is_alphanumeric())
    );

    let encrypted = fs::read_to_string(store.path().join("example/login.gpg")).expect("entry");
    assert_eq!(encrypted, format!("{generated}\n"));
}

#[test]
fn generates_custom_length_without_symbols() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.invalid\n").expect("gpg id");
    let gpg = encrypting_gpg_script(store.path());

    let assert = rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/login",
            "18",
            "--no-symbols",
        ])
        .assert()
        .success();
    let generated = stdout_line(&assert);

    assert_eq!(generated.chars().count(), 18);
    assert!(
        generated
            .chars()
            .all(|character| character.is_ascii_alphanumeric())
    );
}

#[test]
fn generates_json_output_for_integrations() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.invalid\n").expect("gpg id");
    let gpg = encrypting_gpg_script(store.path());

    let assert = rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/login",
            "--json",
        ])
        .assert()
        .success()
        .stderr("");
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");
    let password = output["password"].as_str().expect("password");

    assert_eq!(output["name"], "example/login");
    assert_eq!(password.chars().count(), 14);
}

#[test]
fn refuses_to_overwrite_without_force() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.invalid\n").expect("gpg id");
    fs::create_dir_all(store.path().join("example")).expect("entry dir");
    fs::write(store.path().join("example/login.gpg"), "existing\n").expect("entry");
    let gpg = encrypting_gpg_script(store.path());

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/login",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "\"code\": \"entry_already_exists\"",
        ));

    let encrypted = fs::read_to_string(store.path().join("example/login.gpg")).expect("entry");
    assert_eq!(encrypted, "existing\n");
}

#[test]
fn force_overwrites_existing_entry() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.invalid\n").expect("gpg id");
    fs::create_dir_all(store.path().join("example")).expect("entry dir");
    fs::write(store.path().join("example/login.gpg"), "existing\n").expect("entry");
    let gpg = encrypting_gpg_script(store.path());

    let assert = rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/login",
            "--force",
        ])
        .assert()
        .success();
    let generated = stdout_line(&assert);

    let encrypted = fs::read_to_string(store.path().join("example/login.gpg")).expect("entry");
    assert_eq!(encrypted, format!("{generated}\n"));
}

#[test]
fn generates_memorable_passphrase() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.invalid\n").expect("gpg id");
    let gpg = encrypting_gpg_script(store.path());

    let assert = rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/passphrase",
            "--phrase",
            "--words",
            "4",
            "--separator",
            "-",
            "--capitalize",
            "--number",
        ])
        .assert()
        .success();
    let generated = stdout_line(&assert);
    let parts = generated.split('-').collect::<Vec<_>>();

    assert_eq!(parts.len(), 5);
    assert!(
        parts[..4]
            .iter()
            .all(|word| word.chars().next().is_some_and(char::is_uppercase))
    );
    assert!(parts[4].chars().all(|character| character.is_ascii_digit()));
}

#[test]
fn dry_run_generates_password_without_store_or_gpg() {
    let temp_dir = tempfile::TempDir::new().expect("temp dir");
    let missing_store = temp_dir.path().join("missing-store");

    let assert = rpass()
        .args([
            "--store-dir",
            missing_store.to_str().expect("store path"),
            "generate",
            "--dry-run",
            "--length",
            "20",
        ])
        .assert()
        .success()
        .stderr("");
    let generated = stdout_line(&assert);

    assert_eq!(generated.chars().count(), 20);
    assert!(!missing_store.exists());
}

#[test]
fn dry_run_generates_passphrase_without_entry() {
    let assert = rpass()
        .args([
            "generate",
            "--dry-run",
            "--phrase",
            "--words",
            "4",
            "--separator",
            "_",
        ])
        .assert()
        .success()
        .stderr("");
    let generated = stdout_line(&assert);

    assert_eq!(generated.split('_').count(), 4);
}

#[test]
fn dry_run_json_omits_name_when_entry_is_not_provided() {
    let assert = rpass()
        .args(["generate", "--dry-run", "--length", "18", "--json"])
        .assert()
        .success()
        .stderr("");
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");

    assert_eq!(
        output["password"]
            .as_str()
            .expect("password")
            .chars()
            .count(),
        18
    );
    assert_eq!(output["dry_run"], true);
    assert!(output.get("name").is_none());
}

#[test]
fn dry_run_json_includes_name_when_entry_is_provided() {
    let assert = rpass()
        .args([
            "generate",
            "example/login",
            "--dry-run",
            "--length",
            "18",
            "--json",
        ])
        .assert()
        .success()
        .stderr("");
    let output: Value = serde_json::from_slice(&assert.get_output().stdout).expect("json");

    assert_eq!(output["name"], "example/login");
    assert_eq!(
        output["password"]
            .as_str()
            .expect("password")
            .chars()
            .count(),
        18
    );
    assert_eq!(output["dry_run"], true);
}

#[test]
fn requires_entry_without_dry_run() {
    rpass()
        .args(["generate", "--length", "18"])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "entry is required unless --dry-run is used",
        ));
}

#[test]
fn reports_empty_character_set_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");
    fs::write(store.path().join(".gpg-id"), "alice@example.invalid\n").expect("gpg id");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "generate",
            "example/login",
            "--no-lowercase",
            "--no-uppercase",
            "--no-numbers",
            "--no-symbols",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains(
            "\"code\": \"password_generation_failed\"",
        ))
        .stderr(predicate::str::contains(
            "at least one character set must be enabled",
        ));
}

fn stdout_line(assert: &assert_cmd::assert::Assert) -> String {
    String::from_utf8(assert.get_output().stdout.clone())
        .expect("stdout")
        .trim_end_matches(['\r', '\n'])
        .to_string()
}
