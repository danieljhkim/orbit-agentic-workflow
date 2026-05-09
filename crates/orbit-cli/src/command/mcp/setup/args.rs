use std::path::Path;

use clap::{Args, ValueEnum};
use orbit_core::OrbitError;

use super::dispatch::{print_action_summary, run_action};
use super::workspace::{env_home_dir, resolve_workspace_layout};

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum, Default)]
pub enum ScopeArg {
    /// Write to user-level config (~/.claude, ~/.codex, ~/.gemini).
    Home,
    /// Write to repo-local config (.mcp.json, .codex/, .gemini/). Default.
    #[default]
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum McpProvider {
    Claude,
    Codex,
    Gemini,
    Cursor,
    Vscode,
    Windsurf,
}

impl McpProvider {
    pub(super) fn label(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Gemini => "gemini",
            Self::Cursor => "cursor",
            Self::Vscode => "vscode",
            Self::Windsurf => "windsurf",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) enum McpAction {
    Init,
    Remove,
}

impl McpAction {
    pub(super) fn label(self) -> &'static str {
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
    /// Target Cursor integration only.
    #[arg(long)]
    pub cursor: bool,
    /// Target VS Code integration only.
    #[arg(long)]
    pub vscode: bool,
    /// Target Windsurf integration only.
    #[arg(long)]
    pub windsurf: bool,
    /// Target all supported MCP client integrations.
    #[arg(long)]
    pub all: bool,
}

impl ProviderSelectionArgs {
    fn any_explicit_provider(&self) -> bool {
        self.claude || self.codex || self.gemini || self.cursor || self.vscode || self.windsurf
    }

    fn resolve_mode(&self) -> Result<ProviderSelectionMode, OrbitError> {
        if self.auto && (self.any_explicit_provider() || self.all) {
            return Err(OrbitError::InvalidInput(
                "--auto cannot be combined with --claude, --codex, --gemini, --cursor, --vscode, --windsurf, or --all".to_string(),
            ));
        }
        if self.all && self.any_explicit_provider() {
            return Err(OrbitError::InvalidInput(
                "--all cannot be combined with --claude, --codex, --gemini, --cursor, --vscode, or --windsurf".to_string(),
            ));
        }
        if self.auto || (!self.any_explicit_provider() && !self.all) {
            return Ok(ProviderSelectionMode::Auto);
        }
        if self.all {
            return Ok(ProviderSelectionMode::Explicit(vec![
                McpProvider::Claude,
                McpProvider::Codex,
                McpProvider::Gemini,
                McpProvider::Cursor,
                McpProvider::Vscode,
                McpProvider::Windsurf,
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
        if self.cursor {
            providers.push(McpProvider::Cursor);
        }
        if self.vscode {
            providers.push(McpProvider::Vscode);
        }
        if self.windsurf {
            providers.push(McpProvider::Windsurf);
        }
        Ok(ProviderSelectionMode::Explicit(providers))
    }
}

pub(super) enum ProviderSelectionMode {
    Auto,
    Explicit(Vec<McpProvider>),
}

#[derive(Args, Debug, Clone, Default)]
#[command(about = "Initialize MCP client integration for the current workspace")]
pub struct InitArgs {
    #[command(flatten)]
    pub providers: ProviderSelectionArgs,
    /// Scope for written config files (workspace: repo-local, home: user-level).
    #[arg(long, value_enum, default_value_t = ScopeArg::Workspace)]
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
    /// Scope for config files to remove (workspace: repo-local, home: user-level).
    #[arg(long, value_enum, default_value_t = ScopeArg::Workspace)]
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
    // writes repo-local files. `orbit mcp init` defaults to workspace scope
    // as well; pass `--scope home` for a user-level registration.
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

#[cfg(test)]
mod tests {
    use super::*;

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
            ..ProviderSelectionArgs::default()
        };
        assert!(args.resolve_mode().is_err());
    }

    #[test]
    fn provider_selection_all_includes_every_supported_provider() {
        let args = ProviderSelectionArgs {
            all: true,
            ..ProviderSelectionArgs::default()
        };
        match args.resolve_mode().expect("resolve mode") {
            ProviderSelectionMode::Explicit(providers) => assert_eq!(
                providers,
                vec![
                    McpProvider::Claude,
                    McpProvider::Codex,
                    McpProvider::Gemini,
                    McpProvider::Cursor,
                    McpProvider::Vscode,
                    McpProvider::Windsurf,
                ]
            ),
            ProviderSelectionMode::Auto => panic!("expected explicit provider set"),
        }
    }

    #[test]
    fn provider_selection_rejects_auto_combined_with_new_flags() {
        for flag in ["cursor", "vscode", "windsurf"] {
            let mut args = ProviderSelectionArgs {
                auto: true,
                ..ProviderSelectionArgs::default()
            };
            match flag {
                "cursor" => args.cursor = true,
                "vscode" => args.vscode = true,
                "windsurf" => args.windsurf = true,
                _ => unreachable!(),
            }
            assert!(
                args.resolve_mode().is_err(),
                "--auto + --{flag} should error"
            );
        }
    }
}
