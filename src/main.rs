use std::process::ExitCode;

fn main() -> ExitCode {
    match rpass::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
