use chrono::Utc;
use orbit_types::{OrbitError, Workflow};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

use crate::{Store, StoreTx, now_string, parse_timestamp};

#[derive(Debug, Clone)]
pub struct WorkflowInsertParams {
    pub id: String,
    pub name: String,
    pub definition_json: Value,
}

impl Store {
    pub fn list_workflows(&self, include_inactive: bool) -> Result<Vec<Workflow>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let sql = if include_inactive {
            "SELECT id, name, definition_json, is_active, created_at, updated_at FROM workflows ORDER BY created_at DESC"
        } else {
            "SELECT id, name, definition_json, is_active, created_at, updated_at FROM workflows WHERE is_active = 1 ORDER BY created_at DESC"
        };

        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let rows = stmt
            .query_map([], row_to_workflow)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_workflow(&self, id: &str) -> Result<Option<Workflow>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        conn.query_row(
            "SELECT id, name, definition_json, is_active, created_at, updated_at FROM workflows WHERE id = ?1",
            [id],
            row_to_workflow,
        )
        .optional()
        .map_err(|e| OrbitError::Store(e.to_string()))
    }
}

impl<'a> StoreTx<'a> {
    pub fn insert_workflow(
        &mut self,
        params: &WorkflowInsertParams,
    ) -> Result<Workflow, OrbitError> {
        let definition_raw = serde_json::to_string(&params.definition_json)
            .map_err(|e| OrbitError::Store(format!("serialize workflow definition: {e}")))?;

        self.tx
            .execute(
                "INSERT INTO workflows(id, name, definition_json, is_active, created_at, updated_at)
                 VALUES (?1, ?2, ?3, 1, ?4, ?4)",
                params![params.id, params.name, definition_raw, now_string()],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(Workflow {
            id: params.id.clone(),
            name: params.name.clone(),
            definition_json: params.definition_json.clone(),
            is_active: true,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }

    pub fn disable_workflow(&mut self, id: &str) -> Result<bool, OrbitError> {
        let changed = self
            .tx
            .execute(
                "UPDATE workflows SET is_active = 0, updated_at = ?1 WHERE id = ?2",
                params![now_string(), id],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(changed == 1)
    }
}

fn row_to_workflow(row: &rusqlite::Row<'_>) -> rusqlite::Result<Workflow> {
    let definition_raw: String = row.get(2)?;
    let is_active_raw: i64 = row.get(3)?;
    let created_at_raw: String = row.get(4)?;
    let updated_at_raw: String = row.get(5)?;

    let definition_json = serde_json::from_str::<Value>(&definition_raw).map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(
            definition_raw.len(),
            rusqlite::types::Type::Text,
            Box::new(e),
        )
    })?;

    Ok(Workflow {
        id: row.get(0)?,
        name: row.get(1)?,
        definition_json,
        is_active: is_active_raw == 1,
        created_at: parse_timestamp(&created_at_raw)?,
        updated_at: parse_timestamp(&updated_at_raw)?,
    })
}
