use std::env;
use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{ChildStdin, Command, Output, Stdio};

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

    pub fn decrypt(
        &self,
        encrypted_file: &Path,
        passphrase: Option<&str>,
    ) -> Result<String, PasswordStoreError> {
        let output = if let Some(passphrase) = passphrase {
            let mut cmd = self.loopback_decrypt_command();
            configure_passphrase_input(&mut cmd);
            cmd.arg("--decrypt").arg(encrypted_file);
            run_with_passphrase(&mut cmd, passphrase)?
        } else {
            let mut cmd = self.interactive_decrypt_command();
            cmd.arg("--decrypt").arg(encrypted_file);
            cmd.output().map_err(map_gpg_spawn_error)?
        };

        if !output.status.success() {
            if passphrase.is_none() && gpg_requires_passphrase(&output.stderr) {
                return Err(PasswordStoreError::GpgPassphraseRequired);
            }

            return Err(PasswordStoreError::GpgDecryptFailed(gpg_error_message(
                &output.stderr,
            )));
        }

        if output.stdout.is_empty() {
            return Err(PasswordStoreError::GpgEmptyOutput);
        }

        String::from_utf8(output.stdout).map_err(PasswordStoreError::GpgOutputNotUtf8)
    }

    fn interactive_decrypt_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.arg("--quiet").arg("--yes");
        cmd
    }

    fn loopback_decrypt_command(&self) -> Command {
        let mut cmd = Command::new(&self.program);
        cmd.arg("--quiet")
            .arg("--batch")
            .arg("--yes")
            .arg("--no-tty")
            .arg("--pinentry-mode=loopback")
            .arg("--status-fd=2");
        cmd
    }

    pub fn encrypt(
        &self,
        content: &str,
        output_file: &Path,
        recipients: &[String],
    ) -> Result<(), PasswordStoreError> {
        let output_directory = output_file.parent().unwrap_or_else(|| Path::new("."));
        let staged_output = tempfile::NamedTempFile::new_in(output_directory)?.into_temp_path();
        let mut cmd = Command::new(&self.program);
        cmd.arg("--quiet")
            .arg("--batch")
            .arg("--yes")
            .arg("--no-tty")
            .arg("--encrypt")
            .arg("--output")
            .arg(&staged_output)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        for recipient in recipients {
            cmd.arg("--recipient").arg(recipient);
        }

        let mut child = cmd.spawn().map_err(map_gpg_spawn_error)?;
        let stdin_error = match child.stdin.take() {
            Some(mut stdin) => stdin.write_all(content.as_bytes()).err(),
            None => Some(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                "gpg stdin was unavailable",
            )),
        };
        let output = child.wait_with_output().map_err(map_gpg_spawn_error)?;

        if output.status.success() {
            if let Some(error) = stdin_error {
                return Err(PasswordStoreError::Io(error));
            }

            replace_file(&staged_output, output_file)?;
            return Ok(());
        }

        Err(PasswordStoreError::GpgEncryptFailed(gpg_error_message(
            &output.stderr,
        )))
    }

    pub fn program_display(&self) -> String {
        self.program.to_string_lossy().into_owned()
    }

    pub fn version(&self) -> Result<String, PasswordStoreError> {
        let output = Command::new(&self.program)
            .arg("--version")
            .output()
            .map_err(map_gpg_spawn_error)?;

        if !output.status.success() {
            return Err(PasswordStoreError::GpgVersionFailed(gpg_error_message(
                &output.stderr,
            )));
        }

        let stdout =
            String::from_utf8(output.stdout).map_err(PasswordStoreError::GpgOutputNotUtf8)?;
        Ok(first_line(&stdout).to_owned())
    }
}

fn configure_passphrase_input(cmd: &mut Command) {
    cmd.arg("--passphrase-fd=0")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
}

fn run_with_passphrase(cmd: &mut Command, passphrase: &str) -> Result<Output, PasswordStoreError> {
    let mut child = cmd.spawn().map_err(map_gpg_spawn_error)?;
    let stdin_error = write_passphrase(child.stdin.take(), passphrase);
    let output = child.wait_with_output().map_err(map_gpg_spawn_error)?;

    if output.status.success()
        && let Some(error) = stdin_error
    {
        return Err(PasswordStoreError::Io(error));
    }

    Ok(output)
}

fn write_passphrase(stdin: Option<ChildStdin>, passphrase: &str) -> Option<std::io::Error> {
    match stdin {
        Some(mut stdin) => stdin
            .write_all(passphrase.as_bytes())
            .and_then(|_| stdin.write_all(b"\n"))
            .err(),
        None => Some(std::io::Error::new(
            std::io::ErrorKind::BrokenPipe,
            "gpg stdin was unavailable",
        )),
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

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> Result<(), PasswordStoreError> {
    fs::rename(source, destination).map_err(PasswordStoreError::Io)
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> Result<(), PasswordStoreError> {
    if destination.exists() {
        fs::remove_file(destination)?;
    }

    fs::rename(source, destination).map_err(PasswordStoreError::Io)
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

fn gpg_requires_passphrase(stderr: &[u8]) -> bool {
    let message = String::from_utf8_lossy(stderr).to_ascii_lowercase();

    [
        "[gnupg:] need_passphrase",
        "[gnupg:] need_passphrase_sym",
        "[gnupg:] missing_passphrase",
        "no pinentry",
        "pinentry",
        "inappropriate ioctl",
        "can't get input",
        "cannot get input",
        "problem with the agent",
    ]
    .iter()
    .any(|needle| message.contains(needle))
}

fn first_line(output: &str) -> &str {
    output.lines().next().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;

    use super::{
        first_line, gpg_error_message, gpg_program_from_environment_value, gpg_requires_passphrase,
    };

    #[test]
    #[cfg(not(windows))]
    fn decrypt_without_passphrase_allows_interactive_pinentry() {
        let temp_dir = tempfile::TempDir::new().expect("temp dir");
        let encrypted_file = temp_dir.path().join("entry.gpg");
        std::fs::write(&encrypted_file, "").expect("entry");
        let args_file = temp_dir.path().join("args.txt");
        let script = temp_dir.path().join("gpg");
        std::fs::write(
            &script,
            format!(
                "#!/bin/sh\nprintf '%s\\n' \"$@\" > '{}'\nprintf 'secret\\n'\n",
                args_file.display()
            ),
        )
        .expect("script");
        make_executable(&script);

        let output = super::GpgCommand::new(script)
            .decrypt(&encrypted_file, None)
            .expect("decrypt");

        let args = std::fs::read_to_string(args_file).expect("args");
        assert_eq!(output, "secret\n");
        assert!(!args.contains("--batch"));
        assert!(!args.contains("--pinentry-mode=loopback"));
        assert!(!args.contains("--no-tty"));
        assert!(!args.contains("--status-fd=2"));
    }

    #[cfg(not(windows))]
    fn make_executable(path: &std::path::Path) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = std::fs::metadata(path).expect("metadata").permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions).expect("permissions");
    }

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
    fn detects_passphrase_required_gpg_errors() {
        assert!(gpg_requires_passphrase(
            b"gpg: problem with the agent: Inappropriate ioctl for device"
        ));
        assert!(gpg_requires_passphrase(
            b"gpg: Sorry, we are in batchmode - can't get input"
        ));
        assert!(gpg_requires_passphrase(b"gpg: No pinentry"));
        assert!(gpg_requires_passphrase(
            b"[GNUPG:] NEED_PASSPHRASE 0000000000000000 0000000000000000 1 0\ngpg: public key decryption failed: Inappropriate ioctl for device"
        ));
        assert!(!gpg_requires_passphrase(
            b"gpg: public key decryption failed: No secret key\ngpg: decryption failed: No secret key"
        ));
        assert!(!gpg_requires_passphrase(b"gpg: decryption failed"));
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

    #[test]
    fn extracts_first_version_line() {
        assert_eq!(
            first_line("gpg (GnuPG) 2.5.18\nlibgcrypt 1.11.0"),
            "gpg (GnuPG) 2.5.18"
        );
    }
}
