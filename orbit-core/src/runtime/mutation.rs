use orbit_types::OrbitEvent;

use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub(crate) fn record_event(&self, event: OrbitEvent) -> Result<(), OrbitError> {
        let event = crate::runtime::audit::normalize_event(event);
        self.context.audit_store.insert_audit_event(&event)?;
        self.event_bus.publish(event);
        Ok(())
    }

    pub fn with_mutation<F, T>(&self, f: F) -> Result<T, OrbitError>
    where
        F: FnOnce() -> Result<(T, OrbitEvent), OrbitError>,
    {
        let (result, event) = f()?;
        let event = crate::runtime::audit::normalize_event(event);
        self.context.audit_store.insert_audit_event(&event)?;

        self.event_bus.publish(event);
        Ok(result)
    }
}
