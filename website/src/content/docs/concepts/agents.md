---
title: Agents
description: "How Orbit runs coding agents through HTTP and CLI-backed runtimes."
sidebar:
  order: 6
---

## Runtime Paths

Orbit supports agent execution through two backend families:

| Backend | Role |
|---------|------|
| `http` | Programmatic provider communication for multi-turn agent loops. This is the primary path. |
| `cli` | Subprocess-backed provider CLIs retained for experimentation and compatibility. |

`backend: auto` resolves before dispatch. Downstream execution sees a concrete backend.

## Providers

Schema v2 provider values include:

- `claude`
- `codex`
- `gemini`
- `ollama`
- `openai_compat`

HTTP transport support is provider-specific. A provider without a wired HTTP transport must fail structurally instead of silently falling back to CLI.

## Tool Allowlists

Agent-loop activities declare the tool names an agent may call. Empty means no tools are allowed.

```yaml
spec:
  type: agent_loop
  tools:
    - orbit.task.show
    - orbit.graph.search
```

`on_denial` controls whether a denied tool call terminates the loop or returns a structured error for the agent to handle.
