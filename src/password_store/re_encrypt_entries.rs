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
            self.store,
            &self.gpg,
            &target_dir,
            recipients,
            passphrase,
            &mut reencrypted,
        )?;
        Ok(reencrypted
            .into_iter()
            .map(|path| {
                path.strip_prefix(self.store.path())
                    .expect("entry in store")
                    .to_path_buf()
            })
            .collect())
    }
}

fn reencrypt_dir(
    store: &PasswordStore,
    gpg: &GpgCommand,
    dir: &Path,
    recipients: &[String],
    passphrase: Option<&str>,
    reencrypted: &mut Vec<PathBuf>,
) -> Result<(), PasswordStoreError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if super::entry_name::is_git_metadata_component(&name.to_string_lossy())
            || name.to_string_lossy().starts_with(".rpass-backup-")
            || entry.file_type()?.is_symlink()
        {
            continue;
        }
        store.check_path(&path)?;
        if path.is_dir() {
            if path.join(".gpg-id").exists() {
                continue;
            }
            reencrypt_dir(store, gpg, &path, recipients, passphrase, reencrypted)?;
        } else if path.extension().is_some_and(|ext| ext == "gpg") {
            let plaintext = gpg.decrypt(&path, passphrase)?;
            gpg.encrypt(&plaintext, &path, recipients)?;
            reencrypted.push(path);
        }
    }

    Ok(())
}
