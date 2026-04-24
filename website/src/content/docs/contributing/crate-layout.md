---
title: Crate Layout
description: "The Orbit Rust workspace layout and dependency direction."
sidebar:
  order: 3
---

## Crates

| Crate | Responsibility |
|-------|----------------|
| `orbit-cli` | CLI entrypoint. |
| `orbit-core` | Runtime bootstrap, command handling, config, default assets. |
| `orbit-engine` | Activity and job execution. |
| `orbit-tools` | Built-in tools and external MCP plugins. |
| `orbit-agent` | Agent loop transport and CLI runtimes. |
| `orbit-store` | Store implementations. |
| `orbit-policy` | Policy decisions. |
| `orbit-exec` | Process execution and sandbox helpers. |
| `orbit-knowledge` | Code graph build and query. |
| `orbit-common` | Shared types and utilities. |

## Dependency Direction

Do not add cross-crate dependencies without checking the architecture direction. Lower layers should stay reusable and free of higher-level runtime concerns.
