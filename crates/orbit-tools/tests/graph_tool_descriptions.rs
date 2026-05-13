#![allow(missing_docs)]
// ORB-00013: Tests use unwrap/expect to keep fixture setup readable.
#![allow(clippy::expect_used, clippy::unwrap_used)]

use orbit_tools::ToolRegistry;

#[test]
fn graph_tool_descriptions_are_when_to_use_guidance() {
    let mut registry = ToolRegistry::new();
    registry.register_builtins();

    let mut graph_schemas: Vec<_> = registry
        .schemas()
        .into_iter()
        .filter(|schema| schema.name.starts_with("orbit.graph."))
        .collect();
    graph_schemas.sort_by(|left, right| left.name.cmp(&right.name));

    assert!(
        !graph_schemas.is_empty(),
        "expected at least one registered orbit.graph.* tool"
    );

    let violations: Vec<String> = graph_schemas
        .iter()
        .filter_map(|schema| {
            let mut failures = Vec::new();
            if !schema.description.starts_with("Use when ") {
                failures.push("missing `Use when ` prefix");
            }
            if !schema.description.contains("Prefer over grep when ")
                && !schema.description.contains("Use instead of grep when ")
            {
                failures.push("missing grep comparison");
            }

            if failures.is_empty() {
                None
            } else {
                Some(format!("{}: {}", schema.name, failures.join(", ")))
            }
        })
        .collect();

    assert!(
        violations.is_empty(),
        "graph tool description contract violations:\n{}",
        violations.join("\n")
    );
}
