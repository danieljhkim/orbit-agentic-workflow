use chrono::Utc;
use orbit_types::{OrbitError, Task, TaskPriority, TaskStatus, TaskType};
use rusqlite::{OptionalExtension, params};

use crate::{Store, StoreTx, new_id, parse_timestamp};

fn parse_status(raw: &str) -> TaskStatus {
    raw.parse().unwrap_or(TaskStatus::Todo)
}

fn parse_priority(raw: &str) -> TaskPriority {
    raw.parse().unwrap_or(TaskPriority::Medium)
}

fn parse_task_type(raw: &str) -> TaskType {
    raw.parse().unwrap_or(TaskType::Task)
}

fn row_to_task(row: &rusqlite::Row<'_>) -> rusqlite::Result<Task> {
    let status_raw: String = row.get(3)?;
    let priority_raw: String = row.get(4)?;
    let task_type_raw: String = row.get(5)?;
    let parent_id: Option<String> = row.get(7)?;
    let created_at_raw: String = row.get(8)?;
    let updated_at_raw: String = row.get(9)?;

    Ok(Task {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: parse_status(&status_raw),
        priority: parse_priority(&priority_raw),
        task_type: parse_task_type(&task_type_raw),
        owner: row.get(6)?,
        parent_id,
        created_at: parse_timestamp(&created_at_raw)?,
        updated_at: parse_timestamp(&updated_at_raw)?,
    })
}

const SELECT_COLS: &str =
    "id, title, description, status, priority, task_type, owner, parent_id, created_at, updated_at";

impl Store {
    pub fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let sql = format!("SELECT {SELECT_COLS} FROM tasks ORDER BY created_at DESC");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], row_to_task)
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn list_tasks_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
    ) -> Result<Vec<Task>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut conditions = Vec::new();
        let mut param_values: Vec<String> = Vec::new();

        if let Some(s) = status {
            conditions.push(format!("status = ?{}", param_values.len() + 1));
            param_values.push(s.to_string());
        }
        if let Some(p) = priority {
            conditions.push(format!("priority = ?{}", param_values.len() + 1));
            param_values.push(p.to_string());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", conditions.join(" AND "))
        };

        let sql = format!("SELECT {SELECT_COLS} FROM tasks{where_clause} ORDER BY created_at DESC");
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let params: Vec<&dyn rusqlite::types::ToSql> = param_values
            .iter()
            .map(|v| v as &dyn rusqlite::types::ToSql)
            .collect();

        let rows = stmt
            .query_map(params.as_slice(), row_to_task)
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_task(&self, id: &str) -> Result<Option<Task>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let sql = format!("SELECT {SELECT_COLS} FROM tasks WHERE id = ?1");
        conn.query_row(&sql, params![id], row_to_task)
            .optional()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn search_tasks(&self, query: &str) -> Result<Vec<Task>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let pattern = format!("%{query}%");
        let sql = format!(
            "SELECT {SELECT_COLS} FROM tasks WHERE title LIKE ?1 OR description LIKE ?1 ORDER BY created_at DESC"
        );
        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map(params![pattern], row_to_task)
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }
}

pub struct TaskInsertParams {
    pub title: String,
    pub description: String,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub owner: String,
    pub parent_id: Option<String>,
}

impl Default for TaskInsertParams {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            priority: TaskPriority::Medium,
            task_type: TaskType::Task,
            owner: String::new(),
            parent_id: None,
        }
    }
}

#[derive(Default)]
pub struct TaskUpdateFields {
    pub title: Option<String>,
    pub description: Option<String>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub owner: Option<String>,
    pub parent_id: Option<Option<String>>,
}

impl<'a> StoreTx<'a> {
    pub fn insert_task(&mut self, params: &TaskInsertParams) -> Result<Task, OrbitError> {
        let now = Utc::now();
        let task = Task {
            id: new_id("task"),
            title: params.title.clone(),
            description: params.description.clone(),
            status: TaskStatus::Todo,
            priority: params.priority,
            task_type: params.task_type,
            owner: params.owner.clone(),
            parent_id: params.parent_id.clone(),
            created_at: now,
            updated_at: now,
        };

        self.tx
            .execute(
                "INSERT INTO tasks(id, title, description, status, priority, task_type, owner, parent_id, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    task.id,
                    task.title,
                    task.description,
                    task.status.to_string(),
                    task.priority.to_string(),
                    task.task_type.to_string(),
                    task.owner,
                    task.parent_id,
                    task.created_at.to_rfc3339(),
                    task.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(task)
    }

    pub fn update_task(&mut self, id: &str, fields: &TaskUpdateFields) -> Result<bool, OrbitError> {
        let mut sets = Vec::new();
        let mut param_values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();

        if let Some(ref v) = fields.title {
            sets.push("title = ?");
            param_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = fields.description {
            sets.push("description = ?");
            param_values.push(Box::new(v.clone()));
        }
        if let Some(v) = fields.status {
            sets.push("status = ?");
            param_values.push(Box::new(v.to_string()));
        }
        if let Some(v) = fields.priority {
            sets.push("priority = ?");
            param_values.push(Box::new(v.to_string()));
        }
        if let Some(v) = fields.task_type {
            sets.push("task_type = ?");
            param_values.push(Box::new(v.to_string()));
        }
        if let Some(ref v) = fields.owner {
            sets.push("owner = ?");
            param_values.push(Box::new(v.clone()));
        }
        if let Some(ref v) = fields.parent_id {
            sets.push("parent_id = ?");
            param_values.push(Box::new(v.clone()));
        }

        if sets.is_empty() {
            return Ok(false);
        }

        sets.push("updated_at = ?");
        param_values.push(Box::new(Utc::now().to_rfc3339()));

        // Build numbered placeholders
        let set_clause: Vec<String> = sets
            .iter()
            .enumerate()
            .map(|(i, s)| s.replace('?', &format!("?{}", i + 1)))
            .collect();

        let id_param = param_values.len() + 1;
        let sql = format!(
            "UPDATE tasks SET {} WHERE id = ?{}",
            set_clause.join(", "),
            id_param
        );
        param_values.push(Box::new(id.to_string()));

        let params: Vec<&dyn rusqlite::types::ToSql> =
            param_values.iter().map(|v| v.as_ref()).collect();

        let rows = self
            .tx
            .execute(&sql, params.as_slice())
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(rows > 0)
    }

    pub fn set_task_status(&mut self, id: &str, status: TaskStatus) -> Result<bool, OrbitError> {
        let now = Utc::now().to_rfc3339();
        let rows = self
            .tx
            .execute(
                "UPDATE tasks SET status = ?1, updated_at = ?2 WHERE id = ?3",
                params![status.to_string(), now, id],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(rows > 0)
    }

    pub fn delete_task(&mut self, id: &str) -> Result<bool, OrbitError> {
        let rows = self
            .tx
            .execute("DELETE FROM tasks WHERE id = ?1", params![id])
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(rows > 0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Store;

    #[test]
    fn insert_and_get_task() {
        let store = Store::open_in_memory().expect("store");
        let task = store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "test task".to_string(),
                    description: "a description".to_string(),
                    priority: TaskPriority::High,
                    task_type: TaskType::Bug,
                    owner: "alice".to_string(),
                    parent_id: None,
                })
            })
            .expect("insert");

        let found = store.get_task(&task.id).expect("get").expect("some");
        assert_eq!(found.title, "test task");
        assert_eq!(found.description, "a description");
        assert_eq!(found.priority, TaskPriority::High);
        assert_eq!(found.task_type, TaskType::Bug);
        assert_eq!(found.owner, "alice");
        assert_eq!(found.status, TaskStatus::Todo);
    }

    #[test]
    fn list_tasks_filtered_by_status() {
        let store = Store::open_in_memory().expect("store");
        store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "open".to_string(),
                    ..Default::default()
                })?;
                let params2 = TaskInsertParams {
                    title: "done".to_string(),
                    ..Default::default()
                };
                let t2 = tx.insert_task(&params2)?;
                tx.set_task_status(&t2.id, TaskStatus::Done)?;
                Ok(())
            })
            .expect("insert");

        let todos = store
            .list_tasks_filtered(Some(TaskStatus::Todo), None)
            .expect("filter");
        assert_eq!(todos.len(), 1);
        assert_eq!(todos[0].title, "open");
    }

    #[test]
    fn list_tasks_filtered_by_priority() {
        let store = Store::open_in_memory().expect("store");
        store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "low".to_string(),
                    priority: TaskPriority::Low,
                    ..Default::default()
                })?;
                tx.insert_task(&TaskInsertParams {
                    title: "high".to_string(),
                    priority: TaskPriority::High,
                    ..Default::default()
                })?;
                Ok(())
            })
            .expect("insert");

        let high = store
            .list_tasks_filtered(None, Some(TaskPriority::High))
            .expect("filter");
        assert_eq!(high.len(), 1);
        assert_eq!(high[0].title, "high");
    }

    #[test]
    fn search_tasks_by_title() {
        let store = Store::open_in_memory().expect("store");
        store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "fix login bug".to_string(),
                    ..Default::default()
                })?;
                tx.insert_task(&TaskInsertParams {
                    title: "add feature".to_string(),
                    ..Default::default()
                })?;
                Ok(())
            })
            .expect("insert");

        let results = store.search_tasks("login").expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "fix login bug");
    }

    #[test]
    fn search_tasks_by_description() {
        let store = Store::open_in_memory().expect("store");
        store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "task one".to_string(),
                    description: "needs database migration".to_string(),
                    ..Default::default()
                })?;
                tx.insert_task(&TaskInsertParams {
                    title: "task two".to_string(),
                    ..Default::default()
                })?;
                Ok(())
            })
            .expect("insert");

        let results = store.search_tasks("migration").expect("search");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "task one");
    }

    #[test]
    fn update_task_fields() {
        let store = Store::open_in_memory().expect("store");
        let task = store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "original".to_string(),
                    ..Default::default()
                })
            })
            .expect("insert");

        let updated = store
            .with_transaction(|tx| {
                tx.update_task(
                    &task.id,
                    &TaskUpdateFields {
                        title: Some("changed".to_string()),
                        description: Some("new desc".to_string()),
                        priority: Some(TaskPriority::High),
                        owner: Some("bob".to_string()),
                        ..Default::default()
                    },
                )
            })
            .expect("update");
        assert!(updated);

        let found = store.get_task(&task.id).expect("get").expect("some");
        assert_eq!(found.title, "changed");
        assert_eq!(found.description, "new desc");
        assert_eq!(found.priority, TaskPriority::High);
        assert_eq!(found.owner, "bob");
    }

    #[test]
    fn delete_task() {
        let store = Store::open_in_memory().expect("store");
        let task = store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "to delete".to_string(),
                    ..Default::default()
                })
            })
            .expect("insert");

        let deleted = store
            .with_transaction(|tx| tx.delete_task(&task.id))
            .expect("delete");
        assert!(deleted);

        let found = store.get_task(&task.id).expect("get");
        assert!(found.is_none());
    }

    #[test]
    fn set_task_status() {
        let store = Store::open_in_memory().expect("store");
        let task = store
            .with_transaction(|tx| {
                tx.insert_task(&TaskInsertParams {
                    title: "status test".to_string(),
                    ..Default::default()
                })
            })
            .expect("insert");

        store
            .with_transaction(|tx| tx.set_task_status(&task.id, TaskStatus::InProgress))
            .expect("set status");

        let found = store.get_task(&task.id).expect("get").expect("some");
        assert_eq!(found.status, TaskStatus::InProgress);
    }
}
