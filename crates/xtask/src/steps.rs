//! The gate's steps, in order.

use std::process::Command;

use anyhow::{Context as _, bail};

/// Run the gate.
///
/// Fail-fast and cheapest-first: a missing newline is reported in seconds rather than
/// after a full test run and two frontend builds.
pub fn gate(flags: &[String]) -> anyhow::Result<()> {
    let rust_only = flags.iter().any(|flag| flag == "--rust-only");
    let web_only = flags.iter().any(|flag| flag == "--web-only");

    if !web_only {
        step("1/8  cargo fmt --check", || {
            cargo(&["fmt", "--all", "--check"])
        })?;
        step("2/8  cargo clippy -D warnings", || {
            cargo(&[
                "clippy",
                "--workspace",
                "--all-targets",
                "--",
                "-D",
                "warnings",
            ])
        })?;
        step("3/8  cargo test", || cargo(&["test", "--workspace"]))?;
    }

    if !rust_only {
        step("4/8  SPA typecheck, lint, build", || {
            npm("web/spa", &["run", "gate"])
        })?;
        step("5/8  extension typecheck and build", || {
            npm("web/extension", &["run", "gate"])
        })?;
    }

    if !web_only {
        step("6/8  cargo-deny licenses and advisories", || {
            if !cargo_subcommand_exists("deny")? {
                // Deliberately not installed from inside the gate: running the gate should
                // not mutate the developer's toolchain as a side effect. CI installs it
                // explicitly, so this skip never happens where it would matter.
                println!("      skipped — install with `cargo install cargo-deny`");
                return Ok(());
            }
            cargo(&["deny", "check", "licenses", "advisories"])
        })?;
        step("7/8  file length", crate::files::check)?;
        step("8/8  dependency direction", crate::deps::check)?;
    }

    println!("\ngate: green");
    Ok(())
}

fn step(label: &str, run: impl FnOnce() -> anyhow::Result<()>) -> anyhow::Result<()> {
    println!("{label}");
    run().with_context(|| format!("gate step failed: {label}"))
}

fn cargo(args: &[&str]) -> anyhow::Result<()> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    run(Command::new(cargo).args(args), ".")
}

/// Run an npm script, treating an absent workspace as a skip rather than a failure.
///
/// The Rust and JavaScript halves are independent by design (R-DEL-3), so a contributor
/// working only on the backend should not be blocked by a missing `node_modules`.
fn npm(dir: &str, args: &[&str]) -> anyhow::Result<()> {
    let root = repo_root()?;
    let workspace = root.join(dir);

    if !workspace.join("package.json").is_file() {
        println!("      skipped — {dir} has no package.json");
        return Ok(());
    }
    if !workspace.join("node_modules").is_dir() {
        println!("      skipped — run `npm --prefix {dir} install` first");
        return Ok(());
    }

    // npm ships as a .cmd shim on Windows, which CreateProcess will not execute directly.
    #[cfg(windows)]
    let mut command = {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("npm").args(args);
        command
    };
    #[cfg(not(windows))]
    let mut command = {
        let mut command = Command::new("npm");
        command.args(args);
        command
    };

    run(&mut command, dir)
}

fn run(command: &mut Command, dir: &str) -> anyhow::Result<()> {
    let root = repo_root()?;
    let status = command
        .current_dir(root.join(dir))
        .status()
        .with_context(|| format!("could not start {:?}", command.get_program()))?;

    if !status.success() {
        bail!("exited with {status}");
    }
    Ok(())
}

/// Whether `cargo <name>` resolves to an installed subcommand.
///
/// Asked by listing the subcommands rather than by running the tool and interpreting a
/// failure: a missing subcommand and a real gate failure both exit non-zero, and telling
/// them apart from the exit code alone would mean a genuine licence violation could be
/// reported as "not installed".
fn cargo_subcommand_exists(name: &str) -> anyhow::Result<bool> {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .arg("--list")
        .output()
        .context("could not list cargo subcommands")?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line.split_whitespace().next() == Some(name)))
}

/// The repository root, found from this crate's manifest rather than the current
/// directory, so the gate behaves the same wherever it is invoked from.
pub fn repo_root() -> anyhow::Result<std::path::PathBuf> {
    let manifest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest
        .ancestors()
        .nth(2)
        .map(std::path::Path::to_path_buf)
        .context("could not locate the repository root")
}
