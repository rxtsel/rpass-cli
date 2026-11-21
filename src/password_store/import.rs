use std::collections::HashSet;
use std::path::PathBuf;

use super::{
    EntryName, GpgCommand, InsertEntry, ListEntries, PasswordStore, PasswordStoreError,
    importer::{ImportEntry, ImportError, Importer},
};

#[derive(Debug)]
pub struct ImportResult {
    pub imported: usize,
    pub skipped: usize,
    pub errors: Vec<String>,
    pub written: Vec<PathBuf>,
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
        let entries = importer
            .parse(data)
            .map_err(|error| PasswordStoreError::ImportFailed(error.to_string()))?;
        for (index, entry) in entries.iter().enumerate() {
            if EntryName::from_user_input(&sanitized_entry_name(entry)).is_err() {
                return Err(PasswordStoreError::ImportFailed(
                    ImportError::InvalidItemName(index + 1).to_string(),
                ));
            }
        }

        let total = entries.len();
        let existing = ListEntries::new(self.store)
            .execute()?
            .into_iter()
            .collect::<HashSet<_>>();
        let mut imported = 0;
        let mut errors = Vec::new();
        let mut written = Vec::new();
        let mut imported_names = HashSet::new();

        for entry in entries {
            match self.insert_import_entry(entry, force, &existing, &imported_names) {
                Ok(name) => {
                    imported += 1;
                    written.push(PathBuf::from(format!("{name}.gpg")));
                    // shortcut: Unicode case folding can differ from the filesystem; use file identity if imports need those names.
                    imported_names.insert(name.to_lowercase());
                }
                Err(PasswordStoreError::EntryAlreadyExists(name)) => {
                    let advice = if force {
                        "resolve manually"
                    } else {
                        "use --force to overwrite or resolve manually"
                    };
                    errors.push(format!("'{name}' already exists; {advice}"));
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
            written,
        })
    }

    fn insert_import_entry(
        &self,
        entry: ImportEntry,
        force: bool,
        existing: &HashSet<String>,
        imported_names: &HashSet<String>,
    ) -> Result<String, PasswordStoreError> {
        let base_name = sanitized_entry_name(&entry);

        if !force && existing.contains(&base_name) {
            return Err(PasswordStoreError::EntryAlreadyExists(base_name));
        }
        let name = self.resolve_entry_name(&base_name, force, imported_names)?;
        let content = build_entry_content(&entry);
        InsertEntry::new(self.store, self.gpg).execute(&name, &content, true)?;
        Ok(name)
    }

    fn resolve_entry_name(
        &self,
        base_name: &str,
        force: bool,
        imported_names: &HashSet<String>,
    ) -> Result<String, PasswordStoreError> {
        let entry_name = EntryName::from_user_input(base_name).map_err(|error| {
            PasswordStoreError::InvalidEntryName {
                entry: base_name.to_owned(),
                reason: error.message(),
            }
        })?;
        let encrypted_file = entry_name.encrypted_file_path(self.store.path());

        if !imported_names.contains(&base_name.to_lowercase())
            && (!encrypted_file.exists() || force)
        {
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
            if imported_names.contains(&entry_name.as_str().to_lowercase()) {
                continue;
            }
            let encrypted_file = entry_name.encrypted_file_path(self.store.path());
            if encrypted_file.exists() {
                if force {
                    return Err(PasswordStoreError::EntryAlreadyExists(
                        entry_name.into_string(),
                    ));
                }
                continue;
            }
            return Ok(entry_name.into_string());
        }

        Err(PasswordStoreError::EntryAlreadyExists(base_name.to_owned()))
    }
}

fn sanitized_entry_name(entry: &ImportEntry) -> String {
    let name = sanitize_segment(&entry.name);
    match entry.folder.as_deref().map(sanitize_path) {
        Some(folder) => format!("{folder}/{name}"),
        None => name,
    }
}

const INVALIDS: &[(char, &str)] = &[
    ('<', "-"),
    ('>', "-"),
    (':', "-"),
    ('"', "-"),
    ('/', "-"),
    ('\\', "-"),
    ('|', "-"),
    ('?', "-"),
    ('*', "-"),
    ('&', "and"),
    ('@', "At"),
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
            _ if c.is_control() => cleaned.push('-'),
            _ if is_invalid(c) => {
                let replacement = INVALIDS
                    .iter()
                    .find(|&&(ch, _)| ch == c)
                    .map(|&(_, s)| s)
                    .unwrap_or("-");
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
            lines.push(format!("# rpass-note {line}"));
        }
    }

    lines.join("\n") + "\n"
}

#[cfg(test)]
mod tests {
    use super::{build_entry_content, sanitize_path, sanitize_segment};
    use crate::password_store::EntryField;
    use crate::password_store::importer::ImportEntry;

    #[test]
    fn segment_replaces_invalids() {
        assert_eq!(
            sanitize_segment("a<b>c:d\"e/f\\g|h?i*j"),
            "a-b-c-d-e-f-g-h-i-j"
        );
    }

    #[test]
    fn segment_strips_control_chars() {
        assert_eq!(sanitize_segment("foo\x00bar\tbaz"), "foobarbaz");
    }

    #[test]
    fn segment_replaces_line_breaks() {
        assert_eq!(sanitize_segment("foo\nbar\rbaz"), "foo-bar-baz");
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
            "secret\nusername: alice\nurl: https://example.com\notpauth://totp/test?secret=ABC\n# rpass-note Some notes\n# rpass-note with multiple lines\n"
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
        assert_eq!(content, "\n# rpass-note Just a note\n");
    }
}
