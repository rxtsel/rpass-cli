mod support;

use std::fs;
use std::path::Path;

use support::{editing_gpg_script, editing_script, encrypting_gpg_script, rpass};

#[test]
fn insert_multiline_preserves_password_store_entry_shape() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "root@example.invalid\n");
    write_file(
        store.path().join("team/example/.gpg-id"),
        "# team recipients\n\nteam@example.invalid\nbackup@example.invalid\n",
    );
    let gpg = encrypting_gpg_script(store.path());
    let entry_content = "dummy-password\nusername: demo\nurl: https://example.invalid/login\notpauth://totp/example?secret=KRSXG5CTMVRXEZLUKN2XAZLSKNSWG4TFOQ\nrecovery code: 123456\nfree-form note\n";

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .write_stdin(entry_content)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "insert",
            "--multiline",
            "team/example/login",
        ])
        .assert()
        .success()
        .stdout("")
        .stderr("");

    let encrypted =
        fs::read_to_string(store.path().join("team/example/login.gpg")).expect("encrypted entry");
    assert_eq!(encrypted, entry_content);

    let recipients =
        fs::read_to_string(store.path().join("gpg-recipients.txt")).expect("recipients");
    assert_eq!(
        recipients.lines().collect::<Vec<_>>(),
        vec!["team@example.invalid", "backup@example.invalid"]
    );
}

#[test]
fn edit_existing_entry_uses_nearest_recipients_and_preserves_entry_shape() {
    let store = tempfile::TempDir::new().expect("temp dir");
    write_file(store.path().join(".gpg-id"), "root@example.invalid\n");
    write_file(
        store.path().join("team/example/.gpg-id"),
        "team@example.invalid\nbackup@example.invalid\n",
    );
    write_file(store.path().join("team/example/login.gpg"), "encrypted\n");
    let decrypted_content = "old-password\nusername: demo\nurl: https://example.invalid/login\n";
    let edited_content = "new-dummy-password\nusername: demo\nurl: https://example.invalid/login\nnotes: rotated by test\n";
    let gpg = editing_gpg_script(store.path(), decrypted_content);
    let editor = editing_script(store.path(), edited_content);

    rpass()
        .env("PASSWORD_STORE_GPG", gpg)
        .env("EDITOR", editor)
        .args([
            "--store-dir",
            store.path().to_str().expect("store path"),
            "edit",
            "team/example/login",
        ])
        .assert()
        .success()
        .stdout("Entry 'team/example/login' updated\n")
        .stderr("");

    let encrypted =
        fs::read_to_string(store.path().join("team/example/login.gpg")).expect("encrypted entry");
    assert_eq!(encrypted, edited_content);

    let recipients =
        fs::read_to_string(store.path().join("gpg-recipients.txt")).expect("recipients");
    assert_eq!(
        recipients.lines().collect::<Vec<_>>(),
        vec!["team@example.invalid", "backup@example.invalid"]
    );
}

fn write_file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("file");
}
