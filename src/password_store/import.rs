use super::{
    importer::{Importer, ImportEntry},
    EntryName, GpgCommand, InsertEntry, PasswordStore, PasswordStoreError,
};

#[derive(Debug)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
}

pub struct ImportEntries<'store, 'gpg> {
    store: &'store PasswordStore,
    gpg: &'gpg GpgCommand,
}

impl<'store, 'gpg> ImportEntries<'store, 'gpg> {
    pub fn new(store: &'store PasswordStore, gpg: &'gpg GpgCommand) -> Self {
        Self { store, gpg }
    }

    pub fn execute(
        &self,
        importer: &dyn Importer,
        data: &str,
        force: bool,
    ) -> Result<ImportResult, PasswordStoreError> {
        let entries = importer.parse(data).map_err(|error| {
            PasswordStoreError::ImportFailed(error.to_string())
        })?;

        let total = entries.len();
        let mut imported = 0;
        let mut errors = Vec::new();

        for entry in entries {
            match self.insert_import_entry(entry, force) {
                Ok(()) => imported += 1,
                Err(PasswordStoreError::EntryAlreadyExists(name)) => {
                    errors.push(format!("'{}' already exists; use --force to overwrite or resolve manually", name));
                }
                Err(e) => {
                    errors.push(e.to_string());
                }
            }
        }

        Ok(ImportResult {
            imported,
            skipped: total - imported,
            errors,
        })
    }

    fn insert_import_entry(
        &self,
        entry: ImportEntry,
        force: bool,
    ) -> Result<(), PasswordStoreError> {
        let sanitized_folder = entry.folder.as_deref().map(sanitize_path);
        let sanitized_name = sanitize_segment(&entry.name);
        let base_name = match sanitized_folder {
            Some(ref folder) => format!("{folder}/{sanitized_name}"),
            None => sanitized_name,
        };

        let name = self.resolve_entry_name(&base_name, force)?;
        let content = build_entry_content(&entry);
        InsertEntry::new(self.store, self.gpg).execute(&name, &content, true)
    }

    fn resolve_entry_name(
        &self,
        base_name: &str,
        force: bool,
    ) -> Result<String, PasswordStoreError> {
        let entry_name = EntryName::from_user_input(base_name).map_err(|error| {
            PasswordStoreError::InvalidEntryName {
                entry: base_name.to_owned(),
                reason: error.message(),
            }
        })?;
        let encrypted_file = entry_name.encrypted_file_path(self.store.path());

        if !encrypted_file.exists() || force {
            return Ok(entry_name.into_string());
        }

        for i in 1..1000 {
            let candidate = format!("{base_name}-{i}");
            let entry_name = EntryName::from_user_input(&candidate).map_err(|error| {
                PasswordStoreError::InvalidEntryName {
                    entry: candidate,
                    reason: error.message(),
                }
            })?;
            let encrypted_file = entry_name.encrypted_file_path(self.store.path());
            if !encrypted_file.exists() {
                return Ok(entry_name.into_string());
            }
        }

        Err(PasswordStoreError::EntryAlreadyExists(base_name.to_owned()))
    }
}

const INVALIDS: &[(char, &str)] = &[
    ('<', "-"), ('>', "-"), (':', "-"), ('"', "-"),
    ('/', "-"), ('\\', "-"), ('|', "-"), ('?', "-"), ('*', "-"),
    ('&', "and"), ('@', "At"),
];

fn sanitize_path(path: &str) -> String {
    path.trim()
        .split('/')
        .map(sanitize_segment)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("/")
}

fn sanitize_segment(segment: &str) -> String {
    let mut cleaned = String::with_capacity(segment.len());
    for c in segment.trim().chars() {
        match c {
            '\0' | '\t' | '\'' | '[' | ']' => {}
            _ if is_invalid(c) => {
                let replacement = INVALIDS.iter().find(|&&(ch, _)| ch == c).map(|&(_, s)| s).unwrap_or("-");
                cleaned.push_str(replacement);
            }
            _ => cleaned.push(c),
        }
    }
    cleaned
}

fn is_invalid(c: char) -> bool {
    INVALIDS.iter().any(|&(ch, _)| ch == c)
}

fn build_entry_content(entry: &ImportEntry) -> String {
    let mut lines = Vec::new();

    lines.push(entry.password.clone().unwrap_or_default());

    for field in &entry.fields {
        lines.push(format!("{}: {}", field.name, field.value));
    }

    if let Some(ref uri) = entry.otp_uri {
        lines.push(uri.clone());
    }

    if let Some(ref notes) = entry.notes {
        for line in notes.lines() {
            lines.push(line.to_owned());
        }
    }

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::{build_entry_content, sanitize_path, sanitize_segment};
    use crate::password_store::importer::ImportEntry;
    use crate::password_store::EntryField;

    #[test]
    fn segment_replaces_invalids() {
        assert_eq!(sanitize_segment("a<b>c:d\"e/f\\g|h?i*j"), "a-b-c-d-e-f-g-h-i-j");
    }

    #[test]
    fn segment_strips_control_chars() {
        assert_eq!(sanitize_segment("foo\x00bar\tbaz"), "foobarbaz");
    }

    #[test]
    fn segment_replaces_ampersand() {
        assert_eq!(sanitize_segment("foo&bar"), "fooandbar");
    }

    #[test]
    fn segment_replaces_at_sign() {
        assert_eq!(sanitize_segment("foo@bar"), "fooAtbar");
    }

    #[test]
    fn segment_strips_brackets_and_quote() {
        assert_eq!(sanitize_segment("foo'bar[baz]"), "foobarbaz");
    }

    #[test]
    fn segment_trims_whitespace() {
        assert_eq!(sanitize_segment("  foo  "), "foo");
    }

    #[test]
    fn path_preserves_separators_and_sanitizes_segments() {
        assert_eq!(sanitize_path("Social/My Bank"), "Social/My Bank");
        assert_eq!(sanitize_path("Social/My<Bank"), "Social/My-Bank");
    }

    #[test]
    fn folder_preserves_slash_name_replaces_slash() {
        let folder = sanitize_path("Social");
        let name = sanitize_segment("My/Item");
        let combined = format!("{folder}/{name}");
        assert_eq!(combined, "Social/My-Item");
    }

    #[test]
    fn builds_content_with_full_entry() {
        let entry = ImportEntry {
            name: "test".into(),
            password: Some("secret".into()),
            fields: vec![
                EntryField {
                    name: "username".into(),
                    value: "alice".into(),
                },
                EntryField {
                    name: "url".into(),
                    value: "https://example.com".into(),
                },
            ],
            otp_uri: Some("otpauth://totp/test?secret=ABC".into()),
            notes: Some("Some notes\nwith multiple lines".into()),
            folder: None,
        };

        let content = build_entry_content(&entry);
        assert_eq!(
            content,
            "secret\nusername: alice\nurl: https://example.com\notpauth://totp/test?secret=ABC\nSome notes\nwith multiple lines\n"
        );
    }

    #[test]
    fn builds_content_without_password() {
        let entry = ImportEntry {
            name: "note".into(),
            password: None,
            fields: vec![],
            otp_uri: None,
            notes: Some("Just a note".into()),
            folder: None,
        };

        let content = build_entry_content(&entry);
        assert_eq!(content, "\nJust a note\n");
    }
}
