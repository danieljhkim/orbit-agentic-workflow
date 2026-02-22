use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
pub struct AuditCommand {
    #[command(subcommand)]
    pub command: AuditSubcommand,
}

impl Execute for AuditCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum AuditSubcommand {
    List(AuditListArgs),
}

impl Execute for AuditSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            AuditSubcommand::List(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct AuditListArgs {
    #[arg(long, default_value_t = 20)]
    pub limit: usize,
}

impl Execute for AuditListArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        for audit in runtime.list_audits(self.limit)? {
            crate::output::table::print_line(audit.to_string());
        }
        Ok(())
    }
}
