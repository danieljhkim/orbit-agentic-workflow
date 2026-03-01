use std::path::PathBuf;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use orbit_types::{
    AgentSession, AgentSessionStatus, AgentToolCall, Audit, AuditEvent, Job,
    JobRetryBackoffStrategy, JobRun, JobRunState, JobScheduleState, JobTargetType, OrbitError,
    OrbitEvent, StoredTool, Task, TaskPriority, TaskStatus, TaskType, Watch, Work,
};
use serde_json::Value;

use crate::file::job_store::JobFileStore;
use crate::file::task_store::{FileTaskInsert, FileTaskUpdate, TaskFileStore};
use crate::file::work_store::{FileWorkInsert, WorkFileStore};
use crate::sqlite::audit_event_store::{AuditEventFilter, AuditEventInsertParams};
use crate::sqlite::job_store::DueJobsClaim;
use crate::sqlite::task_store::{TaskInsertParams, TaskUpdateFields};
use crate::{Store, WorkInsertParams};

#[derive(Debug, Clone)]
pub struct TaskCreateParams {
    pub title: String,
    pub description: String,
    pub instructions: String,
    pub context_files: Vec<String>,
    pub workspace_path: Option<String>,
    pub identity_id: Option<String>,
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
    pub approved_at: Option<DateTime<Utc>>,
    pub approved_by: Option<String>,
    pub approval_note: Option<String>,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub owner: String,
    pub parent_id: Option<String>,
}

#[derive(Debug, Default, Clone)]
pub struct TaskUpdateParams {
    pub title: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub context_files: Option<Vec<String>>,
    pub workspace_path: Option<Option<String>>,
    pub identity_id: Option<Option<String>>,
    pub assigned_to: Option<Option<String>>,
    pub created_by: Option<Option<String>>,
    pub approved_at: Option<Option<DateTime<Utc>>>,
    pub approved_by: Option<Option<String>>,
    pub approval_note: Option<Option<String>>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub owner: Option<String>,
    pub parent_id: Option<Option<String>>,
}

#[derive(Debug, Clone)]
pub struct WorkCreateParams {
    pub id: String,
    pub spec_type: String,
    pub description: String,
    pub input_schema_json: Value,
    pub output_schema_json: Value,
    pub artifact_path_template: Option<String>,
    pub skill_refs: Vec<String>,
    pub identity_id: Option<String>,
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone)]
pub struct JobCreateParams {
    pub target_type: JobTargetType,
    pub target_id: String,
    pub schedule: String,
    pub agent_cli: String,
    pub timeout_seconds: u64,
    pub retry_max_attempts: u32,
    pub retry_backoff_strategy: JobRetryBackoffStrategy,
    pub retry_initial_delay_seconds: u64,
    pub next_run_at: DateTime<Utc>,
}

pub trait TaskStoreBackend: Send + Sync {
    fn create_task(&self, params: TaskCreateParams) -> Result<Task, OrbitError>;
    fn list_tasks(&self) -> Result<Vec<Task>, OrbitError>;
    fn list_tasks_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
    ) -> Result<Vec<Task>, OrbitError>;
    fn get_task(&self, id: &str) -> Result<Option<Task>, OrbitError>;
    fn search_tasks(&self, query: &str) -> Result<Vec<Task>, OrbitError>;
    fn update_task(&self, id: &str, params: TaskUpdateParams) -> Result<Task, OrbitError>;
    fn delete_task(&self, id: &str) -> Result<bool, OrbitError>;
}

pub trait WorkStoreBackend: Send + Sync {
    fn add_work(&self, params: WorkCreateParams) -> Result<Work, OrbitError>;
    fn list_works(&self, include_inactive: bool) -> Result<Vec<Work>, OrbitError>;
    fn get_work(&self, id: &str) -> Result<Option<Work>, OrbitError>;
    fn disable_work(&self, id: &str) -> Result<bool, OrbitError>;
}

pub trait JobStoreBackend: Send + Sync {
    fn add_job(&self, params: JobCreateParams) -> Result<Job, OrbitError>;
    fn list_jobs(&self, include_disabled: bool) -> Result<Vec<Job>, OrbitError>;
    fn get_job(&self, job_id: &str) -> Result<Option<Job>, OrbitError>;
    fn due_jobs(&self, now: DateTime<Utc>) -> Result<Vec<Job>, OrbitError>;
    fn list_job_runs(&self, job_id: &str) -> Result<Vec<JobRun>, OrbitError>;
    fn get_pending_or_running_job_run(&self, job_id: &str) -> Result<Option<JobRun>, OrbitError>;
    fn set_job_state(&self, job_id: &str, state: JobScheduleState) -> Result<bool, OrbitError>;
    fn mark_job_disabled(&self, job_id: &str) -> Result<bool, OrbitError>;
    fn update_job_next_run(
        &self,
        job_id: &str,
        next_run_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError>;
    fn insert_job_run(
        &self,
        job_id: &str,
        attempt: u32,
        scheduled_at: DateTime<Utc>,
    ) -> Result<JobRun, OrbitError>;
    fn mark_job_run_running(
        &self,
        run_id: &str,
        started_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError>;
    #[allow(clippy::too_many_arguments)]
    fn complete_job_run(
        &self,
        run_id: &str,
        state: JobRunState,
        finished_at: DateTime<Utc>,
        duration_ms: Option<u64>,
        exit_code: Option<i32>,
        agent_response_json: Option<&Value>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<bool, OrbitError>;
    fn claim_due_jobs(&self, now: DateTime<Utc>) -> Result<DueJobsClaim, OrbitError>;
}

pub trait ToolStoreBackend: Send + Sync {
    fn list_tools(&self) -> Result<Vec<StoredTool>, OrbitError>;
    fn get_tool(&self, name: &str) -> Result<Option<StoredTool>, OrbitError>;
    fn insert_tool(&self, tool: &StoredTool) -> Result<(), OrbitError>;
    fn delete_tool(&self, name: &str) -> Result<bool, OrbitError>;
    fn set_tool_enabled(&self, name: &str, enabled: bool) -> Result<bool, OrbitError>;
}

pub trait WatchStoreBackend: Send + Sync {
    fn list_watches(&self) -> Result<Vec<Watch>, OrbitError>;
    fn get_watch(&self, id: &str) -> Result<Option<Watch>, OrbitError>;
    fn insert_watch(
        &self,
        path: &str,
        command: &str,
        debounce_ms: u64,
    ) -> Result<Watch, OrbitError>;
}

pub trait AuditStoreBackend: Send + Sync {
    fn list_audits(&self, limit: usize) -> Result<Vec<Audit>, OrbitError>;
    fn insert_audit_event(&self, event: &OrbitEvent) -> Result<(), OrbitError>;
}

pub trait AuditEventStoreBackend: Send + Sync {
    fn insert_audit_event_record(&self, params: &AuditEventInsertParams) -> Result<(), OrbitError>;
    fn list_audit_events(&self, filter: &AuditEventFilter) -> Result<Vec<AuditEvent>, OrbitError>;
    fn get_audit_event(&self, id: i64) -> Result<Option<AuditEvent>, OrbitError>;
    fn get_audit_event_stats(
        &self,
        since: Option<&DateTime<Utc>>,
        tool: Option<&str>,
    ) -> Result<(i64, i64, i64, i64, f64, i64), OrbitError>;
    fn get_audit_event_durations(
        &self,
        since: Option<&DateTime<Utc>>,
        tool: Option<&str>,
    ) -> Result<Vec<i64>, OrbitError>;
    fn prune_audit_events(&self, older_than: &DateTime<Utc>) -> Result<usize, OrbitError>;
}

pub trait AgentSessionStoreBackend: Send + Sync {
    fn get_agent_session(&self, session_id: &str) -> Result<Option<AgentSession>, OrbitError>;
    fn insert_agent_session(&self, session: &AgentSession) -> Result<(), OrbitError>;
    fn update_agent_session(
        &self,
        session_id: &str,
        tool_calls: &[AgentToolCall],
        outcome: &str,
        status: AgentSessionStatus,
    ) -> Result<bool, OrbitError>;
}

pub trait LockStoreBackend: Send + Sync {
    fn try_lock(&self, name: &str) -> Result<bool, OrbitError>;
    fn unlock(&self, name: &str) -> Result<bool, OrbitError>;
    fn global_job_lock_name(&self) -> &'static str;
}

pub fn task_store_file(root: PathBuf) -> Result<Arc<dyn TaskStoreBackend>, OrbitError> {
    let store = TaskFileStore::new(root);
    store.ensure_layout()?;
    Ok(Arc::new(store))
}

pub fn task_store_sqlite(store: Store) -> Arc<dyn TaskStoreBackend> {
    Arc::new(SqliteTaskStoreBackend { store })
}

pub fn work_store_file(root: PathBuf) -> Result<Arc<dyn WorkStoreBackend>, OrbitError> {
    let store = WorkFileStore::new(root);
    store.ensure_layout()?;
    Ok(Arc::new(store))
}

pub fn work_store_sqlite(store: Store) -> Arc<dyn WorkStoreBackend> {
    Arc::new(SqliteWorkStoreBackend { store })
}

pub fn job_store_file(root: PathBuf) -> Result<Arc<dyn JobStoreBackend>, OrbitError> {
    let store = JobFileStore::new(root);
    store.ensure_layout()?;
    Ok(Arc::new(store))
}

pub fn job_store_sqlite(store: Store) -> Arc<dyn JobStoreBackend> {
    Arc::new(SqliteJobStoreBackend { store })
}

pub fn tool_store_sqlite(store: Store) -> Arc<dyn ToolStoreBackend> {
    Arc::new(SqliteToolStoreBackend { store })
}

pub fn watch_store_sqlite(store: Store) -> Arc<dyn WatchStoreBackend> {
    Arc::new(SqliteWatchStoreBackend { store })
}

pub fn audit_store_sqlite(store: Store) -> Arc<dyn AuditStoreBackend> {
    Arc::new(SqliteAuditStoreBackend { store })
}

pub fn audit_event_store_sqlite(store: Store) -> Arc<dyn AuditEventStoreBackend> {
    Arc::new(SqliteAuditEventStoreBackend { store })
}

pub fn agent_session_store_sqlite(store: Store) -> Arc<dyn AgentSessionStoreBackend> {
    Arc::new(SqliteAgentSessionStoreBackend { store })
}

pub fn lock_store_sqlite(store: Store) -> Arc<dyn LockStoreBackend> {
    Arc::new(SqliteLockStoreBackend { store })
}

impl TaskStoreBackend for TaskFileStore {
    fn create_task(&self, params: TaskCreateParams) -> Result<Task, OrbitError> {
        self.create_task(FileTaskInsert {
            title: params.title,
            description: params.description,
            instructions: params.instructions,
            context_files: params.context_files,
            workspace_path: params.workspace_path,
            identity_id: params.identity_id,
            assigned_to: params.assigned_to,
            created_by: params.created_by,
            approved_at: params.approved_at,
            approved_by: params.approved_by,
            approval_note: params.approval_note,
            priority: params.priority,
            task_type: params.task_type,
            owner: params.owner,
            parent_id: params.parent_id,
        })
    }

    fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        self.list_tasks()
    }

    fn list_tasks_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
    ) -> Result<Vec<Task>, OrbitError> {
        self.list_tasks_filtered(status, priority)
    }

    fn get_task(&self, id: &str) -> Result<Option<Task>, OrbitError> {
        self.get_task(id)
    }

    fn search_tasks(&self, query: &str) -> Result<Vec<Task>, OrbitError> {
        self.search_tasks(query)
    }

    fn update_task(&self, id: &str, params: TaskUpdateParams) -> Result<Task, OrbitError> {
        self.update_task(
            id,
            &FileTaskUpdate {
                title: params.title,
                description: params.description,
                instructions: params.instructions,
                context_files: params.context_files,
                workspace_path: params.workspace_path,
                identity_id: params.identity_id,
                assigned_to: params.assigned_to,
                created_by: params.created_by,
                approved_at: params.approved_at,
                approved_by: params.approved_by,
                approval_note: params.approval_note,
                status: params.status,
                priority: params.priority,
                task_type: params.task_type,
                owner: params.owner,
                parent_id: params.parent_id,
            },
        )
    }

    fn delete_task(&self, id: &str) -> Result<bool, OrbitError> {
        self.delete_task(id)
    }
}

impl WorkStoreBackend for WorkFileStore {
    fn add_work(&self, params: WorkCreateParams) -> Result<Work, OrbitError> {
        self.insert_work(&FileWorkInsert {
            id: params.id,
            spec_type: params.spec_type,
            description: params.description,
            input_schema_json: params.input_schema_json,
            output_schema_json: params.output_schema_json,
            artifact_path_template: params.artifact_path_template,
            skill_refs: params.skill_refs,
            identity_id: params.identity_id,
            assigned_to: params.assigned_to,
            created_by: params.created_by,
        })
    }

    fn list_works(&self, include_inactive: bool) -> Result<Vec<Work>, OrbitError> {
        self.list_works(include_inactive)
    }

    fn get_work(&self, id: &str) -> Result<Option<Work>, OrbitError> {
        self.get_work(id)
    }

    fn disable_work(&self, id: &str) -> Result<bool, OrbitError> {
        self.disable_work(id)
    }
}

impl JobStoreBackend for JobFileStore {
    fn add_job(&self, params: JobCreateParams) -> Result<Job, OrbitError> {
        self.insert_job_v2(
            params.target_type,
            &params.target_id,
            &params.schedule,
            &params.agent_cli,
            params.timeout_seconds,
            params.retry_max_attempts,
            params.retry_backoff_strategy,
            params.retry_initial_delay_seconds,
            params.next_run_at,
        )
    }

    fn list_jobs(&self, include_disabled: bool) -> Result<Vec<Job>, OrbitError> {
        self.list_jobs(include_disabled)
    }

    fn get_job(&self, job_id: &str) -> Result<Option<Job>, OrbitError> {
        self.get_job(job_id)
    }

    fn due_jobs(&self, now: DateTime<Utc>) -> Result<Vec<Job>, OrbitError> {
        self.due_jobs(now)
    }

    fn list_job_runs(&self, job_id: &str) -> Result<Vec<JobRun>, OrbitError> {
        self.list_job_runs(job_id)
    }

    fn get_pending_or_running_job_run(&self, job_id: &str) -> Result<Option<JobRun>, OrbitError> {
        self.get_pending_or_running_job_run(job_id)
    }

    fn set_job_state(&self, job_id: &str, state: JobScheduleState) -> Result<bool, OrbitError> {
        self.set_job_state(job_id, state)
    }

    fn mark_job_disabled(&self, job_id: &str) -> Result<bool, OrbitError> {
        self.mark_job_disabled(job_id)
    }

    fn update_job_next_run(
        &self,
        job_id: &str,
        next_run_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError> {
        self.update_job_next_run(job_id, next_run_at)
    }

    fn insert_job_run(
        &self,
        job_id: &str,
        attempt: u32,
        scheduled_at: DateTime<Utc>,
    ) -> Result<JobRun, OrbitError> {
        self.insert_job_run(job_id, attempt, scheduled_at)
    }

    fn mark_job_run_running(
        &self,
        run_id: &str,
        started_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError> {
        self.mark_job_run_running(run_id, started_at)
    }

    fn complete_job_run(
        &self,
        run_id: &str,
        state: JobRunState,
        finished_at: DateTime<Utc>,
        duration_ms: Option<u64>,
        exit_code: Option<i32>,
        agent_response_json: Option<&Value>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<bool, OrbitError> {
        self.complete_job_run(
            run_id,
            state,
            finished_at,
            duration_ms,
            exit_code,
            agent_response_json,
            error_code,
            error_message,
        )
    }

    fn claim_due_jobs(&self, now: DateTime<Utc>) -> Result<DueJobsClaim, OrbitError> {
        self.claim_due_jobs(now)
    }
}

#[derive(Clone)]
struct SqliteTaskStoreBackend {
    store: Store,
}

impl TaskStoreBackend for SqliteTaskStoreBackend {
    fn create_task(&self, params: TaskCreateParams) -> Result<Task, OrbitError> {
        let task = self.store.with_transaction(|tx| {
            tx.insert_task(&TaskInsertParams {
                title: params.title.clone(),
                description: params.description.clone(),
                instructions: params.instructions.clone(),
                context_files: params.context_files.clone(),
                workspace_path: params.workspace_path.clone(),
                identity_id: params.identity_id.clone(),
                assigned_to: params.assigned_to.clone(),
                created_by: params.created_by.clone(),
                priority: params.priority,
                task_type: params.task_type,
                owner: params.owner.clone(),
                parent_id: params.parent_id.clone(),
            })
        })?;

        if params.approved_at.is_some()
            || params.approved_by.is_some()
            || params.approval_note.is_some()
        {
            self.store.with_transaction(|tx| {
                let _ = tx.update_task(
                    &task.id,
                    &TaskUpdateFields {
                        approved_at: Some(params.approved_at),
                        approved_by: Some(params.approved_by.clone()),
                        approval_note: Some(params.approval_note.clone()),
                        ..Default::default()
                    },
                )?;
                Ok(())
            })?;
        }

        self.get_task(&task.id)?
            .ok_or_else(|| OrbitError::TaskNotFound(task.id))
    }

    fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        self.store.list_tasks()
    }

    fn list_tasks_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
    ) -> Result<Vec<Task>, OrbitError> {
        self.store.list_tasks_filtered(status, priority)
    }

    fn get_task(&self, id: &str) -> Result<Option<Task>, OrbitError> {
        self.store.get_task(id)
    }

    fn search_tasks(&self, query: &str) -> Result<Vec<Task>, OrbitError> {
        self.store.search_tasks(query)
    }

    fn update_task(&self, id: &str, params: TaskUpdateParams) -> Result<Task, OrbitError> {
        let changed = self.store.with_transaction(|tx| {
            tx.update_task(
                id,
                &TaskUpdateFields {
                    title: params.title.clone(),
                    description: params.description.clone(),
                    instructions: params.instructions.clone(),
                    context_files: params.context_files.clone(),
                    workspace_path: params.workspace_path,
                    identity_id: params.identity_id.clone(),
                    assigned_to: params.assigned_to.clone(),
                    created_by: params.created_by.clone(),
                    approved_at: params.approved_at,
                    approved_by: params.approved_by.clone(),
                    approval_note: params.approval_note.clone(),
                    status: params.status,
                    priority: params.priority,
                    task_type: params.task_type,
                    owner: params.owner.clone(),
                    parent_id: params.parent_id.clone(),
                },
            )
        })?;

        if !changed {
            return Err(OrbitError::TaskNotFound(id.to_string()));
        }

        self.get_task(id)?
            .ok_or_else(|| OrbitError::TaskNotFound(id.to_string()))
    }

    fn delete_task(&self, id: &str) -> Result<bool, OrbitError> {
        self.store.with_transaction(|tx| tx.delete_task(id))
    }
}

#[derive(Clone)]
struct SqliteWorkStoreBackend {
    store: Store,
}

impl WorkStoreBackend for SqliteWorkStoreBackend {
    fn add_work(&self, params: WorkCreateParams) -> Result<Work, OrbitError> {
        self.store.with_transaction(|tx| {
            tx.insert_work(&WorkInsertParams {
                id: params.id.clone(),
                spec_type: params.spec_type.clone(),
                description: params.description.clone(),
                input_schema_json: params.input_schema_json.clone(),
                output_schema_json: params.output_schema_json.clone(),
                artifact_path_template: params.artifact_path_template.clone(),
                skill_refs: params.skill_refs.clone(),
                identity_id: params.identity_id.clone(),
                assigned_to: params.assigned_to.clone(),
                created_by: params.created_by.clone(),
            })
        })
    }

    fn list_works(&self, include_inactive: bool) -> Result<Vec<Work>, OrbitError> {
        self.store.list_works(include_inactive)
    }

    fn get_work(&self, id: &str) -> Result<Option<Work>, OrbitError> {
        self.store.get_work(id)
    }

    fn disable_work(&self, id: &str) -> Result<bool, OrbitError> {
        self.store.with_transaction(|tx| tx.disable_work(id))
    }
}

#[derive(Clone)]
struct SqliteJobStoreBackend {
    store: Store,
}

impl JobStoreBackend for SqliteJobStoreBackend {
    fn add_job(&self, params: JobCreateParams) -> Result<Job, OrbitError> {
        self.store.with_transaction(|tx| {
            tx.insert_job_v2(
                params.target_type,
                &params.target_id,
                &params.schedule,
                &params.agent_cli,
                params.timeout_seconds,
                params.retry_max_attempts,
                params.retry_backoff_strategy,
                params.retry_initial_delay_seconds,
                params.next_run_at,
            )
        })
    }

    fn list_jobs(&self, include_disabled: bool) -> Result<Vec<Job>, OrbitError> {
        self.store.list_jobs(include_disabled)
    }

    fn get_job(&self, job_id: &str) -> Result<Option<Job>, OrbitError> {
        self.store.get_job(job_id)
    }

    fn due_jobs(&self, now: DateTime<Utc>) -> Result<Vec<Job>, OrbitError> {
        self.store.due_jobs(now)
    }

    fn list_job_runs(&self, job_id: &str) -> Result<Vec<JobRun>, OrbitError> {
        self.store.list_job_runs(job_id)
    }

    fn get_pending_or_running_job_run(&self, job_id: &str) -> Result<Option<JobRun>, OrbitError> {
        self.store.get_pending_or_running_job_run(job_id)
    }

    fn set_job_state(&self, job_id: &str, state: JobScheduleState) -> Result<bool, OrbitError> {
        self.store
            .with_transaction(|tx| tx.set_job_state(job_id, state))
    }

    fn mark_job_disabled(&self, job_id: &str) -> Result<bool, OrbitError> {
        self.store
            .with_transaction(|tx| tx.mark_job_disabled(job_id))
    }

    fn update_job_next_run(
        &self,
        job_id: &str,
        next_run_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError> {
        self.store
            .with_transaction(|tx| tx.update_job_next_run(job_id, next_run_at))
    }

    fn insert_job_run(
        &self,
        job_id: &str,
        attempt: u32,
        scheduled_at: DateTime<Utc>,
    ) -> Result<JobRun, OrbitError> {
        self.store
            .with_transaction(|tx| tx.insert_job_run(job_id, attempt, scheduled_at))
    }

    fn mark_job_run_running(
        &self,
        run_id: &str,
        started_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError> {
        self.store
            .with_transaction(|tx| tx.mark_job_run_running(run_id, started_at))
    }

    fn complete_job_run(
        &self,
        run_id: &str,
        state: JobRunState,
        finished_at: DateTime<Utc>,
        duration_ms: Option<u64>,
        exit_code: Option<i32>,
        agent_response_json: Option<&Value>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<bool, OrbitError> {
        self.store.with_transaction(|tx| {
            tx.complete_job_run(
                run_id,
                state,
                finished_at,
                duration_ms,
                exit_code,
                agent_response_json,
                error_code,
                error_message,
            )
        })
    }

    fn claim_due_jobs(&self, now: DateTime<Utc>) -> Result<DueJobsClaim, OrbitError> {
        self.store.with_transaction(|tx| tx.claim_due_jobs(now))
    }
}

#[derive(Clone)]
struct SqliteToolStoreBackend {
    store: Store,
}

impl ToolStoreBackend for SqliteToolStoreBackend {
    fn list_tools(&self) -> Result<Vec<StoredTool>, OrbitError> {
        self.store.list_tools()
    }

    fn get_tool(&self, name: &str) -> Result<Option<StoredTool>, OrbitError> {
        self.store.get_tool(name)
    }

    fn insert_tool(&self, tool: &StoredTool) -> Result<(), OrbitError> {
        self.store.with_transaction(|tx| tx.insert_tool(tool))
    }

    fn delete_tool(&self, name: &str) -> Result<bool, OrbitError> {
        self.store.with_transaction(|tx| tx.delete_tool(name))
    }

    fn set_tool_enabled(&self, name: &str, enabled: bool) -> Result<bool, OrbitError> {
        self.store
            .with_transaction(|tx| tx.set_tool_enabled(name, enabled))
    }
}

#[derive(Clone)]
struct SqliteWatchStoreBackend {
    store: Store,
}

impl WatchStoreBackend for SqliteWatchStoreBackend {
    fn list_watches(&self) -> Result<Vec<Watch>, OrbitError> {
        self.store.list_watches()
    }

    fn get_watch(&self, id: &str) -> Result<Option<Watch>, OrbitError> {
        self.store.get_watch(id)
    }

    fn insert_watch(
        &self,
        path: &str,
        command: &str,
        debounce_ms: u64,
    ) -> Result<Watch, OrbitError> {
        self.store
            .with_transaction(|tx| tx.insert_watch(path, command, debounce_ms))
    }
}

#[derive(Clone)]
struct SqliteAuditStoreBackend {
    store: Store,
}

impl AuditStoreBackend for SqliteAuditStoreBackend {
    fn list_audits(&self, limit: usize) -> Result<Vec<Audit>, OrbitError> {
        self.store.list_audits(limit)
    }

    fn insert_audit_event(&self, event: &OrbitEvent) -> Result<(), OrbitError> {
        self.store
            .with_transaction(|tx| tx.insert_audit_event(event))
    }
}

#[derive(Clone)]
struct SqliteAuditEventStoreBackend {
    store: Store,
}

impl AuditEventStoreBackend for SqliteAuditEventStoreBackend {
    fn insert_audit_event_record(&self, params: &AuditEventInsertParams) -> Result<(), OrbitError> {
        self.store.insert_audit_event_record(params)
    }

    fn list_audit_events(&self, filter: &AuditEventFilter) -> Result<Vec<AuditEvent>, OrbitError> {
        self.store.list_audit_events(filter)
    }

    fn get_audit_event(&self, id: i64) -> Result<Option<AuditEvent>, OrbitError> {
        self.store.get_audit_event(id)
    }

    fn get_audit_event_stats(
        &self,
        since: Option<&DateTime<Utc>>,
        tool: Option<&str>,
    ) -> Result<(i64, i64, i64, i64, f64, i64), OrbitError> {
        self.store.get_audit_event_stats(since, tool)
    }

    fn get_audit_event_durations(
        &self,
        since: Option<&DateTime<Utc>>,
        tool: Option<&str>,
    ) -> Result<Vec<i64>, OrbitError> {
        self.store.get_audit_event_durations(since, tool)
    }

    fn prune_audit_events(&self, older_than: &DateTime<Utc>) -> Result<usize, OrbitError> {
        self.store.prune_audit_events(older_than)
    }
}

#[derive(Clone)]
struct SqliteAgentSessionStoreBackend {
    store: Store,
}

impl AgentSessionStoreBackend for SqliteAgentSessionStoreBackend {
    fn get_agent_session(&self, session_id: &str) -> Result<Option<AgentSession>, OrbitError> {
        self.store.get_agent_session(session_id)
    }

    fn insert_agent_session(&self, session: &AgentSession) -> Result<(), OrbitError> {
        self.store
            .with_transaction(|tx| tx.insert_agent_session(session))
    }

    fn update_agent_session(
        &self,
        session_id: &str,
        tool_calls: &[AgentToolCall],
        outcome: &str,
        status: AgentSessionStatus,
    ) -> Result<bool, OrbitError> {
        self.store
            .with_transaction(|tx| tx.update_agent_session(session_id, tool_calls, outcome, status))
    }
}

#[derive(Clone)]
struct SqliteLockStoreBackend {
    store: Store,
}

impl LockStoreBackend for SqliteLockStoreBackend {
    fn try_lock(&self, name: &str) -> Result<bool, OrbitError> {
        self.store.try_lock(name)
    }

    fn unlock(&self, name: &str) -> Result<bool, OrbitError> {
        self.store.unlock(name)
    }

    fn global_job_lock_name(&self) -> &'static str {
        Store::global_job_lock_name()
    }
}
