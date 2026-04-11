use std::str::FromStr;

use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Error)]
#[error("invalid selector `{selector}`: {reason}")]
pub struct SelectorParseError {
    pub selector: String,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Selector {
    Dir {
        path: String,
    },
    File {
        path: String,
    },
    Leaf {
        path: String,
        symbol: String,
        kind: String,
    },
}

impl Selector {
    pub fn parse_many(raw_selectors: &[String]) -> Result<Vec<Self>, SelectorParseError> {
        raw_selectors
            .iter()
            .map(|selector| selector.parse())
            .collect()
    }

    pub(crate) fn lookup_key(&self) -> SelectorLookupKey {
        match self {
            Self::Dir { path } => SelectorLookupKey::Dir(path.clone()),
            Self::File { path } => SelectorLookupKey::File(path.clone()),
            Self::Leaf { path, symbol, kind } => {
                SelectorLookupKey::Leaf(format!("{path}#{symbol}"), kind.clone())
            }
        }
    }
}

impl std::fmt::Display for Selector {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dir { path } => write!(f, "dir:{path}"),
            Self::File { path } => write!(f, "file:{path}"),
            Self::Leaf { path, symbol, kind } => write!(f, "leaf:{path}#{symbol}:{kind}"),
        }
    }
}

impl FromStr for Selector {
    type Err = SelectorParseError;

    fn from_str(selector: &str) -> Result<Self, Self::Err> {
        let trimmed = selector.trim();
        if let Some(path) = trimmed.strip_prefix("dir:") {
            if path.is_empty() {
                return Err(SelectorParseError {
                    selector: selector.to_string(),
                    reason: "dir selector path must not be empty".to_string(),
                });
            }
            return Ok(Self::Dir {
                path: path.to_string(),
            });
        }

        if let Some(path) = trimmed.strip_prefix("file:") {
            if path.is_empty() {
                return Err(SelectorParseError {
                    selector: selector.to_string(),
                    reason: "file selector path must not be empty".to_string(),
                });
            }
            return Ok(Self::File {
                path: path.to_string(),
            });
        }

        if let Some(remainder) = trimmed.strip_prefix("leaf:") {
            let (location, kind) =
                remainder
                    .rsplit_once(':')
                    .ok_or_else(|| SelectorParseError {
                        selector: selector.to_string(),
                        reason: "leaf selectors must use `leaf:<path>#<symbol>:<kind>`".to_string(),
                    })?;
            if location.is_empty() || kind.is_empty() {
                return Err(SelectorParseError {
                    selector: selector.to_string(),
                    reason: "leaf selectors must include both a location and kind".to_string(),
                });
            }
            let (path, symbol) = location.split_once('#').ok_or_else(|| SelectorParseError {
                selector: selector.to_string(),
                reason: "leaf selectors must include `#<symbol>`".to_string(),
            })?;
            if path.is_empty() || symbol.is_empty() {
                return Err(SelectorParseError {
                    selector: selector.to_string(),
                    reason: "leaf selectors must include non-empty path and symbol".to_string(),
                });
            }
            return Ok(Self::Leaf {
                path: path.to_string(),
                symbol: symbol.to_string(),
                kind: kind.to_string(),
            });
        }

        Err(SelectorParseError {
            selector: selector.to_string(),
            reason: "selectors must start with `dir:`, `file:`, or `leaf:`".to_string(),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SelectorLookupKey {
    Dir(String),
    File(String),
    /// Leaf(location, kind) where location = "path#symbol".
    Leaf(String, String),
}

impl SelectorLookupKey {
    pub fn to_selector_string(&self) -> String {
        match self {
            Self::Dir(path) => format!("dir:{path}"),
            Self::File(path) => format!("file:{path}"),
            Self::Leaf(location, kind) => format!("leaf:{location}:{kind}"),
        }
    }
}
