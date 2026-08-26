//! Error surface for the cluster coordinator.
//!
//! [`CoordError`] is the single error type returned by all public
//! functions in this crate. Production builds will map the network
//! variants onto richer diagnostics; for v0.1.0 we keep the enum
//! minimal (5 variants) per the B2 brief.

use thiserror::Error;

/// Failure modes surfaced by the cluster coordinator.
#[derive(Debug, Error)]
pub enum CoordError {
    /// The local node is not the leader and cannot service the
    /// request (callers should retry against the actual leader).
    #[error("not leader (current term = {term})")]
    NotLeader {
        /// The current election term observed locally.
        term: u64,
    },

    /// A quorum could not be assembled for the requested operation
    /// (election vote, lease renewal, ...).
    #[error("no quorum (have {have} / need {need})")]
    NoQuorum {
        /// Number of votes / responses actually collected.
        have: usize,
        /// Number required for a majority.
        need: usize,
    },

    /// A cluster operation timed out.
    #[error("cluster operation timed out after {0} ms")]
    Timeout(u64),

    /// A network-level failure (peer unreachable, malformed reply,
    /// ...). The wrapped string is a short, log-friendly description.
    #[error("network error: {0}")]
    Network(String),

    /// Catch-all for invariant violations / unexpected failures.
    #[error("internal error: {0}")]
    Internal(String),
}

/// `Result` alias for fallible coordinator operations.
pub type Result<T> = core::result::Result<T, CoordError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_leader_display() {
        let e = CoordError::NotLeader { term: 3 };
        assert_eq!(e.to_string(), "not leader (current term = 3)");
    }

    #[test]
    fn no_quorum_display() {
        let e = CoordError::NoQuorum { have: 1, need: 2 };
        assert_eq!(e.to_string(), "no quorum (have 1 / need 2)");
    }

    #[test]
    fn timeout_display() {
        let e = CoordError::Timeout(1500);
        assert_eq!(e.to_string(), "cluster operation timed out after 1500 ms");
    }

    #[test]
    fn network_display() {
        let e = CoordError::Network("peer reset".into());
        assert_eq!(e.to_string(), "network error: peer reset");
    }

    #[test]
    fn internal_display() {
        let e = CoordError::Internal("invariant broken".into());
        assert_eq!(e.to_string(), "internal error: invariant broken");
    }

    #[test]
    fn result_alias_carries_error() {
        let ok: Result<i32> = Ok(7);
        let err: Result<i32> = Err(CoordError::NoQuorum { have: 1, need: 3 });
        assert!(matches!(ok, Ok(7)));
        match err {
            Err(CoordError::NoQuorum { have, need }) => {
                assert_eq!(have, 1);
                assert_eq!(need, 3);
            }
            other => panic!("expected NoQuorum, got {other:?}"),
        }
    }
}
