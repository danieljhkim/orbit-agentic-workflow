use chrono::{Duration, Utc};
use orbit_types::groundhog::{
    Attempt, CheckpointId, Chronicle, Day, DayOutcome, FailureReport, SideEffect, SideEffectKind,
    ToolCallRecord,
};
use proptest::prelude::*;

fn small_string() -> impl Strategy<Value = String> {
    prop::collection::vec(
        prop_oneof![
            Just('a'),
            Just('b'),
            Just('c'),
            Just(' '),
            Just('-'),
            Just('/'),
            Just('_'),
            Just('é'),
            Just('e'),
            Just('\u{301}'),
            Just('0'),
            Just('1'),
        ],
        1..16,
    )
    .prop_map(|chars| chars.into_iter().collect())
}

fn timestamp() -> impl Strategy<Value = chrono::DateTime<Utc>> {
    (0i64..4_102_444_800i64, 0u32..1_000_000_000u32).prop_map(|(secs, nanos)| {
        chrono::DateTime::<Utc>::from_timestamp(secs, nanos)
            .expect("timestamp range stays within chrono support")
    })
}

fn checkpoint_id() -> impl Strategy<Value = CheckpointId> {
    small_string()
}

fn tool_call_record() -> impl Strategy<Value = ToolCallRecord> {
    (0u32..20, small_string(), 0u64..100_000).prop_map(|(seq, tool_name, result_bytes)| {
        ToolCallRecord {
            seq,
            tool_name,
            result_bytes,
        }
    })
}

fn failure_report() -> impl Strategy<Value = FailureReport> {
    (small_string(), small_string(), small_string()).prop_map(
        |(what_tried, what_happened, next_attempt_plan)| FailureReport {
            what_tried,
            what_happened,
            next_attempt_plan,
        },
    )
}

fn attempt() -> impl Strategy<Value = Attempt> {
    (
        timestamp(),
        0i64..3_600,
        prop::collection::vec(tool_call_record(), 0..4),
        prop::option::of(failure_report()),
        any::<bool>(),
    )
        .prop_map(
            |(started_at, duration_secs, tool_calls, failure_report, workspace_reverted)| Attempt {
                started_at,
                ended_at: started_at + Duration::seconds(duration_secs),
                tool_calls,
                failure_report,
                workspace_reverted,
            },
        )
}

fn side_effect() -> impl Strategy<Value = SideEffect> {
    (
        prop_oneof![
            Just(SideEffectKind::FileWrite),
            Just(SideEffectKind::FileDelete),
            Just(SideEffectKind::GitCommit),
            Just(SideEffectKind::DbMutation),
            Just(SideEffectKind::Other),
        ],
        small_string(),
        any::<bool>(),
    )
        .prop_map(|(kind, target, reversible)| SideEffect {
            kind,
            target,
            reversible,
        })
}

fn day_outcome() -> impl Strategy<Value = DayOutcome> {
    prop_oneof![
        Just(DayOutcome::Success),
        small_string().prop_map(|reason| DayOutcome::Abandoned { reason }),
        checkpoint_id().prop_map(DayOutcome::DeviatedTo),
    ]
}

fn day() -> impl Strategy<Value = Day> {
    (
        checkpoint_id(),
        prop::collection::vec(attempt(), 0..4),
        day_outcome(),
        small_string(),
        prop::collection::vec(side_effect(), 0..4),
        timestamp(),
        0i64..86_400,
    )
        .prop_map(
            |(
                checkpoint_id,
                attempts,
                outcome,
                summary,
                side_effects,
                started_at,
                duration_secs,
            )| Day {
                checkpoint_id,
                attempts,
                outcome,
                summary,
                side_effects,
                started_at,
                ended_at: started_at + Duration::seconds(duration_secs),
            },
        )
}

fn chronicle() -> impl Strategy<Value = Chronicle> {
    (
        small_string(),
        small_string(),
        prop::collection::vec(day(), 1..6),
        prop::collection::vec(checkpoint_id(), 0..4),
    )
        .prop_map(|(task_id, plan_id, days, deviation_stack)| {
            let mut chronicle = Chronicle::new(task_id, plan_id);
            chronicle.days = days;
            chronicle.deviation_stack = deviation_stack;
            chronicle
        })
}

fn chronicle_with_indices() -> impl Strategy<Value = (Chronicle, usize, usize)> {
    chronicle().prop_flat_map(|chronicle| {
        let len = chronicle.days.len();
        (Just(chronicle), 0..len, 0..len).prop_map(|(chronicle, left, right)| {
            let (n, m) = if left <= right {
                (left, right)
            } else {
                (right, left)
            };
            (chronicle, n, m)
        })
    })
}

proptest! {
    #[test]
    fn serialize_at_prefix_invariant_holds((chronicle, n, m) in chronicle_with_indices()) {
        let prefix = chronicle.serialize_at(n).expect("index comes from day range");
        let later = chronicle.serialize_at(m).expect("index comes from day range");

        prop_assert!(prefix.len() <= later.len());
        prop_assert_eq!(&prefix, &later[..prefix.len()]);
    }

    #[test]
    fn cache_bytes_round_trip_prefix((chronicle, _, m) in chronicle_with_indices()) {
        let bytes = chronicle.serialize_at(m).expect("index comes from day range");
        let restored = Chronicle::deserialize_cache_bytes(&bytes)
            .expect("serialized cache bytes should round-trip");

        prop_assert_eq!(restored.days.len(), m + 1);
        prop_assert!(restored.deviation_stack.is_empty());

        let reserialized = restored
            .serialize_at(restored.days.len() - 1)
            .expect("restored cache bytes should serialize");
        prop_assert_eq!(reserialized, bytes);
    }
}
