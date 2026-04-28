---
name: orbit-graph
description: Use when navigating and inspecting codebase via the knowledge graph instead of raw file reads.
---

# Orbit Graph

Use `orbit.graph.*` as your default way to navigate code. Start with the smallest tool that can answer the question.

## Tool Invocation

Graph **read** tools are available via two surfaces; both accept identical JSON.

- **MCP** (plugin path): `orbit_graph_search`, `orbit_graph_show`, `orbit_graph_pack`, `orbit_graph_callers`, `orbit_graph_refs`, `orbit_graph_implementors`, `orbit_graph_deps`, `orbit_graph_overview`. Call them directly when loaded.
- **CLI**: `orbit tool run orbit.graph.<action> --input '<json>'`.

Mapping rule: `orbit.graph.<action>` ↔ `orbit_graph_<action>`. See the `orbit` skill for the full reference. Do not prefer shell just because the examples below use CLI syntax.

Graph **write** tools (build/update) are CLI-only — not exposed over MCP.

## Default Workflow

1. **Search first** — Use `orbit.graph.search` when the prompt names a symbol, trait, function, type, file, or Orbit task ID. Add `type`, `kind`, `prefix`, `task_id`, and `source_regex` filters when you can. For content-shape questions ("every file/symbol matching pattern X"), see [Source-Regex Enumeration](#source-regex-enumeration) — one call usually answers the whole question.
2. **Inspect the exact selector** — Use `orbit.graph.show` to confirm the definition, source, lines, or lineage of the match you found.
3. **Use one relationship tool only if needed**:
   - `orbit.graph.implementors` for trait or interface implementation questions
   - `orbit.graph.callers` for transitive caller-chain questions
   - `orbit.graph.refs` for usages or cross-file symbol references; it returns `code_refs` by default and fills `doc_refs` / `config_refs` only when you pass `include`
   - `orbit.graph.deps` for crate-level dependency direction
   - `orbit.graph.history` for task-ID attribution by selector; prefer it over manual `git log` scans when the question starts from a file, symbol, or directory
4. **Gather only when needed** — Use `orbit.graph.pack` only for a small set of exact selectors when you need multi-symbol context for synthesis, editing, or review. `file:` selectors return metadata and symbol summaries, not full file source, and leaf bodies stay hidden unless you pass `summary: false`.
5. **Orient only when scope is unclear** — Use `orbit.graph.overview` when the subtree is unfamiliar or the task is architectural. Broad scopes default to `summary`; ask for `format: "full"` only when you need per-file symbol lists.

## Source-Regex Enumeration

For content-shape questions ("every file/symbol matching pattern X"), call `orbit.graph.search` with `source_regex` ONCE on the broadest viable `prefix`. Do NOT iterate per-subdirectory or per-crate — a single call with the right scope returns the complete answer set, including `matched_lines: [{line_number, snippet}]` so you usually do not need a follow-up `show` or `pack`.

Question shapes that fit:

- "every file that re-exports `X`" → `source_regex: "^\\s*pub\\s+use\\s+.*X"`
- "every top-level constant" → `source_regex: "^\\s*pub\\s+const\\s+"`
- "every cross-language `class ... implements IFoo`" → `source_regex: "class\\s+\\w+\\s+implements\\s+IFoo"`

Caveat: regex matches comments and string literals too. If a match looks suspicious, verify it with `show`. Do not refine by re-running with a narrower prefix — refine the regex instead.

```bash
# One call returns every file re-exporting OrbitError
orbit tool run orbit.graph.search --input '{"type":"file","prefix":"crates/","source_regex":"^\\s*pub\\s+use\\s+.*OrbitError"}'
```

## Task-ID Filtering

When the prompt asks what a task touched, or gives an Orbit task ID as the main lookup key, call `orbit.graph.search` with `task_id`. The filter exact-matches node `task_ids`, composes with query/type/kind/prefix/source-regex filters, and accepts numeric Orbit IDs shaped like `TYYYYMMDD-N[-N...]`.

When the prompt asks which tasks touched a specific selector, call `orbit.graph.history` with that selector instead. `history` reads graph attribution and falls back to a git-log scan when the graph is unavailable.

```bash
# Return selectors touched by a task
orbit tool run orbit.graph.search --input '{"task_id":"T20260421-0528","format":"selectors"}'

# Narrow a task lookup to symbols whose name/path also matches Runtime
orbit tool run orbit.graph.search --input '{"task_id":"T20260421-0528","query":"Runtime","type":"symbol"}'

# Return task attribution for one selector
orbit tool run orbit.graph.history --input '{"selector":"symbol:src/lib.rs#hello:function"}'
```

## Stop Rule

If `search + show`, or `search + implementors`, or a single `search` with `source_regex`, already answers the question, stop.

Do not also run `overview`, `refs`, or `pack` unless they add information the task still requires.

If you are about to call `pack` or `show` on each candidate to verify which one matches, stop and reconsider — that is the verification-loop anti-pattern. Either rephrase the question as a `source_regex` enumeration, or use the appropriate relation tool (`callers`, `implementors`, `refs`).

## When `fs.read` Is Acceptable

- Graph returned `knowledge_unavailable`
- Some selectors were `unresolved_selectors` and you only fall back for those entries
- You need a non-code file such as config, YAML, TOML, or markdown and `orbit.graph.search` defaulted to code-first results
- You need a few extra lines around a symbol you already found with graph tools

## Minimal Commands

```bash
# Exact symbol lookup
orbit tool run orbit.graph.search --input '{"query":"hello","type":"symbol","kind":"function","limit":10}'
orbit tool run orbit.graph.show --input '{"selector":"symbol:src/lib.rs#hello:function"}'
orbit tool run orbit.graph.search --input '{"query":"AgentRuntime","include_non_code":true}'
orbit tool run orbit.graph.search --input '{"task_id":"T20260421-0528","format":"selectors"}'

# Trait/interface implementations
orbit tool run orbit.graph.implementors --input '{"trait_selector":"symbol:src/lib.rs#Greeter:trait"}'

# Callers / usages / dependency tracing
orbit tool run orbit.graph.callers --input '{"selector":"symbol:src/lib.rs#hello:function"}'
orbit tool run orbit.graph.refs --input '{"selector":"symbol:src/lib.rs#hello:function"}'
orbit tool run orbit.graph.refs --input '{"selector":"symbol:src/lib.rs#hello:function","include":["all"]}'
orbit tool run orbit.graph.deps --input '{"crate":"orbit-engine"}'
orbit tool run orbit.graph.history --input '{"selector":"symbol:src/lib.rs#hello:function"}'

# Multi-symbol context
orbit tool run orbit.graph.pack --input '{"selectors":["file:src/lib.rs","symbol:src/lib.rs#hello:function"]}'
orbit tool run orbit.graph.pack --input '{"selectors":["symbol:src/lib.rs#hello:function"],"summary":false}'

# High-level subtree shape
orbit tool run orbit.graph.overview --input '{"prefix":"src/module"}'
orbit tool run orbit.graph.overview --input '{"prefix":"src/module","format":"full"}'
```

## Selector Forms

- `dir:<path>`
- `file:<path>`
- `symbol:<path>#<name>:<kind>`

Common symbol kinds: `function`, `method`, `struct`, `trait`, `impl`, `field`, `module`.

## Avoid

- Skipping graph tools and going straight to `fs.read`
- Running `orbit.graph.overview` by default for exact symbol lookups
- Forgetting that `orbit.graph.search` hides doc/config hits unless you ask for `include_non_code`
- Using `orbit.graph.refs` for trait-implementation questions instead of `orbit.graph.implementors`
- Using `orbit.graph.refs` for caller-chain questions instead of `orbit.graph.callers`
- Using `orbit.graph.refs` for crate dependency questions instead of `orbit.graph.deps`
- Using `git log` manually for selector attribution before trying `orbit.graph.history`
- Packing broad directories or many selectors just to explore
- Reading full files after `show` or `pack` already gave the needed context
- Falling back to `fs.read` globally when only some selectors failed
- Iterating `source_regex` per-crate or per-subdirectory when a single broad `prefix` returns the same set in one call
- Using `pack` or `show` to verify each candidate from a `search` result one at a time — rephrase as `source_regex` or use `callers`/`implementors`/`refs` instead
