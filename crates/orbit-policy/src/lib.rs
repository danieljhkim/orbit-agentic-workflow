#![deny(clippy::print_stderr, clippy::print_stdout)]
// ORB-00004: legacy policy surfaces still need a focused documentation pass.
#![allow(missing_docs)]
#![allow(
    rustdoc::broken_intra_doc_links,
    rustdoc::invalid_html_tags,
    rustdoc::private_intra_doc_links
)]
//! Filesystem-profile policy evaluation for Orbit runtime activities.
//!
//! # Role
//! Consumed by `orbit-core` and `orbit-tools` to interpret the fsProfile-only
//! `PolicyDef` schema and answer "may this profile read/modify this path?"
//!
//! # Key exports
//! - [`PolicyEngine`] — wraps a validated [`orbit_common::types::PolicyDef`]
//! - [`FsPolicyEvaluation`] — structured allow/deny outcome with matched rule
//!
//! # Dependency direction
//! `orbit-types` → `orbit-policy` → orbit-core

pub mod decision;
pub mod engine;
mod evaluator;

pub use decision::PolicyDecision;
pub use engine::{FsPolicyEvaluation, PolicyEngine};
