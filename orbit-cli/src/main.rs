mod audit_middleware;
mod command;
mod output;

use clap::Parser;
use orbit_core::OrbitRuntime;

use crate::command::{Commands, Execute};

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

    let result = match cli.command {
        Commands::Audit(cmd) => cmd.execute(&runtime),
        other => {
            let meta = audit_middleware::extract_command_meta(&other);
            audit_middleware::execute_with_audit(&runtime, meta, || other.execute(&runtime))
        }
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}
