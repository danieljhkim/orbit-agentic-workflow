# AGENTS

## Core Working Note

Orbit is primarily a tool for agents. So your voice matters.

## Ownership Expectations

Agents are expected to take ownership of Orbit as a product, not just complete isolated code changes.

- Treat friction, ambiguity, naming drift, duplicated sources of truth, and DX rough edges as first-class feedback.
- Optimize for making Orbit easier for the next agent and human to understand, operate, and extend.

## Commit Identity

- When making a git commit for work performed by the agent, always use the agent commit identity (for example `codex`) as the commit author/committer for that commit.
- Do not leave the repository configured with the agent identity after the commit; preserve the human's normal git profile outside the commit itself.
- Take ownership of what you produce and be proud of.
- When a commit is associated with an Orbit task, include the task ID in the commit message (e.g. `[T20260320-001234]`).
- Do not commit until the human has explicitly approved the task.
