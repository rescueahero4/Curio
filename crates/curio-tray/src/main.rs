//! `curio` — the binary.
//!
//! The tray runs on the main thread and owns the app's lifetime; the service runs on one
//! dedicated thread; one mpsc channel joins them (R-BE-1, R-BE-2). This file is the boot
//! order from R-BE-5 made literal, and the order is load-bearing at every step — see the
//! comments in [`main`].
//!
//! ## What is not here yet
//!
//! This is the walking skeleton. It boots, serves `/health`, publishes `runtime.json`, and
//! shuts down cleanly — which is what the D0 spike needs to measure (tray behavior, empty-
//! shell RSS, EcoQoS). The jobs worker, the projects watcher, the `/api` surface, and the
//! MCP mount land in P1–P4.

// Windows: no console window for a tray app (R-DEL-9). Inherited stdio pipes still work,
// which is what `--mcp-stdio` and `curio-nmh` rely on — a GUI-subsystem process spawned
// with redirected handles reads and writes them normally. Verified as part of D0.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod boot;
mod cli;
mod service;
mod single_instance;
mod tray;

use std::sync::mpsc as blocking_mpsc;

use curio_runtime::{Discovery, RuntimeFile};
use tao::event::Event;
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tokio::sync::mpsc;
use tray_icon::menu::MenuEvent;

use crate::cli::Invocation;
use crate::service::{Command, ServiceThreadConfig, Status};
use crate::single_instance::InstanceGuard;
use crate::tray::TrayMenu;

/// Events the tray loop reacts to.
enum UserEvent {
    Menu(MenuEvent),
    /// The service thread changed status. Delivered as an event rather than polled so the
    /// loop can stay in `ControlFlow::Wait` — the ~0% idle CPU line in the budget table is
    /// a promise about exactly this loop (R-BE-31).
    Status(Status),
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        // stderr, never stdout. `--mcp-stdio` reserves stdout for protocol frames, and a
        // diagnostic line ahead of a JSON-RPC frame breaks the agent that spawned us
        // (R-MCP-5).
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_env("CURIO_LOG")
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    match cli::parse(std::env::args().skip(1)) {
        // FIRST, before the single-instance guard and before any side effect. This mode is
        // not a second app launch: it opens no database, binds nothing, and starts no tray
        // (R-BE-5, D24).
        Invocation::McpStdio => run_mcp_stdio(),
        Invocation::Version => {
            println!("curio {}", curio_core::VERSION);
            Ok(())
        }
        Invocation::Help => {
            print!("{}", cli::usage());
            Ok(())
        }
        Invocation::Run { open_browser } => run_app(open_browser),
    }
}

/// The MCP stdio proxy.
///
/// A thin forwarder to the live instance's `/mcp`, which is what preserves the
/// single-writer invariant and the event fan-out by construction (D24). **P4** — for now
/// it reports the one thing it can already establish honestly, which is whether there is
/// an instance to forward to at all.
fn run_mcp_stdio() -> anyhow::Result<()> {
    let path = curio_core::paths::runtime_file()?;
    match RuntimeFile::discover(&path) {
        Discovery::Live(_) => {
            anyhow::bail!("the MCP stdio proxy is not implemented yet (P4)");
        }
        // Never a hang, and never a direct database open (R-MCP-5).
        Discovery::Stale | Discovery::Absent => {
            anyhow::bail!("Curio isn't running. Start Curio and try again.");
        }
    }
}

fn run_app(open_browser: bool) -> anyhow::Result<()> {
    let data_root = boot::resolve_data_root()?;
    let app_data = curio_core::paths::app_data_dir()?;
    let runtime_file = app_data.join(curio_runtime::FILE_NAME);

    // Before every other side effect (R-BE-4). Two instances would each hold a write
    // connection to one library, which breaks the invariant the whole storage layer rests
    // on.
    let _guard = match InstanceGuard::acquire(&app_data.join(curio_core::paths::LOCK_FILE_NAME)) {
        Ok(guard) => guard,
        Err(single_instance::Error::AlreadyRunning) => {
            // Not a failure. A second launch's job is to bring the existing instance's
            // dashboard forward and get out of the way.
            return open_existing_instance(&runtime_file);
        }
        Err(err) => return Err(err.into()),
    };

    // We hold the lock, so any runtime.json here was left by a process that is gone.
    if let Discovery::Stale = RuntimeFile::discover(&runtime_file) {
        tracing::info!("reclaiming a stale runtime.json from a previous run");
        RuntimeFile::remove(&runtime_file)?;
    }

    // After the guard, before the database (R-BE-5). A second launch that created
    // directories on its way to discovering it was second would leave marks on a library
    // it never opened.
    let settings = boot::load_config(&data_root)?;
    let quit_token = boot::mint_quit_token(&app_data.join(curio_core::paths::LOCK_FILE_NAME))?;

    let (command_tx, command_rx) = mpsc::unbounded_channel::<Command>();
    let (status_tx, status_rx) = blocking_mpsc::channel::<Status>();

    let service_config = ServiceThreadConfig {
        data_root,
        runtime_file,
        port: cli::port_override(),
        quit_token,
        settings,
    };
    let service_thread = std::thread::Builder::new()
        .name("curio-service".to_owned())
        .spawn(move || service::run(service_config, command_rx, status_tx))?;

    run_tray_loop(command_tx, status_rx, open_browser, service_thread)
}

/// Own the main thread until the user quits.
fn run_tray_loop(
    command_tx: mpsc::UnboundedSender<Command>,
    status_rx: blocking_mpsc::Receiver<Status>,
    open_browser: bool,
    service_thread: std::thread::JoinHandle<()>,
) -> anyhow::Result<()> {
    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build();

    // Menu clicks arrive on a channel of their own; proxy them into this loop so there is
    // one place where events are handled and the loop can otherwise sleep.
    let menu_proxy = event_loop.create_proxy();
    MenuEvent::set_event_handler(Some(move |event: MenuEvent| {
        let _ = menu_proxy.send_event(UserEvent::Menu(event));
    }));

    // Same for status changes, so the loop never polls. This thread blocks on a receive
    // rather than waking on a timer.
    let status_proxy = event_loop.create_proxy();
    std::thread::Builder::new()
        .name("curio-status".to_owned())
        .spawn(move || {
            while let Ok(status) = status_rx.recv() {
                if status_proxy.send_event(UserEvent::Status(status)).is_err() {
                    break;
                }
            }
        })?;

    let mut menu: Option<TrayMenu> = None;
    let mut current = Status::Starting;
    let mut opened = false;
    let mut service_thread = Some(service_thread);

    event_loop.run(move |event, _target, control_flow| {
        // Sleep until something happens. The idle-CPU budget is a promise about this line.
        *control_flow = ControlFlow::Wait;

        match event {
            Event::NewEvents(tao::event::StartCause::Init) => {
                // macOS requires the tray to be created after the event loop is running;
                // doing it here works on both platforms, so there is one code path.
                match TrayMenu::build() {
                    Ok(built) => {
                        built.apply(&current);
                        menu = Some(built);
                    }
                    Err(err) => {
                        tracing::error!(%err, "could not create the tray icon");
                        *control_flow = ControlFlow::Exit;
                    }
                }
            }

            Event::UserEvent(UserEvent::Status(status)) => {
                current = status.clone();
                if let Some(menu) = &menu {
                    menu.apply(&status);
                }

                match status {
                    Status::Running { .. } if open_browser && !opened => {
                        opened = true;
                        request_dashboard(&command_tx);
                    }
                    Status::Failed { message } => {
                        // A tray icon sitting quietly over a broken library is worse than
                        // a visible failure (R-BE-33).
                        tracing::error!("{message}");
                        *control_flow = ControlFlow::Exit;
                    }
                    Status::Stopped => *control_flow = ControlFlow::Exit,
                    _ => {}
                }
            }

            Event::UserEvent(UserEvent::Menu(clicked)) => {
                let Some(menu) = &menu else { return };

                if clicked.id == menu.quit.id() {
                    // Both the tray's Quit and POST /api/system/quit converge here, so
                    // there is one shutdown sequence rather than two that can drift.
                    let _ = command_tx.send(Command::Shutdown);
                } else if clicked.id == menu.pause.id() {
                    let _ = command_tx.send(if current.is_paused() {
                        Command::Resume
                    } else {
                        Command::Pause
                    });
                } else if clicked.id == menu.open.id() {
                    request_dashboard(&command_tx);
                } else if clicked.id == menu.autostart.id() {
                    // P1, with the rest of the platform integration: SMAppService on
                    // macOS, the HKCU Run key on Windows. The OS is the authority for the
                    // state, so this reads back rather than trusting a stored flag.
                    tracing::warn!("Start at Login is not implemented yet (P1)");
                }
            }

            Event::LoopDestroyed => {
                if let Some(thread) = service_thread.take() {
                    let _ = command_tx.send(Command::Shutdown);
                    // Join so the process does not exit while the service is still
                    // checkpointing the database and removing runtime.json (R-BE-7).
                    let _ = thread.join();
                }
            }

            _ => {}
        }
    });
}

/// Ask the service for a nonce and open the dashboard with it.
///
/// The token never enters the URL: a one-time nonce does, and it authorizes exactly one
/// exchange for a session cookie before expiring (R-SEC-5, D22). A URL in browser history
/// is a URL in a log somewhere.
fn request_dashboard(command_tx: &mpsc::UnboundedSender<Command>) {
    let (reply_tx, mut reply_rx) = mpsc::unbounded_channel();
    if command_tx
        .send(Command::RequestNonce { reply: reply_tx })
        .is_err()
    {
        return;
    }

    std::thread::spawn(move || {
        let Some(url) = reply_rx.blocking_recv() else {
            return;
        };
        if let Err(err) = open_url(&url) {
            tracing::warn!(%err, "could not open the dashboard");
        }
    });
}

/// A second launch: bring the running instance's dashboard forward and exit (R-BE-4).
fn open_existing_instance(runtime_file: &std::path::Path) -> anyhow::Result<()> {
    match RuntimeFile::discover(runtime_file) {
        Discovery::Live(file) => {
            tracing::info!(port = file.port, "Curio is already running");
            // Without a nonce this lands on the "Open Curio from the tray" screen rather
            // than an error (R-FE-6a). Requesting one needs an authenticated call, which
            // arrives with the auth surface in P1.
            open_url(&format!("http://127.0.0.1:{}/", file.port))?;
            Ok(())
        }
        // The lock is held but the file is not there: another instance is mid-boot.
        Discovery::Stale | Discovery::Absent => {
            anyhow::bail!("another Curio is starting up; try again in a moment")
        }
    }
}

fn open_url(url: &str) -> std::io::Result<()> {
    #[cfg(windows)]
    let mut command = {
        // `start` is a cmd builtin, and the empty string is the window title — without it
        // cmd treats a quoted URL as the title and opens nothing.
        let mut command = std::process::Command::new("cmd");
        command.args(["/C", "start", "", url]);
        command
    };
    #[cfg(target_os = "macos")]
    let mut command = {
        let mut command = std::process::Command::new("open");
        command.arg(url);
        command
    };
    #[cfg(all(unix, not(target_os = "macos")))]
    let mut command = {
        let mut command = std::process::Command::new("xdg-open");
        command.arg(url);
        command
    };

    command.spawn().map(|_| ())
}
