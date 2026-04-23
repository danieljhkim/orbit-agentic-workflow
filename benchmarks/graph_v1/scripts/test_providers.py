"""Unit tests for provider-specific benchmark helpers."""

from __future__ import annotations

import sys
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parent
if str(HERE) not in sys.path:
    sys.path.insert(0, str(HERE))

import providers


class TestCodexDenials(unittest.TestCase):
    def test_codex_overrides_disable_mcp_surfaces(self):
        overrides = set(providers._codex_overrides())
        self.assertIn("mcp_servers.orbit.enabled=false", overrides)
        self.assertIn("features.tool_call_mcp_elicitation=false", overrides)
        self.assertIn("features.skill_mcp_dependency_install=false", overrides)
        self.assertIn("features.plugins=false", overrides)
        self.assertIn("features.apps=false", overrides)

    def test_codex_prompt_forbids_direct_mcp_use(self):
        prompt = providers._build_codex_prompt(
            prompt="Locate AgentRuntime.",
            arm="hybrid",
            nonce="nonce",
            sweep_id="sweep",
            system_suffix="suffix",
        )
        self.assertIn("use only shell commands", prompt)
        self.assertIn("Ignore Orbit skills", prompt)
        self.assertIn("orbit tool run orbit.graph.*", prompt)

    def test_permission_marker_detection(self):
        self.assertTrue(
            providers._is_permission_denial_message(
                "error: store error: attempt to write a readonly database"
            )
        )
        self.assertTrue(
            providers._is_permission_denial_message("user cancelled MCP tool call")
        )
        self.assertFalse(
            providers._is_permission_denial_message(
                '{"code":"tool_not_found","error":"tool not found: orbit.graph.locate_symbol"}'
            )
        )
        self.assertFalse(
            providers._is_permission_denial_message(
                '{"code":"invalid_input","error":"missing trait_selector"}'
            )
        )

    def test_generic_command_failures_are_not_denials(self):
        events = [
            {
                "type": "item.completed",
                "item": {
                    "type": "command_execution",
                    "command": "orbit tool run orbit.graph.locate_symbol ...",
                    "status": "failed",
                    "exit_code": 1,
                    "aggregated_output": (
                        '{\n  "code": "tool_not_found",\n'
                        '  "error": "tool not found: orbit.graph.locate_symbol"\n}\n'
                    ),
                },
            },
            {
                "type": "item.completed",
                "item": {
                    "type": "command_execution",
                    "command": "orbit tool show orbit.graph.locate_symbol",
                    "status": "failed",
                    "exit_code": 1,
                    "aggregated_output": (
                        "error: store error: attempt to write a readonly database\n"
                    ),
                },
            },
        ]

        failures, denials = providers._codex_failures_and_denials(events)
        self.assertEqual(len(failures), 2)
        self.assertEqual(len(denials), 1)
        self.assertIn("readonly database", denials[0]["message"])

    def test_mcp_cancellation_counts_as_denial(self):
        events = [
            {
                "type": "item.completed",
                "item": {
                    "type": "mcp_tool_call",
                    "tool": "orbit.graph.search",
                    "status": "failed",
                    "error": {"message": "user cancelled MCP tool call"},
                },
            }
        ]

        failures, denials = providers._codex_failures_and_denials(events)
        self.assertEqual(len(failures), 1)
        self.assertEqual(len(denials), 1)
        self.assertEqual(denials[0]["tool"], "orbit.graph.search")


class TestTokenAccountingConvention(unittest.TestCase):
    """`input_tokens` must mean UNCACHED new input for both providers.

    Claude reports that natively; Codex reports total input with
    `cached_input_tokens` as a subset, so the normalizer must subtract.
    Regression guard: if someone flips this back to raw codex
    `input_tokens`, `aggregate.py`'s `input_tokens + output_tokens`
    column stops being cross-provider comparable.
    """

    def _codex_events(self, *, input_tokens: int, cached: int, output: int) -> list[dict]:
        return [
            {
                "type": "turn.completed",
                "usage": {
                    "input_tokens": input_tokens,
                    "cached_input_tokens": cached,
                    "output_tokens": output,
                },
            }
        ]

    def test_codex_input_tokens_is_uncached(self):
        result = providers._normalize_codex_result(
            self._codex_events(input_tokens=100_000, cached=80_000, output=1_500),
            requested_model="gpt-5.4",
            exit_code=0,
        )
        self.assertEqual(result["input_tokens"], 20_000)
        self.assertEqual(result["cache_read_tokens"], 80_000)
        self.assertEqual(result["output_tokens"], 1_500)

    def test_codex_input_tokens_never_negative(self):
        # Defensive: if codex ever reports cached > input (shouldn't happen
        # but the field contracts don't forbid it), clamp to zero rather
        # than propagate a negative.
        result = providers._normalize_codex_result(
            self._codex_events(input_tokens=50, cached=999, output=10),
            requested_model="gpt-5.4",
            exit_code=0,
        )
        self.assertEqual(result["input_tokens"], 0)

    def test_codex_model_usage_mirrors_top_level(self):
        result = providers._normalize_codex_result(
            self._codex_events(input_tokens=100, cached=60, output=20),
            requested_model="gpt-5.4",
            exit_code=0,
        )
        mu = result["model_usage"]["gpt-5.4"]
        self.assertEqual(mu["input_tokens"], result["input_tokens"])
        self.assertEqual(mu["cache_read_tokens"], result["cache_read_tokens"])
        self.assertEqual(mu["output_tokens"], result["output_tokens"])


if __name__ == "__main__":
    unittest.main()
