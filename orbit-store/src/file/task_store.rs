use std::fs;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use orbit_types::{OrbitError, Task, TaskPriority, TaskStatus, TaskType};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub(crate) struct TaskFileStore {
    root: PathBuf,
}

#[derive(Clone)]
pub(crate) struct FileTaskInsert {
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

#[derive(Default, Clone)]
pub(crate) struct FileTaskUpdate {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TaskStateDir {
    Todo,
    InProgress,
    Blocked,
    Done,
    Archived,
}

impl TaskStateDir {
    fn as_dir(self) -> &'static str {
        match self {
            TaskStateDir::Todo => "todo",
            TaskStateDir::InProgress => "in_progress",
            TaskStateDir::Blocked => "blocked",
            TaskStateDir::Done => "done",
            TaskStateDir::Archived => "archived",
        }
    }

    fn to_status(self) -> TaskStatus {
        match self {
            TaskStateDir::Todo => TaskStatus::Todo,
            TaskStateDir::InProgress => TaskStatus::InProgress,
            TaskStateDir::Blocked => TaskStatus::Blocked,
            TaskStateDir::Done => TaskStatus::Done,
            TaskStateDir::Archived => TaskStatus::Cancelled,
        }
    }

    fn from_status(status: TaskStatus) -> Self {
        match status {
            TaskStatus::Todo => TaskStateDir::Todo,
            TaskStatus::InProgress => TaskStateDir::InProgress,
            TaskStatus::Blocked => TaskStateDir::Blocked,
            TaskStatus::Done => TaskStateDir::Done,
            TaskStatus::Cancelled => TaskStateDir::Archived,
        }
    }

    fn all() -> [TaskStateDir; 5] {
        [
            TaskStateDir::Todo,
            TaskStateDir::InProgress,
            TaskStateDir::Blocked,
            TaskStateDir::Done,
            TaskStateDir::Archived,
        ]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TaskFileDocument {
    schema_version: u8,
    id: String,
    title: String,
    description: String,
    #[serde(default)]
    instructions: String,
    #[serde(default)]
    context_files: Vec<String>,
    #[serde(default)]
    workspace_path: Option<String>,
    #[serde(default)]
    identity_id: Option<String>,
    #[serde(default)]
    assigned_to: Option<String>,
    #[serde(default)]
    created_by: Option<String>,
    #[serde(default)]
    approved_at: Option<DateTime<Utc>>,
    #[serde(default)]
    approved_by: Option<String>,
    #[serde(default)]
    approval_note: Option<String>,
    priority: TaskPriority,
    #[serde(rename = "type", default = "default_task_type")]
    task_type: TaskType,
    #[serde(default)]
    owner: String,
    #[serde(default)]
    parent_id: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    acceptance_criteria: Vec<String>,
    #[serde(default)]
    history: Vec<TaskHistoryEntry>,
    #[serde(default)]
    comments: Vec<TaskComment>,
    #[serde(default)]
    job_id: Option<String>,
    #[serde(default)]
    scheduler_id: Option<String>,
    #[serde(default)]
    scheduler_run_id: Option<String>,
    #[serde(default)]
    auto_escalated: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskHistoryEntry {
    at: DateTime<Utc>,
    by: String,
    event: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskComment {
    at: DateTime<Utc>,
    by: String,
    message: String,
}

fn default_task_type() -> TaskType {
    TaskType::Task
}

impl TaskFileStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn ensure_layout(&self) -> Result<(), OrbitError> {
        for state in TaskStateDir::all() {
            fs::create_dir_all(self.root.join(state.as_dir()))
                .map_err(|e| OrbitError::Io(e.to_string()))?;
        }
        Ok(())
    }

    pub(crate) fn create_task(&self, params: FileTaskInsert) -> Result<Task, OrbitError> {
        self.ensure_layout()?;
        if params.title.trim().is_empty() {
            return Err(OrbitError::InvalidInput(
                "task title must not be empty".to_string(),
            ));
        }

        let now = Utc::now();
        let id = self.next_task_id(now)?;
        let history_actor = params
            .created_by
            .clone()
            .unwrap_or_else(|| "human".to_string());
        let doc = TaskFileDocument {
            schema_version: 1,
            id,
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
            created_at: now,
            updated_at: now,
            tags: Vec::new(),
            acceptance_criteria: Vec::new(),
            history: vec![TaskHistoryEntry {
                at: now,
                by: history_actor,
                event: "created".to_string(),
            }],
            comments: Vec::new(),
            job_id: None,
            scheduler_id: None,
            scheduler_run_id: None,
            auto_escalated: None,
        };

        self.write_doc_for_state(TaskStateDir::Todo, &doc)?;
        Ok(doc_to_task(TaskStateDir::Todo, doc))
    }

    pub(crate) fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        let mut tasks = Vec::new();
        for state in TaskStateDir::all() {
            let dir = self.state_dir_path(state);
            if !dir.exists() {
                continue;
            }
            let mut paths = fs::read_dir(&dir)
                .map_err(|e| OrbitError::Io(e.to_string()))?
                .filter_map(Result::ok)
                .map(|entry| entry.path())
                .filter(|path| is_yaml(path))
                .collect::<Vec<_>>();
            paths.sort();

            for path in paths {
                let doc = self.read_doc_at(&path)?;
                tasks.push(doc_to_task(state, doc));
            }
        }

        tasks.sort_by(|a, b| {
            b.created_at
                .cmp(&a.created_at)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(tasks)
    }

    pub(crate) fn list_tasks_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
    ) -> Result<Vec<Task>, OrbitError> {
        let tasks = self.list_tasks()?;
        Ok(tasks
            .into_iter()
            .filter(|task| status.is_none_or(|value| task.status == value))
            .filter(|task| priority.is_none_or(|value| task.priority == value))
            .collect())
    }

    pub(crate) fn get_task(&self, id: &str) -> Result<Option<Task>, OrbitError> {
        let Some((state, path)) = self.locate_task(id)? else {
            return Ok(None);
        };
        let doc = self.read_doc_at(&path)?;
        Ok(Some(doc_to_task(state, doc)))
    }

    pub(crate) fn search_tasks(&self, query: &str) -> Result<Vec<Task>, OrbitError> {
        let lowered = query.to_lowercase();
        let tasks = self.list_tasks()?;
        Ok(tasks
            .into_iter()
            .filter(|task| {
                task.title.to_lowercase().contains(&lowered)
                    || task.description.to_lowercase().contains(&lowered)
            })
            .collect())
    }

    pub(crate) fn update_task(
        &self,
        id: &str,
        fields: &FileTaskUpdate,
    ) -> Result<Task, OrbitError> {
        let Some((current_state, current_path)) = self.locate_task(id)? else {
            return Err(OrbitError::TaskNotFound(id.to_string()));
        };
        let mut doc = self.read_doc_at(&current_path)?;

        if let Some(value) = &fields.title {
            doc.title = value.clone();
        }
        if let Some(value) = &fields.description {
            doc.description = value.clone();
        }
        if let Some(value) = &fields.instructions {
            doc.instructions = value.clone();
        }
        if let Some(value) = &fields.context_files {
            doc.context_files = value.clone();
        }
        if let Some(value) = &fields.workspace_path {
            doc.workspace_path = value.clone();
        }
        if let Some(value) = &fields.identity_id {
            doc.identity_id = value.clone();
        }
        if let Some(value) = &fields.assigned_to {
            doc.assigned_to = value.clone();
        }
        if let Some(value) = &fields.created_by {
            doc.created_by = value.clone();
        }
        if let Some(value) = &fields.approved_at {
            doc.approved_at = *value;
        }
        if let Some(value) = &fields.approved_by {
            doc.approved_by = value.clone();
        }
        if let Some(value) = &fields.approval_note {
            doc.approval_note = value.clone();
        }
        if let Some(value) = fields.priority {
            doc.priority = value;
        }
        if let Some(value) = fields.task_type {
            doc.task_type = value;
        }
        if let Some(value) = &fields.owner {
            doc.owner = value.clone();
        }
        if let Some(value) = &fields.parent_id {
            doc.parent_id = value.clone();
        }

        let target_state = fields
            .status
            .map(TaskStateDir::from_status)
            .unwrap_or(current_state);

        let event = if target_state == current_state {
            None
        } else if target_state == TaskStateDir::Done {
            Some("closed".to_string())
        } else {
            Some("moved".to_string())
        };

        doc.updated_at = Utc::now();
        if let Some(event) = event {
            doc.history.push(TaskHistoryEntry {
                at: doc.updated_at,
                by: "human".to_string(),
                event,
            });
        }

        self.validate_doc(&doc)?;
        let target_path = self.task_path(target_state, &doc.id);
        self.write_doc_at(&target_path, &doc)?;
        if target_path != current_path {
            fs::remove_file(&current_path).map_err(|e| OrbitError::Io(e.to_string()))?;
        }

        Ok(doc_to_task(target_state, doc))
    }

    pub(crate) fn delete_task(&self, id: &str) -> Result<bool, OrbitError> {
        let Some((_, path)) = self.locate_task(id)? else {
            return Ok(false);
        };
        fs::remove_file(path).map_err(|e| OrbitError::Io(e.to_string()))?;
        Ok(true)
    }

    fn next_task_id(&self, now: DateTime<Utc>) -> Result<String, OrbitError> {
        for attempt in 0..1024_u32 {
            let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
            let candidate = if attempt == 0 {
                format!("T{}-{nanos}", now.format("%Y%m%d-%H%M%S"))
            } else {
                format!("T{}-{nanos}-{attempt}", now.format("%Y%m%d-%H%M%S"))
            };
            if self.locate_task(&candidate)?.is_none() {
                return Ok(candidate);
            }
        }
        Err(OrbitError::Execution(
            "unable to allocate unique task id".to_string(),
        ))
    }

    fn locate_task(&self, id: &str) -> Result<Option<(TaskStateDir, PathBuf)>, OrbitError> {
        for state in TaskStateDir::all() {
            let path = self.task_path(state, id);
            if path.exists() {
                return Ok(Some((state, path)));
            }
        }
        Ok(None)
    }

    fn write_doc_for_state(
        &self,
        state: TaskStateDir,
        doc: &TaskFileDocument,
    ) -> Result<(), OrbitError> {
        self.validate_doc(doc)?;
        let path = self.task_path(state, &doc.id);
        self.write_doc_at(&path, doc)
    }

    fn write_doc_at(&self, path: &Path, doc: &TaskFileDocument) -> Result<(), OrbitError> {
        self.validate_doc(doc)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| OrbitError::Io(e.to_string()))?;
        }

        let yaml = serde_yaml::to_string(doc).map_err(|e| OrbitError::Store(e.to_string()))?;
        let nanos = Utc::now().timestamp_nanos_opt().unwrap_or_default();
        let tmp_path = path.with_extension(format!("yaml.tmp.{nanos}"));
        fs::write(&tmp_path, yaml).map_err(|e| OrbitError::Io(e.to_string()))?;
        if let Err(err) = fs::rename(&tmp_path, path) {
            let _ = fs::remove_file(&tmp_path);
            return Err(OrbitError::Io(err.to_string()));
        }
        Ok(())
    }

    fn read_doc_at(&self, path: &Path) -> Result<TaskFileDocument, OrbitError> {
        let raw = fs::read_to_string(path).map_err(|e| OrbitError::Io(e.to_string()))?;
        let doc = serde_yaml::from_str::<TaskFileDocument>(&raw)
            .map_err(|e| OrbitError::Store(format!("invalid task file {}: {e}", path.display())))?;
        self.validate_doc(&doc)?;
        Ok(doc)
    }

    fn validate_doc(&self, doc: &TaskFileDocument) -> Result<(), OrbitError> {
        if doc.schema_version != 1 {
            return Err(OrbitError::InvalidInput(format!(
                "unsupported task schema version: {}",
                doc.schema_version
            )));
        }
        if doc.id.trim().is_empty() {
            return Err(OrbitError::InvalidInput(
                "task id must not be empty".to_string(),
            ));
        }
        if doc.title.trim().is_empty() {
            return Err(OrbitError::InvalidInput(
                "task title must not be empty".to_string(),
            ));
        }
        Ok(())
    }

    fn state_dir_path(&self, state: TaskStateDir) -> PathBuf {
        self.root.join(state.as_dir())
    }

    fn task_path(&self, state: TaskStateDir, id: &str) -> PathBuf {
        self.state_dir_path(state).join(format!("{id}.yaml"))
    }
}

fn is_yaml(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|s| s.to_str()),
        Some("yaml") | Some("yml")
    )
}

fn doc_to_task(state: TaskStateDir, doc: TaskFileDocument) -> Task {
    Task {
        id: doc.id,
        title: doc.title,
        description: doc.description,
        instructions: doc.instructions,
        context_files: doc.context_files,
        workspace_path: doc.workspace_path,
        identity_id: doc.identity_id,
        assigned_to: doc.assigned_to,
        created_by: doc.created_by,
        approved_at: doc.approved_at,
        approved_by: doc.approved_by,
        approval_note: doc.approval_note,
        status: state.to_status(),
        priority: doc.priority,
        task_type: doc.task_type,
        owner: doc.owner,
        parent_id: doc.parent_id,
        created_at: doc.created_at,
        updated_at: doc.updated_at,
    }
}
