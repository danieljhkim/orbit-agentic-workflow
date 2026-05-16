#!/usr/bin/env bash
set -euo pipefail

mode="${1:?usage: run.sh capture|compare <fixture-workspace> <baseline-dir>}"
fixture="${2:?usage: run.sh capture|compare <fixture-workspace> <baseline-dir>}"
baseline="${3:?usage: run.sh capture|compare <fixture-workspace> <baseline-dir>}"

if [[ "$mode" != "capture" && "$mode" != "compare" ]]; then
  echo "mode must be capture or compare" >&2
  exit 2
fi

mkdir -p "$baseline"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
work="$tmpdir/workspace"
mkdir -p "$work"
rsync -a "$fixture"/ "$work"/

run_tool() {
  local name="$1"
  local input="$2"
  orbit tool run "$name" --input "$input"
}

cd "$work"

run_tool orbit.graph.search '{"query":"main","limit":10}' > "$tmpdir/search.json"
run_tool orbit.graph.search '{"query":"main","limit":10,"format":"selectors"}' > "$tmpdir/search_selectors.json"
run_tool orbit.graph.overview '{"format":"summary"}' > "$tmpdir/overview.json"
run_tool orbit.graph.show '{"selector":"dir:.","depth":1,"siblings":2,"children":5}' > "$tmpdir/show.json"
run_tool orbit.graph.refs '{"selector":"symbol:src/main.rs#main:function","include":["all"],"include_simple_name":true}' > "$tmpdir/refs.json" || true
run_tool orbit.graph.callers '{"selector":"symbol:src/main.rs#main:function","depth":2}' > "$tmpdir/callers.json" || true
run_tool orbit.graph.implementors '{"trait_selector":"symbol:src/main.rs#Runnable:trait"}' > "$tmpdir/implementors.json" || true
run_tool orbit.graph.deps '{}' > "$tmpdir/deps.json" || true
run_tool orbit.graph.pack '{"selectors":["dir:."],"summary":true}' > "$tmpdir/pack.json"
run_tool orbit.graph.write '{"selector":"file:src/equivalence_write.rs","new_source":"fn rewritten_equivalence_fixture() {}\n","reason":"equivalence"}' > "$tmpdir/write.json" || true
run_tool orbit.graph.add '{"selector":"symbol:src/equivalence_write.rs#added_equivalence_fixture:function","source":"fn added_equivalence_fixture() {}\n","reason":"equivalence"}' > "$tmpdir/add.json" || true
run_tool orbit.graph.move '{"selector":"symbol:src/equivalence_write.rs#added_equivalence_fixture:function","target_file":"src/equivalence_moved.rs","reason":"equivalence"}' > "$tmpdir/move.json" || true
run_tool orbit.graph.delete '{"selector":"symbol:src/equivalence_moved.rs#added_equivalence_fixture:function","reason":"equivalence"}' > "$tmpdir/delete.json" || true

if [[ "$mode" == "capture" ]]; then
  cp "$tmpdir"/*.json "$baseline"/
  exit 0
fi

for current in "$tmpdir"/*.json; do
  name="$(basename "$current")"
  cmp --silent "$baseline/$name" "$current" || {
    echo "equivalence mismatch: $name" >&2
    diff -u "$baseline/$name" "$current" >&2 || true
    exit 1
  }
done
