use orbit_store::StoreTx;
use orbit_types::OrbitEvent;

use crate::{OrbitError, OrbitRuntime};

impl OrbitRuntime {
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
