//! `curio-nmh` — the native-messaging host.
//!
//! Chrome extensions cannot read files. Curio binds an ephemeral port and publishes the
//! truth in `runtime.json`, so native messaging is the only channel through which an
//! extension can learn where the app is — indirectly, via this helper, which the installer
//! registers with the browser.
//!
//! The whole protocol is one exchange: Chrome spawns this binary, the extension sends a
//! request, this replies `{port, token, state}`, and the process exits (R-EXT-4). No
//! long-lived channel, no bulk transport. It also closes the last manual setup step — the
//! token rides the same reply, so a fresh install captures with zero configuration.
//!
//! ## Two rules that are easy to break and hard to debug
//!
//! * **stdout is reserved for framed messages.** Every diagnostic goes to stderr. A stray
//!   `println!` corrupts the length-prefixed stream, and the browser-side symptom is an
//!   extension that mysteriously cannot connect (R-EXT-5). This is the classic
//!   native-messaging defect.
//! * **Staleness is a PID check and nothing more.** This binary embeds no HTTP client
//!   (R-BE-34). If the recorded process is gone it answers `{state: "stale"}`, which the
//!   extension treats exactly as not-running (R-EXT-6) — and it never tries to launch the
//!   app, because FR-22 says a capture tool must not start things behind the user's back.
//!
//! Every dependency shared with the server is a chance to drag an async runtime into a
//! process that lives for one message, which is why this crate takes almost none.

mod register;

use std::io::{Read as _, Write as _};

use curio_runtime::{Discovery, RuntimeFile};

/// Chrome's cap on a host-to-browser message.
///
/// Ours are two orders of magnitude smaller, so this is a sanity bound rather than a
/// constraint we work against.
const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

fn main() {
    // Registration runs from an installer, not from Chrome (R-EXT-3). It is checked before
    // anything touches stdin, because the native-messaging path blocks waiting for a
    // message that will never come when a human ran this by hand.
    match std::env::args().nth(1).as_deref() {
        Some("--register") => {
            return report(
                register::register().unwrap_or_else(|err| {
                    eprintln!("curio-nmh: {err}");
                    std::process::exit(1);
                }),
                "registered",
            );
        }
        Some("--unregister") => return report(register::unregister(), "removed"),
        Some("--version") => {
            // stdout is reserved for framed messages *in host mode*. This is not host
            // mode: a human ran it, and a human reads stdout.
            println!("curio-nmh {}", curio_core::VERSION);
            return;
        }
        _ => {}
    }

    // Read one request. Its content does not matter — the extension asks the only question
    // this host can answer — but it must be consumed so the framing stays aligned.
    if let Err(err) = read_message() {
        eprintln!("curio-nmh: could not read the request: {err}");
        std::process::exit(1);
    }

    let reply = match locate() {
        Ok(reply) => reply,
        Err(err) => {
            eprintln!("curio-nmh: {err}");
            serde_json::json!({ "state": "stale" })
        }
    };

    if let Err(err) = write_message(&reply) {
        eprintln!("curio-nmh: could not write the reply: {err}");
        std::process::exit(1);
    }
}

/// Print what registration did, in a form an installer log can carry.
///
/// Every diagnostic goes to **stderr** except this one, and this one only runs when a human
/// or an installer invoked the binary directly — never when Chrome spawned it, where a
/// stray byte on stdout corrupts the framed stream (R-EXT-5).
fn report(report: register::Report, verb: &str) {
    if !report.any() {
        // Exit 2, not 1: nothing failed, there was simply no Chromium-family browser to
        // register with. An installer can warn ("the extension will need pairing by hand")
        // without treating the install as broken — which it is not.
        println!("curio-nmh: no browser was {verb}");
        for skipped in &report.skipped {
            println!("curio-nmh: skipped {skipped}");
        }
        std::process::exit(2);
    }
    {
        println!("curio-nmh: {verb} for {}", report.registered.join(", "));
    }
    for skipped in &report.skipped {
        println!("curio-nmh: skipped {skipped}");
    }
}

/// Read `runtime.json` and describe what it found.
fn locate() -> Result<serde_json::Value, String> {
    let path = curio_core::paths::runtime_file().map_err(|err| err.to_string())?;

    Ok(match RuntimeFile::discover(&path) {
        Discovery::Live(file) => serde_json::json!({
            "port": file.port,
            "token": file.token,
            "state": file.state,
        }),
        // Absent and stale are one answer to the extension: there is nothing to talk to.
        // Distinguishing them would tell the popup nothing it could act on — FR-22 rules
        // out launching the app either way.
        Discovery::Stale | Discovery::Absent => serde_json::json!({ "state": "stale" }),
    })
}

/// Read one length-prefixed message from stdin.
fn read_message() -> std::io::Result<serde_json::Value> {
    let mut stdin = std::io::stdin().lock();

    let mut length = [0u8; 4];
    stdin.read_exact(&mut length)?;
    // Native byte order, per the native-messaging protocol — not network order.
    let length = u32::from_ne_bytes(length) as usize;

    if length > MAX_MESSAGE_BYTES {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("message of {length} bytes exceeds the 1 MB limit"),
        ));
    }

    let mut body = vec![0u8; length];
    stdin.read_exact(&mut body)?;
    serde_json::from_slice(&body).map_err(std::io::Error::other)
}

/// Write one length-prefixed message to stdout.
///
/// The only thing in this program permitted to touch stdout.
fn write_message(message: &serde_json::Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(message).map_err(std::io::Error::other)?;
    let length = u32::try_from(body.len())
        .map_err(|_| std::io::Error::other("reply is larger than the protocol allows"))?;

    let mut stdout = std::io::stdout().lock();
    stdout.write_all(&length.to_ne_bytes())?;
    stdout.write_all(&body)?;
    stdout.flush()
}
