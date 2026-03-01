use chrono::Utc;
use orbit_types::{IdentityRole, OrbitError, OrbitEvent, Task, TaskPriority, TaskStatus, TaskType};
use std::path::Path;

use crate::OrbitRuntime;
use crate::task_file_store::{FileTaskInsert, FileTaskUpdate};

pub struct TaskAddParams {
    pub title: String,
    pub description: String,
    pub instructions: String,
    pub context_files: Vec<String>,
    pub workspace_path: Option<String>,
    pub identity_id: Option<String>,
    pub assigned_to: Option<String>,
    pub created_by: Option<String>,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub owner: String,
    pub parent_id: Option<String>,
}

impl Default for TaskAddParams {
    fn default() -> Self {
        Self {
            title: String::new(),
            description: String::new(),
            instructions: String::new(),
            context_files: Vec::new(),
            workspace_path: None,
            identity_id: None,
            assigned_to: None,
            created_by: None,
            priority: TaskPriority::Medium,
            task_type: TaskType::Task,
            owner: String::new(),
            parent_id: None,
        }
    }
}

pub struct TaskUpdateParams {
    pub title: Option<String>,
    pub description: Option<String>,
    pub instructions: Option<String>,
    pub context_files: Option<Vec<String>>,
    pub workspace_path: Option<Option<String>>,
    pub identity_id: Option<Option<String>>,
    pub assigned_to: Option<Option<String>>,
    pub created_by: Option<Option<String>>,
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub owner: Option<String>,
    pub parent_id: Option<Option<String>>,
}

impl OrbitRuntime {
    pub fn add_task(&self, params: TaskAddParams) -> Result<Task, OrbitError> {
        if let Some(ref parent) = params.parent_id {
            let exists = self.context.task_store.get_task(parent)?;
            if exists.is_none() {
                return Err(OrbitError::TaskNotFound(format!(
                    "parent task not found: {parent}"
                )));
            }
        }

        let workspace_path = normalize_workspace_path(params.workspace_path)?;
        let identity_id = params.identity_id.clone();
        let mut assigned_to = params.assigned_to.clone();
        let mut created_by = params.created_by.clone();
        if let Some(id) = identity_id.as_ref() {
            let resolved = self.resolve_identity(id)?;
            if assigned_to.is_none() {
                assigned_to = Some(resolved.name.clone());
            }
            if created_by.is_none() {
                created_by = Some(resolved.name);
            }
        }
        let auto_approve = !self.context.task_approval_required_for_agent;
        let auto_approval_note = if auto_approve {
            Some(
                "Auto-approved by configuration (task.approval.required_for_agent=false)"
                    .to_string(),
            )
        } else {
            None
        };

        self.with_mutation(|_| {
            let task = self.context.task_store.create_task(FileTaskInsert {
                title: params.title.clone(),
                description: params.description.clone(),
                instructions: params.instructions.clone(),
                context_files: params.context_files.clone(),
                workspace_path: workspace_path.clone(),
                identity_id: identity_id.clone(),
                assigned_to: assigned_to.clone(),
                created_by: created_by.clone(),
                approved_at: auto_approve.then_some(Utc::now()),
                approved_by: if auto_approve {
                    Some(created_by.clone().unwrap_or_else(|| "system".to_string()))
                } else {
                    None
                },
                approval_note: auto_approval_note.clone(),
                priority: params.priority,
                task_type: params.task_type,
                owner: params.owner.clone(),
                parent_id: params.parent_id.clone(),
            })?;
            Ok((
                task.clone(),
                OrbitEvent::TaskAdded {
                    id: task.id.clone(),
                },
            ))
        })
    }

    pub fn get_task(&self, id: &str) -> Result<Task, OrbitError> {
        self.context
            .task_store
            .get_task(id)?
            .ok_or_else(|| OrbitError::TaskNotFound(id.to_string()))
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        self.context.task_store.list_tasks()
    }

    pub fn list_tasks_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
    ) -> Result<Vec<Task>, OrbitError> {
        self.context
            .task_store
            .list_tasks_filtered(status, priority)
    }

    pub fn update_task(&self, id: &str, params: TaskUpdateParams) -> Result<Task, OrbitError> {
        // Verify task exists
        self.get_task(id)?;

        if let Some(Some(ref parent)) = params.parent_id {
            let exists = self.context.task_store.get_task(parent)?;
            if exists.is_none() {
                return Err(OrbitError::TaskNotFound(format!(
                    "parent task not found: {parent}"
                )));
            }
        }

        if let Some(Some(identity_id)) = params.identity_id.as_ref() {
            let _ = self.resolve_identity(identity_id)?;
        }

        let workspace_path = match params.workspace_path {
            Some(value) => Some(normalize_workspace_path(value)?),
            None => None,
        };

        let task = self.with_mutation(|_| {
            let task = self.context.task_store.update_task(
                id,
                &FileTaskUpdate {
                    title: params.title,
                    description: params.description,
                    instructions: params.instructions,
                    context_files: params.context_files,
                    workspace_path,
                    identity_id: params.identity_id,
                    assigned_to: params.assigned_to,
                    created_by: params.created_by,
                    approved_at: None,
                    approved_by: None,
                    approval_note: None,
                    status: params.status,
                    priority: params.priority,
                    task_type: params.task_type,
                    owner: params.owner,
                    parent_id: params.parent_id,
                },
            )?;
            Ok((task.clone(), OrbitEvent::TaskUpdated { id: id.to_string() }))
        })?;

        Ok(task)
    }

    pub fn close_task(&self, id: &str) -> Result<(), OrbitError> {
        let task = self.get_task(id)?;

        if task.status == TaskStatus::Done || task.status == TaskStatus::Cancelled {
            return Err(OrbitError::InvalidInput(format!(
                "task '{id}' is already {}",
                task.status
            )));
        }

        self.with_mutation(|_| {
            let _ = self.context.task_store.update_task(
                id,
                &FileTaskUpdate {
                    status: Some(TaskStatus::Done),
                    ..Default::default()
                },
            )?;
            Ok(((), OrbitEvent::TaskClosed { id: id.to_string() }))
        })
    }

    pub fn reopen_task(&self, id: &str) -> Result<(), OrbitError> {
        let task = self.get_task(id)?;

        if task.status != TaskStatus::Done && task.status != TaskStatus::Cancelled {
            return Err(OrbitError::InvalidInput(format!(
                "task '{id}' is not closed (status: {})",
                task.status
            )));
        }

        self.with_mutation(|_| {
            let _ = self.context.task_store.update_task(
                id,
                &FileTaskUpdate {
                    status: Some(TaskStatus::Todo),
                    ..Default::default()
                },
            )?;
            Ok(((), OrbitEvent::TaskReopened { id: id.to_string() }))
        })
    }

    pub fn delete_task(&self, id: &str) -> Result<(), OrbitError> {
        self.with_mutation(|_| {
            let deleted = self.context.task_store.delete_task(id)?;
            if !deleted {
                return Err(OrbitError::TaskNotFound(id.to_string()));
            }
            Ok(((), OrbitEvent::TaskDeleted { id: id.to_string() }))
        })
    }

    pub fn search_tasks(&self, query: &str) -> Result<Vec<Task>, OrbitError> {
        self.context.task_store.search_tasks(query)
    }

    pub fn approve_task(
        &self,
        id: &str,
        approved_by: &str,
        approval_note: Option<String>,
    ) -> Result<Task, OrbitError> {
        let task = self.get_task(id)?;
        let approver = approved_by.trim();
        if approver.is_empty() {
            return Err(OrbitError::InvalidInput(
                "approved_by must not be empty".to_string(),
            ));
        }
        if task.approved_at.is_some() {
            return Ok(task);
        }

        let task = self.with_mutation(|_| {
            let task = self.context.task_store.update_task(
                id,
                &FileTaskUpdate {
                    approved_at: Some(Some(Utc::now())),
                    approved_by: Some(Some(approver.to_string())),
                    approval_note: Some(approval_note.clone()),
                    ..Default::default()
                },
            )?;
            Ok((
                task.clone(),
                OrbitEvent::TaskApproved {
                    id: id.to_string(),
                    approved_by: approver.to_string(),
                },
            ))
        })?;

        Ok(task)
    }

    pub fn approve_task_from_session(
        &self,
        id: &str,
        session_id: &str,
        approval_note: Option<String>,
    ) -> Result<Task, OrbitError> {
        if !self.context.task_delegate_approval {
            return Err(OrbitError::TaskApprovalRequired(
                "delegated approval is disabled (task.approval.delegate_approval=false)"
                    .to_string(),
            ));
        }

        let session = self
            .get_agent_session(session_id)?
            .ok_or_else(|| OrbitError::AgentSessionNotFound(session_id.to_string()))?;
        if session.task_id != id {
            return Err(OrbitError::TaskApprovalRequired(format!(
                "session '{session_id}' is not attached to task '{id}'"
            )));
        }
        if session.identity_role != Some(IdentityRole::Leader) {
            return Err(OrbitError::TaskApprovalRequired(format!(
                "session '{session_id}' identity is not authorized for delegated approval"
            )));
        }
        let approver = session
            .identity_name
            .as_deref()
            .or(session.identity_id.as_deref())
            .ok_or_else(|| {
                OrbitError::TaskApprovalRequired(format!(
                    "session '{session_id}' has no identity attribution"
                ))
            })?;

        self.approve_task(id, approver, approval_note)
    }
}

fn normalize_workspace_path(raw: Option<String>) -> Result<Option<String>, OrbitError> {
    let Some(raw) = raw else {
        return Ok(None);
    };

    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let path = Path::new(trimmed);
    if !path.exists() {
        return Err(OrbitError::InvalidInput(format!(
            "workspace path does not exist: {trimmed}"
        )));
    }
    if !path.is_dir() {
        return Err(OrbitError::InvalidInput(format!(
            "workspace path is not a directory: {trimmed}"
        )));
    }

    let canonical = path.canonicalize().map_err(|e| {
        OrbitError::InvalidInput(format!(
            "failed to canonicalize workspace path '{trimmed}': {e}"
        ))
    })?;
    Ok(Some(canonical.to_string_lossy().to_string()))
}
