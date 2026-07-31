//! What Curio asks the model, and what it does with the answer.
//!
//! Everything here is pure: request bodies in, parsed answers out, no sockets. The HTTP
//! call itself belongs to `curio-server`, which owns transports (R-DEL-2). The split is
//! not ceremony — it is what makes the two things most likely to regress silently
//! testable without a network:
//!
//! * the **two cache breakpoints** on the vision call, and the fact that nothing volatile
//!   sits above them ([`prompt`]);
//! * the **property order** of the dedupe schema, which a `json!` literal would sort away
//!   ([`schema`]).
//!
//! [`policy`] is the third: whether a failure charges an attempt is the difference between
//! a queue that drains when a user adds their API key and a pile of failures they have to
//! notice.

pub mod dedupe;
pub mod policy;
pub mod prompt;
pub mod schema;
pub mod wire;

pub use policy::{Recovery, recover};
pub use prompt::{Context, Image, Vocabulary};
pub use wire::MessagesRequest;
