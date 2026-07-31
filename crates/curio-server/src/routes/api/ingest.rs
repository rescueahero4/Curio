//! `POST /api/items` — a capture becomes a card (FR-2, FR-3).
//!
//! Multipart, because a screenshot is **mandatory for every item** (FR-2) and base64 in
//! JSON would inflate a 20,000-pixel stitch by a third against a 64 MB body cap.
//!
//! The ordering matters: the screenshot is written, the row is inserted as `processing`, and
//! the item is announced — **before** any model is involved. A capture is a card the instant
//! it lands, and an item whose assessment never runs is still an item the user can find,
//! rename, and use (FR-3, FR-26).

use axum::Json;
use axum::extract::{Multipart, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

use curio_core::domain::{Item, JobKind};
use curio_core::events::{Event, EventName};
use curio_db::{items, jobs};

use crate::routes::error::{ApiError, ApiResult};
use crate::state::AppState;

/// The screenshot every item must have.
const SCREENSHOT_FILE: &str = "screenshot.png";

/// The grid's copy. The name must contain "thumb": the file route falls back to the
/// screenshot on a 404 only for filenames that do (Inventory §10.20).
const THUMBNAIL_FILE: &str = "thumb.jpg";

#[derive(Debug, Serialize)]
pub struct Ingested {
    pub item_id: String,
    pub job_id: String,
    pub item: Item,
}

/// Accept a capture.
pub async fn create(State(state): State<AppState>, mut form: Multipart) -> ApiResult<Response> {
    let mut screenshot: Option<Vec<u8>> = None;
    let mut source_url = None;
    let mut title = None;
    let mut viewport_width: Option<f64> = None;
    let mut viewport_height: Option<f64> = None;

    while let Some(field) = form.next_field().await.map_err(|err| {
        ApiError(curio_core::Error::invalid(format!(
            "malformed upload: {err}"
        )))
    })? {
        match field.name().unwrap_or_default().to_owned().as_str() {
            "screenshot" => {
                screenshot = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|err| {
                            ApiError(curio_core::Error::invalid(format!(
                                "could not read the screenshot: {err}"
                            )))
                        })?
                        .to_vec(),
                );
            }
            // `url` is accepted alongside `source_url` because the extension has sent that
            // name since before the rewrite (Inventory §1).
            "source_url" | "url" => source_url = field.text().await.ok().filter(|s| !s.is_empty()),
            "title" => title = field.text().await.ok().filter(|s| !s.is_empty()),
            // The viewport is what makes "the first fold" a real measurement rather than a
            // guess, and this is the only place it exists — the extension sends it with
            // the capture and nothing stores it (R-BE-26).
            "viewport_width" => {
                viewport_width = field
                    .text()
                    .await
                    .ok()
                    .and_then(|v| v.trim().parse::<f64>().ok())
            }
            "viewport_height" => {
                viewport_height = field
                    .text()
                    .await
                    .ok()
                    .and_then(|v| v.trim().parse::<f64>().ok())
            }
            // Unknown fields are ignored rather than refused: the extension sends
            // `captured_at` and viewport dimensions that nothing reads yet, and rejecting
            // an upload over a field we do not use would break capture for no gain.
            _ => {}
        }
    }

    let Some(bytes) = screenshot.filter(|bytes| !bytes.is_empty()) else {
        return Err(ApiError(curio_core::Error::invalid(
            "a screenshot is required — every item in the library has one",
        )));
    };

    let root = state.data_root().to_path_buf();
    let name = title.unwrap_or_else(|| default_name(source_url.as_deref()));

    // Written under a temporary id first: the real id is minted by the insert, and an
    // upload that fails after the file lands would otherwise leave an orphaned directory
    // no row ever references.
    let item = state.with_db_mut(|db| {
        items::create(
            db.conn_mut(),
            Some(&root),
            &items::NewItem {
                name,
                source_url,
                // The path is relative to the data root, so the library survives being
                // moved (R-DA-1).
                screenshot_path: String::new(),
                thumbnail_path: None,
            },
        )
    })?;

    let directory = curio_core::paths::item_dir(&root, &item.id);
    std::fs::create_dir_all(&directory)?;
    std::fs::write(directory.join(SCREENSHOT_FILE), &bytes)?;

    let relative = format!("items/{}/{SCREENSHOT_FILE}", item.id);

    // Built here rather than in the worker: this is the only moment the viewport is known,
    // and a card that shows a full 20,000-pixel stitch scaled to fit is a grey smear that
    // identifies nothing (R-BE-26). A failure costs the thumbnail, never the capture.
    let aspect = match (viewport_width, viewport_height) {
        (Some(width), Some(height)) if height > 0.0 => Some(width / height),
        _ => None,
    };
    let thumbnail = crate::images::thumbnail(&bytes, aspect);
    let thumbnail_relative = if thumbnail.processed
        && std::fs::write(directory.join(THUMBNAIL_FILE), &thumbnail.bytes).is_ok()
    {
        Some(format!("items/{}/{THUMBNAIL_FILE}", item.id))
    } else {
        // R-BE-26 degrades rather than fails. `None` means the grid serves the screenshot,
        // which is correct and merely heavier.
        None
    };

    let item = state.with_db(|db| {
        items::set_media(
            db.conn(),
            Some(&root),
            &item.id,
            &relative,
            thumbnail_relative.as_deref(),
        )
    })?;

    // Enqueued, not run. The worker is what turns this into a description; until it does,
    // the item is visible, editable, and honestly labelled `processing` (FR-26).
    let job = state.with_db(|db| {
        jobs::enqueue(
            db.conn(),
            JobKind::AssessItem,
            &serde_json::json!({ "item_id": item.id }),
        )
    })?;

    if let Ok(payload) = serde_json::to_value(&item) {
        state.publish(Event::new(EventName::ItemCreated, payload));
    }
    if let Ok(payload) = serde_json::to_value(&job) {
        state.publish(Event::new(EventName::JobUpdated, payload));
    }
    // Start assessing now rather than at the next two-second tick. The poll would find it
    // anyway (Inventory §9); this is what makes a capture feel immediate.
    state.wake_worker();

    Ok((
        StatusCode::CREATED,
        Json(Ingested {
            item_id: item.id.clone(),
            job_id: job.id,
            item,
        }),
    )
        .into_response())
}

/// A name for a capture that arrived without one.
///
/// The hostname, because "stripe.com" tells a user what they captured and "Untitled" does
/// not — and a name the assessment will replace anyway should still be useful in the
/// seconds before it does.
fn default_name(source_url: Option<&str>) -> String {
    source_url
        .and_then(|url| {
            url.split("://")
                .nth(1)
                .or(Some(url))?
                .split('/')
                .next()
                .filter(|host| !host.is_empty())
                .map(|host| host.trim_start_matches("www.").to_owned())
        })
        .unwrap_or_else(|| "Untitled capture".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_capture_is_named_after_where_it_came_from() {
        // "stripe.com" tells the user what they captured in the seconds before the
        // assessment replaces it; "Untitled" tells them nothing.
        assert_eq!(
            default_name(Some("https://stripe.com/pricing")),
            "stripe.com"
        );
        assert_eq!(default_name(Some("https://www.linear.app/")), "linear.app");
        assert_eq!(
            default_name(Some("http://localhost:3000/x")),
            "localhost:3000"
        );
    }

    #[test]
    fn a_capture_with_no_url_still_gets_a_name() {
        assert_eq!(default_name(None), "Untitled capture");
        assert_eq!(default_name(Some("")), "Untitled capture");
    }
}
