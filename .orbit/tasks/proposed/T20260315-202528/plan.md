# Plan

**Goal:** Add `rejected -> in-progress` as a valid transition.
**Scope:** `orbit-types/src/task.rs` only.
**Risk:** None — additive change, no existing transitions removed.

## Step 1: Write a failing test

In `orbit-types/src/task.rs`, add a `#[cfg(test)]` module (or extend existing one):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejected_can_transition_to_in_progress() {
        assert!(TaskStatus::Rejected.validate_transition(TaskStatus::InProgress).is_ok());
    }

    #[test]
    fn rejected_can_still_transition_to_backlog() {
        assert!(TaskStatus::Rejected.validate_transition(TaskStatus::Backlog).is_ok());
    }
}
```

Run: `cargo test -p orbit-types` — expect first test to fail.

## Step 2: Implement the fix

Change line 84 in `orbit-types/src/task.rs`:

```rust
// before
TaskStatus::Rejected => target == TaskStatus::Backlog,

// after
TaskStatus::Rejected => target == TaskStatus::Backlog || target == TaskStatus::InProgress,
```

## Step 3: Verify

- `cargo test -p orbit-types` — both new tests pass
- `cargo test --workspace` — full suite green
- Manual smoke test: `orbit task update <a rejected task id> --status in-progress`