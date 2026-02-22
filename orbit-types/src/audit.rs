use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt::{Display, Formatter};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Audit {
    pub id: i64,
    pub event_type: String,
    pub payload: Value,
    pub message: String,
    pub created_at: DateTime<Utc>,
}

impl Display for Audit {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}\t{}",
            self.id,
            self.created_at.to_rfc3339(),
            self.event_type,
            self.message
        )
    }
}
