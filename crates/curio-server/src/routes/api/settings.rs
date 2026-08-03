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

use curio_core::config::{Config, Models, Thresholds};

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
    pub launch_at_login: bool,
    /// Whether this platform can honour the setting at all, and why not when it cannot.
    ///
    /// Carried so the UI can say *why* a control is unavailable rather than showing a
    /// toggle that silently does nothing (PRD §5: every disabled control says why;
    /// Inventory §1's `launchAtLoginSupport`).
    pub launch_at_login_support: AutostartSupport,
    /// The port a user **pinned** in `config.json`, if any. A preference, not an address.
    ///
    /// `None` is the default and means an OS-assigned ephemeral port (D10). Nothing may
    /// display this as "the port Curio is on": it is what was asked for at boot, and with no
    /// pin it answers nothing at all. Use [`bound_port`](Self::bound_port).
    pub port: Option<u16>,
    /// The port Curio is **actually listening on**, read from the bound socket.
    ///
    /// This is the number a user needs — the one to put in a browser, or read out when
    /// something cannot reach the app — and it is the only one that is true in every case.
    /// With an ephemeral port there is no configured value to fall back on, and even with a
    /// pin the two can disagree: `CURIO_PORT` overrides `config.json`, and the socket is
    /// bound once at boot while `config.json` can be edited underneath it at any time.
    ///
    /// The settings page used to print `port` here, which is why a library with `4321` in
    /// its config showed `4321` no matter which socket the running process held.
    pub bound_port: u16,
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
        launch_at_login: config.launch_at_login,
        launch_at_login_support: autostart_support(),
        port: config.port,
        bound_port: state.port(),
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
        mcp_stdio_command: stdio_command(),
        mcp_stdio_args: vec!["--mcp-stdio".to_owned()],
    }
}

/// The executable an agent should spawn for the stdio transport — as an absolute path.
///
/// Not the bare name `curio`. A client spawning the proxy does it without a shell, and even
/// with one there is nothing that puts Curio on `PATH`: it is launched from wherever it was
/// installed or built, and no installer here writes a `PATH` entry. A bare name therefore
/// resolves for nobody, and the failure is silent in the worst way — the registration is
/// accepted, the client lists the server, and every connection dies with "connection closed"
/// because the process was never found to start.
///
/// `curio-nmh` writes the browsers' native-messaging manifests from `current_exe()` for
/// exactly this reason (see `register.rs`); this is the same answer for the same question.
///
/// The fallback is the bare name, which is no worse than what it replaces: if the OS will not
/// say where this process lives, a name the user can correct by hand beats an empty string.
fn stdio_command() -> String {
    std::env::current_exe().map_or_else(|_| "curio".to_owned(), |path| path.display().to_string())
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
    fn the_reported_port_is_the_bound_one_not_the_configured_one() {
        // The defect this exists for, found on a real library: the settings page printed
        // `port`, which is the *preference* in config.json. A user with `4321` pinned there
        // saw "Port 4321" whatever socket the process actually held — a number that looks
        // like an address, reads like an address, and is not one.
        let config = Config {
            port: Some(4321),
            ..Config::default()
        };

        let pinned = AppState::new(
            RuntimeToken::mint(),
            "quit-secret",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            config,
            curio_db::Db::open_in_memory().expect("db"),
        );

        let settings = project(&pinned);

        assert_eq!(settings.bound_port, 51_234, "the socket is the truth");
        assert_eq!(
            settings.port,
            Some(4321),
            "the pin is still reported, as a pin"
        );
        // And the MCP snippet, which has always used the bound port, must agree with it.
        assert!(
            settings.mcp_http_url.contains("51234"),
            "{}",
            settings.mcp_http_url
        );
    }

    #[test]
    fn the_stdio_command_is_a_path_an_agent_can_actually_spawn() {
        // The defect this exists for: the field said `curio`, and nothing on any supported
        // platform puts `curio` on PATH. `claude mcp add` accepted it, Claude Code listed the
        // server, and every session died with "connection closed" — the process was never
        // found to start. A bare name is the one answer that cannot work, so the assertion is
        // simply that this is a path.
        let settings = project(&state());
        let command = std::path::Path::new(&settings.mcp_stdio_command);

        assert!(
            command.is_absolute(),
            "an agent spawns this without a shell: {}",
            settings.mcp_stdio_command
        );
        assert_eq!(settings.mcp_stdio_args, ["--mcp-stdio"]);
    }

    #[test]
    fn an_ephemeral_port_still_reports_a_real_number() {
        // The default: no pin at all (D10). `port` answers nothing, and a page that had only
        // that field to work from could say nothing more useful than "chosen at launch" —
        // which is exactly when a user most needs the number.
        let settings = project(&state());

        assert_eq!(settings.port, None);
        assert_eq!(settings.bound_port, 51_234);
    }

    #[test]
    fn the_key_is_reported_as_a_boolean_and_a_mask() {
        // `project` reads the real OS keychain, not `state`, so whether a key exists is a
        // property of the machine running the test. Asserting "no key" therefore passed on
        // clean CI runners and failed on every developer who had configured one — a test
        // that reports the environment rather than the code.
        //
        // What must hold either way is that the two fields agree, which is the defect worth
        // guarding: a boolean saying "configured" beside an absent mask, or a mask beside a
        // false boolean, is what would mislead the settings page.
        let settings = project(&state());

        assert_eq!(settings.api_key_set, settings.api_key_masked.is_some());

        if let Some(masked) = &settings.api_key_masked {
            // Whatever the machine holds, the projection must never surface it whole
            // (Inventory §10.5).
            assert!(masked.starts_with("sk-ant-…"), "{masked}");
            assert!(masked.len() <= "sk-ant-…".len() + 4, "{masked}");
        }
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
