#![deny(clippy::print_stderr, clippy::print_stdout)]
// ORB-00004: legacy domain-schema surfaces still need a focused documentation pass.
#![allow(missing_docs)]
#![allow(
    rustdoc::broken_intra_doc_links,
    rustdoc::invalid_html_tags,
    rustdoc::private_intra_doc_links
)]
//! Shared leaf crate for the Orbit workspace.
//!
//! The public surface is intentionally split into four namespaces:
//! - [`groundhog`] for Groundhog checkpoint lineage and append-only chronicle
//!   serialization
//! - [`migration`] for forward-only schema migrations of YAML artifacts
//! - [`types`] for Orbit domain types, `OrbitError`, IDs, and the v2 schemas
//! - [`utility`] for generic helpers like filesystem, redaction, logging,
//!   and blob storage
//! - [`tracing`] as the shared structured-event facade used by Orbit crates

pub mod groundhog;
pub mod migration;
pub mod types;
pub mod utility;

/// Re-export Orbit's tracing facade for crates that already depend on
/// `orbit-common` and need to emit structured events without expanding their
/// dependency surface.
pub use tracing;
