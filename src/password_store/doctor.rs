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

    match super::store_directory::check_store_path(store_path, &gpg_id)
        .and_then(|_| super::recipients::read_recipients(&gpg_id))
    {
        Ok(_) => DoctorCheck::ok("gpg_id", format!(".gpg-id found: {}", gpg_id.display())),
        Err(error) => DoctorCheck::fail("gpg_id", format!("{}: {error}", gpg_id.display())),
    }
}

fn gpg_version_check(gpg: &GpgCommand) -> DoctorCheck {
    match gpg.version() {
        Ok(version) => DoctorCheck::ok("gpg", format!("{} ({})", version, gpg.program_display())),
        Err(error) => DoctorCheck::fail("gpg", error.to_string()),
    }
}
