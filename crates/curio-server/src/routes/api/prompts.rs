//! `/api/prompts` — composing, serializing, and sending (FR-12..FR-16).
//!
//! **The dashboard never serializes a prompt** (R-FE-18). It stores TipTap JSON and renders
//! friendly chips; the text a user pastes is produced here, from the same code the MCP
//! `prompt_get` tool and the `prompts/{id}.md` snapshot use. One definition of what a chip
//! expands to, in one place, for three consumers.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use curio_core::domain::{Prompt, VocabularyKind};
use curio_core::prompt::ChipContext;
use curio_db::{prompts, vocabulary};

use crate::routes::error::ApiResult;
use crate::state::AppState;

/// `GET /api/prompts`.
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vec<Prompt>>> {
    Ok(Json(state.with_db(|db| prompts::list(db.conn()))?))
}

#[derive(Debug, Serialize)]
pub struct Template {
    pub doc_json: serde_json::Value,
    /// The seven sections, so the editor can render ghost text without re-deriving the
    /// list. The server owns the template; a client that hard-coded it would drift.
    pub sections: Vec<Section>,
}

#[derive(Debug, Serialize)]
pub struct Section {
    pub id: &'static str,
    pub heading: &'static str,
    pub body: &'static str,
}

/// `GET /api/prompts/template` — the gold-standard scaffold (FR-12).
pub async fn template() -> Json<Template> {
    Json(Template {
        doc_json: curio_core::prompt::starter_document(),
        sections: curio_core::prompt::SECTIONS
            .iter()
            .map(|section| Section {
                id: section.id,
                heading: section.heading,
                body: section.body,
            })
            .collect(),
    })
}

/// `GET /api/prompts/:id`.
pub async fn get(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<Prompt>> {
    Ok(Json(state.with_db(|db| prompts::require(db.conn(), &id))?))
}

#[derive(Debug, Default, Deserialize)]
pub struct CreateBody {
    pub title: Option<String>,
}

/// `POST /api/prompts` — a new prompt, pre-filled from the template.
pub async fn create(
    State(state): State<AppState>,
    body: Option<Json<CreateBody>>,
) -> ApiResult<Json<Prompt>> {
    let title = body.and_then(|Json(body)| body.title);
    Ok(Json(state.with_db(|db| {
        prompts::create(db.conn(), title.as_deref())
    })?))
}

/// The autosave body.
///
/// There is no `title` here, and its absence is the design: a prompt is named by its own
/// first line, derived server-side on every save. Accepting a title would let a client set
/// one that the next keystroke silently overwrote — a write that appears to succeed and does
/// not survive is worse than one that was never offered.
#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    /// The TipTap document. Sent as a JSON value rather than a string so a malformed
    /// document cannot reach the database as valid-looking text.
    pub doc_json: Option<serde_json::Value>,
}

/// `PATCH /api/prompts/:id` — autosave (FR-16).
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateBody>,
) -> ApiResult<Json<Prompt>> {
    Ok(Json(state.with_db(|db| {
        prompts::update(db.conn(), &id, body.doc_json.as_ref())
    })?))
}

/// `DELETE /api/prompts/:id`.
pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Response> {
    let root = state.data_root().to_path_buf();
    state.with_db_mut(|db| prompts::delete(db.conn_mut(), Some(&root), &id))?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

#[derive(Debug, Serialize)]
pub struct Serialized {
    pub text: String,
    /// Where the snapshot was written, so the UI can offer "reveal in folder" and an agent
    /// can be pointed at a file rather than handed a blob.
    pub path: String,
}

/// `POST /api/prompts/:id/serialize` — expand the chips and write the snapshot (FR-14).
pub async fn serialize(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Serialized>> {
    let root = state.data_root().to_path_buf();

    let (text, prompt) = state.with_db(|db| {
        let prompt = prompts::require(db.conn(), &id)?;
        let context = chip_context(db, &root)?;
        Ok::<_, curio_db::Error>((
            curio_core::prompt::serialize(&prompt.doc_json, &context),
            prompt,
        ))
    })?;

    state.with_db(|db| prompts::save_serialized(db.conn(), Some(&root), &prompt.id, &text))?;

    Ok(Json(Serialized {
        text,
        path: root
            .join("prompts")
            .join(format!("{id}.md"))
            .display()
            .to_string(),
    }))
}

/// `POST /api/prompts/:id/sent` — stake the claim on the next project folder (FR-16).
pub async fn mark_sent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Prompt>> {
    Ok(Json(
        state.with_db(|db| prompts::mark_sent(db.conn(), &id))?,
    ))
}

/// `DELETE /api/prompts/:id/sent` — withdraw it.
pub async fn clear_sent(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Response> {
    state.with_db(|db| prompts::clear_sent(db.conn(), &id))?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// Everything the chips in any prompt could refer to.
///
/// Loaded whole rather than looked up per chip: a prompt cites a handful of entities out of
/// a library of thousands, but the whole vocabulary is three small queries while per-chip
/// lookups are N round trips through a mutex the worker also wants.
pub(crate) fn chip_context(
    db: &curio_db::Db,
    root: &std::path::Path,
) -> Result<ChipContext, curio_db::Error> {
    let mut context = ChipContext::default();

    for family in vocabulary::list_families(db.conn())? {
        context
            .families
            .insert(family.id, (family.name, family.description));
    }
    for kind in [VocabularyKind::Tag, VocabularyKind::DesignType] {
        for term in vocabulary::list_terms(db.conn(), kind)? {
            context.terms.insert(term.id, term.name);
        }
    }

    // Items are the expensive half, so only the fields a chip expands to are read — and
    // the directory is absolute, because that is the entire point of an item chip (FR-14).
    let mut statement = db.conn().prepare("SELECT id, name FROM items")?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let id: String = row.get(0)?;
        let name: String = row.get(1)?;
        let directory = curio_core::paths::item_dir(root, &id).display().to_string();
        context.items.insert(id, (name, directory));
    }

    Ok(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn the_template_carries_the_document_and_its_sections() {
        // The server owns the template; a client that hard-coded the seven sections would
        // drift the moment FR-12 changed.
        let Json(template) = template().await;

        assert_eq!(template.doc_json["type"], "doc");
        assert_eq!(template.sections.len(), 7);
        assert_eq!(template.sections[0].id, "brief");
        assert!(!template.sections[0].body.is_empty());

        // The client needs both halves: the document it opens the editor on, and the bodies
        // again as the ghost text to show over any section the user later empties.
        let content = template.doc_json["content"].as_array().expect("content");
        assert_eq!(
            content.iter().filter(|node| node["type"] == "heading").count(),
            template.sections.len()
        );
        assert_eq!(content[1]["content"][0]["text"], template.sections[0].body);
    }

    #[test]
    fn an_autosave_that_still_carries_a_title_is_accepted_and_the_title_ignored() {
        // The field was removed, not rejected. A stale tab left open across an update keeps
        // PATCHing the old shape, and answering it with 422 would break autosave for a user
        // who did nothing wrong — the document still saves, and the name is derived from it
        // either way.
        let body: UpdateBody = serde_json::from_value(serde_json::json!({
            "title": "A name the client invented",
            "doc_json": { "type": "doc", "content": [] },
        }))
        .expect("a body carrying a title still deserializes");

        assert!(body.doc_json.is_some());
    }
}
