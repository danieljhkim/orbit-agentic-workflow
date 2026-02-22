use orbit_types::{OrbitError, Watch};

use crate::{Store, parse_timestamp};

impl Store {
    pub fn list_watches(&self) -> Result<Vec<Watch>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(
                "SELECT id, path, command, debounce_ms, updated_at FROM watches ORDER BY updated_at DESC",
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let updated_at: String = row.get(4)?;
                let debounce_ms: i64 = row.get(3)?;
                Ok(Watch {
                    id: row.get(0)?,
                    path: row.get(1)?,
                    command: row.get(2)?,
                    debounce_ms: debounce_ms as u64,
                    updated_at: parse_timestamp(&updated_at)?,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }
}
