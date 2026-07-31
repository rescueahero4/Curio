//! Curation: resolving the gray zone, writing an assessment back, and editing in bulk.
//!
//! These are the three paths where the family link table carries semantics rather than just
//! membership, and each has an invariant that is easy to get backwards:
//!
//! * **Gray-zone resolution is one decision, and it is a one-way door** (FR-7,
//!   Inventory §10.13). Whichever branch the user picks, the badge clears — the question
//!   has been answered and must not be asked again by a later re-read.
//! * **A re-assessment must not overwrite a name the user chose** (Inventory §10.12). The
//!   model is better at describing than at naming something a person has already named.
//! * **A bulk add preserves gray zones.** Adding a tag to forty items says nothing about
//!   the family question hanging over three of them, so it must not silently answer it.

use rusqlite::Connection;

use curio_core::assessment::{AssessmentOutput, HUMAN_PICKED_SCORE, decide_families};
use curio_core::config::Thresholds;
use curio_core::domain::{CreatedBy, Item, LastEditedBy, VocabularyKind};

use crate::{Error, Result, vocabulary};

use super::links::{finish, set_terms};

/// How a user answered the gray-zone question (FR-7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrayZoneDecision {
    /// Keep the family the model was unsure about.
    Accept,
    /// It is a different family — this one.
    Reassign { family_id: String },
    /// Keep the family the model proposed for this item.
    AcceptProposal,
}

/// Apply a gray-zone decision, clearing the badge either way.
///
/// # Errors
/// Returns [`Error::NotFound`] for an unknown item, [`Error::Invalid`] if the decision does
/// not apply to this item, or a storage failure.
pub fn resolve_gray_zone(
    conn: &mut Connection,
    root: Option<&std::path::Path>,
    id: &str,
    decision: &GrayZoneDecision,
) -> Result<Item> {
    let tx = conn.transaction()?;
    let before = super::require(&tx, id)?;

    match decision {
        GrayZoneDecision::Accept => {
            if before.families.is_empty() {
                return Err(Error::Invalid(
                    "there is no family to accept on this item".to_owned(),
                ));
            }
        }
        GrayZoneDecision::AcceptProposal => {
            if !before.families.iter().any(|link| link.ai_proposed) {
                // The UI only offers this when a proposal exists; a request without one is
                // a stale page acting on an item someone else already resolved.
                return Err(Error::Invalid(
                    "nothing was proposed for this item".to_owned(),
                ));
            }
            tx.execute(
                "DELETE FROM item_families WHERE item_id = ?1 AND ai_proposed = 0",
                [id],
            )?;
        }
        GrayZoneDecision::Reassign { family_id } => {
            let known: i64 = tx.query_row(
                "SELECT COUNT(*) FROM aesthetic_families WHERE id = ?1",
                [family_id],
                |row| row.get(0),
            )?;
            if known == 0 {
                return Err(Error::NotFound {
                    kind: "family",
                    id: family_id.clone(),
                });
            }
            tx.execute("DELETE FROM item_families WHERE item_id = ?1", [id])?;
            tx.execute(
                "INSERT INTO item_families (item_id, family_id, score, gray_zone, ai_proposed)
                   VALUES (?1, ?2, ?3, 0, 0)",
                rusqlite::params![id, family_id, HUMAN_PICKED_SCORE],
            )?;
        }
    }

    // The one-way door. Every branch clears the badge and settles the item, because the
    // user has answered the only question that was holding it.
    tx.execute(
        "UPDATE item_families SET gray_zone = 0 WHERE item_id = ?1",
        [id],
    )?;
    tx.execute(
        "UPDATE items SET status = 'ready', last_edited_by = 'user', updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, curio_core::time::now_iso()],
    )?;

    let item = finish(&tx, root, id)?;
    tx.commit()?;
    Ok(item)
}

/// Write an assessment back (FR-5, R-BE-25).
///
/// Deterministic app code, no model in the loop: the thresholds are applied here, which is
/// what lets a Settings change re-decide an item without spending an API call.
///
/// # Errors
/// Propagates a storage failure.
pub fn apply_assessment(
    conn: &mut Connection,
    root: Option<&std::path::Path>,
    id: &str,
    output: &AssessmentOutput,
    thresholds: Thresholds,
) -> Result<Item> {
    let now = curio_core::time::now_iso();
    let tx = conn.transaction()?;
    let before = super::require(&tx, id)?;

    // Inventory §10.12. The model is better at describing than at re-naming something a
    // person has already named, and overwriting that name is the single most visible way
    // an automated pipeline can feel like it is fighting the user.
    if before.last_edited_by != LastEditedBy::User {
        tx.execute(
            "UPDATE items SET name = ?2 WHERE id = ?1",
            rusqlite::params![id, output.name_suggestion],
        )?;
    }
    tx.execute(
        "UPDATE items SET short_description = ?2, image_recipe = ?3 WHERE id = ?1",
        rusqlite::params![id, output.short_description, output.image_recipe],
    )?;

    set_terms(&tx, id, VocabularyKind::Tag, &output.tags, &now)?;
    set_terms(
        &tx,
        id,
        VocabularyKind::DesignType,
        &output.design_types,
        &now,
    )?;

    let decision = decide_families(
        &output.family_scores,
        output.new_family_proposal.as_ref(),
        thresholds,
    );

    if let Some(proposal) = &decision.create_family {
        let family_id = vocabulary::ensure(
            &tx,
            VocabularyKind::Family,
            &proposal.name,
            CreatedBy::Ai,
            &now,
        )?;
        tx.execute(
            "UPDATE aesthetic_families SET description = ?2, updated_at = ?3
              WHERE id = ?1 AND description = ''",
            rusqlite::params![family_id, proposal.description, now],
        )?;
    }

    tx.execute("DELETE FROM item_families WHERE item_id = ?1", [id])?;
    for link in &decision.links {
        // The model answers in names; only families it already knew about resolve to an
        // existing row, and a name it invented was just created above.
        let family_id = vocabulary::ensure(
            &tx,
            VocabularyKind::Family,
            &link.family,
            CreatedBy::Ai,
            &now,
        )?;
        tx.execute(
            "INSERT OR REPLACE INTO item_families (item_id, family_id, score, gray_zone, ai_proposed)
               VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![
                id,
                family_id,
                link.score,
                i64::from(link.gray_zone),
                i64::from(link.ai_proposed)
            ],
        )?;
    }

    tx.execute(
        "UPDATE items SET status = ?2, error = NULL, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, decision.status.as_str(), now],
    )?;

    let item = finish(&tx, root, id)?;
    tx.commit()?;
    Ok(item)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;
    use crate::items::write::{NewItem, create};
    use curio_core::assessment::{FamilyScore, NewFamilyProposal};
    use curio_core::domain::ItemStatus;

    fn library() -> Db {
        Db::open_in_memory().expect("open")
    }

    fn seeded(db: &mut Db) -> String {
        create(
            db.conn_mut(),
            None,
            &NewItem {
                name: "Capture".to_owned(),
                source_url: None,
                screenshot_path: "items/x/screenshot.png".to_owned(),
                thumbnail_path: None,
            },
        )
        .expect("create")
        .id
    }

    fn family(db: &Db, name: &str) -> String {
        vocabulary::create(
            db.conn(),
            VocabularyKind::Family,
            name,
            "desc",
            &curio_core::time::now_iso(),
        )
        .expect("family")
    }

    fn output(scores: Vec<FamilyScore>) -> AssessmentOutput {
        AssessmentOutput {
            name_suggestion: "Model's name".to_owned(),
            short_description: "As described".to_owned(),
            design_types: vec!["pricing page".to_owned()],
            tags: vec!["saas".to_owned()],
            family_scores: scores,
            new_family_proposal: None,
            image_recipe: None,
        }
    }

    #[test]
    fn a_confident_assessment_lands_the_item_ready() {
        let mut db = library();
        let id = seeded(&mut db);
        family(&db, "Minimal");

        let item = apply_assessment(
            db.conn_mut(),
            None,
            &id,
            &output(vec![FamilyScore {
                family: "Minimal".to_owned(),
                score: 0.9,
            }]),
            Thresholds::default(),
        )
        .expect("assess");

        assert_eq!(item.status, ItemStatus::Ready);
        assert_eq!(item.name, "Model's name");
        assert_eq!(item.tags, ["saas"]);
        assert_eq!(item.families.len(), 1);
    }

    #[test]
    fn a_gray_zone_assessment_holds_the_item_for_a_decision() {
        let mut db = library();
        let id = seeded(&mut db);
        family(&db, "Minimal");

        let item = apply_assessment(
            db.conn_mut(),
            None,
            &id,
            &output(vec![FamilyScore {
                family: "Minimal".to_owned(),
                score: 0.45,
            }]),
            Thresholds::default(),
        )
        .expect("assess");

        assert_eq!(item.status, ItemStatus::NeedsReview);
        assert!(item.families[0].gray_zone);
        assert!(item.needs_review());
    }

    #[test]
    fn a_re_assessment_keeps_a_name_the_user_chose() {
        // Inventory §10.12 — the single most visible way an automated pipeline can feel
        // like it is fighting the user.
        let mut db = library();
        let id = seeded(&mut db);
        db.conn()
            .execute(
                "UPDATE items SET name = 'My name', last_edited_by = 'user' WHERE id = ?1",
                [&id],
            )
            .expect("rename");

        let item = apply_assessment(
            db.conn_mut(),
            None,
            &id,
            &output(Vec::new()),
            Thresholds::default(),
        )
        .expect("assess");

        assert_eq!(item.name, "My name");
        assert_eq!(
            item.short_description, "As described",
            "descriptions still improve"
        );
    }

    #[test]
    fn a_proposal_creates_the_family_with_its_description() {
        let mut db = library();
        let id = seeded(&mut db);
        let mut assessed = output(Vec::new());
        assessed.new_family_proposal = Some(NewFamilyProposal {
            name: "Warm Editorial".to_owned(),
            description: "Serif headlines on paper-warm neutrals".to_owned(),
        });

        let item = apply_assessment(db.conn_mut(), None, &id, &assessed, Thresholds::default())
            .expect("assess");

        assert_eq!(item.status, ItemStatus::Ready);
        assert!(item.families[0].ai_proposed);
        let families = vocabulary::list_families(db.conn()).expect("list");
        assert_eq!(
            families[0].description,
            "Serif headlines on paper-warm neutrals"
        );
        assert_eq!(families[0].created_by, CreatedBy::Ai);
    }

    #[test]
    fn accepting_the_nearest_family_clears_the_badge() {
        let mut db = library();
        let id = seeded(&mut db);
        family(&db, "Minimal");
        apply_assessment(
            db.conn_mut(),
            None,
            &id,
            &output(vec![FamilyScore {
                family: "Minimal".to_owned(),
                score: 0.45,
            }]),
            Thresholds::default(),
        )
        .expect("assess");

        let resolved = resolve_gray_zone(db.conn_mut(), None, &id, &GrayZoneDecision::Accept)
            .expect("resolve");

        assert_eq!(resolved.status, ItemStatus::Ready);
        assert!(!resolved.needs_review(), "the door only opens once");
        assert_eq!(resolved.last_edited_by, LastEditedBy::User);
    }

    #[test]
    fn reassigning_replaces_the_families_and_scores_the_choice_at_one() {
        let mut db = library();
        let id = seeded(&mut db);
        family(&db, "Minimal");
        let editorial = family(&db, "Editorial");
        apply_assessment(
            db.conn_mut(),
            None,
            &id,
            &output(vec![FamilyScore {
                family: "Minimal".to_owned(),
                score: 0.45,
            }]),
            Thresholds::default(),
        )
        .expect("assess");

        let resolved = resolve_gray_zone(
            db.conn_mut(),
            None,
            &id,
            &GrayZoneDecision::Reassign {
                family_id: editorial,
            },
        )
        .expect("resolve");

        assert_eq!(resolved.families.len(), 1);
        assert_eq!(resolved.families[0].name, "Editorial");
        // A person is not 87 % sure, and their choice must outrank the model's wherever
        // the two are sorted together.
        assert!((resolved.families[0].score - HUMAN_PICKED_SCORE).abs() < f64::EPSILON);
    }

    #[test]
    fn reassigning_to_an_unknown_family_is_not_found() {
        let mut db = library();
        let id = seeded(&mut db);

        assert!(matches!(
            resolve_gray_zone(
                db.conn_mut(),
                None,
                &id,
                &GrayZoneDecision::Reassign {
                    family_id: "01NOPE".to_owned()
                }
            ),
            Err(Error::NotFound { .. })
        ));
    }

    #[test]
    fn accepting_a_proposal_that_does_not_exist_is_refused() {
        // A stale page acting on an item someone else already resolved.
        let mut db = library();
        let id = seeded(&mut db);

        assert!(matches!(
            resolve_gray_zone(db.conn_mut(), None, &id, &GrayZoneDecision::AcceptProposal),
            Err(Error::Invalid(_))
        ));
    }
}
