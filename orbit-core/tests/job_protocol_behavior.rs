use orbit_core::job::agent_protocol::{build_invocation, parse_and_validate_response};
use orbit_types::{ExecutionResult, JobRunState, JobTargetType, OrbitError};
use serde_json::json;

fn expected_args() -> Vec<String> {
    vec![
        "run".to_string(),
        "--target-type".to_string(),
        "work".to_string(),
        "--target-id".to_string(),
        "spec-123".to_string(),
        "--mode".to_string(),
        "scheduled".to_string(),
        "--output".to_string(),
        "json".to_string(),
    ]
}

#[test]
fn provider_mapper_supports_claude() {
    let invocation =
        build_invocation("claude", JobTargetType::Work, "spec-123").expect("claude mapper");
    assert_eq!(invocation.program, "claude");
    assert_eq!(invocation.args, expected_args());
}

#[test]
fn provider_mapper_supports_codex() {
    let invocation =
        build_invocation("codex", JobTargetType::Work, "spec-123").expect("codex mapper");
    assert_eq!(invocation.program, "codex");
    assert_eq!(invocation.args, expected_args());
}

#[test]
fn provider_mapper_supports_mock_agent() {
    let invocation =
        build_invocation("mock-agent", JobTargetType::Work, "spec-123").expect("mock-agent mapper");
    assert_eq!(invocation.program, "mock-agent");
    assert_eq!(invocation.args, expected_args());
}

#[test]
fn provider_mapper_uses_binary_basename_for_paths() {
    let invocation = build_invocation("/usr/local/bin/claude", JobTargetType::Work, "spec-123")
        .expect("path-based claude mapper");
    assert_eq!(invocation.program, "/usr/local/bin/claude");
    assert_eq!(invocation.args, expected_args());
}

#[test]
fn provider_mapper_rejects_unsupported_provider() {
    let err = build_invocation("custom-agent", JobTargetType::Work, "spec-123")
        .expect_err("unsupported provider must fail");
    assert!(matches!(
        err,
        OrbitError::UnsupportedAgentProvider(provider) if provider == "custom-agent"
    ));
}

#[test]
fn protocol_parser_accepts_success_envelope() {
    let exec = ExecutionResult {
        success: true,
        stdout: serde_json::to_string(&json!({
            "schemaVersion": 1,
            "status": "success",
            "result": {"ok": true},
            "error": null,
            "durationMs": 55
        }))
        .expect("serialize"),
        stderr: String::new(),
        exit_code: Some(0),
        duration_ms: 55,
        output: None,
    };

    let (envelope, state) = parse_and_validate_response(&exec).expect("valid envelope");
    assert_eq!(state, JobRunState::Success);
    assert_eq!(envelope.status, "success");
}
