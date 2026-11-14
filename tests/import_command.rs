mod support;

use std::fs;

use predicates::prelude::*;
use support::{encrypting_gpg_script, rpass};

#[test]
fn partial_import_fails_and_reports_all_entries() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::create_dir(store.path().join("team")).unwrap();
    fs::write(store.path().join("team/.gpg-id"), "").unwrap();
    fs::write(&export, r#"{"folders":[{"id":"f","name":"team"}],"items":[{"name":"valid","login":{"password":"secret"}},{"name":"lost","folderId":"f","login":{"password":"lost"}}]}"#).unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            "--json",
            export.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"imported\": 1"))
        .stdout(predicate::str::contains("\"skipped\": 1"))
        .stderr(predicate::str::contains("import_incomplete"));
    assert!(store.path().join("valid.gpg").exists());
}

#[test]
fn retry_after_partial_import_does_not_duplicate_saved_passwords() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::create_dir(store.path().join("team")).unwrap();
    fs::write(store.path().join("team/.gpg-id"), "").unwrap();
    fs::write(
        &export,
        r#"{"folders":[{"id":"f","name":"team"}],"items":[{"name":"valid","login":{"password":"secret"}},{"name":"lost","folderId":"f"}]}"#,
    )
    .unwrap();
    for imported in [1, 0] {
        rpass()
            .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "import",
                "--bitwarden",
                "--json",
                export.to_str().unwrap(),
            ])
            .assert()
            .failure()
            .stdout(predicate::str::contains(format!(
                "\"imported\": {imported}"
            )));
    }
    assert!(store.path().join("valid.gpg").exists());
    assert!(!store.path().join("valid-1.gpg").exists());
}

#[test]
fn duplicate_names_in_one_export_still_receive_distinct_paths() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::write(&export, r#"{"items":[{"name":"valid","login":{"password":"one"}},{"name":"valid","login":{"password":"two"}}]}"#).unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            export.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(store.path().join("valid.gpg").exists());
    assert!(store.path().join("valid-1.gpg").exists());
}

#[test]
fn force_import_preserves_distinct_entries_with_the_same_name() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    fs::write(store.path().join("valid.gpg"), "old first").unwrap();
    let export = store.path().join("export.json");
    fs::write(
        &export,
        r#"{"items":[{"name":"valid","login":{"password":"one"}},{"name":"valid","login":{"password":"two"}}]}"#,
    )
    .unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            "--force",
            export.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 2/2"));
    assert!(
        fs::read_to_string(store.path().join("valid.gpg"))
            .unwrap()
            .contains("one")
    );
    assert!(
        fs::read_to_string(store.path().join("valid-1.gpg"))
            .unwrap()
            .contains("two")
    );
    assert!(!store.path().join("valid-2.gpg").exists());
}

#[test]
fn force_import_keeps_case_variant_names_distinct() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::write(&export, r#"{"items":[{"name":"Login","login":{"password":"one"}},{"name":"login","login":{"password":"two"}}]}"#).unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            "--force",
            export.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 2/2"));
    assert!(
        fs::read_to_string(store.path().join("Login.gpg"))
            .unwrap()
            .contains("one")
    );
    assert!(
        fs::read_to_string(store.path().join("login-1.gpg"))
            .unwrap()
            .contains("two")
    );
}

#[test]
fn force_import_does_not_overwrite_unrelated_suffixed_entry() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    fs::write(store.path().join("valid-1.gpg"), "unrelated password").unwrap();
    let export = store.path().join("export.json");
    fs::write(&export, r#"{"items":[{"name":"valid","login":{"password":"one"}},{"name":"valid","login":{"password":"two"}}]}"#).unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            "--force",
            "--json",
            export.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"imported\": 1"))
        .stdout(predicate::str::contains("\"skipped\": 1"))
        .stdout(predicate::str::contains("valid-1"));
    assert_eq!(
        fs::read(store.path().join("valid-1.gpg")).unwrap(),
        b"unrelated password"
    );
    assert!(!store.path().join("valid-2.gpg").exists());
}

#[test]
fn multiline_password_rejects_export_before_writing() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::write(&export, r#"{"items":[{"name":"valid","login":{"password":"one"}},{"name":"multiline","login":{"password":"two\nthree"}}]}"#).unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            export.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("item 2 has a multiline password"));
    assert!(!store.path().join("valid.gpg").exists());
    assert!(!store.path().join("multiline.gpg").exists());
}

#[test]
fn fields_that_change_on_import_reject_export_before_writing() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    for (item, error) in [
        (
            r#""fields":[{"name":"recovery","value":"one\ntwo"}]"#,
            "a field that cannot be stored unchanged",
        ),
        (
            r#""fields":[{"name":"recovery","value":" secret "}]"#,
            "a field that cannot be stored unchanged",
        ),
        (
            r#""fields":[{"name":"recovery:key","value":"secret"}]"#,
            "a field that cannot be stored unchanged",
        ),
    ] {
        fs::write(
            &export,
            format!(r#"{{"items":[{{"name":"valid"}},{{"name":"bad",{item}}}]}}"#),
        )
        .unwrap();
        rpass()
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "import",
                "--bitwarden",
                export.to_str().unwrap(),
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains(format!("item 2 has {error}")));
        assert!(!store.path().join("valid.gpg").exists());
    }
}

#[test]
fn imports_notes_that_look_like_metadata_or_otp() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::write(&export, r#"{"items":[{"name":"note","type":2,"notes":"See https://example.com\nusername: someone else\notpauth://totp/other?secret=ABC"}]}"#).unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            export.to_str().unwrap(),
        ])
        .assert()
        .success();
    let content = fs::read_to_string(store.path().join("note.gpg")).unwrap();
    assert!(content.contains("# rpass-note See https://example.com"));
    assert!(content.contains("# rpass-note username: someone else"));
    assert!(content.contains("# rpass-note otpauth://totp/other?secret=ABC"));
}

#[test]
fn cleaned_invalid_name_rejects_export_before_writing() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    for name in ["'", ".gpg"] {
        fs::write(
            &export,
            format!(r#"{{"items":[{{"name":"valid"}},{{"name":"{name}"}}]}}"#),
        )
        .unwrap();
        rpass()
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "import",
                "--bitwarden",
                export.to_str().unwrap(),
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains("item 2 has no valid name"));
        assert!(!store.path().join("valid.gpg").exists());
    }
}

#[test]
fn imported_line_break_in_title_becomes_safe_name() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::write(
        &export,
        r#"{"items":[{"name":"foo\nbar","login":{"password":"secret"}}]}"#,
    )
    .unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            export.to_str().unwrap(),
        ])
        .assert()
        .success();
    assert!(store.path().join("foo-bar.gpg").exists());
    assert!(!store.path().join("foo\nbar.gpg").exists());
}

#[test]
fn git_folder_in_export_is_rejected_before_writing() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::write(&export, r#"{"folders":[{"id":"f","name":".git"}],"items":[{"name":"valid"},{"name":"hidden","folderId":"f"}]}"#).unwrap();
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            export.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("item 2 has no valid name"));
    assert!(!store.path().join("valid.gpg").exists());
    assert!(!store.path().join(".git/hidden.gpg").exists());
}

#[test]
fn git_failure_still_reports_imported_entries() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let status = std::process::Command::new("git")
        .arg("-C")
        .arg(store.path())
        .arg("init")
        .status()
        .expect("git init");
    assert!(status.success());
    let export = store.path().join("export.json");
    fs::write(
        &export,
        r#"{"items":[{"name":"valid","login":{"password":"one"}}]}"#,
    )
    .unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .env("GIT_AUTHOR_NAME", "")
        .env("GIT_COMMITTER_NAME", "")
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            "--json",
            export.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"imported\": 1"))
        .stderr(predicate::str::contains("git failed"));
    assert!(store.path().join("valid.gpg").exists());
}

#[test]
fn malformed_export_does_not_write_partial_vault() {
    let store = tempfile::TempDir::new().unwrap();
    fs::write(store.path().join(".gpg-id"), "alice\n").unwrap();
    let export = store.path().join("export.json");
    fs::write(
        &export,
        r#"{"items":[{"name":"valid","login":{"password":"secret"}},{"name":""}]}"#,
    )
    .unwrap();
    rpass()
        .env("PASSWORD_STORE_GPG", encrypting_gpg_script(store.path()))
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "import",
            "--bitwarden",
            export.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("item 2 has no valid name"));
    assert!(!store.path().join("valid.gpg").exists());
}
