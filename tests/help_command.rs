use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_concise_root_help() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage: rpass [OPTIONS] <COMMAND>"))
        .stdout(predicate::str::contains(
            "--store-dir <PATH>  Use a password store directory",
        ))
        .stdout(predicate::str::contains(
            "list  List password store entries",
        ))
        .stdout(predicate::str::contains("help  Print").not());
}
