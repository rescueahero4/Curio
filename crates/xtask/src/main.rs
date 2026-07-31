//! `cargo gate` — the single definition of "green".
//!
//! CI invokes this script and restates none of its steps (R-DEL-6). That is the whole
//! point: a checklist duplicated between a workflow file and a contributor's memory drifts,
//! and the drift is only discovered when someone's green branch turns red on merge.
//!
//! Steps run in a fixed order, fail-fast, cheapest first — formatting before lints before
//! tests before the frontend builds — so a trivial mistake is reported in seconds rather
//! than after a full run.

mod deps;
mod files;
mod footprint;
mod steps;

use anyhow::{Context as _, bail};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("gate");

    match command {
        "gate" => steps::gate(&args[1..]),
        "footprint" => footprint::report(),
        "deps" => deps::check().context("dependency-direction check"),
        "files" => files::check().context("file-length check"),
        "help" | "--help" | "-h" => {
            print!("{}", usage());
            Ok(())
        }
        other => {
            bail!("unknown command {other:?}\n\n{}", usage());
        }
    }
}

fn usage() -> String {
    "\
cargo xtask — Curio's development tooling

USAGE:
    cargo gate                 Run the full quality gate (R-DEL-6)
    cargo xtask gate [FLAGS]   The same, with a narrower scope
    cargo xtask footprint      Measure the running binary's private memory (R-DEL-7)
    cargo xtask deps           Only the dependency-direction assertions (R-DEL-2)
    cargo xtask files          Only the file-length check

GATE FLAGS:
    --rust-only   Skip the SPA and extension steps
    --web-only    Only the SPA and extension steps
"
    .to_owned()
}
