use std::path::{Path, PathBuf};

use orbit_store::{ResolvedScope, ScopeResolution};
use serde_json::{Value, json};

/// Carries a global path, optional workspace path, and the resolution strategy
/// for a single artifact type. Call `resolve()` to get the effective store path(s).
#[derive(Debug, Clone)]
pub(crate) struct ArtifactScope {
    pub(crate) global_path: PathBuf,
    pub(crate) workspace_path: Option<PathBuf>,
    pub(crate) resolution: ScopeResolution,
}

impl ArtifactScope {
    pub(crate) fn resolve(&self) -> ResolvedScope {
        match self.resolution {
            ScopeResolution::GlobalOnly => ResolvedScope::Single(self.global_path.clone()),
            ScopeResolution::WorkspaceOnly => ResolvedScope::Single(
                self.workspace_path
                    .clone()
                    .unwrap_or_else(|| self.global_path.clone()),
            ),
            ScopeResolution::WorkspaceReplaces => match &self.workspace_path {
                Some(ws) if ws.is_dir() => ResolvedScope::Single(ws.clone()),
                _ => ResolvedScope::Single(self.global_path.clone()),
            },
            ScopeResolution::MergeByKey => match &self.workspace_path {
                Some(ws) if ws.is_dir() => ResolvedScope::Layered {
                    global: self.global_path.clone(),
                    workspace: ws.clone(),
                },
                _ => ResolvedScope::Single(self.global_path.clone()),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct PersistenceConfig {
    pub(crate) job: ArtifactScope,
    pub(crate) activity: ArtifactScope,
    pub(crate) skill: ArtifactScope,
    pub(crate) task: ArtifactScope,
    pub(crate) audit: ArtifactScope,
}

impl PersistenceConfig {
    pub(crate) fn default_for_data_root(data_root: &Path) -> Self {
        Self::default_for_roots(data_root, data_root)
    }

    /// Two-root defaults. This is the **single source of truth** for which
    /// artifact gets which scope resolution strategy.
    pub(crate) fn default_for_roots(global_root: &Path, workspace_root: &Path) -> Self {
        let ws = if global_root == workspace_root {
            None
        } else {
            Some(workspace_root)
        };

        Self {
            task: ArtifactScope {
                global_path: global_root.join("tasks"),
                workspace_path: ws.map(|p| p.join("tasks")),
                resolution: ScopeResolution::WorkspaceOnly,
            },
            activity: ArtifactScope {
                global_path: global_root.join("activities"),
                workspace_path: ws.map(|p| p.join("activities")),
                resolution: ScopeResolution::MergeByKey,
            },
            job: ArtifactScope {
                global_path: global_root.join("jobs"),
                workspace_path: ws.map(|p| p.join("jobs")),
                resolution: ScopeResolution::MergeByKey,
            },
            skill: ArtifactScope {
                global_path: global_root.join("skills"),
                workspace_path: ws.map(|p| p.join("skills")),
                resolution: ScopeResolution::WorkspaceReplaces,
            },
            audit: ArtifactScope {
                global_path: global_root.join("orbit.db"),
                workspace_path: None,
                resolution: ScopeResolution::GlobalOnly,
            },
        }
    }

    pub(crate) fn as_json_value(&self) -> Value {
        json!({
            "task": { "path": self.task.resolve().into_single().to_string_lossy() },
            "activity": { "path": self.activity.global_path.to_string_lossy(), "resolution": format!("{:?}", self.activity.resolution) },
            "job": { "path": self.job.global_path.to_string_lossy(), "resolution": format!("{:?}", self.job.resolution) },
            "skill": { "path": self.skill.resolve().into_single().to_string_lossy() },
            "audit": { "path": self.audit.global_path.to_string_lossy() },
        })
    }
}
