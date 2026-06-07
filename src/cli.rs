mod tree_output;

use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand, ValueHint};

use crate::password_store::{ListEntries, PasswordStore, StoreDirectory};
use tree_output::EntryTree;

#[derive(Debug, Parser)]
#[command(
    name = "rpass",
    bin_name = "rpass",
    version,
    about = "A password-store compatible backend",
    disable_help_subcommand = true,
    disable_version_flag = true
)]
struct Cli {
    #[arg(short = 'v', long = "version", action = ArgAction::Version, help = "Print version")]
    version: (),

    #[arg(
        long,
        global = true,
        value_name = "PATH",
        value_hint = ValueHint::DirPath,
        help = "Use a store directory instead of PASSWORD_STORE_DIR or ~/.password-store"
    )]
    store_dir: Option<PathBuf>,

    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "List password store entries")]
    List(ListCommand),
}

#[derive(Debug, Parser)]
struct ListCommand {
    #[arg(long)]
    json: bool,
}

pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let store_directory = StoreDirectory::resolve(cli.store_dir)?;

    match cli.command {
        Command::List(command) => list_entries(command, store_directory),
    }
}

fn list_entries(command: ListCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let store = PasswordStore::open(store_directory)?;
    let entries = ListEntries::new(&store).execute()?;

    if command.json {
        print_json_entries(&entries)?;
    } else {
        print_text_entries(&entries);
    }

    Ok(())
}

fn print_text_entries(entries: &[String]) {
    let tree = EntryTree::from_entries(entries);
    print!("{tree}");
}

fn print_json_entries(entries: &[String]) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(entries)?;
    println!("{json}");
    Ok(())
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    PasswordStore(#[from] crate::password_store::PasswordStoreError),

    #[error("failed to serialize JSON output: {0}")]
    Json(#[from] serde_json::Error),
}
