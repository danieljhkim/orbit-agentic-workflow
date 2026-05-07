# User Interface — Overview

**Status:** Draft
**Owner:** gemini
**Last updated:** 2026-05-07

Orbit UI covers the local dashboard served by `orbit-cli` and the project-facing web surface. It gives operators a dense, legible way to monitor tasks, agents, job runs, telemetry, policy denials, scoreboards, and audit signals without turning the interface into a chat transcript or marketing site.

## 1. Motivation

Agent runs produce more state changes, logs, and diagnostics than a human can read linearly. The UI therefore optimizes for scan density, clear status recognition, and quick drill-downs. Canon Refined keeps the pro-tool feel while using readable sans-serif text, subtle rounding, and restrained semantic color [T20260427-29].

The feature remains intentionally narrow: the current design target is the local operator dashboard plus shared visual language for project-facing pages, not a general component library or hosted product shell [T20260506-20].

## 2. Core Concepts

- **Canon Refined and typography:** Layered dark surfaces, fine borders, compact spacing, and muted status colors; `Inter` carries labels and prose while `JetBrains Mono` carries IDs, metrics, timestamps, code, and logs.
- **Dashboard shell:** The local dashboard lives in `crates/orbit-cli/assets/dashboard/` and is intentionally static HTML/CSS/JavaScript served by the CLI web command.
- **Operational surfaces:** Tasks, live logs, job runs, run detail, audit events, policy denials, diagnostics, and scoreboards are separate views over durable Orbit state rather than bespoke workflow engines.
- **Shared vocabulary:** UI-specific terms are kept in `./references/glossary.md`; generic web terms stay out unless Orbit gives them a narrower dashboard meaning [T20260506-20].
- **Project-facing surfaces:** Static docs and project pages should reuse the same visual grammar without importing runtime-only dashboard assumptions.

## 3. At a Glance

| Concern | File | Task |
|---------|------|------|
| Local dashboard | `crates/orbit-cli/assets/dashboard/` | Runtime tabs, tables, tiles, logs, and diagnostics. |
| Theme rules | `./specs/theme.md` | Canon Refined tokens and visual invariants. |
| Current mechanisms | `./2_design.md` | Layout, telemetry, palette, typography, and known limitations. |
| UI vocabulary | `./references/glossary.md` | Orbit-specific terms used by the design docs. |

## Task References

- [T20260427-29] introduced the Canon Refined UI direction.
- [T20260506-20] added the required references folder and clarified the intentionally minimal UI scope.

> Resolve any task above with `orbit task show <ID>` or `git log --grep=<ID>`.
