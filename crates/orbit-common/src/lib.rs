#![deny(clippy::print_stderr, clippy::print_stdout)]
//! Transitional facade for the Orbit workspace.
//!
//! `orbit-common` previously held both the domain model and generic helpers.
//! Those have moved to two leaf crates:
//!
//! - [`orbit_types`] — domain types, `OrbitError`, IDs, activity/job schemas,
//!   and the Groundhog chronicle module
//! - [`orbit_util`] — generic helpers (filesystem, redaction, logging, blob
//!   storage, git, selectors, path normalization)
//!
//! This crate now exists only as a re-export shim so existing
//! `orbit_common::types::*`, `orbit_common::utility::*`, and
//! `orbit_common::groundhog::*` imports continue to resolve while consumers
//! migrate. See `RETIRE.md` in this crate's directory for the retirement
//! plan.

/// Legacy `orbit_common::types::*` namespace. New code should depend on
/// `orbit-types` directly and import from `orbit_types::*`.
pub mod types {
    pub use orbit_types::*;
    // Re-export the type modules so paths like
    // `orbit_common::types::activity_job::JobV2` still resolve.
    pub use orbit_types::{
        activity, activity_job, actor, agent_pair, audit, audit_event, duel, error, event,
        executor_def, friction, id, invocation, job, metrics, policy_decision, policy_def,
        resource, role, run_state, skill, task, task_plan, tool, tool_input, workspace,
    };
}

/// Legacy `orbit_common::utility::*` namespace. New code should depend on
/// `orbit-util` directly and import from `orbit_util::*`.
///
/// Note: the `OrbitError`-aware helper `redact_sensitive_env_error` now lives
/// in `orbit-types`, but the legacy
/// `orbit_common::utility::redaction::redact_sensitive_env_error` path is
/// preserved by re-exporting it through the [`utility::redaction`] submodule
/// below.
pub mod utility {
    pub use orbit_util::{blob_store, error, fs, git, logging, path, selector};

    /// Legacy `orbit_common::utility::redaction` namespace. Combines the
    /// generic helpers from `orbit_util::redaction` with the
    /// `OrbitError`-aware helper from `orbit_types::error`.
    pub mod redaction {
        pub use orbit_types::error::redact_sensitive_env_error;
        pub use orbit_util::redaction::*;
    }
}

/// Legacy `orbit_common::groundhog::*` namespace.
pub mod groundhog {
    pub use orbit_types::groundhog::*;
}

// Top-level re-exports preserve `orbit_common::OrbitError` etc. patterns that
// some callers may rely on.
pub use orbit_types::*;

/// Re-export Orbit's tracing facade for crates that already depend on
/// `orbit-common`.
pub use tracing;
