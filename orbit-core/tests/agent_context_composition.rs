use orbit_core::OrbitRuntime;
use orbit_core::agent::context::{compose_agent_context, parse_planned_tool_calls};
use orbit_core::command::skill::SkillAddParams;
use orbit_core::command::task::TaskAddParams;
use orbit_types::Role;
use tempfile::tempdir;

#[test]
fn composition_merges_in_deterministic_order_and_hash_is_stable() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");

    let task = runtime
        .add_task(TaskAddParams {
            title: "compose".to_string(),
            description: "task description".to_string(),
            instructions: r#"{"tool_calls":[{"name":"fs.read","input":{"path":"README.md"}}]}"#
                .to_string(),
            context_files: vec!["A.md".to_string(), "B.md".to_string()],
            ..Default::default()
        })
        .expect("task");

    runtime
        .add_skill(SkillAddParams {
            name: "s1".to_string(),
            description: None,
            instructions: "skill 1".to_string(),
            context_files: vec!["B.md".to_string(), "C.md".to_string()],
            allowed_tools: vec!["fs.read".to_string(), "fs.write".to_string()],
            role: Role::Admin,
        })
        .expect("skill s1");
    runtime
        .add_skill(SkillAddParams {
            name: "s2".to_string(),
            description: None,
            instructions: "skill 2".to_string(),
            context_files: vec!["C.md".to_string(), "D.md".to_string()],
            allowed_tools: vec!["fs.read".to_string()],
            role: Role::Agent,
        })
        .expect("skill s2");

    runtime
        .attach_skill_to_task(&task.id, "s1")
        .expect("attach s1");
    runtime
        .attach_skill_to_task(&task.id, "s2")
        .expect("attach s2");

    let task = runtime.get_task(&task.id).expect("task");
    let skills = runtime.list_task_skills(&task.id).expect("skills");
    let c1 = compose_agent_context(&runtime, &task, &skills, Role::Admin).expect("compose");
    let c2 = compose_agent_context(&runtime, &task, &skills, Role::Admin).expect("compose");

    assert_eq!(
        c1.context_files,
        vec![
            "A.md".to_string(),
            "B.md".to_string(),
            "C.md".to_string(),
            "D.md".to_string()
        ]
    );
    assert_eq!(
        c1.allowed_tools_raw,
        Some(vec!["fs.read".to_string()]),
        "intersection should preserve deterministic order"
    );
    assert_eq!(c1.role, Role::Agent, "most restrictive role wins");
    assert_eq!(
        c1.effective_allowed_tools,
        vec!["fs.read".to_string()],
        "policy ∩ skill allowlist"
    );
    assert_eq!(c1.composed_context_hash, c2.composed_context_hash);
}

#[test]
fn parse_planned_tool_calls_from_task_instructions_payload() {
    let payload = r#"{
      "tool_calls": [
        {"name":"fs.read","input":{"path":"README.md"}},
        {"name":"time.now","input":{}}
      ]
    }"#;

    let calls = parse_planned_tool_calls(payload).expect("parse");
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "fs.read");
    assert_eq!(calls[1].name, "time.now");
}

#[test]
fn empty_effective_allowlist_is_rejected() {
    let dir = tempdir().expect("tempdir");
    let runtime = OrbitRuntime::from_data_root(dir.path()).expect("runtime");
    let task = runtime
        .add_task(TaskAddParams {
            title: "compose".to_string(),
            instructions: r#"{"tool_calls":[{"name":"fs.read","input":{"path":"README.md"}}]}"#
                .to_string(),
            ..Default::default()
        })
        .expect("task");

    runtime
        .add_skill(SkillAddParams {
            name: "deny-all".to_string(),
            description: None,
            instructions: "strict".to_string(),
            context_files: vec![],
            allowed_tools: vec!["does.not.exist".to_string()],
            role: Role::Agent,
        })
        .expect_err("unknown tool should fail at creation");

    runtime
        .add_skill(SkillAddParams {
            name: "only-write".to_string(),
            description: None,
            instructions: "only write".to_string(),
            context_files: vec![],
            allowed_tools: vec!["fs.write".to_string()],
            role: Role::Agent,
        })
        .expect("skill");
    runtime
        .add_skill(SkillAddParams {
            name: "only-read".to_string(),
            description: None,
            instructions: "only read".to_string(),
            context_files: vec![],
            allowed_tools: vec!["fs.read".to_string()],
            role: Role::Agent,
        })
        .expect("skill");
    runtime
        .attach_skill_to_task(&task.id, "only-write")
        .expect("attach");
    runtime
        .attach_skill_to_task(&task.id, "only-read")
        .expect("attach");

    let task = runtime.get_task(&task.id).expect("task");
    let skills = runtime.list_task_skills(&task.id).expect("skills");
    let result = compose_agent_context(&runtime, &task, &skills, Role::Agent);
    assert!(
        result.is_err(),
        "disjoint skill allowlists should produce empty intersection"
    );
}
