use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IdentityRole {
    Leader,
    Member,
}

impl Display for IdentityRole {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            IdentityRole::Leader => "leader",
            IdentityRole::Member => "member",
        };
        write!(f, "{value}")
    }
}

impl FromStr for IdentityRole {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "leader" => Ok(IdentityRole::Leader),
            "member" => Ok(IdentityRole::Member),
            other => Err(format!("unknown identity role: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedIdentity {
    pub id: String,
    pub name: String,
    pub role: IdentityRole,
    #[serde(default)]
    pub personality: Vec<(String, String)>,
    #[serde(default)]
    pub behavior: Vec<(String, String)>,
}
