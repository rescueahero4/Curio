//! The domain's error type.
//!
//! Deliberately small. Transport-shaped failures (an HTTP status, a JSON-RPC code) are
//! the business of the layer that owns that transport; the domain says what went wrong,
//! and `curio-server` and `curio-mcp` each decide how to say it in their own dialect.
//! That is what keeps the same operation returning a 409 over REST and a `-32000` over
//! MCP without the domain knowing either number exists.

use std::fmt;

/// A domain result.
pub type Result<T> = std::result::Result<T, Error>;

/// What went wrong, in domain terms.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The caller asked for something that isn't there.
    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },

    /// The request was understood and refused.
    ///
    /// Carries a message meant for a human, because both transports surface it verbatim.
    #[error("{0}")]
    Invalid(String),

    /// A limit was exceeded, and the operation refused rather than trimming.
    ///
    /// Never silently truncate: a bulk request over the 500-item cap returns this so the
    /// caller learns both numbers and can decide, which is the difference between a
    /// refusal and a wrong answer (R-BE-18, R-FE-11).
    #[error("{matched} items matched but the limit is {limit}")]
    OverCap { matched: usize, limit: usize },

    /// A mutation was attempted while the app is paused.
    ///
    /// Paused means paused for writes only — reads, search, SSE, and MCP read tools keep
    /// working (D25). Transports render this as `503 + Retry-After` or as a JSON-RPC
    /// error with `reason: "paused"`.
    #[error("Curio is paused; mutations are refused until it resumes")]
    Paused,

    /// No Anthropic API key is configured.
    ///
    /// Not a failure of the work — the work queues. A job that hits this requeues
    /// *without consuming an attempt*, so a user who is offline or hasn't added a key yet
    /// accumulates a backlog that drains cleanly rather than a pile of failures
    /// (FR-26, R-BE-17).
    #[error("no API key is configured")]
    MissingApiKey,

    /// The work cannot proceed yet and should be retried after `retry_after`.
    ///
    /// Parking releases the worker for other jobs instead of blocking it — a bulk run
    /// waiting on the Batch API can wait for hours, and every assessment behind it would
    /// otherwise wait too. Parking refunds the attempt (R-BE-17).
    #[error("parked until {retry_after}")]
    Parked { retry_after: String },

    /// Storage failed. The message comes from `curio-db`, flattened to a string so this
    /// crate never types a `rusqlite` error (R-DEL-2).
    #[error("storage: {0}")]
    Storage(String),

    /// An upstream model call failed.
    #[error("model call: {0}")]
    Model(String),

    /// Filesystem work failed.
    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    /// Serialization failed.
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

impl Error {
    /// A refusal with a message a human should read.
    pub fn invalid(message: impl fmt::Display) -> Self {
        Error::Invalid(message.to_string())
    }

    /// Something that should exist doesn't.
    pub fn not_found(kind: &'static str, id: impl fmt::Display) -> Self {
        Error::NotFound {
            kind,
            id: id.to_string(),
        }
    }

    /// Whether retrying the identical request could plausibly succeed later.
    ///
    /// The jobs worker's retry policy branches on this, so it lives with the error rather
    /// than being re-derived at each call site.
    #[must_use]
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Error::Paused | Error::MissingApiKey | Error::Parked { .. } | Error::Model(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn over_cap_names_both_numbers() {
        // R-BE-18 and R-FE-11: the refusal is only actionable if the caller learns what
        // matched and what the limit is. A bare "too many" would force a guess.
        let rendered = Error::OverCap {
            matched: 812,
            limit: 500,
        }
        .to_string();

        assert!(rendered.contains("812"), "{rendered}");
        assert!(rendered.contains("500"), "{rendered}");
    }

    #[test]
    fn queueing_conditions_are_transient() {
        // FR-26: no key and paused mean "later", not "no". The worker must not burn
        // attempts on them.
        assert!(Error::MissingApiKey.is_transient());
        assert!(Error::Paused.is_transient());
        assert!(!Error::Invalid("bad shape".into()).is_transient());
        assert!(
            !Error::not_found("item", "01J").is_transient(),
            "a missing row will still be missing on retry"
        );
    }
}
