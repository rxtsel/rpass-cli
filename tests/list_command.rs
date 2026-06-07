use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

#[test]
fn lists_password_store_entries_as_text() {
    let store = password_store_with_entries(["email/work.gpg", "github.gpg"]);

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "list",
        ])
        .assert()
        .success()
        .stdout(
            "\
Password Store
\u{251c}\u{2500}\u{2500} email
\u{2502}   \u{2514}\u{2500}\u{2500} work
\u{2514}\u{2500}\u{2500} github
",
        );
}

#[test]
fn lists_password_store_entries_as_json() {
    let store = password_store_with_entries(["email/work.gpg", "github.gpg"]);

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "list",
            "--json",
        ])
        .assert()
        .success()
        .stdout("[\n  \"email/work\",\n  \"github\"\n]\n");
}

#[test]
fn lists_edge_case_entry_names() {
    let store = password_store_with_entries([
        "personal/OpenAI.com.gpg",
        "personal/BlackMagic Cloud.gpg",
        "work/rxtsel.dev/email/contact@rxtsel.dev.gpg",
    ]);

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "list",
            "--json",
        ])
        .assert()
        .success()
        .stdout(
            "[\n  \"personal/BlackMagic Cloud\",\n  \"personal/OpenAI.com\",\n  \"work/rxtsel.dev/email/contact@rxtsel.dev\"\n]\n",
        );
}

#[test]
fn reports_missing_store_directory() {
    let temp_dir = TempDir::new().expect("temp dir");
    let missing_store = temp_dir.path().join("missing");

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            missing_store.to_str().expect("store path"),
            "list",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("password store does not exist"));
}

fn password_store_with_entries<const N: usize>(entries: [&str; N]) -> TempDir {
    let store = TempDir::new().expect("temp dir");

    for entry in entries {
        create_file(store.path().join(entry));
    }

    create_file(store.path().join(".gpg-id"));
    store
}

fn create_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, "").expect("file");
}
