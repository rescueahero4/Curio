//! The event vocabulary.
//!
//! One in-process bus fans out to two push transports: SSE at `/api/events` for the
//! dashboard, and a WebSocket at `/ws` for the extension (D13). Both are served; neither
//! replaces the other, because they solve different problems — `EventSource` cannot send
//! headers, and an MV3 service worker dies after 30 s idle unless something keeps it warm.
//!
//! The names below are a **contract with existing clients** (R-BE-15, Inventory §3), not
//! an internal detail. Renaming one is a breaking change to the dashboard and the
//! extension simultaneously.
//!
//! Two subtleties that live in the consumers rather than here, recorded because they
//! explain the shapes:
//!
//! * `job.updated` may carry a full row **or** a partial `{id, result}` during bulk
//!   progress, and clients merge partials by id (Inventory §10.30). A progress tick for a
//!   240-item retag should not re-send the whole job row eight times a second.
//! * `item.created` is **ignored by the library grid whenever a filter or search is
//!   active** (Inventory §10.25). Prepending a card that does not match the active filter
//!   would be a lie about what the user is looking at.

use std::fmt;

/// The name of a pushed event, as it appears on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum EventName {
    /// A new item exists. Payload: the full item.
    ItemCreated,
    /// An item changed. Payload: the full item; clients replace in place.
    ItemUpdated,
    /// An item is gone. Payload: `{id}` only.
    ItemDeleted,
    /// The watcher registered a project it had not seen.
    ProjectDetected,
    /// A project's record changed — including going `missing`, which never deletes it.
    ProjectUpdated,
    /// A project record is gone, because the user asked for it. Payload: `{id}` only.
    ///
    /// The watcher never publishes this. A folder that vanishes goes `missing` and stays in
    /// the list; only an explicit "Remove" throws the record away.
    ProjectRemoved,
    /// A job was enqueued, started, progressed, or finished.
    ///
    /// Fires at every one of those boundaries, and may be partial (see module docs).
    JobUpdated,
    /// Some part of the vocabulary changed. Payload is deliberately empty `{}`: the
    /// consumer refetches rather than reconciling a diff, because merges and deletes can
    /// touch arbitrarily many rows.
    VocabularyUpdated,
}

impl EventName {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            EventName::ItemCreated => "item.created",
            EventName::ItemUpdated => "item.updated",
            EventName::ItemDeleted => "item.deleted",
            EventName::ProjectDetected => "project.detected",
            EventName::ProjectUpdated => "project.updated",
            EventName::ProjectRemoved => "project.removed",
            EventName::JobUpdated => "job.updated",
            EventName::VocabularyUpdated => "vocabulary.updated",
        }
    }

    /// Every event name, for exhaustiveness in tests and handler registries.
    #[must_use]
    pub const fn all() -> [EventName; 8] {
        [
            EventName::ItemCreated,
            EventName::ItemUpdated,
            EventName::ItemDeleted,
            EventName::ProjectDetected,
            EventName::ProjectUpdated,
            EventName::ProjectRemoved,
            EventName::JobUpdated,
            EventName::VocabularyUpdated,
        ]
    }
}

impl fmt::Display for EventName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// An event ready to fan out: a name and its already-serialized payload.
///
/// The payload is `serde_json::Value` rather than a typed enum because the two transports
/// serialize identically and the domain has no stake in the difference. Keeping it opaque
/// here is what lets `curio-server` own the wire format entirely.
#[derive(Debug, Clone)]
pub struct Event {
    pub name: EventName,
    pub payload: serde_json::Value,
}

impl Event {
    #[must_use]
    pub fn new(name: EventName, payload: serde_json::Value) -> Self {
        Self { name, payload }
    }

    /// `item.deleted` carries only an id — never the item, which no longer exists.
    #[must_use]
    pub fn item_deleted(id: &str) -> Self {
        Self::new(EventName::ItemDeleted, serde_json::json!({ "id": id }))
    }

    /// `project.removed` carries only an id, for the same reason `item.deleted` does.
    #[must_use]
    pub fn project_removed(id: &str) -> Self {
        Self::new(EventName::ProjectRemoved, serde_json::json!({ "id": id }))
    }

    /// `vocabulary.updated` carries an empty object by contract; consumers refetch.
    #[must_use]
    pub fn vocabulary_updated() -> Self {
        Self::new(EventName::VocabularyUpdated, serde_json::json!({}))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wire_names_are_the_ones_clients_already_listen_for() {
        // R-BE-15 / Inventory §3. These strings are a published contract: the dashboard's
        // handler registry and the extension both key off them, so a typo here is a
        // silent no-op in two codebases at once.
        let expected = [
            "item.created",
            "item.updated",
            "item.deleted",
            "project.detected",
            "project.updated",
            "project.removed",
            "job.updated",
            "vocabulary.updated",
        ];
        let actual: Vec<&str> = EventName::all().iter().map(|e| e.as_str()).collect();

        assert_eq!(actual, expected);
    }

    #[test]
    fn deleted_carries_only_an_id() {
        let event = Event::item_deleted("01J000000000000000000000AB");
        assert_eq!(event.payload["id"], "01J000000000000000000000AB");
        assert_eq!(
            event.payload.as_object().map(serde_json::Map::len),
            Some(1),
            "a deleted item has nothing else left to report"
        );
    }

    #[test]
    fn vocabulary_updated_is_an_empty_object_not_null() {
        // Consumers do `JSON.parse(data)` and then refetch; `null` would parse but reads
        // as "no payload was intended", and the contract says `{}`.
        assert_eq!(Event::vocabulary_updated().payload, serde_json::json!({}));
    }
}
