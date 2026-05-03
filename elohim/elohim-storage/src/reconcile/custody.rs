//! Custody reconciliation controller.
//!
//! Multi-trigger reconcile pass:
//! - Gossip arrival (`BlobInventorySnapshot` / `BlobInventoryDelta` for peer X)
//! - Connection event (`ConnectionEstablished` for peer X)
//! - Periodic sweep (timer; cadence per `custody_sweep_seconds`)
//!
//! Each trigger calls `reconcile_pass`, which is idempotent.
//!
//! For each `custody-blob` commitment in `rea_commitments`:
//! - If this peer is the provider AND blob_hash is missing locally:
//!   query `peer_blob_inventory` for hosts; if any candidates exist,
//!   kick a fetch via `blob_fetch::race_fetch` (T17).
//! - If this peer is the receiver AND last_seen for the provider is older
//!   than `placement_grace_seconds`: emit a `placement-gap` REA event
//!   (with cooldown).
//!
//! Local store presence is queried via the trait `LocalBlobStore` so the
//! reconcile pass can be unit-tested without a real blob store.

use crate::db::diesel_schema::{economic_events, peer_blob_inventory, rea_commitments};
use crate::db::models::{NewEconomicEvent, ReaCommitment};
use crate::error::StorageError;
use chrono::{DateTime, Utc};
use diesel::prelude::*;

/// Trait for checking whether a blob exists in this peer's local store.
/// Production: queries the blob_store. Tests: returns a fixed set.
pub trait LocalBlobStore: Send + Sync {
    fn has(&self, blob_hash: &str) -> bool;
}

/// Trait for kicking a fetch. Production: calls `blob_fetch::race_fetch` (T17).
/// Tests: records the kick request.
pub trait FetchKicker: Send + Sync {
    fn kick(&self, blob_hash: &str, candidates: Vec<String>);
}

/// Snapshot of the local blob store taken at reconcile-pass start.
///
/// Implements [`LocalBlobStore`] for the duration of a single pass by
/// pre-fetching the full hash set once via [`crate::blob_store::BlobStore::list_hashes`]
/// and answering `has` from a [`std::collections::HashSet`]. This is cheaper
/// (and avoids holding the blob store across diesel queries) than calling
/// `BlobStore::exists` per commitment.
///
/// Hash format: matches whatever `list_hashes` returns (currently
/// `sha256-<hex>`); reconcile_pass does string-equality lookups against
/// `rea_commitments.resource_classified_as`, so the two stores must share a
/// canonical form. T17 / T22 already standardised on `sha256-<hex>`.
pub struct BlobStoreSnapshot {
    hashes: std::collections::HashSet<String>,
}

impl BlobStoreSnapshot {
    /// Build a snapshot by reading the local pantry once. Returns
    /// [`StorageError`] if the directory walk fails; the caller should skip
    /// the reconcile tick rather than running with a partial view.
    pub fn from_store(store: &crate::blob_store::BlobStore) -> Result<Self, StorageError> {
        let hashes = store.list_hashes()?.into_iter().collect();
        Ok(Self { hashes })
    }

    /// Number of hashes captured in the snapshot. Useful for tests + tracing.
    pub fn len(&self) -> usize {
        self.hashes.len()
    }

    /// `true` when the snapshot captured no hashes.
    pub fn is_empty(&self) -> bool {
        self.hashes.is_empty()
    }
}

impl LocalBlobStore for BlobStoreSnapshot {
    fn has(&self, blob_hash: &str) -> bool {
        self.hashes.contains(blob_hash)
    }
}

/// Outcome of a single reconcile pass; useful for tests + metrics.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconcileOutcome {
    pub kicks_fired: u32,
    pub placement_gaps_emitted: u32,
    pub commitments_examined: u32,
}

/// Timing parameters for a reconcile pass.
#[derive(Debug, Clone, Copy)]
pub struct ReconcileConfig {
    /// How long a custody commitment can be unhonored before placement-gap fires.
    pub placement_grace_seconds: u64,
    /// Minimum time between repeated placement-gap events for the same commitment.
    pub placement_gap_cooldown_seconds: u64,
    /// TTL for peer_blob_inventory entries before they're considered stale.
    pub inventory_freshness_seconds: u64,
}

/// Run one reconcile pass over the custody-blob commitments visible in this
/// peer's projection. Idempotent.
pub fn reconcile_pass(
    conn: &mut SqliteConnection,
    self_cid: &str,
    local_store: &dyn LocalBlobStore,
    fetch_kicker: &dyn FetchKicker,
    cfg: ReconcileConfig,
    now: DateTime<Utc>,
) -> Result<ReconcileOutcome, StorageError> {
    let placement_grace_seconds = cfg.placement_grace_seconds;
    let placement_gap_cooldown_seconds = cfg.placement_gap_cooldown_seconds;
    let inventory_freshness_seconds = cfg.inventory_freshness_seconds;
    let mut outcome = ReconcileOutcome::default();

    let custody_rows = rea_commitments::table
        .filter(rea_commitments::action.eq("custody-blob"))
        .load::<ReaCommitment>(conn)
        .map_err(|e| StorageError::Database(format!("load custody-blob commitments: {e}")))?;

    let stale_before = (now - chrono::Duration::seconds(inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let unhonored_before = (now - chrono::Duration::seconds(placement_grace_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();
    let cooldown_after = (now - chrono::Duration::seconds(placement_gap_cooldown_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    for commitment in custody_rows {
        outcome.commitments_examined += 1;
        let Some(blob_hash) = commitment.resource_classified_as.as_ref() else {
            continue;
        };

        if commitment.provider == self_cid {
            // Own commitment — act if missing.
            if !local_store.has(blob_hash) {
                let candidates =
                    crate::db::peer_blob_inventory::lookup_hosts(conn, blob_hash, &stale_before)?;
                if !candidates.is_empty() {
                    let candidate_peers: Vec<String> =
                        candidates.into_iter().map(|c| c.peer_id).collect();
                    fetch_kicker.kick(blob_hash, candidate_peers);
                    outcome.kicks_fired += 1;
                }
            }
        } else if commitment.receiver == self_cid {
            // Other peer's commitment to me — observe; signal on stale.
            let last_seen: Option<String> = peer_blob_inventory::table
                .filter(peer_blob_inventory::peer_id.eq(&commitment.provider))
                .filter(peer_blob_inventory::blob_hash.eq(blob_hash))
                .select(peer_blob_inventory::last_seen_at)
                .first::<String>(conn)
                .optional()
                .map_err(|e| {
                    StorageError::Database(format!("lookup last_seen for placement-gap: {e}"))
                })?;

            let unhonored = match last_seen {
                None => true, // never observed
                Some(ts) => ts.as_str() < unhonored_before.as_str(),
            };

            if !unhonored {
                continue;
            }

            // Cooldown: don't re-emit if a recent placement-gap event for the
            // same commitment exists.
            let recent_gap = economic_events::table
                .filter(economic_events::action.eq("placement-gap"))
                .filter(economic_events::output_of.eq(&commitment.id))
                .filter(economic_events::has_point_in_time.gt(&cooldown_after))
                .count()
                .get_result::<i64>(conn)
                .map_err(|e| StorageError::Database(format!("count recent placement-gap: {e}")))?;

            if recent_gap > 0 {
                continue;
            }

            // Emit placement-gap event.
            // Bind owned strings so we can borrow them into NewEconomicEvent<'_>.
            let event_id = uuid::Uuid::new_v4().to_string();
            let has_point = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
            let new_event = NewEconomicEvent {
                id: &event_id,
                h_app_id: &commitment.h_app_id,
                action: "placement-gap",
                provider: &commitment.provider,
                receiver: &commitment.receiver,
                resource_conforms_to: commitment.resource_conforms_to.as_deref(),
                resource_inventoried_as: Some(blob_hash.as_str()),
                resource_classified_as_json: None,
                resource_quantity_value: None,
                resource_quantity_unit: None,
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_point_in_time: &has_point,
                has_duration: None,
                input_of: None,
                output_of: Some(commitment.id.as_str()),
                lamad_event_type: None,
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: None,
                state: "observed",
                note: None,
                metadata_json: None,
                dht_anchor_hash: None,
                at_location: None,
                verified_at: None,
            };
            diesel::insert_into(economic_events::table)
                .values(&new_event)
                .execute(conn)
                .map_err(|e| StorageError::Database(format!("insert placement-gap event: {e}")))?;
            outcome.placement_gaps_emitted += 1;
        }
    }

    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::diesel_schema::rea_commitments;
    use crate::db::models::NewReaCommitment;
    use crate::db::peer_blob_inventory::apply_snapshot;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};
    use std::sync::Mutex;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:cust_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn default_cfg() -> ReconcileConfig {
        ReconcileConfig {
            placement_grace_seconds: 300,
            placement_gap_cooldown_seconds: 1800,
            inventory_freshness_seconds: 600,
        }
    }

    struct StaticStore(Vec<String>);
    impl LocalBlobStore for StaticStore {
        fn has(&self, hash: &str) -> bool {
            self.0.iter().any(|h| h == hash)
        }
    }

    struct RecordingKicker {
        kicks: Mutex<Vec<(String, Vec<String>)>>,
    }
    impl FetchKicker for RecordingKicker {
        fn kick(&self, blob_hash: &str, candidates: Vec<String>) {
            self.kicks
                .lock()
                .unwrap()
                .push((blob_hash.to_string(), candidates));
        }
    }

    fn insert_custody_commitment(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        receiver: &str,
        blob_hash: &str,
    ) {
        let row = NewReaCommitment {
            id,
            h_app_id: "test",
            action: "custody-blob",
            provider,
            receiver,
            resource_conforms_to: None,
            resource_classified_as: Some(blob_hash),
            resource_quantity_value: Some(1024.0),
            resource_quantity_unit: Some("bytes"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state: "active",
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: Some("hash1"),
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .unwrap();
    }

    #[test]
    fn own_commitment_with_local_blob_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &"a".repeat(64));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec!["a".repeat(64)]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
        assert!(kicker.kicks.lock().unwrap().is_empty());
    }

    #[test]
    fn own_commitment_with_missing_blob_kicks_when_candidate_exists() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &blob_hash);

        // Use a fresh timestamp so the inventory entry is within freshness window.
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "peer_X",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]), // local store empty
            &kicker,
            default_cfg(),
            now,
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 1);
        let kicks = kicker.kicks.lock().unwrap();
        assert_eq!(kicks.len(), 1);
        assert_eq!(kicks[0].0, blob_hash);
        assert_eq!(kicks[0].1, vec!["peer_X".to_string()]);
    }

    #[test]
    fn own_commitment_with_no_candidates_no_kick() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        insert_custody_commitment(&mut conn, "c1", "self_cid", "other_cid", &"a".repeat(64));

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(outcome.kicks_fired, 0);
    }

    #[test]
    fn others_commitment_unhonored_emits_placement_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            chrono::Utc::now(),
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 1);
        assert_eq!(outcome.kicks_fired, 0);

        // Verify the event landed.
        use crate::db::diesel_schema::economic_events;
        let count: i64 = economic_events::table
            .filter(economic_events::action.eq("placement-gap"))
            .filter(economic_events::output_of.eq("c1"))
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn others_commitment_with_fresh_inventory_no_gap() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        // Fresh inventory entry from other_cid.
        let now = chrono::Utc::now();
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        apply_snapshot(
            &mut conn,
            "other_cid",
            std::slice::from_ref(&blob_hash),
            1,
            &when,
        )
        .unwrap();

        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }

    #[test]
    fn placement_gap_cooldown_suppresses_repeat() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        let blob_hash = "a".repeat(64);
        insert_custody_commitment(&mut conn, "c1", "other_cid", "self_cid", &blob_hash);

        // First pass: emit gap.
        let now = chrono::Utc::now();
        let kicker = RecordingKicker {
            kicks: Mutex::new(Vec::new()),
        };
        let _ = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now,
        )
        .unwrap();

        // Second pass within cooldown — should suppress.
        let outcome = reconcile_pass(
            &mut conn,
            "self_cid",
            &StaticStore(vec![]),
            &kicker,
            default_cfg(),
            now + chrono::Duration::seconds(60),
        )
        .unwrap();

        assert_eq!(outcome.placement_gaps_emitted, 0);
    }

    // -----------------------------------------------------------------------
    // T23: BlobStoreSnapshot adapter
    // -----------------------------------------------------------------------

    /// Snapshot taken after a blob is stored reports `has(hash) == true`,
    /// and reports `has` of an absent hash as `false`.
    #[tokio::test]
    async fn blob_store_snapshot_reflects_local_pantry() {
        let store = crate::blob_store::BlobStore::new_memory();
        let payload = b"reconcile-snapshot-fixture-v1";
        let result = store.store(payload).await.expect("store fixture blob");

        let snapshot = BlobStoreSnapshot::from_store(&store).expect("snapshot");

        assert!(
            snapshot.has(&result.hash),
            "snapshot must report stored hash present"
        );
        assert!(
            !snapshot
                .has("sha256-deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"),
            "snapshot must report unknown hash absent"
        );
        assert_eq!(snapshot.len(), 1, "snapshot captured one hash");
        assert!(!snapshot.is_empty(), "snapshot is non-empty after store");
    }
}
