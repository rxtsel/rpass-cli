use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct EntryName(String);

impl EntryName {
    pub fn from_store_relative_path(path: &Path) -> Option<Self> {
        let path_without_extension = path.strip_suffix(".gpg")?;
        let name = normalize_entry_path(&path_without_extension);

        is_valid_entry_name(&name).then_some(Self(name))
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

fn normalize_entry_path(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn is_valid_entry_name(name: &str) -> bool {
    !name.is_empty() && !name.contains('\\')
}

trait PathGpgExtension {
    fn strip_suffix(&self, suffix: &str) -> Option<PathBuf>;
}

impl PathGpgExtension for Path {
    fn strip_suffix(&self, suffix: &str) -> Option<PathBuf> {
        let file_name = self.file_name()?.to_string_lossy();
        let entry_file_name = file_name.strip_suffix(suffix)?;
        let mut entry_path = self.parent().unwrap_or_else(|| Path::new("")).to_path_buf();
        entry_path.push(entry_file_name);
        Some(entry_path)
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::EntryName;

    #[test]
    fn builds_entry_name_from_nested_gpg_path() {
        let entry = EntryName::from_store_relative_path(Path::new("email/work.gpg"));

        assert_eq!(
            entry.map(EntryName::into_string),
            Some("email/work".to_string())
        );
    }

    #[test]
    fn ignores_paths_without_gpg_extension() {
        let entry = EntryName::from_store_relative_path(Path::new(".gpg-id"));

        assert_eq!(entry, None);
    }
}
