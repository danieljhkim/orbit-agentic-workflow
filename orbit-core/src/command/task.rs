use orbit_store::task_store::{TaskInsertParams, TaskUpdateFields};
use orbit_types::{OrbitError, OrbitEvent, Task, TaskPriority, TaskStatus, TaskType};

use crate::OrbitRuntime;

pub struct TaskAddParams {
    pub title: String,
    pub description: String,
    pub instructions: String,
    pub context_files: Vec<String>,
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
    pub status: Option<TaskStatus>,
    pub priority: Option<TaskPriority>,
    pub task_type: Option<TaskType>,
    pub owner: Option<String>,
    pub parent_id: Option<Option<String>>,
}

impl OrbitRuntime {
    pub fn add_task(&self, params: TaskAddParams) -> Result<Task, OrbitError> {
        if let Some(ref parent) = params.parent_id {
            let exists = self.context.store.get_task(parent)?;
            if exists.is_none() {
                return Err(OrbitError::TaskNotFound(format!(
                    "parent task not found: {parent}"
                )));
            }
        }

        self.with_mutation(|tx| {
            let task = tx.insert_task(&TaskInsertParams {
                title: params.title.clone(),
                description: params.description.clone(),
                instructions: params.instructions.clone(),
                context_files: params.context_files.clone(),
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
            .store
            .get_task(id)?
            .ok_or_else(|| OrbitError::TaskNotFound(id.to_string()))
    }

    pub fn list_tasks(&self) -> Result<Vec<Task>, OrbitError> {
        self.context.store.list_tasks()
    }

    pub fn list_tasks_filtered(
        &self,
        status: Option<TaskStatus>,
        priority: Option<TaskPriority>,
    ) -> Result<Vec<Task>, OrbitError> {
        self.context.store.list_tasks_filtered(status, priority)
    }

    pub fn update_task(&self, id: &str, params: TaskUpdateParams) -> Result<Task, OrbitError> {
        // Verify task exists
        self.get_task(id)?;

        if let Some(Some(ref parent)) = params.parent_id {
            let exists = self.context.store.get_task(parent)?;
            if exists.is_none() {
                return Err(OrbitError::TaskNotFound(format!(
                    "parent task not found: {parent}"
                )));
            }
        }

        self.with_mutation(|tx| {
            tx.update_task(
                id,
                &TaskUpdateFields {
                    title: params.title,
                    description: params.description,
                    instructions: params.instructions,
                    context_files: params.context_files,
                    status: params.status,
                    priority: params.priority,
                    task_type: params.task_type,
                    owner: params.owner,
                    parent_id: params.parent_id,
                },
            )?;
            Ok(((), OrbitEvent::TaskUpdated { id: id.to_string() }))
        })?;

        self.get_task(id)
    }

    pub fn close_task(&self, id: &str) -> Result<(), OrbitError> {
        let task = self.get_task(id)?;

        if task.status == TaskStatus::Done || task.status == TaskStatus::Cancelled {
            return Err(OrbitError::InvalidInput(format!(
                "task '{id}' is already {}",
                task.status
            )));
        }

        self.with_mutation(|tx| {
            tx.set_task_status(id, TaskStatus::Done)?;
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

        self.with_mutation(|tx| {
            tx.set_task_status(id, TaskStatus::Todo)?;
            Ok(((), OrbitEvent::TaskReopened { id: id.to_string() }))
        })
    }

    pub fn delete_task(&self, id: &str) -> Result<(), OrbitError> {
        self.with_mutation(|tx| {
            let deleted = tx.delete_task(id)?;
            if !deleted {
                return Err(OrbitError::TaskNotFound(id.to_string()));
            }
            Ok(((), OrbitEvent::TaskDeleted { id: id.to_string() }))
        })
    }

    pub fn search_tasks(&self, query: &str) -> Result<Vec<Task>, OrbitError> {
        self.context.store.search_tasks(query)
    }
}
