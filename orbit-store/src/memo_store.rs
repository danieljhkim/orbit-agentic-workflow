use orbit_types::{Memo, OrbitError};

use crate::{Store, parse_timestamp};

impl Store {
    pub fn list_memos(&self) -> Result<Vec<Memo>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT id, body, created_at FROM memos ORDER BY created_at DESC")
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let created_at: String = row.get(2)?;
                Ok(Memo {
                    id: row.get(0)?,
                    body: row.get(1)?,
                    created_at: parse_timestamp(&created_at)?,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }
}
