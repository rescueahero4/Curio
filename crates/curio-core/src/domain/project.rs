//! Project origin and presence.

use serde::{Deserialize, Serialize};

/// How Curio came to know about a project.
///
/// The distinction is not bookkeeping. Only a project the **watcher** adopted gets a
/// marker file minted into it, and therefore only that project is rename-followable. A
/// project registered over MCP deliberately gets no fingerprint (R-MCP-1,
/// Inventory §10.17): identity is minted on adoption, never on registration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectOrigin {
    /// The folder appeared under the watched root and Curio adopted it.
    Watcher,
    /// An agent called `project_register`.
    Mcp,
    /// The user added it from the dashboard.
    Manual,
}

impl ProjectOrigin {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectOrigin::Watcher => "watcher",
            ProjectOrigin::Mcp => "mcp",
            ProjectOrigin::Manual => "manual",
        }
    }

    #[must_use]
    pub const fn all() -> [ProjectOrigin; 3] {
        [
            ProjectOrigin::Watcher,
            ProjectOrigin::Mcp,
            ProjectOrigin::Manual,
        ]
    }
}

/// Whether the folder is still on disk.
///
/// A project whose directory disappears is marked `Missing`, **never deleted** (FR-19).
/// The record carries the link back to the prompt that produced it, and there is nothing
/// on disk that could reconstruct that link — so deleting the row to match the filesystem
/// would destroy the one thing the filesystem never held.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProjectStatus {
    Present,
    Missing,
}

impl ProjectStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ProjectStatus::Present => "present",
            ProjectStatus::Missing => "missing",
        }
    }
}

/// The prefix of a fingerprint minted from a `.curio-project` marker file.
///
/// The format is versioned in the value itself because the previous one had to be retired.
/// Fingerprints used to be `ino:<inode>`, and **ext4 re-issues a deleted directory's inode
/// to the next directory created** — so a deleted project and an unrelated new folder
/// shared an identity, and the deleted project's record was re-pointed at the stranger,
/// carrying its `prompt_id` with it. Migration v4 clears every non-`mark:` fingerprint for
/// exactly that reason.
pub const MARKER_FINGERPRINT_PREFIX: &str = "mark:";

/// The marker file Curio mints into a folder it adopts.
pub const MARKER_FILE_NAME: &str = ".curio-project";

/// What a `.curio-project` marker holds.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectMarker {
    /// A ULID minted at adoption. Copying the folder does **not** copy an identity: the
    /// copy is a different project and gets a fresh id the first time it is adopted.
    pub id: String,
    /// Which tool wrote the folder, when that is knowable. Informational.
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

impl ProjectMarker {
    /// The fingerprint this marker produces.
    #[must_use]
    pub fn fingerprint(&self) -> String {
        format!("{MARKER_FINGERPRINT_PREFIX}{}", self.id)
    }
}

/// A catalogued project folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub name: String,
    /// Absolute, unlike an item's screenshot path: a project lives wherever the user's AI
    /// tool put it, which is not necessarily under any Curio-owned root.
    pub path: String,
    pub origin: ProjectOrigin,
    /// The prompt that claimed this folder, if one did.
    pub prompt_id: Option<String>,
    pub status: ProjectStatus,
    pub fingerprint: Option<String>,
    pub detected_at: String,
    pub last_opened_at: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_strings_match_the_check_constraint() {
        // projects.origin CHECK (origin IN ('watcher','mcp','manual'))
        let actual: Vec<&str> = ProjectOrigin::all().iter().map(|o| o.as_str()).collect();
        assert_eq!(actual, ["watcher", "mcp", "manual"]);
    }

    #[test]
    fn status_strings_match_the_check_constraint() {
        // projects.status CHECK (status IN ('present','missing'))
        assert_eq!(ProjectStatus::Present.as_str(), "present");
        assert_eq!(ProjectStatus::Missing.as_str(), "missing");
    }

    #[test]
    fn the_fingerprint_prefix_is_the_one_migration_v4_preserves() {
        // v4 runs `UPDATE projects SET fingerprint = NULL WHERE fingerprint NOT LIKE
        // 'mark:%'`. If this constant drifted, v4 would clear the identities it exists to
        // keep — on every upgraded library, once, irreversibly.
        assert_eq!(MARKER_FINGERPRINT_PREFIX, "mark:");
    }
}
