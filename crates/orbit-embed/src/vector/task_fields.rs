//! Maps a `Task` to the per-field rows that get embedded individually.
//!
//! Each task is indexed as multiple rows whose field names match the logical
//! v2 task documents where possible (title, description, acceptance, plan,
//! execution_summary, comment_<idx>, review_<thread>_msg_<idx>) so the
//! best-matching field can surface as the snippet at search time.

use orbit_common::types::Task;

use super::EmbeddingField;

pub fn task_embedding_fields(task: &Task) -> Vec<EmbeddingField> {
    let mut fields = Vec::new();
    push_field(&mut fields, "title", &task.title);
    push_field(&mut fields, "description", &task.description);
    push_field(&mut fields, "plan", &task.plan);
    push_field(&mut fields, "execution_summary", &task.execution_summary);
    if !task.acceptance_criteria.is_empty() {
        push_field(
            &mut fields,
            "acceptance",
            &task.acceptance_criteria.join("\n"),
        );
    }
    for (idx, comment) in task.comments.iter().enumerate() {
        push_field(&mut fields, format!("comment_{idx}"), &comment.message);
    }
    for thread in &task.review_threads {
        for (idx, message) in thread.messages.iter().enumerate() {
            push_field(
                &mut fields,
                format!("review_{}_msg_{idx}", thread.thread_id),
                &message.body,
            );
        }
    }
    fields
}

fn push_field(fields: &mut Vec<EmbeddingField>, field: impl Into<String>, text: &str) {
    if !text.trim().is_empty() {
        fields.push(EmbeddingField::new(field, text.trim().to_string()));
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use orbit_common::types::{
        ReviewMessage, ReviewThread, ReviewThreadStatus, Task, TaskComment, TaskPriority,
        TaskStatus, TaskType,
    };

    use super::*;

    fn task() -> Task {
        Task {
            id: "ORB-00000".to_string(),
            parent_id: None,
            title: "Index this".to_string(),
            description: "Task description".to_string(),
            acceptance_criteria: vec!["First criterion".to_string()],
            dependencies: Vec::new(),
            plan: "Plan body".to_string(),
            execution_summary: "Summary body".to_string(),
            context_files: Vec::new(),
            workspace_path: None,
            repo_root: None,
            created_by: None,
            planned_by: None,
            implemented_by: None,
            agent: None,
            model: None,
            status: TaskStatus::Backlog,
            priority: TaskPriority::Medium,
            complexity: None,
            task_type: TaskType::Chore,
            pr_status: None,
            external_refs: Vec::new(),
            source_task_id: None,
            batch_id: None,
            tags: Vec::new(),
            comments: vec![TaskComment {
                at: Utc::now(),
                by: "codex:gpt-5.5".to_string(),
                message: "Comment body".to_string(),
            }],
            history: Vec::new(),
            review_threads: vec![ReviewThread {
                thread_id: "rt-1".to_string(),
                path: Some("src/lib.rs".to_string()),
                line: Some(12),
                status: ReviewThreadStatus::Open,
                messages: vec![ReviewMessage {
                    message_id: "rm-1".to_string(),
                    at: Utc::now(),
                    by: "codex:gpt-5.5".to_string(),
                    body: "Review body".to_string(),
                    github_comment_id: None,
                }],
                github_thread_id: None,
            }],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn task_embedding_fields_use_v2_document_names() {
        let field_names = task_embedding_fields(&task())
            .into_iter()
            .map(|field| field.field)
            .collect::<Vec<_>>();

        assert_eq!(
            field_names,
            vec![
                "title",
                "description",
                "plan",
                "execution_summary",
                "acceptance",
                "comment_0",
                "review_rt-1_msg_0"
            ]
        );
    }
}
