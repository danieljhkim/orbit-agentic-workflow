//! Identify CONVENTIONS §4a rollups and mark folded children.
//!
//! Rollups have no structural marker — they are recognized by being the target
//! of folded children (`Status: Superseded by ADR-NNN (folded)`). This module
//! walks the parsed corpus once to identify every (feature, target) folded
//! reference, then promotes those targets to `EntryKind::Rollup` carrying the
//! list of folded legacy IDs.

use std::collections::HashMap;

use super::parse::{EntryKind, ParsedAdrEntry, ParsedStatus};

/// Annotates every entry with its [`EntryKind`]: `Folded` when its status is
/// `Superseded by … (folded)`, `Rollup` when other entries fold into it,
/// otherwise `Standalone`.
pub fn resolve_rollups(entries: &mut [ParsedAdrEntry]) {
    // (feature, target_legacy_id) -> Vec<folded_legacy_id>
    let mut fold_groups: HashMap<(String, String), Vec<String>> = HashMap::new();

    for entry in entries.iter() {
        if let ParsedStatus::SupersededBy {
            target_legacy,
            folded: true,
        } = &entry.status
        {
            fold_groups
                .entry((entry.feature.clone(), target_legacy.clone()))
                .or_default()
                .push(entry.legacy_id.clone());
        }
    }

    for entry in entries.iter_mut() {
        match &entry.status {
            ParsedStatus::SupersededBy {
                target_legacy,
                folded: true,
            } => {
                entry.kind = EntryKind::Folded {
                    target_legacy: target_legacy.clone(),
                };
            }
            _ => {
                let key = (entry.feature.clone(), entry.legacy_id.clone());
                if let Some(folded) = fold_groups.get(&key) {
                    let mut folded = folded.clone();
                    folded.sort();
                    entry.kind = EntryKind::Rollup {
                        folded_legacy_ids: folded,
                    };
                } else {
                    entry.kind = EntryKind::Standalone;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::adr_migration::parse::ParsedStatus;
    use std::path::PathBuf;

    fn entry(feature: &str, legacy_id: &str, status: ParsedStatus) -> ParsedAdrEntry {
        ParsedAdrEntry {
            feature: feature.to_string(),
            legacy_id: legacy_id.to_string(),
            title: format!("title-{legacy_id}"),
            status,
            tasks: Vec::new(),
            context: String::new(),
            decision: String::new(),
            consequences: String::new(),
            validation_warnings: Vec::new(),
            source_path: PathBuf::from("docs/design/feature/4_decisions.md"),
            kind: EntryKind::Unknown,
        }
    }

    #[test]
    fn rollup_with_three_folded_children_collects_all_legacy_ids() {
        let mut entries = vec![
            entry("activity-job", "ADR-001", ParsedStatus::Accepted),
            entry(
                "activity-job",
                "ADR-003",
                ParsedStatus::SupersededBy {
                    target_legacy: "ADR-001".to_string(),
                    folded: true,
                },
            ),
            entry(
                "activity-job",
                "ADR-004",
                ParsedStatus::SupersededBy {
                    target_legacy: "ADR-001".to_string(),
                    folded: true,
                },
            ),
            entry(
                "activity-job",
                "ADR-008",
                ParsedStatus::SupersededBy {
                    target_legacy: "ADR-001".to_string(),
                    folded: true,
                },
            ),
        ];
        resolve_rollups(&mut entries);

        match &entries[0].kind {
            EntryKind::Rollup { folded_legacy_ids } => {
                assert_eq!(
                    folded_legacy_ids,
                    &vec![
                        "ADR-003".to_string(),
                        "ADR-004".to_string(),
                        "ADR-008".to_string()
                    ]
                );
            }
            other => panic!("expected Rollup, got {other:?}"),
        }
        for folded in &entries[1..] {
            assert!(
                matches!(folded.kind, EntryKind::Folded { .. }),
                "entry {} should be Folded, got {:?}",
                folded.legacy_id,
                folded.kind,
            );
        }
    }

    #[test]
    fn standalone_when_no_one_folds_into_it() {
        let mut entries = vec![entry("feature", "ADR-001", ParsedStatus::Accepted)];
        resolve_rollups(&mut entries);
        assert_eq!(entries[0].kind, EntryKind::Standalone);
    }

    #[test]
    fn real_supersession_without_folded_marker_is_standalone() {
        let mut entries = vec![entry(
            "auditability",
            "ADR-007",
            ParsedStatus::SupersededBy {
                target_legacy: "ADR-042".to_string(),
                folded: false,
            },
        )];
        resolve_rollups(&mut entries);
        // Real supersedes are still standalone (they're individual decisions
        // replaced by individual decisions, not rollups).
        assert_eq!(entries[0].kind, EntryKind::Standalone);
    }

    #[test]
    fn fold_groups_are_feature_scoped() {
        // Two features both have ADR-001 + folded children. The rollup
        // resolution must not cross feature boundaries.
        let mut entries = vec![
            entry("feature-a", "ADR-001", ParsedStatus::Accepted),
            entry(
                "feature-a",
                "ADR-002",
                ParsedStatus::SupersededBy {
                    target_legacy: "ADR-001".to_string(),
                    folded: true,
                },
            ),
            entry("feature-b", "ADR-001", ParsedStatus::Accepted),
            entry(
                "feature-b",
                "ADR-003",
                ParsedStatus::SupersededBy {
                    target_legacy: "ADR-001".to_string(),
                    folded: true,
                },
            ),
        ];
        resolve_rollups(&mut entries);

        match &entries[0].kind {
            EntryKind::Rollup { folded_legacy_ids } => {
                assert_eq!(folded_legacy_ids, &vec!["ADR-002".to_string()]);
            }
            other => panic!("feature-a/ADR-001 expected Rollup, got {other:?}"),
        }
        match &entries[2].kind {
            EntryKind::Rollup { folded_legacy_ids } => {
                assert_eq!(folded_legacy_ids, &vec!["ADR-003".to_string()]);
            }
            other => panic!("feature-b/ADR-001 expected Rollup, got {other:?}"),
        }
    }
}
