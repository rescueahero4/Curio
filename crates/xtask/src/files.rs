//! The file-length check (R-DEL-6 step 7).
//!
//! Over 500 lines fails; 400–500 passes with a justification recorded in the pull request.
//!
//! The limit is not about aesthetics. A file that has grown past this point has almost
//! always accumulated more than one responsibility, and the cost lands on review: a
//! reviewer who must hold 700 lines in their head to judge a 20-line diff will judge it
//! less well. Enforcing it mechanically means the decomposition happens when a module is
//! created, which is cheap, rather than when it has to be untangled, which is not.

use std::path::Path;

use anyhow::bail;

const HARD_LIMIT: usize = 500;
const SOFT_LIMIT: usize = 400;

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

fn is_source(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("rs" | "ts" | "tsx" | "js" | "jsx" | "css")
    )
}
