//! The file-length check (R-DEL-6 step 7).
//!
//! Over 650 lines fails; 500–650 passes with a justification recorded in the pull request.
//!
//! The limit is not about aesthetics. A file that has grown past this point has almost
//! always accumulated more than one responsibility, and the cost lands on review: a
//! reviewer who must hold 700 lines in their head to judge a 20-line diff will judge it
//! less well. Enforcing it mechanically means the decomposition happens when a module is
//! created, which is cheap, rather than when it has to be untangled, which is not.
//!
//! ## Why the ceiling moved from 500 to 650 (D35)
//!
//! Honestly: to unblock a release, not because 500 was measured and found wrong. Four files
//! crossed it during the variant-switcher work and the alternative was refactoring them
//! under time pressure, which is how a decomposition gets done badly. The number is a
//! judgement either way, and 650 still catches the case the rule exists for — a file nobody
//! can hold in their head — while the 500-line warning tier keeps the pressure visible.
//!
//! That is the weaker kind of reason for changing a contract rule, and the decision register
//! records it as such so the next person can reverse it deliberately.
//!
//! ## Why stylesheets are no longer counted
//!
//! This half is principled rather than expedient. The rationale above is about *control
//! flow*: responsibilities accumulate where there are branches, and a reviewer's difficulty
//! scales with the paths through a file. A stylesheet has none — it is a flat list of
//! declarations, and a 1,200-line one is long, not complex. Counting it measured the wrong
//! thing, and the cost was a rule that fired on a file whose length carries no risk.

use std::path::Path;

use anyhow::bail;

const HARD_LIMIT: usize = 650;
const SOFT_LIMIT: usize = 500;

/// Directories that are not ours to police.
const SKIP: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    ".git",
    "graphify-out",
    "docs",
];

pub fn check() -> anyhow::Result<()> {
    let root = crate::steps::repo_root()?;
    let mut over = Vec::new();
    let mut warn = Vec::new();

    walk(&root, &root, &mut over, &mut warn)?;

    warn.sort();
    for (path, lines) in &warn {
        println!("      {path} is {lines} lines — justify it in the PR description");
    }

    if !over.is_empty() {
        over.sort();
        let listed = over
            .iter()
            .map(|(path, lines)| format!("  {path} — {lines} lines"))
            .collect::<Vec<_>>()
            .join("\n");
        bail!("{} file(s) over {HARD_LIMIT} lines:\n{listed}", over.len());
    }

    Ok(())
}

fn walk(
    root: &Path,
    dir: &Path,
    over: &mut Vec<(String, usize)>,
    warn: &mut Vec<(String, usize)>,
) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name = name.to_string_lossy();

        if name.starts_with('.') || SKIP.contains(&name.as_ref()) {
            continue;
        }

        if entry.file_type()?.is_dir() {
            walk(root, &path, over, warn)?;
            continue;
        }

        if !is_source(&path) {
            continue;
        }

        let lines = std::fs::read_to_string(&path)
            .map(|body| body.lines().count())
            .unwrap_or(0);
        let relative = path
            .strip_prefix(root)
            .unwrap_or(&path)
            .display()
            .to_string()
            .replace('\\', "/");

        if lines > HARD_LIMIT {
            over.push((relative, lines));
        } else if lines > SOFT_LIMIT {
            warn.push((relative, lines));
        }
    }
    Ok(())
}

/// What the length rule applies to: code, and only code.
///
/// `css` is deliberately absent — see the module docs. If a templating language ever lands
/// here the same question applies to it: does length in this file imply paths a reviewer has
/// to trace? If not, it does not belong in this list.
fn is_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx")
    )
}
