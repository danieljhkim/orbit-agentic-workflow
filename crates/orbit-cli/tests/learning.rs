//! CLI parity tests for `orbit learning <subcommand>`.
//!
//! Per AC #9 of T20260511-6: every MCP-side `orbit.learning.*` tool has a
//! matching CLI subcommand. These black-box tests invoke the CLI against a
//! fresh workspace and assert the JSON output shape matches the host-side
//! serializer (which is what the MCP form returns).

use std::fs;
use std::path::Path;
use std::process::Output;

use assert_cmd::cargo::cargo_bin_cmd;
use serde_json::{Value, json};
use tempfile::{TempDir, tempdir};

#[test]
fn cli_add_then_show_round_trips_every_field() {
    let workspace = TestWorkspace::new();
    let added = workspace.add_learning("rule one", &["foo/**"], &["perf"]);
    let id = added["id"].as_str().expect("id");

    let shown = workspace.run_json(&["learning", "show", id, "--json"], "show learning");
    assert_eq!(shown["id"], added["id"]);
    assert_eq!(shown["summary"], "rule one");
    assert_eq!(shown["scope"]["paths"], json!(["foo/**"]));
    assert_eq!(shown["scope"]["tags"], json!(["perf"]));
    assert_eq!(shown["status"], "active");
}

#[test]
fn cli_search_returns_matched_by_annotation_array() {
    let workspace = TestWorkspace::new();
    workspace.add_learning("path scope", &["foo/**"], &[]);
    workspace.add_learning("tag scope", &[], &["alpha"]);

    let path_hits = workspace.run_json(
        &["learning", "search", "--path", "foo/bar.rs", "--json"],
        "search by path",
    );
    let arr = path_hits.as_array().expect("array");
    assert!(
        !arr.is_empty(),
        "path search should return at least one row"
    );
    for row in arr {
        let matched_by = row["matched_by"].as_array().expect("matched_by present");
        assert!(!matched_by.is_empty());
        let first = matched_by[0].as_str().expect("string");
        assert!(
            first.starts_with("path:") || first.starts_with("tag:") || first.starts_with("query:"),
            "matched_by axis prefix must be path:|tag:|query:"
        );
    }
}

#[test]
fn cli_list_filters_by_status_and_returns_json_array() {
    let workspace = TestWorkspace::new();
    let _a = workspace.add_learning("a", &["a/**"], &[]);
    let b = workspace.add_learning("b", &["b/**"], &[]);
    let c = workspace.add_learning("c", &["c/**"], &[]);

    // Supersede b with c, then list active vs superseded.
    workspace.run(
        &[
            "learning",
            "supersede",
            b["id"].as_str().unwrap(),
            "--with",
            c["id"].as_str().unwrap(),
            "--json",
        ],
        None,
        "supersede",
    );

    let active = workspace.run_json(
        &["learning", "list", "--status", "active", "--json"],
        "list active",
    );
    let active_ids: Vec<&str> = active
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    assert!(!active_ids.contains(&b["id"].as_str().unwrap()));

    let superseded = workspace.run_json(
        &["learning", "list", "--status", "superseded", "--json"],
        "list superseded",
    );
    let superseded_ids: Vec<&str> = superseded
        .as_array()
        .unwrap()
        .iter()
        .map(|item| item["id"].as_str().unwrap())
        .collect();
    assert!(superseded_ids.contains(&b["id"].as_str().unwrap()));
}

#[test]
fn cli_update_then_show_reflects_changes() {
    let workspace = TestWorkspace::new();
    let added = workspace.add_learning("original", &["foo/**"], &["alpha"]);
    let id = added["id"].as_str().unwrap();

    workspace.run(
        &["learning", "update", id, "--summary", "revised", "--json"],
        None,
        "update summary",
    );
    let shown = workspace.run_json(&["learning", "show", id, "--json"], "show updated");
    assert_eq!(shown["summary"], "revised");
}

#[test]
fn cli_reindex_returns_rebuilt_count() {
    let workspace = TestWorkspace::new();
    workspace.add_learning("a", &[], &[]);
    workspace.add_learning("b", &[], &[]);
    let result = workspace.run_json(&["learning", "reindex", "--json"], "reindex");
    assert!(result["rebuilt_count"].as_u64().unwrap() >= 2);
}

#[test]
fn cli_prune_stale_only_reports_without_modifying() {
    let workspace = TestWorkspace::new();
    let learning = workspace.add_learning("stale", &["totally-nonexistent-dir-xyz-123/**"], &[]);
    let report = workspace.run_json(&["learning", "prune", "--json"], "prune stale only");
    let stale = report["stale"].as_array().expect("stale array");
    let stale_ids: Vec<&str> = stale.iter().map(|v| v.as_str().unwrap()).collect();
    assert!(stale_ids.contains(&learning["id"].as_str().unwrap()));
    assert!(report["deleted"].as_array().unwrap().is_empty());
}

#[test]
fn cli_prune_delete_archives_stale_learnings() {
    let workspace = TestWorkspace::new();
    let learning = workspace.add_learning("stale", &["totally-nonexistent-dir-xyz-456/**"], &[]);
    let result = workspace.run_json(&["learning", "prune", "--delete", "--json"], "prune delete");
    let deleted: Vec<&str> = result["deleted"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert!(deleted.contains(&learning["id"].as_str().unwrap()));

    // Verify the YAML moved under `superseded/` with status=superseded and
    // superseded_by=null per §7.3.
    let shown = workspace.run_json(
        &[
            "learning",
            "show",
            learning["id"].as_str().unwrap(),
            "--json",
        ],
        "show archived",
    );
    assert_eq!(shown["status"], "superseded");
    assert!(shown["superseded_by"].is_null());
}

struct TestWorkspace {
    _temp: TempDir,
    home: std::path::PathBuf,
    work: std::path::PathBuf,
}

impl TestWorkspace {
    fn new() -> Self {
        let temp = tempdir().expect("tempdir");
        let home = temp.path().join("home");
        let work = temp.path().join("work");
        fs::create_dir_all(&home).expect("create home");
        fs::create_dir_all(&work).expect("create work");

        let workspace = Self {
            _temp: temp,
            home,
            work,
        };
        workspace.run(
            &["workspace", "init", "--name", "learning-cli-test"],
            None,
            "initialize workspace",
        );
        workspace
    }

    fn add_learning(&self, summary: &str, paths: &[&str], tags: &[&str]) -> Value {
        let mut args = vec!["learning", "add", "--summary", summary, "--json"];
        for path in paths {
            args.push("--path");
            args.push(*path);
        }
        for tag in tags {
            args.push("--tag");
            args.push(*tag);
        }
        self.run_json(&args, "add learning")
    }

    fn run(&self, args: &[&str], stdin: Option<&str>, label: &str) -> Output {
        let output = run_orbit(&self.work, &self.home, args, stdin);
        assert!(
            output.status.success(),
            "{label} failed\nargs: {args:?}\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }

    fn run_json(&self, args: &[&str], label: &str) -> Value {
        let output = self.run(args, None, label);
        serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
            panic!(
                "{label} produced invalid JSON: {e}\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            )
        })
    }
}

fn run_orbit(cwd: &Path, home: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    let mut command = cargo_bin_cmd!("orbit");
    command
        .current_dir(cwd)
        .env("HOME", home)
        .env("USERPROFILE", home)
        .env_remove("ORBIT_ROOT")
        .args(args);
    if let Some(input) = stdin {
        command.write_stdin(input);
    }
    command.output().expect("run orbit")
}
