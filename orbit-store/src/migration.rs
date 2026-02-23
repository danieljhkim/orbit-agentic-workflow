use orbit_types::OrbitError;
use rusqlite::Connection;

pub(crate) fn apply_schema(conn: &Connection) -> Result<(), OrbitError> {
    conn.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS tasks (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                instructions TEXT NOT NULL DEFAULT '',
                context_files TEXT NOT NULL DEFAULT '[]',
                status TEXT NOT NULL DEFAULT 'todo',
                priority TEXT NOT NULL DEFAULT 'medium',
                task_type TEXT NOT NULL DEFAULT 'task',
                owner TEXT NOT NULL DEFAULT '',
                parent_id TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS memos (
                id TEXT PRIMARY KEY,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS watches (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                command TEXT NOT NULL,
                debounce_ms INTEGER NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS audits (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                event_type TEXT NOT NULL,
                payload TEXT NOT NULL,
                message TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS locks (
                name TEXT PRIMARY KEY,
                owner TEXT NOT NULL,
                acquired_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS tools (
                name TEXT PRIMARY KEY,
                path TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                enabled INTEGER NOT NULL DEFAULT 1,
                builtin INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE TABLE IF NOT EXISTS skills (
                schema_version INTEGER NOT NULL,
                name TEXT PRIMARY KEY,
                description TEXT,
                instructions TEXT NOT NULL,
                context_files TEXT NOT NULL DEFAULT '[]',
                allowed_tools TEXT NOT NULL DEFAULT '[]',
                role TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS task_skills (
                task_id TEXT NOT NULL,
                skill_name TEXT NOT NULL,
                attachment_order INTEGER NOT NULL,
                created_at TEXT NOT NULL,
                PRIMARY KEY (task_id, skill_name),
                FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE,
                FOREIGN KEY(skill_name) REFERENCES skills(name) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS agent_sessions (
                session_id TEXT PRIMARY KEY,
                task_id TEXT NOT NULL,
                skill_names TEXT NOT NULL,
                composed_context_hash TEXT NOT NULL,
                effective_allowed_tools TEXT NOT NULL,
                tool_calls TEXT NOT NULL,
                outcome TEXT NOT NULL,
                status TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY(task_id) REFERENCES tasks(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS entries (
                id TEXT PRIMARY KEY,
                entity_type TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                session_id TEXT,
                sequence_number INTEGER NOT NULL,
                entry_type TEXT NOT NULL,
                author_type TEXT NOT NULL,
                author_id TEXT NOT NULL,
                author_model TEXT,
                body TEXT NOT NULL,
                created_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_entries_entity_seq
            ON entries(entity_type, entity_id, sequence_number);

            CREATE INDEX IF NOT EXISTS idx_entries_entity
            ON entries(entity_type, entity_id);

            CREATE INDEX IF NOT EXISTS idx_entries_session
            ON entries(session_id);

            CREATE INDEX IF NOT EXISTS idx_entries_author
            ON entries(author_type, author_id);
        "#,
    )
    .map_err(|e| OrbitError::Store(e.to_string()))?;

    add_column_if_missing(
        conn,
        "ALTER TABLE tasks ADD COLUMN instructions TEXT NOT NULL DEFAULT ''",
    )?;
    add_column_if_missing(
        conn,
        "ALTER TABLE tasks ADD COLUMN context_files TEXT NOT NULL DEFAULT '[]'",
    )?;

    migrate_jobs_table_to_v2(conn)?;
    ensure_job_schema_v2(conn)?;
    ensure_execution_targets_schema(conn)?;
    ensure_audit_events_schema(conn)?;

    Ok(())
}

fn add_column_if_missing(conn: &Connection, sql: &str) -> Result<(), OrbitError> {
    match conn.execute(sql, []) {
        Ok(_) => Ok(()),
        Err(e) if e.to_string().contains("duplicate column name") => Ok(()),
        Err(e) => Err(OrbitError::Store(e.to_string())),
    }
}

fn migrate_jobs_table_to_v2(conn: &Connection) -> Result<(), OrbitError> {
    if !table_exists(conn, "jobs")? {
        conn.execute_batch(
            r#"
                CREATE TABLE jobs (
                    id TEXT PRIMARY KEY,
                    target_type TEXT NOT NULL CHECK (target_type IN ('execution_spec','workflow')),
                    target_id TEXT NOT NULL,
                    schedule TEXT NOT NULL,
                    agent_cli TEXT NOT NULL,
                    timeout_seconds INTEGER NOT NULL,
                    retry_max_attempts INTEGER NOT NULL DEFAULT 0,
                    retry_backoff_strategy TEXT NOT NULL DEFAULT 'none',
                    retry_initial_delay_seconds INTEGER NOT NULL DEFAULT 0,
                    state TEXT NOT NULL CHECK (state IN ('enabled','paused','disabled')),
                    next_run_at TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );
            "#,
        )
        .map_err(|e| OrbitError::Store(e.to_string()))?;
        return Ok(());
    }

    if table_has_column(conn, "jobs", "target_type")? {
        return Ok(());
    }

    if table_has_column(conn, "jobs", "job_id")? {
        conn.execute_batch(
            r#"
                ALTER TABLE jobs RENAME TO jobs_v1;

                CREATE TABLE jobs (
                    id TEXT PRIMARY KEY,
                    target_type TEXT NOT NULL CHECK (target_type IN ('execution_spec','workflow')),
                    target_id TEXT NOT NULL,
                    schedule TEXT NOT NULL,
                    agent_cli TEXT NOT NULL,
                    timeout_seconds INTEGER NOT NULL,
                    retry_max_attempts INTEGER NOT NULL DEFAULT 0,
                    retry_backoff_strategy TEXT NOT NULL DEFAULT 'none',
                    retry_initial_delay_seconds INTEGER NOT NULL DEFAULT 0,
                    state TEXT NOT NULL CHECK (state IN ('enabled','paused','disabled')),
                    next_run_at TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                INSERT INTO jobs(
                    id, target_type, target_id, schedule, agent_cli, timeout_seconds,
                    retry_max_attempts, retry_backoff_strategy, retry_initial_delay_seconds,
                    state, next_run_at, created_at, updated_at
                )
                SELECT
                    job_id,
                    'execution_spec',
                    CASE WHEN task_id = '' THEN job_id ELSE task_id END,
                    schedule_spec,
                    'claude',
                    300,
                    0,
                    'none',
                    0,
                    CASE state
                        WHEN 'active' THEN 'enabled'
                        WHEN 'paused' THEN 'paused'
                        ELSE 'disabled'
                    END,
                    COALESCE(next_run_at, datetime('now')),
                    created_at,
                    updated_at
                FROM jobs_v1;

                DROP TABLE jobs_v1;
            "#,
        )
        .map_err(|e| OrbitError::Store(format!("failed v1 jobs migration: {e}")))?;

        if table_exists(conn, "job_sessions")? {
            migrate_job_sessions_to_job_runs(conn)?;
        }
        return Ok(());
    }

    if table_has_column(conn, "jobs", "command")? {
        conn.execute_batch(
            r#"
                ALTER TABLE jobs RENAME TO jobs_legacy;

                CREATE TABLE jobs (
                    id TEXT PRIMARY KEY,
                    target_type TEXT NOT NULL CHECK (target_type IN ('execution_spec','workflow')),
                    target_id TEXT NOT NULL,
                    schedule TEXT NOT NULL,
                    agent_cli TEXT NOT NULL,
                    timeout_seconds INTEGER NOT NULL,
                    retry_max_attempts INTEGER NOT NULL DEFAULT 0,
                    retry_backoff_strategy TEXT NOT NULL DEFAULT 'none',
                    retry_initial_delay_seconds INTEGER NOT NULL DEFAULT 0,
                    state TEXT NOT NULL CHECK (state IN ('enabled','paused','disabled')),
                    next_run_at TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL
                );

                INSERT INTO jobs(
                    id, target_type, target_id, schedule, agent_cli, timeout_seconds,
                    retry_max_attempts, retry_backoff_strategy, retry_initial_delay_seconds,
                    state, next_run_at, created_at, updated_at
                )
                SELECT
                    id,
                    'execution_spec',
                    id,
                    '@daily',
                    'claude',
                    300,
                    0,
                    'none',
                    0,
                    'disabled',
                    COALESCE(next_run_at, datetime('now')),
                    COALESCE(last_run_at, next_run_at, datetime('now')),
                    COALESCE(last_run_at, next_run_at, datetime('now'))
                FROM jobs_legacy;

                DROP TABLE jobs_legacy;
            "#,
        )
        .map_err(|e| OrbitError::Store(format!("failed legacy jobs migration: {e}")))?;
    }

    Ok(())
}

fn migrate_job_sessions_to_job_runs(conn: &Connection) -> Result<(), OrbitError> {
    conn.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS job_runs (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                attempt INTEGER NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('pending','running','success','failed','timeout')),
                scheduled_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT,
                duration_ms INTEGER,
                exit_code INTEGER,
                agent_response_json TEXT,
                error_code TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(job_id) REFERENCES jobs(id)
            );

            INSERT OR IGNORE INTO job_runs(
                id, job_id, attempt, state, scheduled_at, started_at, finished_at,
                duration_ms, exit_code, agent_response_json, error_code, error_message, created_at
            )
            SELECT
                session_id,
                job_id,
                1,
                CASE status
                    WHEN 'running' THEN 'running'
                    WHEN 'succeeded' THEN 'success'
                    WHEN 'failed' THEN 'failed'
                    ELSE 'failed'
                END,
                trigger_time,
                started_at,
                finished_at,
                NULL,
                exit_code,
                NULL,
                CASE WHEN status = 'failed' THEN 'LEGACY_JOB_SESSION_FAILURE' ELSE NULL END,
                error,
                created_at
            FROM job_sessions;
        "#,
    )
    .map_err(|e| OrbitError::Store(format!("failed job_sessions migration: {e}")))
}

fn ensure_job_schema_v2(conn: &Connection) -> Result<(), OrbitError> {
    conn.execute_batch(
        r#"
            CREATE INDEX IF NOT EXISTS idx_jobs_state ON jobs(state);
            CREATE INDEX IF NOT EXISTS idx_jobs_target ON jobs(target_type, target_id);
            CREATE INDEX IF NOT EXISTS idx_jobs_next_run ON jobs(state, next_run_at);

            CREATE TABLE IF NOT EXISTS job_runs (
                id TEXT PRIMARY KEY,
                job_id TEXT NOT NULL,
                attempt INTEGER NOT NULL,
                state TEXT NOT NULL CHECK (state IN ('pending','running','success','failed','timeout')),
                scheduled_at TEXT NOT NULL,
                started_at TEXT,
                finished_at TEXT,
                duration_ms INTEGER,
                exit_code INTEGER,
                agent_response_json TEXT,
                error_code TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                FOREIGN KEY(job_id) REFERENCES jobs(id)
            );

            CREATE INDEX IF NOT EXISTS idx_job_runs_job
            ON job_runs(job_id, created_at);

            CREATE INDEX IF NOT EXISTS idx_job_runs_state
            ON job_runs(state);

            CREATE UNIQUE INDEX IF NOT EXISTS uq_job_runs_single_running
            ON job_runs(job_id)
            WHERE state = 'running';
        "#,
    )
    .map_err(|e| OrbitError::Store(e.to_string()))
}

fn ensure_execution_targets_schema(conn: &Connection) -> Result<(), OrbitError> {
    conn.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS execution_specs (
                id TEXT PRIMARY KEY,
                type TEXT NOT NULL,
                description TEXT NOT NULL,
                input_schema_json TEXT NOT NULL,
                output_schema_json TEXT NOT NULL,
                artifact_path_template TEXT,
                skill_refs_json TEXT,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_execution_specs_type
            ON execution_specs(type);

            CREATE INDEX IF NOT EXISTS idx_execution_specs_active
            ON execution_specs(is_active);

            CREATE TABLE IF NOT EXISTS workflows (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                definition_json TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_workflows_active
            ON workflows(is_active);
        "#,
    )
    .map_err(|e| OrbitError::Store(e.to_string()))
}

fn ensure_audit_events_schema(conn: &Connection) -> Result<(), OrbitError> {
    conn.execute_batch(
        r#"
            CREATE TABLE IF NOT EXISTS audit_events (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                execution_id TEXT NOT NULL,
                timestamp TEXT NOT NULL,
                command TEXT NOT NULL,
                subcommand TEXT,
                tool_name TEXT,
                target_type TEXT,
                target_id TEXT,
                role TEXT NOT NULL,
                status TEXT NOT NULL,
                exit_code INTEGER NOT NULL,
                duration_ms INTEGER NOT NULL,
                working_directory TEXT NOT NULL,
                arguments_json TEXT,
                stdout_truncated TEXT,
                stderr_truncated TEXT,
                error_message TEXT,
                host TEXT,
                pid INTEGER NOT NULL,
                session_id TEXT
            );

            CREATE INDEX IF NOT EXISTS idx_audit_events_timestamp
            ON audit_events(timestamp);

            CREATE INDEX IF NOT EXISTS idx_audit_events_tool_name
            ON audit_events(tool_name);

            CREATE INDEX IF NOT EXISTS idx_audit_events_status
            ON audit_events(status);

            CREATE INDEX IF NOT EXISTS idx_audit_events_role
            ON audit_events(role);

            CREATE INDEX IF NOT EXISTS idx_audit_events_target
            ON audit_events(target_type, target_id);

            CREATE UNIQUE INDEX IF NOT EXISTS idx_audit_events_execution_id
            ON audit_events(execution_id);
        "#,
    )
    .map_err(|e| OrbitError::Store(e.to_string()))
}

fn table_exists(conn: &Connection, table: &str) -> Result<bool, OrbitError> {
    let exists: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name = ?1",
            [table],
            |row| row.get(0),
        )
        .map_err(|e| OrbitError::Store(e.to_string()))?;
    Ok(exists > 0)
}

fn table_has_column(conn: &Connection, table: &str, column: &str) -> Result<bool, OrbitError> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn
        .prepare(&pragma)
        .map_err(|e| OrbitError::Store(e.to_string()))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| OrbitError::Store(e.to_string()))?;

    for name in rows {
        let name = name.map_err(|e| OrbitError::Store(e.to_string()))?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::apply_schema;
    use rusqlite::Connection;

    #[test]
    fn apply_schema_migrates_legacy_jobs_before_index_creation() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch(
            r#"
                CREATE TABLE jobs (
                    id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    command TEXT NOT NULL,
                    next_run_at TEXT,
                    last_run_at TEXT,
                    last_status TEXT
                );
            "#,
        )
        .expect("legacy jobs");

        apply_schema(&conn).expect("apply schema");

        let mut stmt = conn.prepare("PRAGMA table_info(jobs)").expect("table info");
        let mut rows = stmt.query([]).expect("query");
        let mut saw_target_type = false;
        while let Some(row) = rows.next().expect("row") {
            let name: String = row.get(1).expect("name");
            if name == "target_type" {
                saw_target_type = true;
            }
        }
        assert!(saw_target_type);

        let run_table_exists: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='job_runs'",
                [],
                |row| row.get(0),
            )
            .expect("query job_runs");
        assert_eq!(run_table_exists, 1);
    }

    #[test]
    fn apply_schema_migrates_v1_jobs_to_v2() {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch(
            r#"
                CREATE TABLE jobs (
                    job_id TEXT PRIMARY KEY,
                    name TEXT NOT NULL,
                    task_id TEXT NOT NULL,
                    schedule_spec TEXT NOT NULL,
                    timezone TEXT NOT NULL,
                    state TEXT NOT NULL,
                    created_at TEXT NOT NULL,
                    updated_at TEXT NOT NULL,
                    paused_at TEXT,
                    deleted_at TEXT,
                    last_run_session_id TEXT,
                    last_run_at TEXT,
                    next_run_at TEXT,
                    last_error TEXT
                );

                INSERT INTO jobs(
                    job_id, name, task_id, schedule_spec, timezone, state,
                    created_at, updated_at, next_run_at
                ) VALUES (
                    'job-1', 'demo', 'task-1', '0 * * * *', 'UTC', 'active',
                    '2026-02-23T00:00:00Z', '2026-02-23T00:00:00Z', '2026-02-23T01:00:00Z'
                );
            "#,
        )
        .expect("v1 jobs");

        apply_schema(&conn).expect("apply schema");

        let (target_type, state): (String, String) = conn
            .query_row(
                "SELECT target_type, state FROM jobs WHERE id = 'job-1'",
                [],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("query migrated job");
        assert_eq!(target_type, "execution_spec");
        assert_eq!(state, "enabled");
    }
}
