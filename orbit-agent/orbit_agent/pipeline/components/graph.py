from __future__ import annotations

import ast
import hashlib
import json
import logging
from datetime import datetime, timezone
from pathlib import Path

from orbit_agent.pipeline.context import PipelineContext
from orbit_agent.schemas import (
    CodebaseGraphV1,
    DirNode,
    FileNode,
    LeafHistoryEntry,
    LeafNode,
    SignatureField,
)

from .base import BaseComponent

logger = logging.getLogger(__name__)

EXTENSION_MAP: dict[str, str] = {
    ".py": "python",
    ".rs": "rust",
    ".ts": "typescript",
    ".js": "javascript",
    ".go": "go",
    ".java": "java",
    ".md": "markdown",
    ".yaml": "yaml",
    ".yml": "yaml",
    ".json": "json",
    ".toml": "toml",
}


def _detect_language(file_path: str) -> str:
    ext = Path(file_path).suffix.lower()
    return EXTENSION_MAP.get(ext, ext.lstrip(".") if ext else "unknown")


def _dir_id(path: str) -> str:
    return f"dir:{path}"


def _file_id(path: str) -> str:
    return f"file:{path}"


def _leaf_id(path: str, qualified_name: str, start_line: int | None) -> str:
    suffix = start_line if start_line is not None else "unknown"
    return f"leaf:{path}:{qualified_name}:{suffix}"


def _source_hash(source: str) -> str | None:
    if not source:
        return None
    return hashlib.sha256(source.encode("utf-8")).hexdigest()


def _annotation_to_str(node: ast.AST | None, source: str) -> str | None:
    if node is None:
        return None
    segment = ast.get_source_segment(source, node)
    if segment:
        return segment
    try:
        return ast.unparse(node)
    except Exception:
        return None


def _function_inputs(node: ast.FunctionDef | ast.AsyncFunctionDef, source: str) -> list[SignatureField]:
    items: list[SignatureField] = []

    def add_arg(arg: ast.arg, prefix: str = "") -> None:
        items.append(
            SignatureField(
                name=f"{prefix}{arg.arg}",
                annotation=_annotation_to_str(arg.annotation, source),
            )
        )

    for arg in node.args.posonlyargs:
        add_arg(arg)
    for arg in node.args.args:
        add_arg(arg)
    if node.args.vararg is not None:
        add_arg(node.args.vararg, prefix="*")
    for arg in node.args.kwonlyargs:
        add_arg(arg)
    if node.args.kwarg is not None:
        add_arg(node.args.kwarg, prefix="**")
    return items


def _function_outputs(node: ast.FunctionDef | ast.AsyncFunctionDef, source: str) -> list[SignatureField]:
    annotation = _annotation_to_str(node.returns, source)
    if annotation is None:
        return []
    return [SignatureField(name="return", annotation=annotation)]


def _extract_python_imports(path: str, source: str) -> list[str]:
    try:
        tree = ast.parse(source)
    except SyntaxError as exc:
        logger.warning("Failed to parse Python file for import extraction %s: %s", path, exc)
        return []

    imports: list[str] = []
    for node in tree.body:
        if not isinstance(node, (ast.Import, ast.ImportFrom)):
            continue
        import_source = ast.get_source_segment(source, node)
        if import_source:
            imports.append(import_source)
        elif isinstance(node, ast.Import):
            imports.append("import " + ", ".join(alias.name for alias in node.names))
        else:
            module = "." * node.level + (node.module or "")
            imports.append("from " + module + " import " + ", ".join(alias.name for alias in node.names))
    return imports


def _extract_python_leaves(
    path: str,
    source: str,
    file_id: str,
    file_hash: str | None,
) -> list[LeafNode]:
    try:
        tree = ast.parse(source)
    except SyntaxError as exc:
        logger.warning("Failed to parse Python file for graph extraction %s: %s", path, exc)
        return []

    leaves: list[LeafNode] = []

    def visit(body: list[ast.stmt], parent_id: str, prefix: str = "", inside_class: bool = False) -> list[str]:
        child_ids: list[str] = []

        for node in body:
            if isinstance(node, ast.ClassDef):
                qualified_name = f"{prefix}.{node.name}" if prefix else node.name
                node_source = ast.get_source_segment(source, node) or ""
                leaf = LeafNode(
                    id=_leaf_id(path, qualified_name, getattr(node, "lineno", None)),
                    name=node.name,
                    location=path,
                    language="python",
                    description=ast.get_docstring(node) or "",
                    parent_id=parent_id,
                    kind="class",
                    source=node_source,
                    source_hash=_source_hash(node_source),
                    file_hash_at_capture=file_hash,
                    history=[
                        LeafHistoryEntry(
                            timestamp=datetime.now(timezone.utc),
                            actor="orbit-agent",
                            reason="initial capture",
                            source=node_source,
                            source_hash=_source_hash(node_source),
                            file_hash_at_capture=file_hash,
                        )
                    ],
                    start_line=getattr(node, "lineno", None),
                    end_line=getattr(node, "end_lineno", None),
                )
                leaf.children = visit(node.body, leaf.id, qualified_name, inside_class=True)
                leaves.append(leaf)
                child_ids.append(leaf.id)
            elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
                qualified_name = f"{prefix}.{node.name}" if prefix else node.name
                node_source = ast.get_source_segment(source, node) or ""
                kind = "method" if inside_class else "function"
                leaf = LeafNode(
                    id=_leaf_id(path, qualified_name, getattr(node, "lineno", None)),
                    name=node.name,
                    location=path,
                    language="python",
                    description=ast.get_docstring(node) or "",
                    parent_id=parent_id,
                    kind=kind,
                    source=node_source,
                    source_hash=_source_hash(node_source),
                    file_hash_at_capture=file_hash,
                    history=[
                        LeafHistoryEntry(
                            timestamp=datetime.now(timezone.utc),
                            actor="orbit-agent",
                            reason="initial capture",
                            source=node_source,
                            source_hash=_source_hash(node_source),
                            file_hash_at_capture=file_hash,
                        )
                    ],
                    input_signature=_function_inputs(node, source),
                    output_signature=_function_outputs(node, source),
                    start_line=getattr(node, "lineno", None),
                    end_line=getattr(node, "end_lineno", None),
                )
                leaf.children = visit(node.body, leaf.id, qualified_name, inside_class=False)
                leaves.append(leaf)
                child_ids.append(leaf.id)

        return child_ids

    visit(tree.body, file_id)
    return leaves


class BuildGraphDirsComponent(BaseComponent):
    name = "build_graph_dirs"

    def execute(self, context: PipelineContext) -> PipelineContext:
        logger.info("Building graph directory nodes")
        root_location = "."
        root_id = _dir_id(root_location)

        dir_paths: set[Path] = {Path(".")}
        for file_path in context.file_paths:
            current = file_path.parent
            while True:
                dir_paths.add(current)
                if current == Path("."):
                    break
                current = current.parent

        sorted_dirs = sorted(dir_paths, key=lambda path: (len(path.parts), str(path)))
        dirs: dict[str, DirNode] = {}
        for path in sorted_dirs:
            location = "." if str(path) == "." else str(path)
            parent_id = None if location == "." else _dir_id(str(path.parent))
            dirs[location] = DirNode(
                id=_dir_id(location),
                name=context.repo_path.name if location == "." else path.name,
                location=location,
                language="mixed",
                parent_id=parent_id,
            )

        for location, node in dirs.items():
            if location == ".":
                continue
            parent_location = str(Path(location).parent)
            if parent_location == "":
                parent_location = "."
            dirs[parent_location].dir_children.append(node.id)

        context.codebase_graph = CodebaseGraphV1(root_dir_id=root_id, dirs=list(dirs.values()))
        logger.info("Built %d graph directory node(s)", len(context.codebase_graph.dirs))
        return context


class BuildGraphFilesComponent(BaseComponent):
    name = "build_graph_files"

    def execute(self, context: PipelineContext) -> PipelineContext:
        if context.codebase_graph is None:
            raise ValueError("BuildGraphFilesComponent requires codebase_graph in the pipeline context")

        logger.info("Building graph file nodes")
        dir_index = {node.location: node for node in context.codebase_graph.dirs}
        files: list[FileNode] = []

        for file_path in context.file_paths:
            location = str(file_path)
            parent_location = str(file_path.parent)
            if parent_location == "":
                parent_location = "."
            file_node = FileNode(
                id=_file_id(location),
                name=file_path.name,
                location=location,
                language=_detect_language(location),
                parent_id=_dir_id(parent_location),
                extension=file_path.suffix.lower() or None,
            )
            files.append(file_node)
            dir_index[parent_location].file_children.append(file_node.id)

        context.codebase_graph.files = files
        logger.info("Built %d graph file node(s)", len(files))
        return context


class BuildGraphLeavesComponent(BaseComponent):
    name = "build_graph_leaves"

    def execute(self, context: PipelineContext) -> PipelineContext:
        if context.codebase_graph is None:
            raise ValueError("BuildGraphLeavesComponent requires codebase_graph in the pipeline context")

        logger.info("Building graph leaf nodes")
        file_index = {node.location: node for node in context.codebase_graph.files}
        leaves: list[LeafNode] = []

        for file_path in context.file_paths:
            location = str(file_path)
            if _detect_language(location) != "python":
                continue

            abs_path = context.repo_path / file_path
            try:
                source = abs_path.read_text(encoding="utf-8", errors="replace")
            except OSError as exc:
                logger.warning("Could not read %s for graph leaf extraction: %s", abs_path, exc)
                continue

            file_node = file_index[location]
            file_hash = context.new_hashes.get(location)
            file_node.imports = _extract_python_imports(location, source)
            extracted = _extract_python_leaves(location, source, file_node.id, file_hash)
            file_node.leaf_children.extend([leaf.id for leaf in extracted if leaf.parent_id == file_node.id])
            file_node.exports = [leaf.name for leaf in extracted if leaf.parent_id == file_node.id]
            leaves.extend(extracted)

        context.codebase_graph.leaves = leaves
        logger.info("Built %d graph leaf node(s)", len(leaves))
        return context


class PersistGraphComponent(BaseComponent):
    name = "persist_graph"

    def execute(self, context: PipelineContext) -> PipelineContext:
        if context.codebase_graph is None:
            raise ValueError("PersistGraphComponent requires codebase_graph in the pipeline context")

        logger.info("Persisting codebase graph to %s", context.graph_path)
        context.output_dir.mkdir(parents=True, exist_ok=True)
        context.graph_path.write_text(
            json.dumps(context.codebase_graph.model_dump(mode="json"), indent=2) + "\n"
        )
        return context
