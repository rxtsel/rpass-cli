mod support;

use std::fs;
use std::path::Path;

use predicates::prelude::*;

use support::rpass;

#[test]
fn removes_entry_and_prunes_empty_parent_directories() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(
        store.path().join("Personal/github.com/rxtsel.gpg"),
        "encrypted\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "rm",
            "--force",
            "Personal/github.com/rxtsel",
        ])
        .assert()
        .success()
        .stdout("Entry 'Personal/github.com/rxtsel' removed\n")
        .stderr("");

    assert!(!store.path().join("Personal/github.com/rxtsel.gpg").exists());
    assert!(!store.path().join("Personal/github.com").exists());
    assert!(!store.path().join("Personal").exists());
    assert!(store.path().join(".gpg-id").exists());
}

#[test]
fn git_failure_still_reports_removed_entry() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join("entry.gpg"), "ciphertext");
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(store.path())
            .arg("init")
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(store.path())
            .args(["add", "entry.gpg"])
            .status()
            .unwrap()
            .success()
    );
    assert!(
        std::process::Command::new("git")
            .arg("-C")
            .arg(store.path())
            .args([
                "-c",
                "user.name=test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "-m",
                "seed"
            ])
            .status()
            .unwrap()
            .success()
    );
    rpass()
        .env("GIT_AUTHOR_NAME", "")
        .env("GIT_COMMITTER_NAME", "")
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "rm",
            "--force",
            "entry",
            "--json",
        ])
        .assert()
        .failure()
        .stdout(predicate::str::contains("\"name\": \"entry\""))
        .stderr(predicate::str::contains("\"code\": \"git_failed\""));
    assert!(!store.path().join("entry.gpg").exists());
}

#[test]
fn keeps_parent_directory_when_it_contains_another_entry() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(
        store.path().join("Personal/github.com/rxtsel.gpg"),
        "encrypted\n",
    );
    write_file(
        store.path().join("Personal/github.com/other.gpg"),
        "encrypted\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "rm",
            "--force",
            "Personal/github.com/rxtsel",
        ])
        .assert()
        .success();

    assert!(!store.path().join("Personal/github.com/rxtsel.gpg").exists());
    assert!(store.path().join("Personal/github.com/other.gpg").exists());
    assert!(store.path().join("Personal/github.com").is_dir());
    assert!(store.path().join("Personal").is_dir());
}

#[test]
fn prunes_removed_branch_but_keeps_sibling_directories() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(
        store.path().join("Personal/github.com/rxtsel.gpg"),
        "encrypted\n",
    );
    write_file(
        store.path().join("Personal/example/login.gpg"),
        "encrypted\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "rm",
            "--force",
            "Personal/github.com/rxtsel",
        ])
        .assert()
        .success();

    assert!(!store.path().join("Personal/github.com/rxtsel.gpg").exists());
    assert!(!store.path().join("Personal/github.com").exists());
    assert!(store.path().join("Personal/example/login.gpg").exists());
    assert!(store.path().join("Personal").is_dir());
}

#[test]
fn does_not_prune_directory_with_non_entry_files() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(
        store.path().join("Personal/github.com/.gpg-id"),
        "team@example.invalid\n",
    );
    write_file(
        store.path().join("Personal/github.com/rxtsel.gpg"),
        "encrypted\n",
    );

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "rm",
            "--force",
            "Personal/github.com/rxtsel",
        ])
        .assert()
        .success();

    assert!(!store.path().join("Personal/github.com/rxtsel.gpg").exists());
    assert!(store.path().join("Personal/github.com/.gpg-id").exists());
    assert!(store.path().join("Personal/github.com").is_dir());
    assert!(store.path().join("Personal").is_dir());
}

#[test]
fn reports_missing_entry_as_json() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "rm",
            "--force",
            "missing/entry",
            "--json",
        ])
        .assert()
        .failure()
        .stdout("")
        .stderr(predicate::str::contains("\"code\": \"entry_not_found\""));
}

#[test]
fn removes_entry_as_json_for_integrations() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "alice@example.invalid\n");
    write_file(store.path().join("example/login.gpg"), "encrypted\n");

    rpass()
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "rm",
            "--force",
            "example/login",
            "--json",
        ])
        .assert()
        .success()
        .stdout("{\n  \"name\": \"example/login\"\n}\n")
        .stderr("");

    assert!(!store.path().join("example/login.gpg").exists());
}

#[cfg(unix)]
#[test]
fn rejects_symlink_entry_outside_store() {
    let store = tempfile::TempDir::new().expect("store");
    let outside = tempfile::TempDir::new().expect("outside");
    write_file(outside.path().join("secret.gpg"), "ciphertext");
    std::os::unix::fs::symlink(outside.path(), store.path().join("link")).unwrap();
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "rm",
            "--force",
            "link/secret",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("outside the password store"));
    assert!(outside.path().join("secret.gpg").exists());
}

fn write_file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("file");
}
