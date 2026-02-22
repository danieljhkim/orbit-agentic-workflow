use chrono::Utc;
use orbit_types::{OrbitError, Task};
use rusqlite::params;

use crate::{Store, StoreTx, new_id, parse_timestamp};

impl Store {
    pub fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT id, title, created_at FROM tasks ORDER BY created_at DESC")
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                let created_at: String = row.get(2)?;
                Ok(Task {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    created_at: parse_timestamp(&created_at)?,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }
}

impl<'a> StoreTx<'a> {
    pub fn insert_task(&mut self, title: &str) -> Result<Task, OrbitError> {
        let task = Task {
            id: new_id("task"),
            title: title.to_string(),
            created_at: Utc::now(),
        };

        self.tx
            .execute(
                "INSERT INTO tasks(id, title, created_at) VALUES (?1, ?2, ?3)",
                params![task.id, task.title, task.created_at.to_rfc3339()],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(task)
    }
}
