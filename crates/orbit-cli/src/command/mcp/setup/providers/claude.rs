use orbit_core::OrbitError;
use serde_json::{Map as JsonMap, Value as JsonValue};

use crate::command::mcp::{ORBIT_MCP_SERVER_ID, safe_mcp_tool_names};

use super::super::dispatch::ConfigTarget;
use super::super::format::*;
use super::common::server_args;

pub(in crate::command::mcp::setup) fn apply_claude_init(
    target: &ConfigTarget,
) -> Result<(), OrbitError> {
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

pub(in crate::command::mcp::setup) fn apply_claude_remove(
    target: &ConfigTarget,
) -> Result<(), OrbitError> {
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

pub(super) fn claude_mcp_server_value() -> JsonValue {
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

fn claude_safe_permissions() -> Vec<String> {
    safe_mcp_tool_names()
        .into_iter()
        .map(claude_permission_name)
        .collect()
}

fn claude_permission_name(tool_name: &str) -> String {
    format!("mcp__plugin_orbit_orbit__{}", tool_name.replace('.', "_"))
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::super::super::args::{McpAction, McpProvider, ProviderSelectionMode, ScopeArg};
    use super::super::super::dispatch::run_action;
    use super::*;

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
}
