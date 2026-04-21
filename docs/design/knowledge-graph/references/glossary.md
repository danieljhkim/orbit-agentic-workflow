# Glossary: Knowledge Graph

Lookup table for terms used across the knowledge-graph docs and the `orbit-knowledge` crate. Sorted alphabetically. Cross-references point at [1_overview.md](../1_overview.md), [2_design.md](../2_design.md), and [specs/refs.md](../specs/refs.md).

| Term | Meaning |
|------|---------|
| **Attribution** | The process of mapping commit hunks to leaves and accumulating `task_ids` onto the touched nodes. Runs as the final pipeline stage. See [2_design.md §2.2]. |
| **Blob** | Content-addressed source text for a file or leaf, stored at `.orbit/knowledge/graph/blobs/<hh>/<hash>.txt`. Immutable. |
| **Branch ref** | Mutable JSON file at `.orbit/knowledge/graph/refs/heads/<branch>.json` pointing at the active index for a branch. The only mutable surface in the store. |
| **CodebaseGraphV1** | Top-level serialized graph shape: `{ root_dir_id, dirs, files, leaves }`. Loaded once per query session. |
| **Content-addressed** | Stored under a path derived from the sha256 of the content. Identical content collapses to one file; any change produces a new path. |
| **DirNode** | Directory node. Holds `dir_children`, `file_children`. |
| **FileNode** | Source-file node. Holds `leaf_children`, `extension`, `language`, `source_blob_hash`. |
| **FileDiff / Hunk** | Per-file diff record produced by the history walker. `Hunk` carries old/new start + count in git diff coordinates. |
| **Fingerprint (dirty)** | Hash of `git status --porcelain` output plus path count plus newest mtime. Drives the dirty-refresh debounce. |
| **GraphContextService** | Read-side service over a loaded `CodebaseGraphV1`. Wraps a `GraphNavigator` and adds selector resolution, search, and pack rendering. |
| **GraphNavigator** | Low-level traversal primitive over a loaded graph. |
| **Identity key** | Cross-build stable key used to track a node's lineage across rebuilds. Distinct from `id` (per-snapshot) and `object_hash` (per-content). |
| **Index** | Immutable per-build JSON file at `index/by-id/<root-graph-hash>.json` listing every object in a single graph snapshot. |
| **is_locked** | Base field blocking mutations to a node's body through the working graph. |
| **Leaf** | Generic term for a `LeafNode` — an extracted symbol. At the tool surface this is renamed to "symbol" ([T20260411-0424]). |
| **LeafKind** | Enum: `function`, `method`, `class`, `struct`, `interface`, `trait`, `impl`, `field`, `module`. |
| **LeafNode** | Extracted symbol node. Carries its own `source`, `source_hash`, line range, `input_signature`, `output_signature`, `history`, and `children`. |
| **Lineage lock** | `lineage_locked` flag. Freezes the node's identity so renames and re-identifications across builds are rejected. |
| **Location** | Human-readable path used in selectors. Dirs end with `/`; files use the repo-relative path; leaves use `<file>:<qualified_name>`. |
| **Manifest** | `.orbit/knowledge/manifest.json`. Carries `generated_at` and the pointer to the active build. Consistency hint, not authoritative. |
| **Object** | Serialized node body stored at `.orbit/knowledge/graph/objects/<hh>/<hash>.json`. Immutable. |
| **Object hash** | sha256 of a serialized node. Changes whenever any field on the node changes; drives deduplication. |
| **Pack** | Token-budgeted bundle of selected nodes rendered for prompt consumption via `pack_json`. |
| **Pipeline** | Straight-line build stages: `scan → hash → detect_changes → build_dirs → build_files → build_leaves → persist → manifest → attribute`. |
| **Ref fallback** | Read-side behavior: if the current branch's ref does not exist, fall back to the default branch's ref and emit a stderr warning ([T20260421-0358]). |
| **Refresh lock** | `flock` on `.orbit/knowledge/refresh.lock` single-flighting rebuilds across processes. |
| **RefName** | Validated branch-name string used as a typed ref identifier. |
| **Scan** | First pipeline stage; walks the repo honoring gitignore rules and emits `ctx.file_paths`. |
| **Selector** | Universal addressing primitive for tool inputs. Accepts a location, `location:kind`, or a raw node id. |
| **Source hash** | sha256 of a leaf's source text. Stable across rebuilds when the symbol body is unchanged. |
| **Structural conflict** | Flag on a leaf set when both sides of a merge commit touched that leaf. Informational. |
| **task_ids** | Sorted, deduplicated list of Orbit task IDs attributed to a node. Regex: `\[T\d{8}-\d{4}(?:-\d+)?\]`. Brackets stripped in storage. Introduced in [T20260421-0528]. |
| **TaskGraphScope** | Scope selector for per-task graph operations (workspace-only, global, etc.). |
| **Tree-sitter** | Parser framework underlying every `LanguageExtractor`. |
| **Working graph** | In-memory overlay on a branch snapshot used during an activity to stage edits without touching the persisted store. |
