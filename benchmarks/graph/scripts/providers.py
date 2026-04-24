"""Provider-specific benchmark execution helpers.

This module owns all child-CLI details so run.py can stay focused on
benchmark orchestration.

## Token-accounting convention

Across providers, the normalized per-run record uses these semantics:

    input_tokens         — NEW input tokens for this run, EXCLUDING any
                           tokens read from the prompt cache. Matches
                           Claude's `usage.input_tokens` natively;
                           derived for Codex as `input - cached_input`.
    cache_read_tokens    — input tokens served from cache.
    cache_creation_tokens — input tokens that went into creating a new
                           cache entry (Claude only; Codex reports 0).
    output_tokens        — model-generated output tokens.

This means `input_tokens + output_tokens` is a provider-comparable
measure of "marginal tokens paid for this run." Cache reads are tracked
separately so cross-arm cache efficiency is still observable.
"""

from __future__ import annotations

import json
import os
import re
import subprocess
import uuid
from dataclasses import dataclass
from pathlib import Path

CLAUDE_BIN = os.environ.get("CLAUDE_BIN", "/Users/daniel/.local/bin/claude")
CODEX_BIN = os.environ.get("CODEX_BIN", "codex")
ORBIT_DATA_ROOT = os.environ.get("ORBIT_DATA_ROOT", os.path.expanduser("~/.orbit"))
DEFAULT_MODELS = {
    # Sonnet is the main-run model. We experimented with opus during Gate C
    # of the v2 fixture design (some sonnet runs refused to engage with graph
    # tools under graph-only), but opus's economics make a full sweep
    # impractical — it hit the subscription's usage window mid-sweep, which
    # surfaces at the CLI as a "Credit balance is too low" 400. Sonnet stays
    # well under the window and was sufficient in all smoke tests.
    "claude": "sonnet",
    # v1 ran `gpt-5.4`; v2 regressed to `gpt-5.3-codex` silently. v3 pins the
    # model explicitly so the v3-vs-v2 delta is interpretable. Bump with
    # intent, not drift. Override per-run via `GRAPH_CODEX_MODEL` if needed.
    "codex": os.environ.get("GRAPH_CODEX_MODEL", "gpt-5.3-codex"),
}

CLAUDE_GRAPH_TOOLS = (
    "mcp__orbit-bench__orbit_graph_search",
    "mcp__orbit-bench__orbit_graph_show",
    "mcp__orbit-bench__orbit_graph_callers",
    "mcp__orbit-bench__orbit_graph_implementors",
    "mcp__orbit-bench__orbit_graph_refs",
    "mcp__orbit-bench__orbit_graph_overview",
    "mcp__orbit-bench__orbit_graph_deps",
    "mcp__orbit-bench__orbit_graph_pack",
)

GRAPH_COMMAND_RE = re.compile(r"\borbit tool run (orbit\.graph\.[a-z]+)\b")

ESCAPE_HATCHES = (
    "Agent",
    "Task",
    "Skill",
    "EnterPlanMode",
    "ExitPlanMode",
    "EnterWorktree",
    "ExitWorktree",
    "Monitor",
    "ScheduleWakeup",
    "SendMessage",
    "AskUserQuestion",
    "CronCreate",
    "CronDelete",
    "CronList",
    "TeamCreate",
    "TeamDelete",
    "TaskOutput",
    "TaskStop",
    "PushNotification",
    "RemoteTrigger",
    "NotebookEdit",
)

BASE_DENY = (
    "Bash",
    "Edit",
    "Write",
    "TodoWrite",
    "WebSearch",
    "WebFetch",
    *ESCAPE_HATCHES,
)

CLAUDE_ARM_STEER = {
    "no-graph": (
        "You have filesystem navigation tools (Read, Grep, Glob) but not "
        "the orbit knowledge graph. Answer using the filesystem; verify "
        "by reading source files before stating locations."
    ),
    "graph-only": (
        "You have ONLY orbit knowledge-graph MCP tools "
        "(mcp__orbit-bench__orbit_graph_*). You do NOT have Read, Grep, or "
        "Glob. Answer by querying the graph — start with "
        "mcp__orbit-bench__orbit_graph_search. Do not guess paths; if the "
        "graph cannot answer, say so."
    ),
    "hybrid": (
        "You have both filesystem tools (Read, Grep, Glob) AND orbit "
        "knowledge-graph tools (mcp__orbit-bench__orbit_graph_*). Choose the "
        "tool best fit for each sub-question."
    ),
}

CODEX_ARM_STEER = {
    "no-graph": (
        "You have only a shell command tool for filesystem navigation. Use "
        "`rg`, `git grep`, `find`, `ls`, and focused `sed -n` reads. The "
        "knowledge-graph MCP server is not available in this arm."
    ),
    "graph-only": (
        "You have ONLY the orbit knowledge-graph MCP tools "
        "(orbit_bench.orbit_graph_*). You do NOT have shell/exec. Answer by "
        "calling the MCP graph tools — start with orbit_graph_search or "
        "orbit_graph_overview. Do not guess paths; if the graph cannot "
        "answer, say so."
    ),
    "hybrid": (
        "You have both a shell command tool (rg, ls, cat, find, grep, etc.) "
        "AND the orbit knowledge-graph MCP tools (orbit_bench.orbit_graph_*). "
        "Choose the tool best fit for each sub-question."
    ),
}

ARM_STEER_BY_PROVIDER = {
    "claude": CLAUDE_ARM_STEER,
    "codex": CODEX_ARM_STEER,
}


@dataclass(frozen=True)
class ProviderArmConfig:
    allowed: tuple[str, ...]
    disallowed: tuple[str, ...]


@dataclass(frozen=True)
class ProviderExecution:
    exit_code: int
    normalized_result: dict | None
    raw_stdout: str
    events: list[dict]


def build_system_prompt_suffix(
    nonce: str,
    sweep_id: str,
    arm: str,
    provider: str = "claude",
) -> str:
    steer = ARM_STEER_BY_PROVIDER.get(provider, {}).get(arm, "")
    return (
        f"\n\n<!-- benchmark-nonce: {nonce} sweep: {sweep_id} "
        f"provider: {provider} arm: {arm} -->\n"
        f"You are a benchmark subject. {steer} "
        f"Verify every claim with a tool call; do not answer from memory."
    )


def get_provider(
    name: str,
    *,
    repo_root: Path,
    mcp_config: str,
):
    if name == "claude":
        return ClaudeProvider(repo_root=repo_root, mcp_config=mcp_config)
    if name == "codex":
        return CodexProvider(repo_root=repo_root, mcp_config=mcp_config)
    raise SystemExit(f"unknown provider: {name!r}")


def _parse_jsonl_stream(raw: str) -> list[dict]:
    events = []
    for line in raw.splitlines():
        line = line.strip()
        if not line:
            continue
        try:
            events.append(json.loads(line))
        except json.JSONDecodeError:
            continue
    return events


def _run_jsonl_subprocess(
    *,
    cmd: list[str],
    cwd: Path,
    timeout_s: int,
    input_text: str | None = None,
) -> tuple[int, str, list[dict]]:
    try:
        proc = subprocess.run(
            cmd,
            input=input_text,
            capture_output=True,
            text=True,
            timeout=timeout_s,
            cwd=cwd,
        )
    except subprocess.TimeoutExpired as error:
        return (124, f"timeout after {timeout_s}s: {error}", [])

    raw = proc.stdout
    return (proc.returncode, raw, _parse_jsonl_stream(raw))


def _is_graph_tool_name(name: str) -> bool:
    # Claude MCP naming: `mcp__orbit-bench__orbit_graph_<op>`
    # Codex MCP naming: `orbit_bench.orbit_graph_<op>` / `orbit_bench__orbit_graph_<op>`
    # Codex shell CLI: `orbit.graph.<op>` (extracted by GRAPH_COMMAND_RE)
    return (
        name.startswith("mcp__orbit-bench__orbit_graph_")
        or name.startswith("orbit_bench.orbit_graph_")
        or name.startswith("orbit_bench__orbit_graph_")
        or name.startswith("orbit.graph.")
        or "orbit_graph_" in name  # defensive fallback for unexpected prefixing
    )


def _graph_tool_name_from_command(command: str) -> str | None:
    match = GRAPH_COMMAND_RE.search(command)
    if match is None:
        return None
    return match.group(1)


def _build_codex_prompt(
    *,
    prompt: str,
    arm: str,
    nonce: str,
    sweep_id: str,
    system_suffix: str,
) -> str:
    scaffold = system_suffix.strip()
    return (
        f"<run-nonce sweep={sweep_id} nonce={nonce} provider=codex arm={arm} />\n\n"
        f"{scaffold}\n\n"
        "Benchmark rule: respect the arm's allowed surface. Ignore user-level "
        "Orbit skills that suggest tools outside that surface. Do not fall "
        "back to alternative paths if a preferred tool fails; report the "
        "failure in the final message instead.\n\n"
        f"Task:\n{prompt}"
    )


def _codex_mcp_overrides_from_config(mcp_config_path: str) -> list[str]:
    """Translate an mcp.json config into codex `-c` override flags.

    The mcp.json format matches Claude's `--mcp-config` shape:

        {"mcpServers": {"<name>": {"command": ..., "args": [...], "env": {...}}}}

    We rename the server to `orbit_bench` on the codex side (underscore form
    required for TOML keys) and emit one override per field. The server is
    emitted with `enabled=true`; per-arm gating happens in `_codex_overrides`.
    """
    with open(mcp_config_path) as fh:
        cfg = json.load(fh)
    servers = cfg.get("mcpServers", {}) or {}
    overrides: list[str] = []
    for _raw_name, spec in servers.items():
        # Normalize to `orbit_bench` (TOML-legal key) regardless of the
        # incoming name. Only one server is expected in bench config.
        key = "orbit_bench"
        command = spec.get("command")
        if command:
            overrides.append(f'mcp_servers.{key}.command={json.dumps(command)}')
        args = spec.get("args")
        if args:
            overrides.append(f"mcp_servers.{key}.args={json.dumps(args)}")
        env = spec.get("env")
        if env:
            # TOML inline table: env = { KEY = "value", ... }
            pairs = ", ".join(f'{k} = {json.dumps(v)}' for k, v in env.items())
            overrides.append(f"mcp_servers.{key}.env={{{pairs}}}")
        overrides.append(f"mcp_servers.{key}.enabled=true")
    return overrides


def _codex_overrides(*, arm: str, mcp_config_path: str | None) -> list[str]:
    # Per-arm: no-graph disables the orbit-bench MCP server AND keeps
    # exec_command on. graph-only disables exec_command and keeps the MCP
    # server on. hybrid keeps both on.
    shell_enabled = arm in ("no-graph", "hybrid")
    mcp_enabled = arm in ("graph-only", "hybrid")

    enabled_tools = '["exec_command"]' if shell_enabled else "[]"
    base = [
        "default_tools_enabled=false",
        f"enabled_tools={enabled_tools}",
        'approval_policy="never"',
        # Neutralize any user-level orbit MCP server so only the bench-scoped
        # orbit_bench server is visible to the agent.
        "mcp_servers.orbit.enabled=false",
        # Keep elicitation on — v3 smoke test showed that disabling it causes
        # codex to auto-cancel MCP tool calls ("user cancelled MCP tool call")
        # when a tool schema has any optional field. We accept a small mid-run
        # elicitation risk in exchange for MCP calls actually running.
        "features.skill_mcp_dependency_install=false",
        "features.plugins=false",
        "features.apps=false",
    ]
    if mcp_enabled and mcp_config_path:
        base.extend(_codex_mcp_overrides_from_config(mcp_config_path))
    elif mcp_config_path:
        # Load server definition but disable it, so the arm signal is purely
        # "MCP server off" rather than "MCP server never configured".
        base.extend(
            override if not override.endswith(".enabled=true")
            else override[:-len("=true")] + "=false"
            for override in _codex_mcp_overrides_from_config(mcp_config_path)
        )
    return base


def _is_permission_denial_message(message: str) -> bool:
    lowered = message.lower()
    markers = (
        "permission denied",
        "not allowed",
        "approval",
        "user cancelled",
        "read-only",
        "readonly",
        "sandbox",
        "access denied",
    )
    return any(marker in lowered for marker in markers)


def _count_claude_tool_calls(events: list[dict]) -> dict[str, int]:
    histogram: dict[str, int] = {}
    for event in events:
        if event.get("type") != "assistant":
            continue
        message = event.get("message", {}) or {}
        for block in message.get("content", []) or []:
            if block.get("type") == "tool_use":
                name = block.get("name", "<unknown>")
                histogram[name] = histogram.get(name, 0) + 1
    return histogram


def _codex_item_tool_names(item: dict) -> list[str]:
    item_type = item.get("type")
    if item_type == "command_execution":
        names = ["exec_command"]
        graph_name = _graph_tool_name_from_command(item.get("command", ""))
        if graph_name is not None:
            names.append(graph_name)
        return names
    if item_type == "mcp_tool_call":
        tool_name = item.get("tool")
        return [tool_name] if isinstance(tool_name, str) and tool_name else []
    return []


def _count_codex_tool_calls(events: list[dict]) -> dict[str, int]:
    histogram: dict[str, int] = {}
    for event in events:
        if event.get("type") != "item.started":
            continue
        item = event.get("item", {}) or {}
        for name in _codex_item_tool_names(item):
            histogram[name] = histogram.get(name, 0) + 1
    return histogram


def _codex_failures_and_denials(events: list[dict]) -> tuple[list[dict], list[dict]]:
    failures: list[dict] = []
    denials: list[dict] = []
    for event in events:
        if event.get("type") != "item.completed":
            continue
        item = event.get("item", {}) or {}
        item_type = item.get("type")
        if item_type == "command_execution":
            exit_code = item.get("exit_code")
            status = item.get("status")
            if status == "failed" or (exit_code not in (None, 0)):
                output = item.get("aggregated_output", "")
                failures.append(
                    {
                        "type": item_type,
                        "command": item.get("command"),
                        "exit_code": exit_code,
                        "status": status,
                        "output": output,
                    }
                )
                message = output.strip() or status or ""
                if _is_permission_denial_message(message):
                    denials.append(
                        {
                            "tool": "exec_command",
                            "message": message,
                        }
                    )
        elif item_type == "mcp_tool_call" and item.get("status") == "failed":
            error = item.get("error") or {}
            message = error.get("message", "")
            failures.append(
                {
                    "type": item_type,
                    "tool": item.get("tool"),
                    "status": item.get("status"),
                    "message": message,
                }
            )
            denial_message = message or item.get("status") or ""
            if _is_permission_denial_message(denial_message):
                denials.append(
                    {
                        "tool": item.get("tool", "<unknown>"),
                        "message": denial_message,
                    }
                )
    return (failures, denials)


def _codex_final_message(events: list[dict]) -> str:
    final = ""
    for event in events:
        if event.get("type") != "item.completed":
            continue
        item = event.get("item", {}) or {}
        if item.get("type") == "agent_message":
            final = item.get("text") or final
    return final


def _normalize_model_usage(raw_usage: dict) -> dict:
    out = {}
    for model, entry in (raw_usage or {}).items():
        out[model] = {
            "input_tokens": entry.get("inputTokens", 0),
            "cache_read_tokens": entry.get("cacheReadInputTokens", 0),
            "cache_creation_tokens": entry.get("cacheCreationInputTokens", 0),
            "output_tokens": entry.get("outputTokens", 0),
            "cost_usd": entry.get("costUSD", 0.0),
        }
    return out


def _normalize_claude_result(parsed: dict, events: list[dict], requested_model: str) -> dict:
    usage = parsed.get("usage") or {}
    return {
        "provider": "claude",
        "requested_model": requested_model,
        "is_error": bool(parsed.get("is_error")),
        "error_detail": parsed.get("api_error_status"),
        "final_message": parsed.get("result") or "",
        "turns": parsed.get("num_turns", 0),
        "input_tokens": usage.get("input_tokens", 0),
        "cache_read_tokens": usage.get("cache_read_input_tokens", 0),
        "cache_creation_tokens": usage.get("cache_creation_input_tokens", 0),
        "output_tokens": usage.get("output_tokens", 0),
        "total_cost_usd": parsed.get("total_cost_usd", 0.0),
        "model_usage": _normalize_model_usage(parsed.get("modelUsage", {})),
        "permission_denials": parsed.get("permission_denials", []),
        "tool_calls": _count_claude_tool_calls(events),
        "command_failures": [],
    }


def _normalize_codex_result(events: list[dict], requested_model: str, exit_code: int) -> dict:
    turn_completed = next(
        (event for event in reversed(events) if event.get("type") == "turn.completed"),
        None,
    )
    usage = (turn_completed or {}).get("usage", {}) or {}
    # Codex reports `input_tokens` as TOTAL input (cache_read is a subset).
    # Normalize to match Claude's convention: `input_tokens` means UNCACHED
    # new input only. See module docstring.
    raw_input = usage.get("input_tokens", 0)
    cached_input = usage.get("cached_input_tokens", 0)
    uncached_input = max(0, raw_input - cached_input)
    output_tokens = usage.get("output_tokens", 0)
    failures, denials = _codex_failures_and_denials(events)
    model_usage = {
        requested_model: {
            "input_tokens": uncached_input,
            "cache_read_tokens": cached_input,
            "cache_creation_tokens": 0,
            "output_tokens": output_tokens,
            "cost_usd": 0.0,
        }
    }
    return {
        "provider": "codex",
        "requested_model": requested_model,
        "is_error": exit_code != 0 or turn_completed is None,
        "error_detail": None if turn_completed is not None else "missing turn.completed event",
        "final_message": _codex_final_message(events),
        "turns": sum(1 for event in events if event.get("type") == "turn.completed"),
        "input_tokens": uncached_input,
        "cache_read_tokens": cached_input,
        "cache_creation_tokens": 0,
        "output_tokens": output_tokens,
        "total_cost_usd": 0.0,
        "model_usage": model_usage,
        "permission_denials": denials,
        "tool_calls": _count_codex_tool_calls(events),
        "command_failures": failures,
    }


class BenchmarkProvider:
    name: str
    default_model: str

    def __init__(self, *, repo_root: Path, mcp_config: str):
        self.repo_root = repo_root
        self.mcp_config = mcp_config

    def arm_config(self, arm: str) -> ProviderArmConfig:
        raise NotImplementedError

    def run(
        self,
        *,
        prompt: str,
        arm: str,
        arm_config: ProviderArmConfig,
        system_suffix: str,
        nonce: str,
        sweep_id: str,
        model: str,
        timeout_s: int = 1000,
    ) -> ProviderExecution:
        raise NotImplementedError

    def preflight_probe(
        self,
        *,
        arm: str,
        nonce: str,
        sweep_id: str,
        model: str,
    ) -> tuple[bool, str]:
        raise NotImplementedError


class ClaudeProvider(BenchmarkProvider):
    name = "claude"
    default_model = DEFAULT_MODELS["claude"]

    def arm_config(self, arm: str) -> ProviderArmConfig:
        base_fs = ("Read", "Grep", "Glob")
        if arm == "no-graph":
            return ProviderArmConfig(
                allowed=base_fs,
                disallowed=BASE_DENY + CLAUDE_GRAPH_TOOLS,
            )
        if arm == "graph-only":
            return ProviderArmConfig(
                allowed=CLAUDE_GRAPH_TOOLS,
                disallowed=BASE_DENY + base_fs,
            )
        if arm == "hybrid":
            return ProviderArmConfig(
                allowed=base_fs + CLAUDE_GRAPH_TOOLS,
                disallowed=BASE_DENY,
            )
        raise SystemExit(f"unknown arm: {arm!r}")

    def run(
        self,
        *,
        prompt: str,
        arm: str,
        arm_config: ProviderArmConfig,
        system_suffix: str,
        nonce: str,
        sweep_id: str,
        model: str,
        timeout_s: int = 1000,
    ) -> ProviderExecution:
        preamble = f"<run-nonce sweep={sweep_id} nonce={nonce} provider=claude />\n\n"
        # No --max-budget-usd: subscription plans surface budget exhaustion as
        # a 400 "Credit balance is too low" at the API layer, not a CLI-side
        # abort. Keeping the flag at a small number only caused the pre-flight
        # probe to trip on MCP schema cache-creation on expensive models.
        cmd = [
            CLAUDE_BIN,
            "-p",
            preamble + prompt,
            "--output-format",
            "stream-json",
            "--verbose",
            "--no-session-persistence",
            "--exclude-dynamic-system-prompt-sections",
            "--model",
            model,
            "--mcp-config",
            self.mcp_config,
            "--strict-mcp-config",
            "--append-system-prompt",
            system_suffix,
            "--allowed-tools",
            " ".join(arm_config.allowed),
            "--disallowed-tools",
            " ".join(arm_config.disallowed),
        ]
        exit_code, raw, events = _run_jsonl_subprocess(
            cmd=cmd,
            cwd=self.repo_root,
            timeout_s=timeout_s,
        )
        parsed = next((event for event in events if event.get("type") == "result"), None)
        normalized = None
        if parsed is not None:
            normalized = _normalize_claude_result(parsed, events, model)
        return ProviderExecution(
            exit_code=exit_code,
            normalized_result=normalized,
            raw_stdout=raw,
            events=events,
        )

    def preflight_probe(
        self,
        *,
        arm: str,
        nonce: str,
        sweep_id: str,
        model: str,  # accepted for ABC compatibility; the probe always runs on haiku
    ) -> tuple[bool, str]:
        # The probe is a smoke test: "is the MCP server reachable and does the
        # graph-tool surface load in the child CLI?" That question is
        # model-independent. We pin haiku regardless of the main-run model so
        # the $0.25 budget covers the MCP schema cache-creation tax even when
        # the main run is on opus or sonnet.
        del model
        probe_result = self.run(
            prompt=(
                "Call mcp__orbit-bench__orbit_graph_overview once with input "
                f'{{"format":"summary","workspace":"{self.repo_root}"}}. '
                "After the tool returns, reply with exactly: PROBE_OK."
            ),
            arm=arm,
            arm_config=ProviderArmConfig(
                allowed=(
                    "mcp__orbit-bench__orbit_graph_overview",
                    "mcp__orbit-bench__orbit_graph_search",
                ),
                disallowed=("Read", "Grep", "Glob", "Bash", "Edit", "Write"),
            ),
            system_suffix=build_system_prompt_suffix(
                nonce=f"probe-{uuid.uuid4().hex[:8]}",
                sweep_id="probe",
                arm=arm,
                provider=self.name,
            ),
            nonce=f"probe-{uuid.uuid4().hex[:8]}",
            sweep_id="probe",
            model="haiku",
            timeout_s=90,
        )
        normalized = probe_result.normalized_result
        if probe_result.exit_code != 0 or normalized is None:
            return (
                False,
                f"probe exit={probe_result.exit_code} result_present={normalized is not None}",
            )
        graph_calls = {
            name: count
            for name, count in (normalized.get("tool_calls") or {}).items()
            if _is_graph_tool_name(name)
        }
        final = (normalized.get("final_message") or "").upper()
        # Preferred signal: tool call AND PROBE_OK. Acceptable: tool call only.
        # Accepted but flagged: PROBE_OK only — haiku will sometimes shortcut-
        # answer without actually invoking the tool. Exit=0 + PROBE_OK is
        # sufficient evidence that the CLI invocation and MCP config loaded;
        # the main run will surface any downstream MCP failure quickly.
        if "PROBE_OK" in final and graph_calls:
            return (True, "PROBE_OK")
        if graph_calls:
            return (True, f"graph tool call observed despite missing PROBE_OK: {graph_calls}")
        if "PROBE_OK" in final:
            return (True, "PROBE_OK (haiku shortcut; MCP config loaded but tool not exercised)")
        return (False, f"probe made zero graph-tool calls; final={final[:120]!r}")


class CodexProvider(BenchmarkProvider):
    name = "codex"
    default_model = DEFAULT_MODELS["codex"]

    def arm_config(self, arm: str) -> ProviderArmConfig:
        if arm not in CODEX_ARM_STEER:
            raise SystemExit(f"unknown arm: {arm!r}")
        # Codex doesn't take an --allowed-tools CLI flag; per-arm gating is
        # enforced via the `-c` overrides built from the arm name in
        # `_codex_overrides`. The ProviderArmConfig is retained for
        # record-keeping parity with Claude but is not wired into the child
        # CLI invocation.
        if arm == "no-graph":
            return ProviderArmConfig(allowed=("exec_command",), disallowed=())
        if arm == "graph-only":
            return ProviderArmConfig(allowed=("mcp:orbit_bench.orbit_graph_*",), disallowed=("exec_command",))
        if arm == "hybrid":
            return ProviderArmConfig(
                allowed=("exec_command", "mcp:orbit_bench.orbit_graph_*"),
                disallowed=(),
            )
        raise SystemExit(f"unknown arm: {arm!r}")

    def run(
        self,
        *,
        prompt: str,
        arm: str,
        arm_config: ProviderArmConfig,
        system_suffix: str,
        nonce: str,
        sweep_id: str,
        model: str,
        timeout_s: int = 1000,
    ) -> ProviderExecution:
        del arm_config
        cmd = [
            CODEX_BIN,
            "exec",
            "--json",
            "--ephemeral",
            "--skip-git-repo-check",
            # v3: danger-full-access (was read-only in v2). Two reasons:
            #  1. The orbit binary's SQLite WAL needs write access in the
            #     ORBIT_DATA_ROOT; v2's read-only sandbox produced 45 of the
            #     157 command_failures (see v2/METHOD.md caveat #8).
            #  2. In non-interactive `codex exec` with `approval_policy=never`,
            #     workspace-write auto-CANCELS MCP tool calls ("user cancelled
            #     MCP tool call"); only danger-full-access auto-approves them.
            #     This was verified against a clean smoke test of the user's
            #     production `orbit` MCP server — the cancel behavior is
            #     independent of our server wiring.
            # Benchmark tasks never request writes (edit/apply_patch aren't in
            # enabled_tools) so the widened sandbox only unblocks tool-call
            # auto-approval and the store's journaling. Runs are `--ephemeral`
            # so no codex session state persists between cells.
            "--sandbox",
            "danger-full-access",
            "--cd",
            str(self.repo_root),
            "--model",
            model,
        ]
        for override in _codex_overrides(arm=arm, mcp_config_path=self.mcp_config):
            cmd.extend(["-c", override])
        cmd.append("-")

        exit_code, raw, events = _run_jsonl_subprocess(
            cmd=cmd,
            cwd=self.repo_root,
            timeout_s=timeout_s,
            input_text=_build_codex_prompt(
                prompt=prompt,
                arm=arm,
                nonce=nonce,
                sweep_id=sweep_id,
                system_suffix=system_suffix,
            ),
        )
        normalized = None
        if events:
            normalized = _normalize_codex_result(events, model, exit_code)
        return ProviderExecution(
            exit_code=exit_code,
            normalized_result=normalized,
            raw_stdout=raw,
            events=events,
        )

    def preflight_probe(
        self,
        *,
        arm: str,
        nonce: str,
        sweep_id: str,
        model: str,
    ) -> tuple[bool, str]:
        # Per-arm probe: no-graph smoke-tests the shell; graph-only/hybrid
        # smoke-test the MCP graph server is reachable. The probe verifies
        # the ARM's expected surface is live, not just any tool surface.
        if arm == "no-graph":
            probe_prompt = (
                "Run `orbit --version` exactly once with the shell tool. "
                "After the command returns successfully, reply with exactly: "
                "PROBE_OK."
            )
        else:
            probe_prompt = (
                "Call the orbit_graph_overview MCP tool from the orbit_bench "
                "server exactly once with arguments "
                f'{{"format":"summary","workspace":"{self.repo_root}"}}. '
                "After the tool returns, reply with exactly: PROBE_OK."
            )
        probe_result = self.run(
            prompt=probe_prompt,
            arm=arm,
            arm_config=self.arm_config(arm),
            system_suffix=build_system_prompt_suffix(
                nonce=f"probe-{nonce}",
                sweep_id=f"{sweep_id}-probe",
                arm=arm,
                provider=self.name,
            ),
            nonce=f"probe-{nonce}",
            sweep_id=f"{sweep_id}-probe",
            model=model,
            timeout_s=90,
        )
        normalized = probe_result.normalized_result
        if probe_result.exit_code != 0 or normalized is None:
            return (
                False,
                f"probe exit={probe_result.exit_code} result_present={normalized is not None}",
            )
        if normalized.get("command_failures"):
            return (False, f"probe command failures: {normalized['command_failures']}")
        final = (normalized.get("final_message") or "").upper()
        tool_calls = normalized.get("tool_calls") or {}
        if arm == "no-graph":
            # Success: any shell exec completed and the agent emitted PROBE_OK.
            if "PROBE_OK" in final and "exec_command" in tool_calls:
                return (True, "PROBE_OK")
            return (
                False,
                f"no-graph probe missed shell+PROBE_OK; tool_calls={tool_calls} final={final[:120]!r}",
            )
        # graph-only / hybrid: require an MCP graph call.
        graph_calls = {
            name: count
            for name, count in tool_calls.items()
            if _is_graph_tool_name(name)
        }
        if "PROBE_OK" in final and graph_calls:
            return (True, "PROBE_OK")
        if graph_calls:
            return (True, f"graph call observed despite missing PROBE_OK: {graph_calls}")
        return (False, f"probe made zero graph-tool calls; final={final[:120]!r}")
