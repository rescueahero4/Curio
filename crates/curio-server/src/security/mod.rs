//! Who is asking, and what they may do.
//!
//! Middleware order for any guarded route is fixed (ARCH-06 §Middleware order):
//!
//! ```text
//! Host → Origin → (Sec-Fetch-Site) → soft-disable 503 → credential → handler
//! ```
//!
//! Identity before availability before credentials. That ordering is deliberate: a
//! rebinding attempt learns nothing at all — not even whether the app is paused — while a
//! legitimate paused client gets a clean 503 on a write without needing fresh credentials
//! first.
//!
//! [`origin`] and [`token`] hold the pure decisions; [`guard`] wires them into the axum
//! stack in that order, and [`session`] owns the cookie the exchange mints. The route-group
//! table that says which layer applies where lives in ARCH-01 §Middleware map.

pub mod guard;
pub mod origin;
pub mod session;
pub mod token;

pub use guard::{Access, authenticate, authenticate_quit, identity, refuse_while_paused};
pub use origin::{
    EXTENSION_ORIGIN, host_is_loopback, origin_is_allowed, sec_fetch_site_is_hostile,
};
pub use token::{NonceStore, RuntimeToken, secrets_match};
