# User Interface — Overview

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-04-26

The Orbit User Interface encompasses both the local dashboard (surfaced via `orbit-cli`) and the external-facing project website. It provides a cohesive, high-density visual surface for monitoring autonomous agents, executing workflows, and interpreting system telemetry.

## 1. Motivation

Autonomous agents generate massive amounts of telemetry, state transitions, and diagnostic logs at speeds far exceeding human reading capacity. We need a UI paradigm that prioritizes density, utilitarian function, and immediate visual status recognition without sacrificing readability. The "Canon Refined" aesthetic balances high-density data presentation with modern web affordances (sans-serif readability, subtle rounding, soft semantic colors) to create a highly functional "pro-tool" tailored for technical workflows.

## 2. Core Concepts

- **Canon Refined Aesthetic**: A modern, high-density design language emphasizing a layered dark mode (`#0a0a0a` base), subtle structural borders, and muted semantic colors to surface state at a glance.
- **Dual Typography**: `Inter` (sans-serif) for prose and UI structure, paired with `JetBrains Mono` strictly for data points, IDs, and logs.
- **Local Dashboard**: The live telemetry view served locally during active agent execution, accessible via `orbit-cli`.
- **Project Website**: The public face of the project, adapting the core terminal aesthetic to static documentation, benchmark reports, and marketing pages.

## 3. At a Glance

| Concern | File/Folder | What it is |
|---------|-------------|------------|
| Local Dashboard UI | `crates/orbit-cli/assets/dashboard/` | Core HTML/CSS assets for the local runtime dashboard. |
| UI Theme Definition | `specs/theme.md` | Primary source of truth for color palette, typography, and visual rules. |
| Core Styling | `2_design.md` | Details the implementation of the design principles. |

## Task References

- [T20260427-29]

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
