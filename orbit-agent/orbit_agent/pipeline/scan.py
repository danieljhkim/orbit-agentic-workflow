from __future__ import annotations

from pathlib import Path

SKIP_DIRS: set[str] = {
    "node_modules",
    "__pycache__",
    "target",
    "dist",
    "build",
    ".venv",
    "venv",
    ".egg-info"
}

SKIP_EXTENSIONS: set[str] = {
    ".png",
    ".jpg",
    ".jpeg",
    ".gif",
    ".ico",
    ".woff",
    ".woff2",
    ".ttf",
    ".eot",
    ".pdf",
    ".zip",
    ".tar",
    ".gz",
    ".lock",
    ".exe",
    ".dll",
    ".so",
    ".dylib",
}


def scan_repo(repo_path: Path) -> list[Path]:
    """Walk the repo and return all files that should be indexed as relative paths."""
    results: list[Path] = []

    for path in repo_path.rglob("*"):
        if not path.is_file():
            continue

        rel = path.relative_to(repo_path)

        # Skip hidden directories (any part starting with '.')
        if any(part.startswith(".") for part in rel.parts):
            continue

        # Skip common non-code directories
        if any(part in SKIP_DIRS for part in rel.parts):
            continue

        # Skip binary/non-text extensions
        if path.suffix.lower() in SKIP_EXTENSIONS:
            continue

        results.append(rel)

    results.sort()
    return results
