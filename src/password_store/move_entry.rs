use std::fs;
use std::path::{Path, PathBuf};

use super::insert_entry::recipients_for_entry;
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
        self.store.check_path(source.path())?;
        self.store.check_path(&destination)?;
        if source.path() == destination
            || matches!(source, MoveSource::Directory(_))
                && (source.path().starts_with(&destination)
                    || destination.starts_with(source.path()))
        {
            return Err(PasswordStoreError::InvalidMove);
        }
        let destination_recipients = if source.path().is_dir() {
            destination
                .parent()
                .expect("validated entry path")
                .join("entry.gpg")
        } else {
            destination.clone()
        };
        ensure_recipients_unchanged(
            self.store.path(),
            source.path(),
            &destination,
            &destination_recipients,
            false,
        )?;

        let same_file =
            destination.exists() && source.path().canonicalize()? == destination.canonicalize()?;
        if same_file && source.path().parent() != destination.parent() {
            return Err(PasswordStoreError::InvalidMove);
        }
        if destination.exists() {
            if !force && !same_file {
                return Err(PasswordStoreError::EntryAlreadyExists(
                    new_entry_name.as_str().to_owned(),
                ));
            }
            if source.path().is_dir() != destination.is_dir() {
                return Err(PasswordStoreError::InvalidMove);
            }
        }

        let parent = destination.parent().expect("validated entry path");
        fs::create_dir_all(parent)?;
        if destination.exists() {
            let backup = tempfile::Builder::new()
                .prefix(".rpass-backup-")
                .tempdir_in(parent)?;
            let previous = backup.path().join("previous");
            fs::rename(&destination, &previous)?;
            let moving = if same_file { &previous } else { source.path() };
            if let Err(error) = fs::rename(moving, &destination) {
                let restore_to = if same_file {
                    source.path()
                } else {
                    &destination
                };
                if let Err(restore_error) = fs::rename(&previous, restore_to) {
                    let saved = backup.keep();
                    return Err(std::io::Error::other(format!(
                        "move failed: {error}; restore failed: {restore_error}; old destination saved at {}",
                        saved.display()
                    ))
                    .into());
                }
                return Err(error.into());
            }
            let backup_path = backup.path().to_path_buf();
            backup.close().map_err(|error| {
                std::io::Error::other(format!(
                    "old destination could not be removed from {}: {error}",
                    backup_path.display()
                ))
            })?;
        } else {
            fs::rename(source.path(), &destination)?;
        }
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

fn ensure_recipients_unchanged(
    store_root: &Path,
    source: &Path,
    destination: &Path,
    destination_recipients: &Path,
    has_moved_id: bool,
) -> Result<(), PasswordStoreError> {
    if source.is_dir() {
        let gpg_id = source.join(".gpg-id");
        super::store_directory::check_store_path(store_root, &gpg_id)?;
        let has_moved_id = has_moved_id || gpg_id.exists();
        for item in fs::read_dir(source)? {
            let item = item?;
            let path = item.path();
            super::store_directory::check_store_path(store_root, &path)?;
            if path.is_dir() {
                ensure_recipients_unchanged(
                    store_root,
                    &path,
                    &destination.join(path.strip_prefix(source).expect("child path")),
                    destination_recipients,
                    has_moved_id,
                )?;
            } else if !has_moved_id && path.extension().is_some_and(|ext| ext == "gpg") {
                check_recipients(store_root, &path, destination_recipients)?;
            }
        }
    } else {
        check_recipients(store_root, source, destination_recipients)?;
    }
    Ok(())
}

fn check_recipients(
    store_root: &Path,
    source: &Path,
    destination: &Path,
) -> Result<(), PasswordStoreError> {
    fn recipients(root: &Path, entry: &Path) -> Result<Option<Vec<String>>, PasswordStoreError> {
        match recipients_for_entry(root, entry) {
            Ok(mut recipients) => {
                recipients.sort();
                Ok(Some(recipients))
            }
            Err(PasswordStoreError::GpgIdNotFound) => Ok(None),
            Err(error) => Err(error),
        }
    }
    if recipients(store_root, source)? != recipients(store_root, destination)? {
        return Err(PasswordStoreError::RecipientMismatch);
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
