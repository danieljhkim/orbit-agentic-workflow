Clarified the live runtime config path terminology without rewriting unrelated historical task artifacts.

Summary of changes:
- updated the `watch` config removal error in `orbit-core/src/config/persistence.rs` to name the runtime config file explicitly as `.orbit/config.toml` in a repo-local workspace, or `<data_root>/config.toml` generically
- added a doc comment on `OrbitRuntime::config_path()` in `orbit-core/src/runtime/mod.rs` to make the same contract explicit for maintainers
- updated the active task record so its title/description/plan no longer imply a separate file literally named `orbit.toml`

Scope decision:
- intentionally did not rewrite older done/archived task bundles that still mention `orbit.toml`; those are historical artifacts, not live product guidance
- confirmed there was no current skill template or live product doc using `orbit.toml` as the canonical filename

Validation:
- `rg -n "orbit\.toml|config\.toml" .orbit orbit-core orbit-cli`
- `cargo test -p orbit-core`