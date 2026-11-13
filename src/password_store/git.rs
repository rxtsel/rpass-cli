use std::env;
use std::ffi::OsStr;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

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
            return self.init(store, args);
        }

        self.ensure_repository(store.path())?;
        self.run_in_store(store.path(), args)
    }

    pub fn auto_commit(
        &self,
        store: &PasswordStore,
        message: &str,
        changed: &[PathBuf],
    ) -> Result<(), PasswordStoreError> {
        if !self.is_repository_optional(store.path())? {
            return Ok(());
        }

        self.commit_store_files(store.path(), message, Some(changed))?;
        Ok(())
    }

    fn init(
        &self,
        store: &PasswordStore,
        args: &[String],
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        let mut options = args.iter().skip(1);
        while let Some(arg) = options.next() {
            if arg == "--bare" || arg.starts_with("--bare=") {
                return Err(PasswordStoreError::GitFailed {
                    exit_code: 1,
                    stderr: "bare Git repositories cannot be used as password stores".to_owned(),
                });
            }
            if matches!(arg.as_str(), "-b" | "--initial-branch") && options.next().is_some() {
                continue;
            }
            if matches!(arg.as_str(), "-q" | "--quiet")
                || arg.starts_with("--initial-branch=")
                || arg.starts_with("-b") && arg.len() > 2
            {
                continue;
            }
            return Err(PasswordStoreError::GitFailed {
                exit_code: 1,
                stderr: format!("unsupported git init argument: {arg}"),
            });
        }
        let init = self.run_in_store(store.path(), args)?;
        let commit = self.commit_store_files(store.path(), INITIAL_COMMIT_MESSAGE, None)?;
        Ok(GitCommandOutput {
            stdout: format!("{}{}", init.stdout, commit.stdout),
            stderr: format!("{}{}", init.stderr, commit.stderr),
            exit_code: commit.exit_code,
        })
    }

    fn commit_store_files(
        &self,
        root: &Path,
        message: &str,
        changed: Option<&[PathBuf]>,
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        let case_rename = changed.and_then(|paths| match paths {
            [old, new]
                if old != new
                    && old
                        .to_string_lossy()
                        .eq_ignore_ascii_case(&new.to_string_lossy()) =>
            {
                Some((old, new))
            }
            _ => None,
        });
        let output = if case_rename.is_some() {
            self.raw_git(
                root,
                &[
                    "-c",
                    "core.ignorecase=false",
                    "ls-files",
                    "-z",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                ],
            )?
        } else {
            self.raw_git(
                root,
                &[
                    "ls-files",
                    "-z",
                    "--cached",
                    "--others",
                    "--exclude-standard",
                ],
            )?
        };
        if !output.status.success() {
            return Err(PasswordStoreError::GitFailed {
                exit_code: output.status.code().unwrap_or(1),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }
        let candidates = output
            .stdout
            .split(|byte| *byte == 0)
            .filter(|path| {
                let name = path.rsplit(|byte| *byte == b'/').next().unwrap_or(path);
                name == b".gpg-id" || name.ends_with(b".gpg")
            })
            // shortcut: .rpass-backup-* is reserved for recoverable move backups; revisit if stores need that prefix.
            .filter(|path| {
                !path_from_git(path).components().any(|component| {
                    component
                        .as_os_str()
                        .to_string_lossy()
                        .starts_with(".rpass-backup-")
                })
            })
            .collect::<Vec<_>>();
        let mut paths = Vec::new();
        for path in candidates {
            if let Some(changed) = changed {
                let git_path = path_from_git(path);
                let mut matches = false;
                for target in changed {
                    if git_path_starts_with(root, &git_path, target)? {
                        matches = true;
                        break;
                    }
                }
                if !matches {
                    continue;
                }
            }
            paths.push(path);
        }
        if paths.is_empty() {
            return Ok(GitCommandOutput::empty());
        }
        let mut pathspecs = Vec::new();
        for path in &paths {
            pathspecs.extend_from_slice(b":(literal)");
            pathspecs.extend_from_slice(path);
            pathspecs.push(0);
        }
        if let Some((_, new)) = case_rename {
            return self.commit_case_only_rename(root, message, new, &paths, &pathspecs);
        }
        self.run_in_store_input(
            root,
            &["add", "-A", "--pathspec-from-file=-", "--pathspec-file-nul"],
            Some(&pathspecs),
        )?;
        let staged = self.raw_git(
            root,
            &["diff", "--cached", "--name-only", "-z", "--no-renames"],
        )?;
        if !staged.status.success() {
            return Err(PasswordStoreError::GitFailed {
                exit_code: staged.status.code().unwrap_or(1),
                stderr: String::from_utf8_lossy(&staged.stderr).into_owned(),
            });
        }
        if !staged
            .stdout
            .split(|byte| *byte == 0)
            .any(|path| paths.contains(&path))
        {
            return Ok(GitCommandOutput::empty());
        }
        self.run_in_store_input(
            root,
            &[
                "commit",
                "--only",
                "-m",
                message,
                "--pathspec-from-file=-",
                "--pathspec-file-nul",
            ],
            Some(&pathspecs),
        )
    }

    fn commit_case_only_rename(
        &self,
        root: &Path,
        message: &str,
        new: &Path,
        paths: &[&[u8]],
        pathspecs: &[u8],
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        let mut old_paths = Vec::new();
        let mut new_paths = Vec::new();
        for path in paths {
            let selected = if path_from_git(path).starts_with(new) {
                &mut new_paths
            } else {
                &mut old_paths
            };
            selected.extend_from_slice(b":(literal)");
            selected.extend_from_slice(path);
            selected.push(0);
        }
        if new_paths.is_empty() {
            return Err(PasswordStoreError::GitFailed {
                exit_code: 1,
                stderr: "git did not find the renamed entry; commit it manually".to_owned(),
            });
        }
        let index_dir = tempfile::tempdir()?;
        let index = index_dir.path().join("index");
        if self
            .raw_git(root, &["rev-parse", "--verify", "HEAD"])?
            .status
            .success()
        {
            self.run_in_store_index_input(root, &["read-tree", "HEAD"], None, &index)?;
        } else {
            self.run_in_store_index_input(root, &["read-tree", "--empty"], None, &index)?;
        }
        if !old_paths.is_empty() {
            self.run_in_store_index_input(
                root,
                &[
                    "-c",
                    "core.ignorecase=false",
                    "rm",
                    "-r",
                    "--cached",
                    "--ignore-unmatch",
                    "--pathspec-from-file=-",
                    "--pathspec-file-nul",
                ],
                Some(&old_paths),
                &index,
            )?;
        }
        self.run_in_store_index_input(
            root,
            &[
                "-c",
                "core.ignorecase=false",
                "add",
                "-A",
                "--pathspec-from-file=-",
                "--pathspec-file-nul",
            ],
            Some(&new_paths),
            &index,
        )?;
        let commit = self.run_in_store_index_input(
            root,
            &["-c", "core.ignorecase=false", "commit", "-m", message],
            None,
            &index,
        )?;
        self.run_in_store_input(
            root,
            &[
                "-c",
                "core.ignorecase=false",
                "reset",
                "-q",
                "HEAD",
                "--pathspec-from-file=-",
                "--pathspec-file-nul",
            ],
            Some(pathspecs),
        )?;
        Ok(commit)
    }

    fn run_in_store_index_input<S: AsRef<OsStr>>(
        &self,
        root: &Path,
        args: &[S],
        input: Option<&[u8]>,
        index: &Path,
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        let output = self.raw_git_input(root, args, input, Some(index))?;
        let output = GitCommandOutput::from_output(output);
        if output.exit_code == 0 {
            Ok(output)
        } else {
            Err(PasswordStoreError::GitFailed {
                exit_code: output.exit_code,
                stderr: output.stderr,
            })
        }
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

    fn run_in_store<S: AsRef<OsStr>>(
        &self,
        store_root: &Path,
        args: &[S],
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        self.run_in_store_input(store_root, args, None)
    }

    fn run_in_store_input<S: AsRef<OsStr>>(
        &self,
        store_root: &Path,
        args: &[S],
        input: Option<&[u8]>,
    ) -> Result<GitCommandOutput, PasswordStoreError> {
        let output = self.raw_git_input(store_root, args, input, None)?;
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

    fn raw_git<S: AsRef<OsStr>>(
        &self,
        store_root: &Path,
        args: &[S],
    ) -> Result<Output, PasswordStoreError> {
        self.raw_git_input(store_root, args, None, None)
    }

    fn raw_git_input<S: AsRef<OsStr>>(
        &self,
        store_root: &Path,
        args: &[S],
        input: Option<&[u8]>,
        index: Option<&Path>,
    ) -> Result<Output, PasswordStoreError> {
        let mut command = Command::new(&self.executable);
        command
            .arg("-C")
            .arg(store_root)
            .args(args.iter().map(AsRef::as_ref));
        if let Some(index) = index {
            command.env("GIT_INDEX_FILE", index);
        }
        if let Some(input) = input {
            let mut child = command
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(map_git_spawn_error)?;
            let write_error = child
                .stdin
                .take()
                .expect("piped git stdin")
                .write_all(input)
                .err();
            let output = child.wait_with_output().map_err(map_git_spawn_error)?;
            if output.status.success()
                && let Some(error) = write_error
            {
                return Err(PasswordStoreError::Io(error));
            }
            Ok(output)
        } else {
            command.output().map_err(map_git_spawn_error)
        }
    }
}

fn map_git_spawn_error(error: std::io::Error) -> PasswordStoreError {
    if error.kind() == std::io::ErrorKind::NotFound {
        PasswordStoreError::GitNotFound
    } else {
        PasswordStoreError::Io(error)
    }
}

fn git_path_starts_with(
    _root: &Path,
    path: &Path,
    target: &Path,
) -> Result<bool, PasswordStoreError> {
    #[cfg(windows)]
    {
        git_path_starts_with_windows(_root, path, target)
    }
    #[cfg(not(windows))]
    {
        Ok(path.starts_with(target))
    }
}

#[cfg(any(windows, test))]
fn git_path_starts_with_windows(
    root: &Path,
    path: &Path,
    target: &Path,
) -> Result<bool, PasswordStoreError> {
    if path.starts_with(target) {
        return Ok(true);
    }
    // shortcut: Unicode case folding may differ from NTFS; revisit for non-ASCII case variants.
    let mut components = path.components();
    if !target.components().all(|target| {
        components.next().is_some_and(|component| {
            component.as_os_str().to_string_lossy().to_lowercase()
                == target.as_os_str().to_string_lossy().to_lowercase()
        })
    }) {
        return Ok(false);
    }
    match (
        root.join(path).canonicalize(),
        root.join(target).canonicalize(),
    ) {
        (Ok(path), Ok(target)) => Ok(path.starts_with(target)),
        (Err(path), Err(target))
            if path.kind() == std::io::ErrorKind::NotFound
                && target.kind() == std::io::ErrorKind::NotFound =>
        {
            Ok(true)
        }
        (Err(error), _) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        (_, Err(error)) if error.kind() != std::io::ErrorKind::NotFound => Err(error.into()),
        _ => Ok(false),
    }
}

#[cfg(test)]
mod path_matching_tests {
    use std::fs;
    use std::path::Path;

    use super::git_path_starts_with_windows;

    #[cfg(unix)]
    #[test]
    fn does_not_match_distinct_files_in_case_sensitive_directory() {
        let root = tempfile::TempDir::new().unwrap();
        fs::write(root.path().join("Foo.gpg"), "unrelated").unwrap();
        fs::write(root.path().join("foo.gpg"), "changed").unwrap();
        assert!(
            !git_path_starts_with_windows(root.path(), Path::new("Foo.gpg"), Path::new("foo.gpg"))
                .unwrap()
        );
        assert!(
            git_path_starts_with_windows(root.path(), Path::new("foo.gpg"), Path::new("foo.gpg"))
                .unwrap()
        );
    }

    #[cfg(windows)]
    #[test]
    fn matches_same_file_with_different_case() {
        let root = tempfile::TempDir::new().unwrap();
        fs::write(root.path().join("Foo.gpg"), "ciphertext").unwrap();
        assert!(
            git_path_starts_with_windows(root.path(), Path::new("Foo.gpg"), Path::new("foo.gpg"))
                .unwrap()
        );
        assert!(
            !git_path_starts_with_windows(
                root.path(),
                Path::new("teamwork/Foo.gpg"),
                Path::new("team")
            )
            .unwrap()
        );
    }
}

#[cfg(unix)]
fn path_from_git(bytes: &[u8]) -> PathBuf {
    use std::os::unix::ffi::OsStrExt;
    PathBuf::from(OsStr::from_bytes(bytes))
}

#[cfg(not(unix))]
fn path_from_git(bytes: &[u8]) -> PathBuf {
    PathBuf::from(String::from_utf8_lossy(bytes).into_owned())
}

#[derive(Debug, Serialize)]
pub struct GitCommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

impl GitCommandOutput {
    fn empty() -> Self {
        Self {
            stdout: String::new(),
            stderr: String::new(),
            exit_code: 0,
        }
    }

    fn from_output(output: Output) -> Self {
        Self {
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            exit_code: output.status.code().unwrap_or(1),
        }
    }
}
