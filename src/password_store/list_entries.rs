use std::fs;
use std::path::Path;

use super::{EntryName, PasswordStore, PasswordStoreError};

pub struct ListEntries<'store> {
    store: &'store PasswordStore,
}

impl<'store> ListEntries<'store> {
    pub fn new(store: &'store PasswordStore) -> Self {
        Self { store }
    }

    pub fn execute(&self) -> Result<Vec<String>, PasswordStoreError> {
        let mut entries = collect_entries(self.store.path(), self.store.path())?;
        entries.sort();
        Ok(entries.into_iter().map(EntryName::into_string).collect())
    }
}

fn collect_entries(
    store_root: &Path,
    directory: &Path,
) -> Result<Vec<EntryName>, PasswordStoreError> {
    let mut entries = Vec::new();

    for item in fs::read_dir(directory)? {
        let item = item?;
        let path = item.path();

        if should_skip_directory(&path) {
            continue;
        }

        if path.is_dir() {
            entries.extend(collect_entries(store_root, &path)?);
            continue;
        }

        if let Some(entry) = entry_name_from_path(store_root, &path)? {
            entries.push(entry);
        }
    }

    Ok(entries)
}

fn should_skip_directory(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == ".git")
}

fn entry_name_from_path(
    store_root: &Path,
    path: &Path,
) -> Result<Option<EntryName>, PasswordStoreError> {
    let relative_path = path
        .strip_prefix(store_root)
        .map_err(|_| PasswordStoreError::EntryOutsideStore(path.to_path_buf()))?;

    Ok(EntryName::from_store_relative_path(relative_path))
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::ListEntries;
    use crate::password_store::{PasswordStore, StoreDirectory};

    #[test]
    fn lists_nested_entries_in_deterministic_order() {
        let temp_dir = TempDir::new().expect("temp dir");
        create_file(temp_dir.path().join("z.gpg"));
        create_file(temp_dir.path().join("email").join("work.gpg"));
        create_file(temp_dir.path().join("email").join("personal.gpg"));
        create_file(temp_dir.path().join(".gpg-id"));

        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");
        let entries = ListEntries::new(&store).execute().expect("entries");

        assert_eq!(entries, vec!["email/personal", "email/work", "z"]);
    }

    #[test]
    fn ignores_git_metadata() {
        let temp_dir = TempDir::new().expect("temp dir");
        create_file(temp_dir.path().join("a.gpg"));
        create_file(
            temp_dir
                .path()
                .join(".git")
                .join("objects")
                .join("ignored.gpg"),
        );

        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");
        let entries = ListEntries::new(&store).execute().expect("entries");

        assert_eq!(entries, vec!["a"]);
    }

    fn create_file(path: impl AsRef<std::path::Path>) {
        let path = path.as_ref();
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, "").expect("file");
    }
}
