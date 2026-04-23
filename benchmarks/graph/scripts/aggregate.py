"""Sweep aggregator.

Reads benchmark records under `benchmarks/graph/runs/` and emits two
markdown tables to stdout: the primary `(provider, arm, task_class)`
headline table and the secondary `(provider, model, arm, task_class)`
per-model breakdown.
"""

from __future__ import annotations

import argparse
import json
import statistics
import sys
from collections import defaultdict
from dataclasses import dataclass
from pathlib import Path

import yaml

BENCH_ROOT = Path(__file__).resolve().parents[1]
ARMS = {"no-graph", "graph-only", "hybrid"}
PROVIDERS = {"claude", "codex"}
CLAUDE_SHELL_OR_FS_TOOLS = {"Bash", "Glob", "Grep", "Read"}
GRAPH_TOOL_PREFIXES = ("mcp__orbit-bench__orbit_graph_", "orbit.graph.")


@dataclass(frozen=True)
class ToolUtilization:
    graph_calls: int = 0
    shell_or_fs_calls: int = 0
    other_calls: int = 0


def _parse_jsonl_stream(raw: str) -> list[dict]:
    events = []
    for line in raw.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(event, dict):
            events.append(event)
    return events


def _is_graph_tool_name(name: str) -> bool:
    return any(name.startswith(prefix) for prefix in GRAPH_TOOL_PREFIXES)


def _load_transcript_events(transcript_path: Path) -> list[dict] | None:
    if not transcript_path.exists():
        return None
    return _parse_jsonl_stream(transcript_path.read_text())


def _classify_claude_transcript(events: list[dict]) -> ToolUtilization:
    graph_calls = 0
    shell_or_fs_calls = 0
    other_calls = 0
    for event in events:
        if event.get("type") != "assistant":
            continue
        message = event.get("message", {}) or {}
        for block in message.get("content", []) or []:
            if block.get("type") != "tool_use":
                continue
            name = block.get("name")
            if not isinstance(name, str) or not name:
                other_calls += 1
            elif _is_graph_tool_name(name):
                graph_calls += 1
            elif name in CLAUDE_SHELL_OR_FS_TOOLS:
                shell_or_fs_calls += 1
            else:
                other_calls += 1
    return ToolUtilization(
        graph_calls=graph_calls,
        shell_or_fs_calls=shell_or_fs_calls,
        other_calls=other_calls,
    )


def _classify_codex_transcript(events: list[dict]) -> ToolUtilization:
    graph_calls = 0
    shell_or_fs_calls = 0
    other_calls = 0
    for event in events:
        if event.get("type") != "item.completed":
            continue
        item = event.get("item", {}) or {}
        item_type = item.get("type")
        if item_type == "command_execution":
            shell_or_fs_calls += 1
            continue
        if item_type != "mcp_tool_call":
            continue
        name = item.get("tool") or item.get("name")
        if not isinstance(name, str) or not name:
            other_calls += 1
        elif _is_graph_tool_name(name):
            graph_calls += 1
        else:
            other_calls += 1
    return ToolUtilization(
        graph_calls=graph_calls,
        shell_or_fs_calls=shell_or_fs_calls,
        other_calls=other_calls,
    )


def _classify_transcript(provider: str, transcript_path: Path) -> ToolUtilization | None:
    events = _load_transcript_events(transcript_path)
    if events is None:
        return None
    if provider == "claude":
        return _classify_claude_transcript(events)
    if provider == "codex":
        return _classify_codex_transcript(events)
    return ToolUtilization()


def _format_graph_call_rate(graph_runs: int, total_runs: int) -> str:
    return f"{graph_runs}/{total_runs} = {graph_runs / total_runs:.1%}"


def _format_tool_utilization(cell_runs: list[dict]) -> tuple[str | int, str, str | int]:
    utilization = [r.get("_tool_utilization") for r in cell_runs]
    if any(stats is None for stats in utilization):
        return ("-", "N/A", "-")

    resolved = [stats for stats in utilization if stats is not None]
    graph_calls = sum(stats.graph_calls for stats in resolved)
    shell_or_fs_calls = sum(stats.shell_or_fs_calls for stats in resolved)
    graph_runs = sum(1 for stats in resolved if stats.graph_calls > 0)
    return (
        graph_calls,
        _format_graph_call_rate(graph_runs, len(resolved)),
        shell_or_fs_calls,
    )


def _total_other_calls(runs: list[dict]) -> int:
    total = 0
    for record in runs:
        stats = record.get("_tool_utilization")
        if stats is None:
            continue
        total += stats.other_calls
    return total


def _fixture_map(tasks_dir: Path) -> dict[str, dict]:
    fixtures = {}
    for p in tasks_dir.glob("*.yaml"):
        if p.stem.startswith("_"):
            continue
        fx = yaml.safe_load(p.read_text())
        fixtures[fx["task_id"]] = fx
    return fixtures


def _iter_arm_dirs(runs_dir: Path):
    for provider_dir in runs_dir.iterdir():
        if not provider_dir.is_dir() or provider_dir.name.startswith("_"):
            continue
        if provider_dir.name not in PROVIDERS:
            continue
        for arm_dir in provider_dir.iterdir():
            if arm_dir.is_dir() and arm_dir.name in ARMS:
                yield (provider_dir.name, arm_dir)


def load_runs(runs_dir: Path, tasks_dir: Path) -> list[dict]:
    fixtures = _fixture_map(tasks_dir)
    out = []
    for provider, arm_dir in _iter_arm_dirs(runs_dir):
        for task_dir in arm_dir.iterdir():
            if not task_dir.is_dir():
                continue
            for run_path in task_dir.glob("*.json"):
                if run_path.name.endswith(".transcript.json"):
                    continue
                try:
                    record = json.loads(run_path.read_text())
                except json.JSONDecodeError:
                    continue
                if not isinstance(record, dict) or "verdict" not in record:
                    continue
                record["provider"] = record.get("provider", provider)
                record["arm"] = record.get("arm", arm_dir.name)
                task_id = record.get("task_id", task_dir.name)
                fx = fixtures.get(task_id, {})
                record["_task_class"] = fx.get("class", "unknown")
                transcript_path = run_path.with_name(f"{run_path.stem}.transcript.json")
                record["_tool_utilization"] = _classify_transcript(
                    record["provider"],
                    transcript_path,
                )
                out.append(record)
    return out


def primary_table(runs: list[dict]) -> str:
    cells: dict[tuple[str, str, str], list[dict]] = defaultdict(list)
    for r in runs:
        if r["verdict"] == "error":
            continue
        cells[(r["provider"], r["arm"], r["_task_class"])].append(r)

    rows = []
    for (provider, arm, cls), cell_runs in sorted(cells.items()):
        # `input_tokens + output_tokens` is the marginal (uncached) token
        # spend for the run, provider-comparable by convention — see the
        # module docstring in scripts/providers.py.
        totals = [r["input_tokens"] + r["output_tokens"] for r in cell_runs]
        passes = sum(1 for r in cell_runs if r["verdict"] == "pass")
        tps = (sum(totals) / max(1, passes)) if passes else float("inf")
        graph_calls, graph_call_rate, shell_or_fs_calls = _format_tool_utilization(cell_runs)
        rows.append(
            {
                "provider": provider,
                "arm": arm,
                "task_class": cls,
                "runs": len(cell_runs),
                "pass_rate": f"{passes / max(1, len(cell_runs)):.0%}",
                "median_total_tokens": int(statistics.median(totals)) if totals else 0,
                "p90_total_tokens": (
                    int(statistics.quantiles(totals, n=10)[-1])
                    if len(totals) >= 10
                    else (max(totals) if totals else 0)
                ),
                "tokens_per_success": f"{tps:.0f}" if tps != float("inf") else "∞",
                "graph_calls": graph_calls,
                "graph_call_rate": graph_call_rate,
                "shell_or_fs_calls": shell_or_fs_calls,
            }
        )
    return _render("Primary: provider × arm × task_class", rows)


def secondary_table(runs: list[dict]) -> str:
    cells: dict[tuple[str, str, str, str], dict] = defaultdict(
        lambda: {"cache_read_tokens": 0, "output_tokens": 0, "cost_usd": 0.0, "runs": 0}
    )
    for r in runs:
        if r["verdict"] == "error":
            continue
        for model, mu in (r.get("model_usage") or {}).items():
            k = (r["provider"], model, r["arm"], r["_task_class"])
            cells[k]["cache_read_tokens"] += mu.get("cache_read_tokens", 0)
            cells[k]["output_tokens"] += mu.get("output_tokens", 0)
            cells[k]["cost_usd"] += mu.get("cost_usd", 0.0)
            cells[k]["runs"] += 1
    rows = []
    for (provider, model, arm, cls), vals in sorted(cells.items()):
        rows.append(
            {
                "provider": provider,
                "model": model,
                "arm": arm,
                "task_class": cls,
                "runs": vals["runs"],
                "cache_read_tokens": vals["cache_read_tokens"],
                "output_tokens": vals["output_tokens"],
                "cost_usd": f"{vals['cost_usd']:.4f}",
            }
        )
    return _render("Secondary: provider × model × arm × task_class", rows)


def error_table(runs: list[dict]) -> str:
    errs = [r for r in runs if r["verdict"] == "error"]
    if not errs:
        return ""
    rows = [
        {
            "provider": r["provider"],
            "arm": r["arm"],
            "task_id": r["task_id"],
            "seed": r["seed"],
            "diagnostic": (r.get("diagnostic") or "")[:80],
        }
        for r in errs
    ]
    return _render("Errors (excluded from aggregates)", rows)


def _render(title: str, rows: list[dict]) -> str:
    if not rows:
        return f"### {title}\n\n_(no runs)_\n"
    cols = list(rows[0].keys())
    header = "| " + " | ".join(cols) + " |"
    sep = "|" + "|".join("---" for _ in cols) + "|"
    body = "\n".join("| " + " | ".join(str(r[c]) for c in cols) + " |" for r in rows)
    return f"### {title}\n\n{header}\n{sep}\n{body}\n"


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser()
    ap.add_argument("--runs", default=str(BENCH_ROOT / "runs"))
    ap.add_argument("--tasks", default=str(BENCH_ROOT / "tasks"))
    args = ap.parse_args(argv)

    runs = load_runs(Path(args.runs), Path(args.tasks))
    if not runs:
        print("no runs found", file=sys.stderr)
        return 1

    print(primary_table(runs))
    print(secondary_table(runs))
    err = error_table(runs)
    if err:
        print(err)
    other_calls = _total_other_calls(runs)
    if other_calls:
        print(
            (
                "warning: encountered "
                f"{other_calls} other tool-use events outside graph/filesystem/shell buckets"
            ),
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
