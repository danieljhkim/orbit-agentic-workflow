#!/usr/bin/env python3
"""Check whether design docs are stale relative to the code they reference.

A design doc under `docs/design/` is considered stale if any `crates/...rs`
file it references has a `git log` commit date newer than the doc's declared
`**Last updated:** YYYY-MM-DD` frontmatter field. When the doc has no
`Last updated` field, the doc's own last-commit date is used as a fallback.

Exits 0 if all docs are current; 1 if any are stale (missing references are
reported but do not affect the exit code unless `--include-missing` is set).

Usage:
    scripts/check_design_doc_decay.py
    scripts/check_design_doc_decay.py --warn-only      # always exit 0
    scripts/check_design_doc_decay.py --include-missing
"""

from __future__ import annotations

import argparse
import datetime
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
DOC_GLOB = "docs/design/**/*.md"
REF_PATTERN = re.compile(r"crates/[a-zA-Z0-9_/.\-]+\.rs")
LAST_UPDATED_PATTERN = re.compile(
    r"^\s*\*\*Last updated:\*\*\s*(\d{4}-\d{2}-\d{2})\s*$", re.MULTILINE
)


def git_last_commit_date(path: Path) -> datetime.date | None:
    """Return the date of the last commit touching `path`, or None if untracked."""
    result = subprocess.run(
        ["git", "log", "-1", "--format=%cs", "--", str(path)],
        capture_output=True, text=True, cwd=REPO_ROOT,
    )
    out = result.stdout.strip()
    if result.returncode != 0 or not out:
        return None
    try:
        return datetime.date.fromisoformat(out)
    except ValueError:
        return None


def declared_last_updated(doc: Path) -> datetime.date | None:
    """Parse the `**Last updated:** YYYY-MM-DD` field from the doc, if present."""
    body = doc.read_text(encoding="utf-8", errors="ignore")
    m = LAST_UPDATED_PATTERN.search(body)
    if not m:
        return None
    try:
        return datetime.date.fromisoformat(m.group(1))
    except ValueError:
        return None


def extract_refs(doc: Path) -> set[str]:
    """Extract distinct `crates/...rs` references from the doc body."""
    body = doc.read_text(encoding="utf-8", errors="ignore")
    return {m.group(0) for m in REF_PATTERN.finditer(body)}


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--warn-only", action="store_true",
                        help="exit 0 even if stale docs are found")
    parser.add_argument("--include-missing", action="store_true",
                        help="also fail when docs reference files that no longer exist")
    args = parser.parse_args()

    stale_found = False
    missing_found = False

    for doc in sorted(REPO_ROOT.glob(DOC_GLOB)):
        rel_doc = doc.relative_to(REPO_ROOT)

        declared = declared_last_updated(doc)
        if declared is not None:
            doc_date = declared
            doc_source = "declared"
        else:
            doc_date = git_last_commit_date(doc)
            doc_source = "git"
            if doc_date is None:
                continue  # untracked file with no Last updated field

        refs = extract_refs(doc)
        newer_refs: list[tuple[str, datetime.date]] = []
        missing_refs: list[str] = []
        for ref in sorted(refs):
            ref_path = REPO_ROOT / ref
            if not ref_path.exists():
                missing_refs.append(ref)
                continue
            ref_date = git_last_commit_date(ref_path)
            if ref_date is not None and ref_date > doc_date:
                newer_refs.append((ref, ref_date))

        if newer_refs:
            stale_found = True
            print(f"STALE   {rel_doc}  ({doc_source} {doc_date}) — newer code:")
            for ref, ref_date in newer_refs:
                print(f"          {ref_date}  {ref}")
        if missing_refs:
            missing_found = True
            print(f"MISSING {rel_doc} references files that no longer exist:")
            for ref in missing_refs:
                print(f"          {ref}")

    exit_code = 0
    if stale_found and not args.warn_only:
        exit_code = 1
    if missing_found and args.include_missing and not args.warn_only:
        exit_code = 1
    return exit_code


if __name__ == "__main__":
    sys.exit(main())
