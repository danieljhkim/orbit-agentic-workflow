use assert_cmd::Command;
use predicates::prelude::*;
use std::path::Path;

fn orbit_in(dir: &Path) -> Command {
    #[allow(deprecated)]
    let mut cmd = Command::cargo_bin("orbit").expect("binary exists");
    cmd.current_dir(dir);
    cmd
}

fn write_instructions(dir: &Path, name: &str, content: &str) -> String {
    let path = dir.join(name);
    std::fs::write(&path, content).expect("write instructions");
    path.to_string_lossy().to_string()
}

fn add_task(dir: &Path, title: &str) -> String {
    let output = orbit_in(dir)
        .args(["task", "add", "--title", title])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    String::from_utf8(output).expect("utf8").trim().to_string()
}

#[test]
fn skill_add_list_show_update_delete_flow() {
    let dir = tempfile::tempdir().expect("tempdir");
    let instructions = write_instructions(dir.path(), "skill.txt", "Always verify invariants");

    orbit_in(dir.path())
        .args([
            "skill",
            "add",
            "--name",
            "refactor-rust",
            "--description",
            "Refactor helper",
            "--instructions",
            &instructions,
            "--allowed-tools",
            "fs.read,fs.write",
            "--role",
            "agent",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added skill 'refactor-rust'"));

    orbit_in(dir.path())
        .args(["skill", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("refactor-rust"));

    orbit_in(dir.path())
        .args(["skill", "show", "refactor-rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Name:"))
        .stdout(predicate::str::contains("refactor-rust"))
        .stdout(predicate::str::contains("agent"));

    let updated = write_instructions(dir.path(), "skill-update.txt", "Updated instructions");
    orbit_in(dir.path())
        .args([
            "skill",
            "update",
            "refactor-rust",
            "--instructions",
            &updated,
            "--allowed-tools",
            "fs.read",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Updated skill 'refactor-rust'"));

    orbit_in(dir.path())
        .args(["skill", "delete", "refactor-rust"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Deleted skill 'refactor-rust'"));
}

#[test]
fn skill_attach_and_detach_to_task() {
    let dir = tempfile::tempdir().expect("tempdir");
    let instructions = write_instructions(dir.path(), "attach-skill.txt", "Attach skill");
    let task_id = add_task(dir.path(), "needs-skill");

    orbit_in(dir.path())
        .args([
            "skill",
            "add",
            "--name",
            "attachable",
            "--instructions",
            &instructions,
        ])
        .assert()
        .success();

    orbit_in(dir.path())
        .args(["skill", "attach", &task_id, "attachable"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Attached skill"));

    orbit_in(dir.path())
        .args(["skill", "detach", &task_id, "attachable"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Detached skill"));
}

#[test]
fn skill_doctor_reports_health() {
    let dir = tempfile::tempdir().expect("tempdir");
    let instructions = write_instructions(dir.path(), "doctor-skill.txt", "Doctor skill");
    let ctx = dir.path().join("ARCHITECTURE.md");
    std::fs::write(&ctx, "ok").expect("write context");

    orbit_in(dir.path())
        .args([
            "skill",
            "add",
            "--name",
            "doctor-ok",
            "--instructions",
            &instructions,
            "--context",
            &ctx.to_string_lossy(),
        ])
        .assert()
        .success();

    orbit_in(dir.path())
        .args(["skill", "doctor"])
        .assert()
        .success()
        .stdout(predicate::str::contains("doctor-ok"))
        .stdout(predicate::str::contains("All skills healthy"));
}
