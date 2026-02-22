use orbit_types::JobStatus;

pub fn next_after_run(success: bool) -> JobStatus {
    if success {
        JobStatus::Complete
    } else {
        JobStatus::Failed
    }
}
