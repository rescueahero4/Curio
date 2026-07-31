//! The Messages API request body, as types rather than `json!` literals.
//!
//! Typed for one specific reason: the output schema travels as a
//! [`RawValue`](serde_json::value::RawValue) so its property order survives to the wire
//! (see [`super::schema`]). A `serde_json::Value` cannot hold one without flattening it
//! through a sorted map, so the body has to be a `Serialize` struct all the way down.
//!
//! Only the fields Curio actually sends are modelled. This is not an SDK.

use serde::Serialize;
use serde_json::value::RawValue;

/// `cache_control: {"type": "ephemeral"}` — one cache breakpoint.
#[derive(Debug, Clone, Serialize)]
pub struct CacheControl {
    #[serde(rename = "type")]
    pub kind: &'static str,
}

impl CacheControl {
    #[must_use]
    pub fn ephemeral() -> Self {
        Self { kind: "ephemeral" }
    }
}

/// One block of the system prompt.
///
/// A list rather than a string because the breakpoints are placed **between** blocks, and
/// R-BE-23 requires exactly two of them.
#[derive(Debug, Clone, Serialize)]
pub struct SystemBlock {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_control: Option<CacheControl>,
}

impl SystemBlock {
    /// A block that ends a cacheable prefix.
    #[must_use]
    pub fn cached(text: impl Into<String>) -> Self {
        Self {
            kind: "text",
            text: text.into(),
            cache_control: Some(CacheControl::ephemeral()),
        }
    }
}

/// A base64 image source. The only image form Curio sends — the screenshot is local, so
/// there is no URL to hand over.
#[derive(Debug, Clone, Serialize)]
pub struct ImageSource {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub media_type: String,
    pub data: String,
}

/// One content block of a turn.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Content {
    Text { text: String },
    Image { source: ImageSource },
}

impl Content {
    #[must_use]
    pub fn text(text: impl Into<String>) -> Self {
        Content::Text { text: text.into() }
    }

    /// A base64 image. `media_type` must be one the API accepts — `image/png`,
    /// `image/jpeg`, or `image/webp`.
    #[must_use]
    pub fn image(media_type: impl Into<String>, data: impl Into<String>) -> Self {
        Content::Image {
            source: ImageSource {
                kind: "base64",
                media_type: media_type.into(),
                data: data.into(),
            },
        }
    }
}

/// One turn.
#[derive(Debug, Clone, Serialize)]
pub struct Message {
    pub role: &'static str,
    pub content: Vec<Content>,
}

impl Message {
    #[must_use]
    pub fn user(content: Vec<Content>) -> Self {
        Self {
            role: "user",
            content,
        }
    }
}

/// `output_config.format` — the structured-output constraint.
#[derive(Debug, Serialize)]
pub struct OutputFormat {
    #[serde(rename = "type")]
    pub kind: &'static str,
    /// Carried raw so property order survives (Inventory §10.8).
    pub schema: Box<RawValue>,
}

/// `output_config` — where both `effort` and `format` live.
///
/// `effort` is `None` for utility calls and that absence is deliberate, not an oversight:
/// the cheap utility model **rejects** the parameter outright (R-BE-24, Inventory §10.7).
#[derive(Debug, Serialize)]
pub struct OutputConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effort: Option<&'static str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<OutputFormat>,
}

/// A `POST /v1/messages` body.
#[derive(Debug, Serialize)]
pub struct MessagesRequest {
    pub model: String,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub system: Vec<SystemBlock>,
    pub messages: Vec<Message>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_config: Option<OutputConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_schema_reaches_the_wire_with_its_order_intact() {
        // The reason this module exists. Round-tripping through `serde_json::Value` here
        // would sort `reason` after `canonical` and quietly undo Inventory §10.8.
        let request = MessagesRequest {
            model: "m".to_owned(),
            max_tokens: 10,
            system: Vec::new(),
            messages: Vec::new(),
            output_config: Some(OutputConfig {
                effort: None,
                format: Some(OutputFormat {
                    kind: "json_schema",
                    schema: super::super::schema::raw(r#"{"zebra":1,"apple":2}"#),
                }),
            }),
        };

        let body = serde_json::to_string(&request).expect("serialize");
        assert!(
            body.contains(r#"{"zebra":1,"apple":2}"#),
            "the schema was re-serialized and reordered: {body}"
        );
    }

    #[test]
    fn an_absent_effort_is_omitted_rather_than_null() {
        // R-BE-24: the utility model rejects the parameter. `"effort": null` is still the
        // parameter as far as the API is concerned.
        let config = OutputConfig {
            effort: None,
            format: None,
        };
        assert_eq!(serde_json::to_string(&config).expect("serialize"), "{}");
    }

    #[test]
    fn an_image_block_carries_a_base64_source() {
        let json = serde_json::to_value(Content::image("image/png", "AAAA")).expect("serialize");

        assert_eq!(json["type"], "image");
        assert_eq!(json["source"]["type"], "base64");
        assert_eq!(json["source"]["media_type"], "image/png");
    }
}
