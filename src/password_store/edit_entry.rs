use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use tempfile::NamedTempFile;

use super::insert_entry::recipients_for_entry;
use super::{EntryName, GpgCommand, PasswordStore, PasswordStoreError};

pub struct EditEntry<'store, 'gpg> {
    store: &'store PasswordStore,
    gpg: &'gpg GpgCommand,
}

impl<'store, 'gpg> EditEntry<'store, 'gpg> {
    pub fn new(store: &'store PasswordStore, gpg: &'gpg GpgCommand) -> Self {
        Self { store, gpg }
    }

    pub fn execute(&self, entry_name: &str) -> Result<(), PasswordStoreError> {
        let entry_name = EntryName::from_user_input(entry_name).map_err(|error| {
            PasswordStoreError::InvalidEntryName {
                entry: entry_name.to_owned(),
                reason: error.message(),
            }
        })?;
        let encrypted_file = entry_name.encrypted_file_path(self.store.path());
        let content = if encrypted_file.exists() {
            self.gpg.decrypt(&encrypted_file, None)?
        } else {
            String::new()
        };
        let edited_content = edit_content(&content)?;
        let recipients = recipients_for_entry(self.store.path(), &encrypted_file)?;

        if let Some(parent) = encrypted_file.parent() {
            fs::create_dir_all(parent)?;
        }

        self.gpg
            .encrypt(&edited_content, &encrypted_file, &recipients)
    }
}

fn edit_content(content: &str) -> Result<String, PasswordStoreError> {
    let temp_file = NamedTempFile::new()?;
    fs::write(temp_file.path(), content)?;
    run_editor(temp_file.path())?;
    fs::read_to_string(temp_file.path()).map_err(PasswordStoreError::Io)
}

fn run_editor(path: &Path) -> Result<(), PasswordStoreError> {
    let editor = env::var("EDITOR").unwrap_or_else(|_| default_editor().to_string());
    let status = editor_command(&editor, path)
        .status()
        .map_err(|error| PasswordStoreError::EditorFailed(error.to_string()))?;

    if status.success() {
        Ok(())
    } else {
        Err(PasswordStoreError::EditorFailed(format!(
            "editor exited with {status}"
        )))
    }
}

#[cfg(windows)]
fn editor_command(editor: &str, path: &Path) -> Command {
    let mut command = Command::new("cmd");
    command
        .arg("/C")
        .arg(format!("{editor} \"{}\"", path.display()));
    command
}

#[cfg(not(windows))]
fn editor_command(editor: &str, path: &Path) -> Command {
    let mut command = Command::new("sh");
    command
        .arg("-c")
        .arg(format!("{editor} \"$1\""))
        .arg("rpass-editor")
        .arg(path);
    command
}

#[cfg(windows)]
fn default_editor() -> &'static str {
    "notepad"
}

#[cfg(not(windows))]
fn default_editor() -> &'static str {
    "vi"
}
