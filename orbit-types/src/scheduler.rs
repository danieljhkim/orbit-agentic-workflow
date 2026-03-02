use std::fmt::{Display, Formatter};
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::OrbitId;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum SchedulerTargetType {
    #[cfg_attr(feature = "clap", value(name = "job", alias = "job"))]
    Job,
}

impl Display for SchedulerTargetType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerTargetType::Job => write!(f, "job"),
        }
    }
}

impl FromStr for SchedulerTargetType {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "job" => Ok(SchedulerTargetType::Job),
            other => Err(format!("unknown scheduler target type: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum SchedulerScheduleState {
    Enabled,
    Paused,
    Disabled,
}

impl Display for SchedulerScheduleState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerScheduleState::Enabled => write!(f, "enabled"),
            SchedulerScheduleState::Paused => write!(f, "paused"),
            SchedulerScheduleState::Disabled => write!(f, "disabled"),
        }
    }
}

impl FromStr for SchedulerScheduleState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "enabled" | "active" => Ok(SchedulerScheduleState::Enabled),
            "paused" => Ok(SchedulerScheduleState::Paused),
            "disabled" | "deleted" => Ok(SchedulerScheduleState::Disabled),
            other => Err(format!("unknown scheduler state: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum SchedulerRetryBackoffStrategy {
    None,
    Fixed,
    Exponential,
}

impl Display for SchedulerRetryBackoffStrategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerRetryBackoffStrategy::None => write!(f, "none"),
            SchedulerRetryBackoffStrategy::Fixed => write!(f, "fixed"),
            SchedulerRetryBackoffStrategy::Exponential => write!(f, "exponential"),
        }
    }
}

impl FromStr for SchedulerRetryBackoffStrategy {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "none" => Ok(SchedulerRetryBackoffStrategy::None),
            "fixed" => Ok(SchedulerRetryBackoffStrategy::Fixed),
            "exponential" => Ok(SchedulerRetryBackoffStrategy::Exponential),
            other => Err(format!("unknown retry backoff strategy: {other}")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum SchedulerRunState {
    Pending,
    Running,
    Success,
    Failed,
    Timeout,
}

impl Display for SchedulerRunState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerRunState::Pending => write!(f, "pending"),
            SchedulerRunState::Running => write!(f, "running"),
            SchedulerRunState::Success => write!(f, "success"),
            SchedulerRunState::Failed => write!(f, "failed"),
            SchedulerRunState::Timeout => write!(f, "timeout"),
        }
    }
}

impl FromStr for SchedulerRunState {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "pending" => Ok(SchedulerRunState::Pending),
            "running" => Ok(SchedulerRunState::Running),
            "success" => Ok(SchedulerRunState::Success),
            "failed" => Ok(SchedulerRunState::Failed),
            "timeout" => Ok(SchedulerRunState::Timeout),
            other => Err(format!("unknown scheduler run state: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentRunError {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentResponseEnvelope {
    #[serde(rename = "schemaVersion")]
    pub schema_version: u32,
    pub status: String,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<AgentRunError>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Scheduler {
    pub scheduler_id: OrbitId,
    pub target_type: SchedulerTargetType,
    pub target_id: OrbitId,
    pub schedule: String,
    pub agent_cli: String,
    pub timeout_seconds: u64,
    pub retry_max_attempts: u32,
    pub retry_backoff_strategy: SchedulerRetryBackoffStrategy,
    pub retry_initial_delay_seconds: u64,
    pub state: SchedulerScheduleState,
    pub next_run_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SchedulerRun {
    pub run_id: OrbitId,
    pub scheduler_id: OrbitId,
    pub attempt: u32,
    pub state: SchedulerRunState,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,
    pub exit_code: Option<i32>,
    pub agent_response_json: Option<Value>,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}
