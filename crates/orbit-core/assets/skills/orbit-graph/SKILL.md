---
name: orbit-graph
description: Use when navigating, inspecting, or editing orbit-harnessed codebase via the knowledge graph instead of raw file reads.
---

# Orbit Graph

## Purpose

Use `orbit.graph.*` tools for scoped context and symbol-level edits. Prefer over raw `fs.read` — graph tools return only relevant symbols, imports, and neighbors.

## Command Reference

All graph tool calls go through `orbit tool run`. **If running from a worktree**, pass `--root <original .orbit dir>`.

```bash
# Pack context from selectors
orbit tool run orbit.graph.pack --input '{"selectors": ["file:src/lib.rs", "symbol:src/lib.rs#hello:function"]}'

# Search nodes by name or location
orbit tool run orbit.graph.search --input '{"query": "hello", "type": "symbol", "limit": 10}'

# Show node with lineage, siblings, children, source
orbit tool run orbit.graph.show --input '{"selector": "symbol:src/lib.rs#hello:function"}'

# Edit existing symbol
orbit tool run orbit.graph.write --input '{"selector": "symbol:src/lib.rs#hello:function", "new_source": "pub fn hello() { }", "reason": "updated"}'

# Add new symbol (rejects if exists)
orbit tool run orbit.graph.add --input '{"selector": "symbol:src/lib.rs#greet:function", "source": "pub fn greet() { }", "position": "after:symbol:src/lib.rs#hello:function", "reason": "new helper"}'

# Delete symbol
orbit tool run orbit.graph.delete --input '{"selector": "symbol:src/lib.rs#old_fn:function", "reason": "unused"}'

# Move symbol between files
orbit tool run orbit.graph.move --input '{"selector": "symbol:src/old.rs#helper:function", "target_file": "src/new.rs", "reason": "relocated"}'
```

## Selector Syntax

| Form | Format | Example |
|------|--------|---------|
| Dir | `dir:<path>` | `dir:src/module` |
| File | `file:<path>` | `file:src/lib.rs` |
| Symbol | `symbol:<path>#<name>:<kind>` | `symbol:src/lib.rs#hello:function` |

Symbol kinds: `function`, `method`, `struct`, `trait`, `impl`, `class`, `interface`, `field`, `module`.

## Workflow: Context Gathering

1. Build selectors from `task.context_files`: files become `file:<path>`, named symbols become `symbol:<path>#<name>:<kind>`.
2. Call `orbit.graph.pack` with the selector list.
3. Handle the response:
   - **Success**: Use pack entries as context. Do NOT also `fs.read` the full files.
   - **`knowledge_unavailable`** (check `kind` field): Fall back to `fs.read`. Normal for repos without a built graph.
   - **`unresolved_selectors`**: Fall back to `fs.read` only for those entries. Do NOT fall back globally.
4. Use `orbit.graph.search` to discover related symbols not in context_files. Use `orbit.graph.show` to inspect lineage and neighbors.

## Workflow: Code Mutation

Mutation tools (`write`, `add`, `delete`, `move`) operate on a task-scoped **working graph** that writes changes to disk and tracks edits in a version chain.

- Only `symbol:` selectors accepted (not `file:` or `dir:`)
- Locking is automatic
- Always include `reason` for version chain auditability
- Use `workspace_path` param to target a worktree checkout

## Tool Reference

| Tool | Required Params | Optional Params |
|------|-----------------|-----------------|
| `orbit.graph.pack` | `selectors` (array) | `knowledge_dir` |
| `orbit.graph.search` | `query` | `type`, `prefix`, `limit` |
| `orbit.graph.show` | `selector` | `depth`, `siblings`, `children` |
| `orbit.graph.write` | `selector`, `new_source` | `position`, `reason`, `workspace_path`, `knowledge_dir` |
| `orbit.graph.add` | `selector`, `source` | `position`, `reason`, `workspace_path` |
| `orbit.graph.delete` | `selector` | `reason`, `workspace_path` |
| `orbit.graph.move` | `selector`, `target_file` | `position`, `reason`, `workspace_path` |

## Common Mistakes

| Mistake | Correction |
|---------|------------|
| `orbit graph show ...` | Use `orbit tool run orbit.graph.show --input '{...}'` |
| `file:` selector with `write`/`add`/`delete`/`move` | Mutation tools only accept `symbol:` selectors |
| Falling back to `fs.read` globally when some selectors resolved | Only fall back for `unresolved_selectors` entries |
| Treating `knowledge_unavailable` as fatal | Normal when graph not built; fall back to `fs.read` |
| Omitting `reason` on mutations | Always include for version chain tracking |
| Reading full files after successful pack | Pack entries already contain relevant source |
