//! Identifiers: monotonic ULIDs.
//!
//! ULIDs sort lexicographically by creation time, and Curio leans on that in two places
//! where it is load-bearing rather than convenient (R-DA-5, Inventory §10.9):
//!
//! * **Keyset pagination.** The library grid pages on `(created_at, id)` rather than an
//!   offset, so a capture arriving mid-scroll cannot shift the page under the reader.
//! * **FIFO job claims.** The worker takes the oldest queued job and ties break on id.
//!
//! Both break if two ids minted in the same millisecond can sort the wrong way, which is
//! why generation goes through a *monotonic* generator: within a millisecond it increments
//! the random component instead of redrawing it, so later always sorts later.
//!
//! Use [`IdGenerator`] rather than `Ulid::new()` anywhere an id will be stored.

use std::sync::Mutex;

use ulid::{Generator, Ulid};

/// A process-wide source of monotonic ULIDs.
///
/// Cheap to share: generation is a lock and an increment. One instance per process is the
/// intent — two generators cannot coordinate, and the ordering guarantee only holds
/// within one.
#[derive(Debug)]
pub struct IdGenerator {
    inner: Mutex<Generator>,
}

impl IdGenerator {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Generator::new()),
        }
    }

    /// Mint the next id.
    ///
    /// The generator refuses only when a single millisecond's random space is exhausted —
    /// which needs 2^80 ids in one millisecond. If it somehow happens, falling back to a
    /// fresh random ULID keeps the app running and costs, at worst, a tie-break between
    /// two ids minted in the same millisecond.
    pub fn next(&self) -> Ulid {
        let mut generator = self.inner.lock().unwrap_or_else(|poisoned| {
            // A panic in another thread while holding this lock leaves it poisoned. The
            // guarded state is a counter, not an invariant that can be half-updated, so
            // recovering is strictly better than propagating the panic into id minting.
            poisoned.into_inner()
        });
        generator.generate().unwrap_or_else(|_| Ulid::generate())
    }

    /// Mint the next id as the canonical 26-character string stored in the database.
    pub fn next_string(&self) -> String {
        self.next().to_string()
    }
}

impl Default for IdGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Mint an id from the process-wide generator.
///
/// The monotonic guarantee holds **within one generator**, so there has to be exactly one
/// (R-DA-5). Handing every call site its own [`IdGenerator`] would give each a private
/// counter, and two ids minted in the same millisecond from different ones could sort the
/// wrong way — breaking the FIFO job claim and the keyset cursor in a way that only shows
/// up under load.
pub fn generate() -> String {
    static GENERATOR: std::sync::OnceLock<IdGenerator> = std::sync::OnceLock::new();
    GENERATOR.get_or_init(IdGenerator::new).next_string()
}

/// Parse a stored id, rejecting anything that isn't a ULID.
///
/// # Errors
/// Returns [`crate::Error::Invalid`] if `raw` is not a valid ULID.
pub fn parse(raw: &str) -> crate::Result<Ulid> {
    Ulid::from_string(raw).map_err(|_| crate::Error::invalid(format!("not a valid id: {raw}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_from_one_generator_always_ascend() {
        // The property keyset pagination and FIFO job claims both rest on (R-DA-5). A
        // thousand ids minted as fast as possible land well inside a single millisecond,
        // which is exactly the case a non-monotonic generator gets wrong.
        let generator = IdGenerator::new();
        let ids: Vec<String> = (0..1000).map(|_| generator.next_string()).collect();

        let mut sorted = ids.clone();
        sorted.sort();

        assert_eq!(ids, sorted, "ids must sort in the order they were minted");
    }

    #[test]
    fn ids_are_unique() {
        let generator = IdGenerator::new();
        let ids: std::collections::HashSet<String> =
            (0..1000).map(|_| generator.next_string()).collect();

        assert_eq!(ids.len(), 1000);
    }

    #[test]
    fn ids_are_the_canonical_26_characters() {
        // The database column and every sidecar reference this exact form.
        assert_eq!(IdGenerator::new().next_string().len(), 26);
    }

    #[test]
    fn parse_rejects_non_ulids() {
        assert!(parse("not-an-id").is_err());
        assert!(parse(&IdGenerator::new().next_string()).is_ok());
    }
}
