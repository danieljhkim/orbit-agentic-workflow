use orbit_common::types::{Adr, OrbitError};
use serde::{Deserialize, Serialize};

/// On-disk shape of an ADR record (the contents of `adr.yaml`).
///
/// Wraps an in-memory [`Adr`] with the persisted `schema_version` field so that
/// future schema bumps can migrate older files without changing the in-memory
/// type.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub(super) struct AdrFileDocument {
    pub(super) schema_version: u8,
    #[serde(flatten)]
    pub(super) adr: Adr,
}

pub(super) fn serialize_adr_doc_yaml(doc: &AdrFileDocument) -> Result<String, OrbitError> {
    serde_yaml::to_string(doc).map_err(|e| OrbitError::Store(e.to_string()))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use chrono::Utc;
    use orbit_common::types::{AdrStatus, LegacyValidation};

    use super::super::constants::ADR_SCHEMA_VERSION;
    use super::*;

    fn sample_doc() -> AdrFileDocument {
        let ts = Utc.with_ymd_and_hms(2026, 5, 11, 0, 0, 0).unwrap();
        AdrFileDocument {
            schema_version: ADR_SCHEMA_VERSION,
            adr: Adr {
                id: "ADR-0001".to_string(),
                title: "Test decision".to_string(),
                status: AdrStatus::Proposed,
                owner: "claude".to_string(),
                created_at: ts,
                accepted_at: None,
                last_updated: ts,
                related_features: vec![],
                related_tasks: vec![],
                supersedes: vec![],
                superseded_by: None,
                legacy_ids: vec![],
                validation_warnings: vec![],
                legacy_validation: LegacyValidation::None,
            },
        }
    }

    #[test]
    fn round_trip_through_yaml_preserves_schema_version() {
        let doc = sample_doc();
        let yaml = serialize_adr_doc_yaml(&doc).expect("serialize");
        assert!(
            yaml.contains("schema_version: 1"),
            "yaml should contain schema_version: 1; got:\n{yaml}"
        );
        let back: AdrFileDocument = serde_yaml::from_str(&yaml).expect("deserialize");
        assert_eq!(back, doc);
    }
}
