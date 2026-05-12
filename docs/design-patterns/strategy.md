# Strategy Pattern

Define a family of interchangeable algorithms behind a common trait. Each `impl` is a complete way to perform the *same* operation; the caller picks one based on a key derived from its input (file's language, activity's spec type) and invokes it without caring which:

```rust
trait Strategy {
    fn applies_to(&self) -> Key;
    fn run(&self, input: Input) -> Output;
}
```

A registry holds the candidates and matches `applies_to` against the input-derived key.

## When to reach for it

- **Same logical operation, multiple algorithms.** Parsing source is one operation; the algorithm differs per language. Executing an activity is one operation; the mechanics differ per `spec_type`. Callers want `extract(file)` / `execute(activity)`, not a sprawling `match`.
- **Selection is driven by data, not by call site.** The caller derives a key from the input and looks up; it does not know which impl ran.
- **Strategies are roughly fungible.** Same output shape (or close enough that one calling-side code path handles the variance).

## When NOT to

- **The "strategies" do different things.** If `impl`s return semantically different shapes, or the caller has to branch on which one ran, it's Command, not Strategy. Each `impl Tool` in `crates/orbit-tools/` is a different *operation* keyed by name, not a different *algorithm* for one operation — see `docs/design-patterns/command.md`.
- **Single impl, planned single impl.** A trait with one `impl` is just an abstraction boundary. Don't pay the `Box<dyn ...>` cost for an interface you'll never swap.
- **A few variants known up front.** `match kind { Yaml => …, Json => … }` is clearer than a trait + registry when the set is small, closed, and trivial.

## Reference: linear-scan registry (`FileExtractor`)

The canonical example. One operation — *extract structural anchors from a source file* — implemented per language and per file kind. From `crates/orbit-knowledge/src/extract/mod.rs:53`:

```rust
pub trait FileExtractor: Send + Sync {
    fn file_kind(&self) -> FileKind;
    fn extract(&self, source: &str) -> ExtractionResult;
}

pub struct ExtractorRegistry {
    extractors: Vec<Box<dyn FileExtractor>>,
}

impl ExtractorRegistry {
    pub fn get(&self, kind: FileKind) -> Option<&dyn FileExtractor> {
        self.extractors.iter().find(|e| e.file_kind() == kind).map(|e| e.as_ref())
    }
}
```

Concrete strategies live in sibling modules — `rust.rs`, `python.rs`, `typescript.rs`, …, `markdown.rs`, `config.rs`, `table.rs`. The pipeline derives `FileKind` from the file, asks the registry, calls `extract(source)`, and never branches on language.

Patterns to copy:

- **`applies_to()` predicate on the trait.** Each strategy declares its own key. New strategies need no change to the registry or any central `match` — just `Box::new(NewExtractor)` in the constructor.
- **Linear scan is fine for small, bounded candidate sets.** ~15 extractors with `==` comparison beats a `HashMap`'s hashing overhead.

Use this shape when strategies are known at compile time and selection is a simple equality check on a small enum.

## Reference: hash-map registry, string keys (`ActivityExecutor`)

One operation — *execute one attempt of an activity* — implemented per execution mechanic. From `crates/orbit-engine/src/executor/traits.rs:17`:

```rust
pub trait ActivityExecutor: Send + Sync {
    fn spec_type(&self) -> &str;
    fn execute(&self, host: ExecutorHost<'_>, execution: &ExecutionContext) -> AttemptOutcome;
}

pub struct ActivityExecutorRegistry {
    executors: HashMap<String, Box<dyn ActivityExecutor>>,
}

impl ActivityExecutorRegistry {
    pub fn get(&self, spec_type: &str) -> Option<&dyn ActivityExecutor> {
        self.executors.get(spec_type).map(Box::as_ref)
    }
}
```

Built-ins (`CliCommandExecutor`, `DirectAgentExecutor`, `AutomationExecutor`, `OrbitToolCallExecutor`) are registered in `register_builtins`. YAML-loaded definitions (`load_from_defs`) can override built-ins by name. The retry loop in `activity_runner` calls `registry.get(spec_type).execute(...)` without knowing which mechanic actually runs.

Patterns to copy:

- **String keys when selection is data-driven.** Activity YAML carries `spec_type: "cli_command"`. A string-keyed map lets externally-defined activities pick strategies that were registered after the registry was constructed.
- **Open registration via `register_named`.** New `spec_type`s can be added at runtime from config, without touching the registry struct or any central enum.

Use this shape when the strategy set is open — config-loaded, plugin-loaded, anything beyond a closed enum.

---

**Strategy vs Command in this codebase.** Both use `Box<dyn Trait>` and a registry. The difference is *what varies*. `FileExtractor` varies *how* to extract — every impl extracts; that's Strategy. `Tool` varies *what* to do — `which`, `adr_add`, and `pipeline_invoke` are different operations sharing only a calling convention; that's Command.
