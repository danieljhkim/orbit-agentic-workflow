# Bind implement_change execution to task workspace and pipeline branch

## Goal
Ensure agent-driven implementation steps run in the intended task workspace and on the intended branch created earlier in the pipeline.

## Scope
- agent execution context composition
- tool execution cwd/repo binding
- `implement_change` input/runtime contract
- regression coverage for branch/workspace drift

## Work items
1. Trace how agent-side tool calls receive cwd and repo context today.
2. Decide the canonical source of implementation execution context: task workspace, pipeline-created branch, or both.
3. Thread that context explicitly into agent tool execution instead of relying on ambient cwd.
4. Make `implement_change` and any related tools/activities enforce the intended workspace and branch.
5. Add regression tests that prove a job-created branch is the branch used for implementation and commit creation.

## Done when
- `implement_change` no longer depends on ambient agent cwd
- task implementation commits land on the pipeline branch, not an unrelated checkout
- the execution context is explicit in code and covered by tests