from __future__ import annotations

import hashlib
import json
import logging
import time
from datetime import datetime, timezone
from pathlib import Path

from pydantic import ValidationError

from orbit_agent.agent import BaseAgent, get_agent
from orbit_agent.pipeline.context import PipelineContext
from orbit_agent.schemas import (
    FileSummaryAnalysisV1,
    FileSymbolV1,
    FileSummaryV1,
    SourceFileV1,
    SummarizeFilesInputV1,
    SummarizeFilesResponseV1,
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

SUMMARIZE_SYSTEM_PROMPT = """\
You are a code analyzer. Given a source file, produce a JSON object with these fields:
- summary: one-sentence description of what this file does
- symbols: array of {name, kind, signature, description} where kind is one of: function, struct, class, interface, module, method, field, trait, impl
- imports: array of import/dependency strings
- exports: array of exported symbol names

Be precise. Use exact identifiers from the source. Do not hallucinate names.
Respond with ONLY valid JSON, no markdown fencing."""


def _detect_language(file_path: str) -> str:
    ext = Path(file_path).suffix.lower()
    return EXTENSION_MAP.get(ext, ext.lstrip(".") if ext else "unknown")


SYMBOL_KIND_ALIASES = {
    "attribute": "field",
    "constant": "field",
    "dataclass": "class",
    "enum": "class",
    "file": "module",
    "object": "class",
    "package": "module",
    "property": "field",
    "type_alias": "module",
    "variable": "field",
}

VALID_SYMBOL_KINDS = {
    "function",
    "struct",
    "class",
    "interface",
    "module",
    "method",
    "field",
    "trait",
    "impl",
}


def _normalize_symbol_kind(value: object) -> str | None:
    if not isinstance(value, str):
        return None

    normalized = value.strip().lower().replace("-", "_").replace(" ", "_")
    normalized = SYMBOL_KIND_ALIASES.get(normalized, normalized)
    if normalized in VALID_SYMBOL_KINDS:
        return normalized
    return None


def _coerce_string_list(value: object) -> list[str]:
    if not isinstance(value, list):
        return []
    return [item.strip() for item in value if isinstance(item, str) and item.strip()]


def _parse_analysis(raw: str, file_path: str) -> FileSummaryAnalysisV1:
    try:
        return FileSummaryAnalysisV1.model_validate_json(raw)
    except ValidationError as exc:
        logger.info("Strict summary validation failed for %s: %s", file_path, exc)
    except json.JSONDecodeError as exc:
        logger.warning("Summary JSON decode failed for %s: %s", file_path, exc)
        return FileSummaryAnalysisV1()

    try:
        payload = json.loads(raw)
    except json.JSONDecodeError as exc:
        logger.warning("Summary JSON decode failed for %s: %s", file_path, exc)
        return FileSummaryAnalysisV1()

    if not isinstance(payload, dict):
        logger.warning("Summary payload for %s was not an object, using fallback", file_path)
        return FileSummaryAnalysisV1()

    symbols: list[FileSymbolV1] = []
    for item in payload.get("symbols", []):
        if not isinstance(item, dict):
            continue

        kind = _normalize_symbol_kind(item.get("kind"))
        name = item.get("name")
        if kind is None or not isinstance(name, str) or not name.strip():
            continue

        symbols.append(
            FileSymbolV1(
                name=name.strip(),
                kind=kind,
                signature=item.get("signature", "") if isinstance(item.get("signature"), str) else "",
                description=item.get("description", "") if isinstance(item.get("description"), str) else "",
            )
        )

    summary = payload.get("summary")
    return FileSummaryAnalysisV1(
        summary=summary if isinstance(summary, str) and summary.strip() else "Failed to summarize",
        symbols=symbols,
        imports=_coerce_string_list(payload.get("imports")),
        exports=_coerce_string_list(payload.get("exports")),
    )


class SummarizeFilesComponent(BaseComponent):
    name = "summarize_files"

    def __init__(self, agent: BaseAgent | None = None, delay: float = 0.1) -> None:
        self.agent = agent
        self.delay = delay

    def _read(self, file_paths: list[str], repo_path: Path) -> SummarizeFilesInputV1:
        total = len(file_paths)
        files: list[SourceFileV1] = []

        for i, fp in enumerate(file_paths, 1):
            abs_path = repo_path / fp
            relative_path = str(Path(fp))

            logger.info("Reading file %d of %d for summarization: %s", i, total, relative_path)

            try:
                content_bytes = abs_path.read_bytes()
            except OSError:
                logger.warning("Could not read file: %s", abs_path)
                continue

            files.append(
                SourceFileV1(
                    path=relative_path,
                    hash=hashlib.sha256(content_bytes).hexdigest(),
                    language=_detect_language(fp),
                    content=content_bytes.decode("utf-8", errors="replace"),
                    metadata={
                        "size_bytes": len(content_bytes),
                        "last_modified": datetime.fromtimestamp(abs_path.stat().st_mtime, tz=timezone.utc),
                    },
                )
            )

        return SummarizeFilesInputV1(files=files)

    def _transform(self, data: SummarizeFilesInputV1) -> SummarizeFilesResponseV1:
        agent = self.agent or get_agent()
        total = len(data.files)
        results: list[FileSummaryV1] = []

        for i, source_file in enumerate(data.files, 1):
            logger.info("Summarizing file %d of %d: %s", i, total, source_file.path)
            user_message = f"File: {source_file.path}\n\n{source_file.content}"

            try:
                raw = agent.chat(SUMMARIZE_SYSTEM_PROMPT, user_message)
                analysis = _parse_analysis(raw, source_file.path)
            except Exception:
                logger.warning("LLM response parse failed for %s, using fallback", source_file.path)
                analysis = FileSummaryAnalysisV1()

            results.append(
                FileSummaryV1(
                    path=source_file.path,
                    hash=source_file.hash,
                    language=source_file.language,
                    summary=analysis.summary,
                    symbols=analysis.symbols,
                    imports=analysis.imports,
                    exports=analysis.exports,
                    metadata=source_file.metadata,
                )
            )

            if i < total:
                time.sleep(self.delay)

        return SummarizeFilesResponseV1(files=results)

    def _write(self, response: SummarizeFilesResponseV1, output_dir: Path) -> None:
        output_dir.mkdir(parents=True, exist_ok=True)

        for summary in response.files:
            summary_path = output_dir / f"{summary.hash}.json"
            summary_path.write_text(json.dumps(summary.model_dump(mode="json"), indent=2) + "\n")

    def execute(self, context: PipelineContext) -> PipelineContext:
        if self.agent is None:
            self.agent = context.agent or get_agent()
        data = self._read(context.changed_paths, context.repo_path)
        response = self._transform(data)
        self._write(response, context.files_dir)
        context.summarize_response = response
        return context
