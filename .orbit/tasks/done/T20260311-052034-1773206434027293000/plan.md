# GitHub CLI Orbit Tools Plan

**Goal:** Add a narrow typed set of GitHub-oriented Orbit tools that covers the documented PR workflow without exposing arbitrary `gh` execution.
**Scope:** Built-in tool implementation in `orbit-tools`, any supporting schema/registration updates, and focused tests for command construction and validation.
**Assumptions:** The GitHub CLI is installed in environments where these tools run. Authentication is handled externally; Orbit's role is to surface explicit operations and fail clearly when auth is missing.
**Risks:** If the tool surface becomes too permissive, it undermines Orbit's policy model. If the outputs remain too raw, agents will need brittle text parsing.

## Task 1: Define the GitHub tool surface

**Files:**
- Reference: `gh_commands.md`
- Modify as needed: `orbit-tools/src/builtin/mod.rs`
- Add: `orbit-tools/src/builtin/github/...`

**Steps:**
1. Translate the minimal command list in `gh_commands.md` into Orbit tool names and typed input/output contracts.
2. Decide which operations are mandatory in v1 versus optional (`github.pr.checks` likely in; `github.pr.edit` optional).
3. Keep the scope limited to the documented workflow.

**Done When:**
- There is a concrete list of GitHub tools and expected parameters.
- The tool names match Orbit conventions and avoid raw command passthrough.

## Task 2: Implement built-in GitHub tools in orbit-tools

**Files:**
- Add: `orbit-tools/src/builtin/github/mod.rs`
- Add: tool files under `orbit-tools/src/builtin/github/`
- Modify: `orbit-tools/src/builtin/mod.rs`
- Modify as needed: `orbit-tools/src/lib.rs`

**Steps:**
1. Implement typed wrappers for auth, repo view, and PR lifecycle commands.
2. Use `gh` under the hood with only the required flags for each tool.
3. Return structured JSON and clear Orbit errors for missing auth, bad inputs, or CLI failures.
4. Keep implementation narrow and composable.

**Done When:**
- The documented GitHub operations exist as built-in Orbit tools.
- The implementation stays command-specific rather than generic.

## Task 3: Add tests for registration, validation, and command behavior

**Files:**
- Modify: `orbit-tools/src/lib.rs`
- Add focused tests near the new GitHub tool modules

**Steps:**
1. Verify the new tools are registered in the built-in registry.
2. Add tests for missing required parameters and invalid inputs.
3. Add command-behavior tests using a controlled fake `gh` binary/script so tests do not depend on live GitHub auth.
4. Verify structured output for representative success and failure cases.

**Done When:**
- The GitHub tools are covered by deterministic tests.
- No live GitHub dependency is required in CI for the tool tests.

## Task 4: Document and expose the tool set cleanly

**Files:**
- Modify: `gh_commands.md` if needed for final naming alignment
- Modify any relevant Orbit docs or skills that should reference the new tool names

**Steps:**
1. Align the tool names in docs with the implemented Orbit tool surface.
2. Clarify that Orbit wraps only the approved `gh` commands rather than arbitrary CLI execution.
3. Note any environment prerequisites such as `gh auth status` expectations.

**Done When:**
- The documented command surface and implemented tool names match.
- Future agents can discover and use the tools without guessing.

## Final Verification
- `cargo test -p orbit-tools`
- Add focused tests for each GitHub tool module
- Manual: with authenticated `gh`, validate a representative happy path such as `github.auth.status` and `github.repo.view`