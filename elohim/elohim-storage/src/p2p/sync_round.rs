//! Sync-round planning — the production decision surface for what one sync round
//! asks of each connected peer, and what a locally-authored change pushes.
//!
//! **Why this module exists.** The round's decisions used to live inline in
//! `P2PNode::initiate_sync_round` / `handle_sync_response`, reachable only with a
//! live swarm. That made the plane's *cost* untestable: the one integration proof
//! we had (`tests/sync_libp2p_convergence.rs`) re-implements the round in the test
//! file, so any assertion written there measures the mirror, not the wire. Pulling
//! the decisions out as pure functions lets a test assert the shape of what
//! production actually sends. See habit `sync-scale-honesty`
//! (`genesis/manifests/habits.yaml`) and `tests/sync_scale_honesty.rs`.
//!
//! **Cured 2026-07-27 (was a standing red).** The plane was poll-only with cost
//! proportional to corpus size, not to the number of changes:
//!
//! - [`round_opener`] now carries [`corpus_digest`] of `local`, so a converged
//!   peer-pair exchanges one hash (`InSync`) instead of the entire document list
//!   every tick (`p2p/mod.rs`, hardcoded 60s). Divergent peers fall through to
//!   exactly the previous `ListDocuments` enumeration path.
//! - [`announcements_for_local_change_with_data`] is sent from the
//!   `AnnounceLocalChange` command arm in `p2p/mod.rs` on a local change, so the
//!   poll is no longer the only propagation path.
//!
//! **The doorbell delivers (2026-08-23).** The announce used to be metadata-only
//! on the send side AND a bare ack on the receive side, so nothing moved until
//! the next 60s round (measured cross-peer arrival on the live 3-peer mesh:
//! ~24s, a pull-round profile). Now the announce carries the change bytes when
//! they fit ([`MAX_ANNOUNCE_PAYLOAD_BYTES`], via [`bounded_announce_payload`]),
//! and a metadata-only announce makes the RECEIVER open a `SyncChanges` pull for
//! that one doc. Both arms end in the same `SyncManager::apply_changes` the round
//! uses. The doorbell stays bounded and lossy by design — no retries, no queues —
//! and the 60s round remains the reconciliation backstop.
//!
//! **Binding requirement for whoever touches this next:** these functions must
//! stay the ONLY place the round opener and the announce requests are
//! constructed. Making either return requests without a caller that sends them
//! turns a test green while nothing propagates — the exact fake-green the
//! extraction is here to prevent. `tests/sync_libp2p_convergence.rs` must call
//! [`round_opener`] rather than hand-rolling a second construction site, or it
//! measures its own mirror instead of the wire.
//!
//! # Gate growth, never convergence.
//!
//! When a budget or admission gate lands on the storage plane, the HEAL path
//! must be exempt from it. Freenet #4868: an over-budget peer permanently
//! diverges because the growth UPDATE *and* its ResyncResponse heal hit the same
//! admission gate, so no convergence path exists while over budget. A node that
//! cannot heal is worse than a node that is over budget.

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
    let entries: Vec<String> = local
        .docs
        .iter()
        .map(|d| {
            let mut heads = d.heads.clone();
            heads.sort();
            format!("{}={}", d.doc_id, heads.join(","))
        })
        .collect();
    crate::p2p::reconcile_rails::digest_of_entry_lines(entries)
}

/// The single request that opens a sync round with one peer.
///
/// Carries a digest of what we already hold, so a converged peer answers
/// `InSync` with one hash instead of enumerating its whole corpus. Divergent
/// peers fall through to exactly the previous `ListDocuments` path, so
/// correctness in the divergent case is unchanged — only the CONVERGED steady
/// state gets cheap, which is where a healthy mesh spends nearly all its time.
///
/// `limit` is the page size the responder uses IF the digests differ; it must
/// stay `SYNC_LIST_PAGE_LIMIT` so the fallback paginates exactly as before.
pub fn round_opener(h_app_id: &str, local: &LocalCorpusState) -> SyncRequest {
    SyncRequest::ListDocumentsSince {
        h_app_id: h_app_id.to_string(),
        prefix: None,
        corpus_digest: corpus_digest(local),
        limit: SYNC_LIST_PAGE_LIMIT,
    }
}

/// The largest change payload an announce will carry inline, per peer.
///
/// The doorbell fans out to EVERY connected peer, so the cost of an eager
/// payload is `bytes x peers` — the bound is what keeps that fan-out from
/// becoming a mesh-wide amplifier. 64 KiB comfortably covers a projected
/// content doc (a flat field set plus a body), which is the shape the
/// content-sync producer authors; anything larger degrades to the
/// metadata-only doorbell and the receiver pulls the bytes itself through the
/// existing `SyncChanges`/`Changes` path.
///
/// Wire-visible in one direction only: a receiver never *requires* a payload,
/// so lowering or raising this bound is compatible in both directions.
pub const MAX_ANNOUNCE_PAYLOAD_BYTES: usize = 64 * 1024;

/// Decide whether a freshly-encoded change rides the doorbell or stays behind it.
///
/// `changes` is exactly what `SyncManager::get_changes_since` produces (0 or 1
/// blob today). `Some(bytes)` means the announce delivers; `None` means the
/// announce is metadata-only and the receiver pulls. `None` is ALWAYS safe —
/// the 60s round and the receive-side pull both still converge — so every
/// uncertain case (empty, oversized, multi-chunk) resolves to `None`.
pub fn bounded_announce_payload(changes: Vec<Vec<u8>>) -> Option<Vec<u8>> {
    let mut it = changes.into_iter();
    let first = it.next()?;
    if it.next().is_some() {
        // Multi-chunk: `change_data` is a single blob and the receiver applies
        // it with one `load_incremental`. Never guess at concatenation here.
        return None;
    }
    if first.is_empty() || first.len() > MAX_ANNOUNCE_PAYLOAD_BYTES {
        return None;
    }
    Some(first)
}

/// The push notifications a locally-authored change owes each connected peer,
/// **metadata only** (`change_data: None`).
///
/// The receiving peer pulls the bytes through the existing
/// `SyncChanges`/`Changes` path. This is the shape an oversized change takes
/// (see [`MAX_ANNOUNCE_PAYLOAD_BYTES`]), and the shape a caller with no bytes
/// in hand takes.
pub fn announcements_for_local_change(
    h_app_id: &str,
    doc_id: &str,
    change_hash: &str,
    peers: &[PeerId],
) -> Vec<(PeerId, SyncRequest)> {
    announcements_for_local_change_with_data(h_app_id, doc_id, change_hash, peers, None)
}

/// The same push notifications, carrying the change bytes when they fit.
///
/// `change_data: Some(bytes)` makes the doorbell a delivery: the receiver
/// applies the bytes through the SAME `SyncManager::apply_changes` path a
/// pulled change takes (no weaker validation), so a connected peer converges in
/// one round-trip instead of waiting up to a full 60s round. `None` keeps the
/// old metadata-only behaviour byte-for-byte, so an oversized change still
/// propagates — just by pull rather than push.
///
/// This is the sole constructor of the announce requests, deliberately: the
/// planner staying the only constructor is what keeps `tests/sync_scale_honesty`
/// measuring the wire instead of a test-local mirror.
pub fn announcements_for_local_change_with_data(
    h_app_id: &str,
    doc_id: &str,
    change_hash: &str,
    peers: &[PeerId],
    change_data: Option<Vec<u8>>,
) -> Vec<(PeerId, SyncRequest)> {
    peers
        .iter()
        .map(|p| {
            (
                *p,
                SyncRequest::AnnounceChange {
                    h_app_id: h_app_id.to_string(),
                    doc_id: doc_id.to_string(),
                    change_hash: change_hash.to_string(),
                    change_data: change_data.clone(),
                },
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Byte-pin, captured from the PRE-REFACTOR `corpus_digest` body**
    /// (inline `sha2::Sha256` fold, before `digest_of_entry_lines` was
    /// extracted to `reconcile_rails.rs`). Value obtained by running this
    /// exact fixture through the unmodified function on 2026-08-08:
    /// `sha256:e1c5d7414b6d236e0ffb76b98739b1045f1c0a925a0a9e58d27788eec245cefe`
    /// (verified independently via `printf 'alpha=z9\nbeta=h1,h2\n' | sha256sum`,
    /// matching the entry-line/sort/newline-delimit shape byte-for-byte).
    ///
    /// This is wire-visible: `corpus_digest` is what `round_opener` puts on
    /// `SyncRequest::ListDocumentsSince`, and the `InSync` shortcut fires only
    /// when both peers compute the SAME digest for the SAME corpus. A single
    /// silently-changed byte in the fold (sort order, delimiter, hex casing)
    /// would make every peer pair permanently miss the shortcut and fall back
    /// to full enumeration — a fleet-wide regression invisible to any test
    /// that only checks "digests are stable" without pinning the value itself.
    #[test]
    fn corpus_digest_matches_the_pre_refactor_pinned_value() {
        let local = LocalCorpusState {
            docs: vec![
                DocHead {
                    doc_id: "beta".into(),
                    heads: vec!["h2".into(), "h1".into()],
                },
                DocHead {
                    doc_id: "alpha".into(),
                    heads: vec!["z9".into()],
                },
            ],
        };
        assert_eq!(
            corpus_digest(&local),
            "sha256:e1c5d7414b6d236e0ffb76b98739b1045f1c0a925a0a9e58d27788eec245cefe",
            "corpus_digest must reproduce the pre-refactor byte-for-byte value; \
             a mismatch here means the InSync shortcut just broke fleet-wide"
        );
    }

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

    /// The doorbell must DELIVER when the change fits. An announce whose
    /// `change_data` is `None` for a small change is the inert shape the cure
    /// removed: the receiver acks it and nothing moves until the next 60s round.
    #[test]
    fn a_change_within_the_bound_rides_the_announce() {
        let change = vec![7u8; 1024];
        let payload = bounded_announce_payload(vec![change.clone()])
            .expect("a 1 KiB change is well inside the bound");
        assert_eq!(payload, change, "the payload must be the change verbatim");

        let peers: Vec<PeerId> = vec![PeerId::random(), PeerId::random()];
        let announcements = announcements_for_local_change_with_data(
            "elohim",
            "node:concept-1",
            "change-hash-abc",
            &peers,
            Some(payload.clone()),
        );
        assert_eq!(announcements.len(), peers.len());
        for (_peer, req) in &announcements {
            match req {
                SyncRequest::AnnounceChange { change_data, .. } => assert_eq!(
                    change_data.as_ref(),
                    Some(&payload),
                    "every connected peer must receive the bytes, not just a doorbell"
                ),
                other => panic!("expected AnnounceChange, got {other:?}"),
            }
        }
    }

    /// Above the bound the announce degrades to a doorbell — the fan-out cost is
    /// `bytes x peers`, so an unbounded payload is a mesh-wide amplifier. The
    /// receiver pulls instead; propagation is preserved, amplification is not.
    #[test]
    fn a_change_over_the_bound_stays_a_doorbell() {
        let too_big = vec![0u8; MAX_ANNOUNCE_PAYLOAD_BYTES + 1];
        assert!(
            bounded_announce_payload(vec![too_big]).is_none(),
            "an oversized change must not be fanned out inline"
        );
        // Exactly at the bound is still a delivery (inclusive bound).
        assert!(
            bounded_announce_payload(vec![vec![0u8; MAX_ANNOUNCE_PAYLOAD_BYTES]]).is_some(),
            "the bound is inclusive"
        );
    }

    /// Every uncertain input resolves to the doorbell, never to a guess: nothing
    /// to send, and a multi-chunk change (`change_data` is ONE blob applied with
    /// one `load_incremental` — concatenation would be a guess about the wire).
    #[test]
    fn nothing_and_multi_chunk_resolve_to_the_doorbell() {
        assert!(bounded_announce_payload(vec![]).is_none());
        assert!(bounded_announce_payload(vec![vec![]]).is_none());
        assert!(bounded_announce_payload(vec![vec![1u8; 8], vec![2u8; 8]]).is_none());
    }

    /// The metadata-only constructor stays byte-identical to the pre-cure wire —
    /// an old peer that only acks a doorbell still sees exactly what it saw.
    #[test]
    fn the_metadata_only_constructor_is_unchanged() {
        let peers: Vec<PeerId> = vec![PeerId::random()];
        let announcements =
            announcements_for_local_change("elohim", "node:concept-1", "change-hash-abc", &peers);
        assert_eq!(announcements.len(), 1);
        match &announcements[0].1 {
            SyncRequest::AnnounceChange { change_data, .. } => {
                assert!(change_data.is_none(), "the doorbell carries no bytes")
            }
            other => panic!("expected AnnounceChange, got {other:?}"),
        }
    }
}
