//! What the vision model returns, and what Curio decides with it.
//!
//! The split is the point. The model is shown the thresholds and told **not to apply
//! them** (R-BE-23): it reports a score per family and nothing else. Curio applies the
//! policy in [`decide_families`], in ordinary code, with no model in the loop (FR-5,
//! FR-6).
//!
//! That is why changing a threshold in Settings can re-decide an existing item without
//! spending a single API call — the scores are still on disk, and the decision is a pure
//! function of them.

use serde::{Deserialize, Serialize};

use crate::config::Thresholds;
use crate::domain::ItemStatus;

/// One family, scored.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FamilyScore {
    /// The family's **name**, not its id: the model is shown the vocabulary as text and
    /// answers in the same terms. Resolving a name to a row is the write-back's job.
    pub family: String,
    pub score: f64,
}

/// A family the model thinks should exist but doesn't yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NewFamilyProposal {
    pub name: String,
    /// The gold-standard description format the rubric asks for. It becomes the family's
    /// description verbatim when the proposal is accepted, and it is what a later prompt
    /// serializes into an `Aesthetic:` line — so a thin one degrades the product two steps
    /// downstream.
    pub description: String,
}

/// The structured output of one assessment call (R-BE-25).
///
/// Re-validated on arrival rather than trusted: `structured_output` constrains the model,
/// it does not make the response part of our type system.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AssessmentOutput {
    pub name_suggestion: String,
    pub short_description: String,
    #[serde(default)]
    pub design_types: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub family_scores: Vec<FamilyScore>,
    #[serde(default)]
    pub new_family_proposal: Option<NewFamilyProposal>,
    #[serde(default)]
    pub image_recipe: Option<String>,
}

/// One family link the decision wants written.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyLink {
    /// The family's name. The write-back resolves or creates the row.
    pub family: String,
    pub score: f64,
    /// Held for a human decision: the score fell between the thresholds (FR-7).
    pub gray_zone: bool,
    /// This family did not exist before this assessment proposed it.
    pub ai_proposed: bool,
}

/// What to write after an assessment.
#[derive(Debug, Clone, PartialEq)]
pub struct FamilyDecision {
    pub links: Vec<FamilyLink>,
    pub status: ItemStatus,
    /// Present only when the decision accepted a proposal and the family must be created
    /// with `created_by = 'ai'`.
    pub create_family: Option<NewFamilyProposal>,
}

/// Apply the two thresholds to a set of scores (FR-6, Inventory §9 `decideFamilies`).
///
/// Four outcomes, in the order they are checked:
///
/// 1. **Confident.** The best score is at or above the upper threshold: link every family
///    that clears it, and the item is `ready`. Multiple families are normal — a page can
///    genuinely be both brutalist and editorial.
/// 2. **Gray zone.** The best score sits between the thresholds: link **only the nearest
///    one**, flag it, and hold the item at `needs_review`. Linking the runners-up too
///    would ask the user to adjudicate a set when the question is about one match (FR-7).
/// 3. **Nothing fits, but the model proposed something.** Create the proposed family and
///    link it at 1.0 as `ai_proposed`. The item is `ready`: the model was confident, it
///    simply had no existing word for what it saw.
/// 4. **Nothing fits and nothing was proposed.** No links, `needs_review`. An item with no
///    family and no badge would sit in the library invisible to every family filter.
#[must_use]
pub fn decide_families(
    scores: &[FamilyScore],
    proposal: Option<&NewFamilyProposal>,
    thresholds: Thresholds,
) -> FamilyDecision {
    let best = scores
        .iter()
        .max_by(|a, b| a.score.total_cmp(&b.score))
        .filter(|best| best.score.is_finite());

    match best {
        Some(best) if best.score >= thresholds.upper => FamilyDecision {
            links: scores
                .iter()
                .filter(|candidate| candidate.score >= thresholds.upper)
                .map(|candidate| FamilyLink {
                    family: candidate.family.clone(),
                    score: candidate.score,
                    gray_zone: false,
                    ai_proposed: false,
                })
                .collect(),
            status: ItemStatus::Ready,
            create_family: None,
        },

        Some(best) if best.score >= thresholds.lower => FamilyDecision {
            links: vec![FamilyLink {
                family: best.family.clone(),
                score: best.score,
                gray_zone: true,
                ai_proposed: false,
            }],
            status: ItemStatus::NeedsReview,
            create_family: None,
        },

        _ => match proposal {
            Some(proposal) => FamilyDecision {
                links: vec![FamilyLink {
                    family: proposal.name.clone(),
                    // A proposal is not a match against an existing family, so there is no
                    // score to carry. 1.0 records "this item is the reason the family
                    // exists", which is exactly what it is.
                    score: 1.0,
                    gray_zone: false,
                    ai_proposed: true,
                }],
                status: ItemStatus::Ready,
                create_family: Some(proposal.clone()),
            },
            None => FamilyDecision {
                links: Vec::new(),
                status: ItemStatus::NeedsReview,
                create_family: None,
            },
        },
    }
}

/// The score a human decision carries (Inventory §10.13).
///
/// Not a confidence value — a person is not 87 % sure. It is the highest value in the
/// range so that a user's choice always outranks a model's, wherever the two are sorted
/// together.
pub const HUMAN_PICKED_SCORE: f64 = 1.0;

/// Trim and cap a vocabulary name the model produced (Inventory §9 `cleanVocabularyNames`).
///
/// Returns `None` for anything that cannot become a row. A model occasionally answers a
/// request for a tag with a sentence, and a sentence in the tag filter is permanent
/// clutter that every future filter list has to carry.
#[must_use]
pub fn clean_vocabulary_name(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().count() > crate::domain::MAX_NAME_LEN {
        return None;
    }
    Some(trimmed.to_owned())
}

/// Clean a list of names, dropping rejects and case-insensitive duplicates.
///
/// Case-insensitive because `tags.name` is `UNIQUE COLLATE NOCASE`: letting "Brutalist"
/// and "brutalist" both through would produce a constraint violation on write rather than
/// two tags.
#[must_use]
pub fn clean_vocabulary_names(raw: &[String]) -> Vec<String> {
    let mut seen = std::collections::HashSet::new();
    raw.iter()
        .filter_map(|name| clean_vocabulary_name(name))
        .filter(|name| seen.insert(name.to_lowercase()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thresholds() -> Thresholds {
        Thresholds::default() // 0.4 / 0.5
    }

    fn score(family: &str, score: f64) -> FamilyScore {
        FamilyScore {
            family: family.to_owned(),
            score,
        }
    }

    fn proposal() -> NewFamilyProposal {
        NewFamilyProposal {
            name: "Warm Editorial".to_owned(),
            description: "Serif headlines on paper-warm neutrals…".to_owned(),
        }
    }

    #[test]
    fn a_confident_match_assigns_every_family_above_the_upper_threshold() {
        // A page can genuinely be two things at once; the confident branch does not pick
        // a winner among families the model was equally sure about.
        let decision = decide_families(
            &[
                score("Brutalist", 0.9),
                score("Editorial", 0.72),
                score("Playful", 0.2),
            ],
            None,
            thresholds(),
        );

        assert_eq!(decision.status, ItemStatus::Ready);
        assert_eq!(decision.links.len(), 2);
        assert!(decision.links.iter().all(|link| !link.gray_zone));
    }

    #[test]
    fn a_gray_zone_match_links_only_the_nearest_family() {
        // FR-7. The user is being asked one question — "is this a Brutalist?" — so
        // linking the runners-up would turn a decision into an audit.
        let decision = decide_families(
            &[score("Brutalist", 0.45), score("Editorial", 0.44)],
            None,
            thresholds(),
        );

        assert_eq!(decision.status, ItemStatus::NeedsReview);
        assert_eq!(decision.links.len(), 1);
        assert_eq!(decision.links[0].family, "Brutalist");
        assert!(decision.links[0].gray_zone);
    }

    #[test]
    fn a_proposal_below_the_lower_threshold_creates_the_family() {
        let decision = decide_families(&[score("Brutalist", 0.1)], Some(&proposal()), thresholds());

        assert_eq!(decision.status, ItemStatus::Ready);
        assert_eq!(
            decision.create_family.as_ref().map(|f| f.name.as_str()),
            Some("Warm Editorial")
        );
        assert!(decision.links[0].ai_proposed);
        assert_eq!(decision.links[0].score, HUMAN_PICKED_SCORE);
    }

    #[test]
    fn nothing_fitting_and_nothing_proposed_still_needs_review() {
        // Otherwise the item lands `ready` with no family and is invisible to every
        // family filter — present in the library and unfindable through it.
        let decision = decide_families(&[score("Brutalist", 0.1)], None, thresholds());

        assert_eq!(decision.status, ItemStatus::NeedsReview);
        assert!(decision.links.is_empty());
    }

    #[test]
    fn an_empty_score_list_falls_through_to_the_proposal_branch() {
        // A first capture into an empty library: there is no vocabulary to score against.
        let decision = decide_families(&[], Some(&proposal()), thresholds());

        assert_eq!(decision.status, ItemStatus::Ready);
        assert!(decision.links[0].ai_proposed);
    }

    #[test]
    fn a_score_exactly_on_the_upper_threshold_is_confident() {
        // The boundary is inclusive on the confident side: "assign >= upper".
        let decision = decide_families(&[score("Brutalist", 0.5)], None, thresholds());
        assert_eq!(decision.status, ItemStatus::Ready);
    }

    #[test]
    fn a_score_exactly_on_the_lower_threshold_is_gray() {
        let decision = decide_families(&[score("Brutalist", 0.4)], None, thresholds());
        assert_eq!(decision.status, ItemStatus::NeedsReview);
        assert!(decision.links[0].gray_zone);
    }

    #[test]
    fn a_zero_width_gray_zone_never_produces_a_gray_link() {
        // Equal thresholds are a legitimate choice: assign or reject, never ask.
        let decision = decide_families(
            &[score("Brutalist", 0.49)],
            None,
            Thresholds {
                lower: 0.5,
                upper: 0.5,
            },
        );
        assert!(decision.links.is_empty());
    }

    #[test]
    fn a_non_finite_score_does_not_win() {
        // JSON has no NaN, but a model can emit one through a permissive parser, and
        // `total_cmp` orders NaN above everything — so it would silently become "best".
        let decision = decide_families(&[score("Broken", f64::NAN)], None, thresholds());
        assert_eq!(decision.status, ItemStatus::NeedsReview);
        assert!(decision.links.is_empty());
    }

    #[test]
    fn names_are_trimmed_and_length_capped() {
        assert_eq!(
            clean_vocabulary_name("  minimal  ").as_deref(),
            Some("minimal")
        );
        assert_eq!(clean_vocabulary_name("   "), None);
        assert_eq!(clean_vocabulary_name(&"x".repeat(61)), None);
        assert!(clean_vocabulary_name(&"x".repeat(60)).is_some());
    }

    #[test]
    fn cleaning_a_list_drops_case_insensitive_duplicates() {
        // `tags.name` is UNIQUE COLLATE NOCASE — letting both through is a constraint
        // violation at write time, not two tags.
        let cleaned = clean_vocabulary_names(&[
            "Brutalist".to_owned(),
            "brutalist".to_owned(),
            "  ".to_owned(),
            "Editorial".to_owned(),
        ]);

        assert_eq!(cleaned, vec!["Brutalist", "Editorial"]);
    }

    #[test]
    fn the_assessment_schema_round_trips() {
        let output = AssessmentOutput {
            name_suggestion: "Stripe pricing".to_owned(),
            short_description: "A clean pricing table".to_owned(),
            design_types: vec!["pricing page".to_owned()],
            tags: vec!["saas".to_owned()],
            family_scores: vec![score("Minimal", 0.8)],
            new_family_proposal: None,
            image_recipe: None,
        };

        let json = serde_json::to_string(&output).expect("serialize");
        assert_eq!(
            serde_json::from_str::<AssessmentOutput>(&json).expect("parse"),
            output
        );
    }

    #[test]
    fn a_sparse_model_reply_still_parses() {
        // Absent arrays are the common shape when a model has nothing to say, and a hard
        // parse failure there would fail the whole assessment over an empty tag list.
        let parsed: AssessmentOutput =
            serde_json::from_str(r#"{"name_suggestion":"x","short_description":"y"}"#)
                .expect("parse");

        assert!(parsed.tags.is_empty());
        assert!(parsed.new_family_proposal.is_none());
    }
}
