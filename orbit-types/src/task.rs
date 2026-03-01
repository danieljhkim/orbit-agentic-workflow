use std::fmt::{Display, Formatter};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::OrbitId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
    Blocked,
    Cancelled,
}

impl Display for TaskStatus {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskStatus::Todo => "todo",
            TaskStatus::InProgress => "in_progress",
            TaskStatus::Done => "done",
            TaskStatus::Blocked => "blocked",
            TaskStatus::Cancelled => "cancelled",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TaskStatus {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "todo" => Ok(TaskStatus::Todo),
            "in_progress" => Ok(TaskStatus::InProgress),
            "done" => Ok(TaskStatus::Done),
            "blocked" => Ok(TaskStatus::Blocked),
            "cancelled" => Ok(TaskStatus::Cancelled),
            other => Err(format!("unknown task status: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    Medium,
    High,
    Critical,
}

impl Display for TaskPriority {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskPriority::Low => "low",
            TaskPriority::Medium => "medium",
            TaskPriority::High => "high",
            TaskPriority::Critical => "critical",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TaskPriority {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "low" => Ok(TaskPriority::Low),
            "medium" => Ok(TaskPriority::Medium),
            "high" => Ok(TaskPriority::High),
            "critical" => Ok(TaskPriority::Critical),
            other => Err(format!("unknown task priority: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Task,
    Feature,
    #[value(alias = "bug")]
    Issue,
    Other,
}

impl Display for TaskType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            TaskType::Task => "task",
            TaskType::Feature => "feature",
            TaskType::Issue => "issue",
            TaskType::Other => "other",
        };
        write!(f, "{s}")
    }
}

impl FromStr for TaskType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "task" => Ok(TaskType::Task),
            "feature" => Ok(TaskType::Feature),
            "issue" => Ok(TaskType::Issue),
            "bug" => Ok(TaskType::Issue),
            "other" => Ok(TaskType::Other),
            other => Err(format!("unknown task type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Task {
    pub id: OrbitId,
    pub title: String,
    pub description: String,
    pub instructions: String,
    pub context_files: Vec<String>,
    #[serde(default)]
    pub workspace_path: Option<String>,
    pub status: TaskStatus,
    pub priority: TaskPriority,
    pub task_type: TaskType,
    pub owner: String,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Display for Task {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\t{}\t{}\t{}",
            self.id, self.status, self.priority, self.title
        )
    }
}
