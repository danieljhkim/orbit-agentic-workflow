use chrono::{DateTime, Utc};

use orbit_types::{OrbitError, OrbitEvent};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn run_due_jobs(&self, now: DateTime<Utc>) -> Result<usize, OrbitError> {
        let lock_name = self.context.lock_store.global_job_lock_name();
        if !self.context.lock_store.try_lock(lock_name)? {
            return Ok(0);
        }

        let claim_result = (|| {
            let due_jobs = self.context.job_store.due_jobs(now)?;
            for job in &due_jobs {
                let _ = self.recover_stale_active_run_for_job(job, now)?;
            }

            self.context.job_store.claim_due_jobs(now)
        })();

        let _ = self.context.lock_store.unlock(lock_name);
        let claim = claim_result?;

        for skipped_activity_id in &claim.skipped {
            self.record_event(OrbitEvent::JobSkipped {
                job_id: skipped_activity_id.clone(),
                reason: "pending/running job run already exists".to_string(),
            })?;
        }

        let mut ran = 0usize;
        for claimed in claim.claimed {
            self.execute_claimed_job(&claimed)?;
            ran += 1;
        }
        Ok(ran)
    }
}
