use chrono::{DateTime, Duration, Utc};

use orbit_types::{JobStatus, OrbitError, OrbitEvent};

use crate::OrbitRuntime;

impl OrbitRuntime {
    pub fn run_due_jobs(&self, now: DateTime<Utc>) -> Result<usize, OrbitError> {
        let lock_name = orbit_store::Store::global_job_lock_name();
        if !self.context.store.try_lock(lock_name)? {
            return Ok(0);
        }

        let result = (|| {
            let mut ran = 0usize;
            for job in self.context.store.due_jobs(now)? {
                let started = self.with_mutation(
                    OrbitEvent::JobStarted { id: job.id.clone() },
                    "job started",
                    |tx| {
                        tx.transition_job_status(&job.id, JobStatus::Scheduled, JobStatus::Running)
                    },
                )?;

                if !started {
                    continue;
                }

                let next_run_at = now + Duration::minutes(1);
                let completed = self.with_mutation(
                    OrbitEvent::JobCompleted {
                        id: job.id.clone(),
                        success: true,
                    },
                    "job completed",
                    |tx| tx.complete_job(&job.id, next_run_at, true),
                )?;

                if completed {
                    ran += 1;
                }
            }
            Ok(ran)
        })();

        let _ = self.context.store.unlock(lock_name);
        result
    }
}
