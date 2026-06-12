use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct EntryName(String);

impl EntryName {
    pub fn from_user_input(input: &str) -> Result<Self, EntryNameError> {
        if input.is_empty() {
            return Err(EntryNameError::Empty);
        }

        if input.contains('\\') {
            return Err(EntryNameError::BackslashSeparator);
        }

        if input.ends_with(".gpg") {
            return Err(EntryNameError::GpgExtension);
        }

        let segments = input.split('/').collect::<Vec<_>>();

        if segments.iter().any(|segment| segment.is_empty()) {
            return Err(EntryNameError::EmptySegment);
        }

        if segments
            .iter()
            .any(|segment| matches!(*segment, "." | ".."))
        {
            return Err(EntryNameError::PathTraversal);
        }

        Ok(Self(input.to_owned()))
    }

    pub fn from_store_relative_path(path: &Path) -> Option<Self> {
        let path_without_extension = path.strip_suffix(".gpg")?;
        let name = normalize_entry_path(&path_without_extension);

        is_valid_entry_name(&name).then_some(Self(name))
    }

    pub fn encrypted_file_path(&self, store_root: &Path) -> PathBuf {
        let mut path = self.directory_path(store_root);
        let file_name = format!(
            "{}.gpg",
            path.file_name()
                .expect("validated entry segment")
                .to_string_lossy()
        );
        path.set_file_name(file_name);
        path
    }

    pub fn directory_path(&self, store_root: &Path) -> PathBuf {
        let mut path = store_root.to_path_buf();

        for segment in self.0.split('/') {
            path.push(segment);
        }

        path
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn into_string(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum EntryNameError {
    Empty,
    EmptySegment,
    BackslashSeparator,
    GpgExtension,
    PathTraversal,
}

impl EntryNameError {
    pub fn message(&self) -> &'static str {
        match self {
            Self::Empty => "entry name cannot be empty",
            Self::EmptySegment => "entry name cannot contain empty path segments",
            Self::BackslashSeparator => "entry name must use '/' as the path separator",
            Self::GpgExtension => "entry name must not include the .gpg extension",
            Self::PathTraversal => "entry name cannot contain '.' or '..' path segments",
        }
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

    #[test]
    fn builds_encrypted_file_path_from_entry_name() {
        let entry = EntryName::from_user_input("email/work").expect("entry");

        assert_eq!(
            entry.encrypted_file_path(Path::new("store")),
            Path::new("store").join("email").join("work.gpg")
        );
    }

    #[test]
    fn appends_gpg_extension_to_entry_names_with_dots() {
        let entry = EntryName::from_user_input("dev/expo.dev").expect("entry");

        assert_eq!(
            entry.encrypted_file_path(Path::new("store")),
            Path::new("store").join("dev").join("expo.dev.gpg")
        );
    }

    #[test]
    fn rejects_path_traversal_entry_name() {
        let error = EntryName::from_user_input("../outside").unwrap_err();

        assert_eq!(error, super::EntryNameError::PathTraversal);
    }

    #[test]
    fn rejects_windows_separator_entry_name() {
        let error = EntryName::from_user_input("email\\work").unwrap_err();

        assert_eq!(error, super::EntryNameError::BackslashSeparator);
    }
}
