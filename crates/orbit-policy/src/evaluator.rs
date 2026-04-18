use std::path::{Component, Path, PathBuf};

use crate::PolicyDecision;
use crate::engine::{PolicyContext, PolicyEngine};
use orbit_types::Role;

pub(crate) fn evaluate(engine: &PolicyEngine, ctx: &PolicyContext) -> PolicyDecision {
    match ctx {
        PolicyContext::Tool { name, role } => evaluate_tool(engine, name, *role),
        PolicyContext::Process { command, .. } => evaluate_process(engine, command),
        PolicyContext::FilesystemWrite { path, .. } => evaluate_filesystem_write(engine, path),
    }
}

fn evaluate_tool(engine: &PolicyEngine, tool_name: &str, role: Role) -> PolicyDecision {
    if role != Role::Admin && engine.denied_tools.contains(tool_name) {
        return PolicyDecision::Deny {
            reason: format!("tool `{tool_name}` denied by policy"),
        };
    }
    if !engine.allowed_tools.is_empty() && !engine.allowed_tools.contains(tool_name) {
        return PolicyDecision::Deny {
            reason: format!("tool `{tool_name}` not in allow list"),
        };
    }
    default_decision(engine.default_allow)
}

fn evaluate_process(engine: &PolicyEngine, command: &str) -> PolicyDecision {
    let base_command = command.split_whitespace().next().unwrap_or(command);

    if engine.denied_commands.contains(base_command) || engine.denied_commands.contains(command) {
        return PolicyDecision::Deny {
            reason: format!("command `{command}` denied by policy"),
        };
    }
    if !engine.allowed_commands.is_empty()
        && !engine.allowed_commands.contains(base_command)
        && !engine.allowed_commands.contains(command)
    {
        return PolicyDecision::Deny {
            reason: format!("command `{command}` not in allow list"),
        };
    }
    default_decision(engine.default_allow)
}

fn evaluate_filesystem_write(engine: &PolicyEngine, path: &str) -> PolicyDecision {
    for denied in &engine.deny_write_paths {
        if path_matches_rule(path, denied) {
            return PolicyDecision::Deny {
                reason: format!("write to `{path}` denied by policy (matches `{denied}`)"),
            };
        }
    }

    if !engine.allow_write_paths.is_empty() {
        for allowed in &engine.allow_write_paths {
            if path_matches_rule(path, allowed) {
                return default_decision(true);
            }
        }
        return PolicyDecision::Deny {
            reason: format!("write to `{path}` not in allow list"),
        };
    }

    default_decision(engine.default_allow)
}

fn default_decision(default_allow: bool) -> PolicyDecision {
    if default_allow {
        PolicyDecision::Allow
    } else {
        PolicyDecision::Deny {
            reason: "default deny policy".to_string(),
        }
    }
}

fn path_matches_rule(path: &str, rule: &str) -> bool {
    let Some(path) = normalize_policy_path(path) else {
        return false;
    };
    let Some(rule) = normalize_policy_path(rule) else {
        return false;
    };

    !rule.as_os_str().is_empty() && path.starts_with(&rule)
}

fn normalize_policy_path(path: &str) -> Option<PathBuf> {
    let mut normalized = PathBuf::new();

    for component in Path::new(path).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(segment) => normalized.push(segment),
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::RootDir | Component::Prefix(_) => normalized.push(component.as_os_str()),
        }
    }

    Some(normalized)
}
