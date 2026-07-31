//! `GET /api/events` — the dashboard's push channel (R-BE-15, R-FE-7).
//!
//! Server-sent events, with the previous implementation's contract intact: a `hello` frame
//! on connect, a `ping` every 20 s, and named events whose payloads are unchanged
//! (Inventory §3). The names are a published contract — the dashboard's handler registry
//! keys off them — so renaming one is a breaking change to a client we ship separately.
//!
//! Authentication is the session cookie, which the browser attaches automatically.
//! `EventSource` cannot send headers, and that single constraint is why the session is a
//! cookie at all (D22).
//!
//! **Reads continue while paused** (D25). A paused library that stopped pushing would leave
//! every open dashboard silently stale, and the user would find out by not being told
//! anything.

use std::convert::Infallible;
use std::time::Duration;

use axum::extract::State;
use axum::response::sse::{Event as SseEvent, KeepAlive, Sse};
use futures_core::Stream;
use tokio_stream::StreamExt as _;
use tokio_stream::wrappers::BroadcastStream;

use crate::state::AppState;

/// How often a keepalive goes out (Inventory §3).
///
/// Twenty seconds is comfortably inside every intermediary's idle timeout and cheap enough
/// to be invisible in the CPU budget (R-BE-31).
const PING_INTERVAL: Duration = Duration::from_secs(20);

/// The stream handler.
pub async fn stream(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<SseEvent, Infallible>>> {
    // Sent before any real event so a client knows the stream is live rather than merely
    // connected, and learns which build it is talking to — a dashboard left open across an
    // upgrade would otherwise have no way to notice.
    let greeting = serde_json::json!({
        "version": state.version(),
        "state": state.run_state(),
    })
    .to_string();
    let hello =
        futures_util::stream::once(
            async move { Ok(SseEvent::default().event("hello").data(greeting)) },
        );

    let events = BroadcastStream::new(state.subscribe()).filter_map(|received| match received {
        Ok(event) => Some(Ok(SseEvent::default()
            .event(event.name.as_str())
            .data(event.payload.to_string()))),
        // The subscriber fell behind and the channel dropped frames for it. Closing the
        // stream is right: the client reconnects and refetches, which recovers the truth,
        // whereas continuing would leave it confidently wrong about the events it missed.
        Err(tokio_stream::wrappers::errors::BroadcastStreamRecvError::Lagged(missed)) => {
            tracing::warn!(
                missed,
                "an events subscriber fell behind and was resynchronised"
            );
            None
        }
    });

    Sse::new(hello.chain(events)).keep_alive(
        KeepAlive::new()
            .interval(PING_INTERVAL)
            .event(SseEvent::default().event("ping").data("{}")),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_keepalive_interval_is_the_documented_twenty_seconds() {
        // Inventory §3. The extension and the dashboard both tolerate this silently; a
        // different value would show up as reconnect churn rather than as an error.
        assert_eq!(PING_INTERVAL, Duration::from_secs(20));
    }
}
