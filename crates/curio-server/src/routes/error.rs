//! Turning a domain error into an HTTP answer.
//!
//! One mapping, in one place, so a refusal reads the same wherever it is raised. The domain
//! deliberately does not know what a status code is (`curio-core`'s error module says so):
//! it says what went wrong, and this decides how to say it in HTTP — while `curio-mcp` says
//! the same thing in JSON-RPC.
//!
//! The distinctions that matter to a user, and why each has its own code:
//!
//! * **409 for over-cap**, carrying `matched` and `limit`. The client shows both numbers as
//!   a named refusal (R-FE-11). A 400 would read as "you did it wrong" when the user did
//!   nothing wrong — their selection is simply larger than the cap.
//! * **503 + `Retry-After` for paused**, never a generic error. The dashboard renders it as
//!   a banner (R-FE-8); surfacing it as a failure would tell the user something broke when
//!   they themselves paused it.
//! * **409 for a missing API key**, not 500. Nothing failed — the work queued (FR-26).

use axum::Json;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use curio_core::Error;

/// A domain error, ready to be returned from a handler.
#[derive(Debug)]
pub struct ApiError(pub Error);

impl From<Error> for ApiError {
    fn from(error: Error) -> Self {
        Self(error)
    }
}

impl From<curio_db::Error> for ApiError {
    fn from(error: curio_db::Error) -> Self {
        Self(error.into())
    }
}

impl From<std::io::Error> for ApiError {
    fn from(error: std::io::Error) -> Self {
        Self(Error::Io(error))
    }
}

/// The convenience alias every handler returns.
pub type ApiResult<T> = std::result::Result<T, ApiError>;

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, body) = match &self.0 {
            Error::NotFound { .. } => (StatusCode::NOT_FOUND, "not_found", None),
            Error::Invalid(_) => (StatusCode::BAD_REQUEST, "invalid", None),
            Error::Conflict(_) => (StatusCode::CONFLICT, "conflict", None),

            Error::OverCap { matched, limit } => (
                StatusCode::CONFLICT,
                "over_cap",
                Some(serde_json::json!({ "matched": matched, "limit": limit })),
            ),

            Error::Paused => (StatusCode::SERVICE_UNAVAILABLE, "paused", None),
            Error::MissingApiKey => (StatusCode::CONFLICT, "missing_api_key", None),
            Error::Parked { retry_after } => (
                StatusCode::SERVICE_UNAVAILABLE,
                "parked",
                Some(serde_json::json!({ "retry_after": retry_after })),
            ),

            // Everything below is ours, not the caller's. The message still goes out —
            // this is a local app whose user is also its operator, and hiding the cause
            // behind "internal error" would leave them with nothing to report — but
            // R-SEC-15 governs what may be in that message.
            Error::Storage(_) | Error::Model(_) | Error::Io(_) | Error::Json(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal", None)
            }
        };

        if status == StatusCode::INTERNAL_SERVER_ERROR {
            tracing::error!(error = %self.0, "request failed");
        }

        let mut payload = serde_json::json!({ "error": code, "message": self.0.to_string() });
        if let Some(extra) = body
            && let (Some(payload), Some(extra)) = (payload.as_object_mut(), extra.as_object())
        {
            payload.extend(extra.clone());
        }

        // A paused client needs a number rather than a guess, and the header is the one
        // machine-readable place to put it (R-BE-3).
        if status == StatusCode::SERVICE_UNAVAILABLE {
            return (status, [(header::RETRY_AFTER, "5")], Json(payload)).into_response();
        }
        (status, Json(payload)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn status_of(error: Error) -> StatusCode {
        ApiError(error).into_response().status()
    }

    #[test]
    fn a_missing_row_is_a_404() {
        assert_eq!(
            status_of(Error::not_found("item", "01J")),
            StatusCode::NOT_FOUND
        );
    }

    #[test]
    fn a_name_collision_is_a_409_not_a_400() {
        // The fix is a different action — merge rather than rename — so it must not read
        // as "you typed something malformed".
        assert_eq!(
            status_of(Error::Conflict("taken".into())),
            StatusCode::CONFLICT
        );
        assert_eq!(
            status_of(Error::Invalid("bad".into())),
            StatusCode::BAD_REQUEST
        );
    }

    #[test]
    fn an_over_cap_refusal_carries_both_numbers() {
        // R-FE-11: the refusal is only actionable if the client can say "812 matched, the
        // limit is 500". A bare 409 would force the user to guess.
        let response = ApiError(Error::OverCap {
            matched: 812,
            limit: 500,
        })
        .into_response();

        assert_eq!(response.status(), StatusCode::CONFLICT);
        let body = body_of(response);
        assert_eq!(body["matched"], 812);
        assert_eq!(body["limit"], 500);
        assert_eq!(body["error"], "over_cap");
    }

    #[test]
    fn paused_is_a_503_with_a_retry_after() {
        let response = ApiError(Error::Paused).into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response
                .headers()
                .get(header::RETRY_AFTER)
                .map(|v| v.to_str().unwrap_or_default()),
            Some("5")
        );
    }

    #[test]
    fn a_missing_key_is_not_a_server_error() {
        // FR-26: nothing failed. The work queued, and a 500 would tell the user their
        // capture was lost.
        assert_eq!(status_of(Error::MissingApiKey), StatusCode::CONFLICT);
    }

    #[test]
    fn a_storage_failure_is_ours_and_says_so() {
        assert_eq!(
            status_of(Error::Storage("disk gone".into())),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[test]
    fn every_error_carries_a_machine_readable_code() {
        // The dashboard branches on `error`, not on the prose, so the prose stays free to
        // change without breaking a client.
        for error in [
            Error::not_found("item", "01J"),
            Error::Invalid("bad".into()),
            Error::Conflict("taken".into()),
            Error::Paused,
            Error::MissingApiKey,
        ] {
            let body = body_of(ApiError(error).into_response());
            assert!(body["error"].is_string(), "{body}");
            assert!(body["message"].is_string(), "{body}");
        }
    }

    fn body_of(response: Response) -> serde_json::Value {
        let bytes = tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("runtime")
            .block_on(axum::body::to_bytes(response.into_body(), 64 * 1024))
            .expect("body");
        serde_json::from_slice(&bytes).expect("json")
    }
}
