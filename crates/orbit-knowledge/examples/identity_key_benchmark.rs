#![allow(missing_docs)]

use std::env;
use std::error::Error;
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use orbit_knowledge::graph::LeafNode;
use orbit_knowledge::graph::object_store::RefName;
use orbit_knowledge::pipeline::context::BuildConfig;
use orbit_knowledge::pipeline::run_build;
use serde::Serialize;
use tempfile::TempDir;

type BenchResult<T> = Result<T, Box<dyn Error>>;

const REF_NAME: &str = "identity-key-v1";
const SIMPLE_SOURCE: &str = "pub fn foo() -> u32 {\n    1\n}\n";
const TWO_FUNCTION_SOURCE: &str =
    "pub fn foo() -> u32 {\n    1\n}\n\npub fn bar() -> u32 {\n    2\n}\n";

#[derive(Debug, Serialize)]
struct RunRecord {
    scenario: &'static str,
    mutation: &'static str,
    before: Vec<LeafRecord>,
    after: Vec<LeafRecord>,
    preserved: bool,
    notes: String,
}

#[derive(Debug, Clone, Serialize)]
struct LeafRecord {
    name: String,
    kind: String,
    identity_key: String,
    id: String,
}

struct ScenarioRepo {
    _tempdir: TempDir,
    repo_path: PathBuf,
    knowledge_dir: PathBuf,
    ref_name: RefName,
}

impl ScenarioRepo {
    fn new(initial_files: &[(&str, &str)]) -> BenchResult<Self> {
        let tempdir = TempDir::new()?;
        let repo_path = tempdir.path().join("repo");
        let knowledge_dir = tempdir.path().join("knowledge");
        fs::create_dir_all(&repo_path)?;

        run_git(&repo_path, ["init", "-b", "main"])?;
        run_git(&repo_path, ["config", "user.name", "Orbit Benchmark"])?;
        run_git(
            &repo_path,
            ["config", "user.email", "orbit-benchmark@example.invalid"],
        )?;

        let repo = Self {
            _tempdir: tempdir,
            repo_path,
            knowledge_dir,
            ref_name: RefName::new(REF_NAME)?,
        };

        for (rel_path, content) in initial_files {
            repo.write_file(rel_path, content)?;
        }
        repo.commit_all("initial fixture")?;

        Ok(repo)
    }

    fn build(&self, incremental: bool, relevant_names: &[&str]) -> BenchResult<Vec<LeafRecord>> {
        let ctx = run_build(BuildConfig {
            repo_path: self.repo_path.clone(),
            output_dir: self.knowledge_dir.clone(),
            incremental,
            ref_name: Some(self.ref_name.clone()),
        })?;

        select_leaf_records(&ctx.graph.leaves, relevant_names)
    }

    fn write_file(&self, rel_path: &str, content: &str) -> BenchResult<()> {
        let path = self.repo_path.join(rel_path);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(path, content)?;
        Ok(())
    }

    fn rename_file(&self, from: &str, to: &str) -> BenchResult<()> {
        let to_path = self.repo_path.join(to);
        if let Some(parent) = to_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::rename(self.repo_path.join(from), to_path)?;
        Ok(())
    }

    fn remove_file(&self, rel_path: &str) -> BenchResult<()> {
        fs::remove_file(self.repo_path.join(rel_path))?;
        Ok(())
    }

    fn commit_all(&self, message: &str) -> BenchResult<()> {
        run_git(&self.repo_path, ["add", "-A"])?;
        run_git(&self.repo_path, ["commit", "-m", message])?;
        Ok(())
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("identity-key benchmark failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> BenchResult<()> {
    let output_dir = parse_output_dir()?;
    fs::create_dir_all(&output_dir)?;
    remove_stale_records(&output_dir)?;

    let records = vec![
        run_rename()?,
        run_move()?,
        run_content_edit()?,
        run_delete_recreate()?,
        run_signature_change()?,
    ];

    for record in records {
        let path = output_dir.join(format!("{}.json", record.scenario));
        let json = serde_json::to_string_pretty(&record)?;
        fs::write(path, format!("{json}\n"))?;
    }

    Ok(())
}

fn parse_output_dir() -> BenchResult<PathBuf> {
    let mut args = env::args_os().skip(1);
    let mut output_dir = None;
    while let Some(arg) = args.next() {
        if arg == "--output" {
            let value = args.next().ok_or("--output requires a path")?;
            output_dir = Some(PathBuf::from(value));
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()).into());
        }
    }

    output_dir.ok_or_else(|| "--output is required".into())
}

fn remove_stale_records(output_dir: &Path) -> BenchResult<()> {
    for entry in fs::read_dir(output_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension() == Some(OsStr::new("json")) {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

fn run_rename() -> BenchResult<RunRecord> {
    let repo = ScenarioRepo::new(&[("a.rs", SIMPLE_SOURCE)])?;
    let before = repo.build(false, &["foo"])?;

    repo.rename_file("a.rs", "b.rs")?;
    repo.commit_all("rename a.rs to b.rs")?;
    let after = repo.build(true, &["foo"])?;

    Ok(record(
        "rename",
        "a.rs -> b.rs",
        before,
        after,
        "Path changed from a.rs to b.rs; identity_key includes the leaf location.",
    ))
}

fn run_move() -> BenchResult<RunRecord> {
    let repo = ScenarioRepo::new(&[("src/a.rs", TWO_FUNCTION_SOURCE)])?;
    let before = repo.build(false, &["foo", "bar"])?;

    repo.rename_file("src/a.rs", "src/sub/a.rs")?;
    repo.commit_all("move src/a.rs to src/sub/a.rs")?;
    let after = repo.build(true, &["foo", "bar"])?;

    Ok(record(
        "move",
        "src/a.rs -> src/sub/a.rs",
        before,
        after,
        "Directory changed from src/ to src/sub/; identity_key includes the leaf location.",
    ))
}

fn run_content_edit() -> BenchResult<RunRecord> {
    let repo = ScenarioRepo::new(&[("a.rs", TWO_FUNCTION_SOURCE)])?;
    let before = repo.build(false, &["foo", "bar"])?;

    repo.write_file(
        "a.rs",
        "pub fn foo() -> u32 {\n    42\n}\n\npub fn bar() -> u32 {\n    2\n}\n",
    )?;
    repo.commit_all("edit foo body")?;
    let after = repo.build(true, &["foo", "bar"])?;

    Ok(record(
        "content_edit",
        "edit foo's body",
        before,
        after,
        "Function body changed, but path, qualified name, and kind stayed fixed.",
    ))
}

fn run_delete_recreate() -> BenchResult<RunRecord> {
    let repo = ScenarioRepo::new(&[("a.rs", SIMPLE_SOURCE)])?;
    let before = repo.build(false, &["foo"])?;

    repo.remove_file("a.rs")?;
    repo.commit_all("delete a.rs")?;
    let _deleted = repo.build(true, &[])?;

    repo.write_file("a.rs", SIMPLE_SOURCE)?;
    repo.commit_all("recreate a.rs")?;
    let after = repo.build(true, &["foo"])?;

    Ok(record(
        "delete_recreate",
        "rm + recreate same content",
        before,
        after,
        "The intermediate rebuild removed the leaf; recreating the same path/name/kind produced the same key.",
    ))
}

fn run_signature_change() -> BenchResult<RunRecord> {
    let repo = ScenarioRepo::new(&[("a.rs", "pub fn foo(x: u32) -> u32 {\n    x\n}\n")])?;
    let before = repo.build(false, &["foo"])?;

    repo.write_file("a.rs", "pub fn foo(x: u64) -> u64 {\n    x\n}\n")?;
    repo.commit_all("change foo signature")?;
    let after = repo.build(true, &["foo"])?;

    Ok(record(
        "signature_change",
        "u32 -> u64",
        before,
        after,
        "Signature text changed, but identity_key is derived from path, qualified name, and kind.",
    ))
}

fn record(
    scenario: &'static str,
    mutation: &'static str,
    before: Vec<LeafRecord>,
    after: Vec<LeafRecord>,
    note: &str,
) -> RunRecord {
    let preserved = identity_keys_preserved(&before, &after);
    let outcome = if preserved {
        "Observed identity_key/id preservation."
    } else {
        "Observed identity_key/id change."
    };
    RunRecord {
        scenario,
        mutation,
        before,
        after,
        preserved,
        notes: format!("{note} {outcome}"),
    }
}

fn select_leaf_records(
    leaves: &[LeafNode],
    relevant_names: &[&str],
) -> BenchResult<Vec<LeafRecord>> {
    relevant_names
        .iter()
        .map(|name| {
            let leaf = leaves
                .iter()
                .find(|leaf| leaf.base.name == *name)
                .ok_or_else(|| format!("missing leaf `{name}`"))?;
            Ok(LeafRecord {
                name: leaf.base.name.clone(),
                kind: leaf.kind.to_string(),
                identity_key: leaf.base.identity_key.clone(),
                id: leaf.base.id.clone(),
            })
        })
        .collect()
}

fn identity_keys_preserved(before: &[LeafRecord], after: &[LeafRecord]) -> bool {
    before.len() == after.len()
        && before.iter().zip(after).all(|(left, right)| {
            left.name == right.name
                && left.kind == right.kind
                && left.identity_key == right.identity_key
                && left.id == right.id
        })
}

fn run_git<I, S>(repo_path: &Path, args: I) -> BenchResult<()>
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_path)
        .output()?;

    if output.status.success() {
        return Ok(());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "git command failed in {}: status={} stdout={} stderr={}",
        repo_path.display(),
        output.status,
        stdout.trim(),
        stderr.trim()
    )
    .into())
}
