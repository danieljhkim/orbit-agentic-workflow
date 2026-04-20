use std::path::PathBuf;

use orbit_common::types::OrbitError;

use super::JobFileStore;

impl JobFileStore {
    pub(super) fn ensure_layout(&self) -> Result<(), OrbitError> {
        // runs_dir is NOT created here: job runs are WorkspaceOnly per scoping rules
        // and must not be initialized at global scope. write_run creates run dirs
        // on demand via fs::create_dir_all.
        Ok(())
    }

    pub(super) fn runs_dir(&self) -> PathBuf {
        self.runs_root.clone()
    }

    pub(super) fn run_dir(&self, job_id: &str) -> PathBuf {
        self.runs_dir().join(job_id)
    }

    /// Path to the run bundle directory: `<runs_dir>/<job_id>/<run_id>/`
    pub(super) fn run_bundle_dir(&self, job_id: &str, run_id: &str) -> PathBuf {
        self.run_dir(job_id).join(run_id)
    }

    pub(super) fn archived_runs_dir(&self) -> PathBuf {
        self.runs_dir().join("archived")
    }

    pub(super) fn archived_run_dir(&self, job_id: &str) -> PathBuf {
        self.archived_runs_dir().join(job_id)
    }

    /// Path to the archived run bundle directory: `<archived_runs_dir>/<job_id>/<run_id>/`
    pub(super) fn archived_run_bundle_dir(&self, job_id: &str, run_id: &str) -> PathBuf {
        self.archived_run_dir(job_id).join(run_id)
    }
}
