use std::error::Error;
use std::process::ExitCode;

use clap::Parser;
use wallet_cli::{run, Cli};

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Err(error) = run(cli) {
        eprintln!("error: {error}");
        if let Some(source) = error.source() {
            eprintln!("caused by: {source}");
        }
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
