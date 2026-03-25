use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkspaceStatus {
    Active,
    Invalid,
}

impl fmt::Display for WorkspaceStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WorkspaceStatus::Active => write!(f, "active"),
            WorkspaceStatus::Invalid => write!(f, "invalid"),
        }
    }
}

impl FromStr for WorkspaceStatus {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "active" => Ok(WorkspaceStatus::Active),
            "invalid" => Ok(WorkspaceStatus::Invalid),
            other => Err(format!("unknown workspace status: {other}")),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Workspace {
    pub id: String,
    pub name: String,
    pub root: PathBuf,
    pub orbit_dir: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_remote: Option<String>,
    #[serde(default = "default_base_branch")]
    pub base_branch: String,
    #[serde(default = "default_status")]
    pub status: WorkspaceStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

fn default_base_branch() -> String {
    "main".to_string()
}

fn default_status() -> WorkspaceStatus {
    WorkspaceStatus::Active
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct WorkspaceRegistry {
    #[serde(default)]
    pub workspaces: Vec<Workspace>,
    #[serde(default)]
    pub path_overrides: HashMap<PathBuf, String>,
}

/// Derived directory layout for a workspace.
///
/// All sub-paths are derived from `orbit_dir` in the constructor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePaths {
    pub repo_root: PathBuf,
    pub orbit_dir: PathBuf,
    pub global_dir: PathBuf,
    pub tasks_dir: PathBuf,
    pub activities_dir: PathBuf,
    pub jobs_dir: PathBuf,
    pub runs_dir: PathBuf,
    pub skills_dir: PathBuf,
    pub scoreboard_dir: PathBuf,
    pub diagnostics_dir: PathBuf,
}

impl WorkspacePaths {
    pub fn new(repo_root: PathBuf, orbit_dir: PathBuf, global_dir: PathBuf) -> Self {
        Self {
            tasks_dir: orbit_dir.join("tasks"),
            activities_dir: orbit_dir.join("activities"),
            jobs_dir: orbit_dir.join("jobs"),
            runs_dir: orbit_dir.join("runs"),
            skills_dir: orbit_dir.join("skills"),
            scoreboard_dir: orbit_dir.join("scoreboard"),
            diagnostics_dir: orbit_dir.join("diagnostics"),
            repo_root,
            orbit_dir,
            global_dir,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_json_round_trip() {
        let ws = Workspace {
            id: "ws_orbit".to_string(),
            name: "orbit".to_string(),
            root: PathBuf::from("/abs/path"),
            orbit_dir: PathBuf::from("/abs/path/.orbit"),
            git_remote: Some("git@github.com:user/repo.git".to_string()),
            base_branch: "main".to_string(),
            status: WorkspaceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&ws).expect("serialize");
        let decoded: Workspace = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.id, ws.id);
        assert_eq!(decoded.name, ws.name);
        assert_eq!(decoded.root, ws.root);
        assert_eq!(decoded.git_remote, ws.git_remote);
        assert_eq!(decoded.base_branch, "main");
        assert_eq!(decoded.status, WorkspaceStatus::Active);
    }

    #[test]
    fn registry_json_round_trip() {
        let registry = WorkspaceRegistry {
            workspaces: vec![Workspace {
                id: "ws_test".to_string(),
                name: "test".to_string(),
                root: PathBuf::from("/tmp/test"),
                orbit_dir: PathBuf::from("/tmp/test/.orbit"),
                git_remote: None,
                base_branch: "develop".to_string(),
                status: WorkspaceStatus::Active,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            }],
            path_overrides: HashMap::from([(
                PathBuf::from("/home/user/projects"),
                "ws_test".to_string(),
            )]),
        };
        let json = serde_json::to_string_pretty(&registry).expect("serialize");
        let decoded: WorkspaceRegistry = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(decoded.workspaces.len(), 1);
        assert_eq!(decoded.path_overrides.len(), 1);
        assert_eq!(decoded.workspaces[0].git_remote, None);
    }

    #[test]
    fn workspace_without_optional_fields_deserializes() {
        let json = r#"{
            "id": "ws_1",
            "name": "test",
            "root": "/tmp",
            "orbit_dir": "/tmp/.orbit",
            "created_at": "2026-01-01T00:00:00Z",
            "updated_at": "2026-01-01T00:00:00Z"
        }"#;
        let ws: Workspace = serde_json::from_str(json).expect("deserialize");
        assert_eq!(ws.git_remote, None);
        assert_eq!(ws.base_branch, "main");
        assert_eq!(ws.status, WorkspaceStatus::Active);
    }

    #[test]
    fn empty_registry_deserializes() {
        let json = "{}";
        let registry: WorkspaceRegistry = serde_json::from_str(json).expect("deserialize");
        assert!(registry.workspaces.is_empty());
        assert!(registry.path_overrides.is_empty());
    }

    #[test]
    fn workspace_status_display_and_parse() {
        assert_eq!(WorkspaceStatus::Active.to_string(), "active");
        assert_eq!(WorkspaceStatus::Invalid.to_string(), "invalid");
        assert_eq!(
            "active".parse::<WorkspaceStatus>().unwrap(),
            WorkspaceStatus::Active
        );
        assert_eq!(
            "invalid".parse::<WorkspaceStatus>().unwrap(),
            WorkspaceStatus::Invalid
        );
        assert!("bogus".parse::<WorkspaceStatus>().is_err());
    }

    #[test]
    fn git_remote_omitted_from_json_when_none() {
        let ws = Workspace {
            id: "ws_1".to_string(),
            name: "test".to_string(),
            root: PathBuf::from("/tmp"),
            orbit_dir: PathBuf::from("/tmp/.orbit"),
            git_remote: None,
            base_branch: "main".to_string(),
            status: WorkspaceStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let json = serde_json::to_string(&ws).expect("serialize");
        assert!(!json.contains("git_remote"));
    }

    #[test]
    fn workspace_paths_derives_subdirs() {
        let paths = WorkspacePaths::new(
            PathBuf::from("/repo"),
            PathBuf::from("/repo/.orbit"),
            PathBuf::from("/home/user/.orbit"),
        );

        assert_eq!(paths.repo_root, PathBuf::from("/repo"));
        assert_eq!(paths.orbit_dir, PathBuf::from("/repo/.orbit"));
        assert_eq!(paths.global_dir, PathBuf::from("/home/user/.orbit"));
        assert_eq!(paths.tasks_dir, PathBuf::from("/repo/.orbit/tasks"));
        assert_eq!(paths.activities_dir, PathBuf::from("/repo/.orbit/activities"));
        assert_eq!(paths.jobs_dir, PathBuf::from("/repo/.orbit/jobs"));
        assert_eq!(paths.runs_dir, PathBuf::from("/repo/.orbit/runs"));
        assert_eq!(paths.skills_dir, PathBuf::from("/repo/.orbit/skills"));
        assert_eq!(paths.scoreboard_dir, PathBuf::from("/repo/.orbit/scoreboard"));
        assert_eq!(paths.diagnostics_dir, PathBuf::from("/repo/.orbit/diagnostics"));
    }

    #[test]
    fn workspace_paths_handles_nested_orbit_dir() {
        let paths = WorkspacePaths::new(
            PathBuf::from("/a/b/c"),
            PathBuf::from("/a/b/c/.data/.orbit"),
            PathBuf::from("/global"),
        );

        assert_eq!(paths.tasks_dir, PathBuf::from("/a/b/c/.data/.orbit/tasks"));
        assert_eq!(paths.runs_dir, PathBuf::from("/a/b/c/.data/.orbit/runs"));
    }
}
