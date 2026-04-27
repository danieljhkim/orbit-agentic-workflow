//! Shared leaf crate for the Orbit workspace.
//!
//! The public surface is intentionally split into three namespaces:
//! - [`groundhog`] for Groundhog checkpoint lineage and append-only chronicle
//!   serialization
//! - [`types`] for Orbit domain types, `OrbitError`, IDs, and the v2 schemas
//! - [`utility`] for generic helpers like filesystem, redaction, logging,
//!   and blob storage
//! - [`tracing`] as the shared structured-event facade used by Orbit crates

pub mod groundhog;
pub mod types;
pub mod utility;

/// Re-export Orbit's tracing facade for crates that already depend on
/// `orbit-common` and need to emit structured events without expanding their
/// dependency surface.
pub use tracing;
