//! Back-propagation service — Phase 3.5 Trust-Compute Gradient (Primitive 2).
//!
//! Implements hop-by-hop trust-bubble back-propagation via the sealed predecessor
//! map. When a peer forwards content, it records the sender (its immediate
//! predecessor) in the local `predecessor_records` table, sealed under the 2-of-2
//! scheme. When a [`FeedbackSignal`] arrives for that content, the service unseals
//! the predecessor record(s) and forwards the signal one hop upstream.
//!
//! ## Per-peer privacy
//!
//! Each peer knows only its immediate predecessor — the peer that directly sent
//! it the content. There is no propagation chain embedded in any wire message.
//! The chain is reconstructed through local memory at every hop: each node
//! unseals its own predecessor and forwards; the signal reaches the original
//! author by walking backward through the live peer graph. A peer that is offline
//! or out-of-relationship breaks the chain naturally (humane bounded walk,
//! brainstorm §5.2).
//!
//! ## Fan-out semantic (§5.2)
//!
//! When content has been received from multiple predecessors (different gossip
//! waves), `back_prop_one_hop` sends the signal to ALL of them. The trust-bubble
//! walk fans out at each hop; this ensures every upstream relay in every wave
//! receives the signal, not just the earliest or most recent one.
//!
//! ## Sealed payload
//!
//! The predecessor PeerId is sealed under the 2-of-2 nested scheme from T11
//! (`sealed_against_self`). Sealing requires both public keys (mishpat quorum +
//! imagodei). Unsealing requires both key-pairs. The sealed bytes stored in
//! `predecessor_records.sealed_blob` are the `SealedBlob` bincode/msgpack
//! serialisation — stored as raw `SealedBlob` bytes via `rmp_serde`.
//!
//! ## api/epr.rs wiring
//!
//! The `record_predecessor` call site belongs on the EPR ingest path (PUT
//! /api/v1/epr/:cid) — after a successful `store.put` — to register the sender
//! peer. The `back_prop_one_hop` call site belongs wherever FeedbackSignal EPRs
//! are ingested (a new handler, deferred until the FeedbackSignal ingest route
//! lands). Both wiring points are marked with `TODO(T19)` stubs in `api/epr.rs`.
//!
//! See also: `db/predecessor_records.rs` (T10), `services/sealed_against_self.rs` (T11).

use chrono::Utc;
use diesel::SqliteConnection;
use serde::{Deserialize, Serialize};

use crate::db::predecessor_records::{insert_predecessor, list_predecessors_for_cid};
use crate::error::StorageError;
use crate::p2p::feedback_signal::FeedbackSignal;
use crate::services::sealed_against_self::{
    seal, unseal, ImagodeiPubKey, ImagodeiSecretKey, MishpatQuorumPubKey, MishpatQuorumSecretKey,
    SealedBlob,
};

// ---------------------------------------------------------------------------
// Outbound sink abstraction (plan §"Plan deviations you may need")
//
// P2PHandle is async and tokio-bound; back_prop runs synchronously from service
// layer. Rather than taking a concrete `P2PHandle`, we abstract via `OutboundSink`
// so tests can inject a `MockSink` and production can wire a thin adapter.
// ---------------------------------------------------------------------------

/// Errors produced by [`OutboundSink::send`].
#[derive(Debug, thiserror::Error)]
pub enum SinkError {
    /// The underlying transport could not deliver the payload.
    #[error("sink send failed: {0}")]
    Send(String),
}

/// Abstraction over the P2P outbound transport for back-propagation.
///
/// Production wires a `TokioChannelSink` (or equivalent) that encodes the
/// [`FeedbackSignal`] and dispatches via `P2PCommand`. Tests inject a
/// `MockSink` that records calls without a live swarm.
pub trait OutboundSink: Send + Sync {
    /// Send an encoded [`FeedbackSignal`] to the peer identified by
    /// `peer_id_str` (multibase-encoded libp2p PeerId string, e.g.
    /// `"12D3KooW..."`).
    ///
    /// Returns `Ok(())` on best-effort delivery (the transport accepted the
    /// payload). Callers should log `Err` but NOT abort — back-prop is
    /// best-effort; a sink failure must not prevent processing of other
    /// predecessors.
    fn send(&self, peer_id_str: &str, payload: Vec<u8>) -> Result<(), SinkError>;
}

// ---------------------------------------------------------------------------
// Sealing keys bundle — passed to record_predecessor (pub keys only)
// ---------------------------------------------------------------------------

/// Public keys required to seal a predecessor record.
///
/// Wraps the two T11 public-key newtypes for ergonomic passing to
/// [`record_predecessor`].
pub struct SealingPubKeys<'a> {
    pub mishpat_pk: &'a MishpatQuorumPubKey,
    pub imagodei_pk: &'a ImagodeiPubKey,
}

/// Both key-pairs required to unseal a predecessor record.
///
/// Used internally by [`read_predecessors`] and [`back_prop_one_hop`].
pub struct UnsealingKeys<'a> {
    pub mishpat_pk: &'a MishpatQuorumPubKey,
    pub mishpat_sk: &'a MishpatQuorumSecretKey,
    pub imagodei_pk: &'a ImagodeiPubKey,
    pub imagodei_sk: &'a ImagodeiSecretKey,
}

// ---------------------------------------------------------------------------
// Sealed payload wire format
// ---------------------------------------------------------------------------

/// The plaintext payload stored inside the sealed blob.
///
/// Currently only carries the predecessor PeerId string; room added for
/// future Phase 5 fields (e.g. received-from multiaddr, timestamp proof).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct PredecessorPayload {
    /// String representation of the predecessor's libp2p PeerId.
    peer_id: String,
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Record the immediate predecessor (the sender) for a content CID.
///
/// Should be called after a successful EPR ingest from a remote peer, passing
/// that peer's PeerId. The predecessor PeerId is sealed under the 2-of-2 scheme
/// and stored in `predecessor_records`.
///
/// Idempotent: if the same `(target_cid, predecessor_peer_id)` pair is already
/// recorded, the call is a no-op (T10 `on_conflict_do_nothing`).
///
/// # Arguments
///
/// * `conn` — SQLite connection
/// * `target_cid` — CIDv1 of the content EPR whose predecessor this is
/// * `predecessor_peer_id` — String PeerId of the peer that sent us the content
/// * `keys` — public keys for sealing (both keys required)
pub fn record_predecessor(
    conn: &mut SqliteConnection,
    target_cid: &str,
    predecessor_peer_id: &str,
    keys: &SealingPubKeys<'_>,
) -> Result<(), StorageError> {
    let payload = PredecessorPayload {
        peer_id: predecessor_peer_id.to_string(),
    };
    let plaintext = rmp_serde::to_vec_named(&payload)
        .map_err(|e| StorageError::Internal(format!("back_prop: serialize payload: {e}")))?;

    let sealed = seal(&plaintext, keys.mishpat_pk, keys.imagodei_pk)
        .map_err(|e| StorageError::Internal(format!("back_prop: seal predecessor: {e}")))?;

    let blob_bytes = rmp_serde::to_vec_named(&sealed)
        .map_err(|e| StorageError::Internal(format!("back_prop: serialize sealed blob: {e}")))?;

    let now = Utc::now().to_rfc3339();

    insert_predecessor(conn, target_cid, predecessor_peer_id, &now, blob_bytes)
        .map_err(StorageError::Diesel)?;

    Ok(())
}

/// Unseal and return all predecessor PeerId strings for a content CID.
///
/// Used internally by [`back_prop_one_hop`]; also callable directly in tests.
///
/// Returns the peer IDs in `received_at` ascending order (stable chronological
/// traversal, as established by T10's `list_predecessors_for_cid` ordering).
pub fn read_predecessors(
    conn: &mut SqliteConnection,
    target_cid: &str,
    keys: &UnsealingKeys<'_>,
) -> Result<Vec<String>, StorageError> {
    let rows = list_predecessors_for_cid(conn, target_cid).map_err(StorageError::Diesel)?;

    let mut peer_ids = Vec::with_capacity(rows.len());
    for row in rows {
        let sealed: SealedBlob = rmp_serde::from_slice(&row.sealed_blob).map_err(|e| {
            StorageError::Internal(format!(
                "back_prop: deserialize sealed blob for cid={target_cid}: {e}"
            ))
        })?;

        let plaintext = unseal(
            &sealed,
            keys.mishpat_pk,
            keys.mishpat_sk,
            keys.imagodei_pk,
            keys.imagodei_sk,
        )
        .map_err(|e| {
            StorageError::Internal(format!(
                "back_prop: unseal predecessor for cid={target_cid}: {e}"
            ))
        })?;

        let payload: PredecessorPayload = rmp_serde::from_slice(&plaintext).map_err(|e| {
            StorageError::Internal(format!(
                "back_prop: deserialize payload for cid={target_cid}: {e}"
            ))
        })?;

        peer_ids.push(payload.peer_id);
    }

    Ok(peer_ids)
}

/// Walk back one hop: for `signal.target_cid`, unseal all predecessors and
/// forward the signal to each.
///
/// ## Fan-out
///
/// If the content was received from multiple predecessors (different gossip
/// waves), the signal is forwarded to ALL of them. Trust-bubble walks fan out
/// at each hop; this matches brainstorm §5.2 "chain reconstructs through local
/// memory of every participant in propagation."
///
/// ## Best-effort
///
/// Per-peer sink errors are logged (tracing `warn!`) but do NOT abort the
/// loop — remaining predecessors still receive the signal. The function returns
/// `Ok(Vec<String>)` containing the peer IDs that were successfully targeted
/// (regardless of whether the sink delivered the bytes).
///
/// ## Returns
///
/// - `Ok(vec![])` — no predecessor recorded; this peer is the origin (or the
///   predecessor record has been deleted/expired). The sink is not called.
/// - `Ok(peer_ids)` — predecessor(s) found; signal forwarded to all of them.
///   `peer_ids` contains the string PeerIds that were attempted.
/// - `Err(...)` — DB or unseal failure (not a sink failure).
pub fn back_prop_one_hop(
    conn: &mut SqliteConnection,
    signal: &FeedbackSignal,
    keys: &UnsealingKeys<'_>,
    sink: &dyn OutboundSink,
) -> Result<Vec<String>, StorageError> {
    let peer_ids = read_predecessors(conn, &signal.target_cid, keys)?;

    if peer_ids.is_empty() {
        return Ok(vec![]);
    }

    // Encode the signal for wire transmission (MessagePack, matching the
    // FeedbackSignal wire contract in `p2p/feedback_signal.rs`).
    let payload_bytes = rmp_serde::to_vec_named(signal)
        .map_err(|e| StorageError::Internal(format!("back_prop: serialize signal: {e}")))?;

    let mut targeted = Vec::with_capacity(peer_ids.len());
    for peer_id in &peer_ids {
        match sink.send(peer_id, payload_bytes.clone()) {
            Ok(()) => {
                targeted.push(peer_id.clone());
            }
            Err(e) => {
                // Best-effort: log and continue to remaining predecessors.
                tracing::warn!(
                    target: "back_prop",
                    peer_id = %peer_id,
                    target_cid = %signal.target_cid,
                    error = %e,
                    "back_prop sink send failed — best-effort, continuing"
                );
                // Still include the peer in targeted so callers know we tried.
                targeted.push(peer_id.clone());
            }
        }
    }

    Ok(targeted)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use dryoc::classic::crypto_box::crypto_box_seed_keypair;
    use std::sync::Mutex;

    // -----------------------------------------------------------------------
    // Test pool fixture (mirrors T10's pattern)
    // -----------------------------------------------------------------------

    fn test_pool() -> DbPool {
        let url = format!(
            "file:back_prop_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    // -----------------------------------------------------------------------
    // Deterministic keypairs (mirrors T11's test pattern)
    // -----------------------------------------------------------------------

    fn mishpat_keypair() -> (MishpatQuorumPubKey, MishpatQuorumSecretKey) {
        let (pk, sk) = crypto_box_seed_keypair(&[0u8; 32]);
        (MishpatQuorumPubKey(pk), MishpatQuorumSecretKey(sk))
    }

    fn imagodei_keypair() -> (ImagodeiPubKey, ImagodeiSecretKey) {
        let (pk, sk) = crypto_box_seed_keypair(&[1u8; 32]);
        (ImagodeiPubKey(pk), ImagodeiSecretKey(sk))
    }

    fn sealing_pub_keys<'a>(
        mishpat_pk: &'a MishpatQuorumPubKey,
        imagodei_pk: &'a ImagodeiPubKey,
    ) -> SealingPubKeys<'a> {
        SealingPubKeys {
            mishpat_pk,
            imagodei_pk,
        }
    }

    fn unsealing_keys<'a>(
        mishpat_pk: &'a MishpatQuorumPubKey,
        mishpat_sk: &'a MishpatQuorumSecretKey,
        imagodei_pk: &'a ImagodeiPubKey,
        imagodei_sk: &'a ImagodeiSecretKey,
    ) -> UnsealingKeys<'a> {
        UnsealingKeys {
            mishpat_pk,
            mishpat_sk,
            imagodei_pk,
            imagodei_sk,
        }
    }

    // -----------------------------------------------------------------------
    // MockSink — records send calls for assertion
    // -----------------------------------------------------------------------

    struct MockSink {
        calls: Mutex<Vec<(String, Vec<u8>)>>,
    }

    impl MockSink {
        fn new() -> Self {
            MockSink {
                calls: Mutex::new(vec![]),
            }
        }

        fn recorded_calls(&self) -> Vec<(String, Vec<u8>)> {
            self.calls.lock().unwrap().clone()
        }
    }

    impl OutboundSink for MockSink {
        fn send(&self, peer_id_str: &str, payload: Vec<u8>) -> Result<(), SinkError> {
            self.calls
                .lock()
                .unwrap()
                .push((peer_id_str.to_string(), payload));
            Ok(())
        }
    }

    /// Sink that always returns an error — used to test best-effort behaviour.
    struct FailingSink;

    impl OutboundSink for FailingSink {
        fn send(&self, _peer_id_str: &str, _payload: Vec<u8>) -> Result<(), SinkError> {
            Err(SinkError::Send("test transport failure".to_string()))
        }
    }

    // -----------------------------------------------------------------------
    // Helper: a minimal FeedbackSignal for a given target_cid
    // -----------------------------------------------------------------------

    fn make_signal(target_cid: &str) -> FeedbackSignal {
        use crate::p2p::feedback_signal::{SignalKind, StandingImpact};

        FeedbackSignal {
            target_cid: target_cid.to_string(),
            signal_kind: SignalKind::Squelch,
            evidence_cid: None,
            standing_impact: StandingImpact::Advisory,
            signed_by: "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=".to_string(),
            signature: "BBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBBB=".to_string(),
        }
    }

    // -----------------------------------------------------------------------
    // Test 1: record_and_read_predecessor_round_trips
    // -----------------------------------------------------------------------

    #[test]
    fn record_and_read_predecessor_round_trips() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let pub_keys = sealing_pub_keys(&mishpat_pk, &imagodei_pk);
        let sec_keys = unsealing_keys(&mishpat_pk, &mishpat_sk, &imagodei_pk, &imagodei_sk);

        record_predecessor(
            &mut conn,
            "bafyreiabcdef_t1",
            "12D3KooWRoundTripPeer",
            &pub_keys,
        )
        .expect("record_predecessor should succeed");

        let peer_ids =
            read_predecessors(&mut conn, "bafyreiabcdef_t1", &sec_keys).expect("read_predecessors");

        assert_eq!(peer_ids.len(), 1, "should have exactly one predecessor");
        assert_eq!(peer_ids[0], "12D3KooWRoundTripPeer");
    }

    // -----------------------------------------------------------------------
    // Test 2: back_prop_no_predecessor_returns_none
    // -----------------------------------------------------------------------

    #[test]
    fn back_prop_no_predecessor_returns_none() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let sec_keys = unsealing_keys(&mishpat_pk, &mishpat_sk, &imagodei_pk, &imagodei_sk);
        let sink = MockSink::new();
        let signal = make_signal("bafyreiabcdef_t2_no_predecessor");

        let result = back_prop_one_hop(&mut conn, &signal, &sec_keys, &sink)
            .expect("back_prop_one_hop should not error");

        assert!(result.is_empty(), "no predecessors → empty result");
        assert!(
            sink.recorded_calls().is_empty(),
            "sink must not be invoked when no predecessor exists"
        );
    }

    // -----------------------------------------------------------------------
    // Test 3: back_prop_with_predecessor_sends_to_peer
    // -----------------------------------------------------------------------

    #[test]
    fn back_prop_with_predecessor_sends_to_peer() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let pub_keys = sealing_pub_keys(&mishpat_pk, &imagodei_pk);
        let sec_keys = unsealing_keys(&mishpat_pk, &mishpat_sk, &imagodei_pk, &imagodei_sk);

        record_predecessor(
            &mut conn,
            "bafyreiabcdef_t3",
            "12D3KooWSendToPeer",
            &pub_keys,
        )
        .expect("record_predecessor");

        let sink = MockSink::new();
        let signal = make_signal("bafyreiabcdef_t3");

        let targeted =
            back_prop_one_hop(&mut conn, &signal, &sec_keys, &sink).expect("back_prop_one_hop");

        assert_eq!(targeted.len(), 1, "one predecessor → one targeted peer");
        assert_eq!(targeted[0], "12D3KooWSendToPeer");

        let calls = sink.recorded_calls();
        assert_eq!(calls.len(), 1, "sink should have received exactly one call");
        assert_eq!(calls[0].0, "12D3KooWSendToPeer");

        // Verify the payload decodes back to the original signal.
        let decoded_signal: FeedbackSignal =
            rmp_serde::from_slice(&calls[0].1).expect("payload should decode to FeedbackSignal");
        assert_eq!(decoded_signal, signal);
    }

    // -----------------------------------------------------------------------
    // Test 4: record_predecessor_idempotent_on_duplicate
    // -----------------------------------------------------------------------

    #[test]
    fn record_predecessor_idempotent_on_duplicate() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let (mishpat_pk, _mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, _imagodei_sk) = imagodei_keypair();

        let pub_keys = sealing_pub_keys(&mishpat_pk, &imagodei_pk);

        record_predecessor(
            &mut conn,
            "bafyreiabcdef_t4",
            "12D3KooWIdempotentPeer",
            &pub_keys,
        )
        .expect("first record_predecessor");

        // Second call with the same (target_cid, predecessor_peer_id) must not error.
        record_predecessor(
            &mut conn,
            "bafyreiabcdef_t4",
            "12D3KooWIdempotentPeer",
            &pub_keys,
        )
        .expect("second record_predecessor should be idempotent");

        // Only one row should exist.
        let rows = crate::db::predecessor_records::list_predecessors_for_cid(
            &mut conn,
            "bafyreiabcdef_t4",
        )
        .expect("list");
        assert_eq!(
            rows.len(),
            1,
            "exactly one row must exist after duplicate insert"
        );
    }

    // -----------------------------------------------------------------------
    // Test 5: back_prop_with_multiple_predecessors_sends_to_all
    //
    // Semantic: ALL predecessors receive the signal (fan-out per §5.2).
    // -----------------------------------------------------------------------

    #[test]
    fn back_prop_with_multiple_predecessors_sends_to_all() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let pub_keys = sealing_pub_keys(&mishpat_pk, &imagodei_pk);
        let sec_keys = unsealing_keys(&mishpat_pk, &mishpat_sk, &imagodei_pk, &imagodei_sk);

        record_predecessor(
            &mut conn,
            "bafyreiabcdef_t5",
            "12D3KooWPeerAlpha",
            &pub_keys,
        )
        .expect("record peer alpha");
        record_predecessor(&mut conn, "bafyreiabcdef_t5", "12D3KooWPeerBeta", &pub_keys)
            .expect("record peer beta");

        let sink = MockSink::new();
        let signal = make_signal("bafyreiabcdef_t5");

        let targeted =
            back_prop_one_hop(&mut conn, &signal, &sec_keys, &sink).expect("back_prop_one_hop");

        assert_eq!(targeted.len(), 2, "two predecessors → two targeted peers");

        let mut peer_ids: Vec<String> = targeted.clone();
        peer_ids.sort();
        assert!(
            peer_ids.contains(&"12D3KooWPeerAlpha".to_string()),
            "Alpha should be targeted"
        );
        assert!(
            peer_ids.contains(&"12D3KooWPeerBeta".to_string()),
            "Beta should be targeted"
        );

        let calls = sink.recorded_calls();
        assert_eq!(
            calls.len(),
            2,
            "sink should receive exactly two calls (one per predecessor)"
        );

        let called_peers: Vec<&str> = calls.iter().map(|(p, _)| p.as_str()).collect();
        assert!(
            called_peers.contains(&"12D3KooWPeerAlpha"),
            "sink should have been called for Alpha"
        );
        assert!(
            called_peers.contains(&"12D3KooWPeerBeta"),
            "sink should have been called for Beta"
        );
    }

    // -----------------------------------------------------------------------
    // Test 6 (bonus): sink failure is best-effort — peer still appears in result
    // -----------------------------------------------------------------------

    #[test]
    fn sink_failure_is_best_effort_and_does_not_abort() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let (mishpat_pk, mishpat_sk) = mishpat_keypair();
        let (imagodei_pk, imagodei_sk) = imagodei_keypair();

        let pub_keys = sealing_pub_keys(&mishpat_pk, &imagodei_pk);
        let sec_keys = unsealing_keys(&mishpat_pk, &mishpat_sk, &imagodei_pk, &imagodei_sk);

        record_predecessor(
            &mut conn,
            "bafyreiabcdef_t6",
            "12D3KooWFailingPeer",
            &pub_keys,
        )
        .expect("record_predecessor");

        let sink = FailingSink;
        let signal = make_signal("bafyreiabcdef_t6");

        // back_prop_one_hop must not propagate the sink error as Err.
        let targeted = back_prop_one_hop(&mut conn, &signal, &sec_keys, &sink)
            .expect("back_prop_one_hop must return Ok even on sink failure");

        // The peer appears in targeted (we tried) even though the sink failed.
        assert_eq!(targeted.len(), 1);
        assert_eq!(targeted[0], "12D3KooWFailingPeer");
    }
}
