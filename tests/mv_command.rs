mod support;

use std::fs;
use std::path::Path;

use predicates::prelude::*;

use support::rpass;

#[test]
fn moves_entry_to_new_name_and_prunes_empty_source_directories() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(
        store.path().join("Personal/github.com/rxtsel.gpg"),
        "encrypted-rxtsel\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "Personal/github.com/rxtsel",
            "Work/git.example/alice",
        ])
        .assert()
        .success()
        .stdout("Entry 'Personal/github.com/rxtsel' moved to 'Work/git.example/alice'\n")
        .stderr("");

    assert!(!store.path().join("Personal/github.com/rxtsel.gpg").exists());
    assert!(!store.path().join("Personal/github.com").exists());
    assert!(!store.path().join("Personal").exists());
    assert_eq!(
        fs::read_to_string(store.path().join("Work/git.example/alice.gpg")).expect("moved entry"),
        "encrypted-rxtsel\n"
    );
}

#[test]
fn keeps_source_parent_when_it_contains_another_entry() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(store.path().join("Personal/github.com/rxtsel.gpg"), "one\n");
    write_file(store.path().join("Personal/github.com/other.gpg"), "two\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "Personal/github.com/rxtsel",
            "Archive/rxtsel",
        ])
        .assert()
        .success();

    assert!(!store.path().join("Personal/github.com/rxtsel.gpg").exists());
    assert!(store.path().join("Personal/github.com/other.gpg").exists());
    assert!(store.path().join("Personal/github.com").is_dir());
    assert_eq!(
        fs::read_to_string(store.path().join("Archive/rxtsel.gpg")).expect("moved entry"),
        "one\n"
    );
}

#[test]
fn refuses_to_overwrite_existing_entry_without_force() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(store.path().join("old.gpg"), "old\n");
    write_file(store.path().join("new.gpg"), "new\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "old",
            "new",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("entry already exists: new"));

    assert_eq!(
        fs::read_to_string(store.path().join("old.gpg")).expect("source"),
        "old\n"
    );
    assert_eq!(
        fs::read_to_string(store.path().join("new.gpg")).expect("destination"),
        "new\n"
    );
}

#[test]
fn force_overwrites_existing_entry() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(store.path().join("old.gpg"), "old\n");
    write_file(store.path().join("new.gpg"), "new\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "--force",
            "old",
            "new",
        ])
        .assert()
        .success();

    assert!(!store.path().join("old.gpg").exists());
    assert_eq!(
        fs::read_to_string(store.path().join("new.gpg")).expect("destination"),
        "old\n"
    );
}

#[test]
fn moves_directory_tree() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(
        store.path().join("Personal/github.com/rxtsel.gpg"),
        "rxtsel\n",
    );
    write_file(store.path().join("Personal/git.example/work.gpg"), "work\n");
    write_file(
        store.path().join("Personal/git.example/.gpg-id"),
        "team@example.invalid\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "Personal",
            "Archive/Personal",
        ])
        .assert()
        .success()
        .stdout("Entry 'Personal' moved to 'Archive/Personal'\n")
        .stderr("");

    assert!(!store.path().join("Personal").exists());
    assert_eq!(
        fs::read_to_string(store.path().join("Archive/Personal/github.com/rxtsel.gpg"))
            .expect("moved entry"),
        "rxtsel\n"
    );
    assert_eq!(
        fs::read_to_string(store.path().join("Archive/Personal/git.example/work.gpg"))
            .expect("moved entry"),
        "work\n"
    );
    assert!(
        store
            .path()
            .join("Archive/Personal/git.example/.gpg-id")
            .exists()
    );
}

#[test]
fn reports_missing_source_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "missing/entry",
            "new/entry",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"entry_not_found\""));
}

#[test]
fn moves_entry_as_json_for_integrations() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(store.path().join("old.gpg"), "old\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "mv",
            "old",
            "new",
            "--json",
        ])
        .assert()
        .success()
        .stdout("{\n  \"old_name\": \"old\",\n  \"new_name\": \"new\"\n}\n")
        .stderr("");
}

fn write_file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("file");
}
