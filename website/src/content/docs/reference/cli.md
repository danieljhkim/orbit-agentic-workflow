---
title: CLI Commands
description: "A compact map of the Orbit CLI command surface."
sidebar:
  order: 2
---

## Environment

| Command | Purpose |
|---------|---------|
| `orbit init` | Initialize the global Orbit root, skills, and skill links. |
| `orbit workspace` | Initialize and manage workspaces. |
| `orbit config` | Show or update Orbit configuration. |

## Operate

| Command | Purpose |
|---------|---------|
| `orbit run job <job_id>` | Run an arbitrary job by ID. |
| `orbit run ship <task_id> ...` | Ship explicitly selected tasks through the PR pipeline by default. |
| `orbit run ship --mode local <task_id> ...` | Run the local-only task path for explicitly selected tasks. |
| `orbit run ship-auto` | Auto-select backlog tasks, print a human-readable status summary, and support structured details with `--json`. |
| `orbit run duel-plan <task_id>` | Run a planning duel for one task. |
| `orbit task` | Create, update, and manage tasks. |
| `orbit task artifact put <task_id> <source_path>` | Store a UTF-8 file under a task's artifacts directory. |

## Observe

| Command | Purpose |
|---------|---------|
| `orbit graph` | Build and query the knowledge graph. |
| `orbit audit` | Query the audit event log. |
| `orbit metrics` | Inspect token, tool-call, and knowledge-pack metrics. |
| `orbit scoreboard` | Generate read-only scoreboard summaries. |
| `orbit run history` | Show recent job runs. Filter to one job with `-j <job_id>`. |
| `orbit run show [run_id]` | Show structured state and step summary for a job run (defaults to latest). |
| `orbit run logs [run_id]` | Print raw stdout/stderr captured for a job run. |
| `orbit run events [run_id]` | Show audit events recorded for a job run. |
| `orbit run trace [run_id]` | Show audit event parent/child trace for a job run. |

## Definitions

| Command | Purpose |
|---------|---------|
| `orbit activity` | View activity definitions. |
| `orbit job` | View and manage job definitions. |
| `orbit tool` | View and manage tools and external MCP plugins. |
| `orbit policy` | View filesystem profile policies and runtime scoping. |
| `orbit executor` | View executors. |

## Services

| Command | Purpose |
|---------|---------|
| `orbit mcp init` / `orbit mcp remove` | Register or unregister MCP client integration for Claude, Codex, and Gemini. |
| `orbit mcp serve` | Serve the safe default MCP tool surface. |
| `orbit web serve` | Serve the Orbit dashboard. |
