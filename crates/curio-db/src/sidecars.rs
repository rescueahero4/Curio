//! Writing the projections: `item.md` beside each screenshot, `prompts/{id}.md` beside
//! each prompt.
//!
//! These calls happen **inside** the transaction that changed the row (R-DA-4, FR-5). A
//! failure here rolls the row back, which is the point: the alternative is a committed
//! change with a stale file next to it, and an agent reading the library with `cat` would
//! then be reading something the database no longer believes.
//!
//! The rendering itself is `curio-core`'s ([`curio_core::sidecar`],
//! [`curio_core::prompt`]) — this module only decides where the bytes go and how they
//! land.

use std::path::Path;

use curio_core::domain::Item;

use crate::Result;

/// Write an item's sidecar, creating its directory if needed.
///
/// A no-op when there is no data root (an in-memory library has nowhere to project to).
///
/// # Errors
/// Propagates a filesystem failure, which the caller must let roll the transaction back.
pub fn write_item(data_root: Option<&Path>, item: &Item) -> Result<()> {
    let Some(root) = data_root else {
        return Ok(());
    };
    let directory = curio_core::paths::item_dir(root, &item.id);
    std::fs::create_dir_all(&directory)?;
    write_atomic(
        &directory.join(curio_core::sidecar::SIDECAR_FILE_NAME),
        curio_core::sidecar::render(item).as_bytes(),
    )
}

/// Remove an item's whole directory — sidecar, screenshot, thumbnail.
///
/// Deleting the row and leaving the screenshot behind would accumulate orphaned megabytes
/// that nothing ever references again. Failure is logged rather than propagated: the row
/// is already gone, and refusing the delete because a file was locked would leave the user
/// with an item they cannot remove.
pub fn remove_item(data_root: Option<&Path>, item_id: &str) {
    let Some(root) = data_root else {
        return;
    };
    let directory = curio_core::paths::item_dir(root, item_id);
    if let Err(err) = std::fs::remove_dir_all(&directory)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%err, item = item_id, "could not remove the item directory");
    }
}

/// Write a prompt's markdown snapshot (FR-14).
///
/// # Errors
/// Propagates a filesystem failure.
pub fn write_prompt(data_root: Option<&Path>, id: &str, title: &str, text: &str) -> Result<()> {
    let Some(root) = data_root else {
        return Ok(());
    };
    let directory = root.join("prompts");
    std::fs::create_dir_all(&directory)?;
    write_atomic(
        &directory.join(format!("{id}.md")),
        curio_core::prompt::snapshot(title, text).as_bytes(),
    )
}

/// Remove a prompt's snapshot. Best-effort, for the same reason as [`remove_item`].
pub fn remove_prompt(data_root: Option<&Path>, id: &str) {
    let Some(root) = data_root else {
        return;
    };
    let path = root.join("prompts").join(format!("{id}.md"));
    if let Err(err) = std::fs::remove_file(&path)
        && err.kind() != std::io::ErrorKind::NotFound
    {
        tracing::warn!(%err, prompt = id, "could not remove the prompt snapshot");
    }
}

/// Write via a temporary file and rename.
///
/// A sidecar is read by agents that may be walking the library while Curio writes it.
/// Writing in place means a reader can observe a half-written file; a rename is atomic on
/// both target platforms, so a reader sees either the old file or the new one.
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, bytes)?;
    // Windows rejects a rename onto an existing file, unlike POSIX. Removing first opens
    // a window where neither file exists, which is strictly better than the alternative of
    // not writing at all — and the window is microseconds against a reader that retries.
    let _ = std::fs::remove_file(path);
    std::fs::rename(&temporary, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use curio_core::domain::{ItemStatus, LastEditedBy};

    fn item(id: &str) -> Item {
        Item {
            id: id.to_owned(),
            name: "Stripe pricing".to_owned(),
            short_description: "A clean pricing table.".to_owned(),
            source_url: None,
            image_recipe: None,
            screenshot_path: format!("items/{id}/screenshot.png"),
            thumbnail_path: None,
            status: ItemStatus::Ready,
            last_edited_by: LastEditedBy::Ai,
            error: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            design_types: Vec::new(),
            tags: Vec::new(),
            families: Vec::new(),
        }
    }

    #[test]
    fn writing_a_sidecar_creates_the_item_directory() {
        let root = tempfile::tempdir().expect("tempdir");
        write_item(Some(root.path()), &item("01A")).expect("write");

        let sidecar = root.path().join("items").join("01A").join("item.md");
        assert!(sidecar.exists());
        assert!(
            std::fs::read_to_string(sidecar)
                .expect("read")
                .contains("Stripe pricing")
        );
    }

    #[test]
    fn rewriting_a_sidecar_replaces_it_rather_than_appending() {
        // Every mutation regenerates the file. Appending would grow it without bound and
        // leave two contradictory frontmatter blocks in one document.
        let root = tempfile::tempdir().expect("tempdir");
        write_item(Some(root.path()), &item("01A")).expect("first");

        let mut renamed = item("01A");
        renamed.name = "Renamed".to_owned();
        write_item(Some(root.path()), &renamed).expect("second");

        let body = std::fs::read_to_string(root.path().join("items/01A/item.md")).expect("read");
        assert!(body.contains("Renamed"));
        assert!(!body.contains("Stripe pricing"));
    }

    #[test]
    fn no_temporary_file_is_left_behind() {
        let root = tempfile::tempdir().expect("tempdir");
        write_item(Some(root.path()), &item("01A")).expect("write");

        let leftovers: Vec<_> = std::fs::read_dir(root.path().join("items/01A"))
            .expect("read dir")
            .filter_map(|entry| entry.ok().map(|entry| entry.file_name()))
            .filter(|name| name.to_string_lossy().ends_with(".tmp"))
            .collect();

        assert!(leftovers.is_empty(), "{leftovers:?}");
    }

    #[test]
    fn removing_an_item_takes_its_directory_with_it() {
        // Otherwise a deleted library accumulates orphaned screenshots nothing references.
        let root = tempfile::tempdir().expect("tempdir");
        write_item(Some(root.path()), &item("01A")).expect("write");

        remove_item(Some(root.path()), "01A");

        assert!(!root.path().join("items").join("01A").exists());
    }

    #[test]
    fn removing_an_item_that_was_never_written_is_not_an_error() {
        let root = tempfile::tempdir().expect("tempdir");
        remove_item(Some(root.path()), "01NEVER");
    }

    #[test]
    fn an_in_memory_library_projects_nowhere() {
        write_item(None, &item("01A")).expect("no-op");
        remove_item(None, "01A");
    }

    #[test]
    fn a_prompt_snapshot_carries_the_do_not_edit_header() {
        let root = tempfile::tempdir().expect("tempdir");
        write_prompt(Some(root.path()), "01P", "Pricing", "## Brief\n\nA page.").expect("write");

        let body = std::fs::read_to_string(root.path().join("prompts/01P.md")).expect("read");
        assert!(body.contains("not read back"), "{body}");
        assert!(body.contains("# Pricing"));
    }

    #[test]
    fn removing_a_prompt_snapshot_is_best_effort() {
        let root = tempfile::tempdir().expect("tempdir");
        write_prompt(Some(root.path()), "01P", "Pricing", "text").expect("write");

        remove_prompt(Some(root.path()), "01P");
        remove_prompt(Some(root.path()), "01P");

        assert!(!root.path().join("prompts/01P.md").exists());
    }
}
