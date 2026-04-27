use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use orbit_core::OrbitError;
use serde_json::{Map as JsonMap, Value as JsonValue};
use toml::{Table as TomlTable, Value as TomlValue};

use super::{ORBIT_MCP_SERVER_ID, safe_mcp_tool_names};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ScopeArg {
    /// Write to user-level config (~/.claude, ~/.codex, ~/.gemini). Default.
    #[default]
    Home,
    /// Write to repo-local config (.mcp.json, .codex/, .gemini/).
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum McpProvider {
    Claude,
    Codex,
    Gemini,
}

impl McpProvider {
    fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum McpAction {
    Init,
    Remove,
}

impl McpAction {
    fn label(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Remove => "remove",
        }
    }
}

#[derive(Args, Debug, Clone, Default)]
pub struct ProviderSelectionArgs {
    /// Use auto-detected provider targets for the current workspace.
    #[arg(long)]
    pub auto: bool,
    /// Target Claude Code integration only.
    #[arg(long)]
    pub claude: bool,
    /// Target Codex CLI integration only.
    #[arg(long)]
    pub codex: bool,
    /// Target Gemini CLI integration only.
    #[arg(long)]
    pub gemini: bool,
    /// Target all supported MCP client integrations.
    #[arg(long)]
    pub all: bool,
}

impl ProviderSelectionArgs {
    fn resolve_mode(&self) -> Result<ProviderSelectionMode, OrbitError> {
        if self.auto && (self.claude || self.codex || self.gemini || self.all) {
            return Err(OrbitError::InvalidInput(
                "--auto cannot be combined with --claude, --codex, --gemini, or --all".to_string(),
            ));
        }
        if self.all && (self.claude || self.codex || self.gemini) {
            return Err(OrbitError::InvalidInput(
                "--all cannot be combined with --claude, --codex, or --gemini".to_string(),
            ));
        }
        if self.auto || (!self.claude && !self.codex && !self.gemini && !self.all) {
            return Ok(ProviderSelectionMode::Auto);
        }
        if self.all {
            return Ok(ProviderSelectionMode::Explicit(vec![
                McpProvider::Claude,
                McpProvider::Codex,
                McpProvider::Gemini,
            ]));
        }

        let mut providers = Vec::new();
        if self.claude {
            providers.push(McpProvider::Claude);
        }
        if self.codex {
            providers.push(McpProvider::Codex);
        }
        if self.gemini {
            providers.push(McpProvider::Gemini);
        }
        Ok(ProviderSelectionMode::Explicit(providers))
    }
}

enum ProviderSelectionMode {
    Auto,
    Explicit(Vec<McpProvider>),
}

#[derive(Args, Debug, Clone, Default)]
#[command(about = "Initialize MCP client integration for the current workspace")]
pub struct InitArgs {
    #[command(flatten)]
    pub providers: ProviderSelectionArgs,
    /// Scope for written config files (home: user-level, workspace: repo-local).
    #[arg(long, value_enum, default_value_t = ScopeArg::Home)]
    pub scope: ScopeArg,
}

impl InitArgs {
    pub fn execute_without_runtime(self, root_override: Option<&Path>) -> Result<(), OrbitError> {
        let layout = resolve_workspace_layout(root_override)?;
        let providers = run_action(
            McpAction::Init,
            &layout.repo_root,
            &layout.orbit_root,
            self.providers.resolve_mode()?,
            env_home_dir(),
            self.scope,
        )?;
        print_action_summary(McpAction::Init, &providers);
        Ok(())
    }
}

#[derive(Args, Debug, Clone, Default)]
#[command(about = "Remove MCP client integration for the current workspace")]
pub struct RemoveArgs {
    #[command(flatten)]
    pub providers: ProviderSelectionArgs,
    /// Scope for config files to remove (home: user-level, workspace: repo-local).
    #[arg(long, value_enum, default_value_t = ScopeArg::Home)]
    pub scope: ScopeArg,
}

impl RemoveArgs {
    pub fn execute_without_runtime(self, root_override: Option<&Path>) -> Result<(), OrbitError> {
        let layout = resolve_workspace_layout(root_override)?;
        let providers = run_action(
            McpAction::Remove,
            &layout.repo_root,
            &layout.orbit_root,
            self.providers.resolve_mode()?,
            env_home_dir(),
            self.scope,
        )?;
        print_action_summary(McpAction::Remove, &providers);
        Ok(())
    }
}

pub(crate) fn init_auto_for_workspace(
    repo_root: &Path,
    orbit_root: &Path,
) -> Result<Vec<String>, OrbitError> {
    // `orbit workspace init` is a per-workspace setup, so its auto-MCP path
    // writes repo-local files. `orbit mcp init` separately defaults to home
    // scope for users who want a single global registration.
    run_action(
        McpAction::Init,
        repo_root,
        orbit_root,
        ProviderSelectionMode::Auto,
        env_home_dir(),
        ScopeArg::Workspace,
    )
    .map(|providers| {
        providers
            .into_iter()
            .map(|provider| provider.label().to_string())
            .collect()
    })
}

#[derive(Debug, Clone)]
struct WorkspaceLayout {
    repo_root: PathBuf,
    orbit_root: PathBuf,
}

fn resolve_workspace_layout(root_override: Option<&Path>) -> Result<WorkspaceLayout, OrbitError> {
    if let Some(orbit_root) = root_override {
        return Ok(WorkspaceLayout {
            repo_root: orbit_root.parent().unwrap_or(orbit_root).to_path_buf(),
            orbit_root: orbit_root.to_path_buf(),
        });
    }

    let cwd = env::current_dir().map_err(|err| OrbitError::Io(err.to_string()))?;
    if cwd.file_name().is_some_and(|name| name == ".orbit") && cwd.is_dir() {
        return Ok(WorkspaceLayout {
            repo_root: cwd.parent().unwrap_or(&cwd).to_path_buf(),
            orbit_root: cwd,
        });
    }

    for ancestor in cwd.ancestors() {
        let orbit_root = ancestor.join(".orbit");
        if orbit_root.is_dir() {
            return Ok(WorkspaceLayout {
                repo_root: ancestor.to_path_buf(),
                orbit_root,
            });
        }
    }

    Err(OrbitError::InvalidInput(
        "current directory is not inside an initialized Orbit workspace; run `orbit workspace init` first or pass `--root <path/to/.orbit>`".to_string(),
    ))
}

fn env_home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("USERPROFILE")
                .filter(|value| !value.is_empty())
                .map(PathBuf::from)
        })
}

fn run_action(
    action: McpAction,
    repo_root: &Path,
    orbit_root: &Path,
    selection: ProviderSelectionMode,
    home_dir: Option<PathBuf>,
    scope: ScopeArg,
) -> Result<Vec<McpProvider>, OrbitError> {
    let providers = resolve_providers(selection, repo_root, home_dir.as_deref());
    for provider in &providers {
        let target = ConfigTarget::resolve(scope, provider, repo_root, home_dir.as_deref())?;
        match (action, provider) {
            (McpAction::Init, McpProvider::Claude) => apply_claude_init(&target)?,
            (McpAction::Remove, McpProvider::Claude) => apply_claude_remove(&target)?,
            (McpAction::Init, McpProvider::Codex) => apply_codex_init(&target)?,
            (McpAction::Remove, McpProvider::Codex) => apply_codex_remove(&target)?,
            (McpAction::Init, McpProvider::Gemini) => apply_gemini_init(&target)?,
            (McpAction::Remove, McpProvider::Gemini) => apply_gemini_remove(&target)?,
        }
    }
    let _ = orbit_root;
    Ok(providers)
}

/// Resolved file targets for a single provider+scope.
///
/// Each provider has at most two writable files: the MCP server registry
/// (`mcp_path`) and an optional permissions/settings file (`settings_path`,
/// only used by Claude today). Scope determines whether they live in HOME
/// or in the repo.
struct ConfigTarget {
    mcp_path: PathBuf,
    settings_path: Option<PathBuf>,
}

impl ConfigTarget {
    fn resolve(
        scope: ScopeArg,
        provider: &McpProvider,
        repo_root: &Path,
        home_dir: Option<&Path>,
    ) -> Result<Self, OrbitError> {
        match (scope, provider) {
            (ScopeArg::Home, McpProvider::Claude) => {
                let home = require_home_dir(home_dir)?;
                Ok(Self {
                    mcp_path: home.join(".claude").join(".mcp.json"),
                    settings_path: Some(home.join(".claude").join("settings.json")),
                })
            }
            (ScopeArg::Workspace, McpProvider::Claude) => Ok(Self {
                mcp_path: repo_root.join(".mcp.json"),
                settings_path: Some(repo_root.join(".claude").join("settings.json")),
            }),
            (ScopeArg::Home, McpProvider::Codex) => {
                let home = require_home_dir(home_dir)?;
                Ok(Self {
                    mcp_path: home.join(".codex").join("config.toml"),
                    settings_path: None,
                })
            }
            (ScopeArg::Workspace, McpProvider::Codex) => Ok(Self {
                mcp_path: repo_root.join(".codex").join("config.toml"),
                settings_path: None,
            }),
            (ScopeArg::Home, McpProvider::Gemini) => {
                let home = require_home_dir(home_dir)?;
                Ok(Self {
                    mcp_path: home.join(".gemini").join("settings.json"),
                    settings_path: None,
                })
            }
            (ScopeArg::Workspace, McpProvider::Gemini) => Ok(Self {
                mcp_path: repo_root.join(".gemini").join("settings.json"),
                settings_path: None,
            }),
        }
    }
}

fn require_home_dir(home_dir: Option<&Path>) -> Result<&Path, OrbitError> {
    home_dir.ok_or_else(|| {
        OrbitError::InvalidInput(
            "cannot resolve HOME/USERPROFILE for MCP integration files".to_string(),
        )
    })
}

fn resolve_providers(
    selection: ProviderSelectionMode,
    repo_root: &Path,
    home_dir: Option<&Path>,
) -> Vec<McpProvider> {
    match selection {
        ProviderSelectionMode::Explicit(providers) => providers,
        ProviderSelectionMode::Auto => auto_detected_providers(repo_root, home_dir),
    }
}

fn auto_detected_providers(repo_root: &Path, home_dir: Option<&Path>) -> Vec<McpProvider> {
    let mut providers = Vec::new();
    if repo_root.join(".claude").is_dir() {
        providers.push(McpProvider::Claude);
    }
    if home_dir
        .map(|home| home.join(".codex").join("config.toml").is_file())
        .unwrap_or(false)
    {
        providers.push(McpProvider::Codex);
    }
    let gemini_repo = repo_root.join(".gemini").is_dir();
    let gemini_home = home_dir
        .map(|home| home.join(".gemini").join("settings.json").is_file())
        .unwrap_or(false);
    if gemini_repo || gemini_home {
        providers.push(McpProvider::Gemini);
    }
    providers
}

fn print_action_summary(action: McpAction, providers: &[McpProvider]) {
    if providers.is_empty() {
        println!("mcp {}: no providers selected", action.label());
        return;
    }

    let labels = providers
        .iter()
        .map(|provider| provider.label())
        .collect::<Vec<_>>()
        .join(", ");
    println!("mcp {}: {}", action.label(), labels);
}

fn apply_claude_init(target: &ConfigTarget) -> Result<(), OrbitError> {
    let mut root = load_json_object(&target.mcp_path)?;
    let mcp_servers = ensure_json_object(&mut root, "mcpServers")?;
    mcp_servers.insert(ORBIT_MCP_SERVER_ID.to_string(), claude_mcp_server_value());
    write_json_object(&target.mcp_path, &root)?;

    if let Some(settings_path) = &target.settings_path {
        let mut settings = load_json_object(settings_path)?;
        let permissions = ensure_json_object(&mut settings, "permissions")?;
        let allow = ensure_json_string_array(permissions, "allow")?;
        merge_unique_strings(allow, claude_safe_permissions());
        write_json_object(settings_path, &settings)?;
    }
    Ok(())
}

fn apply_claude_remove(target: &ConfigTarget) -> Result<(), OrbitError> {
    let mut root = load_json_object(&target.mcp_path)?;
    if let Some(mcp_servers) = root
        .get_mut("mcpServers")
        .and_then(JsonValue::as_object_mut)
    {
        mcp_servers.remove(ORBIT_MCP_SERVER_ID);
        if mcp_servers.is_empty() {
            root.remove("mcpServers");
        }
    }
    write_or_remove_json_object(&target.mcp_path, &root)?;

    if let Some(settings_path) = &target.settings_path {
        let mut settings = load_json_object(settings_path)?;
        let mut remove_keys = Vec::new();
        if let Some(permissions) = settings
            .get_mut("permissions")
            .and_then(JsonValue::as_object_mut)
        {
            if let Some(allow) = permissions
                .get_mut("allow")
                .and_then(JsonValue::as_array_mut)
            {
                remove_known_strings(allow, &claude_safe_permissions());
                if allow.is_empty() {
                    permissions.remove("allow");
                }
            }
            if permissions.is_empty() {
                remove_keys.push("permissions".to_string());
            }
        }
        for key in remove_keys {
            settings.remove(&key);
        }
        write_or_remove_json_object(settings_path, &settings)?;
    }
    Ok(())
}

fn apply_codex_init(target: &ConfigTarget) -> Result<(), OrbitError> {
    let mut root = load_toml_table(&target.mcp_path)?;
    let mcp_servers = ensure_toml_table(&mut root, "mcp_servers")?;
    mcp_servers.insert(
        ORBIT_MCP_SERVER_ID.to_string(),
        TomlValue::Table(codex_mcp_server_table()),
    );
    write_toml_table(&target.mcp_path, &root)
}

fn apply_codex_remove(target: &ConfigTarget) -> Result<(), OrbitError> {
    let mut root = load_toml_table(&target.mcp_path)?;
    if let Some(mcp_servers) = root
        .get_mut("mcp_servers")
        .and_then(TomlValue::as_table_mut)
    {
        mcp_servers.remove(ORBIT_MCP_SERVER_ID);
        if mcp_servers.is_empty() {
            root.remove("mcp_servers");
        }
    }
    write_or_remove_toml_table(&target.mcp_path, &root)
}

fn apply_gemini_init(target: &ConfigTarget) -> Result<(), OrbitError> {
    let mut settings = load_json_object(&target.mcp_path)?;
    let mcp_servers = ensure_json_object(&mut settings, "mcpServers")?;
    mcp_servers.insert(ORBIT_MCP_SERVER_ID.to_string(), gemini_mcp_server_value());
    write_json_object(&target.mcp_path, &settings)
}

fn apply_gemini_remove(target: &ConfigTarget) -> Result<(), OrbitError> {
    let mut settings = load_json_object(&target.mcp_path)?;
    if let Some(mcp_servers) = settings
        .get_mut("mcpServers")
        .and_then(JsonValue::as_object_mut)
    {
        mcp_servers.remove(ORBIT_MCP_SERVER_ID);
        if mcp_servers.is_empty() {
            settings.remove("mcpServers");
        }
    }
    write_or_remove_json_object(&target.mcp_path, &settings)
}

fn server_args() -> Vec<String> {
    vec!["mcp".to_string(), "serve".to_string()]
}

fn claude_mcp_server_value() -> JsonValue {
    JsonValue::Object(JsonMap::from_iter([
        (
            "command".to_string(),
            JsonValue::String("orbit".to_string()),
        ),
        (
            "args".to_string(),
            JsonValue::Array(server_args().into_iter().map(JsonValue::String).collect()),
        ),
    ]))
}

fn gemini_mcp_server_value() -> JsonValue {
    JsonValue::Object(JsonMap::from_iter([
        (
            "command".to_string(),
            JsonValue::String("orbit".to_string()),
        ),
        (
            "args".to_string(),
            JsonValue::Array(server_args().into_iter().map(JsonValue::String).collect()),
        ),
    ]))
}

fn codex_mcp_server_table() -> TomlTable {
    TomlTable::from_iter([
        (
            "command".to_string(),
            TomlValue::String("orbit".to_string()),
        ),
        (
            "args".to_string(),
            TomlValue::Array(server_args().into_iter().map(TomlValue::String).collect()),
        ),
        ("enabled".to_string(), TomlValue::Boolean(true)),
    ])
}

fn claude_safe_permissions() -> Vec<String> {
    safe_mcp_tool_names()
        .into_iter()
        .map(claude_permission_name)
        .collect()
}

fn claude_permission_name(tool_name: &str) -> String {
    format!("mcp__plugin_orbit_orbit__{}", tool_name.replace('.', "_"))
}

fn merge_unique_strings(existing: &mut Vec<JsonValue>, values: Vec<String>) {
    let mut seen = existing
        .iter()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect::<std::collections::BTreeSet<_>>();

    for value in values {
        if seen.insert(value.clone()) {
            existing.push(JsonValue::String(value));
        }
    }
}

fn remove_known_strings(existing: &mut Vec<JsonValue>, values: &[String]) {
    existing.retain(|value| {
        value
            .as_str()
            .map(|candidate| !values.iter().any(|item| item == candidate))
            .unwrap_or(true)
    });
}

fn load_json_object(path: &Path) -> Result<JsonMap<String, JsonValue>, OrbitError> {
    if !path.exists() {
        return Ok(JsonMap::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|err| OrbitError::Io(format!("failed to read '{}': {err}", path.display())))?;
    if raw.trim().is_empty() {
        return Ok(JsonMap::new());
    }

    let value: JsonValue = serde_json::from_str(&raw).map_err(|err| {
        OrbitError::InvalidInput(format!("invalid JSON '{}': {err}", path.display()))
    })?;
    value.as_object().cloned().ok_or_else(|| {
        OrbitError::InvalidInput(format!(
            "expected top-level JSON object in '{}'",
            path.display()
        ))
    })
}

fn write_json_object(path: &Path, root: &JsonMap<String, JsonValue>) -> Result<(), OrbitError> {
    let parent = path.parent().ok_or_else(|| {
        OrbitError::InvalidInput(format!("path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)
        .map_err(|err| OrbitError::Io(format!("failed to create '{}': {err}", parent.display())))?;
    let mut rendered =
        serde_json::to_string_pretty(&JsonValue::Object(root.clone())).map_err(|err| {
            OrbitError::Execution(format!("serialize JSON '{}': {err}", path.display()))
        })?;
    rendered.push('\n');
    fs::write(path, rendered)
        .map_err(|err| OrbitError::Io(format!("failed to write '{}': {err}", path.display())))
}

fn write_or_remove_json_object(
    path: &Path,
    root: &JsonMap<String, JsonValue>,
) -> Result<(), OrbitError> {
    if root.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|err| {
                OrbitError::Io(format!("failed to remove '{}': {err}", path.display()))
            })?;
        }
        return Ok(());
    }
    write_json_object(path, root)
}

fn ensure_json_object<'a>(
    root: &'a mut JsonMap<String, JsonValue>,
    key: &str,
) -> Result<&'a mut JsonMap<String, JsonValue>, OrbitError> {
    let value = root
        .entry(key.to_string())
        .or_insert_with(|| JsonValue::Object(JsonMap::new()));
    value
        .as_object_mut()
        .ok_or_else(|| OrbitError::InvalidInput(format!("expected '{key}' to be a JSON object")))
}

fn ensure_json_string_array<'a>(
    root: &'a mut JsonMap<String, JsonValue>,
    key: &str,
) -> Result<&'a mut Vec<JsonValue>, OrbitError> {
    let value = root
        .entry(key.to_string())
        .or_insert_with(|| JsonValue::Array(Vec::new()));
    let array = value
        .as_array_mut()
        .ok_or_else(|| OrbitError::InvalidInput(format!("expected '{key}' to be a JSON array")))?;
    if array.iter().any(|item| !item.is_string()) {
        return Err(OrbitError::InvalidInput(format!(
            "expected '{key}' to contain only strings"
        )));
    }
    Ok(array)
}

fn load_toml_table(path: &Path) -> Result<TomlTable, OrbitError> {
    if !path.exists() {
        return Ok(TomlTable::new());
    }

    let raw = fs::read_to_string(path)
        .map_err(|err| OrbitError::Io(format!("failed to read '{}': {err}", path.display())))?;
    if raw.trim().is_empty() {
        return Ok(TomlTable::new());
    }

    let value: TomlValue = toml::from_str(&raw).map_err(|err| {
        OrbitError::InvalidInput(format!("invalid TOML '{}': {err}", path.display()))
    })?;
    value.as_table().cloned().ok_or_else(|| {
        OrbitError::InvalidInput(format!(
            "expected top-level TOML table in '{}'",
            path.display()
        ))
    })
}

fn write_toml_table(path: &Path, root: &TomlTable) -> Result<(), OrbitError> {
    let parent = path.parent().ok_or_else(|| {
        OrbitError::InvalidInput(format!("path has no parent: {}", path.display()))
    })?;
    fs::create_dir_all(parent)
        .map_err(|err| OrbitError::Io(format!("failed to create '{}': {err}", parent.display())))?;
    let rendered = toml::to_string_pretty(&TomlValue::Table(root.clone())).map_err(|err| {
        OrbitError::Execution(format!("serialize TOML '{}': {err}", path.display()))
    })?;
    fs::write(path, rendered)
        .map_err(|err| OrbitError::Io(format!("failed to write '{}': {err}", path.display())))
}

fn write_or_remove_toml_table(path: &Path, root: &TomlTable) -> Result<(), OrbitError> {
    if root.is_empty() {
        if path.exists() {
            fs::remove_file(path).map_err(|err| {
                OrbitError::Io(format!("failed to remove '{}': {err}", path.display()))
            })?;
        }
        return Ok(());
    }
    write_toml_table(path, root)
}

fn ensure_toml_table<'a>(
    root: &'a mut TomlTable,
    key: &str,
) -> Result<&'a mut TomlTable, OrbitError> {
    let value = root
        .entry(key.to_string())
        .or_insert_with(|| TomlValue::Table(TomlTable::new()));
    value
        .as_table_mut()
        .ok_or_else(|| OrbitError::InvalidInput(format!("expected '{key}' to be a TOML table")))
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use tempfile::tempdir;

    use super::{
        McpAction, McpProvider, ProviderSelectionArgs, ProviderSelectionMode, ScopeArg,
        auto_detected_providers, claude_mcp_server_value, claude_permission_name,
        codex_mcp_server_table, gemini_mcp_server_value, run_action,
    };

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    #[test]
    fn provider_selection_defaults_to_auto() {
        let args = ProviderSelectionArgs::default();
        assert!(matches!(
            args.resolve_mode().expect("resolve mode"),
            ProviderSelectionMode::Auto
        ));
    }

    #[test]
    fn provider_selection_rejects_conflicting_flags() {
        let args = ProviderSelectionArgs {
            auto: true,
            claude: true,
            codex: false,
            gemini: false,
            all: false,
        };
        assert!(args.resolve_mode().is_err());
    }

    #[test]
    fn provider_selection_all_includes_gemini() {
        let args = ProviderSelectionArgs {
            auto: false,
            claude: false,
            codex: false,
            gemini: false,
            all: true,
        };
        match args.resolve_mode().expect("resolve mode") {
            ProviderSelectionMode::Explicit(providers) => assert_eq!(
                providers,
                vec![McpProvider::Claude, McpProvider::Codex, McpProvider::Gemini]
            ),
            ProviderSelectionMode::Auto => panic!("expected explicit provider set"),
        }
    }

    #[test]
    fn auto_detects_expected_providers() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        std::fs::create_dir_all(repo.path().join(".claude")).expect("create .claude");
        std::fs::create_dir_all(repo.path().join(".gemini")).expect("create .gemini");
        std::fs::create_dir_all(home.path().join(".codex")).expect("create codex dir");
        std::fs::write(
            home.path().join(".codex").join("config.toml"),
            "model = \"gpt-5.4\"\n",
        )
        .expect("write global codex config");

        let providers = auto_detected_providers(repo.path(), Some(home.path()));
        assert_eq!(
            providers,
            vec![McpProvider::Claude, McpProvider::Codex, McpProvider::Gemini]
        );
    }

    #[test]
    fn auto_detects_gemini_from_home_when_repo_lacks_dotgemini() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        std::fs::create_dir_all(home.path().join(".gemini")).expect("create gemini home dir");
        std::fs::write(home.path().join(".gemini").join("settings.json"), "{}\n")
            .expect("write global gemini settings");

        let providers = auto_detected_providers(repo.path(), Some(home.path()));
        assert_eq!(providers, vec![McpProvider::Gemini]);
    }

    #[test]
    fn claude_workspace_scope_init_and_remove_preserve_unrelated_entries() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        std::fs::create_dir_all(repo.path().join(".claude")).expect("create .claude");
        std::fs::write(
            repo.path().join(".mcp.json"),
            "{\n  \"mcpServers\": {\n    \"other\": {\"command\": \"demo\"}\n  }\n}\n",
        )
        .expect("write mcp file");
        std::fs::write(
            repo.path().join(".claude").join("settings.json"),
            "{\n  \"permissions\": {\n    \"allow\": [\"OtherTool\"]\n  },\n  \"theme\": \"light\"\n}\n",
        )
        .expect("write settings");

        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        let providers = run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Claude]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("init claude");
        assert_eq!(providers, vec![McpProvider::Claude]);

        let mcp: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(repo.path().join(".mcp.json")).expect("read mcp"),
        )
        .expect("parse mcp");
        assert!(mcp["mcpServers"]["orbit"].is_object());
        assert!(mcp["mcpServers"]["other"].is_object());
        let args = mcp["mcpServers"]["orbit"]["args"]
            .as_array()
            .expect("args array");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].as_str(), Some("mcp"));
        assert_eq!(args[1].as_str(), Some("serve"));

        let settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(repo.path().join(".claude").join("settings.json"))
                .expect("read settings"),
        )
        .expect("parse settings");
        let allow = settings["permissions"]["allow"]
            .as_array()
            .expect("allow array");
        assert!(allow.iter().any(|item| item == "OtherTool"));
        assert!(
            allow
                .iter()
                .any(|item| item == &claude_permission_name("orbit.task.show"))
        );
        assert_eq!(settings["theme"], "light");

        run_action(
            McpAction::Remove,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Claude]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("remove claude");

        let mcp: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(repo.path().join(".mcp.json")).expect("read mcp"),
        )
        .expect("parse mcp");
        assert!(mcp["mcpServers"]["orbit"].is_null());
        assert!(mcp["mcpServers"]["other"].is_object());
    }

    #[test]
    fn home_scope_writes_to_home_paths_and_skips_repo_files() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![
                McpProvider::Claude,
                McpProvider::Codex,
                McpProvider::Gemini,
            ]),
            Some(home.path().to_path_buf()),
            ScopeArg::Home,
        )
        .expect("init home scope");

        let claude_mcp: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude").join(".mcp.json"))
                .expect("read claude home mcp"),
        )
        .expect("parse claude mcp");
        let claude_args = claude_mcp["mcpServers"]["orbit"]["args"]
            .as_array()
            .expect("claude args");
        assert_eq!(claude_args.len(), 2);
        assert_eq!(claude_args[0].as_str(), Some("mcp"));
        assert_eq!(claude_args[1].as_str(), Some("serve"));
        assert!(claude_mcp["mcpServers"]["orbit"]["cwd"].is_null());

        let claude_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".claude").join("settings.json"))
                .expect("read claude home settings"),
        )
        .expect("parse claude settings");
        let allow = claude_settings["permissions"]["allow"]
            .as_array()
            .expect("allow array");
        assert!(
            allow
                .iter()
                .any(|item| item == &claude_permission_name("orbit.task.show"))
        );

        let codex_config = std::fs::read_to_string(home.path().join(".codex").join("config.toml"))
            .expect("read codex home config");
        let codex_parsed: toml::Value = toml::from_str(&codex_config).expect("parse codex");
        let codex_args = codex_parsed["mcp_servers"]["orbit"]["args"]
            .as_array()
            .expect("codex args");
        assert_eq!(codex_args.len(), 2);
        assert_eq!(codex_args[0].as_str(), Some("mcp"));
        assert_eq!(codex_args[1].as_str(), Some("serve"));
        assert!(codex_parsed["mcp_servers"]["orbit"].get("cwd").is_none());

        let gemini_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".gemini").join("settings.json"))
                .expect("read gemini home settings"),
        )
        .expect("parse gemini");
        let gemini_args = gemini_settings["mcpServers"]["orbit"]["args"]
            .as_array()
            .expect("gemini args");
        assert_eq!(gemini_args.len(), 2);
        assert!(gemini_settings["mcpServers"]["orbit"]["cwd"].is_null());

        // Repo-local files should not have been touched.
        assert!(!repo.path().join(".mcp.json").exists());
        assert!(!repo.path().join(".codex").join("config.toml").exists());
        assert!(!repo.path().join(".gemini").join("settings.json").exists());
        assert!(!repo.path().join(".claude").join("settings.json").exists());
    }

    #[test]
    fn home_scope_remove_strips_only_orbit_entries() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        std::fs::create_dir_all(home.path().join(".codex")).expect("create codex home");
        std::fs::write(
            home.path().join(".codex").join("config.toml"),
            "model = \"gpt-5.4\"\n[mcp_servers.other]\ncommand = \"demo\"\n",
        )
        .expect("write codex config");
        std::fs::create_dir_all(home.path().join(".gemini")).expect("create gemini home");
        std::fs::write(
            home.path().join(".gemini").join("settings.json"),
            "{\n  \"theme\": \"dark\",\n  \"mcpServers\": {\n    \"other\": {\"command\": \"demo\"}\n  }\n}\n",
        )
        .expect("write gemini settings");

        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Codex, McpProvider::Gemini]),
            Some(home.path().to_path_buf()),
            ScopeArg::Home,
        )
        .expect("init home scope");

        run_action(
            McpAction::Remove,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Codex, McpProvider::Gemini]),
            Some(home.path().to_path_buf()),
            ScopeArg::Home,
        )
        .expect("remove home scope");

        let codex_config = std::fs::read_to_string(home.path().join(".codex").join("config.toml"))
            .expect("read codex");
        let codex_parsed: toml::Value = toml::from_str(&codex_config).expect("parse codex");
        assert_eq!(codex_parsed["model"].as_str(), Some("gpt-5.4"));
        assert_eq!(
            codex_parsed["mcp_servers"]["other"]["command"].as_str(),
            Some("demo")
        );
        assert!(
            codex_parsed["mcp_servers"]
                .as_table()
                .and_then(|t| t.get("orbit"))
                .is_none()
        );

        let gemini_settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(home.path().join(".gemini").join("settings.json"))
                .expect("read gemini"),
        )
        .expect("parse gemini");
        assert_eq!(gemini_settings["theme"], "dark");
        assert!(gemini_settings["mcpServers"]["orbit"].is_null());
        assert!(gemini_settings["mcpServers"]["other"].is_object());
    }

    #[test]
    fn codex_workspace_scope_init_and_remove_preserve_unrelated_entries() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        std::fs::create_dir_all(repo.path().join(".codex")).expect("create .codex");
        std::fs::write(
            repo.path().join(".codex").join("config.toml"),
            "model = \"gpt-5.4\"\n[mcp_servers.other]\ncommand = \"demo\"\n",
        )
        .expect("write config");
        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Codex]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("init codex");

        let config = std::fs::read_to_string(repo.path().join(".codex").join("config.toml"))
            .expect("read config");
        let parsed: toml::Value = toml::from_str(&config).expect("parse config");
        assert_eq!(parsed["model"].as_str(), Some("gpt-5.4"));
        assert_eq!(
            parsed["mcp_servers"]["orbit"]["command"].as_str(),
            Some("orbit")
        );
        let args = parsed["mcp_servers"]["orbit"]["args"]
            .as_array()
            .expect("args array");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].as_str(), Some("mcp"));
        assert_eq!(args[1].as_str(), Some("serve"));
        assert!(parsed["mcp_servers"]["orbit"].get("cwd").is_none());
        assert_eq!(
            parsed["mcp_servers"]["other"]["command"].as_str(),
            Some("demo")
        );

        run_action(
            McpAction::Remove,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Codex]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("remove codex");

        let config = std::fs::read_to_string(repo.path().join(".codex").join("config.toml"))
            .expect("read config");
        let parsed: toml::Value = toml::from_str(&config).expect("parse config");
        assert!(
            parsed
                .get("mcp_servers")
                .and_then(toml::Value::as_table)
                .and_then(|table| table.get("orbit"))
                .is_none()
        );
        assert_eq!(
            parsed["mcp_servers"]["other"]["command"].as_str(),
            Some("demo")
        );
    }

    #[test]
    fn gemini_workspace_scope_init_and_remove_preserve_unrelated_entries() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        std::fs::create_dir_all(repo.path().join(".gemini")).expect("create .gemini");
        std::fs::write(
            repo.path().join(".gemini").join("settings.json"),
            "{\n  \"theme\": \"dark\",\n  \"mcpServers\": {\n    \"other\": {\"command\": \"demo\"}\n  }\n}\n",
        )
        .expect("write settings");
        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Gemini]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("init gemini");

        let settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(repo.path().join(".gemini").join("settings.json"))
                .expect("read settings"),
        )
        .expect("parse settings");
        assert_eq!(settings["theme"], "dark");
        assert!(settings["mcpServers"]["orbit"].is_object());
        assert!(settings["mcpServers"]["other"].is_object());
        assert!(settings["mcpServers"]["orbit"]["cwd"].is_null());
        let args = settings["mcpServers"]["orbit"]["args"]
            .as_array()
            .expect("args array");
        assert_eq!(args.len(), 2);
        assert_eq!(args[0].as_str(), Some("mcp"));
        assert_eq!(args[1].as_str(), Some("serve"));

        run_action(
            McpAction::Remove,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Gemini]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("remove gemini");

        let settings: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(repo.path().join(".gemini").join("settings.json"))
                .expect("read settings"),
        )
        .expect("parse settings");
        assert_eq!(settings["theme"], "dark");
        assert!(settings["mcpServers"]["orbit"].is_null());
        assert!(settings["mcpServers"]["other"].is_object());
    }

    #[test]
    fn workspace_scope_codex_init_is_idempotent() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Codex]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("init codex");
        let first = std::fs::read_to_string(repo.path().join(".codex").join("config.toml"))
            .expect("read first config");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Codex]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("init codex again");
        let second = std::fs::read_to_string(repo.path().join(".codex").join("config.toml"))
            .expect("read second config");

        assert_eq!(first, second);
    }

    #[test]
    fn workspace_scope_gemini_init_is_idempotent() {
        let repo = tempdir().expect("repo tempdir");
        let home = tempdir().expect("home tempdir");
        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Gemini]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("init gemini");
        let first = std::fs::read_to_string(repo.path().join(".gemini").join("settings.json"))
            .expect("read first settings");

        run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Gemini]),
            Some(home.path().to_path_buf()),
            ScopeArg::Workspace,
        )
        .expect("init gemini again");
        let second = std::fs::read_to_string(repo.path().join(".gemini").join("settings.json"))
            .expect("read second settings");

        assert_eq!(first, second);
    }

    #[test]
    fn server_value_builders_emit_mcp_serve_only() {
        let claude = claude_mcp_server_value();
        let claude_args = claude["args"].as_array().expect("claude args");
        assert_eq!(claude_args.len(), 2);
        assert_eq!(claude_args[0].as_str(), Some("mcp"));
        assert_eq!(claude_args[1].as_str(), Some("serve"));

        let gemini = gemini_mcp_server_value();
        let gemini_args = gemini["args"].as_array().expect("gemini args");
        assert_eq!(gemini_args.len(), 2);
        assert!(gemini.get("cwd").is_none());

        let codex = codex_mcp_server_table();
        let codex_args = codex["args"].as_array().expect("codex args");
        assert_eq!(codex_args.len(), 2);
        assert!(codex.get("cwd").is_none());
        assert_eq!(codex["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn home_scope_without_home_dir_errors() {
        let repo = tempdir().expect("repo tempdir");
        let orbit_root = repo.path().join(".orbit");
        std::fs::create_dir_all(&orbit_root).expect("create orbit root");

        let err = run_action(
            McpAction::Init,
            repo.path(),
            &orbit_root,
            ProviderSelectionMode::Explicit(vec![McpProvider::Claude]),
            None,
            ScopeArg::Home,
        )
        .expect_err("home scope without home dir should fail");

        assert!(matches!(
            err,
            orbit_core::OrbitError::InvalidInput(message) if message.contains("HOME")
        ));
    }

    #[test]
    fn env_lock_smoke() {
        let _guard = ENV_LOCK.lock().expect("lock env");
    }
}
