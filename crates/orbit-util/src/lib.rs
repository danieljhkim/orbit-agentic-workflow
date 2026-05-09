#![deny(clippy::print_stderr, clippy::print_stdout)]
//! Generic utility helpers for the Orbit workspace.
//!
//! This crate is a leaf with respect to other Orbit crates: it MUST NOT depend
//! on any other `orbit-*` crate. Domain types live in `orbit-types`, which
//! depends on this crate (the only legal direction).
//!
//! # Modules
//! - [`blob_store`] — content-addressed blob storage
//! - [`error`] — `UtilError`: a small error type used by utility helpers that
//!   need to surface failure without taking on a dependency on the domain
//!   error in `orbit-types`. `orbit-types::OrbitError` provides
//!   `From<UtilError>` for ergonomic `?` propagation in callers.
//! - [`fs`] — filesystem helpers
//! - [`git`] — read-only git query helpers
//! - [`logging`] — tracing setup and log routing
//! - [`path`] — path normalization helpers
//! - [`redaction`] — generic secret/env scrubbing
//! - [`selector`] — workspace-relative path selectors
//!
//! # Anti-cycle note
//! This crate intentionally does NOT re-export `tracing`. The shared
//! `tracing` re-export lives in `orbit-types` (alongside the domain model).
//! Re-exporting it here would create the temptation `orbit-util →
//! orbit-types → orbit-util` and silently flip the dependency direction.
//! Callers that want the shared facade should depend on `tracing` directly
//! or via `orbit-types::tracing`.

pub mod blob_store;
pub mod error;
pub mod fs;
pub mod git;
pub mod logging;
pub mod path;
pub mod redaction;
pub mod selector;

pub use error::UtilError;
