"""Unit tests for benchmark aggregation helpers."""

from __future__ import annotations

import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import aggregate


class AggregateHarnessTest(unittest.TestCase):
    def setUp(self) -> None:
        self.tempdir = tempfile.TemporaryDirectory()
        self.root = Path(self.tempdir.name)
        self.runs_dir = self.root / "runs"
        self.tasks_dir = self.root / "tasks"
        self.tasks_dir.mkdir(parents=True)

    def tearDown(self) -> None:
        self.tempdir.cleanup()

    def _write_task(self, task_id: str, task_class: str) -> None:
        (self.tasks_dir / f"{task_id}.yaml").write_text(
            f"task_id: {task_id}\nclass: {task_class}\n"
        )

    def _write_run(
        self,
        *,
        provider: str,
        arm: str,
        task_id: str,
        seed: int,
        verdict: str = "pass",
        input_tokens: int = 10,
        output_tokens: int = 5,
        transcript_events: list[dict] | None = None,
    ) -> None:
        task_dir = self.runs_dir / provider / arm / task_id
        task_dir.mkdir(parents=True, exist_ok=True)
        run_path = task_dir / f"{seed}.json"
        run_path.write_text(
            json.dumps(
                {
                    "provider": provider,
                    "arm": arm,
                    "task_id": task_id,
                    "seed": seed,
                    "verdict": verdict,
                    "input_tokens": input_tokens,
                    "output_tokens": output_tokens,
                }
            )
        )
        if transcript_events is not None:
            transcript_path = task_dir / f"{seed}.transcript.json"
            transcript_path.write_text(
                "".join(json.dumps(event) + "\n" for event in transcript_events)
            )

    def test_load_runs_classifies_claude_and_codex_transcripts(self):
        self._write_task("claude-task", "locate")
        self._write_task("codex-task", "trace")
        self._write_run(
            provider="claude",
            arm="hybrid",
            task_id="claude-task",
            seed=1,
            transcript_events=[
                {
                    "type": "assistant",
                    "message": {
                        "content": [
                            {
                                "type": "tool_use",
                                "name": "mcp__orbit-bench__orbit_graph_search",
                            },
                            {"type": "tool_use", "name": "Grep"},
                            {"type": "tool_use", "name": "Bash"},
                        ]
                    },
                }
            ],
        )
        self._write_run(
            provider="codex",
            arm="hybrid",
            task_id="codex-task",
            seed=1,
            transcript_events=[
                {
                    "type": "item.completed",
                    "item": {"type": "command_execution", "command": "rg foo"},
                },
                {
                    "type": "item.completed",
                    "item": {"type": "mcp_tool_call", "tool": "orbit.graph.search"},
                },
                {
                    "type": "item.completed",
                    "item": {"type": "mcp_tool_call", "tool": "fs.read"},
                },
            ],
        )

        runs = aggregate.load_runs(self.runs_dir, self.tasks_dir)
        by_task = {run["task_id"]: run["_tool_utilization"] for run in runs}

        self.assertEqual(
            by_task["claude-task"],
            aggregate.ToolUtilization(graph_calls=1, shell_or_fs_calls=2, other_calls=0),
        )
        self.assertEqual(
            by_task["codex-task"],
            aggregate.ToolUtilization(graph_calls=1, shell_or_fs_calls=1, other_calls=1),
        )

    def test_primary_table_appends_columns_and_handles_missing_transcripts(self):
        self._write_task("missing-transcript", "locate")
        self._write_run(
            provider="claude",
            arm="hybrid",
            task_id="missing-transcript",
            seed=1,
            transcript_events=None,
        )

        table = aggregate.primary_table(aggregate.load_runs(self.runs_dir, self.tasks_dir))

        self.assertIn(
            "| provider | arm | task_class | runs | pass_rate | "
            "median_total_tokens | p90_total_tokens | tokens_per_success | "
            "graph_calls | graph_call_rate | shell_or_fs_calls |",
            table,
        )
        self.assertIn("| - | N/A | - |", table)

    def test_main_reports_unclassified_tool_use_events_to_stderr(self):
        self._write_task("unknown-tool", "impact")
        self._write_run(
            provider="claude",
            arm="hybrid",
            task_id="unknown-tool",
            seed=1,
            transcript_events=[
                {
                    "type": "assistant",
                    "message": {"content": [{"type": "tool_use", "name": "MysteryTool"}]},
                }
            ],
        )

        stdout = io.StringIO()
        stderr = io.StringIO()
        with redirect_stdout(stdout), redirect_stderr(stderr):
            exit_code = aggregate.main(
                ["--runs", str(self.runs_dir), "--tasks", str(self.tasks_dir)]
            )

        self.assertEqual(exit_code, 0)
        self.assertIn("other tool-use events", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
