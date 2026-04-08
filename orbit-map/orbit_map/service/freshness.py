from __future__ import annotations

import hashlib
from pathlib import Path
from typing import Iterable, Literal

from orbit_map.schemas.graph.nodes import FileNode

FreshnessStatus = Literal["fresh", "stale", "unknown"]


def file_freshness_status(
    file_node: FileNode, repo_path: Path | str | None
) -> FreshnessStatus:
    current_hash = _current_source_blob_hash(file_node, repo_path)
    if current_hash is None:
        return "unknown"
    if current_hash == file_node.source_blob_hash:
        return "fresh"
    return "stale"


def compute_freshness_report(
    file_nodes: Iterable[FileNode],
    repo_path: Path | str | None,
) -> dict[str, object]:
    fresh_count = 0
    stale_files: list[str] = []
    unknown_files: list[str] = []

    for file_node in sorted(file_nodes, key=lambda item: item.location):
        status = file_freshness_status(file_node, repo_path)
        if status == "fresh":
            fresh_count += 1
            continue
        if status == "stale":
            stale_files.append(file_node.location)
            continue
        unknown_files.append(file_node.location)

    stale_count = len(stale_files)
    unknown_count = len(unknown_files)
    compared_count = fresh_count + stale_count + unknown_count
    if stale_count:
        status: FreshnessStatus = "stale"
    elif unknown_count or compared_count == 0:
        status = "unknown"
    else:
        status = "fresh"

    return {
        "status": status,
        "fresh_count": fresh_count,
        "stale_count": stale_count,
        "unknown_count": unknown_count,
        "stale_files": stale_files,
        "unknown_files": unknown_files,
    }


def _current_source_blob_hash(
    file_node: FileNode, repo_path: Path | str | None
) -> str | None:
    if repo_path is None or file_node.source_blob_hash is None:
        return None

    source_path = Path(repo_path) / file_node.location
    if not source_path.is_file():
        return None

    try:
        source = source_path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return None
    return hashlib.sha256(source.encode("utf-8")).hexdigest()
