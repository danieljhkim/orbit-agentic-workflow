use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub fn execute_watch_run_command(&self, path: &str) -> Result<(), OrbitError> {
        self.trigger_watch_once(path)
    }
}
