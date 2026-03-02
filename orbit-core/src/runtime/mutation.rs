use orbit_types::OrbitEvent;

use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub(crate) fn record_event(&self, event: OrbitEvent) -> Result<(), OrbitError> {
        self.context.audit_store.insert_audit_event(&event)?;
        self.event_log.append(event);
        Ok(())
    }

    pub fn with_mutation<F, T>(&self, f: F) -> Result<T, OrbitError>
    where
        F: FnOnce() -> Result<(T, OrbitEvent), OrbitError>,
    {
        let (result, event) = f()?;
        self.context.audit_store.insert_audit_event(&event)?;
        self.event_log.append(event);
        Ok(result)
    }
}
