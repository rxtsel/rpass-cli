use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_version_with_lowercase_short_flag() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("rpass "));
}
