use clap::{Args, Subcommand};
use orbit_core::{OrbitError, OrbitRuntime};
use orbit_types::{DEFAULT_POLICY_NAME, PolicyDef, UNRESTRICTED_FS_PROFILE};
use serde_json::{Value, json};

use crate::command::Execute;

#[derive(Args)]
#[command(about = "Manage filesystem profile policies")]
pub struct PolicyCommand {
    #[command(subcommand)]
    pub command: PolicySubcommand,
}

impl Execute for PolicyCommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        self.command.execute(runtime)
    }
}

#[derive(Subcommand)]
pub enum PolicySubcommand {
    /// List all policy definitions
    List(PolicyListArgs),
    /// Show a specific policy definition
    Show(PolicyShowArgs),
    /// Dry-run a path against the active policy's fsProfile rules
    Check(PolicyCheckArgs),
}

impl Execute for PolicySubcommand {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        match self {
            PolicySubcommand::List(args) => args.execute(runtime),
            PolicySubcommand::Show(args) => args.execute(runtime),
            PolicySubcommand::Check(args) => args.execute(runtime),
        }
    }
}

#[derive(Args)]
pub struct PolicyListArgs {
    #[arg(long)]
    pub json: bool,
}

impl Execute for PolicyListArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let defs = runtime.list_policy_defs()?;
        if self.json {
            let values: Vec<Value> = defs
                .iter()
                .map(|d| {
                    json!({
                        "name": d.name,
                        "description": d.description,
                        "fs_profiles": sorted_profile_names(d),
                        "created_at": d.created_at.to_rfc3339(),
                        "updated_at": d.updated_at.to_rfc3339(),
                    })
                })
                .collect();
            crate::output::json::print_pretty(&Value::Array(values))
        } else {
            if defs.is_empty() {
                println!("No policy definitions found.");
                return Ok(());
            }
            let mut table = crate::output::table::build_table(&[
                "NAME",
                "DESCRIPTION",
                "FSPROFILES",
                "UPDATED",
            ]);
            for def in &defs {
                table.add_row(vec![
                    def.name.clone(),
                    def.description.clone().unwrap_or_default(),
                    sorted_profile_names(def).join(", "),
                    def.updated_at.format("%Y-%m-%d %H:%M").to_string(),
                ]);
            }
            println!("{table}");
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct PolicyShowArgs {
    pub name: String,
    #[arg(long)]
    pub json: bool,
}

impl Execute for PolicyShowArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let def = runtime
            .get_policy_def(&self.name)?
            .ok_or_else(|| OrbitError::InvalidInput(format!("policy not found: {}", self.name)))?;

        if self.json {
            crate::output::json::print_pretty(&policy_json(&def)?)
        } else {
            print_policy(&def)?;
            Ok(())
        }
    }
}

#[derive(Args)]
pub struct PolicyCheckArgs {
    pub profile_name: String,
    pub path: String,
    #[arg(long)]
    pub json: bool,
}

impl Execute for PolicyCheckArgs {
    fn execute(self, runtime: &OrbitRuntime) -> Result<(), OrbitError> {
        let def = runtime
            .get_policy_def(DEFAULT_POLICY_NAME)?
            .ok_or_else(|| {
                OrbitError::InvalidInput(format!("policy not found: {}", DEFAULT_POLICY_NAME))
            })?;

        let read = def.check_path(
            &self.profile_name,
            orbit_types::FsOperation::Read,
            &self.path,
        )?;
        let modify = def.check_path(
            &self.profile_name,
            orbit_types::FsOperation::Modify,
            &self.path,
        )?;

        if self.json {
            return crate::output::json::print_pretty(&json!({
                "policy": DEFAULT_POLICY_NAME,
                "profile": self.profile_name,
                "path": self.path,
                "read": {
                    "allowed": read.allowed,
                    "matched_rule": read.matched_rule,
                },
                "modify": {
                    "allowed": modify.allowed,
                    "matched_rule": modify.matched_rule,
                },
            }));
        }

        println!("Policy:  {}", DEFAULT_POLICY_NAME);
        println!("Profile: {}", self.profile_name);
        println!("Path:    {}", self.path);
        println!(
            "read:    {} ({})",
            status_word(read.allowed),
            read.matched_rule
        );
        println!(
            "modify:  {} ({})",
            status_word(modify.allowed),
            modify.matched_rule
        );
        Ok(())
    }
}

fn policy_json(def: &PolicyDef) -> Result<Value, OrbitError> {
    Ok(json!({
        "name": def.name,
        "description": def.description,
        "deny_read": def.deny_read,
        "deny_modify": def.deny_modify,
        "fs_profiles": effective_profiles_json(def)?,
        "created_at": def.created_at.to_rfc3339(),
        "updated_at": def.updated_at.to_rfc3339(),
    }))
}

fn print_policy(def: &PolicyDef) -> Result<(), OrbitError> {
    println!("Name:        {}", def.name);
    if let Some(desc) = &def.description {
        println!("Description: {desc}");
    }
    println!("Created:     {}", def.created_at.to_rfc3339());
    println!("Updated:     {}", def.updated_at.to_rfc3339());

    println!("\nGlobal Denies:");
    println!("  denyRead:   {}", render_rule_list(&def.deny_read));
    println!("  denyModify: {}", render_rule_list(&def.deny_modify));

    println!("\nfsProfiles:");
    for profile_name in sorted_profile_names(def) {
        let effective = def.effective_profile(&profile_name)?;
        println!("  {}:", profile_name);
        println!("    read:   {}", render_rule_list(&effective.read));
        println!("    modify: {}", render_rule_list(&effective.modify));
    }

    Ok(())
}

fn effective_profiles_json(def: &PolicyDef) -> Result<Value, OrbitError> {
    let mut profiles = Vec::new();
    for profile_name in sorted_profile_names(def) {
        let effective = def.effective_profile(&profile_name)?;
        profiles.push(json!({
            "name": profile_name,
            "read": effective.read,
            "modify": effective.modify,
        }));
    }
    Ok(Value::Array(profiles))
}

fn sorted_profile_names(def: &PolicyDef) -> Vec<String> {
    let mut names: Vec<String> = def.fs_profiles.keys().cloned().collect();
    names.sort();
    if !names.iter().any(|name| name == UNRESTRICTED_FS_PROFILE) {
        names.push(UNRESTRICTED_FS_PROFILE.to_string());
    }
    names
}

fn render_rule_list(rules: &[String]) -> String {
    if rules.is_empty() {
        "[]".to_string()
    } else {
        rules.join(", ")
    }
}

fn status_word(allowed: bool) -> &'static str {
    if allowed { "allowed" } else { "denied" }
}
