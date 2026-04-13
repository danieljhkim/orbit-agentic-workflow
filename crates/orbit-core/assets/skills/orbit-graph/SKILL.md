---
name: orbit-graph
description: Use when navigating and inspecting an orbit-harnessed codebase via the knowledge graph instead of raw file reads.
---

# Orbit Graph

## Purpose

Use `orbit.graph.*` tools for scoped context and symbol-level inspection. Prefer over raw `fs.read` — graph tools return only relevant symbols, imports, and neighbors.

## Command Reference

All graph tool calls go through `orbit tool run`. **If running from a worktree**, pass `--root <original .orbit dir>`.

```bash
# Pack context from selectors (dir, file, or symbol)
orbit tool run orbit.graph.pack --input '{"selectors": ["dir:src", "file:src/lib.rs", "symbol:src/lib.rs#hello:function"]}'

# Search nodes — omit query to browse all nodes
orbit tool run orbit.graph.search --input '{"query": "hello", "type": "symbol", "kind": "function", "limit": 10}'

# Browse all nodes (no query)
orbit tool run orbit.graph.search --input '{"limit": 30}'

# Overview — aggregate summary of the graph
orbit tool run orbit.graph.overview --input '{"prefix": "crates/orbit-knowledge/src"}'

# Find references — who uses this symbol?
orbit tool run orbit.graph.refs --input '{"selector": "symbol:src/lib.rs#hello:function"}'

# Show node with lineage, siblings, children, source
orbit tool run orbit.graph.show --input '{"selector": "symbol:src/lib.rs#hello:function"}'
```

## Selector Syntax

| Form | Format | Example |
|------|--------|---------|
| Dir | `dir:<path>` | `dir:src/module` |
| File | `file:<path>` | `file:src/lib.rs` |
| Symbol | `symbol:<path>#<name>:<kind>` | `symbol:src/lib.rs#hello:function` |

Symbol kinds: `function`, `method`, `struct`, `trait`, `impl`, `class`, `interface`, `field`, `module`.

## Workflow: Exploration

1. Start with `orbit.graph.overview` to get a high-level picture: node counts, languages, symbol kinds, per-file symbol listings.
2. Use `orbit.graph.search` with no query (browse mode) to list all nodes, or with `kind` filter to find specific types (e.g. all structs).
3. Use `orbit.graph.refs` to find who uses a symbol — returns leaves whose source mentions the symbol name.
4. Use `orbit.graph.show` to inspect a specific node's lineage, siblings, and children.

## Workflow: Context Gathering

1. Build selectors from `task.context_files`: files become `file:<path>`, named symbols become `symbol:<path>#<name>:<kind>`.
2. Call `orbit.graph.pack` with the selector list.
3. Handle the response:
   - **Success**: Use pack entries as context. Do NOT also `fs.read` the full files.
   - **`knowledge_unavailable`** (check `kind` field): Fall back to `fs.read`. Normal for repos without a built graph.
   - **`unresolved_selectors`**: Fall back to `fs.read` only for those entries. Do NOT fall back globally.
4. Dir pack entries include `children` (child file/dir selectors). File pack entries include `symbol_summary` (name/kind/selector for each symbol in the file).

## Tool Reference

| Tool | Required Params | Optional Params |
|------|-----------------|-----------------|
| `orbit.graph.pack` | `selectors` (array) | `knowledge_dir` |
| `orbit.graph.search` | *(none)* | `query`, `type`, `kind`, `prefix`, `limit`, `format` |
| `orbit.graph.overview` | *(none)* | `prefix`, `knowledge_dir` |
| `orbit.graph.refs` | `selector` | `limit`, `knowledge_dir` |
| `orbit.graph.show` | `selector` | `depth`, `siblings`, `children` |

## Search Output Formats

`orbit.graph.search` returns structured results by default:

```json
{
  "total": 5,
  "results": [
    { "selector": "symbol:src/lib.rs#hello:function", "name": "hello", "kind": "function", "file": "src/lib.rs" }
  ]
}
```

Pass `"format": "selectors"` for legacy flat array output.

## Common Mistakes

| Mistake | Correction |
|---------|------------|
| `orbit graph show ...` | Use `orbit tool run orbit.graph.show --input '{...}'` |
| Falling back to `fs.read` globally when some selectors resolved | Only fall back for `unresolved_selectors` entries |
| Treating `knowledge_unavailable` as fatal | Normal when graph not built; fall back to `fs.read` |
| Reading full files after successful pack | Pack entries already contain relevant source |
