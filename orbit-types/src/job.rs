use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::OrbitId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Scheduled,
    Running,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Job {
    pub id: OrbitId,
    pub name: String,
    pub command: String,
    pub next_run_at: DateTime<Utc>,
    pub last_run_at: Option<DateTime<Utc>>,
    pub status: JobStatus,
}
