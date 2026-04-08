import json
from pathlib import Path
from unittest.mock import MagicMock

import pytest

from orbit_map.graph.store import GraphObjectStore
from orbit_map.schemas.graph.nodes import CodebaseGraphV1, DirNode, FileNode
from orbit_map.service.freshness import compute_freshness_report, file_freshness_status
from orbit_map.service.lineage_pack import _generate_overview, render_lineage_pack


@pytest.fixture
def mock_service():
    service = MagicMock()
    # Mock navigator
    service.navigator = MagicMock()

    # Helper to create mock nodes
    def create_node(node_type, location, node_id=None):
        node = MagicMock()
        node.node_type = node_type
        node.location = location
        node.id = node_id or f"{node_type}:{location}"
        return node

    service.create_node = create_node
    service.selector_for_node.side_effect = lambda n: f"{n.node_type}:{n.location}"

    return service


def _build_file_graph(
    location: str, *, source_blob_hash: str | None = None
) -> CodebaseGraphV1:
    root = DirNode(
        id="dir-src",
        name="src",
        location="src",
        language="python",
        dir_children=[],
        file_children=["file-example"],
    )
    file_node = FileNode(
        id="file-example",
        name="example.py",
        location=location,
        language="python",
        parent_id=root.id,
        extension=".py",
        source_blob_hash=source_blob_hash,
    )
    return CodebaseGraphV1(
        root_dir_id=root.id,
        dirs=[root],
        files=[file_node],
        leaves=[],
    )


def _persist_knowledge_graph(tmp_path: Path, source: str) -> tuple[Path, Path, FileNode]:
    repo_path = tmp_path / "repo"
    knowledge_dir = tmp_path / "knowledge"
    source_path = repo_path / "src" / "example.py"
    source_path.parent.mkdir(parents=True, exist_ok=True)
    source_path.write_text(source, encoding="utf-8")

    graph = _build_file_graph("src/example.py")
    GraphObjectStore(knowledge_dir / "graph").write_graph(graph, repo_path=repo_path)
    return repo_path, knowledge_dir, graph.files[0]


def test_generate_overview_single_file(mock_service):
    file_node = mock_service.create_node("file", "orbit_map/service/graph_context.py")
    mock_service.get_file_context.return_value = MagicMock(
        summary="This file defines GraphContextService.",
        exports=["GraphContextService"],
    )

    selector = "file:orbit_map/service/graph_context.py"
    overview = _generate_overview(mock_service, [(selector, file_node)])

    assert "File: `file:orbit_map/service/graph_context.py`" in overview
    assert "This file defines GraphContextService." in overview
    assert "Key exports: GraphContextService." in overview


def test_generate_overview_directory(mock_service):
    dir_node = mock_service.create_node("dir", "orbit_map/service")
    dir_node.description = "Service layer for graph operations."
    dir_node.file_children = ["file1", "file2"]
    dir_node.dir_children = []

    selector = "dir:orbit_map/service"
    overview = _generate_overview(mock_service, [(selector, dir_node)])

    assert "Directory: `dir:orbit_map/service`" in overview
    assert "Service layer for graph operations." in overview
    assert "Contains 2 files and 0 subdirectories." in overview


def test_generate_overview_multiple_with_shared_ancestor(mock_service):
    ancestor = mock_service.create_node("dir", "orbit_map")
    file1 = mock_service.create_node("file", "orbit_map/a.py")
    file2 = mock_service.create_node("file", "orbit_map/b.py")

    mock_service.navigator.get_lineage.side_effect = [
        [ancestor, file1],
        [ancestor, file2],
    ]

    mock_service.get_file_context.side_effect = [
        MagicMock(summary="Module A", exports=[]),
        MagicMock(summary="Module B", exports=[]),
    ]

    requested_nodes = [
        ("file:orbit_map/a.py", file1),
        ("file:orbit_map/b.py", file2),
    ]

    overview = _generate_overview(mock_service, requested_nodes)

    assert "Selection includes 2 nodes under `dir:orbit_map`:" in overview
    assert "- `file:orbit_map/a.py` (file): Module A" in overview
    assert "- `file:orbit_map/b.py` (file): Module B" in overview


def test_generate_overview_multiple_no_shared_ancestor(mock_service):
    # One file in orbit_map, one in orbit_core
    file1 = mock_service.create_node("file", "orbit_map/a.py")
    file2 = mock_service.create_node("file", "orbit_core/b.py")

    mock_service.navigator.get_lineage.side_effect = [[file1], [file2]]

    mock_service.get_file_context.side_effect = [
        MagicMock(summary="Module A", exports=[]),
        MagicMock(summary="Module B", exports=[]),
    ]

    requested_nodes = [
        ("file:orbit_map/a.py", file1),
        ("file:orbit_core/b.py", file2),
    ]

    overview = _generate_overview(mock_service, requested_nodes)

    assert "Selection includes 2 nodes under root:" in overview
    assert "- `file:orbit_map/a.py` (file): Module A" in overview
    assert "- `file:orbit_core/b.py` (file): Module B" in overview


def test_file_freshness_status_reports_fresh_for_matching_repo_file(tmp_path):
    repo_path, _, file_node = _persist_knowledge_graph(tmp_path, "print('fresh')\n")

    report = compute_freshness_report([file_node], repo_path)

    assert file_freshness_status(file_node, repo_path) == "fresh"
    assert report == {
        "status": "fresh",
        "fresh_count": 1,
        "stale_count": 0,
        "unknown_count": 0,
        "stale_files": [],
        "unknown_files": [],
    }


def test_compute_freshness_report_reports_stale_when_repo_file_changes(tmp_path):
    repo_path, _, file_node = _persist_knowledge_graph(tmp_path, "print('before')\n")

    (repo_path / file_node.location).write_text("print('after')\n", encoding="utf-8")
    report = compute_freshness_report([file_node], repo_path)

    assert file_freshness_status(file_node, repo_path) == "stale"
    assert report["status"] == "stale"
    assert report["stale_files"] == [file_node.location]
    assert report["unknown_files"] == []


def test_compute_freshness_report_reports_unknown_without_repo_path(tmp_path):
    _, _, file_node = _persist_knowledge_graph(tmp_path, "print('unknown')\n")

    report = compute_freshness_report([file_node], None)

    assert file_freshness_status(file_node, None) == "unknown"
    assert report["status"] == "unknown"
    assert report["unknown_files"] == [file_node.location]


def test_compute_freshness_report_reports_unknown_without_source_blob_hash(tmp_path):
    repo_path = tmp_path / "repo"
    source_path = repo_path / "src" / "example.py"
    source_path.parent.mkdir(parents=True, exist_ok=True)
    source_path.write_text("print('no hash')\n", encoding="utf-8")

    file_node = _build_file_graph("src/example.py", source_blob_hash=None).files[0]
    report = compute_freshness_report([file_node], repo_path)

    assert file_freshness_status(file_node, repo_path) == "unknown"
    assert report["status"] == "unknown"
    assert report["unknown_files"] == [file_node.location]


def test_render_lineage_pack_json_includes_freshness(tmp_path):
    repo_path, knowledge_dir, _ = _persist_knowledge_graph(tmp_path, "print('pack')\n")

    report = json.loads(
        render_lineage_pack(
            knowledge_dir,
            ["file:src/example.py"],
            format="json",
            repo_path=repo_path,
        )
    )

    assert "freshness" in report
    assert report["freshness"]["status"] == "fresh"
    assert report["freshness"]["fresh_count"] == 1
