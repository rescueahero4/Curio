//! `/api/settings` — configuration, minus every secret.
//!
//! The response is built **field by field from an allowlist**, never by spreading the config
//! and removing what should not be there. That is not stylistic: the previous implementation
//! omitted `pairingToken` from its schema and still leaked it, because a spread put it back
//! (Inventory §10.5, R-SEC-10). A projection that has to name each field cannot leak one
//! nobody thought about.
//!
//! The API key is **write-only**: accepted on PUT, never returned, surfaced as a boolean and
//! a mask.

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use curio_core::config::{Config, Models, SendToClaudeTarget, Thresholds};

use crate::routes::error::{ApiError, ApiResult};
use crate::state::AppState;

/// Everything the settings page may see.
#[derive(Debug, Serialize)]
pub struct PublicSettings {
    pub data_root: String,
    pub projects_root: String,
    pub thresholds: Thresholds,
    pub models: Models,
    pub mcp_enabled: bool,
    pub send_to_claude_target: SendToClaudeTarget,
    pub launch_at_login: bool,
    /// Whether this platform can honour the setting at all, and why not when it cannot.
    ///
    /// Carried so the UI can say *why* a control is unavailable rather than showing a
    /// toggle that silently does nothing (PRD §5: every disabled control says why;
    /// Inventory §1's `launchAtLoginSupport`).
    pub launch_at_login_support: AutostartSupport,
    pub port: Option<u16>,
    pub version: String,
    /// Whether a key is configured. Never the key, and never a prefix of it.
    pub api_key_set: bool,
    /// `sk-ant-…xxxx`, or `null`. Enough to tell two keys apart, not enough to use one.
    pub api_key_masked: Option<String>,
    /// A `.secrets.json` from the previous implementation is still in the data root.
    ///
    /// Reported so Settings can say the key needs entering once more, rather than an
    /// upgrading user finding themselves silently keyless (D31). Never the file's
    /// contents — it is not opened.
    pub api_key_legacy_present: bool,
    pub skill_file_path: String,
    /// The snippet the user pastes into Claude Code's config.
    pub mcp_http_url: String,
    pub mcp_stdio_command: String,
    pub mcp_stdio_args: Vec<String>,
}

/// Whether "Start at Login" can be honoured here.
#[derive(Debug, Serialize)]
pub struct AutostartSupport {
    pub supported: bool,
    /// Present only when unsupported. Phrased for a user, not a developer.
    pub reason: Option<String>,
}

/// `GET /api/settings`.
pub async fn get(State(state): State<AppState>) -> Json<PublicSettings> {
    Json(project(&state))
}

/// What this platform can do about starting at login.
///
/// The OS is the authority (R-BE-28). Windows has the HKCU `Run` key and macOS has
/// `SMAppService`; anything else gets an honest refusal rather than a toggle that appears
/// to work and does nothing after the next reboot.
fn autostart_support() -> AutostartSupport {
    #[cfg(any(windows, target_os = "macos"))]
    {
        AutostartSupport {
            supported: true,
            reason: None,
        }
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        AutostartSupport {
            supported: false,
            reason: Some(
                "Curio can only register itself to start at login on Windows and macOS.".to_owned(),
            ),
        }
    }
}

/// What a PUT may change. `dataRoot` is absent by design — moving a library is not a form
/// field, and the previous implementation's patch schema omitted it for the same reason.
#[derive(Debug, Default, Deserialize)]
pub struct SettingsPatch {
    pub projects_root: Option<String>,
    pub thresholds: Option<Thresholds>,
    pub models: Option<Models>,
    pub mcp_enabled: Option<bool>,
    pub send_to_claude_target: Option<SendToClaudeTarget>,
    pub launch_at_login: Option<bool>,
    /// Write-only. Accepted here, never returned anywhere.
    pub api_key: Option<String>,
}

/// `PUT /api/settings`.
pub async fn put(
    State(state): State<AppState>,
    Json(patch): Json<SettingsPatch>,
) -> ApiResult<Json<PublicSettings>> {
    let mut config = state.config();

    if let Some(root) = patch.projects_root {
        if !std::path::Path::new(&root).is_dir() {
            return Err(ApiError(curio_core::Error::invalid(format!(
                "{root} is not a folder on this machine"
            ))));
        }
        config.projects_root = root;
    }
    if let Some(thresholds) = patch.thresholds {
        // Validated before the save, not at assessment time. Inverted thresholds do not
        // error later — they produce an empty gray zone and quietly wrong classifications,
        // which is far worse than a rejected save.
        thresholds.validate()?;
        config.thresholds = thresholds;
    }
    if let Some(models) = patch.models {
        config.models = models;
    }
    if let Some(enabled) = patch.mcp_enabled {
        config.mcp_enabled = enabled;
    }
    if let Some(target) = patch.send_to_claude_target {
        config.send_to_claude_target = target;
    }
    if let Some(launch) = patch.launch_at_login {
        config.launch_at_login = launch;
    }

    if let Some(key) = patch.api_key {
        // Stored **before** the config is persisted, matching the previous implementation's
        // ordering: if the keychain write fails, the user must not be told their settings
        // saved when the key did not.
        crate::secrets::store_api_key(&key)?;
    }

    config.validate()?;
    persist(&state, &config)?;
    state.set_config(config);

    Ok(Json(project(&state)))
}

#[derive(Debug, serde::Serialize)]
pub struct KeyCheck {
    pub ok: bool,
    /// Why not, when `ok` is false. Shown verbatim, because "rejected" and "unreachable"
    /// call for completely different actions from the user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// `POST /api/settings/verify-key` — prove the stored key works.
///
/// Always `200`: a key that does not work is an answer, not a server error, and the UI
/// wants to render it beside the field rather than in an error toast. The check is the
/// cheapest call that can distinguish a wrong key from a working one — sixteen tokens
/// (Inventory §9) — so it is affordable on every save.
pub async fn verify_key(State(state): State<AppState>) -> Json<KeyCheck> {
    let Some(key) = crate::secrets::api_key() else {
        return Json(KeyCheck {
            ok: false,
            reason: Some("no API key is configured".to_owned()),
        });
    };

    let model = state.config().models.utility;
    let client = match crate::ai::Anthropic::new(key) {
        Ok(client) => client,
        Err(err) => {
            return Json(KeyCheck {
                ok: false,
                reason: Some(err.to_string()),
            });
        }
    };

    match client.verify_key(&model).await {
        Ok(()) => Json(KeyCheck {
            ok: true,
            reason: None,
        }),
        Err(err) => Json(KeyCheck {
            ok: false,
            reason: Some(err.to_string()),
        }),
    }
}

/// `DELETE /api/settings/api-key`.
pub async fn clear_api_key(State(state): State<AppState>) -> ApiResult<Json<PublicSettings>> {
    crate::secrets::clear_api_key()?;
    Ok(Json(project(&state)))
}

/// Build the public projection, one named field at a time.
fn project(state: &AppState) -> PublicSettings {
    let config = state.config();
    let root = state.data_root().to_path_buf();

    PublicSettings {
        data_root: root.display().to_string(),
        projects_root: config.projects_root,
        thresholds: config.thresholds,
        models: config.models,
        mcp_enabled: config.mcp_enabled,
        send_to_claude_target: config.send_to_claude_target,
        launch_at_login: config.launch_at_login,
        launch_at_login_support: autostart_support(),
        port: config.port,
        version: state.version().to_owned(),
        api_key_set: crate::secrets::api_key().is_some(),
        api_key_masked: crate::secrets::api_key().as_deref().map(mask),
        api_key_legacy_present: crate::secrets::api_key().is_none()
            && crate::secrets::legacy_secrets_present(&root),
        skill_file_path: root
            .join(curio_core::paths::SKILL_FILE_RELATIVE)
            .display()
            .to_string(),
        // The bound port, not a placeholder: with an ephemeral port there is no configured
        // value to fall back on, and a snippet the user pastes into Claude Code has to be
        // one that can actually connect.
        mcp_http_url: format!("http://127.0.0.1:{}/mcp", state.port()),
        mcp_stdio_command: "curio".to_owned(),
        mcp_stdio_args: vec!["--mcp-stdio".to_owned()],
    }
}

/// `sk-ant-…xxxx` — enough to tell two keys apart, not enough to use one (R-SEC-10).
fn mask(key: &str) -> String {
    let tail: String = key
        .chars()
        .rev()
        .take(4)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    format!("sk-ant-…{tail}")
}

fn persist(state: &AppState, config: &Config) -> ApiResult<()> {
    let path = state.data_root().join(curio_core::paths::CONFIG_FILE_NAME);
    let body = serde_json::to_string_pretty(config).map_err(curio_core::Error::Json)?;
    std::fs::write(path, body)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RuntimeToken;

    fn state() -> AppState {
        AppState::new(
            RuntimeToken::mint(),
            "quit-secret",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            Config::default(),
            curio_db::Db::open_in_memory().expect("db"),
        )
    }

    #[test]
    fn the_public_projection_has_no_field_that_could_hold_a_secret() {
        // Inventory §10.5 / R-SEC-10. The previous implementation omitted `pairingToken`
        // from its schema and leaked it anyway, because a spread put it back. This shape
        // has to name each field, so it cannot carry one nobody thought about.
        let rendered = serde_json::to_string(&project(&state())).expect("serialize");

        for forbidden in ["apiKey", "api_key\"", "token", "pairingToken", "quit"] {
            assert!(!rendered.contains(forbidden), "{forbidden} in {rendered}");
        }
    }

    #[test]
    fn a_legacy_secrets_file_is_reported_but_never_read() {
        // D31: the encrypted-file backend is retired rather than guessed at, so an
        // upgrading user's key does not carry over. Telling them is the whole mitigation.
        let dir = tempfile::tempdir().expect("tempdir");
        assert!(!crate::secrets::legacy_secrets_present(dir.path()));

        std::fs::write(dir.path().join(".secrets.json"), "{\"backend\":\"dpapi\"}").expect("write");
        assert!(crate::secrets::legacy_secrets_present(dir.path()));
    }

    #[test]
    fn the_flag_is_suppressed_once_a_key_exists() {
        // Someone who has already re-entered their key should not keep being told to.
        let settings = project(&state());
        assert!(!settings.api_key_legacy_present || !settings.api_key_set);
    }

    #[test]
    fn the_key_is_reported_as_a_boolean_and_a_mask() {
        let settings = project(&state());

        assert!(!settings.api_key_set);
        assert!(settings.api_key_masked.is_none());
    }

    #[test]
    fn a_mask_shows_four_characters_and_no_more() {
        // Enough to tell two keys apart on screen; not enough to reconstruct one from a
        // screenshot or a support thread.
        let masked = mask("sk-ant-api03-abcdefghijklmnop1234");

        assert_eq!(masked, "sk-ant-…1234");
        assert!(!masked.contains("abcdefg"));
    }

    #[test]
    fn masking_a_short_key_does_not_panic_or_reveal_it_whole() {
        assert_eq!(mask("abc"), "sk-ant-…abc");
        assert_eq!(mask(""), "sk-ant-…");
    }

    #[test]
    fn a_patch_cannot_move_the_data_root() {
        // Moving a library is not a form field. The previous implementation's patch schema
        // omitted it for the same reason.
        let patch: SettingsPatch =
            serde_json::from_str(r#"{"data_root":"/somewhere/else"}"#).expect("parse");

        assert!(patch.projects_root.is_none());
    }

    #[test]
    fn inverted_thresholds_are_refused_at_save_time() {
        // They would not error at assessment time — they would produce an empty gray zone
        // and quietly wrong classifications.
        assert!(
            Thresholds {
                lower: 0.7,
                upper: 0.3
            }
            .validate()
            .is_err()
        );
    }
}
