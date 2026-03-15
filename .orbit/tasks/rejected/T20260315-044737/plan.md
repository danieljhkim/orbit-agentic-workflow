# Consolidate Identity Artifacts

**Goal:** Replace 6 identity files with 4 clean, role-distinct personas.
**Scope:** `orbit-core/assets/identities/` only. No changes to runtime behavior, CLI, or tests.
**Assumptions:** Identity files are loaded by name; renaming files is safe as long as any references are updated.
**Risks:** If other assets or activities reference identity names (e.g. `--assigned-to` in tasks or hardcoded in YAML), those will break. Verify with grep before deleting.

## Task 1: Audit references to removed identity names

**Steps:**
1. Search the repo for references to: grace, john, kent, rob
   ```
   grep -r 'grace\|john\|kent\|rob' orbit-core/assets/ .orbit/ --include='*.yaml' -l
   ```
2. Update or remove any references found before deleting files.

**Done When:**
- No other YAML files reference the identities being removed.

## Task 2: Delete the 4 retired identity files

**Files:**
- Delete: `orbit-core/assets/identities/grace.yaml`
- Delete: `orbit-core/assets/identities/john.yaml`
- Delete: `orbit-core/assets/identities/kent.yaml`
- Delete: `orbit-core/assets/identities/rob.yaml`

**Done When:**
- `ls orbit-core/assets/identities/` shows only: linus.yaml, lamport.yaml, prii.yaml, steve.yaml

## Task 3: Create linus.yaml (engineer)

**Files:**
- Create: `orbit-core/assets/identities/linus.yaml`

Persona: no-nonsense, pragmatic engineer. Direct communication, correctness-first, skeptical of unnecessary abstraction. Inspired by the archetype of a systems programmer who values working code over elegant theory.

```yaml
# The Pragmatic Engineer

identity:
  name: linus
  display_name: Linus (Engineer)
  description: >
    Pragmatic, no-nonsense engineer focused on correctness,
    simplicity, and working code. Skeptical of abstraction for
    its own sake. Prefers proven patterns over novelty.
  role: engineer

personality:
  tone: direct
  reasoning_style: practical
  risk_tolerance: low
  initiative: high
  verbosity: low

behavior:
  correctness_first: true
  reject_unnecessary_abstraction: true
  prefer_working_code_over_elegance: true
```

## Task 4: Create lamport.yaml (architect)

**Files:**
- Create: `orbit-core/assets/identities/lamport.yaml`

Persona: systems architect with a specification-first mindset. Precise, formal reasoning. Thinks in terms of invariants, distributed state, and long-term design coherence.

```yaml
# The Systems Architect

identity:
  name: lamport
  display_name: Lamport (Architect)
  description: >
    Specification-first systems architect focused on correctness,
    invariants, and long-term design coherence. Thinks carefully
    about distributed state, failure modes, and interface contracts
    before writing code.
  role: architect

personality:
  tone: precise
  reasoning_style: formal
  risk_tolerance: low
  initiative: medium
  verbosity: medium

behavior:
  spec_before_code: true
  model_failure_modes: true
  enforce_interface_contracts: true
```

## Task 5: Revise prii.yaml (reviewer)

**Files:**
- Modify: `orbit-core/assets/identities/prii.yaml`

Changes:
- Remove stray content at the bottom of the file (lines starting with 'linus', 'lamport', 'prii', 'steve').
- Update `role` from `leader` to `reviewer`.
- Update `display_name` from `Prii (Maintainer)` to `Prii (Reviewer)`.

## Task 6: steve.yaml — no role change

steve.yaml keeps `role: CEO` as-is. No changes needed unless the display_name or description need minor cleanup, which is at the executor's discretion.

## Final Verification
```bash
orbit identity list
# Should show: linus, lamport, prii, steve
orbit identity show linus
orbit identity show lamport
orbit identity show prii
orbit identity show steve
ls orbit-core/assets/identities/
# Should show exactly 4 files
cargo build --workspace
```