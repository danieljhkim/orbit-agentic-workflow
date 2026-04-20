//! Shared leaf crate for the Orbit workspace.
//!
//! The public surface is intentionally split into three namespaces:
//! - [`groundhog`] for Groundhog checkpoint lineage and append-only chronicle
//!   serialization
//! - [`types`] for Orbit domain types, `OrbitError`, IDs, and the v2 schemas
//! - [`utility`] for generic helpers like filesystem, redaction, logging,
//!   and blob storage

pub mod groundhog;
pub mod types;
pub mod utility;
