use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use serde::Serialize;

use super::{PasswordStore, PasswordStoreError};

const INITIAL_COMMIT_MESSAGE: &str = "Added current contents of password store.";

pub struct GitCommand {
    executable: PathBuf,
}

impl GitCommand {
    pub fn from_environment() -> Self {
        Self {
            executable: env::var_os("PASSWORD_STORE_GIT")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("git")),
        }
    }

    pub fn execute(
        &self,
        store: &PasswordStore,
        args: &[String],
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        if args.first().is_some_and(|arg| arg == "init") {
            return self.init(store);
        }

        self.ensure_repository(store.path())?;
        self.run_in_store(store.path(), args)
    }

    pub fn auto_commit(
        &self,
        store: &PasswordStore,
        message: &str,
    ) -> Result<(), PasswordStoreError> {
        if !self.is_repository_optional(store.path())? {
            return Ok(());
        }

        self.run_in_store(store.path(), &["add".to_string(), "-A".to_string()])?;
        self.run_in_store(
            store.path(),
            &["commit".to_string(), "-m".to_string(), message.to_string()],
        )?;

        Ok(())
    }

    fn init(&self, store: &PasswordStore) -> Result<GitCommandOutput, PasswordStoreError> {
        let init = self.run_in_store(store.path(), &["init".to_string()])?;
        let add = self.run_in_store(store.path(), &["add".to_string(), "-A".to_string()])?;

        if !self.has_staged_changes(store.path())? {
            return Ok(GitCommandOutput {
                stdout: format!("{}{}", init.stdout, add.stdout),
                stderr: format!("{}{}", init.stderr, add.stderr),
                exit_code: init.exit_code,
            });
        }

        let commit = self.run_in_store(
            store.path(),
            &[
                "commit".to_string(),
                "-m".to_string(),
                INITIAL_COMMIT_MESSAGE.to_string(),
            ],
        )?;

        Ok(GitCommandOutput {
            stdout: format!("{}{}{}", init.stdout, add.stdout, commit.stdout),
            stderr: format!("{}{}{}", init.stderr, add.stderr, commit.stderr),
            exit_code: commit.exit_code,
        })
    }

    fn has_staged_changes(&self, store_root: &Path) -> Result<bool, PasswordStoreError> {
        let output = self.raw_git(store_root, &["diff", "--cached", "--quiet"])?;
        Ok(!output.status.success())
    }

    fn ensure_repository(&self, store_root: &Path) -> Result<(), PasswordStoreError> {
        let output = self.raw_git(store_root, &["rev-parse", "--is-inside-work-tree"])?;

        if output.status.success() {
            return Ok(());
        }

        Err(PasswordStoreError::GitRepositoryNotFound)
    }

    fn is_repository_optional(&self, store_root: &Path) -> Result<bool, PasswordStoreError> {
        match self.raw_git(store_root, &["rev-parse", "--is-inside-work-tree"]) {
            Ok(output) => Ok(output.status.success()),
            Err(PasswordStoreError::GitNotFound) if !store_root.join(".git").exists() => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn run_in_store(
        &self,
        store_root: &Path,
        args: &[String],
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        let output = self.raw_git(store_root, args)?;
        let command_output = GitCommandOutput::from_output(output);

        if command_output.exit_code == 0 {
            Ok(command_output)
        } else {
            Err(PasswordStoreError::GitFailed {
                exit_code: command_output.exit_code,
                stderr: command_output.stderr,
            })
        }
    }

    fn raw_git<S: AsRef<str>>(
        &self,
        store_root: &Path,
        args: &[S],
    ) -> Result<Output, PasswordStoreError> {
        Command::new(&self.executable)
            .arg("-C")
            .arg(store_root)
            .args(args.iter().map(AsRef::as_ref))
            .output()
            .map_err(|error| {
                if error.kind() == std::io::ErrorKind::NotFound {
                    PasswordStoreError::GitNotFound
                } else {
                    PasswordStoreError::Io(error)
                }
            })
    }
}

#[derive(Debug, Serialize)]
pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl GitCommandOutput {
    fn from_output(output: Output) -> Self {
        Self {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(1),
        }
    }
}
