use std::sync::{Arc, Mutex};

use orbit_types::OrbitEvent;

#[derive(Clone, Default)]
pub struct EventLog {
    events: Arc<Mutex<Vec<OrbitEvent>>>,
}

impl EventLog {
    pub fn append(&self, event: OrbitEvent) {
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
