use std::env;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct StoreDirectory {
    path: PathBuf,
}

impl StoreDirectory {
    pub fn resolve(explicit_path: Option<PathBuf>) -> Result<Self, PasswordStoreError> {
        let path = resolve_store_path(
            explicit_path,
            password_store_dir_from_environment(),
            default_password_store_dir(),
        )?;

        Ok(Self { path })
    }

    #[cfg(test)]
    pub fn from_path(path: impl AsRef<Path>) -> Self {
        Self {
            path: path.as_ref().to_path_buf(),
        }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn into_path_buf(self) -> PathBuf {
        self.path
    }
}

#[derive(Debug, Clone)]
pub struct PasswordStore {
    path: PathBuf,
}

impl PasswordStore {
    pub fn open(directory: StoreDirectory) -> Result<Self, PasswordStoreError> {
        let path = directory.into_path_buf();

        if !path.exists() {
            return Err(PasswordStoreError::StoreNotFound(path));
        }

        if !path.is_dir() {
            return Err(PasswordStoreError::StoreIsNotDirectory(path));
        }

        Ok(Self { path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[derive(Debug, thiserror::Error)]
pub enum PasswordStoreError {
    #[error("password store does not exist: {0}")]
    StoreNotFound(PathBuf),

    #[error("password store path is not a directory: {0}")]
    StoreIsNotDirectory(PathBuf),

    #[error("home directory is unavailable")]
    HomeDirectoryUnavailable,

    #[error("entry path is outside the password store: {0}")]
    EntryOutsideStore(PathBuf),

    #[error("entry already exists: {0}")]
    EntryAlreadyExists(String),

    #[error("entry does not exist: {0}")]
    EntryNotFound(String),

    #[error("invalid entry name '{entry}': {reason}")]
    InvalidEntryName { entry: String, reason: &'static str },

    #[error("gpg executable was not found; install GnuPG 2.x or set PASSWORD_STORE_GPG")]
    GpgNotFound,

    #[error("gpg requires a passphrase; use --passphrase to provide it")]
    GpgPassphraseRequired,

    #[error("no .gpg-id file found for entry")]
    GpgIdNotFound,

    #[error("gpg failed to decrypt entry: {0}")]
    GpgDecryptFailed(String),

    #[error("gpg failed to encrypt entry: {0}")]
    GpgEncryptFailed(String),

    #[error("gpg decrypted entry was empty")]
    GpgEmptyOutput,

    #[error("gpg output was not valid UTF-8: {0}")]
    GpgOutputNotUtf8(#[from] std::string::FromUtf8Error),

    #[error("gpg failed to report its version: {0}")]
    GpgVersionFailed(String),

    #[error("entry does not contain an otpauth URI")]
    OtpNotFound,

    #[error("entry contains an invalid otpauth URI")]
    InvalidOtpUri,

    #[error("failed to access password store: {0}")]
    Io(#[from] std::io::Error),
}

impl PasswordStoreError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::StoreNotFound(_) => "store_not_found",
            Self::StoreIsNotDirectory(_) => "store_is_not_directory",
            Self::HomeDirectoryUnavailable => "home_directory_unavailable",
            Self::EntryOutsideStore(_) => "entry_outside_store",
            Self::EntryAlreadyExists(_) => "entry_already_exists",
            Self::EntryNotFound(_) => "entry_not_found",
            Self::InvalidEntryName { .. } => "invalid_entry_name",
            Self::GpgNotFound => "gpg_not_found",
            Self::GpgPassphraseRequired => "gpg_passphrase_required",
            Self::GpgIdNotFound => "gpg_id_not_found",
            Self::GpgDecryptFailed(_) => "gpg_decrypt_failed",
            Self::GpgEncryptFailed(_) => "gpg_encrypt_failed",
            Self::GpgEmptyOutput => "gpg_empty_output",
            Self::GpgOutputNotUtf8(_) => "gpg_output_not_utf8",
            Self::GpgVersionFailed(_) => "gpg_version_failed",
            Self::OtpNotFound => "otp_not_found",
            Self::InvalidOtpUri => "invalid_otp_uri",
            Self::Io(_) => "io_error",
        }
    }
}

fn password_store_dir_from_environment() -> Option<PathBuf> {
    env::var_os("PASSWORD_STORE_DIR")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn default_password_store_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|home_dir| home_dir.join(".password-store"))
}

fn resolve_store_path(
    explicit_path: Option<PathBuf>,
    environment_path: Option<PathBuf>,
    default_path: Option<PathBuf>,
) -> Result<PathBuf, PasswordStoreError> {
    explicit_path
        .or(environment_path)
        .or(default_path)
        .ok_or(PasswordStoreError::HomeDirectoryUnavailable)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::{PasswordStore, PasswordStoreError, StoreDirectory, resolve_store_path};

    #[test]
    fn opens_existing_store_directory() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store = PasswordStore::open(StoreDirectory::from_path(temp_dir.path()));

        assert!(store.is_ok());
    }

    #[test]
    fn rejects_missing_store_directory() {
        let temp_dir = TempDir::new().expect("temp dir");
        let missing_store = temp_dir.path().join("missing");
        let error = PasswordStore::open(StoreDirectory::from_path(&missing_store)).unwrap_err();

        assert!(matches!(error, PasswordStoreError::StoreNotFound(path) if path == missing_store));
    }

    #[test]
    fn rejects_file_as_store_directory() {
        let temp_dir = TempDir::new().expect("temp dir");
        let store_file = temp_dir.path().join("store-file");
        fs::write(&store_file, "").expect("store file");

        let error = PasswordStore::open(StoreDirectory::from_path(&store_file)).unwrap_err();

        assert!(
            matches!(error, PasswordStoreError::StoreIsNotDirectory(path) if path == store_file)
        );
    }

    #[test]
    fn explicit_path_wins_over_environment() {
        let temp_dir = TempDir::new().expect("temp dir");
        let explicit_store = temp_dir.path().join("explicit");
        let env_store = temp_dir.path().join("environment");

        let resolved =
            resolve_store_path(Some(explicit_store.clone()), Some(env_store), None).expect("path");

        assert_eq!(resolved, explicit_store);
    }

    #[test]
    fn environment_path_wins_over_default_path() {
        let env_store = PathBuf::from("environment");
        let default_store = PathBuf::from("default");

        let resolved =
            resolve_store_path(None, Some(env_store.clone()), Some(default_store)).expect("path");

        assert_eq!(resolved, env_store);
    }
}
