//! The service thread, and the one seam between it and the tray.
//!
//! The tray owns the main thread because both target platforms insist on it — macOS
//! requires UI on the main thread, and a Windows tray icon needs a message pump on the
//! thread that created it. Everything real therefore runs here, on one tokio
//! `current_thread` runtime on one dedicated thread (R-BE-1).
//!
//! The two channels below are the **entire** interface between them (R-BE-2): commands go
//! one way, status comes back the other, and no state is shared. That narrowness is
//! deliberate — it is what keeps a future split into two processes a refactor rather than
//! a rewrite.

use std::path::PathBuf;

use std::sync::mpsc as blocking_mpsc;

use curio_server::{Service, ServiceConfig};
use tokio::sync::mpsc;

/// What the tray can ask the service to do.
#[derive(Debug)]
pub enum Command {
    /// Stop accepting mutations. Reads, search and browsing keep working (D25).
    Pause,
    Resume,
    /// Mint a single-use nonce so the dashboard can be opened without a token in the URL
    /// (R-SEC-5). The reply carries the URL to open.
    RequestNonce {
        reply: mpsc::UnboundedSender<String>,
    },
    /// Shut down cleanly, in R-BE-7's order.
    Shutdown,
}

/// What the tray knows about the service.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    Starting,
    Running {
        port: u16,
    },
    Paused {
        port: u16,
    },
    /// Boot failed. The message is shown to the user, because a tray icon over a broken
    /// library is worse than a visible failure (R-BE-33).
    Failed {
        message: String,
    },
    Stopped,
}

impl Status {
    /// The tray's status line.
    #[must_use]
    pub fn label(&self) -> String {
        match self {
            Status::Starting => "Starting…".to_owned(),
            Status::Running { port } => format!("Running on 127.0.0.1:{port}"),
            Status::Paused { port } => format!("Paused — 127.0.0.1:{port}"),
            Status::Failed { message } => format!("Failed: {message}"),
            Status::Stopped => "Stopped".to_owned(),
        }
    }

    #[must_use]
    pub fn port(&self) -> Option<u16> {
        match self {
            Status::Running { port } | Status::Paused { port } => Some(*port),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_paused(&self) -> bool {
        matches!(self, Status::Paused { .. })
    }
}

/// Everything the service thread needs to start.
pub struct ServiceThreadConfig {
    /// Everything the user owns. `library.db` sits directly inside it (R-DA-1).
    pub data_root: PathBuf,
    pub runtime_file: PathBuf,
    pub port: Option<u16>,
    /// Minted into the lock file at boot; never the runtime token (R-SEC-8).
    pub quit_token: String,
    pub settings: curio_core::config::Config,
}

/// Status flows back on a blocking channel rather than a tokio one: the tray thread has no
/// runtime to await on, and a blocking receive is exactly what lets it sleep instead of
/// polling — which is the ~0% idle CPU line in the budget table (R-BE-31).
pub type StatusSender = blocking_mpsc::Sender<Status>;

/// Run the service until told to stop. Called on the service thread.
///
/// Builds its own `current_thread` runtime: the multi-threaded flavour would add a worker
/// pool to a process whose entire point is a small idle footprint, and there is nothing
/// here to parallelise across cores.
pub fn run(
    config: ServiceThreadConfig,
    mut commands: mpsc::UnboundedReceiver<Command>,
    status: StatusSender,
) {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            let _ = status.send(Status::Failed {
                message: err.to_string(),
            });
            return;
        }
    };

    runtime.block_on(async move {
        let service = match Service::start(ServiceConfig {
            data_root: config.data_root,
            runtime_file: config.runtime_file,
            port: config.port,
            version: curio_core::VERSION.to_owned(),
            quit_token: config.quit_token,
            config: config.settings,
        })
        .await
        {
            Ok(service) => service,
            Err(err) => {
                // Loud, per R-BE-33: the tray shows the failure and runtime.json was never
                // written, so nothing advertises a broken instance.
                tracing::error!(%err, "the service could not start");
                let _ = status.send(Status::Failed {
                    message: err.to_string(),
                });
                return;
            }
        };

        let port = service.port();
        let _ = status.send(Status::Running { port });

        // Two ways to stop, one shutdown sequence (R-BE-7). The tray's Quit arrives as a
        // command; `POST /api/system/quit` raises the state's quit signal. Selecting over
        // both here is what keeps them converging rather than growing a second ordering
        // that skips the WAL checkpoint.
        loop {
            let command = tokio::select! {
                received = commands.recv() => match received {
                    Some(command) => command,
                    // The tray dropped its sender, which means the main thread is gone.
                    None => break,
                },
                () = service.state().quit_requested() => Command::Shutdown,
            };

            match command {
                Command::Pause => {
                    if let Err(err) = service.set_paused(true) {
                        tracing::warn!(%err, "could not publish the paused state");
                    }
                    let _ = status.send(Status::Paused { port });
                }
                Command::Resume => {
                    if let Err(err) = service.set_paused(false) {
                        tracing::warn!(%err, "could not publish the running state");
                    }
                    let _ = status.send(Status::Running { port });
                }
                Command::RequestNonce { reply } => {
                    // The token itself never travels this way. A nonce authorizes exactly
                    // one exchange and nothing else, so even if this URL reached a log it
                    // would be inert within thirty seconds (R-SEC-5).
                    let nonce = service.state().mint_nonce();
                    let _ = reply.send(format!("http://127.0.0.1:{port}/?t={nonce}"));
                }
                Command::Shutdown => break,
            }
        }

        if let Err(err) = service.shutdown().await {
            tracing::warn!(%err, "shutdown did not complete cleanly");
        }
        let _ = status.send(Status::Stopped);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_status_line_names_the_bound_port() {
        // With an ephemeral port, the tray is where a user finds out which one they got.
        assert_eq!(
            Status::Running { port: 51_234 }.label(),
            "Running on 127.0.0.1:51234"
        );
    }

    #[test]
    fn paused_reads_as_paused_not_as_stopped() {
        // D25: paused is a soft-disable. A user who reads "Stopped" would reasonably
        // conclude their library is unreachable, when browsing still works.
        let paused = Status::Paused { port: 51_234 };
        assert!(paused.label().contains("Paused"));
        assert!(paused.is_paused());
        assert_eq!(paused.port(), Some(51_234));
    }

    #[test]
    fn a_failed_boot_surfaces_its_reason() {
        // R-BE-33. "Failed" alone would send the user to the logs; the schema-too-new
        // message in particular tells them exactly what to do.
        let failed = Status::Failed {
            message: "library is newer than this build".to_owned(),
        };
        assert!(failed.label().contains("newer than this build"));
        assert_eq!(failed.port(), None);
    }

    #[test]
    fn a_starting_service_has_no_port_yet() {
        // The port does not exist until the bind succeeds, and the bind happens after
        // migrations (R-BE-5).
        assert_eq!(Status::Starting.port(), None);
        assert!(!Status::Starting.is_paused());
    }
}
