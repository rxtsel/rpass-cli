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

#[test]
fn force_move_onto_itself_preserves_entry_and_directory() {
    let store = tempfile::TempDir::new().expect("store");
    write_file(store.path().join("one.gpg"), "ciphertext");
    write_file(store.path().join("folder/two.gpg"), "ciphertext2");
    for name in ["one", "folder"] {
        rpass()
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "mv",
                "--force",
                name,
                name,
            ])
            .assert()
            .failure();
    }
    assert_eq!(
        fs::read(store.path().join("one.gpg")).unwrap(),
        b"ciphertext"
    );
    assert_eq!(
        fs::read(store.path().join("folder/two.gpg")).unwrap(),
        b"ciphertext2"
    );
}

#[test]
fn rejects_move_across_recipient_boundaries() {
    let store = tempfile::TempDir::new().expect("store");
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(store.path().join("team/.gpg-id"), "bob\n");
    write_file(store.path().join("team/secret.gpg"), "ciphertext");
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "team/secret",
            "secret",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("different GPG recipients"));
    assert!(store.path().join("team/secret.gpg").exists());
    assert!(!store.path().join("secret.gpg").exists());
}

#[test]
fn rejects_directory_move_if_inherited_recipients_change() {
    let store = tempfile::TempDir::new().expect("store");
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(store.path().join("team/.gpg-id"), "bob\n");
    write_file(store.path().join("team/section/secret.gpg"), "ciphertext");
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "team/section",
            "section",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("different GPG recipients"));
    assert!(store.path().join("team/section/secret.gpg").exists());
}

#[test]
fn force_directory_move_checks_recipients_after_destination_is_replaced() {
    let store = tempfile::TempDir::new().expect("store");
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(store.path().join("source/.gpg-id"), "bob\n");
    write_file(store.path().join("source/branch/secret.gpg"), "ciphertext");
    write_file(store.path().join("dest/.gpg-id"), "bob\n");
    write_file(store.path().join("dest/old.gpg"), "old");
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "--force",
            "source/branch",
            "dest",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("different GPG recipients"));
    assert!(store.path().join("source/branch/secret.gpg").exists());
    assert!(store.path().join("dest/old.gpg").exists());
}

#[cfg(unix)]
#[test]
fn force_move_reports_failed_backup_cleanup() {
    use std::os::unix::fs::PermissionsExt;

    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(store.path().join("source/new.gpg"), "new");
    write_file(store.path().join("destination/private/old.gpg"), "old");
    let private = store.path().join("destination/private");
    fs::set_permissions(&private, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read_dir(&private).is_err() {
        rpass()
            .args([
                "--store-dir",
                store.path().to_str().unwrap(),
                "mv",
                "--force",
                "source",
                "destination",
            ])
            .assert()
            .failure()
            .stderr(predicate::str::contains(
                "old destination could not be removed",
            ));
        assert!(store.path().join("destination/new.gpg").exists());
        let backup = fs::read_dir(store.path())
            .unwrap()
            .map(Result::unwrap)
            .find(|item| {
                item.file_name()
                    .to_string_lossy()
                    .starts_with(".rpass-backup-")
            })
            .unwrap()
            .path();
        fs::set_permissions(
            backup.join("previous/private"),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
    } else {
        fs::set_permissions(&private, fs::Permissions::from_mode(0o700)).unwrap();
    }
}

#[cfg(windows)]
#[test]
fn renames_only_the_case_of_files_and_directories() {
    let store = tempfile::TempDir::new().unwrap();
    write_file(store.path().join(".gpg-id"), "alice\n");
    write_file(store.path().join("old.gpg"), "secret");
    write_file(store.path().join("folder/child.gpg"), "child");
    for (old, new, force) in [("old", "OLD", false), ("folder", "FOLDER", true)] {
        let mut command = rpass();
        command.args(["--store-dir", store.path().to_str().unwrap(), "mv"]);
        if force {
            command.arg("--force");
        }
        command.args([old, new]).assert().success();
    }
    rpass()
        .args([
            "--store-dir",
            store.path().to_str().unwrap(),
            "mv",
            "FOLDER/child",
            "folder/child",
        ])
        .assert()
        .failure();
    let names = fs::read_dir(store.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(names.contains(&"OLD.gpg".to_owned()));
    assert!(names.contains(&"FOLDER".to_owned()));
    assert!(!names.contains(&"old.gpg".to_owned()));
    assert!(!names.contains(&"folder".to_owned()));
    assert_eq!(fs::read(store.path().join("OLD.gpg")).unwrap(), b"secret");
    assert_eq!(
        fs::read(store.path().join("FOLDER/child.gpg")).unwrap(),
        b"child"
    );
}

fn write_file(path: impl AsRef<Path>, content: &str) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, content).expect("file");
}
