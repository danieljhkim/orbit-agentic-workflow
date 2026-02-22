use std::process::{Child, Command, Stdio};

use orbit_types::OrbitError;

use crate::runner::ExecRequest;

pub(crate) fn spawn(req: &ExecRequest) -> Result<Child, OrbitError> {
    Command::new(&req.program)
        .args(&req.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| OrbitError::Execution(format!("failed to spawn `{}`: {e}", req.program)))
}
