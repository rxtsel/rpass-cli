use super::{DecryptedEntry, EntryName, GpgCommand, PasswordStore, PasswordStoreError};

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ShowEntryOutput {
    pub content: String,
    pub parsed: DecryptedEntry,
}

pub struct ShowEntry<'store, 'gpg> {
    store: &'store PasswordStore,
    gpg: &'gpg GpgCommand,
}

impl<'store, 'gpg> ShowEntry<'store, 'gpg> {
    pub fn new(store: &'store PasswordStore, gpg: &'gpg GpgCommand) -> Self {
        Self { store, gpg }
    }

    pub fn execute(
        &self,
        entry_name: &str,
        passphrase: Option<&str>,
    ) -> Result<ShowEntryOutput, PasswordStoreError> {
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

        let content = self.gpg.decrypt(&encrypted_file, passphrase)?;
        let parsed = DecryptedEntry::parse(&content);

        Ok(ShowEntryOutput { content, parsed })
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::ShowEntry;
    use crate::password_store::{
        DecryptedEntry, EntryField, GpgCommand, PasswordStore, PasswordStoreError, StoreDirectory,
    };

    #[test]
    fn rejects_invalid_entry_names_before_decrypting() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");
        let gpg = GpgCommand::new("missing-gpg");

        let error = ShowEntry::new(&store, &gpg)
            .execute("../outside", None)
            .unwrap_err();

        assert!(matches!(
            error,
            PasswordStoreError::InvalidEntryName { entry, .. } if entry == "../outside"
        ));
    }

    #[test]
    fn reports_missing_entry() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");
        let gpg = GpgCommand::new("missing-gpg");

        let error = ShowEntry::new(&store, &gpg)
            .execute("missing", None)
            .unwrap_err();

        assert!(matches!(error, PasswordStoreError::EntryNotFound(entry) if entry == "missing"));
    }

    #[test]
    fn decrypts_and_parses_existing_entry() {
        let temp_dir = TempDir::new().expect("temp dir");
        let encrypted_entry = temp_dir.path().join("email").join("work.gpg");
        create_file(&encrypted_entry);
        let gpg = fake_gpg_script(temp_dir.path(), "secret\nusername: alice\n");
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path())).expect("store");

        let output = ShowEntry::new(&store, &GpgCommand::new(gpg))
            .execute("email/work", None)
            .expect("entry");

        assert_eq!(
            output.parsed,
            DecryptedEntry {
                password: "secret".to_string(),
                fields: vec![EntryField {
                    name: "username".to_string(),
                    value: "alice".to_string()
                }],
                otp_uri: None,
                extra_lines: Vec::new(),
            }
        );
        assert_eq!(output.content, "secret\nusername: alice\n");
    }

    fn create_file(path: &std::path::Path) {
        fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
        fs::write(path, "").expect("file");
    }

    #[cfg(windows)]
    fn fake_gpg_script(directory: &std::path::Path, output: &str) -> std::path::PathBuf {
        let script = directory.join("gpg.cmd");
        let output_file = directory.join("gpg-output.txt");

        fs::write(&output_file, output).expect("output file");
        fs::write(
            &script,
            format!("@echo off\r\ntype \"{}\"\r\n", output_file.display()),
        )
        .expect("script");
        script
    }

    #[cfg(not(windows))]
    fn fake_gpg_script(directory: &std::path::Path, output: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script = directory.join("gpg");
        fs::write(&script, format!("#!/bin/sh\nprintf '{}'\n", output)).expect("script");
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("permissions");
        script
    }
}
