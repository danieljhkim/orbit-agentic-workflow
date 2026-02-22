mod command;
mod output;

use clap::Parser;
use orbit_core::OrbitRuntime;

use crate::command::Execute;

fn main() {
    let _config = command::config::CliConfig;
    let cli = command::Cli::parse();
    let runtime = match OrbitRuntime::initialize() {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("failed to initialize runtime: {err}");
            std::process::exit(1);
        }
    };

    if let Err(err) = cli.command.execute(&runtime) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
