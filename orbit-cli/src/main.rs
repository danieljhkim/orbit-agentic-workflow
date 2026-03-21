mod audit_middleware;
mod command;
mod output;
mod parse;

use clap::Parser;
use orbit_core::OrbitRuntime;

use crate::command::workspace::{WorkspaceCommand, WorkspaceSubcommand};
use crate::command::{Commands, Execute, init::InitCommand};

fn main() {
    let cli = command::Cli::parse();
    let root_override = cli.root.clone();
    let workspace_override = cli.workspace.clone();

    // Commands that run without a pre-existing runtime
    match cli.command {
        Commands::Init(cmd) => {
            if let Err(err) = execute_init_command(cmd, root_override.as_deref()) {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
            return;
        }
        Commands::Workspace(WorkspaceCommand {
            command: WorkspaceSubcommand::Init(args),
        }) => {
            if let Err(err) = args.execute_without_runtime() {
                eprintln!("error: {err}");
                std::process::exit(1);
            }
            return;
        }
        _ => {}
    }

    let runtime = match OrbitRuntime::initialize_with_overrides(
        root_override.as_deref(),
        workspace_override.as_deref(),
    ) {
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
            let mut guard = audit_middleware::AuditGuard::new(&runtime, meta);
            let result = other.execute(&runtime);
            match &result {
                Ok(()) => guard.mark_success(),
                Err(orbit_core::OrbitError::PolicyDenied(msg)) => guard.mark_denied(msg),
                Err(err) => guard.mark_failure(err),
            }
            result
        }
    };

    if let Err(err) = result {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn execute_init_command(
    cmd: InitCommand,
    root_override: Option<&std::path::Path>,
) -> Result<(), orbit_core::OrbitError> {
    cmd.execute_without_runtime(root_override)
}
