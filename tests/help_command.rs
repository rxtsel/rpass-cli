use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn prints_concise_root_help() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .arg("-h")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Usage: rpass [OPTIONS] [ENTRY] [COMMAND]",
        ))
        .stdout(predicate::str::contains(
            "--store-dir <PATH>  Use a store directory instead of PASSWORD_STORE_DIR",
        ))
        .stdout(predicate::str::contains("List password store entries"))
        .stdout(predicate::str::contains("Show a password store entry"))
        .stdout(predicate::str::contains(
            "Generate an OTP code for a password store entry",
        ))
        .stdout(predicate::str::contains(
            "search  Search password store entries",
        ))
        .stdout(predicate::str::contains(
            "doctor  Check the local rpass environment",
        ))
        .stdout(predicate::str::contains("help  Print").not());
}

#[test]
fn show_help_only_advertises_passphrase_stdin() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args(["show", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--passphrase-stdin"))
        .stdout(predicate::str::contains("--passphrase <").not());
}

#[test]
fn otp_help_only_advertises_passphrase_stdin() {
    Command::cargo_bin("rpass")
        .expect("rpass binary")
        .args(["otp", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--passphrase-stdin"))
        .stdout(predicate::str::contains("--passphrase <").not());
}
