use orbit_common::types::OrbitError;

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn generate_scoreboard_summary(
        &self,
    ) -> Result<orbit_store::scoreboard_summary::ScoreboardSummary, OrbitError> {
        let tasks = self.list_tasks()?;
        orbit_store::friction_bounty::refresh_from_tasks(&self.paths().scoreboard_dir, &tasks)?;
        let audit_tool_calls = self.audit_tool_call_counts_by_role(None)?;
        let summary = orbit_store::scoreboard_summary::generate_summary_with_audit_tool_calls(
            &self.paths().scoreboard_dir,
            &tasks,
            &audit_tool_calls,
        )?;
        let _ =
            orbit_store::scoreboard_summary::write_summary(&self.paths().scoreboard_dir, &summary)?;
        Ok(summary)
    }

    pub fn scoreboard_summary_path(&self) -> std::path::PathBuf {
        orbit_store::scoreboard_summary::summary_path(&self.paths().scoreboard_dir)
    }

    /// Read the append-only duel scoreboard log. The CLI's
    /// `orbit duel scoreboard` command aggregates the returned runs in
    /// memory via `orbit_store::duel_scoreboard::aggregate`. Returns an
    /// empty vector when the file does not yet exist — an unrun
    /// scoreboard is not an error condition.
    pub fn load_duel_runs(&self) -> Result<Vec<orbit_common::types::DuelRun>, OrbitError> {
        orbit_store::duel_scoreboard::load_runs(&self.paths().scoreboard_dir)
    }
}
