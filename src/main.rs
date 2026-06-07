use std::process::ExitCode;

fn main() -> ExitCode {
    match rpass::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if error.should_print() {
                eprintln!("{error}");
            }
            ExitCode::FAILURE
        }
    }
}
