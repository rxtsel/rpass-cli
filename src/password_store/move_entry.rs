use std::fs;
use std::path::{Path, PathBuf};

use super::{EntryName, PasswordStore, PasswordStoreError};
use crate::password_store::remove_entry::prune_empty_parent_directories;

pub struct MoveEntry<'store> {
    store: &'store PasswordStore,
}

impl<'store> MoveEntry<'store> {
    pub fn new(store: &'store PasswordStore) -> Self {
        Self { store }
    }

    pub fn execute(
        &self,
        old_entry_name: &str,
        new_entry_name: &str,
        force: bool,
    ) -> Result<(), PasswordStoreError> {
        let old_entry_name = parse_entry_name(old_entry_name)?;
        let new_entry_name = parse_entry_name(new_entry_name)?;
        let source = move_source(self.store.path(), &old_entry_name)?;
        let destination = destination_for_source(self.store.path(), &new_entry_name, &source);

        if destination.exists() {
            if !force {
                return Err(PasswordStoreError::EntryAlreadyExists(
                    new_entry_name.as_str().to_owned(),
                ));
            }

            remove_destination(&destination)?;
        }

        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::rename(source.path(), &destination)?;
        prune_empty_parent_directories(self.store.path(), source.path().parent());

        Ok(())
    }
}

fn parse_entry_name(entry_name: &str) -> Result<EntryName, PasswordStoreError> {
    EntryName::from_user_input(entry_name).map_err(|error| PasswordStoreError::InvalidEntryName {
        entry: entry_name.to_owned(),
        reason: error.message(),
    })
}

fn move_source(
    store_root: &Path,
    entry_name: &EntryName,
) -> Result<MoveSource, PasswordStoreError> {
    let entry_file = entry_name.encrypted_file_path(store_root);
    if entry_file.exists() {
        return Ok(MoveSource::File(entry_file));
    }

    let entry_directory = entry_name.directory_path(store_root);
    if entry_directory.is_dir() {
        return Ok(MoveSource::Directory(entry_directory));
    }

    Err(PasswordStoreError::EntryNotFound(
        entry_name.as_str().to_owned(),
    ))
}

fn destination_for_source(
    store_root: &Path,
    entry_name: &EntryName,
    source: &MoveSource,
) -> PathBuf {
    match source {
        MoveSource::File(_) => entry_name.encrypted_file_path(store_root),
        MoveSource::Directory(_) => entry_name.directory_path(store_root),
    }
}

fn remove_destination(destination: &Path) -> Result<(), PasswordStoreError> {
    if destination.is_dir() {
        fs::remove_dir_all(destination)?;
    } else {
        fs::remove_file(destination)?;
    }

    Ok(())
}

enum MoveSource {
    File(PathBuf),
    Directory(PathBuf),
}

impl MoveSource {
    fn path(&self) -> &Path {
        match self {
            Self::File(path) | Self::Directory(path) => path,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use tempfile::TempDir;

    use super::MoveEntry;
    use crate::password_store::{PasswordStore, PasswordStoreError, StoreDirectory};

    #[test]
    fn moves_entry_file() {
        let temp_dir = TempDir::new().expect("temp dir");
        create_file(temp_dir.path().join("old.gpg"), "old\n");
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");

        MoveEntry::new(&store)
            .execute("old", "new", false)
            .expect("move");

        assert!(!temp_dir.path().join("old.gpg").exists());
        assert_eq!(
            fs::read_to_string(temp_dir.path().join("new.gpg")).expect("destination"),
            "old\n"
        );
    }

    #[test]
    fn reports_missing_source() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");

        let error = MoveEntry::new(&store)
            .execute("missing", "new", false)
            .unwrap_err();

        assert!(matches!(error, PasswordStoreError::EntryNotFound(entry) if entry == "missing"));
    }

    fn create_file(path: impl AsRef<Path>, content: &str) {
        let path = path.as_ref();
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, content).expect("file");
    }
}
