use std::path::PathBuf;

use clap::ValueEnum;

use crate::password_store::{ListEntries, PasswordStore, StoreDirectory};

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Powershell,
    Fish,
}

pub fn complete_entries(prefix: &str, store_dir: Option<PathBuf>) {
    let Ok(store_directory) = StoreDirectory::resolve(store_dir) else {
        return;
    };
    let Ok(store) = PasswordStore::open(store_directory) else {
        return;
    };
    let entries = ListEntries::new(&store).execute().unwrap_or_default();

    for entry in entries.iter().filter(|entry| entry.starts_with(prefix)) {
        println!("{entry}");
    }
}

pub fn print_completions(shell: Shell) {
    let script = match shell {
        Shell::Bash => include_str!("../../completions/rpass.bash"),
        Shell::Zsh => include_str!("../../completions/_rpass"),
        Shell::Powershell => include_str!("../../completions/rpass.ps1"),
        Shell::Fish => include_str!("../../completions/rpass.fish"),
    };
    println!("{script}");
}
