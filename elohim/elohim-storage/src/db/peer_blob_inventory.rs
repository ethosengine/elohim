//! CRUD for `peer_blob_inventory` and `peer_inventory_cursor`.
//!
//! Source of truth: signed inventory envelopes on topic `elohim/inventory/blob/2`
//! (`p2p::inventory_gossip`) and negentropy-v1 range reconciliation
//! (`p2p::inventory_recon`). Category C operational projection, rebuildable by recon.
//! Manifest counterpart: rea_commitments(action='custody-blob'). Every `blob_cid` is
//! the byte CID text of an address the caller already admitted by full decode.
//!
//! ## Sequence semantics
//!
//! - A delta applies only when `sequence == stored_max + 1` (or the peer is new).
//!   A replay (`sequence <= stored_max`) is dropped; a gap is reported so the caller
//!   runs a recon session with that peer.
//! - [`rebase_after_recon`] is the ONLY path that moves a cursor to an arbitrary
//!   sequence: a completed recon proves the projected set equals the holder's set at
//!   the responder's sequence.
//!
//! ## Witness (provenance)
//!
//! Every row records the [`InventoryWitness`] that admitted it. Custody floors
//! ([`holds_bound_fresh`]) count only rows whose witness bound the peer id to the
//! speaker; a `legacy-hint` row — every row older than the column, and anything
//! written without a witness — stays a fetch hint ([`lookup_hosts`]) and never counts.

use crate::db::diesel_schema::{peer_blob_inventory, peer_inventory_cursor};
use crate::db::models::{NewPeerInventoryCursorRow, PeerBlobInventoryRow};
use crate::error::StorageError;
use diesel::prelude::*;

/// Which witness admitted a `peer_blob_inventory` row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InventoryWitness {
    /// libp2p gossipsub: the signed `message.source` is the envelope's author, the
    /// envelope signature verifies under the author's key, and the author is bound in
    /// `peer_transport_manifest` (`p2p::inventory_gossip::admit`).
    SignedAuthor,
    /// iroh-gossip, or an admitted page applied later from the reorder window: the
    /// envelope signature over the bound author's key is the authentication.
    EndToEnd,
    /// A negentropy recon session answered over the peer's own authenticated libp2p
    /// connection (the responder is the peer the rows are keyed by).
    Recon,
    /// This node fetched the bytes from the peer and they hashed to their CID.
    FetchVerified,
    /// This node's own store verified the bytes.
    SelfVerified,
    /// No recorded witness: a fetch hint only, never custody evidence.
    LegacyHint,
}

impl InventoryWitness {
    /// Every witness that binds a row to the peer it names.
    pub const BOUND: [InventoryWitness; 5] = [
        Self::SignedAuthor,
        Self::EndToEnd,
        Self::Recon,
        Self::FetchVerified,
        Self::SelfVerified,
    ];

    /// The `witness` column spelling.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SignedAuthor => "signed-author",
            Self::EndToEnd => "end-to-end",
            Self::Recon => "recon",
            Self::FetchVerified => "fetch-verified",
            Self::SelfVerified => "self",
            Self::LegacyHint => "legacy-hint",
        }
    }

    /// Parse a column value; anything unknown reads as [`Self::LegacyHint`].
    pub fn parse(text: &str) -> Self {
        Self::BOUND
            .into_iter()
            .find(|w| w.as_str() == text)
            .unwrap_or(Self::LegacyHint)
    }

    /// Whether a row admitted by this witness may count toward a custody floor.
    pub fn is_bound(self) -> bool {
        self != Self::LegacyHint
    }
}

/// The witnessed row every writer in this module puts.
struct WitnessedRow<'a> {
    peer_id: &'a str,
    blob_cid: &'a str,
    last_seen_at: &'a str,
    source: &'a str,
    sequence: i64,
    witness: &'static str,
}

fn put_row(conn: &mut SqliteConnection, row: &WitnessedRow<'_>) -> QueryResult<usize> {
    use peer_blob_inventory::dsl as t;
    diesel::replace_into(t::peer_blob_inventory)
        .values((
            t::peer_id.eq(row.peer_id),
            t::blob_cid.eq(row.blob_cid),
            t::last_seen_at.eq(row.last_seen_at),
            t::source.eq(row.source),
            t::sequence.eq(row.sequence),
            t::witness.eq(row.witness),
        ))
        .execute(conn)
}

/// Apply a signed inventory delta from `peer_id`.
///
/// Returns:
/// - `Ok(DeltaApplyOutcome::Applied)` — delta was applied and cursor advanced.
/// - `Ok(DeltaApplyOutcome::Replay)` — sequence ≤ stored max; dropped.
/// - `Ok(DeltaApplyOutcome::Gap { expected, received })` — sequence skipped; nothing
///   applied, the caller reconciles with the source peer.
/// - `Err(_)` — actual database failure.
pub fn apply_delta(
    conn: &mut SqliteConnection,
    peer_id: &str,
    added: &[String],
    removed: &[String],
    sequence: i64,
    emitted_at: &str,
    witness: InventoryWitness,
) -> Result<DeltaApplyOutcome, StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        let stored_max = read_cursor_sequence(conn, peer_id)?;
        match stored_max {
            Some(max) if sequence <= max => {
                Ok::<DeltaApplyOutcome, diesel::result::Error>(DeltaApplyOutcome::Replay)
            }
            Some(max) if sequence != max + 1 => Ok(DeltaApplyOutcome::Gap {
                expected: max + 1,
                received: sequence,
            }),
            _ => {
                write_set_change(
                    conn,
                    peer_id,
                    added,
                    removed,
                    Stamp {
                        source: "gossip-delta",
                        sequence,
                        at: emitted_at,
                        witness,
                    },
                )?;
                upsert_cursor(conn, peer_id, sequence, emitted_at)?;
                Ok(DeltaApplyOutcome::Applied)
            }
        }
    })
    .map_err(|e| StorageError::Database(format!("apply_delta: {e}")))
}

/// Project a completed recon session with `peer_id`: `add` are the byte CIDs the peer
/// holds that this projection lacked, `remove` the ones it listed that the peer does not
/// hold. The cursor is re-based to the responder's `sequence` — the one path that moves
/// a cursor to an arbitrary sequence, because a converged recon proves the set.
pub fn rebase_after_recon(
    conn: &mut SqliteConnection,
    peer_id: &str,
    add: &[String],
    remove: &[String],
    sequence: i64,
    when: &str,
) -> Result<(), StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        write_set_change(
            conn,
            peer_id,
            add,
            remove,
            Stamp {
                source: "recon",
                sequence,
                at: when,
                witness: InventoryWitness::Recon,
            },
        )?;
        upsert_cursor(conn, peer_id, sequence, when)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("rebase_after_recon: {e}")))
}

/// Record a successful direct fetch. Promotes the entry to source='fetch-success'
/// (the strongest evidence). Does NOT touch the cursor — fetch-success is an
/// out-of-band evidence path, not a gossip arrival.
pub fn record_fetch_success(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_cid: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    super::read_then_write_transaction(conn, |conn| {
        let existing_seq: Option<i64> = peer_blob_inventory::table
            .filter(peer_blob_inventory::peer_id.eq(peer_id))
            .filter(peer_blob_inventory::blob_cid.eq(blob_cid))
            .select(peer_blob_inventory::sequence)
            .first::<i64>(conn)
            .optional()?;

        put_row(
            conn,
            &WitnessedRow {
                peer_id,
                blob_cid,
                last_seen_at: observed_at,
                source: "fetch-success",
                sequence: existing_seq.unwrap_or(0),
                witness: InventoryWitness::FetchVerified.as_str(),
            },
        )?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("record_fetch_success: {e}")))
}

/// Stamp a row recording that *this* peer holds `blob_cid` locally (the blob store
/// verified the bytes when it stored them). Idempotent; does NOT touch the cursor —
/// self-custody is local evidence, not a gossip arrival.
pub fn record_self_custody(
    conn: &mut SqliteConnection,
    peer_id: &str,
    blob_cid: &str,
    observed_at: &str,
) -> Result<(), StorageError> {
    put_row(
        conn,
        &WitnessedRow {
            peer_id,
            blob_cid,
            last_seen_at: observed_at,
            source: "self-custody",
            sequence: 0,
            witness: InventoryWitness::SelfVerified.as_str(),
        },
    )
    .map(|_| ())
    .map_err(|e| StorageError::Database(format!("record_self_custody: {e}")))
}

/// The peers known to host a blob, ordered by evidence strength (fetch-success first,
/// then by recency).
pub fn lookup_hosts(
    conn: &mut SqliteConnection,
    blob_cid: &str,
    fresh_after: &str,
) -> Result<Vec<PeerBlobInventoryRow>, StorageError> {
    use peer_blob_inventory::dsl;

    dsl::peer_blob_inventory
        .filter(dsl::blob_cid.eq(blob_cid))
        .filter(dsl::last_seen_at.gt(fresh_after))
        .order(dsl::last_seen_at.desc())
        .select(PeerBlobInventoryRow::as_select())
        .load::<PeerBlobInventoryRow>(conn)
        .map(|rows| {
            let (mut fetch_success, rest): (Vec<_>, Vec<_>) =
                rows.into_iter().partition(|r| r.source == "fetch-success");
            fetch_success.extend(rest);
            fetch_success
        })
        .map_err(|e| StorageError::Database(format!("lookup_hosts: {e}")))
}

/// The byte CIDs `peer_id` is projected to hold — the initiator's set for a recon
/// session with that peer.
pub fn cids_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Vec<String>, StorageError> {
    peer_blob_inventory::table
        .filter(peer_blob_inventory::peer_id.eq(peer_id))
        .select(peer_blob_inventory::blob_cid)
        .load::<String>(conn)
        .map_err(|e| StorageError::Database(format!("cids_for_peer: {e}")))
}

/// Whether any of `identities` (one holder's spellings: its `agent_cid` and the
/// transport ids bound to it) is projected to hold `blob_cid` under a bound witness,
/// seen after `fresh_after`. This is the custody-floor read: a `legacy-hint` row or an
/// expired row never counts.
pub fn holds_bound_fresh(
    conn: &mut SqliteConnection,
    identities: &[String],
    blob_cid: &str,
    fresh_after: &str,
) -> Result<bool, StorageError> {
    if identities.is_empty() {
        return Ok(false);
    }
    let bound: Vec<&str> = InventoryWitness::BOUND.iter().map(|w| w.as_str()).collect();
    peer_blob_inventory::table
        .filter(peer_blob_inventory::peer_id.eq_any(identities))
        .filter(peer_blob_inventory::blob_cid.eq(blob_cid))
        .filter(peer_blob_inventory::last_seen_at.gt(fresh_after))
        .filter(peer_blob_inventory::witness.eq_any(bound))
        .count()
        .get_result::<i64>(conn)
        .map(|n| n > 0)
        .map_err(|e| StorageError::Database(format!("holds_bound_fresh: {e}")))
}

/// Outcome of [`apply_delta`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeltaApplyOutcome {
    Applied,
    Replay,
    Gap { expected: i64, received: i64 },
}

/// How an arriving set change is stamped on each row it adds.
struct Stamp<'a> {
    source: &'a str,
    sequence: i64,
    at: &'a str,
    witness: InventoryWitness,
}

fn write_set_change(
    conn: &mut SqliteConnection,
    peer_id: &str,
    added: &[String],
    removed: &[String],
    stamp: Stamp<'_>,
) -> Result<(), diesel::result::Error> {
    for cid in added {
        put_row(
            conn,
            &WitnessedRow {
                peer_id,
                blob_cid: cid,
                last_seen_at: stamp.at,
                source: stamp.source,
                sequence: stamp.sequence,
                witness: stamp.witness.as_str(),
            },
        )?;
    }
    for cid in removed {
        diesel::delete(
            peer_blob_inventory::table
                .filter(peer_blob_inventory::peer_id.eq(peer_id))
                .filter(peer_blob_inventory::blob_cid.eq(cid)),
        )
        .execute(conn)?;
    }
    Ok(())
}

fn upsert_cursor(
    conn: &mut SqliteConnection,
    peer_id: &str,
    sequence: i64,
    last_updated: &str,
) -> Result<(), diesel::result::Error> {
    let row = NewPeerInventoryCursorRow {
        peer_id: peer_id.to_string(),
        last_sequence: sequence,
        last_updated: last_updated.to_string(),
    };
    diesel::replace_into(peer_inventory_cursor::table)
        .values(&row)
        .execute(conn)
        .map(|_| ())
}

fn read_cursor_sequence(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Option<i64>, diesel::result::Error> {
    peer_inventory_cursor::table
        .filter(peer_inventory_cursor::peer_id.eq(peer_id))
        .select(peer_inventory_cursor::last_sequence)
        .first::<i64>(conn)
        .optional()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;
    use elohim_epr::address::compute_bytes_cid;

    fn cid(label: &str) -> String {
        compute_bytes_cid(label.as_bytes()).to_string()
    }

    const T0: &str = "2026-11-06T00:00:00Z";
    const T1: &str = "2026-11-06T00:00:30Z";
    const LONG_AGO: &str = "2026-11-01T00:00:00Z";

    #[test]
    fn delta_applied_when_in_order() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let first = apply_delta(
            &mut conn,
            "peer_B",
            &[cid("h1")],
            &[],
            1,
            T0,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        assert_eq!(first, DeltaApplyOutcome::Applied);
        let next = apply_delta(
            &mut conn,
            "peer_B",
            &[cid("h2")],
            &[cid("h1")],
            2,
            T1,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        assert_eq!(next, DeltaApplyOutcome::Applied);

        assert_eq!(
            lookup_hosts(&mut conn, &cid("h2"), LONG_AGO).unwrap().len(),
            1
        );
        assert!(lookup_hosts(&mut conn, &cid("h1"), LONG_AGO)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn delta_gap_returns_gap_and_writes_nothing() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        apply_delta(
            &mut conn,
            "peer_C",
            &[cid("h1")],
            &[],
            1,
            T0,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        let outcome = apply_delta(
            &mut conn,
            "peer_C",
            &[cid("h2")],
            &[],
            5,
            T1,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        assert_eq!(
            outcome,
            DeltaApplyOutcome::Gap {
                expected: 2,
                received: 5
            }
        );
        assert!(lookup_hosts(&mut conn, &cid("h2"), LONG_AGO)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn replayed_delta_drops_and_never_moves_the_cursor() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for seq in 1..=5 {
            apply_delta(
                &mut conn,
                "peer_D",
                &[cid(&format!("h{seq}"))],
                &[],
                seq,
                T0,
                InventoryWitness::SignedAuthor,
            )
            .unwrap();
        }
        let outcome = apply_delta(
            &mut conn,
            "peer_D",
            &[cid("late")],
            &[],
            3,
            T1,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        assert_eq!(outcome, DeltaApplyOutcome::Replay);
        assert!(lookup_hosts(&mut conn, &cid("late"), LONG_AGO)
            .unwrap()
            .is_empty());
        assert_eq!(read_cursor_sequence(&mut conn, "peer_D").unwrap(), Some(5));
    }

    #[test]
    fn recon_rebases_the_cursor_and_projects_the_set() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        apply_delta(
            &mut conn,
            "peer_R",
            &[cid("stale")],
            &[],
            900,
            T0,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();

        rebase_after_recon(&mut conn, "peer_R", &[cid("held")], &[cid("stale")], 3, T1).unwrap();
        assert_eq!(read_cursor_sequence(&mut conn, "peer_R").unwrap(), Some(3));
        assert_eq!(
            cids_for_peer(&mut conn, "peer_R").unwrap(),
            vec![cid("held")]
        );
        // The next in-order delta after the re-base applies.
        let next = apply_delta(
            &mut conn,
            "peer_R",
            &[cid("new")],
            &[],
            4,
            T1,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        assert_eq!(next, DeltaApplyOutcome::Applied);
        let host = lookup_hosts(&mut conn, &cid("held"), LONG_AGO).unwrap();
        assert_eq!(host[0].source, "recon");
    }

    #[test]
    fn lookup_hosts_orders_fetch_success_first_and_filters_stale() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let blob = cid("shared");
        apply_delta(
            &mut conn,
            "older",
            std::slice::from_ref(&blob),
            &[],
            1,
            T0,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        apply_delta(
            &mut conn,
            "newer",
            std::slice::from_ref(&blob),
            &[],
            1,
            T1,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        record_fetch_success(&mut conn, "older", &blob, T0).unwrap();

        let rows = lookup_hosts(&mut conn, &blob, LONG_AGO).unwrap();
        assert_eq!(rows[0].peer_id, "older");
        assert_eq!(rows[0].source, "fetch-success");
        assert_eq!(rows[1].peer_id, "newer");
        assert_eq!(lookup_hosts(&mut conn, &blob, T0).unwrap().len(), 1);
    }

    #[test]
    fn record_self_custody_is_idempotent() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let blob = cid("mine");
        record_self_custody(&mut conn, "self", &blob, T0).unwrap();
        record_self_custody(&mut conn, "self", &blob, T1).unwrap();
        let rows = lookup_hosts(&mut conn, &blob, LONG_AGO).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].source, "self-custody");
    }

    fn witness_of(conn: &mut SqliteConnection, peer: &str, blob: &str) -> InventoryWitness {
        let text: String = peer_blob_inventory::table
            .filter(peer_blob_inventory::peer_id.eq(peer))
            .filter(peer_blob_inventory::blob_cid.eq(blob))
            .select(peer_blob_inventory::witness)
            .first(conn)
            .unwrap();
        InventoryWitness::parse(&text)
    }

    #[test]
    fn every_writer_records_its_witness_and_an_unwitnessed_insert_is_a_hint() {
        use crate::db::models::NewPeerBlobInventoryRow;
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let blob = cid("w");
        apply_delta(
            &mut conn,
            "lib",
            std::slice::from_ref(&blob),
            &[],
            1,
            T0,
            InventoryWitness::SignedAuthor,
        )
        .unwrap();
        apply_delta(
            &mut conn,
            "iroh",
            std::slice::from_ref(&blob),
            &[],
            1,
            T0,
            InventoryWitness::EndToEnd,
        )
        .unwrap();
        rebase_after_recon(&mut conn, "recon", std::slice::from_ref(&blob), &[], 1, T0).unwrap();
        record_fetch_success(&mut conn, "fetched", &blob, T0).unwrap();
        record_self_custody(&mut conn, "me", &blob, T0).unwrap();
        diesel::insert_into(peer_blob_inventory::table)
            .values(&NewPeerBlobInventoryRow {
                peer_id: "old".into(),
                blob_cid: blob.clone(),
                last_seen_at: T0.into(),
                source: "gossip-delta".into(),
                sequence: 1,
            })
            .execute(&mut conn)
            .unwrap();
        for (peer, witness) in [
            ("lib", InventoryWitness::SignedAuthor),
            ("iroh", InventoryWitness::EndToEnd),
            ("recon", InventoryWitness::Recon),
            ("fetched", InventoryWitness::FetchVerified),
            ("me", InventoryWitness::SelfVerified),
            ("old", InventoryWitness::LegacyHint),
        ] {
            assert_eq!(witness_of(&mut conn, peer, &blob), witness, "{peer}");
            assert_eq!(
                holds_bound_fresh(&mut conn, &[peer.to_string()], &blob, LONG_AGO).unwrap(),
                witness.is_bound(),
                "{peer}"
            );
        }
        // The legacy row stays a fetch hint.
        assert!(lookup_hosts(&mut conn, &blob, LONG_AGO)
            .unwrap()
            .iter()
            .any(|r| r.peer_id == "old" && r.witness == "legacy-hint"));
    }
}
