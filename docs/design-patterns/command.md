# Command Pattern

Package a discrete action as a struct that exposes a uniform `execute` method. The shape:

```rust
trait Command {
    fn schema(&self) -> Schema;
    fn execute(&self, ctx: &Ctx, input: Input) -> Result<Output, Error>;
}
```

Each `impl Command for FooAction` carries any state the action needs and encapsulates its logic. A registry holds `Arc<dyn Command>` keyed by name and dispatches by name at runtime — callers don't know which action they're invoking, they look it up.

## When to reach for it

- **Heterogeneous actions behind a uniform call site.** A dispatcher (CLI subcommand router, RPC server, agent tool host) needs to invoke any of N actions without knowing which.
- **Open registration.** New actions are added by registering an `impl`, not by editing a central `match`.
- **Per-action state and metadata co-located with logic.** Schema, parameter declarations, embedded clients, paths — all live on the action struct.

## When NOT to

- **Stateless + closed registry.** A `fn(Input) -> Result<Output>` stored in a `HashMap` or matched in an `enum` is simpler — no trait objects, no `Send + Sync` boilerplate, no `Arc<dyn ...>`. See `crates/orbit-common/src/migration/mod.rs:21`: `Step` is `fn(Value) -> Result<Value, OrbitError>` because migration steps carry no state and the chain is built once at module load.
- **Exactly one variant.** A free function is enough; the trait is overhead.
- **You think you need rollback.** "Encode an action as an object so you can undo it" is a textbook Command motivation, but in this codebase actions are almost always non-reversibly lossy (dropped fields, NOT NULL additions, side effects). Forward-fix migrations / forward-fix commits instead. See `docs/design/task-artifacts/4_decisions.md` ADR-008 for the reasoning.

## The `Tool` trait

The codebase's Command surface is the `Tool` trait at `crates/orbit-tools/src/lib.rs:225`:

```rust
pub trait Tool: Send + Sync {
    fn schema(&self) -> ToolSchema;
    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError>;
}
```

The registry at `crates/orbit-tools/src/registry.rs:10` stores `Arc<dyn Tool>` keyed by `ToolSchema::name` and exposes `execute(name, ctx, input)`. Adding a new tool means writing a struct, `impl Tool`, and registering in `builtin::register_builtins` — the dispatcher does not change.

The three references below cover the distinct command shapes in this codebase.

## Reference: self-contained command (`ProcWhichTool`)

The simplest shape. Zero fields; `execute()` parses input, does the work inline, returns JSON. From `crates/orbit-tools/src/builtin/proc/which.rs:7`:

```rust
pub struct ProcWhichTool;

impl Tool for ProcWhichTool {
    fn schema(&self) -> ToolSchema { /* name, description, one required string param */ }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let command = input.get("command").and_then(Value::as_str)
            .ok_or_else(|| OrbitError::InvalidInput("missing `command`".into()))?;
        let result = run_process(&ExecRequest { /* ... */ }, &NoSandbox)?;
        Ok(json!({ "command": command, "path": result.stdout.trim(), "found": result.success }))
    }
}
```

Patterns to copy:

- **Unit struct for a stateless command.** `Arc<ProcWhichTool>` in the registry has zero per-instance cost; no `new()` needed.
- **`get().and_then(Value::as_str).ok_or_else(InvalidInput)`** is the idiom for required string params. The shared `super::required_string` helper wraps this.

Use this shape when the command is self-contained — no runtime, no host, just input → output.

## Reference: host-action dispatcher (`OrbitPipelineInvokeTool`)

The most common shape in the `orbit.*` namespace. The struct declares the schema; execution delegates to an `OrbitToolHost` via an action enum. From `crates/orbit-tools/src/builtin/orbit/pipeline/invoke.rs:7`:

```rust
pub struct OrbitPipelineInvokeTool;

impl Tool for OrbitPipelineInvokeTool {
    fn schema(&self) -> ToolSchema { /* params + identity_params() */ }

    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        super::super::execute_host_action(ctx, input, OrbitBuiltinAction::PipelineInvoke)
    }
}
```

`execute_host_action` (`orbit/mod.rs:204`) resolves the caller's identity, requires a host on the context, and forwards `(action, input, agent, model, reservation_owner)` into the runtime.

Patterns to copy:

- **Schema lives here; logic lives in the host.** `orbit-tools` declares the agent-facing API surface; the implementation of `PipelineInvoke` lives in `orbit-core`. This keeps `orbit-tools` free of runtime / store dependencies.
- **New tools of this kind land in three places.** New struct in `orbit/<area>/<verb>.rs`, new variant in `OrbitBuiltinAction`, new match arm in the host's `execute()`. The dispatcher is untouched.

Use this shape when the real work needs the runtime (task store, registry, sandboxed exec).

## Reference: compatibility shim (`OrbitGraphHistoryTool`)

A tool whose entire `execute()` is a structured deprecation error. From `crates/orbit-tools/src/builtin/orbit/graph_history.rs:10`:

```rust
impl Tool for OrbitGraphHistoryTool {
    fn schema(&self) -> ToolSchema { /* keeps the old name + params */ }

    fn execute(&self, _ctx: &ToolContext, input: Value) -> Result<Value, OrbitError> {
        let selector_str = super::required_string(&input, &["selector"], "selector")?;
        let _: Selector = selector_str.parse()
            .map_err(|error| OrbitError::InvalidInput(format!("{error}")))?;
        Err(OrbitError::InvalidInput(REMOVED_GRAPH_HISTORY_MESSAGE.to_string()))
    }
}
```

Patterns to copy:

- **Keep the schema; replace the body.** Agents discover tools by name. A removed tool that vanishes leaves a worse trail than one that returns a redirect explaining the replacement.
- **Validate inputs before returning the deprecation error.** Malformed callers get a real validation message ("invalid selector"), not the deprecation message — easier to diagnose.

Use this shape when removing a tool. Pair it with an explanatory constant (here, `REMOVED_GRAPH_HISTORY_MESSAGE` exported from the workflow that *used to* implement the operation) so the redirect text lives next to the replacement guidance.
