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
    use std::path::PathBuf;

    use tempfile::TempDir;

    use super::DoctorReport;
    use crate::password_store::{GpgCommand, StoreDirectory};

    #[test]
    fn reports_ready_environment() {
        let temp_dir = TempDir::new().expect("temp dir");
        fs::write(temp_dir.path().join(".gpg-id"), "KEY").expect("gpg id");
        let gpg = fake_gpg();
        let store_directory = StoreDirectory::from_path(temp_dir.path());

        let report = DoctorReport::run(&store_directory, &GpgCommand::new(gpg));

        assert!(report.ok, "doctor report: {report:#?}");
        assert!(report.checks.iter().all(|check| check.ok));
    }

    #[test]
    fn reports_missing_gpg_id() {
        let temp_dir = TempDir::new().expect("temp dir");
        let gpg = fake_gpg();
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

    fn fake_gpg() -> std::path::PathBuf {
        // Use a program where --version exits 0 everywhere:
        // - GNU coreutils true: prints version to stdout, exits 0
        // - macOS/BSD true: exits 0 silently
        // - cargo: available during tests on all platforms
        if cfg!(windows) {
            PathBuf::from("cargo")
        } else {
            PathBuf::from("true")
        }
    }
}
