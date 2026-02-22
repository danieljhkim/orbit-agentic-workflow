use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::OrbitId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Memo {
    pub id: OrbitId,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
