//! `config.json` — the user-editable settings file.
//!
//! Rewritten on every boot and every save, so it is also the file that documents itself:
//! a user who opens it sees every setting with its current value rather than only the ones
//! they changed.
//!
//! Two things are deliberately **not** here (R-BE-28):
//!
//! * **`pairingToken` is gone.** It was a long-lived secret in a user-editable file. Its
//!   replacement is the per-run runtime token, which lives in `runtime.json` — a
//!   machine-written, owner-only file in the app-data directory that is deleted at quit
//!   (D12, D21, ARCH-08 break #2).
//! * **The API key is not stored here.** It lives in the OS keychain, and the settings API
//!   is write-only for it: accepted on PUT, never returned, surfaced only as a boolean and
//!   a mask (R-SEC-10).

use serde::{Deserialize, Serialize};

/// The lower and upper family-match thresholds.
///
/// Between them is the **gray zone**: the model is neither confident enough to assign the
/// family nor confident enough to reject it, so the item is held for a human decision
/// (FR-6, FR-7). The model is explicitly told not to apply these itself — it reports
/// scores, and Curio applies the policy, so changing a threshold re-decides existing items
/// without re-running a single model call.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Thresholds {
    pub lower: f64,
    pub upper: f64,
}

impl Default for Thresholds {
    fn default() -> Self {
        Self {
            lower: 0.4,
            upper: 0.5,
        }
    }
}

impl Thresholds {
    /// # Errors
    /// Returns [`crate::Error::Invalid`] if the thresholds are out of range or inverted.
    pub fn validate(&self) -> crate::Result<()> {
        if !(0.0..=1.0).contains(&self.lower) || !(0.0..=1.0).contains(&self.upper) {
            return Err(crate::Error::invalid(
                "thresholds must be between 0.0 and 1.0",
            ));
        }
        if self.lower > self.upper {
            // Inverted thresholds do not produce an error at assessment time — they
            // produce an empty gray zone and quietly wrong classifications, which is far
            // worse than a rejected save.
            return Err(crate::Error::invalid(
                "the lower threshold cannot exceed the upper threshold",
            ));
        }
        Ok(())
    }
}

/// Which models to call.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Models {
    /// The vision model that assesses screenshots. One structured call per item (FR-4).
    pub vision: String,
    /// The cheaper model for utility work — vocabulary dedupe and similar.
    ///
    /// Called without an `effort` parameter, which this class of model rejects
    /// (Inventory §10.7).
    pub utility: String,
}

impl Default for Models {
    fn default() -> Self {
        Self {
            vision: "claude-sonnet-5".to_owned(),
            utility: "claude-haiku-4-5".to_owned(),
        }
    }
}

/// Where "Send to Claude" sends.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SendToClaudeTarget {
    #[default]
    ClaudeCode,
    ClaudeDesktop,
    Clipboard,
}

/// The whole of `config.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Config {
    /// Everything the user owns. Resolved at boot; see [`crate::paths`].
    pub data_root: String,
    /// The watched directory where AI tools write projects.
    pub projects_root: String,
    pub thresholds: Thresholds,
    pub models: Models,
    /// Whether the MCP surface answers. Read **per request**, never cached at router
    /// construction, and it gates both transports (R-MCP-8, R-MCP-6).
    ///
    /// Defaults to off: an MCP server that appears without the user asking is a surface
    /// they did not choose.
    pub mcp_enabled: bool,
    pub send_to_claude_target: SendToClaudeTarget,
    /// Stored, but the OS is the authority — this is a cache of what the OS reports, and
    /// a disagreement is resolved in the OS's favour (R-BE-28).
    pub launch_at_login: bool,
    /// An optional fixed port.
    ///
    /// Absent by default, which means an OS-assigned ephemeral port — the setting that
    /// deletes the entire port-conflict class (D10). It exists for development and
    /// unpacked extension installs, where the legacy discovery ladder needs a
    /// deterministic address to find (D11). `CURIO_PORT` wins over this value; there is
    /// no port-walk in any mode.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
}

impl Config {
    /// # Errors
    /// Returns [`crate::Error::Invalid`] if any field is out of range.
    pub fn validate(&self) -> crate::Result<()> {
        self.thresholds.validate()?;
        if self.port.is_some_and(|port| port < 1024) {
            return Err(crate::Error::invalid(
                "a configured port must be 1024 or above",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_shipped_values() {
        let config = Config::default();
        assert_eq!(config.thresholds.lower, 0.4);
        assert_eq!(config.thresholds.upper, 0.5);
        assert!(
            !config.mcp_enabled,
            "MCP is opt-in: a surface the user did not ask for should not appear"
        );
        assert!(
            config.port.is_none(),
            "absent means ephemeral, which is the decision that removes port conflicts (D10)"
        );
    }

    #[test]
    fn inverted_thresholds_are_rejected() {
        // They would not error at assessment time — they would produce an empty gray zone
        // and quietly wrong classifications.
        let thresholds = Thresholds {
            lower: 0.7,
            upper: 0.3,
        };
        assert!(thresholds.validate().is_err());
    }

    #[test]
    fn equal_thresholds_are_allowed() {
        // A zero-width gray zone is a legitimate choice: assign or reject, never ask.
        let thresholds = Thresholds {
            lower: 0.5,
            upper: 0.5,
        };
        assert!(thresholds.validate().is_ok());
    }

    #[test]
    fn an_absent_port_stays_absent_through_a_round_trip() {
        // Serializing `"port": null` would make an ephemeral install look configured, and
        // config.json is rewritten on every boot — so the wrong shape would become
        // permanent on the first save.
        let json = serde_json::to_string(&Config::default()).unwrap();
        assert!(!json.contains("port"), "{json}");
    }

    #[test]
    fn the_pairing_token_field_is_gone() {
        // ARCH-08 break #2. A long-lived secret in a user-editable file was replaced by
        // the per-run token in runtime.json. An unknown field must not resurrect it.
        let json = serde_json::to_string(&Config::default()).unwrap();
        assert!(!json.contains("pairingToken"), "{json}");
    }

    #[test]
    fn unknown_fields_are_tolerated() {
        // The file is user-editable and rewritten on boot. A stray key from an older
        // build, or a typo, must not stop the app from starting.
        let parsed: Result<Config, _> =
            serde_json::from_str(r#"{"mcpEnabled": true, "somethingElse": 1}"#);
        assert!(parsed.is_ok());
        assert!(parsed.unwrap().mcp_enabled);
    }

    #[test]
    fn field_names_are_camel_case_on_disk() {
        // Existing config.json files were written by the previous implementation.
        let json = serde_json::to_string(&Config::default()).unwrap();
        assert!(json.contains("mcpEnabled"), "{json}");
        assert!(json.contains("projectsRoot"), "{json}");
        assert!(json.contains("sendToClaudeTarget"), "{json}");
    }

    #[test]
    fn send_to_claude_targets_serialize_kebab_case() {
        assert_eq!(
            serde_json::to_string(&SendToClaudeTarget::ClaudeCode).unwrap(),
            "\"claude-code\""
        );
    }
}
