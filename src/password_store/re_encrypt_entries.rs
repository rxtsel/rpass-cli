use std::fs;
use std::path::{Path, PathBuf};

use tempfile::{TempDir, TempPath};

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
    ) -> Result<(Vec<PathBuf>, Option<String>), PasswordStoreError> {
        let target_dir = match subfolder {
            Some(sub) => self.store.path().join(sub),
            None => self.store.path().to_path_buf(),
        };

        let mut staged = Vec::new();
        prepare_entries(
            self.store,
            &self.gpg,
            &target_dir,
            recipients,
            passphrase,
            &mut staged,
        )?;
        if staged.is_empty() {
            return Ok((Vec::new(), None));
        }

        let backup = tempfile::Builder::new()
            .prefix(".rpass-backup-")
            .tempdir_in(self.store.path())?;
        for (path, _) in &staged {
            let saved = backup.path().join(
                path.strip_prefix(self.store.path())
                    .expect("entry in store"),
            );
            fs::create_dir_all(saved.parent().expect("backup parent"))?;
            fs::copy(path, saved)?;
        }
        let (changed, warning) = install_staged(self.store.path(), staged, backup)?;
        Ok((
            changed
                .into_iter()
                .map(|path| {
                    path.strip_prefix(self.store.path())
                        .expect("entry in store")
                        .to_path_buf()
                })
                .collect(),
            warning,
        ))
    }
}

fn install_staged(
    root: &Path,
    staged: Vec<(PathBuf, TempPath)>,
    backup: TempDir,
) -> Result<(Vec<PathBuf>, Option<String>), PasswordStoreError> {
    let mut changed: Vec<PathBuf> = Vec::new();
    for (path, encrypted) in staged {
        if let Err(error) = encrypted.persist(&path) {
            for original in &changed {
                let saved = backup
                    .path()
                    .join(original.strip_prefix(root).expect("entry in store"));
                if let Err(restore_error) = fs::copy(saved, original) {
                    let saved = backup.keep();
                    return Err(std::io::Error::other(format!(
                        "re-encryption failed: {}; restore failed: {restore_error}; originals remain at {}",
                        error.error,
                        saved.display()
                    ))
                    .into());
                }
            }
            return Err(error.error.into());
        }
        changed.push(path);
    }
    Ok((changed, close_backup(backup)))
}

fn close_backup(backup: TempDir) -> Option<String> {
    let path = backup.path().to_path_buf();
    backup.close().err().map(|error| {
        format!(
            "warning: entries were re-encrypted, but old ciphertext could not be removed from {}: {error}",
            path.display()
        )
    })
}

fn prepare_entries(
    store: &PasswordStore,
    gpg: &GpgCommand,
    dir: &Path,
    recipients: &[String],
    passphrase: Option<&str>,
    staged: &mut Vec<(PathBuf, TempPath)>,
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
            prepare_entries(store, gpg, &path, recipients, passphrase, staged)?;
        } else if path.extension().is_some_and(|ext| ext == "gpg") {
            if recipients.is_empty() {
                return Err(PasswordStoreError::GpgIdNotFound);
            }
            let plaintext = gpg.decrypt(&path, passphrase)?;
            let encrypted = tempfile::NamedTempFile::new_in(path.parent().expect("entry parent"))?
                .into_temp_path();
            gpg.encrypt(&plaintext, &encrypted, recipients)?;
            staged.push((path, encrypted));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::{NamedTempFile, TempDir};

    #[cfg(unix)]
    use super::close_backup;
    use super::install_staged;

    #[test]
    fn failed_second_install_restores_first_ciphertext() {
        let root = TempDir::new().unwrap();
        let first = root.path().join("first.gpg");
        let second = root.path().join("second.gpg");
        fs::write(&first, "original first").unwrap();
        fs::write(&second, "original second").unwrap();
        let backup = TempDir::new_in(root.path()).unwrap();
        fs::copy(&first, backup.path().join("first.gpg")).unwrap();
        fs::copy(&second, backup.path().join("second.gpg")).unwrap();
        let staged = [&first, &second]
            .into_iter()
            .map(|path| {
                let temp = NamedTempFile::new_in(root.path()).unwrap().into_temp_path();
                fs::write(&temp, "new ciphertext").unwrap();
                (path.to_path_buf(), temp)
            })
            .collect();
        fs::remove_file(&second).unwrap();
        fs::create_dir(&second).unwrap();

        assert!(install_staged(root.path(), staged, backup).is_err());
        assert_eq!(fs::read_to_string(&first).unwrap(), "original first");
    }

    #[cfg(unix)]
    #[test]
    fn reports_backup_cleanup_failure_without_rolling_back_entries() {
        use std::os::unix::fs::PermissionsExt;

        let root = TempDir::new().unwrap();
        let backup = TempDir::new_in(root.path()).unwrap();
        let path = backup.path().to_path_buf();
        fs::write(path.join("old.gpg"), "old ciphertext").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o000)).unwrap();

        let warning = close_backup(backup).expect("cleanup warning");
        assert!(warning.contains(path.to_str().unwrap()));
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        assert_eq!(
            fs::read_to_string(path.join("old.gpg")).unwrap(),
            "old ciphertext"
        );
        fs::remove_dir_all(path).unwrap();
    }
}
