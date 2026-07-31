//! The Anthropic transport.
//!
//! Sockets only. What to *ask* lives in [`curio_core::ai`], which builds every request
//! body without touching the network — this module posts what that module produced and
//! turns HTTP outcomes into domain errors (R-DEL-2).
//!
//! ## The error mapping is the interesting part
//!
//! The worker's retry policy branches on the error's *kind*, so which variant a status
//! code becomes decides whether a user's retry budget gets spent (R-BE-17):
//!
//! | Upstream | Domain error | Effect on the job |
//! |---|---|---|
//! | 429, `overloaded_error` | [`Parked`](curio_core::Error::Parked) | waits, **attempt refunded** |
//! | 5xx, network failure | [`Model`](curio_core::Error::Model) | retries, attempt spent |
//! | 401 / 403 | [`Invalid`](curio_core::Error::Invalid) | fails after its retries, visibly |
//! | 400, refusal, truncation | [`Invalid`](curio_core::Error::Invalid) | same input will not improve |
//!
//! A rate limit is the one worth arguing about: it is charged nothing because being told
//! "slow down" is not the job going wrong, and a user importing a folder of screenshots
//! would otherwise watch their queue turn into failures at exactly the moment it was
//! working hardest.

use std::time::Duration;

use curio_core::Error;
use curio_core::ai::MessagesRequest;
use serde::Deserialize;

/// The API Curio talks to. Overridable so tests can point at a local stub.
const DEFAULT_BASE_URL: &str = "https://api.anthropic.com";

/// The env var that redirects the client. Test and proxy affordance only.
pub const BASE_URL_ENV: &str = "CURIO_ANTHROPIC_BASE_URL";

/// The API version header every request carries.
const API_VERSION: &str = "2023-06-01";

/// How many times a *transport* failure is retried inside one call (Inventory §9,
/// "maxRetries 2").
///
/// Distinct from the job-level retry budget: this covers a connection that dropped
/// mid-request, where re-sending immediately usually works and bothering the queue would
/// be theatre. Job-level retries cover the call having genuinely failed.
const TRANSPORT_RETRIES: u32 = 2;

/// A single call can involve a large image and a thinking model.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(180);

/// A configured client. Cheap to clone; the connection pool is shared.
#[derive(Clone)]
pub struct Anthropic {
    http: reqwest::Client,
    api_key: String,
    base_url: String,
}

impl Anthropic {
    /// Build a client for `api_key`.
    ///
    /// # Errors
    /// Returns [`Error::Model`] if the HTTP stack cannot be constructed — in practice a
    /// missing system TLS store.
    pub fn new(api_key: impl Into<String>) -> curio_core::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|err| Error::Model(format!("could not start the HTTP client: {err}")))?;

        Ok(Self {
            http,
            api_key: api_key.into(),
            base_url: std::env::var(BASE_URL_ENV)
                .ok()
                .filter(|value| !value.trim().is_empty())
                .unwrap_or_else(|| DEFAULT_BASE_URL.to_owned()),
        })
    }

    /// One `POST /v1/messages`, returning the reply's first text block.
    ///
    /// # Errors
    /// See the table in the module documentation.
    pub async fn messages(&self, request: &MessagesRequest) -> curio_core::Result<String> {
        let body = serde_json::to_string(request)?;
        let response: MessagesResponse = self.post("/v1/messages", body).await?;

        // A refusal and a truncation are both "this exact request will not do better",
        // which is why neither is a `Model` error the worker would retry three times.
        match response.stop_reason.as_deref() {
            Some("refusal") => {
                return Err(Error::invalid(
                    "the model declined to assess this image — it may show something its \
                     safety policy covers",
                ));
            }
            Some("max_tokens") => {
                return Err(Error::invalid(
                    "the model's reply was cut off before it finished",
                ));
            }
            _ => {}
        }

        response
            .content
            .into_iter()
            .find_map(|block| match block {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Other => None,
            })
            .ok_or_else(|| Error::Model("the reply carried no text".to_owned()))
    }

    /// Whether this key works, using the cheapest call that proves it (Inventory §9).
    ///
    /// # Errors
    /// Propagates the mapped error so Settings can show *why* — "rejected" and
    /// "unreachable" call for different actions from the user.
    pub async fn verify_key(&self, model: &str) -> curio_core::Result<()> {
        self.messages(&curio_core::ai::prompt::verify_key(model))
            .await
            .map(|_| ())
    }

    /// Submit a batch (R-BE-18). `custom_id` is the item id, all the way back.
    ///
    /// # Errors
    /// See the table in the module documentation.
    pub async fn create_batch(
        &self,
        requests: &[(String, MessagesRequest)],
    ) -> curio_core::Result<String> {
        #[derive(serde::Serialize)]
        struct Entry<'a> {
            custom_id: &'a str,
            params: &'a MessagesRequest,
        }
        #[derive(serde::Serialize)]
        struct Body<'a> {
            requests: Vec<Entry<'a>>,
        }

        let body = serde_json::to_string(&Body {
            requests: requests
                .iter()
                .map(|(custom_id, params)| Entry { custom_id, params })
                .collect(),
        })?;

        let created: BatchStatus = self.post("/v1/messages/batches", body).await?;
        Ok(created.id)
    }

    /// Where a batch has got to.
    ///
    /// # Errors
    /// See the table in the module documentation.
    pub async fn batch_status(&self, batch_id: &str) -> curio_core::Result<BatchStatus> {
        self.get(&format!("/v1/messages/batches/{batch_id}")).await
    }

    /// Every result, keyed by the `custom_id` that went in.
    ///
    /// Results arrive in **any order**, so this returns a map rather than a list: keying by
    /// position would silently attach one item's assessment to another's row.
    ///
    /// # Errors
    /// See the table in the module documentation.
    pub async fn batch_results(
        &self,
        batch_id: &str,
    ) -> curio_core::Result<std::collections::HashMap<String, Result<String, String>>> {
        let url = format!("{}/v1/messages/batches/{batch_id}/results", self.base_url);
        let response = self
            .send(|| self.http.get(&url))
            .await?
            .text()
            .await
            .map_err(|err| Error::Model(format!("could not read the batch results: {err}")))?;

        let mut out = std::collections::HashMap::new();
        // JSONL: one result per line. A line that will not parse is skipped rather than
        // failing the batch — one malformed row must not discard 499 good ones.
        for line in response.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(entry) = serde_json::from_str::<BatchResultLine>(line) else {
                tracing::warn!("a batch result line could not be parsed; skipping it");
                continue;
            };
            out.insert(entry.custom_id, entry.result.into_outcome());
        }
        Ok(out)
    }

    /// Ask the API to stop a batch (R-BE-19).
    ///
    /// Cancelling the job locally without this leaves the batch running and billing.
    ///
    /// # Errors
    /// See the table in the module documentation.
    pub async fn cancel_batch(&self, batch_id: &str) -> curio_core::Result<()> {
        let _: serde_json::Value = self
            .post(
                &format!("/v1/messages/batches/{batch_id}/cancel"),
                String::new(),
            )
            .await?;
        Ok(())
    }

    async fn post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: String,
    ) -> curio_core::Result<T> {
        let url = format!("{}{path}", self.base_url);
        let response = self
            .send(|| {
                self.http
                    .post(&url)
                    .header("content-type", "application/json")
                    .body(body.clone())
            })
            .await?;
        parse(response).await
    }

    async fn get<T: serde::de::DeserializeOwned>(&self, path: &str) -> curio_core::Result<T> {
        let url = format!("{}{path}", self.base_url);
        let response = self.send(|| self.http.get(&url)).await?;
        parse(response).await
    }

    /// Send with auth headers, retrying transport failures and honouring status mapping.
    async fn send(
        &self,
        build: impl Fn() -> reqwest::RequestBuilder,
    ) -> curio_core::Result<reqwest::Response> {
        let mut last: Option<Error> = None;

        for attempt in 0..=TRANSPORT_RETRIES {
            if attempt > 0 {
                // A short, fixed pause. The long backoff belongs to the job, not here.
                tokio::time::sleep(Duration::from_millis(500 * u64::from(attempt))).await;
            }

            let sent = build()
                .header("x-api-key", &self.api_key)
                .header("anthropic-version", API_VERSION)
                .send()
                .await;

            match sent {
                Ok(response) if response.status().is_success() => return Ok(response),
                Ok(response) => {
                    let error = status_error(response).await;
                    // A refusal is a refusal however many times it is asked; only server
                    // trouble is worth a second try inside one call.
                    if !matches!(error, Error::Model(_)) {
                        return Err(error);
                    }
                    last = Some(error);
                }
                Err(err) => {
                    last = Some(Error::Model(format!("could not reach the API: {err}")));
                }
            }
        }

        Err(last.unwrap_or_else(|| Error::Model("the API could not be reached".to_owned())))
    }
}

async fn parse<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> curio_core::Result<T> {
    let body = response
        .text()
        .await
        .map_err(|err| Error::Model(format!("could not read the reply: {err}")))?;

    serde_json::from_str(&body).map_err(|err| {
        // The body is deliberately not logged: a request echo can contain the prompt, and
        // the prompt can contain a user's page titles and URLs (R-SEC-15).
        Error::Model(format!("the reply was not in the expected shape: {err}"))
    })
}

/// Turn a non-2xx response into the domain error whose retry behaviour fits it.
async fn status_error(response: reqwest::Response) -> Error {
    let status = response.status();
    let retry_after = response
        .headers()
        .get("retry-after")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<i64>().ok());

    // Only the API's own message is surfaced, never the whole body.
    let message = response
        .text()
        .await
        .ok()
        .and_then(|body| serde_json::from_str::<ApiErrorBody>(&body).ok())
        .map_or_else(
            || format!("HTTP {}", status.as_u16()),
            |parsed| parsed.error.message,
        );

    match status.as_u16() {
        // Being told to slow down is not the job going wrong.
        429 | 529 => Error::Parked {
            retry_after: curio_core::time::seconds_from_now(retry_after.unwrap_or(30)),
        },
        401 | 403 => Error::invalid(format!("the API key was rejected: {message}")),
        code if code >= 500 => Error::Model(format!("the API is having trouble: {message}")),
        _ => Error::invalid(message),
    }
}

// --- reply shapes ----------------------------------------------------------------------

#[derive(Debug, Deserialize)]
struct MessagesResponse {
    #[serde(default)]
    content: Vec<ContentBlock>,
    #[serde(default)]
    stop_reason: Option<String>,
}

/// Only text blocks are read. Thinking blocks and anything the API adds later fall into
/// `Other` rather than failing the parse — a new block type must not break assessment.
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(other)]
    Other,
}

/// A batch's lifecycle, as the API reports it.
#[derive(Debug, Clone, Deserialize)]
pub struct BatchStatus {
    pub id: String,
    /// `in_progress`, `canceling`, or `ended`.
    #[serde(default)]
    pub processing_status: String,
}

impl BatchStatus {
    /// Whether every request in the batch has settled.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.processing_status == "ended"
    }
}

#[derive(Debug, Deserialize)]
struct BatchResultLine {
    custom_id: String,
    result: BatchResult,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum BatchResult {
    Succeeded { message: MessagesResponse },
    Errored,
    Canceled,
    Expired,
}

impl BatchResult {
    /// Flatten to "the text" or "why not", so the caller handles one shape per item.
    fn into_outcome(self) -> Result<String, String> {
        match self {
            BatchResult::Succeeded { message } => message
                .content
                .into_iter()
                .find_map(|block| match block {
                    ContentBlock::Text { text } => Some(text),
                    ContentBlock::Other => None,
                })
                .ok_or_else(|| "the reply carried no text".to_owned()),
            BatchResult::Errored => Err("the model call failed".to_owned()),
            BatchResult::Canceled => Err("the batch was cancelled".to_owned()),
            BatchResult::Expired => Err("the batch expired before it ran".to_owned()),
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiErrorBody {
    error: ApiErrorDetail,
}

#[derive(Debug, Deserialize)]
struct ApiErrorDetail {
    #[serde(default)]
    message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rate_limit_parks_rather_than_failing() {
        // R-BE-17. Importing a folder of screenshots is exactly when rate limits appear,
        // and it must not be the moment the queue turns into failures.
        let error = Error::Parked {
            retry_after: curio_core::time::seconds_from_now(30),
        };
        assert!(matches!(
            curio_core::ai::recover(2, &error),
            curio_core::ai::Recovery::Park { .. }
        ));
    }

    #[test]
    fn a_thinking_block_does_not_break_the_parse() {
        // Adaptive thinking is on by default on current models; the reply carries blocks
        // this client does not read, and an unknown one must not fail an assessment.
        let response: MessagesResponse = serde_json::from_str(
            r#"{"content":[{"type":"thinking","thinking":"…"},{"type":"text","text":"answer"}],
                "stop_reason":"end_turn"}"#,
        )
        .expect("parse");

        assert_eq!(response.content.len(), 2);
        assert!(matches!(
            response.content.into_iter().find_map(|b| match b {
                ContentBlock::Text { text } => Some(text),
                ContentBlock::Other => None,
            }),
            Some(text) if text == "answer"
        ));
    }

    #[test]
    fn batch_results_are_keyed_by_custom_id_not_position() {
        // Results come back in any order. Keying by position attaches one item's
        // assessment to another item's row — silently, and permanently.
        let line: BatchResultLine = serde_json::from_str(
            r#"{"custom_id":"01J","result":{"type":"succeeded",
                "message":{"content":[{"type":"text","text":"{}"}]}}}"#,
        )
        .expect("parse");

        assert_eq!(line.custom_id, "01J");
        assert_eq!(line.result.into_outcome().expect("text"), "{}");
    }

    #[test]
    fn every_batch_failure_shape_becomes_a_readable_reason() {
        for (raw, expected) in [
            (r#"{"type":"errored"}"#, "failed"),
            (r#"{"type":"canceled"}"#, "cancelled"),
            (r#"{"type":"expired"}"#, "expired"),
        ] {
            let result: BatchResult = serde_json::from_str(raw).expect("parse");
            let reason = result.into_outcome().expect_err("a failure");
            assert!(reason.contains(expected), "{reason}");
        }
    }

    #[test]
    fn a_batch_is_only_finished_when_the_api_says_ended() {
        let running = BatchStatus {
            id: "b".to_owned(),
            processing_status: "in_progress".to_owned(),
        };
        assert!(!running.is_finished());

        let ended = BatchStatus {
            id: "b".to_owned(),
            processing_status: "ended".to_owned(),
        };
        assert!(ended.is_finished());
    }

    #[test]
    fn the_base_url_is_overridable_for_tests() {
        // Without this every test of the worker would need the internet and a real key.
        assert_eq!(BASE_URL_ENV, "CURIO_ANTHROPIC_BASE_URL");
    }
}
