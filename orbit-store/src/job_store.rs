use chrono::{DateTime, Utc};
use orbit_types::{
    Job, JobRetryBackoffStrategy, JobRun, JobRunState, JobScheduleState, JobTargetType, JobTrigger,
    OrbitError, Role,
};
use rusqlite::{OptionalExtension, params};
use serde_json::Value;

use crate::{Store, StoreTx, new_id, now_string, parse_timestamp};

#[derive(Debug, Clone)]
pub struct ClaimedJobRun {
    pub job: Job,
    pub run: JobRun,
}

#[derive(Debug, Clone, Default)]
pub struct DueJobsClaim {
    pub claimed: Vec<ClaimedJobRun>,
    pub skipped: Vec<String>,
}

const JOB_COLS: &str = "id, target_type, target_id, schedule, agent_cli, timeout_seconds, retry_max_attempts, retry_backoff_strategy, retry_initial_delay_seconds, state, next_run_at, created_at, updated_at";
const JOB_RUN_COLS: &str = "id, job_id, attempt, state, scheduled_at, started_at, finished_at, duration_ms, exit_code, agent_response_json, error_code, error_message, created_at";

impl Store {
    pub fn list_jobs(&self, include_disabled: bool) -> Result<Vec<Job>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let sql = if include_disabled {
            format!("SELECT {JOB_COLS} FROM jobs ORDER BY created_at DESC")
        } else {
            format!(
                "SELECT {JOB_COLS} FROM jobs WHERE state != 'disabled' ORDER BY created_at DESC"
            )
        };

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let rows = stmt
            .query_map([], row_to_job)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_job(&self, job_id: &str) -> Result<Option<Job>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        conn.query_row(
            &format!("SELECT {JOB_COLS} FROM jobs WHERE id = ?1"),
            [job_id],
            row_to_job,
        )
        .optional()
        .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn due_jobs(&self, now: DateTime<Utc>) -> Result<Vec<Job>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {JOB_COLS}
                 FROM jobs
                 WHERE state = 'enabled'
                   AND next_run_at <= ?1
                 ORDER BY next_run_at ASC"
            ))
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        let rows = stmt
            .query_map([now.to_rfc3339()], row_to_job)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn list_job_runs(&self, job_id: &str) -> Result<Vec<JobRun>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        let mut stmt = conn
            .prepare(&format!(
                "SELECT {JOB_RUN_COLS}
                 FROM job_runs
                 WHERE job_id = ?1
                 ORDER BY created_at DESC"
            ))
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        let rows = stmt
            .query_map([job_id], row_to_job_run)
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_job_run(&self, run_id: &str) -> Result<Option<JobRun>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        conn.query_row(
            &format!("SELECT {JOB_RUN_COLS} FROM job_runs WHERE id = ?1"),
            [run_id],
            row_to_job_run,
        )
        .optional()
        .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_running_job_run(&self, job_id: &str) -> Result<Option<JobRun>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        conn.query_row(
            &format!(
                "SELECT {JOB_RUN_COLS}
                 FROM job_runs
                 WHERE job_id = ?1 AND state = 'running'
                 ORDER BY created_at DESC
                 LIMIT 1"
            ),
            [job_id],
            row_to_job_run,
        )
        .optional()
        .map_err(|e| OrbitError::Store(e.to_string()))
    }

    pub fn get_pending_or_running_job_run(
        &self,
        job_id: &str,
    ) -> Result<Option<JobRun>, OrbitError> {
        let conn = self
            .conn
            .lock()
            .map_err(|e| OrbitError::Store(format!("mutex poisoned: {e}")))?;

        conn.query_row(
            &format!(
                "SELECT {JOB_RUN_COLS}
                 FROM job_runs
                 WHERE job_id = ?1 AND (state = 'pending' OR state = 'running')
                 ORDER BY created_at DESC
                 LIMIT 1"
            ),
            [job_id],
            row_to_job_run,
        )
        .optional()
        .map_err(|e| OrbitError::Store(e.to_string()))
    }

    // Backward compatibility aliases.
    pub fn list_job_sessions(&self, job_id: &str) -> Result<Vec<JobRun>, OrbitError> {
        self.list_job_runs(job_id)
    }

    pub fn get_job_session(&self, run_id: &str) -> Result<Option<JobRun>, OrbitError> {
        self.get_job_run(run_id)
    }

    pub fn get_running_job_session(&self, job_id: &str) -> Result<Option<JobRun>, OrbitError> {
        self.get_running_job_run(job_id)
    }

    pub fn is_job_session_cancel_requested(&self, _run_id: &str) -> Result<bool, OrbitError> {
        Ok(false)
    }
}

impl<'a> StoreTx<'a> {
    #[allow(clippy::too_many_arguments)]
    pub fn insert_job_v2(
        &mut self,
        target_type: JobTargetType,
        target_id: &str,
        schedule: &str,
        agent_cli: &str,
        timeout_seconds: u64,
        retry_max_attempts: u32,
        retry_backoff_strategy: JobRetryBackoffStrategy,
        retry_initial_delay_seconds: u64,
        next_run_at: DateTime<Utc>,
    ) -> Result<Job, OrbitError> {
        let now = Utc::now();
        let job = Job {
            job_id: new_id("job"),
            target_type,
            target_id: target_id.to_string(),
            schedule: schedule.to_string(),
            agent_cli: agent_cli.to_string(),
            timeout_seconds,
            retry_max_attempts,
            retry_backoff_strategy,
            retry_initial_delay_seconds,
            state: JobScheduleState::Enabled,
            next_run_at,
            created_at: now,
            updated_at: now,
        };

        self.tx
            .execute(
                "INSERT INTO jobs(
                    id, target_type, target_id, schedule, agent_cli,
                    timeout_seconds, retry_max_attempts, retry_backoff_strategy,
                    retry_initial_delay_seconds, state, next_run_at, created_at, updated_at
                 ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    job.job_id,
                    job.target_type.to_string(),
                    job.target_id,
                    job.schedule,
                    job.agent_cli,
                    job.timeout_seconds as i64,
                    job.retry_max_attempts as i64,
                    job.retry_backoff_strategy.to_string(),
                    job.retry_initial_delay_seconds as i64,
                    job.state.to_string(),
                    job.next_run_at.to_rfc3339(),
                    job.created_at.to_rfc3339(),
                    job.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(job)
    }

    pub fn set_job_state(
        &mut self,
        job_id: &str,
        state: JobScheduleState,
    ) -> Result<bool, OrbitError> {
        let changed = self
            .tx
            .execute(
                "UPDATE jobs
                 SET state = ?1, updated_at = ?2
                 WHERE id = ?3",
                params![state.to_string(), now_string(), job_id],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(changed == 1)
    }

    pub fn mark_job_disabled(&mut self, job_id: &str) -> Result<bool, OrbitError> {
        self.set_job_state(job_id, JobScheduleState::Disabled)
    }

    pub fn update_job_next_run(
        &mut self,
        job_id: &str,
        next_run_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError> {
        let changed = self
            .tx
            .execute(
                "UPDATE jobs
                 SET next_run_at = ?1, updated_at = ?2
                 WHERE id = ?3",
                params![next_run_at.to_rfc3339(), now_string(), job_id],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(changed == 1)
    }

    pub fn next_job_run_attempt(
        &mut self,
        job_id: &str,
        scheduled_at: DateTime<Utc>,
    ) -> Result<u32, OrbitError> {
        let max_attempt: Option<i64> = self
            .tx
            .query_row(
                "SELECT MAX(attempt) FROM job_runs WHERE job_id = ?1 AND scheduled_at = ?2",
                params![job_id, scheduled_at.to_rfc3339()],
                |row| row.get::<_, Option<i64>>(0),
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok((max_attempt.unwrap_or(0) as u32) + 1)
    }

    pub fn insert_job_run(
        &mut self,
        job_id: &str,
        attempt: u32,
        scheduled_at: DateTime<Utc>,
    ) -> Result<JobRun, OrbitError> {
        let now = Utc::now();
        let run = JobRun {
            run_id: new_id("jrun"),
            job_id: job_id.to_string(),
            attempt,
            state: JobRunState::Pending,
            scheduled_at,
            started_at: None,
            finished_at: None,
            duration_ms: None,
            exit_code: None,
            agent_response_json: None,
            error_code: None,
            error_message: None,
            created_at: now,
        };

        self.tx
            .execute(
                "INSERT INTO job_runs(
                    id, job_id, attempt, state, scheduled_at, started_at,
                    finished_at, duration_ms, exit_code, agent_response_json,
                    error_code, error_message, created_at
                ) VALUES (?1, ?2, ?3, ?4, ?5, NULL, NULL, NULL, NULL, NULL, NULL, NULL, ?6)",
                params![
                    run.run_id,
                    run.job_id,
                    run.attempt as i64,
                    run.state.to_string(),
                    run.scheduled_at.to_rfc3339(),
                    run.created_at.to_rfc3339(),
                ],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;

        Ok(run)
    }

    pub fn mark_job_run_running(
        &mut self,
        run_id: &str,
        started_at: DateTime<Utc>,
    ) -> Result<bool, OrbitError> {
        let changed = self
            .tx
            .execute(
                "UPDATE job_runs
                 SET state = 'running', started_at = ?1
                 WHERE id = ?2",
                params![started_at.to_rfc3339(), run_id],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(changed == 1)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn complete_job_run(
        &mut self,
        run_id: &str,
        state: JobRunState,
        finished_at: DateTime<Utc>,
        duration_ms: Option<u64>,
        exit_code: Option<i32>,
        agent_response_json: Option<&Value>,
        error_code: Option<&str>,
        error_message: Option<&str>,
    ) -> Result<bool, OrbitError> {
        let response_raw = agent_response_json
            .map(serde_json::to_string)
            .transpose()
            .map_err(|e| OrbitError::Store(format!("serialize agent response: {e}")))?;

        let changed = self
            .tx
            .execute(
                "UPDATE job_runs
                 SET state = ?1,
                     finished_at = ?2,
                     duration_ms = ?3,
                     exit_code = ?4,
                     agent_response_json = ?5,
                     error_code = ?6,
                     error_message = ?7
                 WHERE id = ?8",
                params![
                    state.to_string(),
                    finished_at.to_rfc3339(),
                    duration_ms.map(|v| v as i64),
                    exit_code,
                    response_raw,
                    error_code,
                    error_message,
                    run_id,
                ],
            )
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(changed == 1)
    }

    pub fn claim_due_jobs(&mut self, now: DateTime<Utc>) -> Result<DueJobsClaim, OrbitError> {
        let due_jobs = {
            let mut stmt = self
                .tx
                .prepare(&format!(
                    "SELECT {JOB_COLS}
                     FROM jobs
                     WHERE state = 'enabled'
                       AND next_run_at <= ?1
                     ORDER BY next_run_at ASC"
                ))
                .map_err(|e| OrbitError::Store(e.to_string()))?;

            let rows = stmt
                .query_map([now.to_rfc3339()], row_to_job)
                .map_err(|e| OrbitError::Store(e.to_string()))?;
            rows.collect::<Result<Vec<_>, _>>()
                .map_err(|e| OrbitError::Store(e.to_string()))?
        };

        let mut result = DueJobsClaim::default();
        for job in due_jobs {
            let running_exists: Option<String> = self
                .tx
                .query_row(
                    "SELECT id FROM job_runs WHERE job_id = ?1 AND (state = 'pending' OR state = 'running') LIMIT 1",
                    [job.job_id.clone()],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|e| OrbitError::Store(e.to_string()))?;

            if running_exists.is_some() {
                result.skipped.push(job.job_id.clone());
                continue;
            }

            let run = self.insert_job_run(&job.job_id, 1, now)?;
            result.claimed.push(ClaimedJobRun { job, run });
        }

        Ok(result)
    }

    // Backward compatibility wrappers.
    pub fn mark_job_deleted(&mut self, job_id: &str) -> Result<bool, OrbitError> {
        self.mark_job_disabled(job_id)
    }

    // Backward compatibility wrappers used by v2.1 runtime paths.
    pub fn insert_job(
        &mut self,
        _name: &str,
        task_id: &str,
        schedule_spec: &str,
        _timezone: &str,
        next_run_at: Option<DateTime<Utc>>,
    ) -> Result<Job, OrbitError> {
        self.insert_job_v2(
            JobTargetType::ExecutionSpec,
            task_id,
            schedule_spec,
            "claude",
            300,
            0,
            JobRetryBackoffStrategy::None,
            0,
            next_run_at.unwrap_or_else(Utc::now),
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub fn insert_job_session(
        &mut self,
        job_id: &str,
        _task_id: &str,
        _trigger: JobTrigger,
        _created_by_role: Role,
        trigger_time: DateTime<Utc>,
        _composed_context_hash: Option<&str>,
        _effective_allowlist_hash: Option<&str>,
    ) -> Result<JobRun, OrbitError> {
        let attempt = self.next_job_run_attempt(job_id, trigger_time)?;
        let run = self.insert_job_run(job_id, attempt, trigger_time)?;
        let _ = self.mark_job_run_running(&run.run_id, Utc::now())?;
        self.get_job_run_from_tx(&run.run_id)?
            .ok_or_else(|| OrbitError::JobRunNotFound(run.run_id.clone()))
    }

    pub fn finish_job_session(
        &mut self,
        run_id: &str,
        state: JobRunState,
        exit_code: Option<i32>,
        error: Option<&str>,
    ) -> Result<bool, OrbitError> {
        let mapped_state = match state {
            JobRunState::Cancelled => JobRunState::Failed,
            JobRunState::Succeeded => JobRunState::Success,
            other => other,
        };
        self.complete_job_run(
            run_id,
            mapped_state,
            Utc::now(),
            None,
            exit_code,
            None,
            None,
            error,
        )
    }

    pub fn request_cancel_running_session(
        &mut self,
        job_id: &str,
    ) -> Result<Option<String>, OrbitError> {
        let run_id: Option<String> = self
            .tx
            .query_row(
                "SELECT id FROM job_runs WHERE job_id = ?1 AND state = 'running' ORDER BY created_at DESC LIMIT 1",
                [job_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| OrbitError::Store(e.to_string()))?;
        Ok(run_id)
    }

    fn get_job_run_from_tx(&self, run_id: &str) -> Result<Option<JobRun>, OrbitError> {
        self.tx
            .query_row(
                &format!("SELECT {JOB_RUN_COLS} FROM job_runs WHERE id = ?1"),
                [run_id],
                row_to_job_run,
            )
            .optional()
            .map_err(|e| OrbitError::Store(e.to_string()))
    }
}

fn row_to_job(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    let target_type_raw: String = row.get(1)?;
    let state_raw: String = row.get(9)?;
    let next_run_at_raw: String = row.get(10)?;
    let created_at_raw: String = row.get(11)?;
    let updated_at_raw: String = row.get(12)?;
    let timeout_seconds: i64 = row.get(5)?;
    let retry_max_attempts: i64 = row.get(6)?;
    let retry_initial_delay_seconds: i64 = row.get(8)?;
    let backoff_raw: String = row.get(7)?;

    Ok(Job {
        job_id: row.get(0)?,
        target_type: parse_target_type(&target_type_raw)?,
        target_id: row.get(2)?,
        schedule: row.get(3)?,
        agent_cli: row.get(4)?,
        timeout_seconds: timeout_seconds as u64,
        retry_max_attempts: retry_max_attempts as u32,
        retry_backoff_strategy: parse_backoff_strategy(&backoff_raw)?,
        retry_initial_delay_seconds: retry_initial_delay_seconds as u64,
        state: parse_job_state(&state_raw)?,
        next_run_at: parse_timestamp(&next_run_at_raw)?,
        created_at: parse_timestamp(&created_at_raw)?,
        updated_at: parse_timestamp(&updated_at_raw)?,
    })
}

fn row_to_job_run(row: &rusqlite::Row<'_>) -> rusqlite::Result<JobRun> {
    let state_raw: String = row.get(3)?;
    let scheduled_at_raw: String = row.get(4)?;
    let started_at_raw: Option<String> = row.get(5)?;
    let finished_at_raw: Option<String> = row.get(6)?;
    let agent_response_raw: Option<String> = row.get(9)?;
    let created_at_raw: String = row.get(12)?;

    let response_json = agent_response_raw
        .map(|raw| {
            serde_json::from_str::<Value>(&raw).map_err(|e| {
                rusqlite::Error::FromSqlConversionFailure(
                    raw.len(),
                    rusqlite::types::Type::Text,
                    Box::new(e),
                )
            })
        })
        .transpose()?;

    let attempt: i64 = row.get(2)?;
    let duration_ms: Option<i64> = row.get(7)?;

    Ok(JobRun {
        run_id: row.get(0)?,
        job_id: row.get(1)?,
        attempt: attempt as u32,
        state: parse_job_run_state(&state_raw)?,
        scheduled_at: parse_timestamp(&scheduled_at_raw)?,
        started_at: parse_optional_timestamp(started_at_raw)?,
        finished_at: parse_optional_timestamp(finished_at_raw)?,
        duration_ms: duration_ms.map(|v| v as u64),
        exit_code: row.get(8)?,
        agent_response_json: response_json,
        error_code: row.get(10)?,
        error_message: row.get(11)?,
        created_at: parse_timestamp(&created_at_raw)?,
    })
}

fn parse_optional_timestamp(raw: Option<String>) -> rusqlite::Result<Option<DateTime<Utc>>> {
    raw.map(|value| parse_timestamp(&value)).transpose()
}

fn parse_target_type(raw: &str) -> rusqlite::Result<JobTargetType> {
    raw.parse::<JobTargetType>()
        .map_err(|e| parse_enum_error(raw, e))
}

fn parse_job_state(raw: &str) -> rusqlite::Result<JobScheduleState> {
    raw.parse::<JobScheduleState>()
        .map_err(|e| parse_enum_error(raw, e))
}

fn parse_backoff_strategy(raw: &str) -> rusqlite::Result<JobRetryBackoffStrategy> {
    raw.parse::<JobRetryBackoffStrategy>()
        .map_err(|e| parse_enum_error(raw, e))
}

fn parse_job_run_state(raw: &str) -> rusqlite::Result<JobRunState> {
    raw.parse::<JobRunState>()
        .map_err(|e| parse_enum_error(raw, e))
}

fn parse_enum_error(raw: &str, message: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        raw.len(),
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            message,
        )),
    )
}
