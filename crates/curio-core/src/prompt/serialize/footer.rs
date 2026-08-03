//! Telling the agent where the result should end up.
//!
//! Separate from the document walk because it is not part of the document: no chip produces
//! it, no section contains it, and it survives a prompt the user emptied completely.

use super::ChipContext;

/// The write-back instruction every serialized prompt ends with.
///
/// Two halves, in this order because they are not equally cheap. Building under the watched
/// root costs the agent nothing further — the watcher adopts the folder within a few
/// seconds, mints `.curio-project`, and spends the prompt's claim. Registering is the
/// fallback for what is actually the normal case: a person working in their own directory.
///
/// **Once** is stated rather than implied. A folder Curio already knows is never re-adopted
/// and cannot spend a second claim, so a repeated call is harmless but pointless, and an
/// agent iterating on one project across several turns should not make it every turn.
///
/// It steers toward `project_register`'s `variants` argument rather than the agent
/// hand-writing `curio-variants.json`. The argument validates every entry against what is on
/// disk and writes the shape Curio actually reads; a hand-written file parses either way, so
/// a wrong one fails silently into an unlabelled switcher.
pub(super) fn write_back_footer(context: &ChipContext) -> String {
    let mut footer = String::from("## Where this lands\n\n");

    if !context.projects_root.trim().is_empty() {
        footer.push_str(&format!(
            "Curio is watching `{}`. A project built directly inside it is picked up within a \
             few seconds, and nothing further is needed.\n\n",
            context.projects_root.trim()
        ));
    }

    footer.push_str(
        "If the project lands anywhere else, call the `project_register` MCP tool **once**, \
         after the folder is finished, with its absolute path. Pass its `variants` argument if \
         you produced several versions, so Curio writes `curio-variants.json` itself instead of \
         you hand-writing it. If you have no Curio MCP connection, say so in your reply and it \
         can be registered by hand.",
    );

    footer
}

#[cfg(test)]
mod tests {
    use crate::prompt::serialize::fixtures::*;
    use crate::prompt::serialize::serialize;

    #[test]
    fn the_footer_names_the_root_curio_is_watching() {
        // Half the instruction is "build here and do nothing else", which is only usable if
        // the prompt says where here is.
        let output = serialize(
            &doc(vec![paragraph("brief", serde_json::json!([text("Hi.")]))]),
            &context(),
        );

        assert!(
            output.contains("C:\\Users\\me\\Curio\\projects"),
            "{output}"
        );
        assert!(output.contains("project_register"), "{output}");
    }

    #[test]
    fn the_register_instruction_says_to_call_it_once() {
        // A folder Curio already knows cannot spend a second claim, so a repeated call is
        // harmless but pointless — and an agent iterating across turns will otherwise make it
        // every turn.
        let output = serialize(&doc(vec![]), &context());
        assert!(output.contains("**once**"), "{output}");
    }

    #[test]
    fn the_footer_prefers_the_variants_argument_over_hand_written_json() {
        // The live failure this came from: an agent hand-wrote curio-variants.json in a shape
        // Curio parses but cannot read — trailing slash on the folder, wrong key names, no
        // version. `project_register` writes it correctly by construction.
        let output = serialize(&doc(vec![]), &context());

        assert!(output.contains("variants"), "{output}");
        assert!(output.contains("curio-variants.json"), "{output}");
    }

    #[test]
    fn without_a_configured_root_only_the_register_half_survives() {
        // Emitting a path that is not there would be worse than saying nothing: an agent
        // would build into a directory nothing is watching and believe it was done.
        let mut context = context();
        context.projects_root = String::new();

        let output = serialize(&doc(vec![]), &context);

        assert!(!output.contains("is watching"), "{output}");
        assert!(output.contains("project_register"), "{output}");
    }

    #[test]
    fn the_footer_never_makes_mcp_a_prerequisite() {
        // FR-14 is the whole product bet: an agent that has never heard of Curio must be able
        // to follow the prompt. The footer is an instruction to a connected agent and has to
        // give an unconnected one somewhere to go.
        let output = serialize(&doc(vec![]), &context());

        assert!(
            output.contains("no Curio MCP connection"),
            "the footer must tell an unconnected agent what to do: {output}"
        );
    }
}
