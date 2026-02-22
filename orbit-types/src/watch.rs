use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::OrbitId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Watch {
    pub id: OrbitId,
    pub path: String,
    pub command: String,
    pub debounce_ms: u64,
    pub updated_at: DateTime<Utc>,
}
