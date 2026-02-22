use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub fn execute_task_add_command(&self, title: &str) -> Result<(), OrbitError> {
        self.add_task(title)?;
        Ok(())
    }
}
