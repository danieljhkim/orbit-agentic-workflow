use std::path::{Path, PathBuf};

use orbit_store::{ResolvedScope, ScopeResolution};
use orbit_types::WorkspacePaths;
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
                Some(ws) => ResolvedScope::Layered {
                    global: self.global_path.clone(),
                    workspace: ws.clone(),
                },
                None => ResolvedScope::Single(self.global_path.clone()),
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

    /// Two-root defaults (raw paths). Delegates to [`Self::from_workspace_paths`].
    pub(crate) fn default_for_roots(global_root: &Path, workspace_root: &Path) -> Self {
        let repo_root = workspace_root
            .parent()
            .unwrap_or(workspace_root)
            .to_path_buf();
        let paths = WorkspacePaths::new(
            repo_root,
            workspace_root.to_path_buf(),
            global_root.to_path_buf(),
        );
        Self::from_workspace_paths(&paths)
    }

    /// Build persistence config from [`WorkspacePaths`]. This is the **single
    /// source of truth** for which artifact gets which scope resolution strategy.
    pub(crate) fn from_workspace_paths(paths: &WorkspacePaths) -> Self {
        let ws = if paths.global_dir == paths.orbit_dir {
            None
        } else {
            Some(&paths.orbit_dir)
        };

        Self {
            // Tasks are workspace-local: each repo tracks its own task backlog.
            task: ArtifactScope {
                global_path: paths.global_dir.join("tasks"),
                workspace_path: ws.map(|_| paths.tasks_dir.clone()),
                resolution: ScopeResolution::WorkspaceOnly,
            },
            // Activities and jobs use MergeByKey.
            activity: ArtifactScope {
                global_path: paths.global_dir.join("activities"),
                workspace_path: ws.map(|_| paths.activities_dir.clone()),
                resolution: ScopeResolution::MergeByKey,
            },
            job: ArtifactScope {
                global_path: paths.global_dir.join("jobs"),
                workspace_path: ws.map(|_| paths.jobs_dir.clone()),
                resolution: ScopeResolution::MergeByKey,
            },
            // Skills use WorkspaceReplaces.
            skill: ArtifactScope {
                global_path: paths.global_dir.join("skills"),
                workspace_path: ws.map(|_| paths.skills_dir.clone()),
                resolution: ScopeResolution::WorkspaceReplaces,
            },
            // Audit is a single global database.
            audit: ArtifactScope {
                global_path: paths.global_dir.join("orbit.db"),
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
