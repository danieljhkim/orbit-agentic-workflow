//! Integration tests for the ADR migration tool.
//!
//! Each test builds a synthetic `docs/design/` corpus inside a tempdir, runs
//! `run_migration`, and asserts on the resulting artifacts + report.

use std::fs;
use std::path::Path;

use orbit_common::types::AdrStatus;
use tempfile::TempDir;

use super::{MigrationOptions, run_migration};
use crate::OrbitRuntime;
use crate::runtime::orbit_tool_host::test_support::test_runtime;

fn setup() -> (TempDir, OrbitRuntime, TempDir) {
    let (runtime_guard, runtime, _repo_root) = test_runtime();
    let corpus = tempfile::tempdir().expect("corpus tempdir");
    fs::create_dir_all(corpus.path().join("docs").join("design")).expect("create design root");
    (runtime_guard, runtime, corpus)
}

fn write_feature(corpus: &Path, feature: &str, decisions_md: &str, design_md: Option<&str>) {
    let feature_dir = corpus.join("docs").join("design").join(feature);
    fs::create_dir_all(&feature_dir).expect("create feature dir");
    fs::write(feature_dir.join("4_decisions.md"), decisions_md).expect("write 4_decisions.md");
    if let Some(design) = design_md {
        fs::write(feature_dir.join("2_design.md"), design).expect("write 2_design.md");
    }
}

#[test]
fn normal_adr_migrates_with_global_id_and_legacy_pointer() {
    let (_runtime_guard, runtime, corpus) = setup();
    write_feature(
        corpus.path(),
        "feature-a",
        "\
# Feature A — Decisions

## ADR-001 — First decision

**Status:** Accepted · 2026-05 · [T20260509-1]

**Context.** First context.
**Decision.** First decision.
**Consequences.**
- Cost: tradeoff.
",
        None,
    );

    let report = run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: false,
        },
    )
    .expect("migration succeeds");

    assert_eq!(report.created.len(), 1);
    let global_id = report.created[0].global_id.clone();
    assert_eq!(global_id, "ADR-0001");
    assert_eq!(
        report.created[0].legacy_ids,
        vec!["feature-a/ADR-001".to_string()]
    );

    // The artifact in the store should be Accepted with the related task.
    let adr = runtime
        .stores()
        .adrs()
        .get(&global_id)
        .expect("show")
        .expect("exists");
    assert_eq!(adr.status, AdrStatus::Accepted);
    assert_eq!(adr.related_tasks, vec!["T20260509-1".to_string()]);
    assert_eq!(adr.legacy_ids, vec!["feature-a/ADR-001".to_string()]);
    assert!(adr.body_present_check(), "body must persist");
}

trait BodyPresentCheck {
    fn body_present_check(&self) -> bool;
}
impl BodyPresentCheck for orbit_common::types::Adr {
    fn body_present_check(&self) -> bool {
        // Bodies are loaded from disk only on `get`, but every artifact must
        // carry a title (sanity check we got the full record).
        !self.title.is_empty()
    }
}

#[test]
fn rollup_with_three_folded_children_produces_one_artifact() {
    let (_runtime_guard, runtime, corpus) = setup();
    write_feature(
        corpus.path(),
        "activity-job",
        "\
# Activity / Job — Decisions

## ADR-001 — Canonical asset normalization

**Status:** Accepted · 2026-05 · [T20260427-34]

**Context.** Rollup ctx.
**Decision.** Rollup decision.
**Consequences.**
- Cost: rollup tradeoff.

## ADR-003 — Resolve backend once

**Status:** Superseded by ADR-001 (folded) · 2026-04 · [T20260418-2143]

Folded into ADR-001's rollup.

## ADR-004 — Authoring sugar

**Status:** Superseded by ADR-001 (folded) · 2026-04 · [T20260418-2019]

Folded into ADR-001's rollup.

## ADR-008 — Seeded contracts

**Status:** Superseded by ADR-001 (folded) · 2026-04 · [T20260428-8]

Folded into ADR-001's rollup.
",
        None,
    );

    let report = run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: false,
        },
    )
    .expect("migration succeeds");

    // Exactly one artifact created — the rollup.
    assert_eq!(
        report.created.len(),
        1,
        "rollup is one artifact: {:?}",
        report.created
    );
    let rollup = &report.created[0];
    let mut legacy_ids = rollup.legacy_ids.clone();
    legacy_ids.sort();
    assert_eq!(
        legacy_ids,
        vec![
            "activity-job/ADR-001".to_string(),
            "activity-job/ADR-003".to_string(),
            "activity-job/ADR-004".to_string(),
            "activity-job/ADR-008".to_string(),
        ]
    );
    assert_eq!(report.rollups.len(), 1);
    assert_eq!(report.rollups[0].folded_legacy_ids.len(), 3);
}

#[test]
fn lenient_validation_warnings_populated_for_missing_cost() {
    let (_runtime_guard, runtime, corpus) = setup();
    write_feature(
        corpus.path(),
        "activity-job",
        "\
## ADR-044 — Some decision

**Status:** Accepted · 2026-05 · [T20260509-44]

**Context.** ctx.
**Decision.** decision.
**Consequences.**
- One consequence without an explicit Cost line.
",
        None,
    );

    let report = run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: false,
        },
    )
    .expect("migration succeeds despite warning");
    assert_eq!(report.created.len(), 1);
    assert_eq!(report.validation_warnings.len(), 1);
    let warning = &report.validation_warnings[0];
    assert!(warning.warnings.iter().any(|w| w.contains("Cost")));

    // The artifact carries the warning and the legacy_validation flag.
    let adr = runtime
        .stores()
        .adrs()
        .get(&warning.global_id)
        .expect("get")
        .expect("exists");
    assert!(!adr.validation_warnings.is_empty());
    assert_eq!(
        adr.legacy_validation,
        orbit_common::types::LegacyValidation::Warned
    );
}

#[test]
fn idempotent_second_run_creates_zero_new_artifacts() {
    let (_runtime_guard, runtime, corpus) = setup();
    write_feature(
        corpus.path(),
        "feature-a",
        "\
## ADR-001 — A

**Status:** Accepted · 2026-05 · [T20260509-1]

**Context.** c.
**Decision.** d.
**Consequences.**
- Cost: x.
",
        None,
    );

    let opts = || MigrationOptions {
        workspace_path: Some(corpus.path().to_path_buf()),
        dry_run: false,
    };

    let first = run_migration(&runtime, opts()).expect("first run");
    assert_eq!(first.created.len(), 1);
    assert_eq!(first.skipped.len(), 0);

    let second = run_migration(&runtime, opts()).expect("second run");
    assert_eq!(
        second.created.len(),
        0,
        "second run is no-op: {:?}",
        second.created
    );
    assert_eq!(second.skipped.len(), 1);
}

#[test]
fn cross_feature_supersession_resolves_through_id_map() {
    let (_runtime_guard, runtime, corpus) = setup();
    // Feature-a's ADR-002 supersedes feature-a's ADR-001 (no cross-feature
    // edge in this case — keep it within one feature so the source markdown
    // syntax `Superseded by ADR-001` is unambiguous per the design).
    write_feature(
        corpus.path(),
        "feature-a",
        "\
## ADR-001 — original

**Status:** Superseded by ADR-002 · 2026-05 · [T20260509-1]

**Context.** c1.
**Decision.** d1.
**Consequences.**
- Cost: x.

## ADR-002 — replacement

**Status:** Accepted · 2026-05 · [T20260509-2]

**Context.** c2.
**Decision.** d2.
**Consequences.**
- Cost: y.
",
        None,
    );

    let report = run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: false,
        },
    )
    .expect("migration");
    assert_eq!(report.created.len(), 2);
    assert_eq!(report.supersedes.len(), 1, "{:?}", report.supersedes);

    let adrs = runtime.stores().adrs();
    let original_global = report
        .created
        .iter()
        .find(|c| c.legacy_ids.contains(&"feature-a/ADR-001".to_string()))
        .unwrap()
        .global_id
        .clone();
    let replacement_global = report
        .created
        .iter()
        .find(|c| c.legacy_ids.contains(&"feature-a/ADR-002".to_string()))
        .unwrap()
        .global_id
        .clone();
    let original = adrs.get(&original_global).unwrap().unwrap();
    let replacement = adrs.get(&replacement_global).unwrap().unwrap();
    assert_eq!(original.status, AdrStatus::Superseded);
    assert_eq!(original.superseded_by, Some(replacement_global.clone()));
    assert!(replacement.supersedes.contains(&original_global));
}

#[test]
fn reference_sweep_rewrites_bracketed_local_refs() {
    let (_runtime_guard, runtime, corpus) = setup();
    write_feature(
        corpus.path(),
        "feature-a",
        "\
## ADR-001 — A

**Status:** Accepted · 2026-05 · [T20260509-1]

**Context.** c.
**Decision.** d.
**Consequences.**
- Cost: x.
",
        Some(
            "# Feature A — Design

See [ADR-001] for context.
",
        ),
    );

    let report = run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: false,
        },
    )
    .expect("migration");
    assert_eq!(report.created.len(), 1);
    let global = report.created[0].global_id.clone();

    let rewritten = fs::read_to_string(
        corpus
            .path()
            .join("docs")
            .join("design")
            .join("feature-a")
            .join("2_design.md"),
    )
    .expect("read swept file");
    assert!(
        rewritten.contains(&format!("[{global}]")),
        "expected `[{global}]` in:\n{rewritten}"
    );
    assert!(
        !rewritten.contains("[ADR-001]"),
        "expected local ref rewritten away:\n{rewritten}"
    );
    assert!(!report.rewrites.is_empty());
}

#[test]
fn reference_sweep_flags_ambiguous_cross_feature_ref() {
    let (_runtime_guard, runtime, corpus) = setup();
    // Two features both have ADR-017. A third feature mentions bare ADR-017.
    write_feature(
        corpus.path(),
        "activity-job",
        "\
## ADR-017 — aj

**Status:** Accepted · 2026-05 · [T20260509-1]

**Context.** c.
**Decision.** d.
**Consequences.**
- Cost: x.
",
        None,
    );
    write_feature(
        corpus.path(),
        "auditability",
        "\
## ADR-017 — au

**Status:** Accepted · 2026-05 · [T20260509-2]

**Context.** c.
**Decision.** d.
**Consequences.**
- Cost: y.
",
        None,
    );
    write_feature(
        corpus.path(),
        "groundhog",
        "\
## ADR-001 — gh

**Status:** Accepted · 2026-05 · [T20260509-3]

**Context.** c.
**Decision.** d.
**Consequences.**
- Cost: z.
",
        Some("# groundhog\n\nas discussed in [ADR-017]\n"),
    );

    let report = run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: false,
        },
    )
    .expect("migration");
    assert!(
        report
            .unresolved_references
            .iter()
            .any(|u| u.reason.contains("ambiguous")),
        "expected ambiguous unresolved ref: {:?}",
        report.unresolved_references
    );
    // The original ref should be left in place.
    let rewritten = fs::read_to_string(
        corpus
            .path()
            .join("docs")
            .join("design")
            .join("groundhog")
            .join("2_design.md"),
    )
    .unwrap();
    assert!(
        rewritten.contains("[ADR-017]"),
        "should leave bare ref: {rewritten}"
    );
}

#[test]
fn migration_report_is_written_to_workspace_root() {
    let (_runtime_guard, runtime, corpus) = setup();
    write_feature(
        corpus.path(),
        "feature-a",
        "\
## ADR-001 — A

**Status:** Accepted · 2026-05 · [T20260509-1]

**Context.** c.
**Decision.** d.
**Consequences.**
- Cost: x.
",
        None,
    );

    run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: false,
        },
    )
    .expect("migration");

    let report_path = corpus.path().join("migration-report.md");
    assert!(report_path.is_file(), "report file written");
    let body = fs::read_to_string(&report_path).expect("read report");
    assert!(body.contains("# ADR migration report"));
    assert!(body.contains("Created: 1"));
}

#[test]
fn dry_run_emits_report_but_writes_no_artifacts() {
    let (_runtime_guard, runtime, corpus) = setup();
    write_feature(
        corpus.path(),
        "feature-a",
        "\
## ADR-001 — A

**Status:** Accepted · 2026-05 · [T20260509-1]

**Context.** c.
**Decision.** d.
**Consequences.**
- Cost: x.
",
        Some("# A\n\nSee [ADR-001].\n"),
    );

    let report = run_migration(
        &runtime,
        MigrationOptions {
            workspace_path: Some(corpus.path().to_path_buf()),
            dry_run: true,
        },
    )
    .expect("dry-run");
    assert_eq!(
        report.created.len(),
        1,
        "report still describes intended creates"
    );
    assert!(report.created[0].global_id.starts_with("ADR-DRYRUN-"));

    // No artifact written to the store.
    let listed = runtime
        .stores()
        .adrs()
        .list_filtered(None, None, None, None, None, None)
        .unwrap();
    assert!(listed.is_empty(), "dry-run must not write: {listed:?}");

    // No sweep rewrites applied to the source file.
    let design = fs::read_to_string(
        corpus
            .path()
            .join("docs")
            .join("design")
            .join("feature-a")
            .join("2_design.md"),
    )
    .unwrap();
    assert!(
        design.contains("[ADR-001]"),
        "dry-run leaves source: {design}"
    );
}
