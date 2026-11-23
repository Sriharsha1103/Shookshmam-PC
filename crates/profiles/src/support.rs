//! Support-bundle builder (PHASE-5-SPEC §7). Bundles app/OS/adapter versions,
//! device inventory and capability states plus a sanitized log tail. No data is
//! uploaded anywhere; sanitization strips the user's home path and email-like
//! tokens so personal paths/data are not recorded.

use serde::Serialize;
use std::collections::BTreeMap;

/// True if a token looks like an email address (has `@` with a dotted domain).
fn is_emailish(token: &str) -> bool {
    if let Some((local, domain)) = token.split_once('@') {
        !local.is_empty() && domain.contains('.') && !domain.starts_with('.')
    } else {
        false
    }
}

/// Redact a single log line: replace the home directory with `<HOME>` and any
/// email-like tokens with `<redacted-email>`.
pub fn sanitize_line(line: &str, home_dir: Option<&str>) -> String {
    let mut out = line.to_string();
    if let Some(home) = home_dir {
        if !home.is_empty() {
            out = out.replace(home, "<HOME>");
        }
    }
    out.split_whitespace()
        .map(|tok| {
            if is_emailish(tok) {
                "<redacted-email>".to_string()
            } else {
                tok.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SupportBundle {
    pub app_version: String,
    pub os: String,
    pub adapter_versions: BTreeMap<String, String>,
    pub device_inventory: Vec<String>,
    pub capability_states: BTreeMap<String, String>,
    pub log_tail: Vec<String>,
}

impl SupportBundle {
    pub fn new(app_version: impl Into<String>, os: impl Into<String>) -> Self {
        SupportBundle {
            app_version: app_version.into(),
            os: os.into(),
            adapter_versions: BTreeMap::new(),
            device_inventory: Vec::new(),
            capability_states: BTreeMap::new(),
            log_tail: Vec::new(),
        }
    }

    pub fn with_adapter(mut self, name: impl Into<String>, version: impl Into<String>) -> Self {
        self.adapter_versions.insert(name.into(), version.into());
        self
    }

    pub fn with_inventory(mut self, inventory: Vec<String>) -> Self {
        self.device_inventory = inventory;
        self
    }

    pub fn with_capability_state(mut self, key: impl Into<String>, state: impl Into<String>) -> Self {
        self.capability_states.insert(key.into(), state.into());
        self
    }

    /// Attach a sanitized tail of raw log lines.
    pub fn with_logs(mut self, raw_lines: &[String], home_dir: Option<&str>) -> Self {
        self.log_tail = raw_lines.iter().map(|l| sanitize_line(l, home_dir)).collect();
        self
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("support bundle serializes")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_home_path_and_email() {
        let line = r"error reading C:\Users\alice\config sent to alice@example.com now";
        let out = sanitize_line(line, Some(r"C:\Users\alice"));
        assert!(out.contains("<HOME>"));
        assert!(!out.contains("alice@example.com"));
        assert!(out.contains("<redacted-email>"));
    }

    #[test]
    fn non_email_tokens_preserved() {
        assert_eq!(sanitize_line("cpu temp 50C @ idle", None), "cpu temp 50C @ idle");
    }

    #[test]
    fn bundle_serializes() {
        let bundle = SupportBundle::new("0.1.0", "Windows 11")
            .with_adapter("linux-hwmon", "0.1.0")
            .with_inventory(vec!["cpu0".into()])
            .with_capability_state("cpu_fan", "unvalidated")
            .with_logs(&["started ok".to_string()], None);
        let json = bundle.to_json();
        assert!(json.contains("app_version"));
        assert!(json.contains("linux-hwmon"));
    }
}
