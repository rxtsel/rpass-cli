use std::path::Path;

use serde::Serialize;

use super::{GpgCommand, StoreDirectory};

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct DoctorReport {
    pub ok: bool,
    pub store_dir: String,
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    pub fn run(store_directory: &StoreDirectory, gpg: &GpgCommand) -> Self {
        let store_path = store_directory.path();
        let checks = vec![
            store_directory_check(store_path),
            gpg_id_check(store_path),
            gpg_version_check(gpg),
        ];
        let ok = checks.iter().all(|check| check.ok);

        Self {
            ok,
            store_dir: store_path.display().to_string(),
            checks,
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct DoctorCheck {
    pub name: &'static str,
    pub ok: bool,
    pub message: String,
}

impl DoctorCheck {
    fn ok(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            ok: true,
            message: message.into(),
        }
    }

    fn fail(name: &'static str, message: impl Into<String>) -> Self {
        Self {
            name,
            ok: false,
            message: message.into(),
        }
    }
}

fn store_directory_check(store_path: &Path) -> DoctorCheck {
    if !store_path.exists() {
        return DoctorCheck::fail(
            "store_directory",
            format!("store directory does not exist: {}", store_path.display()),
        );
    }

    if !store_path.is_dir() {
        return DoctorCheck::fail(
            "store_directory",
            format!("store path is not a directory: {}", store_path.display()),
        );
    }

    DoctorCheck::ok(
        "store_directory",
        format!("store directory exists: {}", store_path.display()),
    )
}

fn gpg_id_check(store_path: &Path) -> DoctorCheck {
    let gpg_id = store_path.join(".gpg-id");

    if gpg_id.is_file() {
        DoctorCheck::ok("gpg_id", format!(".gpg-id found: {}", gpg_id.display()))
    } else {
        DoctorCheck::fail("gpg_id", format!(".gpg-id not found: {}", gpg_id.display()))
    }
}

fn gpg_version_check(gpg: &GpgCommand) -> DoctorCheck {
    match gpg.version() {
        Ok(version) => DoctorCheck::ok("gpg", format!("{} ({})", version, gpg.program_display())),
        Err(error) => DoctorCheck::fail("gpg", error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::DoctorReport;
    use crate::password_store::{GpgCommand, StoreDirectory};

    #[test]
    fn reports_ready_environment() {
        let temp_dir = TempDir::new().expect("temp dir");
        fs::write(temp_dir.path().join(".gpg-id"), "KEY").expect("gpg id");
        let gpg = fake_gpg_script(temp_dir.path(), "gpg (GnuPG) test\n");
        let store_directory = StoreDirectory::from_path(temp_dir.path());

        let report = DoctorReport::run(&store_directory, &GpgCommand::new(gpg));

        assert!(report.ok);
        assert!(report.checks.iter().all(|check| check.ok));
    }

    #[test]
    fn reports_missing_gpg_id() {
        let temp_dir = TempDir::new().expect("temp dir");
        let gpg = fake_gpg_script(temp_dir.path(), "gpg (GnuPG) test\n");
        let store_directory = StoreDirectory::from_path(temp_dir.path());

        let report = DoctorReport::run(&store_directory, &GpgCommand::new(gpg));

        assert!(!report.ok);
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.name == "gpg_id" && !check.ok)
        );
    }

    #[cfg(windows)]
    fn fake_gpg_script(directory: &std::path::Path, output: &str) -> std::path::PathBuf {
        let script = directory.join("gpg-version.cmd");
        let output_file = directory.join("gpg-version-output.txt");

        fs::write(&output_file, output).expect("output file");
        fs::write(
            &script,
            format!("@echo off\r\ntype \"{}\"\r\n", output_file.display()),
        )
        .expect("script");
        script
    }

    #[cfg(not(windows))]
    fn fake_gpg_script(directory: &std::path::Path, output: &str) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;

        let script = directory.join("gpg-version");
        fs::write(&script, format!("#!/bin/sh\nprintf '{}'\n", output)).expect("script");
        let mut permissions = fs::metadata(&script).expect("metadata").permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(&script, permissions).expect("permissions");
        script
    }
}
