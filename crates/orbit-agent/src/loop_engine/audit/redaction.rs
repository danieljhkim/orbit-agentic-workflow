//! Redaction middleware applied at blob-write time.
//!
//! Scrubs bearer tokens, API keys, and common sensitive header values from
//! verbatim HTTP/tool payloads before they touch disk. Redaction is a
//! write-time transformation: stored bytes are already safe, so read-side
//! tooling (`orbit.audit.loop.*` in a follow-up task) does not need to
//! re-apply it.

use regex::Regex;

pub struct RedactionMiddleware {
    patterns: Vec<(Regex, &'static str)>,
}

impl RedactionMiddleware {
    pub fn default_redaction() -> Self {
        // Order matters: JSON-shaped patterns run first so the wildcard
        // header patterns don't clobber them with a different replacement
        // string. Each pattern is a tuple of (matcher, replacement literal).
        let patterns = vec![
            (
                Regex::new(r#"(?i)"authorization"\s*:\s*"[^"]*""#).expect("regex"),
                r#""authorization":"[REDACTED_AUTH]""#,
            ),
            (
                Regex::new(r#"(?i)"x-api-key"\s*:\s*"[^"]*""#).expect("regex"),
                r#""x-api-key":"[REDACTED_AUTH]""#,
            ),
            (
                Regex::new(r#"(?i)"api[_-]?key"\s*:\s*"[^"]*""#).expect("regex"),
                r#""api_key":"[REDACTED_AUTH]""#,
            ),
            (
                Regex::new(r#"(?i)bearer\s+[A-Za-z0-9._\-+/=]+"#).expect("regex"),
                "Bearer [REDACTED_AUTH]",
            ),
            // Raw header lines (non-JSON): `Authorization: ...` / `x-api-key: ...`.
            (
                Regex::new(r"(?im)^(\s*authorization\s*:\s*).+$").expect("regex"),
                "${1}[REDACTED_AUTH]",
            ),
            (
                Regex::new(r"(?im)^(\s*x-api-key\s*:\s*).+$").expect("regex"),
                "${1}[REDACTED_AUTH]",
            ),
            (
                Regex::new(r"(?im)^(\s*api[_-]?key\s*:\s*).+$").expect("regex"),
                "${1}[REDACTED_AUTH]",
            ),
        ];
        Self { patterns }
    }

    pub fn empty() -> Self {
        Self { patterns: vec![] }
    }

    pub fn apply(&self, bytes: &[u8]) -> Vec<u8> {
        let text = match std::str::from_utf8(bytes) {
            Ok(s) => s,
            Err(_) => return bytes.to_vec(),
        };
        let mut out = text.to_owned();
        for (pattern, replacement) in &self.patterns {
            out = pattern.replace_all(&out, *replacement).into_owned();
        }
        out.into_bytes()
    }
}

impl Default for RedactionMiddleware {
    fn default() -> Self {
        Self::default_redaction()
    }
}
