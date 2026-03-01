use std::path::Path;
use std::sync::{Arc, Mutex};

use orbit_types::OrbitError;
use rusqlite::{Connection, Transaction};

use crate::sqlite::migration;

#[derive(Clone)]
pub struct Store {
    pub(crate) conn: Arc<Mutex<Connection>>,
}

pub struct StoreTx<'a> {
    pub(crate) tx: Transaction<'a>,
}

impl Store {
    pub fn open(path: &Path) -> Result<Self, OrbitError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| OrbitError::Store(e.to_string()))?;
        }

        let conn = Connection::open(path).map_err(|e| OrbitError::Store(e.to_string()))?;
        conn.pragma_update(None, "journal_mode", "WAL")
            .map_err(|e| OrbitError::Store(format!("failed to set WAL mode: {e}")))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| OrbitError::Store(format!("failed to enable foreign keys: {e}")))?;

        migration::apply_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn open_in_memory() -> Result<Self, OrbitError> {
        let conn = Connection::open_in_memory().map_err(|e| OrbitError::Store(e.to_string()))?;
        conn.pragma_update(None, "foreign_keys", "ON")
            .map_err(|e| OrbitError::Store(format!("failed to enable foreign keys: {e}")))?;
        migration::apply_schema(&conn)?;
        Ok(Self {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    pub fn with_transaction<T, F>(&self, op: F) -> Result<T, OrbitError>
    where
        F: FnOnce(&mut StoreTx<'_>) -> Result<T, OrbitError>,
    {
        let mut conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let tx = conn
            .transaction()
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let mut store_tx = StoreTx { tx };
        let result = op(&mut store_tx)?;
        store_tx
            .tx
            .commit()
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(result)
    }
}
