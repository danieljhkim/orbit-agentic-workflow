use std::time::{Duration, Instant};

use orbit_engine::{
    Criterion, CriterionOutcome, VerifierConfig, VerifierResult, verify_checkpoint,
    verify_checkpoint_with_config,
};

#[test]
fn file_and_semantic_criteria_record_pass_and_skip_outcomes()
-> Result<(), Box<dyn std::error::Error>> {
    let workspace = tempfile::tempdir()?;
    std::fs::write(workspace.path().join("notes.txt"), "groundhog ready\n")?;

    let criteria = vec![
        Criterion::FileExists {
            path: "notes.txt".to_string(),
        },
        Criterion::FileContains {
            path: "notes.txt".to_string(),
            pattern: "groundhog".to_string(),
        },
        Criterion::Semantic {
            statement: "The implementation is coherent.".to_string(),
        },
    ];

    let result = verify_checkpoint(&criteria, workspace.path());
    match result {
        VerifierResult::Passed { runs } => {
            assert_eq!(runs.len(), 3);
            assert_eq!(runs[0].outcome, CriterionOutcome::Passed);
            assert_eq!(runs[1].outcome, CriterionOutcome::Passed);
            assert_eq!(runs[2].outcome, CriterionOutcome::SkippedSemantic);
        }
        other => panic!("expected Passed result, got {other:?}"),
    }

    Ok(())
}

#[test]
fn command_criteria_run_in_parallel() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = tempfile::tempdir()?;
    let criteria = vec![
        Criterion::Command {
            command: "sleep 1".to_string(),
            expect_exit: 0,
        },
        Criterion::Command {
            command: "sleep 1".to_string(),
            expect_exit: 0,
        },
    ];

    let started = Instant::now();
    let result = verify_checkpoint(&criteria, workspace.path());
    let elapsed = started.elapsed();

    assert!(
        matches!(result, VerifierResult::Passed { .. }),
        "{result:?}"
    );
    assert!(
        elapsed <= Duration::from_millis(1500),
        "expected parallel execution within 1.5s, got {elapsed:?}"
    );

    Ok(())
}

#[test]
fn command_failure_captures_stderr_in_the_failure_report() -> Result<(), Box<dyn std::error::Error>>
{
    let workspace = tempfile::tempdir()?;
    let criteria = vec![Criterion::Command {
        command: "echo boom >&2; false".to_string(),
        expect_exit: 0,
    }];

    let result = verify_checkpoint(&criteria, workspace.path());
    match result {
        VerifierResult::Failed { report, runs } => {
            assert_eq!(runs.len(), 1);
            assert_eq!(runs[0].outcome, CriterionOutcome::Failed);
            assert!(report.what_happened.contains("command criterion"));
            assert!(report.what_happened.contains("boom"));
            assert!(runs[0].captured_output.contains("stderr:\nboom"));
        }
        other => panic!("expected Failed result, got {other:?}"),
    }

    Ok(())
}

#[test]
fn output_truncation_cap_is_configurable() -> Result<(), Box<dyn std::error::Error>> {
    let workspace = tempfile::tempdir()?;
    let criteria = vec![Criterion::Command {
        command: "printf 'abcdefghijklmnopqrstuvwxyz'; exit 1".to_string(),
        expect_exit: 0,
    }];

    let result = verify_checkpoint_with_config(
        &criteria,
        workspace.path(),
        VerifierConfig {
            output_cap_bytes: 24,
        },
    );

    match result {
        VerifierResult::Failed { report, runs } => {
            assert!(report.what_happened.contains("Captured output:"));
            assert!(runs[0].captured_output.contains("[truncated"));
            assert!(
                !runs[0]
                    .captured_output
                    .contains("abcdefghijklmnopqrstuvwxyz")
            );
        }
        other => panic!("expected Failed result, got {other:?}"),
    }

    Ok(())
}
