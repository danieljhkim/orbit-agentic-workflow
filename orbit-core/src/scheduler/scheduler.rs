use chrono::{DateTime, Utc};

use orbit_types::{OrbitError, OrbitEvent};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn run_due_schedulers(&self, now: DateTime<Utc>) -> Result<usize, OrbitError> {
        let lock_name = self.context.lock_store.global_job_lock_name();
        if !self.context.lock_store.try_lock(lock_name)? {
            return Ok(0);
        }

        let claim_result = (|| {
            let due_schedulers = self.context.scheduler_store.due_schedulers(now)?;
            for scheduler in &due_schedulers {
                let _ = self.recover_stale_active_run_for_job(scheduler, now)?;
            }

            self.context.scheduler_store.claim_due_schedulers(now)
        })();

        let _ = self.context.lock_store.unlock(lock_name);
        let claim = claim_result?;

        for skipped_job_id in &claim.skipped {
            self.record_event(OrbitEvent::SchedulerSkipped {
                scheduler_id: skipped_job_id.clone(),
                reason: "pending/running scheduler run already exists".to_string(),
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
