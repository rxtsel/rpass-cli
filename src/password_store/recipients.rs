use std::fs;
use std::path::{Path, PathBuf};

use super::{PasswordStore, PasswordStoreError};

#[derive(Debug)]
pub struct RecipientsResult {
    pub gpg_id_path: PathBuf,
    pub recipients: Vec<String>,
    pub changed: bool,
}

pub struct Recipients<'store> {
    store: &'store PasswordStore,
}

impl<'store> Recipients<'store> {
    pub fn new(store: &'store PasswordStore) -> Self {
        Self { store }
    }

    pub fn list(&self, subfolder: Option<&str>) -> Result<RecipientsResult, PasswordStoreError> {
        let gpg_id_path = gpg_id_path(self.store.path(), subfolder);
        let recipients = read_recipients(&gpg_id_path)?;

        Ok(RecipientsResult {
            gpg_id_path: relative_path(self.store.path(), &gpg_id_path),
            recipients,
            changed: false,
        })
    }

    pub fn add(
        &self,
        subfolder: Option<&str>,
        recipient: &str,
    ) -> Result<RecipientsResult, PasswordStoreError> {
        let gpg_id_path = gpg_id_path(self.store.path(), subfolder);
        let mut recipients = read_recipients(&gpg_id_path)?;
        let changed = !recipients.iter().any(|existing| existing == recipient);

        if changed {
            recipients.push(recipient.to_owned());
            write_recipients(&gpg_id_path, &recipients)?;
        }

        Ok(RecipientsResult {
            gpg_id_path: relative_path(self.store.path(), &gpg_id_path),
            recipients,
            changed,
        })
    }

    pub fn remove(
        &self,
        subfolder: Option<&str>,
        recipient: &str,
    ) -> Result<RecipientsResult, PasswordStoreError> {
        let gpg_id_path = gpg_id_path(self.store.path(), subfolder);
        let mut recipients = read_recipients(&gpg_id_path)?;
        let original_len = recipients.len();
        recipients.retain(|existing| existing != recipient);

        if recipients.len() == original_len {
            return Err(PasswordStoreError::RecipientNotFound(recipient.to_owned()));
        }

        write_recipients(&gpg_id_path, &recipients)?;

        Ok(RecipientsResult {
            gpg_id_path: relative_path(self.store.path(), &gpg_id_path),
            recipients,
            changed: true,
        })
    }
}

fn gpg_id_path(store_root: &Path, subfolder: Option<&str>) -> PathBuf {
    match subfolder {
        Some(subfolder) => store_root.join(subfolder).join(".gpg-id"),
        None => store_root.join(".gpg-id"),
    }
}

fn read_recipients(path: &Path) -> Result<Vec<String>, PasswordStoreError> {
    if !path.exists() {
        return Err(PasswordStoreError::GpgIdNotFound);
    }

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

fn write_recipients(path: &Path, recipients: &[String]) -> Result<(), PasswordStoreError> {
    fs::write(path, recipients.join("\n") + "\n")?;
    Ok(())
}

fn relative_path(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}
