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

## Reference: `orbit-tools`

The canonical example. The `Tool` trait at `crates/orbit-tools/src/lib.rs:225`:

```rust
pub trait Tool: Send + Sync {
    fn schema(&self) -> ToolSchema;
    fn execute(&self, ctx: &ToolContext, input: Value) -> Result<Value, OrbitError>;
}
```

The registry at `crates/orbit-tools/src/registry.rs:10` stores `Arc<dyn Tool>` keyed by `ToolSchema::name` and exposes `execute(name, ctx, input)`. Adding a new tool means writing a struct, `impl Tool`, and registering in `builtin::register_builtins` — the dispatcher does not change.

Concrete commands (one struct per file, each self-contained):

- **Process exec** — `crates/orbit-tools/src/builtin/proc/which.rs:7`, `crates/orbit-tools/src/builtin/proc/spawn.rs`
- **ADR lifecycle** — `crates/orbit-tools/src/builtin/orbit/adr/{add,list,update,show,supersede}.rs`
- **Review threads** — `crates/orbit-tools/src/builtin/orbit/review_thread/{add,list,resolve}.rs`
- **Pipeline** — `crates/orbit-tools/src/builtin/orbit/pipeline/{invoke,wait}.rs`
- **Groundhog checkpoints** — `crates/orbit-tools/src/builtin/orbit/groundhog/{checkpoint_success,checkpoint_failure,checkpoint_deviate,side_effect}.rs`

The smallest illustrative implementation is `ProcWhichTool` at `crates/orbit-tools/src/builtin/proc/which.rs:7` — zero fields, a `schema()` declaring its one parameter, and an `execute()` that shells out to `which`. Read it first if the trait surface above is too abstract.
