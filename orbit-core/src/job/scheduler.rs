use chrono::{DateTime, Utc};

use orbit_types::{OrbitError, OrbitEvent};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn run_due_jobs(&self, now: DateTime<Utc>) -> Result<usize, OrbitError> {
        let lock_name = orbit_store::Store::global_job_lock_name();
        if !self.context.store.try_lock(lock_name)? {
            return Ok(0);
        }

        let result = (|| {
            let claim = self
                .context
                .store
                .with_transaction(|tx| tx.claim_due_jobs(now))?;

            for skipped_job_id in &claim.skipped {
                self.with_mutation(|_| {
                    Ok((
                        (),
                        OrbitEvent::JobSkipped {
                            job_id: skipped_job_id.clone(),
                            reason: "pending/running job run already exists".to_string(),
                        },
                    ))
                })?;
                let _ = self.append_job_system_entry(
                    skipped_job_id,
                    "scheduler skipped run: pending/running job run already exists".to_string(),
                );
            }

            let mut ran = 0usize;
            for claimed in claim.claimed {
                self.execute_claimed_job(&claimed)?;
                ran += 1;
            }
            Ok(ran)
        })();

        let _ = self.context.store.unlock(lock_name);
        result
    }
}
