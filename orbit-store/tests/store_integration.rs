use chrono::Utc;
use orbit_store::Store;
use orbit_store::task_store::TaskInsertParams;
use orbit_types::{SchedulerRetryBackoffStrategy, SchedulerTargetType};

#[test]
fn due_schedulers_query_returns_scheduled_entries() {
    let store = Store::open_in_memory().expect("store");
    let next_run = Utc::now();

    store
        .with_transaction(|tx| {
            let task = tx.insert_task(&TaskInsertParams {
                title: "scheduler task".to_string(),
                ..Default::default()
            })?;
            let _job = tx.insert_job_v2(
                SchedulerTargetType::Job,
                &task.id,
                "every 1h",
                "mock-agent",
                300,
                0,
                SchedulerRetryBackoffStrategy::None,
                0,
                next_run,
            )?;
            Ok(())
        })
        .expect("insert scheduler");

    let due = store.due_schedulers(next_run).expect("due schedulers");
    assert_eq!(due.len(), 1);
}
