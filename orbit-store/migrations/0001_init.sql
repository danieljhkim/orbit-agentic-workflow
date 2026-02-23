-- v2 initial schema; runtime applies equivalent SQL via execute_batch.
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

CREATE TABLE IF NOT EXISTS jobs (
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

CREATE INDEX IF NOT EXISTS idx_jobs_state
ON jobs(state);

CREATE INDEX IF NOT EXISTS idx_jobs_target
ON jobs(target_type, target_id);

CREATE INDEX IF NOT EXISTS idx_jobs_next_run
ON jobs(state, next_run_at);

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
    updated_at TEXT NOT NULL
);
