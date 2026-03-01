use orbit_store::StoreTx;
use orbit_types::OrbitEvent;

use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
    pub(crate) fn record_event(&self, event: OrbitEvent) -> Result<(), OrbitError> {
        let event = crate::runtime::audit::normalize_event(event);
        self.context.store.with_transaction(|tx| {
            tx.insert_audit_event(&event)?;
            Ok(())
        })?;
        self.event_bus.publish(event);
        Ok(())
    }

    pub(crate) fn with_file_mutation<F, T>(&self, f: F, event: OrbitEvent) -> Result<T, OrbitError>
    where
        F: FnOnce() -> Result<T, OrbitError>,
    {
        let result = f()?;
        self.record_event(event)?;
        Ok(result)
    }

    pub fn with_mutation<F, T>(&self, f: F) -> Result<T, OrbitError>
    where
        F: FnOnce(&mut StoreTx<'_>) -> Result<(T, OrbitEvent), OrbitError>,
    {
        let (result, event) = self.context.store.with_transaction(|tx| {
            let (result, event) = f(tx)?;
            let event = crate::runtime::audit::normalize_event(event);
            tx.insert_audit_event(&event)?;
            Ok((result, event))
        })?;

        self.event_bus.publish(event);
        Ok(result)
    }
}
