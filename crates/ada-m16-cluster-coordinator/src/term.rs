//! Election term counter.
//!
//! In Raft / Raft-style protocols, the term is a monotonically
//! increasing logical clock. Each new election increments the term;
//! a node that sees a higher term from a peer immediately steps
//! down to [`NodeState::Follower`](crate::node::NodeState::Follower).
//!
//! [`Term`] is a `u64` newtype to keep the math and comparisons
//! explicit and to leave room for the `Display` and
//! [`next`](Self::next) helper that the v0.1.0 election state machine
//! uses.

use core::fmt;

/// Monotonic election term (Raft §5.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Term(pub u64);

impl Term {
    /// Initial term (term 0). A fresh node starts here, before the
    /// first election.
    pub const ZERO: Self = Self(0);

    /// Construct a term from a raw `u64`.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Return the next term (`self + 1`).
    ///
    /// Saturates at `u64::MAX` to avoid panics in production. A
    /// production cluster that hits the cap has been running for
    /// longer than the age of the universe and needs to be replaced.
    #[must_use]
    pub const fn next(self) -> Self {
        Self(self.0.saturating_add(1))
    }

    /// Raw `u64` value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for Term {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "term({})", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn next_increments_by_one() {
        assert_eq!(Term::ZERO.next(), Term(1));
        assert_eq!(Term(7).next(), Term(8));
    }

    #[test]
    fn next_saturates_at_max() {
        let t = Term(u64::MAX);
        assert_eq!(t.next(), Term(u64::MAX));
    }

    #[test]
    fn ordering() {
        assert!(Term(2) > Term(1));
        assert!(Term(0) < Term(1));
    }

    #[test]
    fn display() {
        assert_eq!(Term(0).to_string(), "term(0)");
        assert_eq!(Term(42).to_string(), "term(42)");
    }
}
