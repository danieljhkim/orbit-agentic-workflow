use orbit_core::OrbitRuntime;
use orbit_core::command::skill::{SkillAddParams, SkillDoctorStatus, SkillUpdateParams};
use orbit_core::command::task::TaskAddParams;
use orbit_types::Role;
use tempfile::tempdir;

#[test]
fn skill_crud_round_trip() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");

    let created = runtime
        .add_skill(SkillAddParams {
            name: "rust-refactor".to_string(),
            description: Some("desc".to_string()),
            instructions: "use crate boundaries".to_string(),
            context_files: vec![],
            allowed_tools: vec!["fs.read".to_string()],
            role: Role::Agent,
        })
        .expect("add skill");
    assert_eq!(created.name, "rust-refactor");

    let listed = runtime.list_skills().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].name, "rust-refactor");

    let shown = runtime.show_skill("rust-refactor").expect("show");
    assert_eq!(shown.instructions, "use crate boundaries");

    let updated = runtime
        .update_skill(
            "rust-refactor",
            SkillUpdateParams {
                instructions: Some("updated".to_string()),
                ..Default::default()
            },
        )
        .expect("update");
    assert_eq!(updated.instructions, "updated");

    runtime.delete_skill("rust-refactor").expect("delete");
    assert!(runtime.show_skill("rust-refactor").is_err());
}

#[test]
fn add_skill_rejects_unknown_allowed_tool() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");

    let result = runtime.add_skill(SkillAddParams {
        name: "bad".to_string(),
        description: None,
        instructions: "do".to_string(),
        context_files: vec![],
        allowed_tools: vec!["missing.tool".to_string()],
        role: Role::Agent,
    });

    assert!(result.is_err());
}

#[test]
fn attach_and_detach_skill_to_task() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");

    let task = runtime
        .add_task(TaskAddParams {
            title: "task".to_string(),
            ..Default::default()
        })
        .expect("add task");
    runtime
        .add_skill(SkillAddParams {
            name: "alpha".to_string(),
            description: None,
            instructions: "do".to_string(),
            context_files: vec![],
            allowed_tools: vec![],
            role: Role::Agent,
        })
        .expect("add skill");

    runtime
        .attach_skill_to_task(&task.id, "alpha")
        .expect("attach");
    let attached = runtime.list_task_skills(&task.id).expect("list attached");
    assert_eq!(attached.len(), 1);
    assert_eq!(attached[0].name, "alpha");

    runtime
        .detach_skill_from_task(&task.id, "alpha")
        .expect("detach");
    let attached = runtime.list_task_skills(&task.id).expect("list attached");
    assert!(attached.is_empty());
}

#[test]
fn skill_doctor_reports_missing_context_file() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");

    runtime
        .add_skill(SkillAddParams {
            name: "with-missing-file".to_string(),
            description: None,
            instructions: "do".to_string(),
            context_files: vec!["/tmp/definitely_missing_orbit_skill_file".to_string()],
            allowed_tools: vec![],
            role: Role::Agent,
        })
        .expect("add skill");

    let report = runtime.doctor_skills().expect("doctor");
    assert_eq!(report.len(), 1);
    assert_eq!(report[0].status, SkillDoctorStatus::Warning);
}

#[test]
fn legacy_sql_skill_rows_are_exported_to_file_catalog() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");
    runtime
        .add_skill(SkillAddParams {
            name: "legacy-export".to_string(),
            description: Some("legacy desc".to_string()),
            instructions: "legacy instructions".to_string(),
            context_files: vec![],
            allowed_tools: vec![],
            role: Role::Agent,
        })
        .expect("add skill");

    let restarted = OrbitRuntime::from_data_root(dir.path()).expect("runtime restart");
    let file_skills = restarted.list_file_skills().expect("file skills");
    assert!(
        file_skills.iter().any(|skill| skill.id == "legacy-export"),
        "legacy sqlite row should be exported into file skill catalog"
    );
}
