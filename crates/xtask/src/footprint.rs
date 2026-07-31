//! `cargo xtask footprint` — measure what the app actually costs at rest.
//!
//! The budget is binding (R-BE-31, D17): **≤ 25 MB idle private RSS** with the tray, and
//! **≤ 12 MB for the empty shell** at D0. Those numbers are the point of the rewrite, and
//! the discipline that makes them meaningful is that they are claims until measured — a
//! regression may be consciously re-budgeted, but it may never be silently missed.
//!
//! The two platforms are measured with different tools reporting different things, so the
//! numbers are **not comparable across platforms** and the report says which was used.
//! Release CI archives the output rather than failing on it (R-DEL-7): CI runners measure
//! memory noisily, so a regression blocks release by human judgement.

use anyhow::{Context as _, bail};

pub fn report() -> anyhow::Result<()> {
    let pid = find_curio()?;
    let kib = private_memory_kib(pid)?;
    let mb = kib as f64 / 1024.0;

    println!("curio pid {pid}");
    println!("{}: {mb:.1} MB ({kib} KiB)", method());
    println!();
    println!("budget: idle RSS ≤ 25 MB with tray; empty shell ≤ 12 MB (ARCH-01 §Budget)");
    println!(
        "note: Windows and macOS numbers are NOT comparable — different counters measure \
         different things. Record which was used."
    );

    if mb > 25.0 {
        println!();
        println!(
            "OVER BUDGET. This does not fail the gate — record the number in \
             docs/architecture/D0-report.md and either fix it or revise the budget \
             consciously (D17)."
        );
    }

    Ok(())
}

#[cfg(windows)]
fn method() -> &'static str {
    // NOT Working Set. Windows trims the working set of an idle process aggressively — an
    // idle Curio measured that way reports about 20 KiB, which is not a triumph, it is the
    // OS having paged out memory the process still owns. Private Bytes (private commit) is
    // what the process actually holds and cannot be trimmed away, which is the honest
    // number to hold a budget against.
    "private bytes (private commit)"
}

#[cfg(not(windows))]
fn method() -> &'static str {
    "resident set size, via ps"
}

#[cfg(windows)]
fn find_curio() -> anyhow::Result<u32> {
    let output = std::process::Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq curio.exe", "/FO", "CSV", "/NH"])
        .output()
        .context("could not run tasklist")?;
    let text = String::from_utf8_lossy(&output.stdout);

    // "curio.exe","1234","Console","1","12,345 K"
    for line in text.lines() {
        let fields: Vec<&str> = line.split("\",\"").collect();
        if let Some(Ok(pid)) = fields.get(1).map(|f| f.trim_matches('"').parse::<u32>()) {
            return Ok(pid);
        }
    }
    bail!("curio is not running — start it with `cargo run`, then measure")
}

#[cfg(not(windows))]
fn find_curio() -> anyhow::Result<u32> {
    let output = std::process::Command::new("pgrep")
        .args(["-x", "curio"])
        .output()
        .context("could not run pgrep")?;
    let text = String::from_utf8_lossy(&output.stdout);

    text.lines()
        .next()
        .and_then(|line| line.trim().parse::<u32>().ok())
        .context("curio is not running — start it with `cargo run`, then measure")
}

#[cfg(windows)]
fn private_memory_kib(pid: u32) -> anyhow::Result<u64> {
    // `tasklist` reports Working Set, which Windows trims out from under an idle process —
    // measuring Curio that way returns roughly 20 KiB, which says nothing about what the
    // app costs. PrivateMemorySize64 is private commit: memory this process owns, that no
    // other process shares, and that the OS cannot trim away behind our back.
    let output = std::process::Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            &format!("(Get-Process -Id {pid}).PrivateMemorySize64"),
        ])
        .output()
        .context("could not query the process's private memory")?;

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map(|bytes| bytes / 1024)
        .with_context(|| format!("could not read private memory for pid {pid}"))
}

#[cfg(not(windows))]
fn private_memory_kib(pid: u32) -> anyhow::Result<u64> {
    let output = std::process::Command::new("ps")
        .args(["-o", "rss=", "-p", &pid.to_string()])
        .output()
        .context("could not run ps")?;

    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .context("could not read rss")
}
