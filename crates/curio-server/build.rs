//! Guarantee the SPA asset directory exists before `rust-embed` looks at it.
//!
//! `web/spa/dist` is a build output and is gitignored, so a fresh clone does not have it
//! and `rust-embed` fails at macro-expansion time on a directory that is not there. That
//! would make `cargo build` depend on having run `npm run build` first, which turns a
//! two-command onboarding into a four-command one and makes the Rust build fail with an
//! error about a frontend.
//!
//! Creating the directory keeps the two build systems independent: the binary compiles
//! from a fresh clone, and serving simply reports that the dashboard has not been built
//! yet (see `spa.rs`).

use std::path::Path;

fn main() {
    let dist = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../web/spa/dist");
    if !dist.exists() {
        let _ = std::fs::create_dir_all(&dist);
    }

    // Rebuild when the assets change, so a `npm run build` is picked up without a
    // `cargo clean`.
    println!("cargo:rerun-if-changed=../../web/spa/dist");
}
