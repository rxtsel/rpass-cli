use std::fs;
use std::path::Path;

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn create_entry(store: &Path, name: &str) {
    let path = store.join(name);
    fs::create_dir_all(path.parent().expect("parent dir")).expect("parent dir");
    fs::write(path, "").expect("entry");
}

#[test]
fn complete_entries_is_hidden_from_help() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete-entries").not());
}

#[test]
fn empty_prefix_returns_all_entries() {
    let temp_dir = TempDir::new().expect("temp dir");
    create_entry(temp_dir.path(), "a.gpg");
    create_entry(temp_dir.path(), "b.gpg");

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            temp_dir.path().to_str().expect("store path"),
            "complete-entries",
        ])
        .assert()
        .success()
        .stdout("a\nb\n");
}

#[test]
fn missing_store_returns_no_candidates() {
    let temp_dir = TempDir::new().expect("temp dir");
    let missing_store = temp_dir.path().join("missing");

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            missing_store.to_str().expect("store path"),
            "complete-entries",
        ])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn prefix_filters_entries() {
    let temp_dir = TempDir::new().expect("temp dir");
    create_entry(temp_dir.path(), "email/work.gpg");
    create_entry(temp_dir.path(), "email/personal.gpg");
    create_entry(temp_dir.path(), "github.gpg");

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            temp_dir.path().to_str().expect("store path"),
            "complete-entries",
            "--",
            "email/",
        ])
        .assert()
        .success()
        .stdout("email/personal\nemail/work\n");
}

#[test]
fn no_match_returns_empty() {
    let temp_dir = TempDir::new().expect("temp dir");
    create_entry(temp_dir.path(), "a.gpg");

    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args([
            "--store-dir",
            temp_dir.path().to_str().expect("store path"),
            "complete-entries",
            "--",
            "z",
        ])
        .assert()
        .success()
        .stdout("");
}

#[test]
fn completions_command_generates_script() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -F _rpass rpass"));
}

#[test]
fn bash_completions_script_contains_function() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args(["completions", "bash"])
        .assert()
        .success()
        .stdout(predicate::str::contains("_rpass()"));
}

#[test]
fn zsh_completions_script_contains_compdef() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args(["completions", "zsh"])
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef rpass"));
}

#[test]
fn powershell_completions_script_contains_register() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args(["completions", "powershell"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Register-ArgumentCompleter"));
}

#[test]
fn fish_completions_script_contains_complete_directive() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args(["completions", "fish"])
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c rpass"));
}
