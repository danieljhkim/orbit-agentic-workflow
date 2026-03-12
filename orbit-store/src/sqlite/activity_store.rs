use chrono::Utc;
use orbit_types::{Activity, OrbitError};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

use crate::{Store, StoreTx, now_string, parse_timestamp};

#[derive(Debug, Clone)]
pub struct ActivityInsertParams {
    pub id: String,
    pub spec_type: String,
    pub description: String,
    pub instruction: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub artifact_path_template: Option<String>,
    pub skill_refs: Vec<String>,
    pub identity_id: Option<String>,
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
}

impl Store {
    pub fn list_activities(&self, include_inactive: bool) -> Result<Vec<Activity>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let sql = if include_inactive {
            "SELECT id, type, description, instruction, input_schema_json, output_schema_json, artifact_path_template, skill_refs_json, identity_id, assigned_to, created_by, is_active, created_at, updated_at FROM activities ORDER BY created_at DESC"
        } else {
            "SELECT id, type, description, instruction, input_schema_json, output_schema_json, artifact_path_template, skill_refs_json, identity_id, assigned_to, created_by, is_active, created_at, updated_at FROM activities WHERE is_active = 1 ORDER BY created_at DESC"
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let rows = stmt
            .query_map([], row_to_work)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_activity(&self, id: &str) -> Result<Option<Activity>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        conn.query_row(
            "SELECT id, type, description, instruction, input_schema_json, output_schema_json, artifact_path_template, skill_refs_json, identity_id, assigned_to, created_by, is_active, created_at, updated_at FROM activities WHERE id = ?1",
            [id],
            row_to_work,
        )
        .optional()
        .map_err(|e| OrbitError::Store(e.to_string()))
    }

    #[allow(clippy::too_many_arguments)]
    pub fn update_activity(
        &self,
        id: &str,
        description: Option<String>,
        instruction: Option<String>,
        input_schema_json: Option<Value>,
        output_schema_json: Option<Value>,
        artifact_path_template: Option<Option<String>>,
        skill_refs: Option<Vec<String>>,
        identity_id: Option<Option<String>>,
        assigned_to: Option<Option<String>>,
        is_active: Option<bool>,
    ) -> Result<Activity, OrbitError> {
        let mut activity = self
            .get_activity(id)?
            .ok_or_else(|| OrbitError::InvalidInput(format!("activity not found: {id}")))?;

        if let Some(v) = description {
            activity.description = v;
        }
        if let Some(v) = instruction {
            activity.instruction = v;
        }
        if let Some(v) = input_schema_json {
            activity.input_schema_json = v;
        }
        if let Some(v) = output_schema_json {
            activity.output_schema_json = v;
        }
        if let Some(v) = artifact_path_template {
            activity.artifact_path_template = v;
        }
        if let Some(v) = skill_refs {
            activity.skill_refs = v;
        }
        if let Some(v) = identity_id {
            activity.identity_id = v;
        }
        if let Some(v) = assigned_to {
            activity.assigned_to = v;
        }
        if let Some(v) = is_active {
            activity.is_active = v;
        }
        activity.updated_at = Utc::now();

        let input_raw = serde_json::to_string(&activity.input_schema_json)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let output_raw = serde_json::to_string(&activity.output_schema_json)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let skill_refs_raw = serde_json::to_string(&activity.skill_refs)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let updated_at_str = now_string();

        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;
        conn.execute(
            "UPDATE activities SET description=?1, instruction=?2, input_schema_json=?3, output_schema_json=?4, artifact_path_template=?5, skill_refs_json=?6, identity_id=?7, assigned_to=?8, is_active=?9, updated_at=?10 WHERE id=?11",
            params![
                activity.description,
                activity.instruction,
                input_raw,
                output_raw,
                activity.artifact_path_template,
                skill_refs_raw,
                activity.identity_id,
                activity.assigned_to,
                activity.is_active as i64,
                updated_at_str,
                id,
            ],
        )
        .map_err(|e| OrbitError::Store(e.to_string()))?;

        activity.updated_at =
            parse_timestamp(&updated_at_str).map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(activity)
    }
}

impl<'a> StoreTx<'a> {
    pub fn insert_work(&mut self, params: &ActivityInsertParams) -> Result<Activity, OrbitError> {
        let input_schema_raw = serde_json::to_string(&params.input_schema_json)
            .map_err(|e| OrbitError::Store(format!("serialize input schema: {e}")))?;
        let output_schema_raw = serde_json::to_string(&params.output_schema_json)
            .map_err(|e| OrbitError::Store(format!("serialize output schema: {e}")))?;
        let skill_refs_raw = serde_json::to_string(&params.skill_refs)
            .map_err(|e| OrbitError::Store(format!("serialize skill refs: {e}")))?;

        self.tx
            .execute(
                "INSERT INTO activities(
                    id, type, description, instruction, input_schema_json, output_schema_json,
                    artifact_path_template, skill_refs_json, identity_id, assigned_to, created_by, is_active, created_at, updated_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 1, ?12, ?12)",
                params![
                    params.id,
                    params.spec_type,
                    params.description,
                    params.instruction,
                    input_schema_raw,
                    output_schema_raw,
                    params.artifact_path_template,
                    skill_refs_raw,
                    params.identity_id,
                    params.assigned_to,
                    params.created_by,
                    now_string(),
                ],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(Activity {
            id: params.id.clone(),
            spec_type: params.spec_type.clone(),
            description: params.description.clone(),
            instruction: params.instruction.clone(),
            input_schema_json: params.input_schema_json.clone(),
            output_schema_json: params.output_schema_json.clone(),
            artifact_path_template: params.artifact_path_template.clone(),
            skill_refs: params.skill_refs.clone(),
            identity_id: params.identity_id.clone(),
            assigned_to: params.assigned_to.clone(),
            created_by: params.created_by.clone(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn disable_activity(&mut self, id: &str) -> Result<bool, OrbitError> {
        let changed = self
            .tx
            .execute(
                "UPDATE activities SET is_active = 0, updated_at = ?1 WHERE id = ?2",
                params![now_string(), id],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(changed == 1)
    }
}

fn row_to_work(row: &rusqlite::Row<'_>) -> rusqlite::Result<Activity> {
    let input_raw: String = row.get(4)?;
    let output_raw: String = row.get(5)?;
    let skill_refs_raw: Option<String> = row.get(7)?;
    let is_active_raw: i64 = row.get(11)?;
    let created_at_raw: String = row.get(12)?;
    let updated_at_raw: String = row.get(13)?;

    let input_schema_json = serde_json::from_str::<Value>(&input_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            input_raw.len(),
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;

    let output_schema_json = serde_json::from_str::<Value>(&output_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            output_raw.len(),
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;

    let skill_refs = match skill_refs_raw {
        Some(raw) => serde_json::from_str::<Vec<String>>(&raw).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(
                raw.len(),
                rusqlite::types::Type::Text,
                Box::new(e),
            )
        })?,
        None => Vec::new(),
    };

    Ok(Activity {
        id: row.get(0)?,
        spec_type: row.get(1)?,
        description: row.get(2)?,
        instruction: row.get(3)?,
        input_schema_json,
        output_schema_json,
        artifact_path_template: row.get(6)?,
        skill_refs,
        identity_id: row.get(8)?,
        assigned_to: row.get(9)?,
        created_by: row.get(10)?,
        is_active: is_active_raw == 1,
        created_at: parse_timestamp(&created_at_raw)?,
        updated_at: parse_timestamp(&updated_at_raw)?,
    })
}
