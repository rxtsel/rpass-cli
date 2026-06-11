mod tree_output;

use std::io::{BufRead, IsTerminal, Read, Write};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use clap::{ArgAction, Parser, Subcommand, ValueHint};
use serde::Serialize;

use crate::password_store::{
    DecryptedEntry, DoctorReport, EditEntry, GpgCommand, InsertEntry, ListEntries, OtpCode,
    PasswordStore, SearchEntries, ShowEntry, StoreDirectory,
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

    #[command(about = "Insert a password store entry")]
    Insert(InsertCommand),

    #[command(about = "Edit a password store entry")]
    Edit(EditCommand),

    #[command(about = "Generate an OTP code for a password store entry")]
    Otp(OtpCommand),

    #[command(about = "Search password store entries")]
    Search(SearchCommand),

    #[command(about = "Check the local rpass environment")]
    Doctor(DoctorCommand),
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
    passphrase_stdin: bool,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct InsertCommand {
    entry: String,

    #[arg(short = 'e', long)]
    echo: bool,

    #[arg(short = 'm', long)]
    multiline: bool,

    #[arg(short = 'f', long)]
    force: bool,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct EditCommand {
    entry: String,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct OtpCommand {
    entry: String,

    #[arg(long)]
    passphrase_stdin: bool,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct SearchCommand {
    query: String,

    #[arg(long)]
    json: bool,
}

#[derive(Debug, Parser)]
struct DoctorCommand {
    #[arg(long)]
    json: bool,
}

pub fn run() -> Result<(), CliError> {
    let cli = Cli::parse();
    let wants_json_error = cli.command.wants_json();
    let store_directory = StoreDirectory::resolve(cli.store_dir)?;

    let result = match cli.command {
        Command::List(command) => list_entries(command, store_directory),
        Command::Show(command) => show_entry(command, store_directory),
        Command::Insert(command) => insert_entry(command, store_directory),
        Command::Edit(command) => edit_entry(command, store_directory),
        Command::Otp(command) => generate_otp(command, store_directory),
        Command::Search(command) => search_entries(command, store_directory),
        Command::Doctor(command) => run_doctor(command, store_directory),
    };

    if let Err(error) = result {
        if wants_json_error {
            print_json_error(&error)?;
            return Err(CliError::Reported);
        }

        return Err(error);
    }

    Ok(())
}

impl Command {
    fn wants_json(&self) -> bool {
        match self {
            Self::List(command) => command.json,
            Self::Show(command) => command.json,
            Self::Insert(command) => command.json,
            Self::Edit(command) => command.json,
            Self::Otp(command) => command.json,
            Self::Search(command) => command.json,
            Self::Doctor(command) => command.json,
        }
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
    let passphrase = command_passphrase(command.passphrase_stdin)?;
    let output = ShowEntry::new(&store, &gpg).execute(&command.entry, passphrase.as_deref())?;

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

fn insert_entry(command: InsertCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let store = PasswordStore::open(store_directory)?;
    let gpg = GpgCommand::from_environment();
    let content = command_entry_content(&command.entry, command.multiline, command.echo)?;

    InsertEntry::new(&store, &gpg).execute(&command.entry, &content, command.force)?;

    if command.json {
        print_json_insert(&command.entry)?;
    }

    Ok(())
}

fn print_json_insert(entry_name: &str) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(&InsertJson { name: entry_name })?;
    println!("{json}");
    Ok(())
}

fn edit_entry(command: EditCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let store = PasswordStore::open(store_directory)?;
    let gpg = GpgCommand::from_environment();

    let changed = EditEntry::new(&store, &gpg).execute(&command.entry)?;

    if changed {
        if command.json {
            print_json_insert(&command.entry)?;
        } else {
            println!("Entry '{}' updated", command.entry);
        }
    }

    Ok(())
}

fn generate_otp(command: OtpCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let store = PasswordStore::open(store_directory)?;
    let gpg = GpgCommand::from_environment();
    let passphrase = command_passphrase(command.passphrase_stdin)?;
    let output = ShowEntry::new(&store, &gpg).execute(&command.entry, passphrase.as_deref())?;
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

fn command_entry_content(
    entry_name: &str,
    multiline: bool,
    echo: bool,
) -> Result<String, CliError> {
    if multiline {
        if std::io::stdin().is_terminal() {
            print_multiline_help(entry_name);
        }

        return command_stdin();
    }

    if std::io::stdin().is_terminal() {
        let password = if echo {
            prompt_line("Enter password: ")?
        } else {
            rpassword::prompt_password("Enter password: ")
                .map_err(CliError::ReadTerminalPassword)?
        };
        let confirmation = if echo {
            prompt_line("Retype password: ")?
        } else {
            rpassword::prompt_password("Retype password: ")
                .map_err(CliError::ReadTerminalPassword)?
        };

        if password != confirmation {
            return Err(CliError::PasswordConfirmationMismatch);
        }

        return Ok(format!("{password}\n"));
    }

    command_stdin_first_line()
}

fn print_multiline_help(entry_name: &str) {
    eprintln!("Enter multiline secret for {entry_name}.");
    eprintln!("First line is password. Additional lines are metadata.");
    eprintln!("{}", multiline_end_hint());
}

#[cfg(windows)]
fn multiline_end_hint() -> &'static str {
    "Press Ctrl-Z then Enter when finished."
}

#[cfg(not(windows))]
fn multiline_end_hint() -> &'static str {
    "Press Ctrl-D when finished."
}

fn prompt_line(prompt: &str) -> Result<String, CliError> {
    eprint!("{prompt}");
    std::io::stderr().flush().map_err(CliError::ReadStdin)?;

    let mut input = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut input)
        .map_err(CliError::ReadStdin)?;
    Ok(input.trim_end_matches(['\r', '\n']).to_owned())
}

fn command_stdin_first_line() -> Result<String, CliError> {
    let mut input = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut input)
        .map_err(CliError::ReadStdin)?;
    Ok(input)
}

fn command_stdin() -> Result<String, CliError> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(CliError::ReadStdin)?;
    Ok(input)
}

fn command_passphrase(passphrase_stdin: bool) -> Result<Option<String>, CliError> {
    if !passphrase_stdin {
        return Ok(None);
    }

    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .map_err(CliError::ReadPassphrase)?;
    Ok(Some(input.trim_end_matches(['\r', '\n']).to_owned()))
}

fn current_unix_timestamp() -> Result<u64, CliError> {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(CliError::SystemClock)?;

    Ok(duration.as_secs())
}

fn run_doctor(command: DoctorCommand, store_directory: StoreDirectory) -> Result<(), CliError> {
    let gpg = GpgCommand::from_environment();
    let report = DoctorReport::run(&store_directory, &gpg);

    if command.json {
        print_json_doctor(&report)?;
    } else {
        print_text_doctor(&report);
    }

    if report.ok {
        Ok(())
    } else {
        Err(CliError::DoctorFailed)
    }
}

fn print_text_doctor(report: &DoctorReport) {
    println!("rpass doctor");
    println!("store dir: {}", report.store_dir);

    for check in &report.checks {
        let status = if check.ok { "ok" } else { "fail" };
        println!("[{status}] {}: {}", check.name, check.message);
    }

    if report.ok {
        println!("rpass is ready");
    } else {
        println!("rpass needs attention");
    }
}

fn print_json_doctor(report: &DoctorReport) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(report)?;
    println!("{json}");
    Ok(())
}

fn print_json_error(error: &CliError) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(&ErrorJson {
        error: ErrorBody {
            code: error.code(),
            message: error.to_string(),
        },
    })?;
    eprintln!("{json}");
    Ok(())
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
struct InsertJson<'entry> {
    name: &'entry str,
}

#[derive(Debug, Serialize)]
struct OtpJson<'entry> {
    name: &'entry str,
    code: &'entry str,
    remaining_seconds: u64,
    period: u64,
}

#[derive(Debug, Serialize)]
struct ErrorJson<'error> {
    error: ErrorBody<'error>,
}

#[derive(Debug, Serialize)]
struct ErrorBody<'error> {
    code: &'error str,
    message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error(transparent)]
    PasswordStore(#[from] crate::password_store::PasswordStoreError),

    #[error("failed to serialize JSON output: {0}")]
    Json(#[from] serde_json::Error),

    #[error("system clock is before the Unix epoch: {0}")]
    SystemClock(#[from] std::time::SystemTimeError),

    #[error("failed to read stdin: {0}")]
    ReadStdin(std::io::Error),

    #[error("failed to read password from terminal: {0}")]
    ReadTerminalPassword(std::io::Error),

    #[error("failed to read passphrase from stdin: {0}")]
    ReadPassphrase(std::io::Error),

    #[error("password confirmation did not match")]
    PasswordConfirmationMismatch,

    #[error("doctor checks failed")]
    DoctorFailed,

    #[error("error already reported")]
    Reported,
}

impl CliError {
    pub fn should_print(&self) -> bool {
        !matches!(self, Self::Reported)
    }

    fn code(&self) -> &'static str {
        match self {
            Self::PasswordStore(error) => error.code(),
            Self::Json(_) => "json_serialization_failed",
            Self::SystemClock(_) => "system_clock_before_unix_epoch",
            Self::ReadStdin(_) => "read_stdin_failed",
            Self::ReadTerminalPassword(_) => "read_terminal_password_failed",
            Self::ReadPassphrase(_) => "read_passphrase_failed",
            Self::PasswordConfirmationMismatch => "password_confirmation_mismatch",
            Self::DoctorFailed => "doctor_checks_failed",
            Self::Reported => "reported",
        }
    }
}
