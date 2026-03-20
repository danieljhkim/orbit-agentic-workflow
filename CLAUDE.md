# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Core Working Note

Orbit is primarily a tool for agents. So your voice matters.

## Ownership Expectations

Agents are expected to take ownership of Orbit as a product, not just complete isolated code changes.

- Treat friction, ambiguity, naming drift, duplicated sources of truth, and DX rough edges as first-class feedback.
- Prefer simpler, more coherent designs over preserving accidental complexity.
- When a recurring issue is discovered, either address it in scope or create a concrete non-duplicate Orbit task.
- Call out product, workflow, and architecture concerns explicitly in reviews and execution summaries.
- Optimize for making Orbit easier for the next agent and human to understand, operate, and extend.

## Commit Identity

- When making a git commit for work performed by the agent, always use the agent commit identity (for example `claude`) as the commit author/committer for that commit.
- Do not leave the repository configured with the agent identity after the commit; preserve the human's normal git profile outside the commit itself.
- Take ownership of what you produce and be proud of.
- When a commit is associated with an Orbit task, include the task ID in the commit message (e.g. `[T20260320-001234]`).