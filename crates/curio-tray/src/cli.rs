//! Command-line arguments.
//!
//! Hand-rolled rather than delegated to a parser crate. The surface is four flags, and
//! the idle-footprint budget makes a dependency that exists to format help text a poor
//! trade (R-BE-31).

/// What this invocation should do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Invocation {
    /// Run the app: tray, service, dashboard.
    Run { open_browser: bool },
    /// Act as the MCP stdio proxy for a client that spawned us.
    ///
    /// Branches **first**, before the single-instance guard and before anything is
    /// written, because this mode must not behave like a second app launch — it opens no
    /// database, starts no listener, no tray, no watcher, no worker (R-BE-5, R-MCP-5, D24).
    McpStdio,
    /// Print the version and exit.
    Version,
    /// Print usage and exit.
    Help,
}

/// Parse arguments, honouring the environment overrides the previous implementation had.
#[must_use]
pub fn parse<I, S>(args: I) -> Invocation
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut open_browser = !env_flag("CURIO_NO_OPEN");

    for arg in args {
        match arg.as_ref() {
            "--mcp-stdio" => return Invocation::McpStdio,
            "--version" | "-V" => return Invocation::Version,
            "--help" | "-h" => return Invocation::Help,
            "--no-open" => open_browser = false,
            _ => {}
        }
    }

    Invocation::Run { open_browser }
}

/// The port override, if any.
///
/// Precedence is fixed (R-BE-6): `CURIO_PORT` wins over the config file, and the legacy
/// `CURIOL_PORT` is honoured only when `CURIO_PORT` is unset — an existing user's
/// environment keeps working without the old name quietly outranking the new one.
///
/// Returns `None` when neither is set **or** when the value does not parse, because a
/// typo'd port should fall back to the ephemeral default rather than stop the app.
#[must_use]
pub fn port_override() -> Option<u16> {
    read_port("CURIO_PORT").or_else(|| read_port("CURIOL_PORT"))
}

fn read_port(key: &str) -> Option<u16> {
    let raw = std::env::var(key).ok()?;
    match raw.trim().parse::<u16>() {
        Ok(port) if port >= 1024 => Some(port),
        _ => {
            eprintln!("curio: ignoring {key}={raw:?} — expected a port of 1024 or above");
            None
        }
    }
}

fn env_flag(key: &str) -> bool {
    std::env::var(key).is_ok_and(|value| value == "1" || value.eq_ignore_ascii_case("true"))
}

/// The usage text.
#[must_use]
pub fn usage() -> String {
    format!(
        "curio {version}\n\
         A local-first design inspiration library.\n\
         \n\
         USAGE:\n    curio [OPTIONS]\n\
         \n\
         OPTIONS:\n\
         \x20   --no-open        Do not open the dashboard at startup\n\
         \x20   --mcp-stdio      Act as an MCP stdio server for an agent that spawned this process\n\
         \x20   -V, --version    Print the version\n\
         \x20   -h, --help       Print this message\n\
         \n\
         ENVIRONMENT:\n\
         \x20   CURIO_DATA_ROOT  Where the library lives (default: ~/Curio)\n\
         \x20   CURIO_PORT       Pin the port instead of using an ephemeral one\n\
         \x20   CURIO_NO_OPEN    Set to 1 to suppress opening the dashboard\n",
        version = curio_core::VERSION,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_means_run_and_open() {
        assert_eq!(
            parse(Vec::<String>::new()),
            Invocation::Run { open_browser: true }
        );
    }

    #[test]
    fn no_open_suppresses_the_browser() {
        assert_eq!(
            parse(["--no-open"]),
            Invocation::Run {
                open_browser: false
            }
        );
    }

    #[test]
    fn mcp_stdio_wins_over_everything_else() {
        // It must branch before any side effect, so it cannot be conditional on the rest
        // of the command line (R-BE-5).
        assert_eq!(parse(["--no-open", "--mcp-stdio"]), Invocation::McpStdio);
        assert_eq!(parse(["--mcp-stdio", "--version"]), Invocation::McpStdio);
    }

    #[test]
    fn unknown_arguments_are_ignored_rather_than_fatal() {
        // A launcher, a login item, or a shell may append arguments we did not ask for.
        // Refusing to start over one would be a support thread with no upside.
        assert_eq!(
            parse(["--psn_0_12345"]),
            Invocation::Run { open_browser: true }
        );
    }

    #[test]
    fn version_and_help_are_recognised_in_both_forms() {
        assert_eq!(parse(["--version"]), Invocation::Version);
        assert_eq!(parse(["-V"]), Invocation::Version);
        assert_eq!(parse(["--help"]), Invocation::Help);
        assert_eq!(parse(["-h"]), Invocation::Help);
    }

    #[test]
    fn usage_names_the_stamped_version() {
        // R-DEL-12: one version, and this is one of the surfaces that reports it.
        assert!(usage().contains(curio_core::VERSION));
    }
}
