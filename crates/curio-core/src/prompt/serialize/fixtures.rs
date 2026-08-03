//! Document builders shared by the tests in this module's four files.
//!
//! One library rather than one per file: every test here needs the same small vocabulary,
//! and four copies of `context()` drifting apart would be worse than the indirection.

use super::{ChipContext, FamilySample};

/// The document half, with the write-back footer removed.
///
/// The body rules and the footer are independent. A body test that asserted the whole
/// output would fail every time the footer's wording was improved, which would make the
/// footer expensive to touch for no gain in what those tests actually check.
pub(super) fn body(output: &str) -> String {
    match output.find("## Where this lands") {
        Some(at) => output[..at].trim_end().to_owned(),
        None => output.trim_end().to_owned(),
    }
}

pub(super) fn sample(id: &str, name: &str) -> FamilySample {
    FamilySample {
        id: id.to_owned(),
        name: name.to_owned(),
        directory: format!("C:\\Users\\me\\Curio\\items\\{id}"),
    }
}

pub(super) fn context() -> ChipContext {
    let mut context = ChipContext::default();
    context.families.insert(
        "fam1".to_owned(),
        (
            "Warm Editorial".to_owned(),
            "Serif headlines on paper-warm neutrals".to_owned(),
        ),
    );
    // A family the library holds nothing eligible for — every item in it is gray-zone,
    // or it has none at all.
    context.families.insert(
        "fam2".to_owned(),
        ("Cold Brutalist".to_owned(), "Concrete and grids".to_owned()),
    );
    context.family_samples.insert(
        "fam1".to_owned(),
        vec![
            sample("01JAAA", "Stripe pricing"),
            sample("01JBBB", "Monzo blog"),
        ],
    );
    context.items.insert(
        "item1".to_owned(),
        (
            "Stripe pricing".to_owned(),
            "C:\\Users\\me\\Curio\\items\\01J".to_owned(),
        ),
    );
    context
        .terms
        .insert("tag1".to_owned(), "brutalist".to_owned());
    context.projects_root = "C:\\Users\\me\\Curio\\projects".to_owned();
    context
}

/// A chip node, for the tests that build documents by hand.
pub(super) fn chip(kind: &str, id: &str, label: &str) -> serde_json::Value {
    serde_json::json!({ "type": kind, "attrs": { "id": id, "label": label } })
}

pub(super) fn paragraph(section: &str, content: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "type": "paragraph", "attrs": { "section": section }, "content": content })
}

pub(super) fn text(value: &str) -> serde_json::Value {
    serde_json::json!({ "type": "text", "text": value })
}

pub(super) fn doc(content: Vec<serde_json::Value>) -> serde_json::Value {
    serde_json::json!({ "type": "doc", "content": content })
}

pub(super) fn heading(level: u64, value: &str) -> serde_json::Value {
    serde_json::json!({
        "type": "heading",
        "attrs": { "level": level },
        "content": [text(value)],
    })
}
