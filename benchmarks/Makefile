.PHONY: help graph-run graph-run-claude graph-run-codex graph-sweep graph-sweep-dry-run graph-sweep-claude graph-sweep-codex graph-aggregate graph-test

PYTHON ?= python3
BENCH_ROOT := $(CURDIR)

# GRAPH_VERSION selects which version's tasks/ and runs/ to target.
#   v3 → living harness at benchmarks/graph/v3/    (default; round 3 in progress)
#   v1 → frozen snapshot at benchmarks/graph/v1/
#   v2 → frozen snapshot at benchmarks/graph/v2/
# See benchmarks/CONVENTIONS.md for versioning rules.
GRAPH_VERSION ?= v3

# Scripts live once under benchmarks/graph/scripts/ and are expected to stay
# backward-compatible with all frozen run records. If a future round needs
# to break back-compat (e.g. record schema change), copy scripts into that
# version's directory at freeze time and override GRAPH_SCRIPTS when
# reproducing it. See benchmarks/CONVENTIONS.md §Freezing a version.
GRAPH_SCRIPTS ?= $(BENCH_ROOT)/graph/scripts
GRAPH_DIR := $(BENCH_ROOT)/graph/$(GRAPH_VERSION)

GRAPH_PROVIDER ?= claude
GRAPH_ARM ?= no-graph
GRAPH_TASK ?= locate-agentruntime
GRAPH_TASKS ?=
GRAPH_SEED ?= 1
GRAPH_N ?= 1
GRAPH_ARMS ?= no-graph graph-only hybrid
GRAPH_SWEEP_ID ?=
GRAPH_RUN_ARGS ?=
GRAPH_SWEEP_ARGS ?=

# Export GRAPH_VERSION so the Python scripts resolve VERSION_ROOT correctly.
export GRAPH_VERSION

help:
	@echo "Benchmark Make Targets"
	@echo ""
	@echo "  make -C benchmarks graph-run"
	@echo "    Run one graph benchmark cell."
	@echo "    Vars: GRAPH_PROVIDER=claude|codex GRAPH_ARM=... GRAPH_TASK=... GRAPH_SEED=..."
	@echo ""
	@echo "  make -C benchmarks graph-sweep"
	@echo "    Run a graph sweep."
	@echo "    Vars: GRAPH_PROVIDER=claude|codex GRAPH_N=... GRAPH_TASKS=\"...\" GRAPH_ARMS=\"...\" GRAPH_SWEEP_ID=..."
	@echo ""
	@echo "  make -C benchmarks graph-sweep-dry-run"
	@echo "    Materialize sweep order without executing runs."
	@echo ""
	@echo "  make -C benchmarks graph-run-codex"
	@echo "  make -C benchmarks graph-run-claude"
	@echo "  make -C benchmarks graph-sweep-codex"
	@echo "  make -C benchmarks graph-sweep-claude"
	@echo ""
	@echo "  make -C benchmarks graph-aggregate"
	@echo "    Print aggregate benchmark tables for GRAPH_VERSION."
	@echo ""
	@echo "  make -C benchmarks graph-test"
	@echo "    Run benchmark script unit tests (living scripts only)."
	@echo ""
	@echo "Versioning:"
	@echo "  GRAPH_VERSION=v3 (default)  targets benchmarks/graph/v3/ (living)."
	@echo "  GRAPH_VERSION=v2            targets benchmarks/graph/v2/ (frozen)."
	@echo "  GRAPH_VERSION=v1            targets benchmarks/graph/v1/ (frozen)."
	@echo "  See benchmarks/CONVENTIONS.md."

graph-run:
	$(PYTHON) $(GRAPH_SCRIPTS)/run.py \
		$(GRAPH_ARM) \
		$(GRAPH_TASK) \
		$(GRAPH_SEED) \
		--provider $(GRAPH_PROVIDER) \
		$(GRAPH_RUN_ARGS)

graph-run-claude:
	$(MAKE) graph-run GRAPH_PROVIDER=claude

graph-run-codex:
	$(MAKE) graph-run GRAPH_PROVIDER=codex

graph-sweep:
	$(PYTHON) $(GRAPH_SCRIPTS)/sweep.py \
		--provider $(GRAPH_PROVIDER) \
		$(if $(GRAPH_TASKS),--tasks $(GRAPH_TASKS),) \
		--n $(GRAPH_N) \
		$(if $(GRAPH_SWEEP_ID),--sweep-id $(GRAPH_SWEEP_ID),) \
		$(if $(GRAPH_ARMS),--arms $(GRAPH_ARMS),) \
		$(GRAPH_SWEEP_ARGS)

graph-sweep-dry-run:
	$(MAKE) graph-sweep GRAPH_SWEEP_ARGS="$(GRAPH_SWEEP_ARGS) --dry-run"

graph-sweep-claude:
	$(MAKE) graph-sweep GRAPH_PROVIDER=claude

graph-sweep-codex:
	$(MAKE) graph-sweep GRAPH_PROVIDER=codex

graph-aggregate:
	$(PYTHON) $(GRAPH_SCRIPTS)/aggregate.py \
		--runs $(GRAPH_DIR)/runs \
		--tasks $(GRAPH_DIR)/tasks

graph-test:
	$(PYTHON) -m unittest discover -s $(BENCH_ROOT)/graph/scripts -p 'test_*.py'
