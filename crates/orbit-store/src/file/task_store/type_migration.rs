use std::fs;
use std::path::{Path, PathBuf};

use orbit_common::types::OrbitError;
use orbit_common::utility::fs::atomic_write_text_volatile as write_atomic;
use serde::Deserialize;

use super::constants::TASK_DOC_FILE_NAME;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskTypeMigrationChange {
    pub path: PathBuf,
    pub task_id: Option<String>,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TaskTypeMigrationSummary {
    pub changed: usize,
    pub unchanged: usize,
    pub friction_skipped: usize,
    pub changes: Vec<TaskTypeMigrationChange>,
}

#[derive(Debug, Deserialize)]
struct RawTaskTypeDoc {
    id: Option<String>,
    #[serde(rename = "type")]
    task_type: Option<String>,
}

pub fn migrate_task_types(
    tasks_root: &Path,
    dry_run: bool,
) -> Result<TaskTypeMigrationSummary, OrbitError> {
    let mut summary = TaskTypeMigrationSummary::default();
    if !tasks_root.exists() {
        return Ok(summary);
    }

    let mut task_docs = Vec::new();
    collect_task_docs(tasks_root, &mut task_docs)?;
    task_docs.sort();

    for path in task_docs {
        let raw = fs::read_to_string(&path).map_err(|err| {
            OrbitError::Io(format!(
                "failed to read task file {}: {err}",
                path.display()
            ))
        })?;
        let doc: RawTaskTypeDoc = serde_yaml::from_str(&raw).map_err(|err| {
            OrbitError::Store(format!("invalid task file {}: {err}", path.display()))
        })?;
        let Some(task_type) = doc.task_type.as_deref() else {
            summary.unchanged += 1;
            continue;
        };
        let Some(target_type) = migration_target(task_type) else {
            if task_type == "friction" {
                summary.friction_skipped += 1;
            } else {
                summary.unchanged += 1;
            }
            continue;
        };

        let change = TaskTypeMigrationChange {
            path: path.clone(),
            task_id: doc.id.clone(),
            from: task_type.to_string(),
            to: target_type.to_string(),
        };
        if !dry_run {
            let updated = rewrite_top_level_type(&raw, target_type).ok_or_else(|| {
                OrbitError::Store(format!(
                    "could not rewrite top-level type in {}",
                    path.display()
                ))
            })?;
            write_atomic(&path, &updated).map_err(|err| {
                OrbitError::Io(format!(
                    "failed to write task file {}: {err}",
                    path.display()
                ))
            })?;
        }
        summary.changed += 1;
        summary.changes.push(change);
    }

    Ok(summary)
}

fn migration_target(task_type: &str) -> Option<&'static str> {
    match task_type {
        "epic" => Some("feature"),
        "task" => Some("chore"),
        "issue" => Some("bug"),
        _ => None,
    }
}

fn collect_task_docs(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), OrbitError> {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(err) => {
            return Err(OrbitError::Io(format!(
                "failed to read task directory {}: {err}",
                root.display()
            )));
        }
    };

    for entry in entries {
        let entry = entry.map_err(|err| OrbitError::Io(err.to_string()))?;
        let path = entry.path();
        let file_type = entry
            .file_type()
            .map_err(|err| OrbitError::Io(err.to_string()))?;
        if file_type.is_dir() {
            collect_task_docs(&path, out)?;
        } else if file_type.is_file()
            && path.file_name().and_then(|name| name.to_str()) == Some(TASK_DOC_FILE_NAME)
        {
            out.push(path);
        }
    }
    Ok(())
}

fn rewrite_top_level_type(raw: &str, target_type: &str) -> Option<String> {
    let mut rewritten = String::with_capacity(raw.len());
    let mut changed = false;
    for line in raw.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        if !changed && line_without_newline.starts_with("type:") {
            rewritten.push_str("type: ");
            rewritten.push_str(target_type);
            if line.ends_with('\n') {
                rewritten.push('\n');
            }
            changed = true;
        } else {
            rewritten.push_str(line);
        }
    }
    changed.then_some(rewritten)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_task(root: &Path, state: &str, id: &str, task_type: &str) -> PathBuf {
        let dir = root.join(state).join(id);
        fs::create_dir_all(&dir).expect("create task dir");
        let path = dir.join(TASK_DOC_FILE_NAME);
        fs::write(
            &path,
            format!(
                "schema_version: 2\nid: {id}\ntype: {task_type}\npriority: medium\ntitle: {id}\n"
            ),
        )
        .expect("write task yaml");
        path
    }

    #[test]
    fn maps_dropped_task_types_and_reports_changes() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let task = write_task(root, "proposed", "T20260510-1", "task");
        let epic = write_task(root, "backlog", "T20260510-2", "epic");
        let issue = write_task(root, "review", "T20260510-3", "issue");

        let summary = migrate_task_types(root, false).expect("migrate task types");

        assert_eq!(summary.changed, 3);
        assert!(
            fs::read_to_string(task)
                .expect("task")
                .contains("type: chore")
        );
        assert!(
            fs::read_to_string(epic)
                .expect("epic")
                .contains("type: feature")
        );
        assert!(
            fs::read_to_string(issue)
                .expect("issue")
                .contains("type: bug")
        );
        assert_eq!(
            summary
                .changes
                .iter()
                .map(|change| (change.from.as_str(), change.to.as_str()))
                .collect::<Vec<_>>(),
            vec![("epic", "feature"), ("task", "chore"), ("issue", "bug")]
        );
    }

    #[test]
    fn dry_run_does_not_write_and_second_write_is_idempotent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let task = write_task(root, "proposed", "T20260510-1", "task");

        let dry_run = migrate_task_types(root, true).expect("dry run");
        assert_eq!(dry_run.changed, 1);
        assert!(
            fs::read_to_string(&task)
                .expect("task")
                .contains("type: task")
        );

        let first = migrate_task_types(root, false).expect("write");
        let second = migrate_task_types(root, false).expect("write again");
        assert_eq!(first.changed, 1);
        assert_eq!(second.changed, 0);
        assert!(
            fs::read_to_string(task)
                .expect("task")
                .contains("type: chore")
        );
    }

    #[test]
    fn leaves_friction_records_untouched() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let friction = write_task(root, "friction", "T20260510-1", "friction");
        let before = fs::read_to_string(&friction).expect("before");

        let summary = migrate_task_types(root, false).expect("migrate");

        assert_eq!(summary.changed, 0);
        assert_eq!(summary.friction_skipped, 1);
        assert_eq!(fs::read_to_string(friction).expect("after"), before);
    }
}
