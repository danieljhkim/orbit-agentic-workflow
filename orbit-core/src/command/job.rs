use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub fn execute_job_run_command(&self) -> Result<usize, OrbitError> {
        self.run_jobs()
    }
}
