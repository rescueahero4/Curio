//! The service: everything that is not the tray.
//!
//! One tokio `current_thread` runtime on one dedicated thread owns the HTTP listener, the
//! database's single write connection, the jobs worker, the projects watcher, and the
//! event fan-out (R-BE-1). The tray owns the main thread and talks to this over one mpsc
//! channel — the only seam between them, kept narrow so a future two-process split stays
//! cheap.
//!
//! ## Boot order is a contract
//!
//! [`Service::start`] follows R-BE-5 exactly, and the ordering is load-bearing rather than
//! stylistic:
//!
//! ```text
//! open DB + migrate → bind 127.0.0.1:0 → write runtime.json → worker → watcher → ready
//! ```
//!
//! `runtime.json` is written **after both** the migration and the bind succeed, so its
//! existence means a healthy instance is listening on that port. A client can therefore
//! trust the file rather than probing, and no client can ever discover a half-migrated
//! database (R-BE-33). A migration failure is loud: boot fails, the file is never written,
//! and nothing advertises a broken library.
//!
//! Shutdown mirrors it in reverse, ending with the file deleted and the token dead
//! (R-BE-7).

pub mod routes;
pub mod security;
pub mod spa;
pub mod state;

use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use curio_db::Db;
use curio_runtime::RuntimeFile;
use tokio::net::TcpListener;
use tokio::sync::oneshot;

pub use state::AppState;

use crate::security::RuntimeToken;

/// What can go wrong bringing the service up.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Migrations failed, or the library is newer than this build.
    ///
    /// Fatal and visible: `runtime.json` is never written, so no client discovers a
    /// half-migrated instance (R-BE-33).
    #[error("the library could not be opened: {0}")]
    Library(#[from] curio_db::Error),

    #[error("could not bind 127.0.0.1:{port}: {source}")]
    Bind {
        port: u16,
        #[source]
        source: std::io::Error,
    },

    #[error("could not publish runtime.json: {0}")]
    Runtime(#[source] std::io::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// How to bring the service up.
#[derive(Debug, Clone)]
pub struct ServiceConfig {
    /// The library file.
    pub database: PathBuf,
    /// Where to publish `runtime.json` — the app-data directory, never the data root
    /// (R-DA-2).
    pub runtime_file: PathBuf,
    /// `None` means an OS-assigned ephemeral port, which is the default and the decision
    /// that removes the entire port-conflict class (D10). A value pins it for development
    /// and unpacked extension installs (D11). There is no port-walk in either mode.
    pub port: Option<u16>,
    /// The single stamped version (R-DEL-12).
    pub version: String,
}

/// A running service.
pub struct Service {
    state: AppState,
    port: u16,
    runtime_file: PathBuf,
    shutdown: oneshot::Sender<()>,
    finished: oneshot::Receiver<()>,
}

impl Service {
    /// Bring the service up, following R-BE-5's boot order.
    ///
    /// # Errors
    /// Returns [`Error::Library`] if the database cannot be opened or migrated,
    /// [`Error::Bind`] if the port is unavailable, or [`Error::Runtime`] if the file
    /// cannot be published.
    pub async fn start(config: ServiceConfig) -> Result<Self> {
        // 1. The library, migrated. Before the bind, so a broken library never reaches
        //    the point of having an address to advertise.
        let db = Db::open(&config.database)?;
        tracing::info!(schema = db.schema_version().unwrap_or(-1), "library opened");

        // 2. The address. One bind attempt, bound or failed — no walk in any mode
        //    (R-BE-6).
        let requested = config.port.unwrap_or(0);
        let listener = TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, requested)))
            .await
            .map_err(|source| Error::Bind {
                port: requested,
                source,
            })?;
        let port = listener.local_addr()?.port();

        // 3. The token, minted fresh for this run and dead at quit (D21).
        let token = RuntimeToken::mint();
        let state = AppState::new(token, config.version.clone(), db);

        // 4. Only now is there something true to advertise.
        let published = RuntimeFile {
            port,
            token: state.token().expose().to_owned(),
            pid: std::process::id(),
            version: config.version.clone(),
            state: state.run_state(),
        };
        published
            .write_atomic(&config.runtime_file)
            .map_err(Error::Runtime)?;

        let (shutdown, shutdown_rx) = oneshot::channel();
        let (finished_tx, finished) = oneshot::channel();

        let app = routes::build(state.clone(), port);
        tokio::spawn(async move {
            let served = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await;
            if let Err(err) = served {
                tracing::error!(%err, "the listener stopped unexpectedly");
            }
            let _ = finished_tx.send(());
        });

        // The jobs worker and the projects watcher start here in P2. Neither exists yet,
        // so the boot order has nothing to run between publishing and ready.

        tracing::info!(port, "listening on 127.0.0.1");
        Ok(Self {
            state,
            port,
            runtime_file: config.runtime_file,
            shutdown,
            finished,
        })
    }

    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    #[must_use]
    pub fn state(&self) -> &AppState {
        &self.state
    }

    /// Pause or resume, and republish the state.
    ///
    /// Soft-disable: the listener stays bound and reads keep working (D2, D25). The state
    /// is republished because `runtime.json` is where a newly-launched extension reads it
    /// before any socket exists (R-EXT-4).
    ///
    /// # Errors
    /// Returns [`Error::Runtime`] if the file cannot be rewritten.
    pub fn set_paused(&self, paused: bool) -> Result<()> {
        self.state.set_paused(paused);

        // Checkpoint on pause so a user who backs up while paused — the natural moment to
        // do it — copies a complete library rather than one missing its WAL (R-DA-9).
        if paused {
            self.state.with_db(|db| {
                if let Err(err) = db.checkpoint() {
                    tracing::warn!(%err, "checkpoint on pause failed");
                }
            });
        }

        self.republish()
    }

    /// Shut down in R-BE-7's order, ending with the token dead.
    ///
    /// # Errors
    /// Returns [`Error::Runtime`] if `runtime.json` cannot be removed.
    pub async fn shutdown(self) -> Result<()> {
        // The watcher and the worker stop here in P2, before the listener, so no job is
        // interrupted mid-write.

        let _ = self.shutdown.send(());
        let _ = self.finished.await;

        self.state.with_db(|db| {
            if let Err(err) = db.checkpoint() {
                tracing::warn!(%err, "checkpoint on quit failed");
            }
        });

        // Last: the file goes, and with it every client's ability to find a token that no
        // longer authenticates anything.
        RuntimeFile::remove(&self.runtime_file).map_err(Error::Runtime)?;
        tracing::info!("stopped");
        Ok(())
    }

    fn republish(&self) -> Result<()> {
        let published = RuntimeFile {
            port: self.port,
            token: self.state.token().expose().to_owned(),
            pid: std::process::id(),
            version: self.state.version().to_owned(),
            state: self.state.run_state(),
        };
        published
            .write_atomic(&self.runtime_file)
            .map_err(Error::Runtime)
    }
}
