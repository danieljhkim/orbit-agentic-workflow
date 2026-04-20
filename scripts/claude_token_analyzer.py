#!/usr/bin/env python3
"""
Claude Code Token Usage Analyzer

Parses Claude Code session transcripts (JSONL) to show actual token consumption
broken down by I/O vs cache reads. Reveals the real cost of prompt caching.

Session transcripts are stored at:
    ~/.claude/projects/<project-hash>/<session-id>.jsonl

Each assistant message contains a usage object with:
    input_tokens, output_tokens, cache_creation_input_tokens, cache_read_input_tokens
"""

import json
import os
import glob
import argparse
from datetime import datetime, timedelta
from collections import defaultdict


def find_transcripts(base_path, start_date, end_date):
    """Find all JSONL transcript files within the date range."""
    transcripts = []
    for jsonl_file in glob.glob(f"{base_path}/**/*.jsonl", recursive=True):
        try:
            mtime = os.path.getmtime(jsonl_file)
            file_date = datetime.fromtimestamp(mtime)
            if start_date <= file_date <= end_date:
                transcripts.append(jsonl_file)
        except OSError:
            continue
    return transcripts


def parse_transcript(filepath):
    """Extract token usage from a single transcript file."""
    messages = []
    with open(filepath, "r") as f:
        for line in f:
            try:
                entry = json.loads(line)
                if entry.get("type") == "assistant" and "message" in entry:
                    usage = entry["message"].get("usage", {})
                    timestamp = entry.get("timestamp", "")
                    inp = usage.get("input_tokens", 0)
                    out = usage.get("output_tokens", 0)
                    cache_create = usage.get("cache_creation_input_tokens", 0)
                    cache_read = usage.get("cache_read_input_tokens", 0)
                    model = entry["message"].get("model", "unknown")

                    if inp or out or cache_create or cache_read:
                        messages.append({
                            "timestamp": timestamp,
                            "model": model,
                            "input": inp,
                            "output": out,
                            "cache_create": cache_create,
                            "cache_read": cache_read,
                        })
            except (json.JSONDecodeError, KeyError):
                continue
    return messages


def aggregate(messages):
    """Aggregate token counts from a list of messages."""
    totals = {"input": 0, "output": 0, "cache_create": 0, "cache_read": 0, "count": 0}
    for m in messages:
        totals["input"] += m["input"]
        totals["output"] += m["output"]
        totals["cache_create"] += m["cache_create"]
        totals["cache_read"] += m["cache_read"]
        totals["count"] += 1
    return totals


def format_number(n):
    """Format large numbers with commas."""
    return f"{n:,}"


def print_summary(totals, label="TOKEN USAGE SUMMARY"):
    """Print a formatted summary of token usage."""
    io_total = totals["input"] + totals["output"]
    all_total = io_total + totals["cache_create"] + totals["cache_read"]

    if all_total == 0:
        print(f"\n{label}\nNo token usage data found.\n")
        return

    ratio = totals["cache_read"] / io_total if io_total > 0 else 0
    cache_pct = totals["cache_read"] / all_total * 100 if all_total > 0 else 0
    io_pct = io_total / all_total * 100 if all_total > 0 else 0

    print(f"\n{'=' * 60}")
    print(f"  {label}")
    print(f"{'=' * 60}")
    print(f"  API messages:        {format_number(totals['count']):>20}")
    print(f"  Input tokens:        {format_number(totals['input']):>20}")
    print(f"  Output tokens:       {format_number(totals['output']):>20}")
    print(f"  I/O total:           {format_number(io_total):>20}")
    print(f"  Cache creation:      {format_number(totals['cache_create']):>20}")
    print(f"  Cache reads:         {format_number(totals['cache_read']):>20}")
    print(f"  ALL tokens:          {format_number(all_total):>20}")
    print(f"{'─' * 60}")
    print(f"  Cache read : I/O ratio:    {ratio:,.0f}:1")
    print(f"  Cache reads % of total:    {cache_pct:.2f}%")
    print(f"  I/O % of total:            {io_pct:.2f}%")
    print(f"{'=' * 60}\n")


def print_breakdown(buckets, bucket_labels, title="BREAKDOWN"):
    """Print a time-based breakdown table."""
    print(f"\n{'=' * 80}")
    print(f"  {title}")
    print(f"{'=' * 80}")
    print(f"  {'Period':<14} {'I/O':>12} {'Cache Reads':>16} {'Ratio':>8} {'Messages':>10}")
    print(f"  {'─' * 14} {'─' * 12} {'─' * 16} {'─' * 8} {'─' * 10}")

    for key in sorted(buckets.keys()):
        t = buckets[key]
        io = t["input"] + t["output"]
        cr = t["cache_read"]
        ratio = f"{cr / io:,.0f}:1" if io > 0 else "N/A"
        label = bucket_labels.get(key, key)
        print(f"  {label:<14} {format_number(io):>12} {format_number(cr):>16} {ratio:>8} {format_number(t['count']):>10}")

    print(f"{'=' * 80}\n")


def main():
    parser = argparse.ArgumentParser(description="Analyze Claude Code token usage from session transcripts")
    parser.add_argument("--days", type=int, default=1, help="Number of days to analyze (default: 1 = today)")
    parser.add_argument("--weekly", action="store_true", help="Show weekly breakdown")
    parser.add_argument("--daily", action="store_true", help="Show daily breakdown")
    parser.add_argument("--by-model", action="store_true", help="Show breakdown by model")
    parser.add_argument("--path", type=str, default=None, help="Custom path to Claude projects dir")
    args = parser.parse_args()

    base_path = args.path or os.path.expanduser("~/.claude/projects")

    if not os.path.exists(base_path):
        print(f"Error: Claude projects directory not found at {base_path}")
        print("Make sure Claude Code has been used and transcripts exist.")
        return

    end_date = datetime.now()
    start_date = end_date - timedelta(days=args.days)

    transcripts = find_transcripts(base_path, start_date, end_date)

    if not transcripts:
        print(f"No transcripts found between {start_date.strftime('%Y-%m-%d')} and {end_date.strftime('%Y-%m-%d')}")
        return

    all_messages = []
    for t in transcripts:
        all_messages.extend(parse_transcript(t))

    if not all_messages:
        print("No token usage data found in transcripts.")
        return

    totals = aggregate(all_messages)
    date_range = f"{start_date.strftime('%Y-%m-%d')} to {end_date.strftime('%Y-%m-%d')}"
    print_summary(totals, f"TOKEN USAGE - {date_range} ({len(transcripts)} sessions)")

    if args.weekly:
        buckets = defaultdict(lambda: {"input": 0, "output": 0, "cache_create": 0, "cache_read": 0, "count": 0})
        labels = {}
        for m in all_messages:
            try:
                ts = datetime.fromisoformat(m["timestamp"].replace("Z", "+00:00"))
                week_start = ts - timedelta(days=ts.weekday())
                key = week_start.strftime("%Y-%m-%d")
                labels[key] = f"Week {key}"
                for field in ["input", "output", "cache_create", "cache_read"]:
                    buckets[key][field] += m[field]
                buckets[key]["count"] += 1
            except (ValueError, KeyError):
                continue
        print_breakdown(dict(buckets), labels, "WEEKLY BREAKDOWN")

    if args.daily:
        buckets = defaultdict(lambda: {"input": 0, "output": 0, "cache_create": 0, "cache_read": 0, "count": 0})
        labels = {}
        for m in all_messages:
            try:
                ts = datetime.fromisoformat(m["timestamp"].replace("Z", "+00:00"))
                key = ts.strftime("%Y-%m-%d")
                labels[key] = key
                for field in ["input", "output", "cache_create", "cache_read"]:
                    buckets[key][field] += m[field]
                buckets[key]["count"] += 1
            except (ValueError, KeyError):
                continue
        print_breakdown(dict(buckets), labels, "DAILY BREAKDOWN")

    if args.by_model:
        buckets = defaultdict(lambda: {"input": 0, "output": 0, "cache_create": 0, "cache_read": 0, "count": 0})
        labels = {}
        for m in all_messages:
            key = m["model"]
            labels[key] = key[:14]
            for field in ["input", "output", "cache_create", "cache_read"]:
                buckets[key][field] += m[field]
            buckets[key]["count"] += 1
        print_breakdown(dict(buckets), labels, "BY MODEL")


if __name__ == "__main__":
    main()