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
//!
//! **A blank line is not nothing.** The client splits stdout on `\n` and hands each piece
//! to `JSON.parse`, so an empty piece is a parse error, not a no-op. That is why forwarding
//! is allowed to produce *no* output but never *empty* output — see [`reply_for`], which
//! owns that decision so it can be tested without a socket.

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
            Ok(Some(body)) => body,
            Ok(None) => continue,

            // A notification is owed no reply even when the forward fails. An error frame
            // carries a null id, which is the same defect the blank line was: a frame the
            // client cannot correlate with anything it sent. The next request it makes gets
            // the refusal properly, so nothing is concealed by staying quiet here.
            Err(refusal) if is_notification(&line) => {
                eprintln!("curio: dropped a notification — {}", refusal.message());
                continue;
            }

            Err(refusal) => error_frame(&line, refusal).to_string(),
        };

        writeln!(stdout, "{frame}")?;
        stdout.flush()?;
    }

    Ok(())
}

/// POST one frame and return what should be written back, if anything.
async fn forward(
    client: &reqwest::Client,
    instance: &RuntimeFile,
    frame: &str,
) -> Result<Option<String>, Refusal> {
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

    let status = response.status().as_u16();
    let body = response.text().await.map_err(|_| Refusal::NotRunning)?;

    Ok(reply_for(status, &body, frame))
}

/// `202 Accepted` — the Streamable HTTP transport's answer to a POST it owes no reply for.
const ACCEPTED: u16 = 202;

/// What to write for one exchange. `None` means write nothing at all.
///
/// There are two ways a body legitimately arrives empty and only one of them is silence:
///
/// - **`202 Accepted`.** Per the MCP spec, a POST whose body holds only notifications is
///   answered with no content. Nothing is owed, so nothing is written — writing the empty
///   body is what produced the bare newline that clients choke on.
/// - **Anything else.** A 500 with no body, or a response truncated mid-flight. If the frame
///   carried an `id` the client is blocked on that id until something answers it, so this
///   becomes an error frame rather than a silence (the rule the tests below call *garbage in
///   must not become silence out*).
///
/// The body is trimmed on the way through for the same reason the empty case exists at all:
/// `writeln!` adds the newline, so a body that already ends in one would write a blank line
/// after the frame and reintroduce the bug one line later.
fn reply_for(status: u16, body: &str, request: &str) -> Option<String> {
    let body = body.trim();

    if status == ACCEPTED {
        return None;
    }
    if !body.is_empty() {
        return Some(body.to_owned());
    }
    if is_notification(request) {
        return None;
    }

    Some(error_frame(request, Refusal::EmptyReply).to_string())
}

/// Whether a frame is a JSON-RPC notification — a well-formed object carrying no `id`.
///
/// Unparseable text is deliberately **not** a notification. It lacks an id because it lacks
/// structure, not because the client waived its reply, and a client that sent something
/// malformed is still waiting to hear about it.
///
/// A batch (a JSON array) is not one either. It may contain requests, and answering the
/// whole batch with silence would strand every id inside it.
fn is_notification(frame: &str) -> bool {
    serde_json::from_str::<serde_json::Value>(frame)
        .ok()
        .and_then(|parsed| parsed.as_object().map(|object| !object.contains_key("id")))
        .unwrap_or(false)
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
    fn a_forwarded_notification_produces_no_output_at_all() {
        // The bug this file was opened for. `notifications/initialized` is answered 202 with
        // an empty body, and writing that body emitted a bare `\n` — which the client splits
        // out as a frame and hands to `JSON.parse`, producing "Unexpected end of JSON input"
        // and a connector badged **failed** on every single connection.
        assert_eq!(
            reply_for(
                ACCEPTED,
                "",
                r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#
            ),
            None
        );
    }

    #[test]
    fn a_request_carrying_an_id_never_resolves_to_silence() {
        // The other half, and the reason this is not a blanket empty-body skip: a client
        // correlates by id and will wait on that id forever. An empty body against a frame
        // that expected a reply must still answer it.
        let frame = reply_for(500, "", r#"{"jsonrpc":"2.0","id":4,"method":"tools/list"}"#)
            .expect("a request must be answered");
        let parsed: serde_json::Value = serde_json::from_str(&frame).expect("valid JSON");

        assert_eq!(parsed["id"], 4);
        assert_eq!(parsed["error"]["data"]["reason"], "empty_reply");
    }

    #[test]
    fn an_empty_reply_does_not_claim_curio_is_closed() {
        // Curio answered — it just answered with nothing. Telling the user to start an app
        // that is already running sends them somewhere there is nothing to fix.
        let message = Refusal::EmptyReply.message();
        assert!(!message.contains("Start Curio"), "{message}");
    }

    #[test]
    fn a_body_that_already_ends_in_a_newline_does_not_write_a_blank_line() {
        // `writeln!` supplies the newline. A body carrying its own would put an empty line
        // after a perfectly good frame and reintroduce the same parse error one line later.
        let frame = reply_for(200, "{\"jsonrpc\":\"2.0\",\"id\":1}\n", r#"{"id":1}"#)
            .expect("a reply with a body is written");

        assert!(!frame.ends_with('\n'), "{frame:?}");
    }

    #[test]
    fn unparseable_input_is_not_mistaken_for_a_notification() {
        // Garbage has no id because it has no structure, not because the client waived its
        // reply. Silence here would strand a client that sent something malformed.
        assert!(!is_notification("not json at all"));
        assert!(reply_for(500, "", "not json at all").is_some());
    }

    #[test]
    fn a_batch_is_not_a_notification() {
        // An array may hold requests. Answering the whole batch with silence would strand
        // every id inside it.
        assert!(!is_notification(
            r#"[{"jsonrpc":"2.0","id":1,"method":"tools/list"}]"#
        ));
    }

    #[test]
    fn the_not_running_message_does_not_offer_to_launch_anything() {
        // FR-22, same rule as the extension: tools report that Curio is closed; they do not
        // start it behind the user's back.
        let message = Refusal::NotRunning.message();
        assert!(message.contains("Start Curio"), "{message}");
    }
}
