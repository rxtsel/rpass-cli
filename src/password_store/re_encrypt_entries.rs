use std::fs;
use std::path::{Path, PathBuf};

use super::{GpgCommand, PasswordStore, PasswordStoreError};

pub struct ReEncryptEntries<'store> {
    store: &'store PasswordStore,
    gpg: GpgCommand,
}

impl<'store> ReEncryptEntries<'store> {
    pub fn new(store: &'store PasswordStore, gpg: GpgCommand) -> Self {
        Self { store, gpg }
    }

    pub fn execute(
        &self,
        subfolder: Option<&str>,
        recipients: &[String],
        passphrase: Option<&str>,
    ) -> Result<Vec<PathBuf>, PasswordStoreError> {
        if recipients.is_empty() {
            return Ok(Vec::new());
        }

        let target_dir = match subfolder {
            Some(sub) => self.store.path().join(sub),
            None => self.store.path().to_path_buf(),
        };

        let mut reencrypted = Vec::new();
        reencrypt_dir(
            &self.gpg,
            &target_dir,
            &target_dir,
            recipients,
            passphrase,
            &mut reencrypted,
        )?;
        Ok(reencrypted)
    }
}

fn reencrypt_dir(
    gpg: &GpgCommand,
    dir: &Path,
    target_dir: &Path,
    recipients: &[String],
    passphrase: Option<&str>,
    reencrypted: &mut Vec<PathBuf>,
) -> Result<(), PasswordStoreError> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if path.is_dir() {
            if path.join(".gpg-id").exists() {
                // This subdir manages its own recipients — skip it entirely
                continue;
            }
            reencrypt_dir(gpg, &path, target_dir, recipients, passphrase, reencrypted)?;
        } else if path.extension().map_or(false, |ext| ext == "gpg") {
            let plaintext = gpg.decrypt(&path, passphrase)?;
            gpg.encrypt(&plaintext, &path, recipients)?;
            reencrypted.push(path);
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::ReEncryptEntries;
    use crate::password_store::{GpgCommand, PasswordStore, StoreDirectory};

    fn store_with_files(files: &[(&str, &str)]) -> TempDir {
        let dir = TempDir::new().expect("temp dir");
        for (path, content) in files {
            let full_path = dir.path().join(path);
            fs::create_dir_all(full_path.parent().expect("parent")).expect("create dir");
            fs::write(&full_path, content).expect("write file");
        }
        dir
    }

    #[cfg(not(windows))]
    fn passthrough_gpg_script(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
        use std::os::unix::fs::PermissionsExt;

        let script = dir.join("gpg-test");
        let log = dir.join("gpg-test-log.txt");

        fs::write(
            &script,
            format!(
                r#"#!/bin/sh
set -eu
log='{log}'
mode='encrypt'
output=''
input_file=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --decrypt) mode='decrypt'; shift ;;
    --recipient) printf 'recipient:%s\n' "$2" >> "$log"; shift 2 ;;
    --output) output="$2"; shift 2 ;;
    -*) shift ;;
    *) input_file="$1"; shift ;;
  esac
done
if [ "$mode" = "decrypt" ]; then cat "$input_file"; exit 0; fi
printf 'encrypt\n' >> "$log"
cat > "$output"
"#,
                log = log.display()
            ),
        )
        .expect("write script");

        let mut perms = fs::metadata(&script).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).expect("set permissions");

        (script, log)
    }

    #[test]
    #[cfg(not(windows))]
    fn re_encrypts_gpg_files_in_store() {
        let temp = store_with_files(&[(".gpg-id", "alice\n"), ("entry.gpg", "secret\n")]);
        let (script, log) = passthrough_gpg_script(temp.path());
        let store = PasswordStore::open(StoreDirectory::from_path(temp.path())).expect("store");
        let gpg = GpgCommand::new(script);

        let result = ReEncryptEntries::new(&store, gpg)
            .execute(None, &["alice".to_string()], None)
            .expect("re-encrypt");

        assert_eq!(result.len(), 1);
        let log_content = fs::read_to_string(log).expect("log");
        assert!(log_content.contains("recipient:alice"));
        assert_eq!(
            fs::read_to_string(temp.path().join("entry.gpg")).expect("entry"),
            "secret\n"
        );
    }

    #[test]
    #[cfg(not(windows))]
    fn skips_entries_in_subdir_with_own_gpg_id() {
        let temp = store_with_files(&[
            (".gpg-id", "alice\n"),
            ("team/.gpg-id", "team\n"),
            ("entry.gpg", "root\n"),
            ("team/entry.gpg", "team-secret\n"),
        ]);
        let (script, log) = passthrough_gpg_script(temp.path());
        let store = PasswordStore::open(StoreDirectory::from_path(temp.path())).expect("store");
        let gpg = GpgCommand::new(script);

        let result = ReEncryptEntries::new(&store, gpg)
            .execute(None, &["alice".to_string()], None)
            .expect("re-encrypt");

        assert_eq!(result.len(), 1, "only root entry should be re-encrypted");
        let log_content = fs::read_to_string(log).expect("log");
        assert_eq!(log_content.lines().filter(|l| *l == "encrypt").count(), 1);
    }

    #[test]
    #[cfg(not(windows))]
    fn returns_empty_when_no_gpg_files() {
        let temp = store_with_files(&[(".gpg-id", "alice\n")]);
        let (script, _) = passthrough_gpg_script(temp.path());
        let store = PasswordStore::open(StoreDirectory::from_path(temp.path())).expect("store");
        let gpg = GpgCommand::new(script);

        let result = ReEncryptEntries::new(&store, gpg)
            .execute(None, &["alice".to_string()], None)
            .expect("re-encrypt");

        assert!(result.is_empty());
    }
}
