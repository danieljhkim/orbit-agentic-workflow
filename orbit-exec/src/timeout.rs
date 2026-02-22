use std::process::{Child, Output};
use std::time::Duration;

use orbit_types::OrbitError;
use wait_timeout::ChildExt;

pub(crate) fn wait_with_optional_timeout(
    mut child: Child,
    timeout_ms: Option<u64>,
) -> Result<(bool, Output), OrbitError> {
    match timeout_ms {
        Some(timeout_ms) => {
            let timeout = Duration::from_millis(timeout_ms);
            match child
                .wait_timeout(timeout)
                .map_err(|e| OrbitError::Execution(format!("wait timeout error: {e}")))?
            {
                Some(_) => Ok((
                    false,
                    child.wait_with_output().map_err(|e| {
                        OrbitError::Execution(format!("failed waiting for output: {e}"))
                    })?,
                )),
                None => {
                    child.kill().map_err(|e| {
                        OrbitError::Execution(format!("failed to kill timed out process: {e}"))
                    })?;
                    Ok((
                        true,
                        child.wait_with_output().map_err(|e| {
                            OrbitError::Execution(format!(
                                "failed waiting for timed out process output: {e}"
                            ))
                        })?,
                    ))
                }
            }
        }
        None => Ok((
            false,
            child.wait_with_output().map_err(|e| {
                OrbitError::Execution(format!("failed waiting for process output: {e}"))
            })?,
        )),
    }
}
