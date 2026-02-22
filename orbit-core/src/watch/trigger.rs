use orbit_types::{OrbitError, OrbitEvent};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn trigger_watch_path(&self, path: &str) -> Result<(), OrbitError> {
        self.with_mutation(|_| {
            Ok((
                (),
                OrbitEvent::WatchTriggered {
                    path: path.to_string(),
                },
            ))
        })?;
        Ok(())
    }
}
