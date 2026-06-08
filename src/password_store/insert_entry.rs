use std::fs;
use std::path::{Path, PathBuf};

use super::{EntryName, GpgCommand, PasswordStore, PasswordStoreError};

pub struct InsertEntry<'store, 'gpg> {
    store: &'store PasswordStore,
    gpg: &'gpg GpgCommand,
}

impl<'store, 'gpg> InsertEntry<'store, 'gpg> {
    pub fn new(store: &'store PasswordStore, gpg: &'gpg GpgCommand) -> Self {
        Self { store, gpg }
    }

    pub fn execute(
        &self,
        entry_name: &str,
        content: &str,
        force: bool,
    ) -> Result<(), PasswordStoreError> {
        let entry_name = EntryName::from_user_input(entry_name).map_err(|error| {
            PasswordStoreError::InvalidEntryName {
                entry: entry_name.to_owned(),
                reason: error.message(),
            }
        })?;
        let encrypted_file = entry_name.encrypted_file_path(self.store.path());

        if encrypted_file.exists() && !force {
            return Err(PasswordStoreError::EntryAlreadyExists(
                entry_name.as_str().to_owned(),
            ));
        }

        let recipients = recipients_for_entry(self.store.path(), &encrypted_file)?;

        if let Some(parent) = encrypted_file.parent() {
            fs::create_dir_all(parent)?;
        }

        self.gpg.encrypt(content, &encrypted_file, &recipients)
    }
}

fn recipients_for_entry(
    store_root: &Path,
    encrypted_file: &Path,
) -> Result<Vec<String>, PasswordStoreError> {
    let mut directory = encrypted_file.parent().unwrap_or(store_root).to_path_buf();

    loop {
        let gpg_id = directory.join(".gpg-id");
        if gpg_id.exists() {
            return read_recipients(&gpg_id);
        }

        if directory == store_root {
            return Err(PasswordStoreError::GpgIdNotFound);
        }

        directory = directory
            .parent()
            .map(PathBuf::from)
            .ok_or(PasswordStoreError::GpgIdNotFound)?;
    }
}

fn read_recipients(path: &Path) -> Result<Vec<String>, PasswordStoreError> {
    let recipients = fs::read_to_string(path)?
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect::<Vec<_>>();

    if recipients.is_empty() {
        return Err(PasswordStoreError::GpgIdNotFound);
    }

    Ok(recipients)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::recipients_for_entry;
    use crate::password_store::PasswordStoreError;

    #[test]
    fn reads_nearest_recipients() {
        let store = TempDir::new().expect("store");
        fs::write(store.path().join(".gpg-id"), "root\n").expect("root gpg id");
        fs::create_dir_all(store.path().join("team")).expect("team dir");
        fs::write(store.path().join("team/.gpg-id"), "team\n").expect("team gpg id");

        let recipients = recipients_for_entry(store.path(), &store.path().join("team/app.gpg"))
            .expect("recipients");

        assert_eq!(recipients, vec!["team".to_string()]);
    }

    #[test]
    fn reports_missing_recipients() {
        let store = TempDir::new().expect("store");

        let error = recipients_for_entry(store.path(), &store.path().join("app.gpg")).unwrap_err();

        assert!(matches!(error, PasswordStoreError::GpgIdNotFound));
    }
}
