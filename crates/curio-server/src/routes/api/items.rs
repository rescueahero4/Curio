//! `/api/items` and `/api/bulk` — the library surface (FR-8, FR-10, FR-11).

use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use curio_core::domain::{Item, ItemStatus, LastEditedBy};
use curio_core::events::{Event, EventName};
use curio_core::query::{ItemQuery, Page};
use curio_db::items;

use crate::routes::error::{ApiError, ApiResult};
use crate::state::AppState;

/// `GET /api/items` — a filtered, searched, keyset-paged slice of the library.
///
/// Facets are **repeatable** query parameters (`?tag=a&tag=b`), which is what lets one
/// parameter carry a set without inventing a delimiter that a tag name could contain.
pub async fn list(
    State(state): State<AppState>,
    Query(raw): Query<Vec<(String, String)>>,
) -> ApiResult<Json<Page<Item>>> {
    let query = parse_query(&raw)?;
    Ok(Json(state.with_db(|db| items::list(db.conn(), &query))?))
}

/// `GET /api/items/:id`.
pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Item>> {
    Ok(Json(state.with_db(|db| items::require(db.conn(), &id))?))
}

/// What a PATCH accepts. Every field is optional; `null` clears the nullable ones.
#[derive(Debug, Default, Deserialize)]
pub struct PatchBody {
    pub name: Option<String>,
    pub short_description: Option<String>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub source_url: Option<Option<String>>,
    #[serde(default, deserialize_with = "explicit_null")]
    pub image_recipe: Option<Option<String>>,
    pub tags: Option<Vec<String>>,
    pub design_types: Option<Vec<String>>,
    pub family_ids: Option<Vec<String>>,
    pub status: Option<ItemStatus>,
}

/// Tell an absent field from one explicitly set to `null`.
///
/// Autosave PATCHes carry only what changed. Without this distinction `Option<String>`
/// collapses both cases to `None`, and every save would clear the fields it did not
/// mention — silently, on a field the user could not see.
fn explicit_null<'de, T, D>(deserializer: D) -> Result<Option<Option<T>>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    Deserialize::deserialize(deserializer).map(Some)
}

/// `PATCH /api/items/:id` — edit in place, stamping the user as the author (FR-8).
pub async fn patch(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<PatchBody>,
) -> ApiResult<Json<Item>> {
    let patch = items::ItemPatch {
        name: body.name,
        short_description: body.short_description,
        source_url: body.source_url,
        image_recipe: body.image_recipe,
        tags: body.tags,
        design_types: body.design_types,
        family_ids: body.family_ids,
        status: body.status,
    };

    let root = state.data_root().to_path_buf();
    let item = state.with_db_mut(|db| {
        items::patch(db.conn_mut(), Some(&root), &id, &patch, LastEditedBy::User)
    })?;

    // The dashboard that sent the PATCH suppresses its own echo with the pending-patch
    // guard (R-FE-13); every other open tab needs it.
    publish_item(&state, EventName::ItemUpdated, &item);
    Ok(Json(item))
}

/// `DELETE /api/items/:id`.
pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Response> {
    let root = state.data_root().to_path_buf();
    state.with_db_mut(|db| items::delete(db.conn_mut(), Some(&root), &id))?;

    state.publish(Event::item_deleted(&id));
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// How a user answered the gray-zone question (FR-7).
#[derive(Debug, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum ResolveBody {
    Accept,
    Reassign { family_id: String },
    AcceptProposal,
}

/// `POST /api/items/:id/resolve-grayzone` — one decision, and the badge clears.
pub async fn resolve_gray_zone(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<ResolveBody>,
) -> ApiResult<Json<Item>> {
    let decision = match body {
        ResolveBody::Accept => items::GrayZoneDecision::Accept,
        ResolveBody::AcceptProposal => items::GrayZoneDecision::AcceptProposal,
        ResolveBody::Reassign { family_id } => items::GrayZoneDecision::Reassign { family_id },
    };

    let root = state.data_root().to_path_buf();
    let item = state
        .with_db_mut(|db| items::resolve_gray_zone(db.conn_mut(), Some(&root), &id, &decision))?;

    publish_item(&state, EventName::ItemUpdated, &item);
    Ok(Json(item))
}

/// Which items a bulk operation acts on: explicit ids, or the current filter.
#[derive(Debug, Deserialize)]
pub struct BulkBody {
    #[serde(default)]
    pub ids: Option<Vec<String>>,
    /// The same query parameters `GET /api/items` takes, as pairs.
    #[serde(default)]
    pub filter: Option<Vec<(String, String)>>,
    #[serde(default)]
    pub add_tags: Vec<String>,
    #[serde(default)]
    pub remove_tags: Vec<String>,
    #[serde(default)]
    pub add_types: Vec<String>,
    #[serde(default)]
    pub remove_types: Vec<String>,
    #[serde(default)]
    pub add_families: Vec<String>,
    #[serde(default)]
    pub remove_families: Vec<String>,
    /// Delete every selected item. Mutually exclusive with the edits above.
    #[serde(default)]
    pub delete: bool,
}

#[derive(Debug, Serialize)]
pub struct BulkResult {
    pub changed: usize,
}

/// `POST /api/bulk/edit` — synchronous vocabulary edits or deletion over a frozen set.
///
/// Membership is resolved **once, here** (R-BE-18). A filter re-resolved later could act on
/// a different set than the one the user was looking at when they pressed the button.
pub async fn bulk_edit(
    State(state): State<AppState>,
    Json(body): Json<BulkBody>,
) -> ApiResult<Json<BulkResult>> {
    let ids = resolve_selection(&state, &body)?;
    curio_core::query::enforce_bulk_cap(ids.len())?;

    let root = state.data_root().to_path_buf();

    if body.delete {
        for id in &ids {
            state.with_db_mut(|db| items::delete(db.conn_mut(), Some(&root), id))?;
            state.publish(Event::item_deleted(id));
        }
        return Ok(Json(BulkResult { changed: ids.len() }));
    }

    let edit = items::BulkEdit {
        add_tags: body.add_tags,
        remove_tags: body.remove_tags,
        add_types: body.add_types,
        remove_types: body.remove_types,
        add_families: body.add_families,
        remove_families: body.remove_families,
    };
    if edit.is_empty() {
        return Err(ApiError(curio_core::Error::invalid(
            "a bulk edit needs at least one change",
        )));
    }

    let changed =
        state.with_db_mut(|db| items::bulk_edit(db.conn_mut(), Some(&root), &ids, &edit))?;
    for item in &changed {
        publish_item(&state, EventName::ItemUpdated, item);
    }
    state.publish(Event::vocabulary_updated());

    Ok(Json(BulkResult {
        changed: changed.len(),
    }))
}

fn resolve_selection(state: &AppState, body: &BulkBody) -> ApiResult<Vec<String>> {
    match (&body.ids, &body.filter) {
        (Some(ids), None) => Ok(ids.clone()),
        (None, Some(filter)) => {
            let query = parse_query(filter)?;
            Ok(state.with_db(|db| items::matching_ids(db.conn(), &query))?)
        }
        // Exactly one, never both and never neither. "Both" has no sensible meaning, and
        // "neither" would silently mean "every item in the library".
        _ => Err(ApiError(curio_core::Error::invalid(
            "a bulk operation takes either ids or a filter",
        ))),
    }
}

fn publish_item(state: &AppState, name: EventName, item: &Item) {
    match serde_json::to_value(item) {
        Ok(payload) => state.publish(Event::new(name, payload)),
        // The mutation already committed. Failing it now because the announcement could
        // not be serialized would trade a stale dashboard for lost work.
        Err(err) => tracing::error!(%err, "could not serialize an item for the event stream"),
    }
}

/// Turn repeatable query pairs into an [`ItemQuery`].
pub(crate) fn parse_query(raw: &[(String, String)]) -> ApiResult<ItemQuery> {
    let mut query = ItemQuery::unfiltered();
    let mut limit = None;

    for (key, value) in raw {
        match key.as_str() {
            "tag" => query.tags.push(value.clone()),
            "type" => query.types.push(value.clone()),
            "family" => query.families.push(value.clone()),
            "status" => {
                let status = ItemStatus::all()
                    .into_iter()
                    .find(|candidate| candidate.as_str() == value)
                    .ok_or_else(|| {
                        curio_core::Error::invalid(format!("{value} is not an item status"))
                    })?;
                query.statuses.push(status);
            }
            "q" => query.search = Some(value.clone()),
            "needs_review" => query.needs_review = value == "1" || value == "true",
            "limit" => limit = value.parse::<usize>().ok(),
            "cursor" => query.cursor = Some(curio_core::query::Cursor::parse(value)?),
            // Unknown keys are ignored rather than refused: a bookmarked URL from an older
            // build must still open the library, and a stray `utm_source` is not an error.
            _ => {}
        }
    }

    Ok(query.with_limit(limit))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(raw: &[(&str, &str)]) -> Vec<(String, String)> {
        raw.iter()
            .map(|(key, value)| ((*key).to_owned(), (*value).to_owned()))
            .collect()
    }

    #[test]
    fn repeated_facets_accumulate() {
        // One parameter carries a set without inventing a delimiter a tag name could
        // contain.
        let query =
            parse_query(&pairs(&[("tag", "a"), ("tag", "b"), ("family", "f")])).expect("parse");

        assert_eq!(query.tags, ["a", "b"]);
        assert_eq!(query.families, ["f"]);
        assert!(query.is_filtered());
    }

    #[test]
    fn an_unknown_status_is_refused_rather_than_ignored() {
        // Ignoring it would silently widen the result to the whole library, which reads as
        // a filter that does nothing.
        assert!(parse_query(&pairs(&[("status", "almost_ready")])).is_err());
    }

    #[test]
    fn an_unknown_parameter_does_not_break_a_bookmarked_url() {
        let query =
            parse_query(&pairs(&[("utm_source", "somewhere"), ("q", "dark")])).expect("parse");

        assert_eq!(query.search.as_deref(), Some("dark"));
    }

    #[test]
    fn the_page_size_is_clamped() {
        assert_eq!(
            parse_query(&pairs(&[("limit", "9000")]))
                .expect("parse")
                .limit,
            200
        );
        assert_eq!(parse_query(&pairs(&[])).expect("parse").limit, 60);
    }

    #[test]
    fn a_malformed_cursor_is_refused() {
        // Silently starting over would loop the grid: the client asks for the next page,
        // receives page one, and appends it.
        assert!(parse_query(&pairs(&[("cursor", "nonsense")])).is_err());
    }

    #[test]
    fn needs_review_accepts_the_forms_the_client_sends() {
        assert!(
            parse_query(&pairs(&[("needs_review", "1")]))
                .expect("parse")
                .needs_review
        );
        assert!(
            parse_query(&pairs(&[("needs_review", "true")]))
                .expect("parse")
                .needs_review
        );
        assert!(
            !parse_query(&pairs(&[("needs_review", "0")]))
                .expect("parse")
                .needs_review
        );
    }

    #[test]
    fn a_resolve_body_parses_each_of_the_three_decisions() {
        for raw in [
            r#"{"action":"accept"}"#,
            r#"{"action":"accept_proposal"}"#,
            r#"{"action":"reassign","family_id":"01F"}"#,
        ] {
            serde_json::from_str::<ResolveBody>(raw).unwrap_or_else(|err| panic!("{raw}: {err}"));
        }
    }

    #[test]
    fn a_patch_can_clear_a_nullable_field_and_leave_others_alone() {
        // The three-state problem: absent means "leave alone", null means "clear". Folding
        // them together would make every save wipe the fields it did not mention.
        let body: PatchBody = serde_json::from_str(r#"{"source_url": null}"#).expect("parse");

        assert_eq!(body.source_url, Some(None));
        assert_eq!(body.image_recipe, None);
    }
}
