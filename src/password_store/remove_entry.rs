use std::fs;
use std::path::{Path, PathBuf};

use super::{EntryName, PasswordStore, PasswordStoreError};

pub struct RemoveEntry<'store> {
    store: &'store PasswordStore,
}

impl<'store> RemoveEntry<'store> {
    pub fn new(store: &'store PasswordStore) -> Self {
        Self { store }
    }

    pub fn execute(&self, entry_name: &str) -> Result<(), PasswordStoreError> {
        let entry_name = EntryName::from_user_input(entry_name).map_err(|error| {
            PasswordStoreError::InvalidEntryName {
                entry: entry_name.to_owned(),
                reason: error.message(),
            }
        })?;
        let encrypted_file = entry_name.encrypted_file_path(self.store.path());

        if !encrypted_file.exists() {
            return Err(PasswordStoreError::EntryNotFound(entry_name.into_string()));
        }

        fs::remove_file(&encrypted_file)?;
        prune_empty_parent_directories(self.store.path(), encrypted_file.parent());

        Ok(())
    }
}

fn prune_empty_parent_directories(store_root: &Path, start: Option<&Path>) {
    let Some(mut directory) = start.map(PathBuf::from) else {
        return;
    };

    while directory != store_root {
        if fs::remove_dir(&directory).is_err() {
            break;
        }

        let Some(parent) = directory.parent() else {
            break;
        };
        directory = parent.to_path_buf();
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::RemoveEntry;
    use crate::password_store::{PasswordStore, PasswordStoreError, StoreDirectory};

    #[test]
    fn removes_entry_file() {
        let temp_dir = TempDir::new().expect("temp dir");
        create_file(temp_dir.path().join("example/login.gpg"));
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");

        RemoveEntry::new(&store)
            .execute("example/login")
            .expect("remove");

        assert!(!temp_dir.path().join("example/login.gpg").exists());
    }

    #[test]
    fn reports_missing_entry() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");

        let error = RemoveEntry::new(&store).execute("missing").unwrap_err();

        assert!(matches!(error, PasswordStoreError::EntryNotFound(entry) if entry == "missing"));
    }

    fn create_file(path: impl AsRef<Path>) {
        let path = path.as_ref();
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, "").expect("file");
    }
}
