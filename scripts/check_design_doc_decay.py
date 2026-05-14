#!/usr/bin/env python3
"""Compatibility wrapper for `orbit design check`."""

from __future__ import annotations

import os
import subprocess
import sys
from pathlib import Path


def orbit_command(repo_root: Path) -> list[str]:
    override = os.environ.get("ORBIT_BIN", "").strip()
    if override:
        return [override]

    return ["cargo", "run", "--quiet", "-p", "orbit-cli", "--bin", "orbit", "--"]


def main() -> int:
    repo_root = Path(__file__).resolve().parent.parent
    command = [*orbit_command(repo_root), "design", "check", *sys.argv[1:]]
    return subprocess.run(command, cwd=repo_root).returncode


if __name__ == "__main__":
    sys.exit(main())
