use orbit_types::OrbitError;

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn run_watch_forever(&self) -> Result<(), OrbitError> {
        Err(OrbitError::Execution(
            "watch foreground loop is planned but not implemented yet".to_string(),
        ))
    }
}
