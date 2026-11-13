use std::fs;
use std::path::{Path, PathBuf};

use super::{PasswordStoreError, StoreDirectory};

#[derive(Debug)]
pub struct InitStoreResult {
    pub gpg_id_path: PathBuf,
    pub recipients: Vec<String>,
    pub removed: bool,
}

pub struct InitStore {
    store_directory: StoreDirectory,
}

impl InitStore {
    pub fn new(store_directory: StoreDirectory) -> Self {
        Self { store_directory }
    }

    pub fn execute(
        &self,
        subfolder: Option<&str>,
        recipients: &[String],
    ) -> Result<InitStoreResult, PasswordStoreError> {
        let removed = recipients.len() == 1 && recipients.first().is_some_and(String::is_empty);
        if !removed {
            if recipients.is_empty() {
                return Err(PasswordStoreError::InvalidRecipient);
            }
            for recipient in recipients {
                super::recipients::validate_recipient(recipient)?;
            }
        }
        let store_root = self.store_directory.path();
        fs::create_dir_all(store_root)?;

        let target_directory = match subfolder {
            Some(subfolder) => store_root.join(subfolder),
            None => store_root.to_path_buf(),
        };
        let gpg_id_path = target_directory.join(".gpg-id");
        super::store_directory::check_store_path(store_root, &gpg_id_path)?;
        fs::create_dir_all(&target_directory)?;
        if removed {
            remove_gpg_id_if_present(&gpg_id_path)?;
        } else {
            super::store_directory::atomic_write(
                &gpg_id_path,
                (recipients.join("\n") + "\n").as_bytes(),
            )?;
        }

        Ok(InitStoreResult {
            gpg_id_path: relative_path(store_root, &gpg_id_path),
            recipients: if removed {
                Vec::new()
            } else {
                recipients.to_vec()
            },
            removed,
        })
    }
}

fn remove_gpg_id_if_present(path: &Path) -> Result<(), PasswordStoreError> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
