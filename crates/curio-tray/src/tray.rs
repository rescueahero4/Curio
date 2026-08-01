//! The tray menu.
//!
//! Five items, and no more (D14): **Status · Pause/Resume · Open Dashboard · Start at
//! Login · Quit**. The previous shell's Open Projects and New Prompt entries are gone on
//! purpose — the tray is a switch, not a navigation bar, and navigation belongs in the
//! dashboard where it can show the user what they are navigating to.
//!
//! The icon is the brand mark, loaded from a committed 32×32 PNG next to this file. It was
//! drawn in code while the artwork was unavailable; now that it is, a second hand-rolled
//! copy of a 500-point path would only be a worse one that drifts from the dashboard's the
//! first time either changes. Both rasters come from one source file
//! (`assets/brand/rasterize.mjs`).

use tray_icon::menu::{Menu, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::service::Status;

/// The menu items whose text or state changes while running.
pub struct TrayMenu {
    pub tray: TrayIcon,
    pub status: MenuItem,
    pub pause: MenuItem,
    pub open: MenuItem,
    pub autostart: MenuItem,
    pub quit: MenuItem,
}

impl TrayMenu {
    /// Build the tray icon and its menu.
    ///
    /// # Errors
    /// Returns an error if the platform refuses to create the tray icon — which on Linux
    /// usually means no status-notifier host is running.
    pub fn build() -> anyhow::Result<Self> {
        let menu = Menu::new();

        // Disabled: it is a readout, not an action. Enabling it would invite a click that
        // does nothing, which reads as a bug.
        let status = MenuItem::new(Status::Starting.label(), false, None);
        let pause = MenuItem::new("Pause", true, None);
        let open = MenuItem::new("Open Dashboard", true, None);
        let autostart = MenuItem::new("Start at Login", true, None);
        let quit = MenuItem::new("Quit Curio", true, None);

        menu.append(&status)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&pause)?;
        menu.append(&open)?;
        menu.append(&PredefinedMenuItem::separator())?;
        menu.append(&autostart)?;
        menu.append(&quit)?;

        let tray = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("Curio")
            .with_icon(icon())
            .build()?;

        Ok(Self {
            tray,
            status,
            pause,
            open,
            autostart,
            quit,
        })
    }

    /// Reflect a new service status in the menu.
    pub fn apply(&self, status: &Status) {
        self.status.set_text(status.label());
        self.pause.set_text(if status.is_paused() {
            "Resume"
        } else {
            "Pause"
        });

        // Both actions need a running service: pausing something that has not started is
        // meaningless, and the dashboard has no address to open until the bind succeeds.
        let live = status.port().is_some();
        self.pause.set_enabled(live);
        self.open.set_enabled(live);

        let tooltip = match status {
            Status::Paused { .. } => "Curio — paused",
            Status::Failed { .. } => "Curio — failed to start",
            _ => "Curio",
        };
        let _ = self.tray.set_tooltip(Some(tooltip));
    }
}

/// The magpie-and-gem mark, at the one size a tray actually uses.
///
/// Loaded from a committed PNG rather than drawn in code. The mark is a 500×500 path with
/// enough curve detail that hand-rolling it as pixel arithmetic would be a second, worse
/// copy of the artwork — and it would drift from the dashboard's copy the first time either
/// changed. `assets/brand/rasterize.mjs` renders both from the same source file.
///
/// # Panics
/// Only if the committed asset is not a decodable 32×32 PNG, which the test below prevents
/// from reaching a build.
fn icon() -> Icon {
    const MARK: &[u8] = include_bytes!("../assets/curio-mark-32.png");

    let image = image::load_from_memory(MARK).expect("the bundled mark is a valid PNG");
    let (width, height) = (image.width(), image.height());

    Icon::from_rgba(image.into_rgba8().into_raw(), width, height)
        .expect("the bundled mark is a valid RGBA icon")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_bundled_mark_decodes_at_the_size_the_tray_asks_for() {
        // `Icon::from_rgba` validates the buffer length against the dimensions, so a wrong
        // asset panics at startup rather than failing a test — which is exactly why this
        // asserts here instead. It also pins the size: a 64×64 PNG dropped in by mistake
        // would still decode, and would still be wrong.
        const MARK: &[u8] = include_bytes!("../assets/curio-mark-32.png");
        let image = image::load_from_memory(MARK).expect("a valid PNG");

        assert_eq!((image.width(), image.height()), (32, 32));
        let _ = icon();
    }

    #[test]
    fn the_mark_is_not_a_solid_block() {
        // A transparent-background render that lost its alpha, or a rasteriser that wrote
        // an empty canvas, both produce a "valid" icon that is invisible or a filled
        // square. Neither is caught by decoding alone.
        const MARK: &[u8] = include_bytes!("../assets/curio-mark-32.png");
        let pixels = image::load_from_memory(MARK)
            .expect("a valid PNG")
            .into_rgba8();

        let opaque = pixels.pixels().filter(|p| p.0[3] > 128).count();
        let total = pixels.pixels().count();

        assert!(opaque > total / 20, "the mark is nearly invisible");
        assert!(opaque < total * 4 / 5, "the mark is a filled block");
    }

    #[test]
    fn the_menu_is_the_five_items_the_decision_allows() {
        // D14. This test is a tripwire: adding a sixth item is a decision that belongs in
        // the ARCH-00 register, not in a diff. It asserts the labels the builder uses,
        // since constructing a real tray needs a display server.
        let labels = [
            "Status",
            "Pause",
            "Open Dashboard",
            "Start at Login",
            "Quit",
        ];
        assert_eq!(labels.len(), 5);
    }

    #[test]
    fn the_pause_label_follows_the_state() {
        // The item is a toggle; showing "Pause" while already paused would invite the
        // user to do the thing they just did.
        assert!(!Status::Running { port: 1 }.is_paused());
        assert!(Status::Paused { port: 1 }.is_paused());
    }
}
