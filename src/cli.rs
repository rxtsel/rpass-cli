mod tree_output;

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{ArgAction, Parser, Subcommand, ValueHint};
use serde::Serialize;

use crate::password_store::{
    DecryptedEntry, GpgCommand, ListEntries, OtpCode, PasswordStore, SearchEntries, ShowEntry,
    StoreDirectory,
};
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

    #[command(about = "Show a password store entry")]
    Show(ShowCommand),

    #[command(about = "Generate an OTP code for a password store entry")]
    Otp(OtpCommand),

    #[command(about = "Search password store entries")]
    Search(SearchCommand),
}

#[derive(Debug, Parser)]
struct ListCommand {
    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct ShowCommand {
    entry: String,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct OtpCommand {
    entry: String,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct SearchCommand {
    query: String,

    #[arg(long)]
    json: bool,
}

pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let store_directory = StoreDirectory::resolve(cli.store_dir)?;

    match cli.command {
        Command::List(command) => list_entries(command, store_directory),
        Command::Show(command) => show_entry(command, store_directory),
        Command::Otp(command) => generate_otp(command, store_directory),
        Command::Search(command) => search_entries(command, store_directory),
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

fn search_entries(command: SearchCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let store = PasswordStore::open(store_directory)?;
    let entries = SearchEntries::new(&store).execute(&command.query)?;

    if command.json {
        print_json_entries(&entries)?;
    } else {
        print_text_entries(&entries);
    }

    Ok(())
}

fn show_entry(command: ShowCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let store = PasswordStore::open(store_directory)?;
    let gpg = GpgCommand::from_environment();
    let output = ShowEntry::new(&store, &gpg).execute(&command.entry)?;

    if command.json {
        print_json_entry(&command.entry, output.parsed)?;
    } else {
        print!("{}", output.content);
    }

    Ok(())
}

fn print_json_entry(entry_name: &str, entry: DecryptedEntry) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(&ShowEntryJson {
        name: entry_name,
        password: &entry.password,
        fields: &entry.fields,
        otp_uri: entry.otp_uri.as_deref(),
        extra_lines: &entry.extra_lines,
    })?;
    println!("{json}");
    Ok(())
}

fn generate_otp(command: OtpCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let store = PasswordStore::open(store_directory)?;
    let gpg = GpgCommand::from_environment();
    let output = ShowEntry::new(&store, &gpg).execute(&command.entry)?;
    let otp = OtpCode::generate_at(&output.parsed, current_unix_timestamp()?)?;

    if command.json {
        print_json_otp(&command.entry, &otp)?;
    } else {
        println!("{}", otp.code);
    }

    Ok(())
}

fn print_json_otp(entry_name: &str, otp: &OtpCode) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(&OtpJson {
        name: entry_name,
        code: &otp.code,
        remaining_seconds: otp.remaining_seconds,
        period: otp.period,
    })?;
    println!("{json}");
    Ok(())
}

fn current_unix_timestamp() -> Result<u64, CliError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(CliError::SystemClock)?;

    Ok(duration.as_secs())
}

#[derive(Debug, Serialize)]
struct ShowEntryJson<'entry> {
    name: &'entry str,
    password: &'entry str,
    fields: &'entry [crate::password_store::EntryField],
    otp_uri: Option<&'entry str>,
    extra_lines: &'entry [String],
}

#[derive(Debug, Serialize)]
struct OtpJson<'entry> {
    name: &'entry str,
    code: &'entry str,
    remaining_seconds: u64,
    period: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    PasswordStore(#[from] crate::password_store::PasswordStoreError),

    #[error("failed to serialize JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error("system clock is before the Unix epoch: {0}")]
    SystemClock(#[from] std::time::SystemTimeError),
}
