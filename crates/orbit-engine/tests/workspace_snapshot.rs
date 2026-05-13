#![allow(missing_docs)]
// ORB-00013: Tests use unwrap/expect to keep fixture setup readable.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};
use std::process::Command;

use orbit_engine::WorkspaceSnapshot;

#[test]
fn rewind_restores_the_task_branch_and_keeps_the_scratch_branch()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = TestRepo::new("task-groundhog")?;
    let snapshot = WorkspaceSnapshot::create("T20260420-0509-4", 1, repo.path())?;

    let created_file = repo.path().join("scratch.txt");
    std::fs::write(&created_file, "hello from scratch")?;

    WorkspaceSnapshot::rewind(&snapshot)?;

    assert_eq!(current_branch(repo.path())?, snapshot.task_branch());
    assert_eq!(head_commit(repo.path())?, snapshot.snapshot_ref());
    assert!(branch_exists(repo.path(), snapshot.scratch_branch())?);
    assert!(!created_file.exists());
    Ok(())
}

#[test]
fn commit_success_squashes_the_day_onto_the_task_branch() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = TestRepo::new("task-groundhog")?;
    let task_head_before = head_commit(repo.path())?;
    let snapshot = WorkspaceSnapshot::create("T20260420-0509-4", 1, repo.path())?;

    let created_file = repo.path().join("success.txt");
    std::fs::write(&created_file, "hello from success")?;

    WorkspaceSnapshot::commit_success(&snapshot, "checkpoint summary")?;

    assert_eq!(current_branch(repo.path())?, snapshot.task_branch());
    assert!(created_file.exists());
    assert!(!branch_exists(repo.path(), snapshot.scratch_branch())?);
    assert_eq!(last_commit_subject(repo.path())?, "checkpoint summary");
    assert_ne!(head_commit(repo.path())?, task_head_before);
    Ok(())
}

#[test]
fn rewind_without_edits_leaves_the_task_branch_without_a_new_commit()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = TestRepo::new("task-groundhog")?;
    let task_head_before = head_commit(repo.path())?;
    let snapshot = WorkspaceSnapshot::create("T20260420-0509-4", 2, repo.path())?;

    WorkspaceSnapshot::rewind(&snapshot)?;

    assert_eq!(current_branch(repo.path())?, snapshot.task_branch());
    assert_eq!(head_commit(repo.path())?, task_head_before);
    assert!(branch_exists(repo.path(), snapshot.scratch_branch())?);
    Ok(())
}

#[test]
fn rewind_preserves_preexisting_untracked_files() -> Result<(), Box<dyn std::error::Error>> {
    let repo = TestRepo::new("task-groundhog")?;
    let preserved_file = repo.path().join("keep.txt");
    std::fs::write(&preserved_file, "keep me")?;
    let snapshot = WorkspaceSnapshot::create("T20260420-0509-4", 3, repo.path())?;

    let created_file = repo.path().join("scratch.txt");
    std::fs::write(&created_file, "remove me")?;

    WorkspaceSnapshot::rewind(&snapshot)?;

    assert!(preserved_file.exists());
    assert_eq!(std::fs::read_to_string(&preserved_file)?, "keep me");
    assert!(!created_file.exists());
    Ok(())
}

#[test]
fn rewind_aborts_if_the_task_branch_moves_during_the_day() -> Result<(), Box<dyn std::error::Error>>
{
    let repo = TestRepo::new("task-groundhog")?;
    let snapshot = WorkspaceSnapshot::create("T20260420-0509-4", 4, repo.path())?;
    let parallel = repo.add_worktree(snapshot.task_branch())?;

    std::fs::write(parallel.path().join("parallel.txt"), "parallel change")?;
    git(parallel.path(), &["add", "parallel.txt"])?;
    git(
        parallel.path(),
        &["commit", "-m", "parallel task branch commit"],
    )?;

    let err = WorkspaceSnapshot::rewind(&snapshot).unwrap_err();
    let err_text = err.to_string();

    assert!(err_text.contains("moved from"), "{err_text}");
    assert!(
        err_text.contains("manual intervention required"),
        "{err_text}"
    );
    assert!(err_text.contains(snapshot.task_branch()), "{err_text}");
    Ok(())
}

#[test]
fn create_names_the_existing_branch_when_the_scratch_branch_collides()
-> Result<(), Box<dyn std::error::Error>> {
    let repo = TestRepo::new("task-groundhog")?;
    let collision_branch = "groundhog/T20260420-0509-4/day-5";

    git(repo.path(), &["checkout", "-b", collision_branch])?;
    git(repo.path(), &["checkout", "task-groundhog"])?;

    let err = WorkspaceSnapshot::create("T20260420-0509-4", 5, repo.path()).unwrap_err();
    let err_text = err.to_string();
    assert!(err_text.contains(collision_branch), "{err_text}");

    Ok(())
}

struct TestRepo {
    _tempdir: tempfile::TempDir,
    path: PathBuf,
}

impl TestRepo {
    fn new(task_branch: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let tempdir = tempfile::tempdir()?;
        let path = tempdir.path().to_path_buf();

        git(&path, &["init", "-b", "main"])?;
        git(&path, &["config", "user.name", "Orbit Tests"])?;
        git(&path, &["config", "user.email", "orbit-tests@example.com"])?;
        std::fs::write(path.join("README.md"), "initial\n")?;
        git(&path, &["add", "README.md"])?;
        git(&path, &["commit", "-m", "initial commit"])?;
        git(&path, &["checkout", "-b", task_branch])?;

        Ok(Self {
            _tempdir: tempdir,
            path,
        })
    }

    fn path(&self) -> &Path {
        &self.path
    }

    fn add_worktree(&self, branch: &str) -> Result<TestWorktree, Box<dyn std::error::Error>> {
        let tempdir = tempfile::tempdir()?;
        let path = tempdir.path().join("worktree");
        let path_str = path.to_string_lossy().to_string();
        git(self.path(), &["worktree", "add", &path_str, branch])?;

        Ok(TestWorktree {
            _tempdir: tempdir,
            path,
        })
    }
}

struct TestWorktree {
    _tempdir: tempfile::TempDir,
    path: PathBuf,
}

impl TestWorktree {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn git(repo: &Path, args: &[&str]) -> Result<String, Box<dyn std::error::Error>> {
    let output = Command::new("git").args(args).current_dir(repo).output()?;
    if !output.status.success() {
        return Err(format!(
            "git {} failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        )
        .into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn current_branch(repo: &Path) -> Result<String, Box<dyn std::error::Error>> {
    git(repo, &["rev-parse", "--abbrev-ref", "HEAD"])
}

fn head_commit(repo: &Path) -> Result<String, Box<dyn std::error::Error>> {
    git(repo, &["rev-parse", "HEAD"])
}

fn last_commit_subject(repo: &Path) -> Result<String, Box<dyn std::error::Error>> {
    git(repo, &["log", "-1", "--pretty=%s"])
}

fn branch_exists(repo: &Path, branch: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let status = Command::new("git")
        .args([
            "show-ref",
            "--verify",
            "--quiet",
            &format!("refs/heads/{branch}"),
        ])
        .current_dir(repo)
        .status()?;
    Ok(status.success())
}
