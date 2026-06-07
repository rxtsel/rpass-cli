use std::fs;
use std::path::{Path, PathBuf};

use assert_cmd::Command;
use tempfile::TempDir;

pub fn rpass() -> Command {
    Command::cargo_bin("rpass").expect("rpass binary")
}

pub fn password_store_with_entry(entry: &str) -> TempDir {
    let store = TempDir::new().expect("temp dir");
    create_file(store.path().join(entry));
    store
}

#[allow(dead_code)]
pub fn missing_executable_path(directory: &Path) -> PathBuf {
    directory.join("missing-gpg")
}

fn create_file(path: impl AsRef<Path>) {
    let path = path.as_ref();
    fs::create_dir_all(path.parent().expect("parent")).expect("parent dir");
    fs::write(path, "").expect("file");
}

#[cfg(windows)]
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
pub fn successful_gpg_script(directory: &Path, output: &str) -> PathBuf {
    let script = directory.join("gpg");

    fs::write(&script, format!("#!/bin/sh\nprintf '{}'\n", output)).expect("script");
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
