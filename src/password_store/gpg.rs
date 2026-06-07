use std::env;
use std::ffi::OsString;
use std::path::Path;
use std::process::Command;

use super::PasswordStoreError;

#[derive(Debug, Clone)]
pub struct GpgCommand {
    program: OsString,
}

impl GpgCommand {
    pub fn from_environment() -> Self {
        Self {
            program: gpg_program_from_environment().unwrap_or_else(default_gpg_program),
        }
    }

    #[cfg(test)]
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            program: program.into(),
        }
    }

    pub fn decrypt(&self, encrypted_file: &Path) -> Result<String, PasswordStoreError> {
        let output = Command::new(&self.program)
            .arg("--quiet")
            .arg("--decrypt")
            .arg(encrypted_file)
            .output()
            .map_err(map_gpg_spawn_error)?;

        if !output.status.success() {
            return Err(PasswordStoreError::GpgDecryptFailed(gpg_error_message(
                &output.stderr,
            )));
        }

        String::from_utf8(output.stdout).map_err(PasswordStoreError::GpgOutputNotUtf8)
    }
}

fn gpg_program_from_environment() -> Option<OsString> {
    gpg_program_from_environment_value(env::var_os("PASSWORD_STORE_GPG"))
}

fn gpg_program_from_environment_value(value: Option<OsString>) -> Option<OsString> {
    value.filter(|value| !value.is_empty())
}

#[cfg(windows)]
fn default_gpg_program() -> OsString {
    windows_gpg_install_paths()
        .into_iter()
        .find(|path| path.exists())
        .map(OsString::from)
        .unwrap_or_else(|| OsString::from("gpg"))
}

#[cfg(not(windows))]
fn default_gpg_program() -> OsString {
    OsString::from("gpg")
}

#[cfg(windows)]
fn windows_gpg_install_paths() -> Vec<std::path::PathBuf> {
    vec![
        std::path::PathBuf::from(r"C:\Program Files\GnuPG\bin\gpg.exe"),
        std::path::PathBuf::from(r"C:\Program Files (x86)\GnuPG\bin\gpg.exe"),
        std::path::PathBuf::from(r"C:\Program Files\Gpg4win\bin\gpg.exe"),
        std::path::PathBuf::from(r"C:\Program Files (x86)\Gpg4win\bin\gpg.exe"),
    ]
}

fn map_gpg_spawn_error(error: std::io::Error) -> PasswordStoreError {
    if error.kind() == std::io::ErrorKind::NotFound {
        return PasswordStoreError::GpgNotFound;
    }

    PasswordStoreError::Io(error)
}

fn gpg_error_message(stderr: &[u8]) -> String {
    let message = String::from_utf8_lossy(stderr).trim().to_owned();

    if message.is_empty() {
        "gpg failed to decrypt the entry".to_string()
    } else {
        message
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{gpg_error_message, gpg_program_from_environment_value};

    #[test]
    fn uses_fallback_message_for_empty_gpg_stderr() {
        assert_eq!(
            gpg_error_message(b""),
            "gpg failed to decrypt the entry".to_string()
        );
    }

    #[test]
    fn trims_gpg_stderr() {
        assert_eq!(
            gpg_error_message(b"gpg: decryption failed\n"),
            "gpg: decryption failed".to_string()
        );
    }

    #[test]
    fn ignores_empty_gpg_environment_override() {
        let program = gpg_program_from_environment_value(Some(OsString::from("")));

        assert_eq!(program, None);
    }

    #[test]
    fn reads_gpg_environment_override() {
        let program = gpg_program_from_environment_value(Some(OsString::from("custom-gpg")));

        assert_eq!(program, Some(OsString::from("custom-gpg")));
    }
}
