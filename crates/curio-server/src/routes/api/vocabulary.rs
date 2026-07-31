//! `/api/vocabulary` — families, design types, and tags (FR-11).
//!
//! Every write here can change what an arbitrary number of items are *called*, so each one
//! ends by rebuilding the affected items' search rows and sidecars. A rename that left a
//! sidecar naming the old family would be exactly the disagreement R-DA-4 forbids, and it
//! would be invisible until an agent read the library and disagreed with the dashboard.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use curio_core::domain::{Family, Term, VocabularyKind};
use curio_core::events::Event;
use curio_db::vocabulary;

use crate::routes::error::{ApiError, ApiResult};
use crate::state::AppState;

#[derive(Debug, Serialize)]
pub struct Vocabulary {
    pub families: Vec<Family>,
    pub design_types: Vec<Term>,
    pub tags: Vec<Term>,
}

/// `GET /api/vocabulary` — everything the filter bar and the pickers need, in one call.
///
/// One request rather than three because every consumer wants all three at once: the filter
/// row, the slash-command picker, and the bulk edit panels each render the whole vocabulary.
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vocabulary>> {
    Ok(Json(state.with_db(|db| {
        Ok::<_, curio_db::Error>(Vocabulary {
            families: vocabulary::list_families(db.conn())?,
            design_types: vocabulary::list_terms(db.conn(), VocabularyKind::DesignType)?,
            tags: vocabulary::list_terms(db.conn(), VocabularyKind::Tag)?,
        })
    })?))
}

#[derive(Debug, Deserialize)]
pub struct CreateBody {
    pub name: String,
    #[serde(default)]
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct Created {
    pub id: String,
}

/// `POST /api/vocabulary/:kind`.
pub async fn create(
    State(state): State<AppState>,
    Path(kind): Path<String>,
    Json(body): Json<CreateBody>,
) -> ApiResult<Json<Created>> {
    let kind = parse_kind(&kind)?;
    let id = state.with_db(|db| {
        vocabulary::create(
            db.conn(),
            kind,
            &body.name,
            &body.description,
            &curio_core::time::now_iso(),
        )
    })?;

    state.publish(Event::vocabulary_updated());
    Ok(Json(Created { id }))
}

#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    pub name: Option<String>,
    pub description: Option<String>,
}

/// `PATCH /api/vocabulary/:kind/:id` — rename, or edit a family's description.
pub async fn update(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, String)>,
    Json(body): Json<UpdateBody>,
) -> ApiResult<Response> {
    let kind = parse_kind(&kind)?;
    let root = state.data_root().to_path_buf();

    state.with_db_mut(|db| {
        vocabulary::update(
            db.conn(),
            kind,
            &id,
            body.name.as_deref(),
            body.description.as_deref(),
            &curio_core::time::now_iso(),
        )?;
        rebuild_linked(db, Some(&root), kind, &id)
    })?;

    announce(&state);
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `DELETE /api/vocabulary/:kind/:id`.
///
/// The items survive; only the name and its links go. Deleting a tag must never be a way to
/// lose a capture.
pub async fn delete(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, String)>,
) -> ApiResult<Response> {
    let kind = parse_kind(&kind)?;
    let root = state.data_root().to_path_buf();

    state.with_db_mut(|db| {
        // Read the affected items *before* the delete: afterwards the link rows are gone
        // by ON DELETE CASCADE and there is nothing left to ask.
        let touched = vocabulary::linked_item_ids(db.conn(), kind, &id)?;
        vocabulary::delete(db.conn(), kind, &id)?;
        rebuild(db, Some(&root), &touched)
    })?;

    announce(&state);
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Deserialize)]
pub struct MergeBody {
    pub into: String,
}

/// `POST /api/vocabulary/:kind/:id/merge` — fold one entry into another.
pub async fn merge(
    State(state): State<AppState>,
    Path((kind, id)): Path<(String, String)>,
    Json(body): Json<MergeBody>,
) -> ApiResult<Response> {
    let kind = parse_kind(&kind)?;
    let root = state.data_root().to_path_buf();

    state.with_db_mut(|db| {
        let touched = vocabulary::merge(db.conn(), kind, &id, &body.into)?;
        rebuild(db, Some(&root), &touched)
    })?;

    announce(&state);
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `POST /api/vocabulary/prune` — drop AI-coined names nothing points at.
///
/// Explicit rather than automatic on every delete: pruning is cheap but not free, and a
/// user watching their vocabulary list should see it shrink because they asked, not as a
/// side effect they have to reverse-engineer.
pub async fn prune(State(state): State<AppState>) -> ApiResult<Json<Pruned>> {
    let removed = state.with_db(|db| vocabulary::prune_orphans(db.conn()))?;
    announce(&state);
    Ok(Json(Pruned { removed }))
}

#[derive(Debug, Serialize)]
pub struct Pruned {
    pub removed: usize,
}

fn rebuild_linked(
    db: &mut curio_db::Db,
    root: Option<&std::path::Path>,
    kind: VocabularyKind,
    id: &str,
) -> Result<(), curio_db::Error> {
    let touched = vocabulary::linked_item_ids(db.conn(), kind, id)?;
    rebuild(db, root, &touched)
}

/// Rewrite the search rows and sidecars of every item a vocabulary change touched.
fn rebuild(
    db: &mut curio_db::Db,
    root: Option<&std::path::Path>,
    ids: &[String],
) -> Result<(), curio_db::Error> {
    for id in ids {
        curio_db::items::touch(db.conn(), root, id)?;
    }
    Ok(())
}

/// `vocabulary.updated` carries `{}` by contract — consumers refetch rather than reconcile,
/// because a merge or a delete can touch arbitrarily many rows (Inventory §3).
fn announce(state: &AppState) {
    state.publish(Event::vocabulary_updated());
}

/// Map the URL segment to a kind.
///
/// `types` rather than `design_types` in the URL because that is what the previous
/// implementation published and what existing clients call.
fn parse_kind(raw: &str) -> ApiResult<VocabularyKind> {
    match raw {
        "families" => Ok(VocabularyKind::Family),
        "types" => Ok(VocabularyKind::DesignType),
        "tags" => Ok(VocabularyKind::Tag),
        other => Err(ApiError(curio_core::Error::invalid(format!(
            "{other} is not a vocabulary — expected families, types, or tags"
        )))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_url_segments_are_the_ones_clients_already_use() {
        assert_eq!(
            parse_kind("families").expect("families"),
            VocabularyKind::Family
        );
        assert_eq!(
            parse_kind("types").expect("types"),
            VocabularyKind::DesignType
        );
        assert_eq!(parse_kind("tags").expect("tags"), VocabularyKind::Tag);
    }

    #[test]
    fn an_unknown_segment_says_what_was_expected() {
        let error = parse_kind("colours").expect_err("refused");
        let message = error.0.to_string();

        assert!(message.contains("families"), "{message}");
        assert!(message.contains("tags"), "{message}");
    }
}
