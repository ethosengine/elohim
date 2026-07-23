//! Honest self-stewardship recording for shard placement.
//!
//! ## Why this exists
//!
//! `peer_statuses` is structurally SELF-ONLY today: each node's rows are
//! author-local (one production writer — a self-authored zome call whose
//! post_commit signal never leaves the author), and cross-agent aggregation is
//! explicitly deferred in the infrastructure zome. `services::peer_selection`
//! ranks candidates out of `peer_statuses`, so the selector can currently only
//! ever select **this node itself**.
//!
//! The distribution path treated that self-selection as a remote peer: it looked
//! the selected `agent_cid` up in the transport tables, found nothing (you have
//! no transport binding TO yourself), incremented
//! `elohim_shard_push_peer_unresolved_total`, and skipped the shard. Result:
//! `shard_locations` stayed empty and the resilience card read all zeros.
//!
//! Dialing yourself is the wrong move. When the selected holder IS this node,
//! the honest move is to record the location we can actually verify — and to
//! record NOTHING when we cannot.
//!
//! ## The verification rule (non-negotiable)
//!
//! A `shard_locations` row is a claim that a named agent holds those bytes.
//! A fabricated row is worse than a dark card — it makes the resilience surface
//! lie. So a self-held row is written **only** when the local blob store
//! actually holds the shard's bytes, probed through the existing
//! [`LocalBlobStore`] seam (the same one `reconcile::custody` uses).
//!
//! This is NOT a no-op in practice: `sharding::ShardEncoder` uses
//! `encoding = "none"` for blobs ≤ `SINGLE_SHARD_MAX` (16 MiB), where the single
//! shard IS the whole blob and `shard_hash == blob_hash` — bytes this node
//! demonstrably holds (they are what it just encoded). Larger content
//! (`"chunked"` / `"rs-4-7"`) produces derived shard payloads that are NOT in
//! the local blob store, so those honestly record no self-held location and
//! leave the placement gap visible.
//!
//! ## Identity coherence
//!
//! `shard_locations.peer_id` is an `agent_cid` (`uhCAk…`) join key — the
//! resilience join is `humans.agent_pub_key == shard_locations.peer_id`. This
//! module REFUSES to write a value from any other identity namespace (a libp2p
//! `12D3Koo…` / iroh NodeId / random placeholder-shaped junk fails the
//! `is_agent_cid` gate), per `elohim-storage/CLAUDE.md` → "Identity &
//! Transport-Identity Coherence". A junk join key is a row that can never join.
//!
//! Category C (operational): `shard_locations` is local projection state
//! rebuilt from shard-plane events, not DHT-notarized.

use diesel::SqliteConnection;

use crate::db::models::NewShardLocation;
use crate::reconcile::custody::LocalBlobStore;
use crate::StorageError;

/// `shard_locations.status` for a location this node holds itself.
///
/// The column is free-form TEXT (no CHECK constraint, no enum) and already
/// carries `announced` (pushed + acked by a remote peer), `verified`, `seeded`
/// (fixture-authored) and `lost`. `self-held` is added as a distinct member of
/// that vocabulary specifically so a self-held location is never mistaken for a
/// peer-verified one. Nothing filters on `status` except the `ne("lost")`
/// liveness guard and the resilience join (which does not filter at all), so a
/// new value flows through without shadowing an existing read.
pub const SELF_HELD_STATUS: &str = "self-held";

/// Outcome of one self-held recording attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelfHeldOutcome {
    /// Bytes verified present locally; a `shard_locations` row was written.
    Recorded,
    /// The local blob store does NOT hold this shard's bytes — nothing written,
    /// the placement gap stays honestly visible.
    BytesAbsent,
    /// The supplied self identity is not an `agent_cid` (`uhCAk…`) — refused, so
    /// a cross-namespace value can never land in the join-key column.
    NotAgentCid,
}

/// Record that THIS node holds `shard_hash`, but only if it verifiably does.
///
/// Returns [`SelfHeldOutcome::Recorded`] iff `local_store.has(shard_hash)` and
/// `self_agent_cid` is a well-formed `agent_cid`. Never dials, never pushes.
pub fn record_self_held_shard(
    conn: &mut SqliteConnection,
    local_store: &dyn LocalBlobStore,
    shard_hash: &str,
    self_agent_cid: &str,
    h_app_id: &str,
) -> Result<SelfHeldOutcome, StorageError> {
    if !crate::identity_namespace::is_agent_cid(self_agent_cid) {
        tracing::warn!(
            self_agent_cid = %self_agent_cid,
            shard_hash = %shard_hash,
            "self-stewardship refused: identity is not an agent_cid (uhCAk…); a \
             cross-namespace value in shard_locations.peer_id can never join"
        );
        return Ok(SelfHeldOutcome::NotAgentCid);
    }

    if !local_store.has(shard_hash) {
        tracing::debug!(
            shard_hash = %shard_hash,
            "self-stewardship skipped: local blob store does not hold these bytes \
             (gap left honestly visible)"
        );
        return Ok(SelfHeldOutcome::BytesAbsent);
    }

    crate::db::shard_locations::upsert_location(
        conn,
        &NewShardLocation {
            shard_hash,
            peer_id: self_agent_cid,
            h_app_id,
            status: SELF_HELD_STATUS,
        },
    )?;

    Ok(SelfHeldOutcome::Recorded)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::shard_locations::get_locations_for_shard;
    use crate::test_util::test_pool;

    /// Minimal `LocalBlobStore` fake: holds an explicit hash list.
    struct FakeStore(Vec<&'static str>);
    impl LocalBlobStore for FakeStore {
        fn has(&self, hash: &str) -> bool {
            self.0.contains(&hash)
        }
    }

    #[test]
    fn records_a_self_held_row_when_bytes_are_present() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = FakeStore(vec!["sha256-present"]);

        let outcome = record_self_held_shard(
            &mut conn,
            &store,
            "sha256-present",
            "uhCAkSelfAgent",
            "lamad",
        )
        .expect("record");
        assert_eq!(outcome, SelfHeldOutcome::Recorded);

        let rows = get_locations_for_shard(&mut conn, "sha256-present").expect("load");
        assert_eq!(rows.len(), 1, "exactly one location row");
        assert_eq!(rows[0].peer_id, "uhCAkSelfAgent");
        assert_eq!(
            rows[0].status, SELF_HELD_STATUS,
            "self-held must be distinguishable from a peer-verified location"
        );
    }

    #[test]
    fn records_nothing_when_bytes_are_absent() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = FakeStore(vec!["sha256-something-else"]);

        let outcome = record_self_held_shard(
            &mut conn,
            &store,
            "sha256-absent",
            "uhCAkSelfAgent",
            "lamad",
        )
        .expect("record");
        assert_eq!(outcome, SelfHeldOutcome::BytesAbsent);

        let rows = get_locations_for_shard(&mut conn, "sha256-absent").expect("load");
        assert!(
            rows.is_empty(),
            "an unverifiable location must never be fabricated, got {rows:?}"
        );
    }

    /// A transport id (or any non-`uhCAk` value) is refused outright — it could
    /// never join `humans.agent_pub_key`.
    #[test]
    fn refuses_a_non_agent_cid_identity() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = FakeStore(vec!["sha256-present"]);

        let outcome = record_self_held_shard(
            &mut conn,
            &store,
            "sha256-present",
            "12D3KooWTransportId",
            "lamad",
        )
        .expect("record");
        assert_eq!(outcome, SelfHeldOutcome::NotAgentCid);

        assert!(
            get_locations_for_shard(&mut conn, "sha256-present")
                .expect("load")
                .is_empty(),
            "a cross-namespace identity must not produce a row"
        );
    }

    /// Re-running the same recording is idempotent (upsert semantics) — one row.
    #[test]
    fn self_held_recording_is_idempotent() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let store = FakeStore(vec!["sha256-present"]);

        for _ in 0..3 {
            record_self_held_shard(
                &mut conn,
                &store,
                "sha256-present",
                "uhCAkSelfAgent",
                "lamad",
            )
            .expect("record");
        }

        let rows = get_locations_for_shard(&mut conn, "sha256-present").expect("load");
        assert_eq!(rows.len(), 1, "upsert keeps a single row");
    }
}
