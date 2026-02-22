use std::sync::{Arc, Mutex};

use orbit_types::OrbitEvent;

#[derive(Clone, Default)]
pub struct EventBus {
    events: Arc<Mutex<Vec<OrbitEvent>>>,
}

impl EventBus {
    pub fn publish(&self, event: OrbitEvent) {
        if let Ok(mut events) = self.events.lock() {
            events.push(event);
        }
    }

    pub fn snapshot(&self) -> Vec<OrbitEvent> {
        self.events
            .lock()
            .map(|events| events.clone())
            .unwrap_or_default()
    }
}
