//! The dependency-direction check (R-DEL-2, R-DEL-6 step 8).
//!
//! Three seams have to stay cuttable, and each rule below protects one of them:
//!
//! * **`curio-core` sees no SQL, no HTTP, no MCP.** The storage seam is a trait the domain
//!   defines. Unwinding an accidental `rusqlite` import from the domain six months in is
//!   the expensive version of this check; running it on every gate is the cheap one.
//! * **`curio-db` is the only crate with SQLite.** One writer, one place SQL can be, one
//!   place to look when storage behaves oddly.
//! * **`curio-nmh` stays tiny.** Chrome spawns it once per connection, so its startup time
//!   is user-visible latency in the extension popup's status dot. Anything that pulls an
//!   async runtime into a process that lives for one message is a regression a reviewer
//!   would have to notice by eye.
//!
//! The assertions run against the real resolved graph, transitively — a crate that reaches
//! rusqlite through an intermediary is caught the same as a direct dependency.

use std::collections::{BTreeSet, HashMap};

use anyhow::{Context as _, bail};
use cargo_metadata::{DependencyKind, Metadata, MetadataCommand, Package, PackageId};

/// How far a rule looks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Reach {
    /// The crate must not reach the dependency at all, however indirectly.
    ///
    /// The right strength for a layering rule: `curio-core` reaching rusqlite through some
    /// intermediary is exactly as bad as importing it, because the seam is gone either way.
    Transitive,
    /// The crate must not name the dependency itself.
    ///
    /// The right strength where reaching it *through the proper layer* is the architecture.
    /// `curio-server` reaches SQLite through `curio-db` by design — that is the design.
    /// What it must not do is take its own dependency and start writing SQL.
    Direct,
}

/// A crate, and what it must not depend on.
struct Rule {
    crate_name: &'static str,
    reach: Reach,
    forbidden: &'static [&'static str],
    why: &'static str,
}

const RULES: &[Rule] = &[
    Rule {
        crate_name: "curio-core",
        reach: Reach::Transitive,
        forbidden: &[
            "rusqlite",
            "libsqlite3-sys",
            "axum",
            "hyper",
            "rmcp",
            "tokio",
        ],
        why: "the domain defines traits; the outer layers implement them (R-DEL-2)",
    },
    Rule {
        crate_name: "curio-mcp",
        reach: Reach::Transitive,
        forbidden: &["rusqlite", "libsqlite3-sys"],
        why: "MCP tools call curio-core services, never SQL (R-MCP-13)",
    },
    Rule {
        crate_name: "curio-server",
        reach: Reach::Direct,
        forbidden: &["rusqlite", "libsqlite3-sys"],
        why: "curio-db is the only crate that may name SQLite (R-DEL-2)",
    },
    Rule {
        crate_name: "curio-tray",
        reach: Reach::Direct,
        forbidden: &["rusqlite", "libsqlite3-sys", "axum"],
        why: "the shell drives the service; it does not reimplement it (R-DEL-2)",
    },
    Rule {
        crate_name: "curio-nmh",
        reach: Reach::Transitive,
        forbidden: &[
            "tokio",
            "axum",
            "hyper",
            "reqwest",
            "rusqlite",
            "libsqlite3-sys",
        ],
        why: "Chrome spawns it per connection; its startup time is visible latency (R-EXT-3)",
    },
    Rule {
        crate_name: "curio-runtime",
        reach: Reach::Transitive,
        forbidden: &["tokio", "axum", "hyper", "rusqlite"],
        why: "curio-nmh depends on it, so its weight is curio-nmh's weight (D27)",
    },
];

/// Crates that may depend on `xtask`. None may.
const DEV_ONLY: &[&str] = &["xtask"];

pub fn check() -> anyhow::Result<()> {
    let metadata = MetadataCommand::new()
        .exec()
        .context("could not read cargo metadata")?;
    let packages: HashMap<&PackageId, &Package> =
        metadata.packages.iter().map(|pkg| (&pkg.id, pkg)).collect();

    let mut violations = Vec::new();

    for rule in RULES {
        let Some(root) = find_package(&metadata, rule.crate_name) else {
            // The crate does not exist yet. Not a failure — the gate must pass on a
            // partially built workspace — but say so, because a silently skipped
            // assertion is worse than a missing one.
            println!("      note: {} not present, rule skipped", rule.crate_name);
            continue;
        };

        let (found, verb) = match rule.reach {
            Reach::Transitive => (reachable_from(&metadata, &packages, &root.id), "reaches"),
            Reach::Direct => (
                root.dependencies
                    .iter()
                    .filter(|dep| dep.kind == DependencyKind::Normal)
                    .map(|dep| dep.name.clone())
                    .collect(),
                "directly depends on",
            ),
        };

        for forbidden in rule.forbidden {
            if found.contains(*forbidden) {
                violations.push(format!(
                    "  {} {verb} {} — {}",
                    rule.crate_name, forbidden, rule.why
                ));
            }
        }
    }

    for package in &metadata.packages {
        if !package.id.repr.contains("path+") {
            continue;
        }
        for dev_only in DEV_ONLY {
            if package.name.as_str() == *dev_only {
                continue;
            }
            if package
                .dependencies
                .iter()
                .any(|dep| dep.name == *dev_only && dep.kind == DependencyKind::Normal)
            {
                violations.push(format!(
                    "  {} depends on {dev_only}, which is dev-only tooling (R-DEL-2)",
                    package.name
                ));
            }
        }
    }

    if !violations.is_empty() {
        bail!("dependency direction violated:\n{}", violations.join("\n"));
    }

    println!("      {} rules hold", RULES.len());
    Ok(())
}

fn find_package<'a>(metadata: &'a Metadata, name: &str) -> Option<&'a Package> {
    metadata
        .packages
        .iter()
        .find(|package| package.name.as_str() == name && package.id.repr.contains("path+"))
}

/// Every crate reachable from `root` through normal (non-dev, non-build) dependencies.
///
/// Dev-dependencies are excluded deliberately: a test may use `tempfile` or an HTTP client
/// without that saying anything about what ships. Build-dependencies likewise do not end
/// up in the binary.
fn reachable_from<'a>(
    metadata: &'a Metadata,
    packages: &HashMap<&'a PackageId, &'a Package>,
    root: &'a PackageId,
) -> BTreeSet<String> {
    let Some(resolve) = metadata.resolve.as_ref() else {
        return BTreeSet::new();
    };
    let nodes: HashMap<&PackageId, _> = resolve.nodes.iter().map(|node| (&node.id, node)).collect();

    let mut seen = BTreeSet::new();
    let mut queue = vec![root];
    let mut visited = BTreeSet::new();

    while let Some(id) = queue.pop() {
        if !visited.insert(id.repr.clone()) {
            continue;
        }
        let Some(node) = nodes.get(id) else { continue };

        for dep in &node.deps {
            let ships = dep
                .dep_kinds
                .iter()
                .any(|kind| kind.kind == DependencyKind::Normal);
            if !ships {
                continue;
            }
            if let Some(package) = packages.get(&dep.pkg) {
                seen.insert(package.name.to_string());
            }
            queue.push(&dep.pkg);
        }
    }

    seen
}
