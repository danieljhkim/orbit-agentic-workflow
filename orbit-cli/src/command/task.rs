use clap::{Args, Subcommand};
use orbit_core::command::task::{TaskAddParams, TaskUpdateParams};
use orbit_core::{OrbitError, OrbitRuntime, TaskPriority, TaskStatus, TaskType};
use serde_json::{Value, json};

use crate::command::Execute;

#[derive(Args)]
pub struct TaskCommand {
    #[command(subcommand)]
    pub command: TaskSubcommand,
}

impl Execute for TaskCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum TaskSubcommand {
    /// Create a new task
    Add(TaskAddArgs),
    /// List tasks with optional filters
    List(TaskListArgs),
    /// Show detailed information about a task
    Show(TaskShowArgs),
    /// Update task fields
    Update(TaskUpdateArgs),
    /// Approve a task for agent execution
    Approve(TaskApproveArgs),
    /// Close a task (set status to done)
    Close(TaskCloseArgs),
    /// Reopen a closed task
    Reopen(TaskReopenArgs),
    /// Delete a task permanently
    Delete(TaskDeleteArgs),
    /// Search tasks by title or description
    Search(TaskSearchArgs),
}

impl Execute for TaskSubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            TaskSubcommand::Add(args) => args.execute(runtime),
            TaskSubcommand::List(args) => args.execute(runtime),
            TaskSubcommand::Show(args) => args.execute(runtime),
            TaskSubcommand::Update(args) => args.execute(runtime),
            TaskSubcommand::Approve(args) => args.execute(runtime),
            TaskSubcommand::Close(args) => args.execute(runtime),
            TaskSubcommand::Reopen(args) => args.execute(runtime),
            TaskSubcommand::Delete(args) => args.execute(runtime),
            TaskSubcommand::Search(args) => args.execute(runtime),
        }
    }
}

// --- Add ---

#[derive(Args)]
pub struct TaskAddArgs {
    /// Task title
    #[arg(long)]
    pub title: String,
    /// Task description
    #[arg(long, default_value = "")]
    pub description: String,
    /// Task instructions payload (agent planning input)
    #[arg(long, default_value = "")]
    pub instructions: String,
    /// Comma-separated context file paths
    #[arg(long, default_value = "")]
    pub context: String,
    /// Repository workspace path
    #[arg(long)]
    pub workspace: Option<String>,
    /// Optional agent identity id
    #[arg(long)]
    pub identity: Option<String>,
    /// Optional assignee display name
    #[arg(long)]
    pub assigned_to: Option<String>,
    /// Optional creator display name
    #[arg(long)]
    pub created_by: Option<String>,
    /// Priority level
    #[arg(long, value_enum, default_value_t = TaskPriority::Medium)]
    pub priority: TaskPriority,
    /// Task type
    #[arg(long = "type", value_enum, default_value_t = TaskType::Task)]
    pub task_type: TaskType,
    /// Task owner
    #[arg(long, default_value = "")]
    pub owner: String,
    /// Parent task ID
    #[arg(long)]
    pub parent: Option<String>,
}

impl Execute for TaskAddArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let task = runtime.add_task(TaskAddParams {
            title: self.title,
            description: self.description,
            instructions: self.instructions,
            context_files: parse_context_csv(&self.context),
            workspace_path: self.workspace,
            identity_id: self.identity,
            assigned_to: self.assigned_to,
            created_by: self.created_by,
            priority: self.priority,
            task_type: self.task_type,
            owner: self.owner,
            parent_id: self.parent,
        })?;

        println!("{}", task.id);
        Ok(())
    }
}

// --- List ---

#[derive(Args)]
pub struct TaskListArgs {
    /// Filter by status
    #[arg(long, value_enum)]
    pub status: Option<TaskStatus>,
    /// Filter by priority
    #[arg(long, value_enum)]
    pub priority: Option<TaskPriority>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for TaskListArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let tasks = if self.status.is_some() || self.priority.is_some() {
            runtime.list_tasks_filtered(self.status, self.priority)?
        } else {
            runtime.list_tasks()?
        };

        if self.json {
            let json_tasks: Vec<Value> = tasks.iter().map(task_to_json).collect();
            crate::output::json::print_pretty(&Value::Array(json_tasks))
        } else {
            print_task_table(&tasks);
            Ok(())
        }
    }
}

// --- Show ---

#[derive(Args)]
pub struct TaskShowArgs {
    /// Task ID
    pub id: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for TaskShowArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let task = runtime.get_task(&self.id)?;

        if self.json {
            crate::output::json::print_pretty(&task_to_json(&task))
        } else {
            println!("ID:          {}", task.id);
            println!("Title:       {}", task.title);
            println!("Status:      {}", task.status);
            println!("Priority:    {}", task.priority);
            println!("Type:        {}", task.task_type);
            if !task.description.is_empty() {
                println!("Description: {}", task.description);
            }
            if !task.instructions.is_empty() {
                println!("Instructions: {}", task.instructions);
            }
            if !task.context_files.is_empty() {
                println!("Context:     {}", task.context_files.join(", "));
            }
            if let Some(ref workspace_path) = task.workspace_path {
                println!("Workspace:   {}", workspace_path);
            }
            if let Some(ref identity_id) = task.identity_id {
                println!("Identity:    {}", identity_id);
            }
            if let Some(ref assigned_to) = task.assigned_to {
                println!("Assigned To: {}", assigned_to);
            }
            if let Some(ref created_by) = task.created_by {
                println!("Created By:  {}", created_by);
            }
            println!("Approved:    {}", yes_no(task.approved_at.is_some()));
            if let Some(ref approved_by) = task.approved_by {
                println!("Approved By: {}", approved_by);
            }
            if let Some(approved_at) = task.approved_at {
                println!("Approved At: {}", approved_at.to_rfc3339());
            }
            if let Some(ref approval_note) = task.approval_note {
                println!("Approval Note: {}", approval_note);
            }
            if !task.owner.is_empty() {
                println!("Owner:       {}", task.owner);
            }
            if let Some(ref parent) = task.parent_id {
                println!("Parent:      {}", parent);
            }
            println!("Created:     {}", task.created_at.to_rfc3339());
            println!("Updated:     {}", task.updated_at.to_rfc3339());
            Ok(())
        }
    }
}

// --- Update ---

#[derive(Args)]
pub struct TaskUpdateArgs {
    /// Task ID
    pub id: String,
    /// New title
    #[arg(long)]
    pub title: Option<String>,
    /// New description
    #[arg(long)]
    pub description: Option<String>,
    /// New instructions payload
    #[arg(long)]
    pub instructions: Option<String>,
    /// New comma-separated context files (empty string clears all)
    #[arg(long)]
    pub context: Option<String>,
    /// New workspace path (use empty string to clear)
    #[arg(long)]
    pub workspace: Option<String>,
    /// New identity id (empty string clears)
    #[arg(long)]
    pub identity: Option<String>,
    /// New assignee (empty string clears)
    #[arg(long)]
    pub assigned_to: Option<String>,
    /// New creator (empty string clears)
    #[arg(long)]
    pub created_by: Option<String>,
    /// New status
    #[arg(long, value_enum)]
    pub status: Option<TaskStatus>,
    /// New priority
    #[arg(long, value_enum)]
    pub priority: Option<TaskPriority>,
    /// New type
    #[arg(long = "type", value_enum)]
    pub task_type: Option<TaskType>,
    /// New owner
    #[arg(long)]
    pub owner: Option<String>,
    /// New parent task ID (use empty string to clear)
    #[arg(long)]
    pub parent: Option<String>,
}

impl Execute for TaskUpdateArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let parent_id = self
            .parent
            .map(|p| if p.is_empty() { None } else { Some(p) });
        let workspace_path = self.workspace.map(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        });
        let identity_id = self.identity.map(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        });
        let assigned_to = self.assigned_to.map(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        });
        let created_by = self.created_by.map(|value| {
            if value.trim().is_empty() {
                None
            } else {
                Some(value)
            }
        });

        let task = runtime.update_task(
            &self.id,
            TaskUpdateParams {
                title: self.title,
                description: self.description,
                instructions: self.instructions,
                context_files: self.context.map(|raw| parse_context_csv(&raw)),
                workspace_path,
                identity_id,
                assigned_to,
                created_by,
                status: self.status,
                priority: self.priority,
                task_type: self.task_type,
                owner: self.owner,
                parent_id,
            },
        )?;

        println!("Updated task '{}'", task.id);
        Ok(())
    }
}

// --- Approve ---

#[derive(Args)]
pub struct TaskApproveArgs {
    /// Task ID
    pub id: String,
    /// Approver identity
    #[arg(long, default_value = "human")]
    pub by: String,
    /// Optional approval note (e.g., verbal confirmation details)
    #[arg(long)]
    pub note: Option<String>,
}

impl Execute for TaskApproveArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let task = runtime.approve_task(&self.id, &self.by, self.note)?;
        println!("Approved task '{}'", task.id);
        Ok(())
    }
}

// --- Close ---

#[derive(Args)]
pub struct TaskCloseArgs {
    /// Task ID
    pub id: String,
}

impl Execute for TaskCloseArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.close_task(&self.id)?;
        println!("Closed task '{}'", self.id);
        Ok(())
    }
}

// --- Reopen ---

#[derive(Args)]
pub struct TaskReopenArgs {
    /// Task ID
    pub id: String,
}

impl Execute for TaskReopenArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.reopen_task(&self.id)?;
        println!("Reopened task '{}'", self.id);
        Ok(())
    }
}

// --- Delete ---

#[derive(Args)]
pub struct TaskDeleteArgs {
    /// Task ID
    pub id: String,
}

impl Execute for TaskDeleteArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        runtime.delete_task(&self.id)?;
        println!("Deleted task '{}'", self.id);
        Ok(())
    }
}

// --- Search ---

#[derive(Args)]
pub struct TaskSearchArgs {
    /// Search query
    pub query: String,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

impl Execute for TaskSearchArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let tasks = runtime.search_tasks(&self.query)?;

        if self.json {
            let json_tasks: Vec<Value> = tasks.iter().map(task_to_json).collect();
            crate::output::json::print_pretty(&Value::Array(json_tasks))
        } else {
            print_task_table(&tasks);
            Ok(())
        }
    }
}

// --- Helpers ---

fn print_task_table(tasks: &[orbit_core::Task]) {
    println!(
        "{:<28} {:<12} {:<8} {:<8} {:<5} TITLE",
        "ID", "STATUS", "PRI", "TYPE", "APPR"
    );
    for task in tasks {
        println!(
            "{:<28} {:<12} {:<8} {:<8} {:<5} {}",
            task.id,
            task.status,
            task.priority,
            task.task_type,
            yes_no(task.approved_at.is_some()),
            task.title
        );
    }
}

fn task_to_json(task: &orbit_core::Task) -> Value {
    json!({
        "id": task.id,
        "title": task.title,
        "description": task.description,
        "instructions": task.instructions,
        "context_files": task.context_files,
        "workspace_path": task.workspace_path,
        "identity_id": task.identity_id,
        "assigned_to": task.assigned_to,
        "created_by": task.created_by,
        "approved_at": task.approved_at.as_ref().map(|value| value.to_rfc3339()),
        "approved_by": task.approved_by,
        "approval_note": task.approval_note,
        "status": task.status.to_string(),
        "priority": task.priority.to_string(),
        "type": task.task_type.to_string(),
        "owner": task.owner,
        "parent_id": task.parent_id,
        "created_at": task.created_at.to_rfc3339(),
        "updated_at": task.updated_at.to_rfc3339(),
    })
}

fn parse_context_csv(raw: &str) -> Vec<String> {
    crate::parse::csv_to_vec(raw)
}

fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
