use chrono::Utc;
use orbit_store::Store;
use orbit_types::{
    JobRetryBackoffStrategy, JobRunState, JobScheduleState, JobTargetType, JobTrigger, Role,
};

#[test]
fn job_state_transitions_and_disabled_visibility() {
    let store = Store::open_in_memory().expect("store");
    let now = Utc::now();

    let job = store
        .with_transaction(|tx| {
            tx.insert_job_v2(
                JobTargetType::ExecutionSpec,
                "spec-demo",
                "every 1h",
                "mock-agent",
                300,
                0,
                JobRetryBackoffStrategy::None,
                0,
                now,
            )
        })
        .expect("insert job");

    let due = store.due_jobs(now).expect("due jobs");
    assert_eq!(due.len(), 1);
    assert_eq!(due[0].job_id, job.job_id);

    store
        .with_transaction(|tx| tx.set_job_state(&job.job_id, JobScheduleState::Paused))
        .expect("pause job");
    let paused = store
        .get_job(&job.job_id)
        .expect("get paused")
        .expect("job");
    assert_eq!(paused.state, JobScheduleState::Paused);

    store
        .with_transaction(|tx| tx.set_job_state(&job.job_id, JobScheduleState::Enabled))
        .expect("resume job");
    store
        .with_transaction(|tx| tx.mark_job_disabled(&job.job_id))
        .expect("disable job");

    let default_list = store.list_jobs(false).expect("list enabled/paused");
    assert!(default_list.iter().all(|item| item.job_id != job.job_id));

    let all_list = store.list_jobs(true).expect("list all");
    let disabled = all_list
        .iter()
        .find(|item| item.job_id == job.job_id)
        .expect("disabled present");
    assert_eq!(disabled.state, JobScheduleState::Disabled);
}

#[test]
fn claim_due_jobs_skips_when_pending_or_running_run_exists() {
    let store = Store::open_in_memory().expect("store");
    let now = Utc::now();

    let job = store
        .with_transaction(|tx| {
            tx.insert_job_v2(
                JobTargetType::ExecutionSpec,
                "spec-claim",
                "every 1m",
                "mock-agent",
                300,
                0,
                JobRetryBackoffStrategy::None,
                0,
                now,
            )
        })
        .expect("insert job");

    let first = store
        .with_transaction(|tx| tx.claim_due_jobs(now))
        .expect("first claim");
    assert_eq!(first.claimed.len(), 1);
    assert!(first.skipped.is_empty());
    assert_eq!(first.claimed[0].job.job_id, job.job_id);
    assert_eq!(first.claimed[0].run.state, JobRunState::Pending);

    let second = store
        .with_transaction(|tx| tx.claim_due_jobs(now))
        .expect("second claim");
    assert!(second.claimed.is_empty());
    assert_eq!(second.skipped, vec![job.job_id.clone()]);
}

#[test]
fn legacy_session_wrappers_map_to_v2_job_runs() {
    let store = Store::open_in_memory().expect("store");
    let now = Utc::now();

    let job = store
        .with_transaction(|tx| {
            tx.insert_job_v2(
                JobTargetType::ExecutionSpec,
                "spec-legacy",
                "every 1m",
                "mock-agent",
                300,
                0,
                JobRetryBackoffStrategy::None,
                0,
                now,
            )
        })
        .expect("insert job");

    let run = store
        .with_transaction(|tx| {
            tx.insert_job_session(
                &job.job_id,
                "task-unused",
                JobTrigger::Manual,
                Role::Admin,
                now,
                None,
                None,
            )
        })
        .expect("insert session");
    assert_eq!(run.state, JobRunState::Running);

    store
        .with_transaction(|tx| {
            tx.finish_job_session(
                &run.run_id,
                JobRunState::Cancelled,
                Some(130),
                Some("cancel requested"),
            )
        })
        .expect("finish session");

    let finished = store
        .get_job_run(&run.run_id)
        .expect("get run")
        .expect("run");
    assert_eq!(finished.state, JobRunState::Failed);
    assert_eq!(finished.exit_code, Some(130));
    assert_eq!(finished.error_message.as_deref(), Some("cancel requested"));
}
