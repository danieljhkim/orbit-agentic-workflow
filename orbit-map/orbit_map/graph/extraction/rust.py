from __future__ import annotations

# RustGraphExtractor — regex-based, no macro expansion, no nested module resolution.
# Extracts top-level symbols (functions, structs, enums, traits, impl blocks, modules)
# as LeafNodes. Methods inside impl blocks are extracted as children of the impl leaf.
# Known limitations:
#   - Regex-based: may misparse complex generics, macros, or attribute-heavy code.
#   - No cross-module resolution; `mod name { }` inline bodies are not descended.
#   - Brace depth counting is naive with respect to strings/comments containing braces.

import logging
import re

from orbit_map.graph.extraction.base import (
    GraphExtractionInput,
    GraphExtractionResult,
    leaf_id,
    leaf_identity_key,
    leaf_location,
    source_hash,
)
from orbit_map.schemas import LeafNode, SignatureField
from orbit_map.schemas.graph.nodes import LeafKind

logger = logging.getLogger(__name__)

_USE_RE = re.compile(r"^use\s+[^;]+;", re.MULTILINE)

# Top-level item pattern: optional visibility + keyword + name
_ITEM_RE = re.compile(
    r"^(?P<vis>pub(?:\([^)]*\))?\s+)?(?P<kw>fn|struct|enum|trait|impl|mod|type|const|static)\s+(?P<name>\w+)",
    re.MULTILINE,
)

_KIND_MAP: dict[str, LeafKind] = {
    "fn": "function",
    "struct": "struct",
    "enum": "struct",
    "trait": "trait",
    "impl": "impl",
    "mod": "module",
    "type": "struct",
    "const": "field",
    "static": "field",
}

# Methods inside impl blocks (4-space indent)
_METHOD_RE = re.compile(
    r"^    (?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?fn\s+(?P<name>\w+)\s*(?P<sig>[^{;]*)",
    re.MULTILINE,
)


def _find_block_end(source: str, start: int) -> int:
    """Return index after the closing brace of the block starting at `start`."""
    i = start
    limit = min(start + 512, len(source))
    while i < limit and source[i] != "{":
        if source[i] == ";":
            return i + 1
        i += 1

    if i >= limit or source[i] != "{":
        nl = source.find("\n", start)
        return nl + 1 if nl != -1 else len(source)

    depth = 0
    while i < len(source):
        ch = source[i]
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i + 1
        i += 1
    return len(source)


def _line_of(source: str, offset: int) -> int:
    return source[:offset].count("\n") + 1


class RustGraphExtractor:
    language = "rust"

    def extract(self, input_data: GraphExtractionInput) -> GraphExtractionResult:
        source = input_data.source
        path = input_data.path

        imports = [m.group(0) for m in _USE_RE.finditer(source)]
        exports: list[str] = []
        leaves: list[LeafNode] = []
        top_level_ids: list[str] = []

        for m in _ITEM_RE.finditer(source):
            kw = m.group("kw")
            name = m.group("name")
            vis = m.group("vis") or ""
            kind: LeafKind = _KIND_MAP.get(kw, "function")

            item_start = m.start()
            item_end = _find_block_end(source, m.end())
            item_source = source[item_start:item_end].strip()

            lid = leaf_id(path, name, kind)
            location = leaf_location(path, name)

            leaf = LeafNode(
                id=lid,
                identity_key=leaf_identity_key(path, name, kind),
                name=name,
                location=location,
                language="rust",
                description=f"Rust {kw} `{name}`",
                parent_id=input_data.file_id,
                kind=kind,
                source=item_source,
                source_hash=source_hash(item_source),
                file_hash_at_capture=input_data.file_hash,
                start_line=_line_of(source, item_start),
                end_line=_line_of(source, item_end),
            )

            # Extract methods from impl blocks
            if kw == "impl":
                method_ids: list[str] = []
                for mm in _METHOD_RE.finditer(item_source):
                    mname = mm.group("name")
                    msig = mm.group("sig").strip()
                    mstart_in_item = mm.start()
                    mend_in_item = _find_block_end(item_source, mm.end())
                    msource = item_source[mstart_in_item:mend_in_item].strip()
                    mstart_abs = item_start + mstart_in_item
                    mend_abs = item_start + mend_in_item

                    mlid = leaf_id(path, f"{name}::{mname}", "method")
                    method_leaf = LeafNode(
                        id=mlid,
                        identity_key=leaf_identity_key(
                            path, f"{name}::{mname}", "method"
                        ),
                        name=mname,
                        location=leaf_location(path, f"{name}::{mname}"),
                        language="rust",
                        description=f"Method `{mname}` in impl `{name}`",
                        parent_id=lid,
                        kind="method",
                        source=msource,
                        source_hash=source_hash(msource),
                        file_hash_at_capture=input_data.file_hash,
                        start_line=_line_of(source, mstart_abs),
                        end_line=_line_of(source, mend_abs),
                        input_signature=(
                            [SignatureField(name="self", annotation=msig)]
                            if msig
                            else []
                        ),
                    )
                    leaves.append(method_leaf)
                    method_ids.append(mlid)

                leaf.children = method_ids

            leaves.append(leaf)
            top_level_ids.append(lid)

            if vis.strip().startswith("pub"):
                exports.append(name)

        return GraphExtractionResult(
            imports=imports,
            exports=exports,
            leaves=leaves,
            top_level_leaf_ids=top_level_ids,
        )
