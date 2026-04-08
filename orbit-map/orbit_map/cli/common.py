from __future__ import annotations

import json
from pathlib import Path
from typing import get_args

import click

from orbit_map.schemas import NodeContextRef
from orbit_map.schemas.graph.nodes import LeafKind
from orbit_map.service import GraphContextService

NODE_TYPE_CHOICES = ("dir", "file", "leaf")
LEAF_KIND_CHOICES = tuple(str(value) for value in get_args(LeafKind))
BUILD_TARGET_CHOICES = ("graph", "knowledge")
BOOTSTRAP_FORMAT_CHOICES = ("markdown", "json")
PACK_FORMAT_CHOICES = ("markdown", "json")
GRAPH_COMPONENT_NAMES = [
    "scan_repo",
    "compute_hashes",
    "build_graph_dirs",
    "build_graph_files",
    "build_graph_leaves",
    "persist_graph",
    "manifest",
    "save_hash_cache",
]
KNOWLEDGE_COMPONENT_NAMES = [
    "summarize_files",
    "manifest",
]
GRAPH_AND_KNOWLEDGE_COMPONENT_NAMES = [
    "scan_repo",
    "compute_hashes",
    "build_graph_dirs",
    "build_graph_files",
    "build_graph_leaves",
    "persist_graph",
    "summarize_files",
    "manifest",
    "save_hash_cache",
]


def resolve_paths(repo: str, output: str) -> tuple[Path, Path]:
    repo_path = Path(repo).resolve()
    output_dir = Path(output) if Path(output).is_absolute() else repo_path / output
    return repo_path, output_dir


def build_component_names(target: str, output_dir: Path) -> list[str]:
    if target == "graph":
        return GRAPH_COMPONENT_NAMES
    if target == "knowledge":
        if (output_dir / "graph" / "refs" / "current.json").exists():
            return KNOWLEDGE_COMPONENT_NAMES
        return GRAPH_AND_KNOWLEDGE_COMPONENT_NAMES
    raise ValueError(f"Unsupported build target: {target}")


def target_label(target: str) -> str:
    return "knowledge" if target == "knowledge" else target


def load_graph_context_service(repo: str, output: str) -> GraphContextService:
    _, output_dir = resolve_paths(repo, output)
    return GraphContextService.from_knowledge_dir(output_dir)


def echo_refs(refs: list[NodeContextRef]) -> None:
    click.echo(json.dumps([ref.model_dump(mode="json") for ref in refs], indent=2))
