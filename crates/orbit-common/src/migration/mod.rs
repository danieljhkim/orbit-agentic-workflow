//! Forward-only schema migrations for YAML artifacts.
//!
//! Designed for the steady-state shape of schema evolution: small, mostly
//! additive `n → n+1` transforms within a single artifact lineage. Each
//! artifact kind builds a [`Plan`] once at module load (typically via
//! `OnceLock`) and reuses it on every read.
//!
//! Out of scope by design (see `docs/design/task-artifacts/4_decisions.md`
//! ADR-008): rollback (forward-fix instead), automatic write-back to disk
//! (read-time migration in memory only), and lossy cross-layout projections
//! (those belong in one-shot importers, not this framework).

use std::collections::BTreeMap;

use serde_yaml::Value;

use crate::types::OrbitError;

/// One step in a migration chain. Takes a YAML document at version `n`
/// and returns the same document at version `n + 1`. The step is
/// responsible for updating the `schema_version` field on the returned
/// value; [`Plan::migrate`] enforces this and bails on a regression.
pub type Step = fn(Value) -> Result<Value, OrbitError>;

/// Registered migration plan for a single artifact lineage.
pub struct Plan {
    kind: &'static str,
    target: u32,
    steps: BTreeMap<u32, Step>,
}

impl Plan {
    /// Create an empty plan whose chain terminates at `target`.
    pub fn new(kind: &'static str, target: u32) -> Self {
        Self {
            kind,
            target,
            steps: BTreeMap::new(),
        }
    }

    /// Register a step that takes the document from `from` to `from + 1`.
    /// Panics on duplicate registration or a step that would land at or
    /// past the target — both are programmer errors caught at module
    /// load when plans are typically built inside a `OnceLock`.
    pub fn add_step(mut self, from: u32, step: Step) -> Self {
        assert!(
            from < self.target,
            "{kind}: step from v{from} would land at or past target v{target}",
            kind = self.kind,
            target = self.target,
        );
        let prev = self.steps.insert(from, step);
        assert!(
            prev.is_none(),
            "{kind}: duplicate step registered from v{from}",
            kind = self.kind,
        );
        self
    }

    pub fn kind(&self) -> &'static str {
        self.kind
    }

    pub fn target(&self) -> u32 {
        self.target
    }

    /// Migrate `value` from its current `schema_version` up to `target`.
    ///
    /// Errors with [`OrbitError::Migration`] when:
    /// - the document is not a mapping or lacks `schema_version`;
    /// - the document is newer than `target` (this framework is
    ///   forward-only and won't downgrade);
    /// - a chain link is missing between the current version and target;
    /// - a step fails to advance `schema_version` by exactly one.
    pub fn migrate(&self, mut value: Value) -> Result<Value, OrbitError> {
        let mut current = self.read_version(&value, None)?;

        if current > self.target {
            return Err(OrbitError::Migration(format!(
                "{kind}: schema_version {current} is newer than supported target {target}",
                kind = self.kind,
                target = self.target,
            )));
        }

        while current < self.target {
            let step = self.steps.get(&current).ok_or_else(|| {
                OrbitError::Migration(format!(
                    "{kind}: missing migration step from v{current} (target v{target})",
                    kind = self.kind,
                    target = self.target,
                ))
            })?;

            value = step(value)?;

            let next = self.read_version(&value, Some(current))?;
            let expected = current + 1;
            if next != expected {
                return Err(OrbitError::Migration(format!(
                    "{kind}: step from v{current} produced schema_version {next}, expected {expected}",
                    kind = self.kind,
                )));
            }
            current = next;
        }

        Ok(value)
    }

    fn read_version(&self, value: &Value, after: Option<u32>) -> Result<u32, OrbitError> {
        read_schema_version(value).map_err(|err| {
            let scope = match after {
                Some(from) => format!("{kind}: after step from v{from}", kind = self.kind),
                None => self.kind.to_string(),
            };
            OrbitError::Migration(format!("{scope}: {err}"))
        })
    }
}

/// Read the top-level `schema_version` field from a YAML mapping. Public
/// so artifact-specific code can introspect a document without going
/// through a full migration.
pub fn read_schema_version(value: &Value) -> Result<u32, OrbitError> {
    let mapping = value.as_mapping().ok_or_else(|| {
        OrbitError::Migration("expected YAML mapping at document root".to_string())
    })?;

    let version = mapping
        .get(Value::String("schema_version".to_string()))
        .ok_or_else(|| OrbitError::Migration("missing schema_version field".to_string()))?;

    let raw = version.as_u64().ok_or_else(|| {
        OrbitError::Migration(format!(
            "schema_version must be a non-negative integer (got {version:?})"
        ))
    })?;

    u32::try_from(raw)
        .map_err(|_| OrbitError::Migration(format!("schema_version {raw} exceeds u32 range")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_yaml::Mapping;

    fn doc(version: u64) -> Value {
        let mut map = Mapping::new();
        map.insert(
            Value::String("schema_version".to_string()),
            Value::Number(version.into()),
        );
        Value::Mapping(map)
    }

    fn bump(mut value: Value) -> Result<Value, OrbitError> {
        let map = value.as_mapping_mut().expect("mapping");
        let current = map
            .get(Value::String("schema_version".to_string()))
            .and_then(Value::as_u64)
            .expect("schema_version");
        map.insert(
            Value::String("schema_version".to_string()),
            Value::Number((current + 1).into()),
        );
        Ok(value)
    }

    #[test]
    fn no_op_when_already_at_target() {
        let plan = Plan::new("kind", 3).add_step(1, bump).add_step(2, bump);
        let migrated = plan.migrate(doc(3)).expect("no-op");
        assert_eq!(read_schema_version(&migrated).unwrap(), 3);
    }

    #[test]
    fn applies_chain_in_order() {
        fn add_alpha(mut value: Value) -> Result<Value, OrbitError> {
            let map = value.as_mapping_mut().unwrap();
            map.insert(
                Value::String("alpha".to_string()),
                Value::String("from-v1".to_string()),
            );
            bump(value)
        }
        fn add_beta(mut value: Value) -> Result<Value, OrbitError> {
            let map = value.as_mapping_mut().unwrap();
            map.insert(
                Value::String("beta".to_string()),
                Value::String("from-v2".to_string()),
            );
            bump(value)
        }
        let plan = Plan::new("kind", 3)
            .add_step(1, add_alpha)
            .add_step(2, add_beta);

        let migrated = plan.migrate(doc(1)).expect("chain");
        let map = migrated.as_mapping().unwrap();
        assert_eq!(
            map.get(Value::String("alpha".to_string()))
                .and_then(Value::as_str),
            Some("from-v1")
        );
        assert_eq!(
            map.get(Value::String("beta".to_string()))
                .and_then(Value::as_str),
            Some("from-v2")
        );
        assert_eq!(read_schema_version(&migrated).unwrap(), 3);
    }

    #[test]
    fn rejects_newer_than_target() {
        let plan = Plan::new("kind", 2).add_step(1, bump);
        let err = plan.migrate(doc(5)).expect_err("reject newer");
        let msg = err.to_string();
        assert!(msg.contains("newer than supported target"), "{msg}");
        assert!(msg.contains("kind"), "{msg}");
    }

    #[test]
    fn rejects_missing_chain_link() {
        let plan = Plan::new("kind", 3).add_step(2, bump); // missing 1 -> 2
        let err = plan.migrate(doc(1)).expect_err("missing step");
        assert!(
            err.to_string().contains("missing migration step from v1"),
            "{err}"
        );
    }

    #[test]
    fn rejects_step_that_does_not_bump_version() {
        fn forgetful(value: Value) -> Result<Value, OrbitError> {
            Ok(value)
        }
        let plan = Plan::new("kind", 2).add_step(1, forgetful);
        let err = plan.migrate(doc(1)).expect_err("no bump");
        assert!(
            err.to_string()
                .contains("produced schema_version 1, expected 2"),
            "{err}"
        );
    }

    #[test]
    fn rejects_step_that_overshoots() {
        fn doubler(mut value: Value) -> Result<Value, OrbitError> {
            let map = value.as_mapping_mut().unwrap();
            map.insert(
                Value::String("schema_version".to_string()),
                Value::Number(99u64.into()),
            );
            Ok(value)
        }
        let plan = Plan::new("kind", 3).add_step(1, doubler);
        let err = plan.migrate(doc(1)).expect_err("overshoot");
        assert!(
            err.to_string()
                .contains("produced schema_version 99, expected 2"),
            "{err}"
        );
    }

    #[test]
    fn propagates_step_error() {
        fn fails(_: Value) -> Result<Value, OrbitError> {
            Err(OrbitError::Migration("intentional".to_string()))
        }
        let plan = Plan::new("kind", 2).add_step(1, fails);
        let err = plan.migrate(doc(1)).expect_err("step error");
        assert!(err.to_string().contains("intentional"), "{err}");
    }

    #[test]
    fn rejects_missing_schema_version_field() {
        let plan = Plan::new("kind", 1);
        let mut map = Mapping::new();
        map.insert(
            Value::String("id".to_string()),
            Value::String("ORB-00001".to_string()),
        );
        let err = plan.migrate(Value::Mapping(map)).expect_err("no field");
        assert!(err.to_string().contains("missing schema_version"), "{err}");
    }

    #[test]
    fn rejects_non_mapping_root() {
        let plan = Plan::new("kind", 1);
        let err = plan
            .migrate(Value::String("not a map".to_string()))
            .expect_err("non-mapping");
        assert!(err.to_string().contains("expected YAML mapping"), "{err}");
    }

    #[test]
    #[should_panic(expected = "would land at or past target")]
    fn rejects_step_registered_at_target() {
        let _ = Plan::new("kind", 2).add_step(2, bump);
    }

    #[test]
    #[should_panic(expected = "duplicate step")]
    fn rejects_duplicate_step() {
        let _ = Plan::new("kind", 3).add_step(1, bump).add_step(1, bump);
    }
}
