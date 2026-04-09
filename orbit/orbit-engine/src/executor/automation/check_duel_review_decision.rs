//! `check_duel_review_decision` automation.
//!
//! Gates the duel fix loop on the arbiter verdict, NOT the raw reviewer
//! output. This is the key behavioral difference from
//! `check_batch_review_decision` — the duel workflow treats the arbiter as
//! authoritative, so a "nitpicky" or fabricated reviewer comment cannot
//! trigger another fix iteration on its own.
//!
//! The arbiter's output is piped into this step's input via the normal
//! `step_output_for_following_input` mechanism. We read the top-level
//! `decision` field and emit `loop_exit: true` iff it is `APPROVED`.

use orbit_types::{Decision, OrbitError};
use serde_json::{Value, json};

use super::input::required_input_string;

/// Extract the arbiter decision from the current input. The arbiter
/// writes a top-level `decision` string (`APPROVED` or `REQUEST_CHANGES`)
/// and we reject any other value loudly — silently treating an unknown
/// decision as one or the other would corrupt the fix-loop gating logic.
fn parse_decision(input: &Value) -> Result<Decision, OrbitError> {
    let raw = required_input_string(input, "decision")?;
    match raw {
        "APPROVED" => Ok(Decision::Approved),
        "REQUEST_CHANGES" => Ok(Decision::RequestChanges),
        other => Err(OrbitError::Execution(format!(
            "check_duel_review_decision: unexpected arbiter decision '{other}' \
             (expected APPROVED or REQUEST_CHANGES)"
        ))),
    }
}

pub(super) fn check_duel_review_decision(input: &Value) -> Result<Value, OrbitError> {
    let decision = parse_decision(input)?;
    let loop_exit = matches!(decision, Decision::Approved);
    Ok(json!({
        "review_decision": match decision {
            Decision::Approved => "APPROVED",
            Decision::RequestChanges => "REQUEST_CHANGES",
        },
        "loop_exit": loop_exit,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approved_arbiter_decision_sets_loop_exit_true() {
        let input = json!({
            "decision": "APPROVED",
            "per_comment": [],
            "reviewer_score": 4.0,
            "implementer_score": 5.0,
            "blocking_comment_ids": [],
            "task_class_ambiguity": "well_specified"
        });
        let out = check_duel_review_decision(&input).unwrap();
        assert_eq!(out["review_decision"], json!("APPROVED"));
        assert_eq!(out["loop_exit"], json!(true));
    }

    #[test]
    fn request_changes_arbiter_decision_sets_loop_exit_false() {
        let input = json!({
            "decision": "REQUEST_CHANGES",
            "per_comment": [
                { "comment_id": "c1", "verdict": "valid", "severity": "high" }
            ],
            "reviewer_score": 3.0,
            "implementer_score": 2.0,
            "blocking_comment_ids": ["c1"],
            "task_class_ambiguity": null
        });
        let out = check_duel_review_decision(&input).unwrap();
        assert_eq!(out["review_decision"], json!("REQUEST_CHANGES"));
        assert_eq!(out["loop_exit"], json!(false));
    }

    #[test]
    fn missing_decision_is_invalid_input() {
        let input = json!({ "per_comment": [] });
        let err = check_duel_review_decision(&input).unwrap_err();
        assert!(matches!(err, OrbitError::InvalidInput(_)));
    }

    #[test]
    fn unknown_decision_value_is_execution_error() {
        let input = json!({ "decision": "MAYBE" });
        let err = check_duel_review_decision(&input).unwrap_err();
        assert!(matches!(err, OrbitError::Execution(_)));
    }

    #[test]
    fn decision_gate_ignores_raw_reviewer_thread_count() {
        // Even with many reviewer comments, if the arbiter says APPROVED,
        // the loop exits. This is the whole point of the authoritative
        // arbiter: the reviewer cannot trigger another iteration on its own.
        let input = json!({
            "decision": "APPROVED",
            "per_comment": [
                { "comment_id": "c1", "verdict": "nitpick" },
                { "comment_id": "c2", "verdict": "out_of_scope" },
                { "comment_id": "c3", "verdict": "invalid" }
            ],
            "blocking_comment_ids": []
        });
        let out = check_duel_review_decision(&input).unwrap();
        assert_eq!(out["loop_exit"], json!(true));
    }
}
