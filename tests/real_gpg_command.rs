mod support;

use std::fs;
use std::io::Write;
use std::process::{Command, Stdio};

use support::rpass;

#[test]
fn changing_recipients_grants_bob_and_revokes_alice_with_real_gpg() {
    let gpg = "gpg";
    assert!(
        Command::new(gpg)
            .arg("--version")
            .output()
            .expect("GnuPG is required for the real-key test")
            .status
            .success()
    );
    let store = tempfile::TempDir::new().unwrap();
    let home = tempfile::TempDir::new().unwrap();
    for name in ["Alice", "Bob"] {
        let status = Command::new(gpg)
            .env("GNUPGHOME", home.path())
            .args([
                "--batch",
                "--pinentry-mode=loopback",
                "--passphrase",
                "",
                "--quick-generate-key",
                &format!("{name} <{name}@example.invalid>"),
                "rsa2048",
                "encr",
                "0",
            ])
            .status()
            .unwrap();
        assert!(
            status.success(),
            "could not generate test GPG key for {name}"
        );
    }
    let root = store.path().to_str().unwrap();
    let alice = "Alice@example.invalid";
    let bob = "Bob@example.invalid";
    let alice_home = tempfile::TempDir::new().unwrap();
    let bob_home = tempfile::TempDir::new().unwrap();
    for (name, destination) in [(alice, alice_home.path()), (bob, bob_home.path())] {
        let key = Command::new(gpg)
            .env("GNUPGHOME", home.path())
            .args(["--batch", "--export-secret-keys", name])
            .output()
            .unwrap();
        assert!(key.status.success());
        let mut import = Command::new(gpg)
            .env("GNUPGHOME", destination)
            .args(["--batch", "--import"])
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        import.stdin.take().unwrap().write_all(&key.stdout).unwrap();
        assert!(import.wait().unwrap().success());
    }
    rpass()
        .env("GNUPGHOME", home.path())
        .args(["--store-dir", root, "init", alice])
        .assert()
        .success();
    rpass()
        .env("GNUPGHOME", home.path())
        .write_stdin("test-secret\n")
        .args(["--store-dir", root, "insert", "entry"])
        .assert()
        .success();
    rpass()
        .env("GNUPGHOME", home.path())
        .args(["--store-dir", root, "recipients", "add", bob])
        .assert()
        .success();
    rpass()
        .env("GNUPGHOME", alice_home.path())
        .args(["--store-dir", root, "show", "entry"])
        .assert()
        .success()
        .stdout("test-secret\n");
    rpass()
        .env("GNUPGHOME", home.path())
        .args(["--store-dir", root, "recipients", "remove", alice])
        .assert()
        .success();
    rpass()
        .env("GNUPGHOME", alice_home.path())
        .args(["--store-dir", root, "show", "entry"])
        .assert()
        .failure();
    rpass()
        .env("GNUPGHOME", bob_home.path())
        .args(["--store-dir", root, "show", "entry"])
        .assert()
        .success()
        .stdout("test-secret\n");
    assert_eq!(
        fs::read_to_string(store.path().join(".gpg-id")).unwrap(),
        format!("{bob}\n")
    );
}
