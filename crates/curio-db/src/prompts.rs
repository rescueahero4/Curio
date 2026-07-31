//! Prompts: stored documents, serialized text, and the claim a sent prompt stakes.

use rusqlite::{Connection, OptionalExtension as _, Row};

use curio_core::domain::Prompt;

use crate::{Error, Result, sidecars};

/// Every prompt, newest first.
///
/// # Errors
/// Propagates a storage failure.
pub fn list(conn: &Connection) -> Result<Vec<Prompt>> {
    let mut statement = conn.prepare(
        "SELECT id, title, doc_json, serialized_text, created_at, updated_at, sent_at
           FROM prompts ORDER BY updated_at DESC, id DESC",
    )?;
    let rows = statement.query_map([], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Read one prompt.
///
/// # Errors
/// Propagates a storage failure.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Prompt>> {
    Ok(conn
        .query_row(
            "SELECT id, title, doc_json, serialized_text, created_at, updated_at, sent_at
               FROM prompts WHERE id = ?1",
            [id],
            map_row,
        )
        .optional()?)
}

/// Read one prompt or fail.
///
/// # Errors
/// Returns [`Error::NotFound`] if there is no such prompt.
pub fn require(conn: &Connection, id: &str) -> Result<Prompt> {
    get(conn, id)?.ok_or_else(|| Error::NotFound {
        kind: "prompt",
        id: id.to_owned(),
    })
}

/// The most recently sent prompt, else the most recently touched one.
///
/// What MCP's `prompt_get {prompt_id: "latest"}` means (Inventory §2). "Sent" beats
/// "edited" because an agent asking for the latest prompt is asking for the one it was
/// handed, not the one the user happens to have open.
///
/// # Errors
/// Propagates a storage failure.
pub fn latest(conn: &Connection) -> Result<Option<Prompt>> {
    Ok(conn
        .query_row(
            "SELECT id, title, doc_json, serialized_text, created_at, updated_at, sent_at
               FROM prompts
              ORDER BY (sent_at IS NULL), sent_at DESC, updated_at DESC, id DESC
              LIMIT 1",
            [],
            map_row,
        )
        .optional()?)
}

/// Create a prompt from the gold-standard template (FR-12).
///
/// # Errors
/// Propagates a storage failure.
pub fn create(conn: &Connection, title: Option<&str>) -> Result<Prompt> {
    let id = curio_core::ids::generate();
    let now = curio_core::time::now_iso();
    let doc = serde_json::to_string(&curio_core::prompt::empty_document())?;

    conn.execute(
        "INSERT INTO prompts (id, title, doc_json, serialized_text, created_at, updated_at)
           VALUES (?1, ?2, ?3, '', ?4, ?4)",
        rusqlite::params![id, title.unwrap_or(curio_core::prompt::UNTITLED), doc, now],
    )?;
    require(conn, &id)
}

/// Save a prompt's title and/or document.
///
/// The snapshot is **not** rewritten here. It carries the serialized text, and serialization
/// needs chips resolved against the library — so the caller serializes first and calls
/// [`save_serialized`]. Autosave fires every few keystrokes; re-resolving every chip against
/// the database on each one would make typing cost a join.
///
/// # Errors
/// Returns [`Error::NotFound`] for an unknown prompt, or a storage failure.
pub fn update(
    conn: &Connection,
    id: &str,
    title: Option<&str>,
    doc_json: Option<&serde_json::Value>,
) -> Result<Prompt> {
    require(conn, id)?;
    let now = curio_core::time::now_iso();

    if let Some(title) = title {
        conn.execute(
            "UPDATE prompts SET title = ?2 WHERE id = ?1",
            rusqlite::params![id, title],
        )?;
    }
    if let Some(doc) = doc_json {
        // Taking a `Value` rather than a string is the validation: a document that is not
        // valid JSON cannot reach this function, so a working prompt can never be replaced
        // by something the editor is then unable to load.
        conn.execute(
            "UPDATE prompts SET doc_json = ?2 WHERE id = ?1",
            rusqlite::params![id, doc.to_string()],
        )?;
    }
    conn.execute(
        "UPDATE prompts SET updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, now],
    )?;
    require(conn, id)
}

/// Store freshly serialized text and rewrite the markdown snapshot (FR-14).
///
/// # Errors
/// Propagates a storage or filesystem failure.
pub fn save_serialized(
    conn: &Connection,
    root: Option<&std::path::Path>,
    id: &str,
    text: &str,
) -> Result<Prompt> {
    let prompt = require(conn, id)?;
    conn.execute(
        "UPDATE prompts SET serialized_text = ?2 WHERE id = ?1",
        rusqlite::params![id, text],
    )?;
    sidecars::write_prompt(root, id, &prompt.title, text)?;
    require(conn, id)
}

/// Stake this prompt's claim on the next project folder (FR-16).
///
/// Millisecond precision, because it is an ordering key: two prompts sent in the same
/// second must still be distinguishable when the watcher picks which one a new folder
/// belongs to (R-DA-6, Inventory §10.15).
///
/// # Errors
/// Returns [`Error::NotFound`] for an unknown prompt.
pub fn mark_sent(conn: &Connection, id: &str) -> Result<Prompt> {
    require(conn, id)?;
    conn.execute(
        "UPDATE prompts SET sent_at = ?2 WHERE id = ?1",
        rusqlite::params![id, curio_core::time::now_iso_millis()],
    )?;
    require(conn, id)
}

/// Withdraw a claim.
///
/// # Errors
/// Propagates a storage failure.
pub fn clear_sent(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("UPDATE prompts SET sent_at = NULL WHERE id = ?1", [id])?;
    Ok(())
}

/// The outstanding claim a newly-detected project should be attributed to.
///
/// Returns the most recent prompt inside the six-hour window (R-BE-21). The window is
/// checked in Rust rather than SQL because the rule belongs to the domain — `curio-core`
/// owns what "open" means, and the two must not drift.
///
/// # Errors
/// Propagates a storage failure.
pub fn open_claim(conn: &Connection) -> Result<Option<Prompt>> {
    let mut statement = conn.prepare(
        "SELECT id, title, doc_json, serialized_text, created_at, updated_at, sent_at
           FROM prompts WHERE sent_at IS NOT NULL ORDER BY sent_at DESC LIMIT 1",
    )?;
    let candidate: Option<Prompt> = statement.query_row([], map_row).optional()?;

    let Some(prompt) = candidate else {
        return Ok(None);
    };
    let Some(sent_at) = prompt.sent_at.as_deref() else {
        return Ok(None);
    };

    let age = match (
        curio_core::time::parse(sent_at),
        curio_core::time::parse(&curio_core::time::now_iso()),
    ) {
        (Ok(sent), Ok(now)) => (now - sent).whole_seconds(),
        // An unparseable timestamp means a hand-edited or foreign row. Treating it as
        // expired is the conservative answer: a wrong claim silently mis-attributes a
        // project to a prompt that did not produce it.
        _ => return Ok(None),
    };

    Ok(Prompt::claim_is_open(age).then_some(prompt))
}

/// Delete a prompt and its snapshot.
///
/// `projects.prompt_id` has **no `ON DELETE`** clause (Inventory §10.16), so the reference
/// must be nulled first or the delete fails the foreign-key check — and with
/// `foreign_keys = ON`, that failure is a user unable to delete a prompt with no
/// explanation that names the project holding it.
///
/// # Errors
/// Propagates a storage failure.
pub fn delete(conn: &mut Connection, root: Option<&std::path::Path>, id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE projects SET prompt_id = NULL WHERE prompt_id = ?1",
        [id],
    )?;
    tx.execute("DELETE FROM prompts WHERE id = ?1", [id])?;
    tx.commit()?;

    sidecars::remove_prompt(root, id);
    Ok(())
}

fn map_row(row: &Row<'_>) -> rusqlite::Result<Prompt> {
    Ok(Prompt {
        id: row.get(0)?,
        title: row.get(1)?,
        // The column is TEXT; the API publishes a value. A row that will not parse is a
        // hand-edited one — `null` keeps the prompt listable rather than making the whole
        // prompts page unopenable over one bad document.
        doc_json: serde_json::from_str(&row.get::<_, String>(2)?)
            .unwrap_or(serde_json::Value::Null),
        serialized_text: row.get(3)?,
        created_at: row.get(4)?,
        updated_at: row.get(5)?,
        sent_at: row.get(6)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    #[test]
    fn a_new_prompt_starts_from_the_template() {
        let db = Db::open_in_memory().expect("open");
        let prompt = create(db.conn(), None).expect("create");

        assert_eq!(prompt.title, curio_core::prompt::UNTITLED);
        assert_eq!(
            prompt.doc_json["content"]
                .as_array()
                .expect("content")
                .len(),
            8
        );
        assert!(prompt.sent_at.is_none());
    }

    #[test]
    fn a_document_that_is_not_json_cannot_reach_the_database() {
        // The type is the guard. `update` takes a `Value`, so malformed JSON is refused by
        // the route's deserializer and never gets as far as replacing a working prompt
        // with something the editor cannot load.
        assert!(serde_json::from_str::<serde_json::Value>("{not json").is_err());
    }

    #[test]
    fn a_document_round_trips_as_a_value_not_a_string() {
        // The bug this exists for: the column is TEXT, and publishing it as a JSON-encoded
        // string made the editor receive a string where it expected a document — which
        // TipTap renders as literal text. `GET /api/prompts/template` always returned a
        // real object, so the two shapes disagreed.
        let db = Db::open_in_memory().expect("open");
        let prompt = create(db.conn(), None).expect("create");
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "attrs": { "section": "brief" } }],
        });

        update(db.conn(), &prompt.id, None, Some(&doc)).expect("update");

        let stored = require(db.conn(), &prompt.id).expect("read").doc_json;
        assert!(stored.is_object(), "an object, never a string: {stored}");
        assert_eq!(stored, doc);
    }

    #[test]
    fn saving_serialized_text_writes_the_snapshot() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = Db::open(&dir.path().join("library.db")).expect("open");
        let prompt = create(db.conn(), Some("Pricing page")).expect("create");

        save_serialized(
            db.conn(),
            Some(dir.path()),
            &prompt.id,
            "## Brief\n\nA page.",
        )
        .expect("save");

        let snapshot = dir.path().join("prompts").join(format!("{}.md", prompt.id));
        let body = std::fs::read_to_string(snapshot).expect("read");
        assert!(body.contains("not read back"));
        assert!(body.contains("A page."));
    }

    #[test]
    fn marking_sent_records_milliseconds() {
        // Two prompts sent in the same second must stay distinguishable, or the watcher
        // cannot tell which one a new folder belongs to.
        let db = Db::open_in_memory().expect("open");
        let prompt = create(db.conn(), None).expect("create");

        let sent = mark_sent(db.conn(), &prompt.id).expect("sent");

        let stamp = sent.sent_at.expect("sent_at");
        assert_eq!(stamp.len(), 24, "{stamp}");
        assert!(stamp.contains('.'), "{stamp}");
    }

    #[test]
    fn a_fresh_claim_is_the_one_a_new_project_gets() {
        let db = Db::open_in_memory().expect("open");
        let prompt = create(db.conn(), None).expect("create");
        mark_sent(db.conn(), &prompt.id).expect("sent");

        assert_eq!(
            open_claim(db.conn()).expect("claim").map(|p| p.id),
            Some(prompt.id)
        );
    }

    #[test]
    fn an_expired_claim_is_not_offered() {
        // Six hours (R-BE-21). A prompt sent yesterday must not adopt a folder the user
        // made by hand this morning.
        let db = Db::open_in_memory().expect("open");
        let prompt = create(db.conn(), None).expect("create");
        db.conn()
            .execute(
                "UPDATE prompts SET sent_at = '2020-01-01T00:00:00.000Z' WHERE id = ?1",
                [&prompt.id],
            )
            .expect("age it");

        assert!(open_claim(db.conn()).expect("claim").is_none());
    }

    #[test]
    fn an_unsent_prompt_stakes_no_claim() {
        let db = Db::open_in_memory().expect("open");
        create(db.conn(), None).expect("create");

        assert!(open_claim(db.conn()).expect("claim").is_none());
    }

    #[test]
    fn latest_prefers_a_sent_prompt_over_a_more_recently_edited_one() {
        // An agent asking for "latest" wants the one it was handed, not whichever the user
        // happens to have open.
        let db = Db::open_in_memory().expect("open");
        let sent = create(db.conn(), Some("Sent")).expect("a");
        mark_sent(db.conn(), &sent.id).expect("sent");
        let edited = create(db.conn(), Some("Edited")).expect("b");
        update(db.conn(), &edited.id, Some("Edited again"), None).expect("update");

        assert_eq!(
            latest(db.conn()).expect("latest").map(|p| p.id),
            Some(sent.id)
        );
    }

    #[test]
    fn deleting_a_prompt_nulls_the_project_that_referenced_it() {
        // Inventory §10.16: projects.prompt_id has no ON DELETE, so with foreign_keys ON
        // this delete fails outright unless the reference is cleared first.
        let dir = tempfile::tempdir().expect("tempdir");
        let mut db = Db::open(&dir.path().join("library.db")).expect("open");
        let prompt = create(db.conn(), None).expect("create");
        db.conn()
            .execute(
                "INSERT INTO projects (id, name, path, prompt_id, detected_at)
                   VALUES ('01P', 'proj', '/tmp/proj', ?1, '2026-01-01T00:00:00Z')",
                [&prompt.id],
            )
            .expect("seed");

        delete(db.conn_mut(), Some(dir.path()), &prompt.id).expect("delete");

        let orphan: Option<String> = db
            .conn()
            .query_row(
                "SELECT prompt_id FROM projects WHERE id = '01P'",
                [],
                |row| row.get(0),
            )
            .expect("read");
        assert!(orphan.is_none(), "the project survives, its link does not");
    }

    #[test]
    fn requiring_a_missing_prompt_names_it() {
        let db = Db::open_in_memory().expect("open");
        assert!(matches!(
            require(db.conn(), "01NOPE"),
            Err(Error::NotFound { .. })
        ));
    }
}
