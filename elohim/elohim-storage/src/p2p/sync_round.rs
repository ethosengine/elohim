//! Sync-round planning — the production decision surface for what one sync round
//! asks of each connected peer, and what a locally-authored change pushes.
//!
//! **Why this module exists.** The round's decisions used to live inline in
//! `P2PNode::initiate_sync_round` / `handle_sync_response`, reachable only with a
//! live swarm. That made the plane's *cost* untestable: the one integration proof
//! we had (`tests/sync_libp2p_convergence.rs`) re-implements the round in the test
//! file, so any assertion written there measures the mirror, not the wire. Pulling
//! the decisions out as pure functions lets a test assert the shape of what
//! production actually sends. See spine node `sync-scale-honesty`
//! (`genesis/manifests/spine.yaml`) and `tests/sync_scale_honesty.rs`.
//!
//! **Known-unsatisfied invariant (standing red).** The plane is poll-only and its
//! steady-state cost is proportional to corpus size, not to the number of changes:
//!
//! - [`round_opener`] ignores `local` — it opens every round with an unconditional
//!   full-corpus enumeration, so a converged peer-pair still exchanges the entire
//!   document list every tick (`p2p/mod.rs`, hardcoded 60s).
//! - [`announcements_for_local_change`] returns nothing — `SyncRequest::AnnounceChange`
//!   is defined and handled by both transports, but no send site exists, so the poll
//!   is the only propagation path.
//!
//! Both are honest reproductions of today's behavior, not stubs to be "filled in"
//! independently of the wire. **Binding requirement for whoever cures this:** these
//! functions must stay the ONLY place the round opener and the announce requests are
//! constructed. Making `announcements_for_local_change` return requests without a
//! caller that sends them turns the test green while nothing propagates — the exact
//! fake-green the extraction is here to prevent.

use crate::p2p::sync_protocol::SyncRequest;
use libp2p::request_response::OutboundFailure;
use libp2p::PeerId;
use std::time::Duration;

/// Fallback round cadence when none is configured (or a zero is configured).
/// Matches the historical hardcoded value so threading the config through is not
/// itself a behavior change.
pub const DEFAULT_ROUND_INTERVAL: Duration = Duration::from_secs(60);

/// The closed set of outbound-failure label values. Prometheus label values must
/// come from a bounded set — peer ids and error strings are cardinality bombs.
pub const OUTBOUND_FAILURE_LABELS: [&str; 5] = [
    "timeout",
    "connection_closed",
    "dial_failure",
    "unsupported_protocols",
    "io",
];

/// Resolve the sync round cadence from config.
///
/// `None` (unset) and `Some(0)` both fall back to [`DEFAULT_ROUND_INTERVAL`].
/// Zero is NOT "disabled" here, deliberately diverging from the archetype-cadence
/// convention elsewhere in `P2PConfig` (`inventory_broadcast_seconds: Some(0)`
/// disables broadcasting): those are supplementary broadcasts, whereas the sync
/// round is currently the ONLY propagation path the plane has — silently disabling
/// it would strand every peer with no error. A zero would also panic
/// `tokio::time::interval` ("interval period must be non-zero") and take the node
/// down at boot.
pub fn round_interval(secs: Option<u64>) -> Duration {
    match secs {
        Some(s) if s > 0 => Duration::from_secs(s),
        _ => DEFAULT_ROUND_INTERVAL,
    }
}

/// Map a libp2p outbound failure to its metric label, reusing the vocabulary
/// `elohim_view_federation_outbound_total` already publishes so both planes read
/// the same way. `Io` deliberately drops the inner error (unbounded string).
pub fn outbound_failure_label(err: &OutboundFailure) -> &'static str {
    match err {
        OutboundFailure::Timeout => "timeout",
        OutboundFailure::ConnectionClosed => "connection_closed",
        OutboundFailure::DialFailure => "dial_failure",
        OutboundFailure::UnsupportedProtocols => "unsupported_protocols",
        OutboundFailure::Io(_) => "io",
    }
}

/// Page size for `ListDocuments` sync-round enumeration — shared by the round
/// opener (page 0) and the `DocumentList` follow-up (next pages), so the cursor
/// arithmetic and the request size can't drift.
pub const SYNC_LIST_PAGE_LIMIT: u32 = 1000;

/// One document's identity and current heads in the local DocStore.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocHead {
    pub doc_id: String,
    pub heads: Vec<String>,
}

/// What this node already holds in a sync namespace — the state a round opener
/// should be a function of, so a peer can answer with the difference instead of
/// its whole corpus.
///
/// Constructed from `SyncManager::list_documents` against the LOCAL store (no
/// network). Today [`round_opener`] ignores it; that is the standing red.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct LocalCorpusState {
    pub docs: Vec<DocHead>,
}

impl LocalCorpusState {
    /// Number of documents held locally in this namespace.
    pub fn len(&self) -> usize {
        self.docs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.docs.is_empty()
    }
}

/// A stable fingerprint of the whole local corpus for one namespace: the sorted
/// set of `(doc_id, sorted heads)`, hashed.
///
/// Both sides sort before hashing, so two peers holding the same documents at
/// the same heads produce the same digest regardless of the order their stores
/// happen to enumerate in — the property the whole `InSync` shortcut rests on.
/// An unsorted digest would differ spuriously between peers and the shortcut
/// would simply never fire, silently degrading to today's full enumeration.
pub fn corpus_digest(local: &LocalCorpusState) -> String {
    use sha2::{Digest, Sha256};
    let mut entries: Vec<String> = local
        .docs
        .iter()
        .map(|d| {
            let mut heads = d.heads.clone();
            heads.sort();
            format!("{}={}", d.doc_id, heads.join(","))
        })
        .collect();
    entries.sort();
    let mut h = Sha256::new();
    for e in &entries {
        h.update(e.as_bytes());
        // Length-delimit so ("ab","c") and ("a","bc") cannot collide.
        h.update(b"\n");
    }
    format!("sha256:{:x}", h.finalize())
}

/// The request that opens a sync round with one peer.
///
/// Today: an unconditional `ListDocuments` from offset 0 — identical whether we
/// hold the entire corpus or nothing at all, which is why the peer must enumerate
/// everything back to us on every tick. `local` is accepted (and ignored) so the
/// cure is a body change here rather than a signature change rippling through
/// every call site.
pub fn round_opener(h_app_id: &str, _local: &LocalCorpusState) -> SyncRequest {
    SyncRequest::ListDocuments {
        h_app_id: h_app_id.to_string(),
        prefix: None,
        offset: 0,
        limit: SYNC_LIST_PAGE_LIMIT,
    }
}

/// The push notifications a locally-authored change owes each connected peer.
///
/// Today: empty — there is no announce send site anywhere in the tree, so a local
/// change waits for each peer's next poll (up to the full round interval) before it
/// can propagate. `AnnounceChange` is already carried by the protocol and handled
/// by both the libp2p and iroh backends; only the producing half is missing.
pub fn announcements_for_local_change(
    _h_app_id: &str,
    _doc_id: &str,
    _change_hash: &str,
    _peers: &[PeerId],
) -> Vec<(PeerId, SyncRequest)> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_interval_uses_the_configured_cadence() {
        assert_eq!(round_interval(Some(15)), Duration::from_secs(15));
    }

    #[test]
    fn round_interval_falls_back_to_the_default_when_unset() {
        assert_eq!(round_interval(None), DEFAULT_ROUND_INTERVAL);
    }

    #[test]
    fn round_interval_refuses_zero_because_tokio_interval_panics_on_it() {
        // `tokio::time::interval(Duration::ZERO)` panics with "interval period
        // must be non-zero" — a config typo would take the node down at boot.
        assert_eq!(round_interval(Some(0)), DEFAULT_ROUND_INTERVAL);
    }

    #[test]
    fn outbound_failure_labels_reuse_the_view_federation_vocabulary() {
        use libp2p::request_response::OutboundFailure;
        assert_eq!(outbound_failure_label(&OutboundFailure::Timeout), "timeout");
        assert_eq!(
            outbound_failure_label(&OutboundFailure::ConnectionClosed),
            "connection_closed"
        );
        assert_eq!(
            outbound_failure_label(&OutboundFailure::DialFailure),
            "dial_failure"
        );
        assert_eq!(
            outbound_failure_label(&OutboundFailure::UnsupportedProtocols),
            "unsupported_protocols"
        );
        assert_eq!(
            outbound_failure_label(&OutboundFailure::Io(std::io::Error::other("boom"))),
            "io"
        );
    }

    #[test]
    fn outbound_failure_labels_are_bounded_so_the_metric_cannot_explode() {
        // Prometheus label values must come from a closed set — an unbounded
        // label (peer id, error string) is a cardinality bomb on a large mesh.
        use libp2p::request_response::OutboundFailure;
        let all = [
            outbound_failure_label(&OutboundFailure::Timeout),
            outbound_failure_label(&OutboundFailure::ConnectionClosed),
            outbound_failure_label(&OutboundFailure::DialFailure),
            outbound_failure_label(&OutboundFailure::UnsupportedProtocols),
            outbound_failure_label(&OutboundFailure::Io(std::io::Error::other("boom"))),
        ];
        for label in all {
            assert!(
                OUTBOUND_FAILURE_LABELS.contains(&label),
                "{label} is not in the declared closed set"
            );
        }
    }
}
