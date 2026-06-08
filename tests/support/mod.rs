use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

#[allow(dead_code)]
pub fn rpass() -> Command {
    Command::cargo_bin("rpass").expect("rpass binary")
}

#[allow(dead_code)]
pub fn password_store_with_entry(entry: &str) -> TempDir {
    let store = TempDir::new().expect("temp dir");
    create_file(store.path().join(entry));
    store
}

#[allow(dead_code)]
pub fn missing_executable_path(directory: &Path) -> PathBuf {
    directory.join("missing-gpg")
}

#[allow(dead_code)]
fn create_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, "").expect("file");
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn successful_gpg_script(directory: &Path, output: &str) -> PathBuf {
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
#[allow(dead_code)]
pub fn successful_gpg_script(directory: &Path, output: &str) -> PathBuf {
    let script = directory.join("gpg");

    fs::write(&script, format!("#!/bin/sh\nprintf '{}'\n", output)).expect("script");
    make_executable(&script);
    script
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn passphrase_gpg_script(directory: &Path, expected_passphrase: &str, output: &str) -> PathBuf {
    let script = directory.join("gpg-passphrase.cmd");
    let expected_file = directory.join("gpg-passphrase.txt");
    let output_file = directory.join("gpg-passphrase-output.txt");

    fs::write(&expected_file, expected_passphrase).expect("passphrase file");
    fs::write(&output_file, output).expect("output file");
    fs::write(
        &script,
        format!(
            "@echo off\r\nset /p passphrase=\r\nset /p expected=<\"{}\"\r\nif \"%passphrase%\"==\"%expected%\" (\r\n  echo [GNUPG:] DECRYPTION_OKAY 1>&2\r\n  type \"{}\"\r\n  exit /b 0\r\n)\r\necho gpg: decryption failed: Bad passphrase 1>&2\r\nexit /b 2\r\n",
            expected_file.display(),
            output_file.display()
        ),
    )
    .expect("script");
    script
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn passphrase_gpg_script(directory: &Path, expected_passphrase: &str, output: &str) -> PathBuf {
    let script = directory.join("gpg-passphrase");
    let expected_file = directory.join("gpg-passphrase.txt");
    let output_file = directory.join("gpg-passphrase-output.txt");

    fs::write(&expected_file, expected_passphrase).expect("passphrase file");
    fs::write(&output_file, output).expect("output file");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\nIFS= read -r passphrase\nexpected=$(cat '{}')\nif [ \"$passphrase\" = \"$expected\" ]; then\n  printf '[GNUPG:] DECRYPTION_OKAY\\n' >&2\n  cat '{}'\n  exit 0\nfi\nprintf 'gpg: decryption failed: Bad passphrase\\n' >&2\nexit 2\n",
            expected_file.display(),
            output_file.display()
        ),
    )
    .expect("script");
    make_executable(&script);
    script
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn empty_success_gpg_script(directory: &Path) -> PathBuf {
    let script = directory.join("gpg-empty.cmd");

    fs::write(&script, "@echo off\r\nexit /b 0\r\n").expect("script");
    script
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn empty_success_gpg_script(directory: &Path) -> PathBuf {
    let script = directory.join("gpg-empty");

    fs::write(&script, "#!/bin/sh\nexit 0\n").expect("script");
    make_executable(&script);
    script
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn encrypting_gpg_script(directory: &Path) -> PathBuf {
    let script = directory.join("gpg-encrypt.cmd");
    let recipients_file = directory.join("gpg-recipients.txt");

    fs::write(
        &script,
        format!(
            r#"@echo off
setlocal enabledelayedexpansion
set output=
if exist "{recipients}" del "{recipients}"
:args
if "%~1"=="" goto readstdin
if "%~1"=="--recipient" (
  echo %~2>>"{recipients}"
  shift
  shift
  goto args
)
if "%~1"=="--output" (
  set output=%~2
  shift
  shift
  goto args
)
shift
goto args
:readstdin
findstr /r ".*" > "%output%"
exit /b 0
"#,
            recipients = recipients_file.display()
        ),
    )
    .expect("script");
    script
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn encrypting_gpg_script(directory: &Path) -> PathBuf {
    let script = directory.join("gpg-encrypt");
    let recipients_file = directory.join("gpg-recipients.txt");

    fs::write(
        &script,
        format!(
            r#"#!/bin/sh
set -eu
recipients_file='{}'
: > "$recipients_file"
output=''
while [ "$#" -gt 0 ]; do
  case "$1" in
    --recipient)
      printf '%s\n' "$2" >> "$recipients_file"
      shift 2
      ;;
    --output)
      output="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
cat > "$output"
"#,
            recipients_file.display()
        ),
    )
    .expect("script");
    make_executable(&script);
    script
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn editing_script(directory: &Path, output: &str) -> PathBuf {
    let script = directory.join("editor.cmd");
    let output_file = directory.join("editor-output.txt");

    fs::write(&output_file, output).expect("editor output");
    fs::write(
        &script,
        format!(
            "@echo off\r\ntype \"{}\" > \"%~1\"\r\n",
            output_file.display()
        ),
    )
    .expect("script");
    script
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn editing_script(directory: &Path, output: &str) -> PathBuf {
    let script = directory.join("editor");
    let output_file = directory.join("editor-output.txt");

    fs::write(&output_file, output).expect("editor output");
    fs::write(
        &script,
        format!("#!/bin/sh\ncat '{}' > \"$1\"\n", output_file.display()),
    )
    .expect("script");
    make_executable(&script);
    script
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn editing_gpg_script(directory: &Path, decrypted_output: &str) -> PathBuf {
    let script = directory.join("gpg-edit.cmd");
    let decrypted_file = directory.join("gpg-decrypted.txt");
    let recipients_file = directory.join("gpg-recipients.txt");

    fs::write(&decrypted_file, decrypted_output).expect("decrypted output");
    fs::write(
        &script,
        format!(
            r#"@echo off
set output=
if exist "{recipients}" del "{recipients}"
:args
if "%~1"=="" goto encrypt
if "%~1"=="--decrypt" goto decrypt
if "%~1"=="--recipient" (
  echo %~2>>"{recipients}"
  shift
  shift
  goto args
)
if "%~1"=="--output" (
  set output=%~2
  shift
  shift
  goto args
)
shift
goto args
:decrypt
type "{decrypted}"
exit /b 0
:encrypt
findstr /r ".*" > "%output%"
exit /b 0
"#,
            recipients = recipients_file.display(),
            decrypted = decrypted_file.display()
        ),
    )
    .expect("script");
    script
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn editing_gpg_script(directory: &Path, decrypted_output: &str) -> PathBuf {
    let script = directory.join("gpg-edit");
    let decrypted_file = directory.join("gpg-decrypted.txt");
    let recipients_file = directory.join("gpg-recipients.txt");

    fs::write(&decrypted_file, decrypted_output).expect("decrypted output");
    fs::write(
        &script,
        format!(
            r#"#!/bin/sh
set -eu
recipients_file='{}'
decrypted_file='{}'
: > "$recipients_file"
output=''
mode='encrypt'
while [ "$#" -gt 0 ]; do
  case "$1" in
    --decrypt)
      mode='decrypt'
      shift
      ;;
    --recipient)
      printf '%s\n' "$2" >> "$recipients_file"
      shift 2
      ;;
    --output)
      output="$2"
      shift 2
      ;;
    *)
      shift
      ;;
  esac
done
if [ "$mode" = "decrypt" ]; then
  cat "$decrypted_file"
  exit 0
fi
cat > "$output"
"#,
            recipients_file.display(),
            decrypted_file.display()
        ),
    )
    .expect("script");
    make_executable(&script);
    script
}

#[cfg(windows)]
#[allow(dead_code)]
pub fn failing_gpg_script(directory: &Path, message: &str) -> PathBuf {
    let script = directory.join("gpg-fail.cmd");

    fs::write(
        &script,
        format!("@echo off\r\necho {message} 1>&2\r\nexit /b 2\r\n"),
    )
    .expect("script");
    script
}

#[cfg(not(windows))]
#[allow(dead_code)]
pub fn failing_gpg_script(directory: &Path, message: &str) -> PathBuf {
    let script = directory.join("gpg-fail");

    fs::write(
        &script,
        format!("#!/bin/sh\nprintf '{}' >&2\nexit 2\n", message),
    )
    .expect("script");
    make_executable(&script);
    script
}

#[cfg(not(windows))]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    let mut permissions = fs::metadata(path).expect("metadata").permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).expect("permissions");
}
