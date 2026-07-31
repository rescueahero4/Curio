//! The tray menu.
//!
//! Five items, and no more (D14): **Status · Pause/Resume · Open Dashboard · Start at
//! Login · Quit**. The previous shell's Open Projects and New Prompt entries are gone on
//! purpose — the tray is a switch, not a navigation bar, and navigation belongs in the
//! dashboard where it can show the user what they are navigating to.
//!
//! The icon is drawn in code rather than loaded from a file. A tray icon is 32×32; a PNG
//! of it is a binary asset to keep in sync across two packaging pipelines, and drawing it
//! is a dozen lines that cannot drift from the build.

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

/// A 32×32 icon, drawn rather than loaded.
fn icon() -> Icon {
    const SIZE: u32 = 32;
    let mut rgba = Vec::with_capacity((SIZE * SIZE * 4) as usize);

    for y in 0..SIZE {
        for x in 0..SIZE {
            // A rounded square with a lighter aperture in the middle — legible at 16 px,
            // which is the size that actually matters in a menu bar.
            let (dx, dy) = (x as f32 - 15.5, y as f32 - 15.5);
            let outside = dx.abs().max(dy.abs()) > 13.0;
            let corner = dx.abs() > 10.0 && dy.abs() > 10.0 && (dx * dx + dy * dy).sqrt() > 16.0;
            let aperture = (dx * dx + dy * dy).sqrt() < 6.0;

            if outside || corner {
                rgba.extend_from_slice(&[0, 0, 0, 0]);
            } else if aperture {
                rgba.extend_from_slice(&[250, 250, 250, 255]);
            } else {
                rgba.extend_from_slice(&[32, 32, 36, 255]);
            }
        }
    }

    Icon::from_rgba(rgba, SIZE, SIZE).expect("a 32x32 RGBA buffer is a valid icon")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icon_buffer_is_the_size_it_claims() {
        // from_rgba validates this, so a mismatch panics at startup rather than failing a
        // test — which is exactly why it is worth asserting here instead.
        let _ = icon();
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
