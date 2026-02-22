use orbit_types::{OrbitError, StoredTool};
use rusqlite::{OptionalExtension, params};

use crate::{Store, StoreTx, now_string};

impl Store {
    pub fn list_tools(&self) -> Result<Vec<StoredTool>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT name, path, description, enabled, builtin FROM tools ORDER BY name")
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([], |row| {
                Ok(StoredTool {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    description: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    builtin: row.get::<_, i32>(4)? != 0,
                })
            })
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_tool(&self, name: &str) -> Result<Option<StoredTool>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare("SELECT name, path, description, enabled, builtin FROM tools WHERE name = ?1")
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let result = stmt
            .query_row(params![name], |row| {
                Ok(StoredTool {
                    name: row.get(0)?,
                    path: row.get(1)?,
                    description: row.get(2)?,
                    enabled: row.get::<_, i32>(3)? != 0,
                    builtin: row.get::<_, i32>(4)? != 0,
                })
            })
            .optional()
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(result)
    }
}

impl<'a> StoreTx<'a> {
    pub fn insert_tool(&mut self, tool: &StoredTool) -> Result<(), OrbitError> {
        self.tx
            .execute(
                "INSERT INTO tools(name, path, description, enabled, builtin, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params![
                    tool.name,
                    tool.path,
                    tool.description,
                    tool.enabled as i32,
                    tool.builtin as i32,
                    now_string(),
                    now_string(),
                ],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(())
    }

    pub fn delete_tool(&mut self, name: &str) -> Result<bool, OrbitError> {
        let affected = self
            .tx
            .execute("DELETE FROM tools WHERE name = ?1", params![name])
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(affected > 0)
    }

    pub fn set_tool_enabled(&mut self, name: &str, enabled: bool) -> Result<bool, OrbitError> {
        let affected = self
            .tx
            .execute(
                "UPDATE tools SET enabled = ?1, updated_at = ?2 WHERE name = ?3",
                params![enabled as i32, now_string(), name],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(affected > 0)
    }
}

#[cfg(test)]
mod tests {
    use orbit_types::StoredTool;

    use crate::Store;

    fn test_tool(name: &str) -> StoredTool {
        StoredTool {
            name: name.to_string(),
            path: format!("/usr/local/bin/{name}"),
            description: format!("Test tool {name}"),
            enabled: true,
            builtin: false,
        }
    }

    #[test]
    fn insert_and_list_tools() {
        let store = Store::open_in_memory().expect("store");

        store
            .with_transaction(|tx| {
                tx.insert_tool(&test_tool("my-tool"))?;
                Ok(())
            })
            .expect("insert");

        let tools = store.list_tools().expect("list");
        assert_eq!(tools.len(), 1);
        assert_eq!(tools[0].name, "my-tool");
        assert!(tools[0].enabled);
        assert!(!tools[0].builtin);
    }

    #[test]
    fn get_tool_returns_none_for_missing() {
        let store = Store::open_in_memory().expect("store");
        let result = store.get_tool("nonexistent").expect("get");
        assert!(result.is_none());
    }

    #[test]
    fn get_tool_returns_inserted() {
        let store = Store::open_in_memory().expect("store");

        store
            .with_transaction(|tx| {
                tx.insert_tool(&test_tool("found-tool"))?;
                Ok(())
            })
            .expect("insert");

        let tool = store.get_tool("found-tool").expect("get").expect("some");
        assert_eq!(tool.name, "found-tool");
    }

    #[test]
    fn delete_tool_removes_it() {
        let store = Store::open_in_memory().expect("store");

        store
            .with_transaction(|tx| {
                tx.insert_tool(&test_tool("deleteme"))?;
                Ok(())
            })
            .expect("insert");

        let deleted = store
            .with_transaction(|tx| tx.delete_tool("deleteme"))
            .expect("delete");
        assert!(deleted);

        let tools = store.list_tools().expect("list");
        assert!(tools.is_empty());
    }

    #[test]
    fn delete_nonexistent_returns_false() {
        let store = Store::open_in_memory().expect("store");

        let deleted = store
            .with_transaction(|tx| tx.delete_tool("nope"))
            .expect("delete");
        assert!(!deleted);
    }

    #[test]
    fn set_tool_enabled_toggles_state() {
        let store = Store::open_in_memory().expect("store");

        store
            .with_transaction(|tx| {
                tx.insert_tool(&test_tool("toggle-tool"))?;
                Ok(())
            })
            .expect("insert");

        store
            .with_transaction(|tx| tx.set_tool_enabled("toggle-tool", false))
            .expect("disable");

        let tool = store.get_tool("toggle-tool").expect("get").expect("some");
        assert!(!tool.enabled);

        store
            .with_transaction(|tx| tx.set_tool_enabled("toggle-tool", true))
            .expect("enable");

        let tool = store.get_tool("toggle-tool").expect("get").expect("some");
        assert!(tool.enabled);
    }
}
