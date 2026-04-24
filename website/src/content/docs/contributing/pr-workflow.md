---
title: PR Workflow
description: "How to keep Orbit changes scoped, tested, and reviewable."
sidebar:
  order: 4
---

## Scope

Keep changes intentional. Avoid unrelated refactors. Update tests when behavior changes.

When a change touches an owned feature's implementation, update that feature's design docs in the same pull request. Flip affected ADR statuses, update the last-updated date, and add an ADR for non-obvious decisions.

## Checks

Run:

```bash
make fmt
make build
```

Use targeted tests while iterating and full workspace tests before landing risky changes.

## Commits

Use clear commit messages. Agent-authored commits should use the agent commit identity for that commit and should not leave the repository configured with that identity afterward.
