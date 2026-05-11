use std::fs;
use std::path::PathBuf;

use chrono::Utc;
use orbit_common::types::{Adr, AdrStatus, LegacyValidation, OrbitError};

use super::bundle::{AdrBundle, bundle_to_adr, read_bundle_at, validate_bundle, write_bundle_at};
use super::constants::ADR_SCHEMA_VERSION;
use super::doc::AdrFileDocument;
use super::layout::{
    AdrStateDir, adr_dir, next_adr_id, state_dir_path, validate_adr_id,
};
use super::lock::{acquire_adr_allocation_lock, acquire_adr_lock};
use crate::file::layout::read_child_dirs;

pub(crate) struct AdrFileStore {
    root: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct AdrCreateParams {
    pub title: String,
    pub owner: String,
    pub related_features: Vec<String>,
    pub related_tasks: Vec<String>,
    pub body: String,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct AdrDocumentUpdateParams {
    pub title: Option<String>,
    pub owner: Option<String>,
    pub body: Option<String>,
    pub related_features: Option<Vec<String>>,
    pub related_tasks: Option<Vec<String>>,
    pub supersedes: Option<Vec<String>>,
    /// Double-`Option` so callers can distinguish "leave unchanged" (`None`)
    /// from "clear this field" (`Some(None)`).
    pub superseded_by: Option<Option<String>>,
    pub legacy_ids: Option<Vec<String>>,
    pub validation_warnings: Option<Vec<String>>,
    pub legacy_validation: Option<LegacyValidation>,
}

impl AdrFileStore {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn add_adr(&self, params: AdrCreateParams) -> Result<Adr, OrbitError> {
        if params.title.trim().is_empty() {
            return Err(OrbitError::InvalidInput(
                "ADR title must not be empty".to_string(),
            ));
        }
        if params.owner.trim().is_empty() {
            return Err(OrbitError::InvalidInput(
                "ADR owner must not be empty".to_string(),
            ));
        }

        let _allocation_lock = acquire_adr_allocation_lock(&self.root)?;
        let id = next_adr_id(&self.root)?;
        let now = Utc::now();
        let adr = Adr {
            id: id.clone(),
            title: params.title,
            status: AdrStatus::Proposed,
            owner: params.owner,
            created_at: now,
            accepted_at: None,
            last_updated: now,
            related_features: params.related_features,
            related_tasks: params.related_tasks,
            supersedes: Vec::new(),
            superseded_by: None,
            legacy_ids: Vec::new(),
            validation_warnings: Vec::new(),
            legacy_validation: LegacyValidation::None,
        };
        let bundle = AdrBundle {
            doc: AdrFileDocument {
                schema_version: ADR_SCHEMA_VERSION,
                adr,
            },
            body: params.body,
        };
        validate_bundle(&bundle)?;

        let target_dir = adr_dir(&self.root, AdrStateDir::Proposed, &id);
        write_bundle_at(&target_dir, &bundle)?;

        Ok(bundle_to_adr(bundle))
    }

    pub(crate) fn get_adr(&self, id: &str) -> Result<Option<Adr>, OrbitError> {
        validate_adr_id(id)?;
        let Some((_, dir)) = self.locate_adr(id)? else {
            return Ok(None);
        };
        let bundle = read_bundle_at(&dir)?;
        Ok(Some(bundle_to_adr(bundle)))
    }

    pub(crate) fn list_adrs(&self) -> Result<Vec<Adr>, OrbitError> {
        let mut adrs = Vec::new();
        for state in AdrStateDir::all() {
            let dir = state_dir_path(&self.root, *state);
            if !dir.exists() {
                continue;
            }
            for adr_dir_path in read_child_dirs(&dir)? {
                // Skip directories without an adr.yaml (e.g., partial / scratch dirs).
                let doc_path = adr_dir_path.join(super::constants::ADR_YAML);
                if !doc_path.is_file() {
                    continue;
                }
                let bundle = read_bundle_at(&adr_dir_path)?;
                adrs.push(bundle_to_adr(bundle));
            }
        }
        Ok(adrs)
    }

    pub(crate) fn list_adrs_filtered(
        &self,
        _status: Option<AdrStatus>,
        _owner: Option<&str>,
        _legacy_id: Option<&str>,
    ) -> Result<Vec<Adr>, OrbitError> {
        todo!("wired in Phase 3 SQLite integration")
    }

    pub(crate) fn update_adr_status(
        &self,
        id: &str,
        new_status: AdrStatus,
    ) -> Result<(), OrbitError> {
        validate_adr_id(id)?;
        let _lock = acquire_adr_lock(&self.root, id)?;

        let Some((current_state, current_dir)) = self.locate_adr(id)? else {
            return Err(OrbitError::AdrNotFound(id.to_string()));
        };
        let current_status = current_state.to_status();
        if current_status == new_status {
            return Ok(());
        }
        AdrStatus::validate_transition(current_status, new_status)?;

        let target_state = AdrStateDir::from_status(new_status);
        let target_dir = adr_dir(&self.root, target_state, id);
        if let Some(parent) = target_dir.parent() {
            fs::create_dir_all(parent).map_err(|e| OrbitError::Io(e.to_string()))?;
        }
        fs::rename(&current_dir, &target_dir).map_err(|e| OrbitError::Io(e.to_string()))?;

        let mut bundle = read_bundle_at(&target_dir)?;
        let now = Utc::now();
        bundle.doc.adr.status = new_status;
        bundle.doc.adr.last_updated = now;
        if new_status == AdrStatus::Accepted && bundle.doc.adr.accepted_at.is_none() {
            bundle.doc.adr.accepted_at = Some(now);
        }
        write_bundle_at(&target_dir, &bundle)?;
        Ok(())
    }

    pub(crate) fn update_adr_document(
        &self,
        id: &str,
        fields: &AdrDocumentUpdateParams,
    ) -> Result<(), OrbitError> {
        validate_adr_id(id)?;
        let _lock = acquire_adr_lock(&self.root, id)?;

        let Some((_, current_dir)) = self.locate_adr(id)? else {
            return Err(OrbitError::AdrNotFound(id.to_string()));
        };
        let mut bundle = read_bundle_at(&current_dir)?;

        if let Some(ref title) = fields.title {
            bundle.doc.adr.title = title.clone();
        }
        if let Some(ref owner) = fields.owner {
            bundle.doc.adr.owner = owner.clone();
        }
        if let Some(ref body) = fields.body {
            bundle.body = body.clone();
        }
        if let Some(ref features) = fields.related_features {
            bundle.doc.adr.related_features = features.clone();
        }
        if let Some(ref tasks) = fields.related_tasks {
            bundle.doc.adr.related_tasks = tasks.clone();
        }
        if let Some(ref supersedes) = fields.supersedes {
            bundle.doc.adr.supersedes = supersedes.clone();
        }
        if let Some(ref superseded_by) = fields.superseded_by {
            bundle.doc.adr.superseded_by = superseded_by.clone();
        }
        if let Some(ref legacy_ids) = fields.legacy_ids {
            bundle.doc.adr.legacy_ids = legacy_ids.clone();
        }
        if let Some(ref warnings) = fields.validation_warnings {
            bundle.doc.adr.validation_warnings = warnings.clone();
        }
        if let Some(legacy_validation) = fields.legacy_validation {
            bundle.doc.adr.legacy_validation = legacy_validation;
        }

        bundle.doc.adr.last_updated = Utc::now();
        write_bundle_at(&current_dir, &bundle)?;
        Ok(())
    }

    pub(crate) fn delete_adr(&self, id: &str) -> Result<bool, OrbitError> {
        validate_adr_id(id)?;
        let _lock = acquire_adr_lock(&self.root, id)?;

        let Some((_, dir)) = self.locate_adr(id)? else {
            return Ok(false);
        };
        fs::remove_dir_all(dir).map_err(|e| OrbitError::Io(e.to_string()))?;
        Ok(true)
    }

    pub(crate) fn rebuild_index(&self) -> Result<(), OrbitError> {
        todo!("wired in Phase 3 SQLite integration")
    }

    fn locate_adr(&self, id: &str) -> Result<Option<(AdrStateDir, PathBuf)>, OrbitError> {
        for state in AdrStateDir::all() {
            let dir = adr_dir(&self.root, *state, id);
            if dir.is_dir() {
                // A stray same-named dir without adr.yaml still counts as "located"
                // here so the caller gets a sensible AdrNotFound / corruption error
                // from read_bundle_at.
                return Ok(Some((*state, dir)));
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    fn create_params(title: &str, body: &str) -> AdrCreateParams {
        AdrCreateParams {
            title: title.to_string(),
            owner: "claude".to_string(),
            related_features: Vec::new(),
            related_tasks: Vec::new(),
            body: body.to_string(),
        }
    }

    #[test]
    fn add_adr_then_get_adr_round_trips_content_and_layout() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());

        let adr = store
            .add_adr(create_params("Initial decision", "## Context\nA body."))
            .expect("add adr");

        assert_eq!(adr.id, "ADR-0001");
        assert_eq!(adr.status, AdrStatus::Proposed);
        assert_eq!(adr.title, "Initial decision");

        let dir = tempdir.path().join("proposed").join("ADR-0001");
        assert!(dir.join("adr.yaml").is_file());
        assert!(dir.join("body.md").is_file());

        let loaded = store
            .get_adr("ADR-0001")
            .expect("get adr")
            .expect("adr exists");
        assert_eq!(loaded, adr);
    }

    #[test]
    fn add_adr_twice_allocates_sequential_ids() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());

        let first = store
            .add_adr(create_params("first", "body 1"))
            .expect("add 1");
        let second = store
            .add_adr(create_params("second", "body 2"))
            .expect("add 2");

        assert_eq!(first.id, "ADR-0001");
        assert_eq!(second.id, "ADR-0002");
    }

    #[test]
    fn update_adr_status_proposed_to_accepted_moves_dir_and_sets_accepted_at() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());
        let adr = store
            .add_adr(create_params("Decide", "Body"))
            .expect("add");

        store
            .update_adr_status(&adr.id, AdrStatus::Accepted)
            .expect("accept");

        assert!(
            !tempdir.path().join("proposed").join(&adr.id).exists(),
            "proposed dir must be gone"
        );
        let accepted_dir = tempdir.path().join("accepted").join(&adr.id);
        assert!(accepted_dir.is_dir(), "accepted dir must exist");

        let loaded = store
            .get_adr(&adr.id)
            .expect("get")
            .expect("adr exists");
        assert_eq!(loaded.status, AdrStatus::Accepted);
        assert!(loaded.accepted_at.is_some(), "accepted_at must be set");
        assert!(
            loaded.last_updated >= adr.last_updated,
            "last_updated must advance"
        );
    }

    #[test]
    fn update_adr_status_same_state_is_idempotent_no_op() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());
        let adr = store
            .add_adr(create_params("Decide", "Body"))
            .expect("add");

        store
            .update_adr_status(&adr.id, AdrStatus::Proposed)
            .expect("idempotent same-state");

        let loaded = store
            .get_adr(&adr.id)
            .expect("get")
            .expect("adr exists");
        assert_eq!(loaded.status, AdrStatus::Proposed);
        assert!(loaded.accepted_at.is_none());
    }

    #[test]
    fn update_adr_status_rejects_accepted_to_proposed() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());
        let adr = store
            .add_adr(create_params("Decide", "Body"))
            .expect("add");
        store
            .update_adr_status(&adr.id, AdrStatus::Accepted)
            .expect("accept");

        let err = store
            .update_adr_status(&adr.id, AdrStatus::Proposed)
            .expect_err("accepted -> proposed is rejected");
        assert!(
            matches!(err, OrbitError::AdrInvalidTransition(_)),
            "expected AdrInvalidTransition, got {err:?}"
        );
    }

    #[test]
    fn update_adr_document_updates_title_body_and_bumps_last_updated() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());
        let adr = store
            .add_adr(create_params("Initial", "Initial body"))
            .expect("add");
        let initial_updated = adr.last_updated;

        // Sleep-free freshness check: re-read, compare.
        store
            .update_adr_document(
                &adr.id,
                &AdrDocumentUpdateParams {
                    title: Some("Revised".to_string()),
                    body: Some("Revised body".to_string()),
                    ..Default::default()
                },
            )
            .expect("update");

        let loaded = store
            .get_adr(&adr.id)
            .expect("get")
            .expect("adr exists");
        assert_eq!(loaded.title, "Revised");
        let body = fs::read_to_string(
            tempdir
                .path()
                .join("proposed")
                .join(&adr.id)
                .join("body.md"),
        )
        .expect("read body");
        assert_eq!(body, "Revised body");
        assert!(loaded.last_updated >= initial_updated);
    }

    #[test]
    fn delete_adr_on_proposed_removes_directory_and_returns_true() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());
        let adr = store
            .add_adr(create_params("Doomed", "Bye"))
            .expect("add");

        let removed = store.delete_adr(&adr.id).expect("delete");
        assert!(removed);
        assert!(
            !tempdir.path().join("proposed").join(&adr.id).exists(),
            "directory must be gone"
        );
        assert!(
            store.get_adr(&adr.id).expect("get").is_none(),
            "adr must no longer be found"
        );
    }

    #[test]
    fn delete_adr_missing_returns_false() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());

        let removed = store.delete_adr("ADR-9999").expect("delete missing");
        assert!(!removed);
    }

    #[test]
    fn list_adrs_returns_all_adrs_across_state_dirs() {
        let tempdir = tempdir().expect("tempdir");
        let store = AdrFileStore::new(tempdir.path().to_path_buf());

        let a = store.add_adr(create_params("A", "ba")).expect("a");
        let b = store.add_adr(create_params("B", "bb")).expect("b");
        let c = store.add_adr(create_params("C", "bc")).expect("c");

        store
            .update_adr_status(&b.id, AdrStatus::Accepted)
            .expect("accept b");
        store
            .update_adr_status(&c.id, AdrStatus::Accepted)
            .expect("accept c");
        store
            .update_adr_status(&c.id, AdrStatus::Superseded)
            .expect("supersede c");

        let mut listed = store.list_adrs().expect("list");
        listed.sort_by(|x, y| x.id.cmp(&y.id));

        let ids: Vec<String> = listed.iter().map(|adr| adr.id.clone()).collect();
        assert_eq!(ids, vec![a.id.clone(), b.id.clone(), c.id.clone()]);

        let statuses: Vec<AdrStatus> = listed.iter().map(|adr| adr.status).collect();
        assert_eq!(
            statuses,
            vec![AdrStatus::Proposed, AdrStatus::Accepted, AdrStatus::Superseded]
        );
    }
}
