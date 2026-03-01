use clap::Args;
use orbit_core::{OrbitError, OrbitRuntime};

use crate::command::Execute;

#[derive(Args)]
pub struct InitCommand {}

impl Execute for InitCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let result = runtime.init_workspace()?;
        println!(
            "initialized identities at {}; created={}; default_work_created={}",
            result.identity_root, result.created_identity_files, result.created_default_work
        );
        Ok(())
    }
}
