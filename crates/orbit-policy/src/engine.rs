use std::collections::HashSet;

use orbit_types::{PolicyDef, Role};

use crate::{PolicyDecision, evaluator};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolicyContext {
    Tool { name: String, role: Role },
    Process { command: String, role: Role },
    FilesystemWrite { path: String, role: Role },
}

impl PolicyContext {
    pub fn tool(role: Role, name: impl Into<String>) -> Self {
        Self::Tool {
            name: name.into(),
            role,
        }
    }

    pub fn process(role: Role, command: impl Into<String>) -> Self {
        Self::Process {
            command: command.into(),
            role,
        }
    }

    pub fn filesystem_write(role: Role, path: impl Into<String>) -> Self {
        Self::FilesystemWrite {
            path: path.into(),
            role,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PolicyEngine {
    pub(crate) default_allow: bool,
    pub(crate) denied_tools: HashSet<String>,
    pub(crate) allowed_tools: HashSet<String>,
    pub(crate) allowed_commands: HashSet<String>,
    pub(crate) denied_commands: HashSet<String>,
    pub(crate) allow_write_paths: Vec<String>,
    pub(crate) deny_write_paths: Vec<String>,
}

impl PolicyEngine {
    pub fn new_local_default_allow() -> Self {
        Self {
            default_allow: true,
            denied_tools: HashSet::new(),
            allowed_tools: HashSet::new(),
            allowed_commands: HashSet::new(),
            denied_commands: HashSet::new(),
            allow_write_paths: Vec::new(),
            deny_write_paths: Vec::new(),
        }
    }

    pub fn from_def(def: &PolicyDef) -> Self {
        let mut engine = Self::new_local_default_allow();

        if let Some(tools) = &def.tools {
            engine.denied_tools = tools.deny.iter().cloned().collect();
            engine.allowed_tools = tools.allow.iter().cloned().collect();
        }

        if let Some(process) = &def.process {
            engine.allowed_commands = process.allow_commands.iter().cloned().collect();
            engine.denied_commands = process.deny_commands.iter().cloned().collect();
        }

        if let Some(fs) = &def.filesystem {
            engine.allow_write_paths = fs.allow_write.clone();
            engine.deny_write_paths = fs.deny_write.clone();
        }

        engine
    }

    pub fn deny_tool(mut self, name: impl Into<String>) -> Self {
        self.denied_tools.insert(name.into());
        self
    }

    pub fn evaluate(&self, ctx: &PolicyContext) -> PolicyDecision {
        evaluator::evaluate(self, ctx)
    }
}
