use std::process::ExitCode;

use scheme::InterpreterShell;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let code = InterpreterShell::run_cli(&args);
    ExitCode::from(code as u8)
}
