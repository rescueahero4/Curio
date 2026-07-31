//! The JSON Schemas the model is constrained to.
//!
//! ## Why these are string literals and not `json!` macros
//!
//! **Property order is load-bearing** and `serde_json`'s default map is a `BTreeMap`,
//! which sorts keys alphabetically. Building the dedupe schema with `json!` would silently
//! reorder `reason` to sit after `canonical` and `merge` — and the whole reason `reason`
//! is first is that a model generates properties in schema order, so making it reason
//! *before* it names the merge produces materially better groupings (Inventory §10.8).
//! That is a behavioural invariant a formatting detail must not be allowed to undo.
//!
//! Parsing these through [`serde_json::value::RawValue`] keeps the bytes exactly as
//! written, all the way to the wire.
//!
//! ## Constraints the API enforces
//!
//! Structured outputs reject numeric bounds (`minimum`/`maximum`) and require
//! `additionalProperties: false` on every object. The score range therefore lives in the
//! `description` text, where the model reads it, rather than in a keyword that would 400.

use serde_json::value::RawValue;

/// The assessment output schema (R-BE-25).
///
/// Mirrors [`crate::assessment::AssessmentOutput`], which re-validates the reply on
/// arrival: `output_config.format` constrains the model, it does not make the response
/// part of our type system.
pub const ASSESSMENT: &str = r#"{
  "type": "object",
  "properties": {
    "name_suggestion": {
      "type": "string",
      "description": "A short, specific name for this reference. Name what it is, not what it looks like."
    },
    "short_description": {
      "type": "string",
      "description": "One or two sentences describing the design, for someone scanning a grid."
    },
    "design_types": {
      "type": "array",
      "items": { "type": "string" },
      "description": "What kind of page or component this is, e.g. 'pricing page', 'nav bar'. Prefer existing vocabulary."
    },
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Descriptive keywords. Prefer existing vocabulary over inventing near-duplicates."
    },
    "family_scores": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "family": { "type": "string", "description": "An aesthetic family name from the supplied vocabulary, verbatim." },
          "score": { "type": "number", "description": "How well this design matches that family, from 0.0 to 1.0." }
        },
        "required": ["family", "score"],
        "additionalProperties": false
      },
      "description": "One entry per supplied family. Report the score only; do not decide which families to assign."
    },
    "new_family_proposal": {
      "type": ["object", "null"],
      "properties": {
        "name": { "type": "string" },
        "description": { "type": "string", "description": "What this family is, in the same format as the supplied family descriptions." }
      },
      "required": ["name", "description"],
      "additionalProperties": false,
      "description": "Propose a family only when no supplied family fits and the design has a coherent identity worth naming. Otherwise null."
    },
    "image_recipe": {
      "type": ["string", "null"],
      "description": "If this image would be worth reproducing, how to prompt for it. Otherwise null."
    }
  },
  "required": ["name_suggestion", "short_description", "design_types", "tags", "family_scores", "new_family_proposal", "image_recipe"],
  "additionalProperties": false
}"#;

/// The vocabulary-dedupe output schema (R-BE-24, Inventory §10.8).
///
/// `reason` is deliberately the **first** property of each group. The model generates
/// properties in the order the schema lists them, so reasoning before committing to a
/// canonical name and a merge list measurably improves the groupings — this ordering was
/// a fix, not a style choice, and reordering it is a regression with no compile error.
pub const DEDUPE: &str = r#"{
  "type": "object",
  "properties": {
    "groups": {
      "type": "array",
      "items": {
        "type": "object",
        "properties": {
          "reason": { "type": "string", "description": "Why these names mean the same thing. Think this through before choosing the canonical name." },
          "canonical": { "type": "string", "description": "The name to keep, verbatim from the supplied list." },
          "merge": {
            "type": "array",
            "items": { "type": "string" },
            "description": "The other names to fold into it, verbatim from the supplied list. Leave empty to withdraw the group."
          }
        },
        "required": ["reason", "canonical", "merge"],
        "additionalProperties": false
      }
    }
  },
  "required": ["groups"],
  "additionalProperties": false
}"#;

/// The bulk-retag output schema (R-BE-18).
///
/// Narrower than [`ASSESSMENT`] on purpose: a re-tag changes vocabulary and nothing else.
/// Letting it return a name or a family score would let a bulk operation quietly rewrite
/// items a user had already curated — and bulk edits deliberately do not stamp
/// `last_edited_by`, so there would be no trace of who did it (Inventory §10.12).
pub const RETAG: &str = r#"{
  "type": "object",
  "properties": {
    "tags": {
      "type": "array",
      "items": { "type": "string" },
      "description": "Descriptive keywords for this item. Prefer existing vocabulary over near-duplicates."
    },
    "design_types": {
      "type": "array",
      "items": { "type": "string" },
      "description": "What kind of page or component this is. Prefer existing vocabulary."
    }
  },
  "required": ["tags", "design_types"],
  "additionalProperties": false
}"#;

/// Parse a schema literal without reordering its properties.
///
/// # Panics
/// Only if a literal in this module is not valid JSON, which the tests below prevent
/// from reaching a build.
#[must_use]
pub fn raw(literal: &str) -> Box<RawValue> {
    RawValue::from_string(literal.to_owned()).expect("a schema literal in this module is malformed")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parsed(literal: &str) -> serde_json::Value {
        serde_json::from_str(literal).expect("valid JSON")
    }

    #[test]
    fn both_schemas_are_valid_json() {
        assert!(parsed(ASSESSMENT).is_object());
        assert!(parsed(DEDUPE).is_object());
    }

    #[test]
    fn reason_is_generated_before_the_merge_it_justifies() {
        // Inventory §10.8. The model emits properties in schema order, so `reason` first
        // means it thinks before it commits. A `json!` macro would have sorted this to
        // `canonical, merge, reason` with no test failing anywhere else.
        let group_properties = DEDUPE
            .find("\"reason\"")
            .zip(DEDUPE.find("\"canonical\""))
            .zip(DEDUPE.find("\"merge\""));
        let ((reason, canonical), merge) = group_properties.expect("all three properties present");

        assert!(reason < canonical, "reason must precede canonical");
        assert!(reason < merge, "reason must precede merge");
    }

    #[test]
    fn raw_parsing_preserves_the_written_order() {
        // The guarantee the whole module rests on: what we write is what goes on the wire.
        let raw = raw(DEDUPE);
        assert_eq!(raw.get(), DEDUPE);
    }

    #[test]
    fn every_object_refuses_extra_properties() {
        // Structured outputs require this on every object; a missing one is a 400 at call
        // time rather than a schema that merely permits more.
        fn check(node: &serde_json::Value) {
            if node.get("type").and_then(serde_json::Value::as_str) == Some("object") {
                assert_eq!(
                    node.get("additionalProperties"),
                    Some(&serde_json::Value::Bool(false)),
                    "an object schema is missing additionalProperties: false"
                );
            }
            match node {
                serde_json::Value::Object(map) => map.values().for_each(check),
                serde_json::Value::Array(items) => items.iter().for_each(check),
                _ => {}
            }
        }
        check(&parsed(ASSESSMENT));
        check(&parsed(DEDUPE));
    }

    #[test]
    fn no_numeric_bounds_reach_the_api() {
        // `minimum`/`maximum` are rejected by structured outputs. The 0.0–1.0 range lives
        // in the score's description, where the model actually reads it.
        assert!(!ASSESSMENT.contains("\"minimum\""));
        assert!(!ASSESSMENT.contains("\"maximum\""));
    }

    #[test]
    fn the_assessment_schema_covers_every_field_of_the_output_type() {
        // R-BE-25. A field added to `AssessmentOutput` without a schema entry would parse
        // as its default forever, silently.
        let sample = crate::assessment::AssessmentOutput {
            name_suggestion: String::new(),
            short_description: String::new(),
            design_types: Vec::new(),
            tags: Vec::new(),
            family_scores: Vec::new(),
            new_family_proposal: None,
            image_recipe: None,
        };
        let serialized = serde_json::to_value(&sample).expect("serialize");
        let schema = parsed(ASSESSMENT);
        let properties = schema["properties"].as_object().expect("properties");

        for field in serialized.as_object().expect("object").keys() {
            assert!(
                properties.contains_key(field),
                "AssessmentOutput.{field} has no schema entry"
            );
        }
    }
}
