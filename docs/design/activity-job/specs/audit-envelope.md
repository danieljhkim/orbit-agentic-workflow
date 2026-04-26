# Spec: V2 Audit Envelope

Activity / Job runs emit a structured v2 audit tree that describes run, step, activity, and control-flow structure. This tree is append-only JSONL and coexists with the lower-level loop transcript/blob sink rather than replacing it.

## Why This Exists

The lower-level loop audit is rich, but it does not describe job structure on its own. Reviewers need to answer questions like:

- which job run emitted this activity?
- which step retried?
- which branch failed a join?
- which workspace produced this run?

The v2 envelope adds that structure.

## Event Tree

Every event carries:

- `schemaVersion`
- `event_type`
- `event_id`
- timestamp
- `run_id`
- `agent_identity`
- optional `parent_event_id`
- optional `workspace_path`

Common event families are:

- `run.*`
- `step.*`
- `activity.*`
- construct-level events for `parallel`, `fan_out`, and `loop`
- policy/tool denial and CLI invocation events

Loop-engine HTTP and tool-call events remain in the lower-level sink and are related to the envelope tree by parentage and shared run identity.

## Persistence Layout

Envelope events append to:

```text
.orbit/state/audit/v2_loop/<run_id>.jsonl
```

Loop-engine events and blobs continue to use the sibling audit layout under:

```text
.orbit/state/audit/loop/<run_id>.jsonl
.orbit/state/audit/blobs/<hh>/<hash>
```

The v2 writer may also keep an in-memory snapshot for smoke assertions and CLI summaries.

## Invariants

- Envelope writes are append-only, one JSON object per line.
- Disk persistence failure should not crash the run by itself; the in-memory event stream is still load-bearing.
- `workspace_path` is attached when the caller has a meaningful repo identity.
- Parent stacks propagate into worker threads so nested branch/worker events remain traversable.

## Failure Modes

- Audit writer mutex poisoning surfaces as a structured audit failure.
- JSONL persistence can fail independently of in-memory event capture.
- Reviewers may need both the envelope JSONL and the lower-level loop JSONL/blob store to reconstruct a full run.

## Migration Notes

- The envelope is additive. It does not retire or rewrite the existing loop-level audit sink.
- CLI backend events are first-class envelope events, so CLI runs remain visible even when no HTTP transcript exists.
- File-backed runtime traces moved from `.orbit/audit/` to `.orbit/state/audit/` in [T20260426-0519]. Existing `.orbit/audit/` files are legacy local artifacts rather than the current write target.

## Agent Signature

Last revised by `codex`.
