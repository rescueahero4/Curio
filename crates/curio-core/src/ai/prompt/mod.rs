//! Building the three calls Curio makes (R-BE-23, R-BE-24).
//!
//! ## The two cache breakpoints
//!
//! The vision system prompt is split into **exactly two** cached blocks — the rubric, then
//! the vocabulary — and that number is measured, not aesthetic: a single breakpoint
//! covering both recorded zero cache reads in the previous implementation (Inventory §9,
//! §10.7). The two blocks change on different schedules (the rubric almost never, the
//! vocabulary whenever a family is added), so one breakpoint per block means adding a tag
//! costs a re-cache of the vocabulary rather than of the whole prompt.
//!
//! Everything volatile — the image, the URL, the title, the thresholds — lives in the user
//! turn, **after** the last breakpoint. A timestamp or a per-item id above a breakpoint
//! would invalidate the prefix on every single call and make the whole arrangement
//! decorative.
//!
//! ## Why the model is told the thresholds but forbidden to apply them
//!
//! It needs them to calibrate how it spends a 0.0–1.0 range. It must not act on them,
//! because the decision is [`crate::assessment::decide_families`] — ordinary code, no
//! model in the loop. That split is what lets a user move a slider in Settings and
//! re-decide their whole library without spending an API call (FR-6).

mod utility;

pub use utility::{ItemSummary, dedupe, retag};

use super::wire::{Content, Message, MessagesRequest, OutputConfig, OutputFormat, SystemBlock};
use crate::config::Thresholds;

/// R-BE-23. Enough for a full structured assessment with room for adaptive thinking.
pub const VISION_MAX_TOKENS: u32 = 8000;

/// R-BE-24. Utility work is a list in, a list out.
pub const UTILITY_MAX_TOKENS: u32 = 2000;

/// R-BE-23. Reported rather than reasoned about at length — the hard thinking is the
/// rubric's job, and the decision is ours.
pub const VISION_EFFORT: &str = "medium";

/// The vocabulary the model scores against, rendered for the prompt.
///
/// Ordering must be **stable across calls** or the cached block is rewritten every time.
/// The caller supplies it already sorted; [`Vocabulary::render`] does not re-sort, so the
/// database's `ORDER BY name` is the single source of that ordering.
#[derive(Debug, Clone, Default)]
pub struct Vocabulary {
    /// `(name, description)`. The description is what makes a family scoreable rather than
    /// a word to guess at.
    pub families: Vec<(String, String)>,
    pub design_types: Vec<String>,
    pub tags: Vec<String>,
}

impl Vocabulary {
    /// The cached vocabulary block.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = String::from("# The library's existing vocabulary\n");

        out.push_str("\n## Aesthetic families\n");
        if self.families.is_empty() {
            out.push_str(
                "There are none yet. Score nothing, and propose a family if this design has \
                 a coherent identity worth naming.\n",
            );
        } else {
            out.push_str(
                "Score every one of these. Use the names verbatim — a near-miss spelling \
                 creates a duplicate family rather than matching this one.\n\n",
            );
            for (name, description) in &self.families {
                if description.trim().is_empty() {
                    out.push_str(&format!("- {name}\n"));
                } else {
                    out.push_str(&format!("- {name}: {description}\n"));
                }
            }
        }

        out.push_str(&list("Design types", &self.design_types));
        out.push_str(&list("Tags", &self.tags));

        out.push_str(
            "\nPrefer an existing term over a new one that means the same thing. Every \
             near-duplicate you invent is a filter the user has to scroll past forever.\n",
        );
        out
    }
}

fn list(heading: &str, values: &[String]) -> String {
    if values.is_empty() {
        return format!("\n## {heading}\nThere are none yet.\n");
    }
    format!("\n## {heading}\n{}\n", values.join(", "))
}

/// What the capture knew about itself.
#[derive(Debug, Clone, Default)]
pub struct Context {
    pub source_url: Option<String>,
    pub title: Option<String>,
}

/// A screenshot, already encoded and already downscaled (R-BE-26).
#[derive(Debug, Clone)]
pub struct Image {
    pub media_type: String,
    pub base64: String,
}

/// One structured-output vision call (R-BE-23, FR-4).
#[must_use]
pub fn assessment(
    model: &str,
    rubric: &str,
    vocabulary: &Vocabulary,
    image: &Image,
    context: &Context,
    thresholds: Thresholds,
) -> MessagesRequest {
    MessagesRequest {
        model: model.to_owned(),
        max_tokens: VISION_MAX_TOKENS,
        // Breakpoint one, then breakpoint two. Exactly two (R-BE-23).
        system: vec![
            SystemBlock::cached(rubric.trim()),
            SystemBlock::cached(vocabulary.render()),
        ],
        messages: vec![Message::user(vec![
            Content::image(&image.media_type, &image.base64),
            Content::text(user_turn(context, thresholds)),
        ])],
        output_config: Some(OutputConfig {
            effort: Some(VISION_EFFORT),
            format: Some(OutputFormat {
                kind: "json_schema",
                schema: super::schema::raw(super::schema::ASSESSMENT),
            }),
        }),
    }
}

fn user_turn(context: &Context, thresholds: Thresholds) -> String {
    let mut out = String::from("Assess this screenshot.\n");

    match (&context.title, &context.source_url) {
        (Some(title), Some(url)) => out.push_str(&format!("\nTitle: {title}\nSource: {url}\n")),
        (Some(title), None) => out.push_str(&format!("\nTitle: {title}\n")),
        (None, Some(url)) => out.push_str(&format!("\nSource: {url}\n")),
        (None, None) => {}
    }

    // The thresholds are context for calibration, and the prohibition is the load-bearing
    // half of the sentence. A model that helpfully "assigns" families would put the
    // decision in a place a Settings change cannot revisit (FR-6).
    out.push_str(&format!(
        "\nThe library assigns a family at a score of {upper} or above, and holds anything \
         from {lower} up to {upper} for a human to confirm.\n\n\
         Do not apply these thresholds yourself. Do not decide which families to assign, \
         and do not omit a family because you think it scores too low. Report a score for \
         every family you were given and let the library apply its own policy — that is \
         what lets the user move these numbers later without re-running you.\n",
        lower = thresholds.lower,
        upper = thresholds.upper,
    ));

    out
}

/// The smallest call that proves a key works (Inventory §9 `verifyApiKey`).
///
/// Sixteen tokens: enough for a reply, too few to cost anything worth measuring. It is a
/// credential check, not a capability check — the point is to tell a user their key is
/// wrong while they are still in Settings rather than three captures later.
#[must_use]
pub fn verify_key(model: &str) -> MessagesRequest {
    MessagesRequest {
        model: model.to_owned(),
        max_tokens: 16,
        system: Vec::new(),
        messages: vec![Message::user(vec![Content::text("Reply with OK.")])],
        output_config: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image() -> Image {
        Image {
            media_type: "image/png".to_owned(),
            base64: "AAAA".to_owned(),
        }
    }

    fn vocabulary() -> Vocabulary {
        Vocabulary {
            families: vec![("Brutalist".to_owned(), "Heavy type, raw edges".to_owned())],
            design_types: vec!["pricing page".to_owned()],
            tags: vec!["saas".to_owned()],
        }
    }

    fn request() -> serde_json::Value {
        let built = assessment(
            "claude-sonnet-5",
            "# Rubric\nLook carefully.",
            &vocabulary(),
            &image(),
            &Context {
                source_url: Some("https://stripe.com/pricing".to_owned()),
                title: Some("Pricing".to_owned()),
            },
            Thresholds::default(),
        );
        serde_json::to_value(&built).expect("serialize")
    }

    #[test]
    fn the_vision_call_carries_exactly_two_cache_breakpoints() {
        // R-BE-23, and the number is measured: one breakpoint over the combined prompt
        // recorded zero cache reads (Inventory §10.7). Three would waste a breakpoint on
        // content that never changes independently.
        let json = request();
        let breakpoints = json["system"]
            .as_array()
            .expect("system blocks")
            .iter()
            .filter(|block| block.get("cache_control").is_some())
            .count();

        assert_eq!(breakpoints, 2);
    }

    #[test]
    fn the_rubric_is_cached_before_the_vocabulary() {
        // Order is the whole point: the rubric is the stabler of the two, so adding a tag
        // must not invalidate it.
        let json = request();
        let system = json["system"].as_array().expect("system blocks");

        assert!(system[0]["text"].as_str().expect("text").contains("Rubric"));
        assert!(
            system[1]["text"]
                .as_str()
                .expect("text")
                .contains("Brutalist")
        );
    }

    #[test]
    fn nothing_volatile_sits_above_a_breakpoint() {
        // A per-item value in a cached block invalidates the prefix on every call and
        // makes the whole arrangement decorative.
        let json = request();
        let cached = serde_json::to_string(&json["system"]).expect("serialize");

        assert!(!cached.contains("stripe.com"), "the URL leaked into cache");
        assert!(!cached.contains("Pricing"), "the title leaked into cache");
        assert!(!cached.contains("0.4"), "a threshold leaked into cache");
    }

    #[test]
    fn the_user_turn_forbids_applying_the_thresholds() {
        // FR-6. Without this the model assigns families itself, and a Settings change can
        // no longer re-decide an item without a fresh API call.
        let json = request();
        let text = json["messages"][0]["content"][1]["text"]
            .as_str()
            .expect("text")
            .to_lowercase();

        assert!(text.contains("do not apply these thresholds yourself"));
        assert!(text.contains("0.4") && text.contains("0.5"));
    }

    #[test]
    fn the_image_precedes_the_instructions() {
        // Vision quality is measurably better when the image arrives before the question
        // about it.
        let json = request();
        assert_eq!(json["messages"][0]["content"][0]["type"], "image");
        assert_eq!(json["messages"][0]["content"][1]["type"], "text");
    }

    #[test]
    fn the_vision_call_uses_the_documented_budget_and_effort() {
        let json = request();
        assert_eq!(json["max_tokens"], VISION_MAX_TOKENS);
        assert_eq!(json["output_config"]["effort"], "medium");
        assert_eq!(json["output_config"]["format"]["type"], "json_schema");
    }

    #[test]
    fn an_empty_library_still_produces_a_usable_vocabulary_block() {
        // The first capture ever: there is nothing to score against, and the prompt has to
        // say so rather than present an empty heading the model reads as an error.
        let rendered = Vocabulary::default().render();

        assert!(rendered.contains("There are none yet"));
        assert!(rendered.contains("propose a family"));
    }

    #[test]
    fn a_capture_with_no_context_omits_the_headings_entirely() {
        // An empty "Source:" line reads as a source that is empty rather than absent.
        let built = assessment(
            "m",
            "r",
            &vocabulary(),
            &image(),
            &Context::default(),
            Thresholds::default(),
        );
        let json = serde_json::to_value(&built).expect("serialize");
        let text = json["messages"][0]["content"][1]["text"]
            .as_str()
            .expect("text");

        assert!(!text.contains("Source:"));
        assert!(!text.contains("Title:"));
    }

    #[test]
    fn verifying_a_key_is_cheap_enough_to_do_on_every_save() {
        let json = serde_json::to_value(verify_key("m")).expect("serialize");

        assert_eq!(json["max_tokens"], 16);
        assert!(
            json.get("system").is_none(),
            "no prompt to cache or pay for"
        );
        assert!(json.get("output_config").is_none());
    }
}
