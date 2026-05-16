//! One-shot migration of the markdown ADR corpus into the v2 artifact store.
//!
//! See `docs/design/adr-artifact/2_design.md` §7. This module ships the tool
//! only; running it against the real `docs/design/*/4_decisions.md` corpus is
//! a separate task gated on human review of a dry-run report.
//!
//! ## Flow
//!
//! 1. **Pass one** — `parse.rs` walks every `docs/design/<feature>/4_decisions.md`,
//!    parses each `## ADR-NNN — title` heading into a [`ParsedAdrEntry`], and
//!    runs lenient validation per ADR-011.
//! 2. **Rollup resolution** — `rollup.rs` collapses folded headings (CONVENTIONS
//!    §4a, `Status: Superseded by ADR-NNN (folded)`) into a single rollup
//!    artifact whose [`Adr::legacy_ids`] carries every folded source path.
//! 3. **Ingest** — `ingest.rs` allocates global IDs (skipping entries whose
//!    `(feature, legacy_id)` is already in the store for idempotency), writes
//!    artifacts via the same handler functions used by the tool surface
//!    (`adr_tools::add` then `adr_tools::update`), and resolves cross-feature
//!    supersession edges through the now-populated map.
//! 4. **Sweep** — `sweep.rs` rewrites four reference forms across
//!    `docs/design/**/*.md` (excluding `4_decisions.md`), logging ambiguous
//!    cases to the report rather than guessing.
//! 5. **Report** — `report.rs` emits `migration-report.md` at the workspace
//!    root with validation warnings, unresolved references, and rollup
//!    composition.

use std::path::{Path, PathBuf};

use orbit_common::types::OrbitError;

use crate::OrbitRuntime;

mod ingest;
mod parse;
mod report;
mod rollup;
mod sweep;

#[cfg(test)]
mod tests;

pub use parse::{EntryKind, ParsedAdrEntry, ParsedStatus};

/// Options controlling a migration run.
#[derive(Debug, Default, Clone)]
pub struct MigrationOptions {
    /// Root containing `docs/design/`. Defaults to the runtime's repo root.
    pub workspace_path: Option<PathBuf>,
    /// When true, run the parser + sweep simulation and emit the report, but
    /// do not write artifacts to the store or rewrite source files.
    pub dry_run: bool,
}

/// Summary returned by [`run_migration`]. Mirrors the structured contents of
/// `migration-report.md` for programmatic inspection (used by tests).
#[derive(Debug, Default, Clone)]
pub struct MigrationReport {
    /// Newly created ADRs (one entry per write). Empty for fully-idempotent re-runs.
    pub created: Vec<CreatedRecord>,
    /// Existing ADRs the migration skipped because a matching `legacy_ids`
    /// entry was already in the store.
    pub skipped: Vec<SkippedRecord>,
    /// Validation warnings recorded on imported artifacts (lenient mode).
    pub validation_warnings: Vec<ValidationWarningRecord>,
    /// Rollup artifacts and the folded source paths absorbed by each.
    pub rollups: Vec<RollupRecord>,
    /// Reference-sweep rewrites that did happen (or would happen, in dry-run mode).
    pub rewrites: Vec<RewriteRecord>,
    /// References the sweep refused to rewrite (ambiguous / unresolvable).
    pub unresolved_references: Vec<UnresolvedRefRecord>,
    /// Supersession edges actually written.
    pub supersedes: Vec<SupersedeRecord>,
}

#[derive(Debug, Clone)]
pub struct CreatedRecord {
    pub global_id: String,
    pub legacy_ids: Vec<String>,
    pub source_path: PathBuf,
    pub title: String,
}

#[derive(Debug, Clone)]
pub struct SkippedRecord {
    pub global_id: String,
    pub legacy_id: String,
    pub source_path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct ValidationWarningRecord {
    pub global_id: String,
    pub legacy_id: String,
    pub source_path: PathBuf,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RollupRecord {
    pub global_id: String,
    pub rollup_legacy_id: String,
    pub source_path: PathBuf,
    pub folded_legacy_ids: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct RewriteRecord {
    pub file: PathBuf,
    pub line: usize,
    pub original: String,
    pub rewritten: String,
}

#[derive(Debug, Clone)]
pub struct UnresolvedRefRecord {
    pub file: PathBuf,
    pub line: usize,
    pub original: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SupersedeRecord {
    pub old_global_id: String,
    pub new_global_id: String,
    pub source_path: PathBuf,
}

/// Run a migration end-to-end. The returned [`MigrationReport`] mirrors
/// `migration-report.md`, which is also written to disk at the workspace
/// root (unless dry-run is set, in which case only the report file is
/// written).
pub fn run_migration(
    runtime: &OrbitRuntime,
    options: MigrationOptions,
) -> Result<MigrationReport, OrbitError> {
    let workspace_path = resolve_workspace(runtime, options.workspace_path.as_deref());
    let design_root = workspace_path.join("docs").join("design");

    let mut parsed = parse::parse_corpus(&design_root)?;
    rollup::resolve_rollups(&mut parsed);

    let ingest_outcome = ingest::ingest(runtime, &parsed, options.dry_run)?;
    let sweep_outcome = sweep::run_sweep(&design_root, &ingest_outcome.id_map, options.dry_run)?;

    let mut report = MigrationReport {
        created: ingest_outcome.created,
        skipped: ingest_outcome.skipped,
        validation_warnings: ingest_outcome.validation_warnings,
        rollups: ingest_outcome.rollups,
        supersedes: ingest_outcome.supersedes,
        rewrites: sweep_outcome.rewrites,
        unresolved_references: sweep_outcome.unresolved,
    };

    let report_path = workspace_path.join("migration-report.md");
    report::write_report(&report_path, &report)?;
    report.created.sort_by(|a, b| a.global_id.cmp(&b.global_id));

    Ok(report)
}

fn resolve_workspace(runtime: &OrbitRuntime, override_path: Option<&Path>) -> PathBuf {
    if let Some(path) = override_path {
        return path.to_path_buf();
    }
    runtime.paths().repo_root.clone()
}
