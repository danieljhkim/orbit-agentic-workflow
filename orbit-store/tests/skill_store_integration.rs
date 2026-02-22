use orbit_store::Store;
use orbit_store::task_store::TaskInsertParams;
use orbit_types::{Role, Skill};

fn sample_skill(name: &str) -> Skill {
    let now = chrono::Utc::now();
    Skill {
        schema_version: 1,
        name: name.to_string(),
        description: Some("desc".to_string()),
        instructions: "instructions".to_string(),
        context_files: vec!["ARCHITECTURE.md".to_string()],
        allowed_tools: vec!["fs.read".to_string()],
        role: Role::Agent,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn skill_name_is_unique() {
    let store = Store::open_in_memory().expect("store");
    let skill = sample_skill("unique");
    store
        .with_transaction(|tx| {
            tx.insert_skill(&skill)?;
            Ok(())
        })
        .expect("first insert");

    let err = store
        .with_transaction(|tx| {
            tx.insert_skill(&skill)?;
            Ok(())
        })
        .expect_err("duplicate insert should fail");
    assert!(err.to_string().contains("UNIQUE"));
}

#[test]
fn attach_detach_and_ordering_are_deterministic() {
    let store = Store::open_in_memory().expect("store");
    let task = store
        .with_transaction(|tx| {
            tx.insert_task(&TaskInsertParams {
                title: "task".to_string(),
                ..Default::default()
            })
        })
        .expect("task");

    store
        .with_transaction(|tx| {
            tx.insert_skill(&sample_skill("b"))?;
            tx.insert_skill(&sample_skill("a"))?;
            tx.attach_skill_to_task(&task.id, "b")?;
            tx.attach_skill_to_task(&task.id, "a")?;
            Ok(())
        })
        .expect("attach");

    let attachments = store
        .list_task_skill_attachments(&task.id)
        .expect("attachments");
    assert_eq!(attachments.len(), 2);
    assert_eq!(attachments[0].skill_name, "b");
    assert_eq!(attachments[1].skill_name, "a");

    store
        .with_transaction(|tx| {
            tx.detach_skill_from_task(&task.id, "b")?;
            Ok(())
        })
        .expect("detach");
    let attachments = store
        .list_task_skill_attachments(&task.id)
        .expect("attachments");
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].skill_name, "a");
}
