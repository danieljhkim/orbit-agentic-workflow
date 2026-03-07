use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::OrbitId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Job {
    pub id: OrbitId,
    pub spec_type: String,
    pub description: String,
    #[serde(default)]
    pub instruction: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub artifact_path_template: Option<String>,
    pub skill_refs: Vec<String>,
    #[serde(default)]
    pub identity_id: Option<String>,
    #[serde(default)]
    pub assigned_to: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
