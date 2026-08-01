//! The stdio proxy: the second door onto the same pipeline.
//!
//! Claude Desktop and most MCP clients spawn a process and talk newline-delimited JSON-RPC
//! over its stdio. Curio is already running with the database open, so this mode is
//! deliberately **not** a second app: it opens no database, binds nothing, starts no tray,
//! and forwards every frame to the live instance's `/mcp` (D24, R-MCP-5).
//!
//! That is what preserves the single-writer invariant *by construction* rather than by
//! discipline. A proxy that opened `library.db` itself would fork the one writer the whole
//! data layer is designed around, and the event fan-out — the SSE stream a dashboard is
//! watching — would silently miss everything the agent did.
//!
//! It also means `mcpEnabled` gates both transports through one gate: the proxy receives the
//! same refusal the HTTP surface would give and passes it through unchanged (R-MCP-6).
//!
//! ## stdout purity
//!
//! Same rule as `curio-nmh` (R-EXT-5): stdout carries protocol frames and nothing else.
//! Every diagnostic goes to stderr. A stray `println!` here corrupts the stream, and the
//! client-side symptom is an MCP server that mysteriously fails to initialize.

use std::io::{BufRead as _, Write as _};

use curio_runtime::{Discovery, RuntimeFile};

use crate::{JSON_RPC_ERROR_CODE, Refusal};

/// Forward stdio frames to the running instance until stdin closes.
///
/// # Errors
/// Returns an error only for a failure that makes forwarding impossible — a stdout that
/// cannot be written, say. A missing instance is **not** one: that is reported as a
/// JSON-RPC error on the stream, because a client that spawned us is waiting for a frame
/// and an exit code tells it nothing it can show a user.
pub async fn run(runtime_file: &std::path::Path) -> std::io::Result<()> {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let client = reqwest::Client::new();

    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        // Re-read per frame rather than once at startup. Curio can be restarted underneath
        // a long-lived client, and the port and token both change when it is (D10, D21) —
        // a cached address would leave the proxy talking to nothing for the rest of the
        // session.
        let reply = match RuntimeFile::discover(runtime_file) {
            Discovery::Live(instance) => forward(&client, &instance, &line).await,
            Discovery::Stale | Discovery::Absent => Err(Refusal::NotRunning),
        };

        let frame = match reply {
            Ok(body) => body,
            Err(refusal) => error_frame(&line, refusal).to_string(),
        };

        writeln!(stdout, "{frame}")?;
        stdout.flush()?;
    }

    Ok(())
}

/// POST one frame and return the reply body.
async fn forward(
    client: &reqwest::Client,
    instance: &RuntimeFile,
    frame: &str,
) -> Result<String, Refusal> {
    let response = client
        .post(format!("http://127.0.0.1:{}/mcp", instance.port))
        .header("content-type", "application/json")
        // rmcp requires a client to declare it accepts both, even when the server is
        // configured to answer JSON — the transport reserves the right to fall back to SSE.
        .header("accept", "application/json, text/event-stream")
        // Loopback, matching the request's own authority: the server's Host check rejects
        // anything else as a DNS-rebinding attempt (R-SEC-6).
        .header("host", format!("127.0.0.1:{}", instance.port))
        .body(frame.to_owned())
        .send()
        .await
        .map_err(|_| Refusal::NotRunning)?;

    response.text().await.map_err(|_| Refusal::NotRunning)
}

/// A JSON-RPC error carrying the id of the request it answers.
///
/// The id matters: a client correlates replies by it, and an error with a null id against a
/// request that had one is a reply the client will wait past forever.
fn error_frame(request: &str, refusal: Refusal) -> serde_json::Value {
    let id = serde_json::from_str::<serde_json::Value>(request)
        .ok()
        .and_then(|parsed| parsed.get("id").cloned())
        .unwrap_or(serde_json::Value::Null);

    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": {
            "code": JSON_RPC_ERROR_CODE,
            "message": refusal.message(),
            "data": { "reason": refusal.reason() },
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_refusal_answers_the_request_it_was_given() {
        // A client correlates by id. An error with a null id against a request that had one
        // is a reply the client waits past forever.
        let frame = error_frame(
            r#"{"jsonrpc":"2.0","id":7,"method":"tools/list"}"#,
            Refusal::NotRunning,
        );

        assert_eq!(frame["id"], 7);
        assert_eq!(frame["error"]["data"]["reason"], "not_running");
    }

    #[test]
    fn a_string_id_survives_as_a_string() {
        // JSON-RPC ids may be numbers or strings, and coercing one to the other breaks
        // correlation just as thoroughly as dropping it.
        let frame = error_frame(r#"{"id":"abc","method":"tools/list"}"#, Refusal::NotRunning);
        assert_eq!(frame["id"], "abc");
    }

    #[test]
    fn an_unparseable_request_still_produces_a_valid_frame() {
        // Garbage in must not become silence out: the client is waiting.
        let frame = error_frame("not json at all", Refusal::NotRunning);

        assert_eq!(frame["jsonrpc"], "2.0");
        assert!(frame["id"].is_null());
    }

    #[test]
    fn the_not_running_message_does_not_offer_to_launch_anything() {
        // FR-22, same rule as the extension: tools report that Curio is closed; they do not
        // start it behind the user's back.
        let message = Refusal::NotRunning.message();
        assert!(message.contains("Start Curio"), "{message}");
    }
}
