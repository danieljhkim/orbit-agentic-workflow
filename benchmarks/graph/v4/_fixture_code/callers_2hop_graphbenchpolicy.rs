//! Synthetic fixture: `callers-2hop-graphbenchpolicy`
//!
//! Tests graph's ability to enumerate transitive callers at depth 2,
//! excluding direct (depth-1) callers.
//!
//! Target symbol: `GraphBenchPolicy::lookup_rule`.
//!
//! Expected answer (3 functions): the level-2 functions below.
//! Excluded from answer: the level-1 functions (direct callers) and the
//! level-2 distractor (does not transitively reach `lookup_rule`).
//!
//! This file is not part of any cargo crate; it lives under
//! `benchmarks/graph/v4/_fixture_code/` and is parsed by the orbit
//! knowledge-graph indexer via the narrow `.orbitignore` negation
//! documented in `benchmarks/graph/v4/METHOD.md`.

pub struct GraphBenchPolicy {
    pub name: String,
    pub rules: Vec<String>,
}

impl GraphBenchPolicy {
    pub fn new(name: String) -> Self {
        Self {
            name,
            rules: Vec::new(),
        }
    }

    /// Target symbol for the callers-2hop fixture.
    pub fn lookup_rule(&self, key: &str) -> Option<String> {
        self.rules.iter().find(|r| r.starts_with(key)).cloned()
    }
}

// =========================================================================
// Level 1 callers — direct invocations of `GraphBenchPolicy::lookup_rule`.
// EXCLUDED from the fixture's expected answer (the prompt asks for 2-hop
// callers, excluding direct callers).
// =========================================================================

pub fn level1_lookup_alpha(policy: &GraphBenchPolicy) -> Option<String> {
    policy.lookup_rule("alpha")
}

pub fn level1_lookup_beta(policy: &GraphBenchPolicy) -> Option<String> {
    policy.lookup_rule("beta")
}

pub fn level1_lookup_gamma(policy: &GraphBenchPolicy) -> Option<String> {
    policy.lookup_rule("gamma")
}

// =========================================================================
// Level 2 callers — functions that call the level-1 callers above.
// These ARE the fixture's expected answer (3 functions).
// =========================================================================

pub fn level2_alpha_consumer(policy: &GraphBenchPolicy) -> Option<String> {
    level1_lookup_alpha(policy)
}

pub fn level2_beta_consumer(policy: &GraphBenchPolicy) -> Option<String> {
    level1_lookup_beta(policy)
}

pub fn level2_combined(policy: &GraphBenchPolicy) -> Option<String> {
    level1_lookup_alpha(policy).or_else(|| level1_lookup_beta(policy))
}

// =========================================================================
// Distractor: a level-2-shaped function that does NOT transitively reach
// `GraphBenchPolicy::lookup_rule`. Reads policy state but never calls lookup_rule
// directly or via a level-1 caller. Must be excluded from the answer.
// =========================================================================

pub fn level2_distractor_no_lookup_rule(policy: &GraphBenchPolicy) -> Option<String> {
    let _name = policy.name.clone();
    None
}
