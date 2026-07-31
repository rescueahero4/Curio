//! `GET /ws` — the extension's push channel (R-BE-32, D13, D23).
//!
//! A second transport rather than a second copy of the first, because the two clients have
//! incompatible constraints. An MV3 service worker is killed after 30 s idle unless
//! something keeps it warm, and a browser `WebSocket` can send **neither headers nor a body**
//! on connect — so the token cannot ride in an `Authorization` header, and putting it in the
//! query string would write it into every log that records a request line.
//!
//! Hence auth by first message, with a deadline: the client sends the runtime token within
//! five seconds or the socket closes. The server answers `hello {state, version}` and
//! thereafter pushes `state` on every pause and resume.
//!
//! **Pausing does not close the socket.** The paused state is announced, not enforced by
//! disconnection — a disconnected extension cannot tell "paused" from "not running", and
//! FR-22 requires it to say which.

use std::time::Duration;

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::Response;

use crate::state::AppState;

/// How long the client has to prove itself (R-BE-32).
const AUTH_DEADLINE: Duration = Duration::from_secs(5);

/// How often the server checks whether the run state changed.
///
/// A poll rather than a subscription because pause/resume is not an [`curio_core::events`]
/// event — it is a mode, not a thing that happened, and adding it to the event vocabulary
/// would put it on the dashboard's SSE stream too, where the session probe already answers
/// it more directly.
const STATE_POLL: Duration = Duration::from_secs(1);

/// Upgrade the connection. The identity layer has already run; the credential has not.
pub async fn upgrade(upgrade: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    upgrade.on_upgrade(move |socket| serve(socket, state))
}

async fn serve(mut socket: WebSocket, state: AppState) {
    if !authenticate(&mut socket, &state).await {
        // Closed without a reason frame: a client that failed the deadline learns only
        // that the socket went away, which is all an unauthenticated caller is owed.
        let _ = socket.send(Message::Close(None)).await;
        return;
    }

    if send_state(&mut socket, &state, "hello").await.is_err() {
        return;
    }

    let mut announced = state.run_state();
    let mut ticker = tokio::time::interval(STATE_POLL);

    loop {
        tokio::select! {
            incoming = socket.recv() => match incoming {
                // The client's 20 s keepalive. Tolerated silently — it carries nothing and
                // exists to keep an MV3 worker alive (R-BE-32).
                Some(Ok(Message::Ping(_) | Message::Text(_) | Message::Pong(_))) => {}
                Some(Ok(Message::Close(_))) | None | Some(Err(_)) => break,
                Some(Ok(Message::Binary(_))) => {}
            },
            _ = ticker.tick() => {
                let current = state.run_state();
                if current != announced {
                    announced = current;
                    if send_state(&mut socket, &state, "state").await.is_err() {
                        break;
                    }
                }
            }
        }
    }
}

/// Read the first message and check it against the runtime token.
///
/// A timeout rather than an indefinite wait: an unauthenticated socket costs a task and a
/// buffer, and a client that connects and says nothing is either broken or probing.
async fn authenticate(socket: &mut WebSocket, state: &AppState) -> bool {
    let first = tokio::time::timeout(AUTH_DEADLINE, socket.recv()).await;

    match first {
        Ok(Some(Ok(Message::Text(presented)))) => state.token().verify(presented.trim()),
        _ => false,
    }
}

async fn send_state(
    socket: &mut WebSocket,
    state: &AppState,
    kind: &str,
) -> Result<(), axum::Error> {
    let frame = serde_json::json!({
        "type": kind,
        "state": state.run_state(),
        "version": state.version(),
    });
    socket.send(Message::Text(frame.to_string().into())).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_auth_deadline_is_the_documented_five_seconds() {
        // R-BE-32. Longer, and an unauthenticated socket holds a task and a buffer for as
        // long as a prober cares to keep it open.
        assert_eq!(AUTH_DEADLINE, Duration::from_secs(5));
    }

    #[test]
    fn the_handshake_frame_carries_the_state_and_the_version() {
        // D23: `hello {state, version}`. The extension renders its status dot from this
        // and must be able to say "paused" rather than "not running" (FR-22).
        let frame = serde_json::json!({
            "type": "hello",
            "state": curio_runtime::State::Paused,
            "version": "0.1.0",
        });

        assert_eq!(frame["state"], "paused");
        assert_eq!(frame["version"], "0.1.0");
    }
}
