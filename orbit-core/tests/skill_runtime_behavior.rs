use orbit_core::OrbitRuntime;
use tempfile::tempdir;

#[test]
fn file_skill_catalog_commands_succeed() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");
    let skill_dir = dir.path().join("skills").join("orbit-assess-codebase");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        r#"---
name: orbit-assess-codebase
description: Review codebase architecture.
---

# Assess Codebase

## Purpose
Review codebase architecture.

## Behavioral Constraints
- Return deterministic output.

## Output Requirements
- JSON response.
"#,
    )
    .expect("write skill");
    std::fs::write(
        skill_dir.join("meta.json"),
        r#"{
  "name": "Assess Codebase",
  "version": "1.0.0",
  "type": "object",
  "required": ["summary"],
  "properties": {
    "summary": {"type": "string"}
  }
}"#,
    )
    .expect("write meta");

    let listed = runtime.list_file_skills().expect("list");
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, "orbit-assess-codebase");
    assert!(listed[0].output_schema.is_some());

    let shown = runtime
        .show_file_skill("orbit-assess-codebase")
        .expect("show");
    assert_eq!(shown.sections.purpose, "Review codebase architecture.");
    assert_eq!(
        shown.meta.and_then(|meta| meta.name).as_deref(),
        Some("Assess Codebase")
    );

    let doctor = runtime.doctor_file_skills().expect("doctor");
    assert_eq!(doctor.len(), 1);
    assert_eq!(
        doctor[0].status,
        orbit_core::command::skill::SkillDoctorStatus::Ok
    );
}
