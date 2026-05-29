//! Computes PeerCapacityView per spec §7.1. Read-only projection from three
//! live sources + constitutional_ratio_registry:
//!   - `total_raw_bytes` — local node: filesystem probe; remote: latest
//!     `infrastructure:system-sample` observation (`disk_total_bytes`).
//!   - per-tier pledges — `rea_commitments` (action `replicates-dwelling`),
//!     summing `capacity_bytes` from each commitment's `metadata_json`
//!     (`ReplicatesDwellingPayload`), tier derived from `provider_role`.
//!   - `unique_shard_bytes` — `peer_blob_inventory` held blobs joined to
//!     `shard_manifests.total_size_bytes`, deduped on blob hash so a blob
//!     satisfying multiple commitments counts once.
//!
//! Producer note: the remote `total_raw_bytes` path and the pledge path read 0
//! until upstream producers land (a periodic `infrastructure:system-sample`
//! emitter + gossip; a `replicates-dwelling` commitment writer). The readers
//! are correct projections; the local capacity probe and held-byte join are
//! live today.

use diesel::sqlite::SqliteConnection;
use diesel::{ExpressionMethods, OptionalExtension, QueryDsl, RunQueryDsl};

use crate::error::StorageError;
use crate::services::constitutional_ratio_registry;
use elohim_views::peer_capacity::{
    ActuallyHeldView, CurrentRatiosView, EffectiveRatiosView, PeerCapacityView,
    PledgeByRecipientView, PledgesView, RatioComplianceView, RatioViolationView, Tier,
    ViolationKind,
};
use elohim_views::replicates_dwelling::{ProviderRole, ReplicatesDwellingPayload};

/// Observation kind that carries per-node disk capacity (`disk_total_bytes`).
/// Declared in `elohim/sdk/domains/infrastructure/manifest.json`. Projected into
/// the `observations` table; read here for remote-peer raw capacity.
const SYSTEM_SAMPLE_KIND: &str = "infrastructure:system-sample";

/// The replicates-* action whose policy payload carries a storage pledge.
const REPLICATES_DWELLING_ACTION: &str = "replicates-dwelling";

pub fn compute_peer_capacity(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<PeerCapacityView, StorageError> {
    let total_raw_bytes = query_latest_total_raw_bytes(conn, peer_cid)?;
    let (pledged_dwelling, pledged_collective, pledged_commons, pledges_by_recipient) =
        aggregate_pledges_by_tier(conn, peer_cid)?;
    let unique_shard_bytes = compute_unique_shard_bytes(conn, peer_cid)?;
    let provenance = constitutional_ratio_registry::effective_ratios();
    let effective = provenance.ratios;
    let total_pledged = pledged_dwelling + pledged_collective + pledged_commons;

    let pledges = PledgesView {
        dwelling_bytes: pledged_dwelling,
        collective_bytes: pledged_collective,
        commons_bytes: pledged_commons,
        total_pledged_bytes: total_pledged,
        pledges_by_recipient,
    };

    let actually_held = ActuallyHeldView {
        unique_shard_bytes,
        free_bytes_remaining: total_raw_bytes as i64 - unique_shard_bytes as i64,
        fragmentation_estimate: 0.0,
    };

    let total_for_pct = total_raw_bytes.max(1);
    let current_dwelling_pct = ((pledged_dwelling * 100) / total_for_pct) as u8;
    let current_collective_pct = ((pledged_collective * 100) / total_for_pct) as u8;
    let current_commons_pct = ((pledged_commons * 100) / total_for_pct) as u8;
    let current_free_pct = 100u8
        .saturating_sub(current_dwelling_pct)
        .saturating_sub(current_collective_pct)
        .saturating_sub(current_commons_pct);

    let mut violations = Vec::new();
    if current_dwelling_pct > effective.dwelling_pct {
        violations.push(RatioViolationView {
            tier: Tier::Dwelling,
            violation_kind: ViolationKind::AboveCeiling,
            current_pct: current_dwelling_pct as i32,
            bound_pct: effective.dwelling_pct as i32,
        });
    }
    if current_free_pct < constitutional_ratio_registry::FREE_MIN_FLOOR_PCT {
        violations.push(RatioViolationView {
            tier: Tier::Free,
            violation_kind: ViolationKind::BelowFloor,
            current_pct: current_free_pct as i32,
            bound_pct: constitutional_ratio_registry::FREE_MIN_FLOOR_PCT as i32,
        });
    }

    let ratio_compliance = RatioComplianceView {
        effective_ratios: EffectiveRatiosView {
            commons_pct: effective.commons_pct as i32,
            dwelling_pct: effective.dwelling_pct as i32,
            collective_pct: effective.collective_pct as i32,
            free_pct: effective.free_pct as i32,
            manifest_cid: provenance.manifest_cid,
        },
        current_ratios: CurrentRatiosView {
            commons_pct: current_commons_pct as i32,
            dwelling_pct: current_dwelling_pct as i32,
            collective_pct: current_collective_pct as i32,
            free_pct: current_free_pct as i32,
        },
        compliant_with_donut: violations.is_empty(),
        violations,
    };

    Ok(PeerCapacityView {
        peer_cid: peer_cid.to_string(),
        computed_at: chrono::Utc::now().to_rfc3339(),
        total_raw_bytes,
        pledges,
        actually_held,
        ratio_compliance,
    })
}

// --- Data-source readers (spec §7.1 + §8.2 unique_shard_accounting). ---

/// Latest raw disk capacity for `peer_cid`, in bytes (TOTAL device capacity,
/// not free space — the view uses it as the ratio denominator and as the
/// minuend of `free_bytes_remaining`).
///
/// For the local node (`peer_cid == ELOHIM_LOCAL_PEER_ID`) we probe the
/// filesystem directly via `system_metrics::filesystem_capacity_bytes` — the
/// same authoritative source `cluster_view::build_local_slice` uses — so the
/// local capacity bar lights up immediately. For a remote peer we read the
/// latest gossiped `infrastructure:system-sample` observation's
/// `disk_total_bytes`, returning 0 until a sampler emits + gossips it.
fn query_latest_total_raw_bytes(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<u64, StorageError> {
    if is_local_peer(peer_cid) {
        let blob_path =
            std::env::var("ELOHIM_BLOB_PATH").unwrap_or_else(|_| "/data/blobs".to_string());
        if let Some(total) = crate::services::system_metrics::filesystem_capacity_bytes(
            std::path::Path::new(&blob_path),
        ) {
            return Ok(total);
        }
    }
    latest_system_sample_disk_total(conn, peer_cid)
}

/// True when `peer_cid` is this node's own libp2p peer id (`ELOHIM_LOCAL_PEER_ID`).
fn is_local_peer(peer_cid: &str) -> bool {
    std::env::var("ELOHIM_LOCAL_PEER_ID")
        .map(|local| local == peer_cid)
        .unwrap_or(false)
}

/// Read `disk_total_bytes` from the latest `infrastructure:system-sample`
/// observation authored by `peer_cid`. Returns 0 when no sample exists.
fn latest_system_sample_disk_total(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<u64, StorageError> {
    use crate::db::diesel_schema::observations::dsl as obs;
    let payload: Option<String> = obs::observations
        .filter(obs::observer_cid.eq(peer_cid))
        .filter(obs::observation_kind.eq(SYSTEM_SAMPLE_KIND))
        .order(obs::observed_at.desc())
        .select(obs::payload_json)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Database(e.to_string()))?;
    let Some(json) = payload else {
        return Ok(0);
    };
    let parsed: serde_json::Value =
        serde_json::from_str(&json).map_err(|e| StorageError::Internal(e.to_string()))?;
    Ok(parsed
        .get("disk_total_bytes")
        .and_then(serde_json::Value::as_u64)
        .unwrap_or(0))
}

/// Sum active replicates-* pledges authored by `peer_cid`, bucketed by tier,
/// with a per-recipient breakdown.
///
/// `capacity_bytes`, recipient hub, and provider role live in each commitment's
/// `metadata_json` (`ReplicatesDwellingPayload`), not in columns — `capacity_bytes`
/// is a `u64` there (exact), whereas `resource_quantity_value` is `f32` and would
/// silently lose precision above ~16 MB. Tier is derived from `provider_role`
/// (`steward_mutual` → Dwelling, `collective_steward` → Collective); no commons-tier
/// replication action is materialized yet. "Active" = state not `cancelled`/
/// `terminated`, mirroring the project-epr resolver precedent (rows are born
/// `proposed` and graduate to `active`).
fn aggregate_pledges_by_tier(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<(u64, u64, u64, Vec<PledgeByRecipientView>), StorageError> {
    use crate::db::diesel_schema::rea_commitments::dsl as rc;
    let rows: Vec<crate::db::models::ReaCommitment> = rc::rea_commitments
        .filter(rc::provider.eq(peer_cid))
        .filter(rc::action.eq(REPLICATES_DWELLING_ACTION))
        .filter(rc::state.ne("cancelled"))
        .filter(rc::state.ne("terminated"))
        .load(conn)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut pledged_dwelling = 0u64;
    let mut pledged_collective = 0u64;
    let pledged_commons = 0u64; // no commons-tier replication action materialized yet
    let mut by_recipient = Vec::with_capacity(rows.len());

    for row in rows {
        let Some(meta) = row.metadata_json.as_deref() else {
            continue;
        };
        let Ok(payload) = serde_json::from_str::<ReplicatesDwellingPayload>(meta) else {
            continue;
        };
        let (tier, role) = match payload.provider_role {
            ProviderRole::StewardMutual => (Tier::Dwelling, "steward_mutual"),
            ProviderRole::CollectiveSteward => (Tier::Collective, "collective_steward"),
        };
        match tier {
            Tier::Dwelling => {
                pledged_dwelling = pledged_dwelling.saturating_add(payload.capacity_bytes)
            }
            Tier::Collective => {
                pledged_collective = pledged_collective.saturating_add(payload.capacity_bytes)
            }
            _ => {}
        }
        by_recipient.push(PledgeByRecipientView {
            tier,
            recipient_hub_id: payload.recipient_dwelling_hub_id,
            commitment_cid: row.dht_anchor_hash.unwrap_or(row.id),
            capacity_bytes: payload.capacity_bytes,
            provider_role: Some(role.to_string()),
        });
    }

    Ok((
        pledged_dwelling,
        pledged_collective,
        pledged_commons,
        by_recipient,
    ))
}

/// Sum of distinct held-blob sizes for `peer_cid`.
///
/// `peer_blob_inventory` carries no sizes, so we join the peer's held
/// `blob_hash`es to `shard_manifests.total_size_bytes`, deduping on `blob_hash`
/// so a blob satisfying multiple commitments counts once (spec §7.1). Inventory
/// rows are PK'd on `(peer_id, blob_hash)`, so they are already distinct per peer.
/// Held blobs with no shard manifest (e.g. raw EPR blobs) contribute 0.
fn compute_unique_shard_bytes(
    conn: &mut SqliteConnection,
    peer_cid: &str,
) -> Result<u64, StorageError> {
    use crate::db::diesel_schema::peer_blob_inventory::dsl as pbi;
    use crate::db::diesel_schema::shard_manifests::dsl as sm;

    let held: Vec<String> = pbi::peer_blob_inventory
        .filter(pbi::peer_id.eq(peer_cid))
        .select(pbi::blob_hash)
        .load::<String>(conn)
        .map_err(|e| StorageError::Database(e.to_string()))?;
    if held.is_empty() {
        return Ok(0);
    }

    let sized: Vec<(String, i64)> = sm::shard_manifests
        .filter(sm::blob_hash.eq_any(&held))
        .select((sm::blob_hash, sm::total_size_bytes))
        .load::<(String, i64)>(conn)
        .map_err(|e| StorageError::Database(e.to_string()))?;

    let mut seen = std::collections::HashSet::new();
    let mut total = 0u64;
    for (hash, size) in sized {
        if seen.insert(hash) {
            total = total.saturating_add(size.max(0) as u64);
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::{
        NewObservationRow, NewPeerBlobInventoryRow, NewReaCommitment, NewShardManifest,
    };
    use crate::test_util::test_pool;
    use elohim_views::replicates_dwelling::{
        ProviderRole, RatioAttestation, ReplicatesDwellingPayload, ScopeFilter,
    };

    // --- insert helpers -----------------------------------------------------

    fn insert_system_sample(
        conn: &mut SqliteConnection,
        observer_cid: &str,
        observed_at: i64,
        disk_total_bytes: u64,
    ) {
        use crate::db::diesel_schema::observations;
        let payload = format!(
            r#"{{"cpu_pct":0.0,"mem_used_bytes":0,"disk_used_bytes":0,"disk_total_bytes":{disk_total_bytes}}}"#
        );
        let row = NewObservationRow {
            observer_cid,
            log_cid: "log:test",
            log_offset: observed_at,
            observed_at,
            seq: observed_at,
            observation_kind: SYSTEM_SAMPLE_KIND,
            subject_cid: None,
            subject_kind: None,
            payload_json: &payload,
            observer_household_cid: None,
            observer_collective_cid: None,
            observer_region: None,
            observer_archetype: None,
            observer_compute_class: None,
            signature_b64: "sig",
        };
        diesel::insert_into(observations::table)
            .values(&row)
            .execute(conn)
            .expect("insert observation");
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_replicates_dwelling(
        conn: &mut SqliteConnection,
        id: &str,
        provider: &str,
        state: &str,
        recipient_hub_id: &str,
        provider_role: ProviderRole,
        capacity_bytes: u64,
    ) {
        use crate::db::diesel_schema::rea_commitments;
        let payload = ReplicatesDwellingPayload {
            action: REPLICATES_DWELLING_ACTION.to_string(),
            provider_dwelling_hub_id: "hub:provider".to_string(),
            recipient_dwelling_hub_id: recipient_hub_id.to_string(),
            provider_role,
            via_collective_hub_id: None,
            capacity_bytes,
            scope_filter: ScopeFilter::default(),
            valid_from: "2026-01-01T00:00:00Z".to_string(),
            valid_until: "2027-01-01T00:00:00Z".to_string(),
            grace_period_days: 14,
            rotation_ttl_days: 90,
            ratio_attestation: RatioAttestation {
                commons_pct: 10,
                dwelling_pct: 60,
                collective_pct: 20,
                free_pct: 10,
                effective_ratio_cid: "cid:ratio".to_string(),
            },
        };
        let metadata = serde_json::to_string(&payload).unwrap();
        let row = NewReaCommitment {
            id,
            h_app_id: "lamad",
            action: REPLICATES_DWELLING_ACTION,
            provider,
            receiver: "agent:recipient",
            resource_conforms_to: None,
            resource_classified_as: None,
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: None,
            medium_of_exchange_id: None,
            state,
            finished: 0,
            note: None,
            metadata_json: Some(&metadata),
            dht_anchor_hash: None,
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .expect("insert commitment");
    }

    fn insert_inventory(conn: &mut SqliteConnection, peer_id: &str, blob_hash: &str) {
        use crate::db::diesel_schema::peer_blob_inventory;
        let row = NewPeerBlobInventoryRow {
            peer_id: peer_id.to_string(),
            blob_hash: blob_hash.to_string(),
            last_seen_at: "2026-05-29T00:00:00Z".to_string(),
            source: "self-custody".to_string(),
            sequence: 1,
            blake3_hash: None,
        };
        diesel::insert_into(peer_blob_inventory::table)
            .values(&row)
            .execute(conn)
            .expect("insert inventory");
    }

    fn insert_manifest(
        conn: &mut SqliteConnection,
        content_id: &str,
        blob_hash: &str,
        total_size_bytes: i64,
    ) {
        use crate::db::diesel_schema::shard_manifests;
        let row = NewShardManifest {
            content_id,
            h_app_id: "lamad",
            blob_hash,
            blob_cid: None,
            encoding: "rs",
            data_shard_count: 4,
            parity_shard_count: 3,
            shard_hashes_json: "[]",
            total_size_bytes,
            shard_size_bytes: total_size_bytes / 4,
            mime_type: "application/octet-stream",
            reach: "commons",
        };
        diesel::insert_into(shard_manifests::table)
            .values(&row)
            .execute(conn)
            .expect("insert manifest");
    }

    // --- view-shape regressions (unchanged behavior for empty peers) --------

    #[test]
    fn empty_peer_returns_zero_capacity() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_peer_capacity(&mut conn, "peer:fresh").unwrap();
        assert_eq!(view.peer_cid, "peer:fresh");
        assert_eq!(view.total_raw_bytes, 0);
        assert_eq!(view.pledges.total_pledged_bytes, 0);
        assert_eq!(view.actually_held.unique_shard_bytes, 0);
    }

    #[test]
    fn ratio_compliance_reflects_effective_ratios() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let view = compute_peer_capacity(&mut conn, "peer:test").unwrap();
        let r = constitutional_ratio_registry::effective_ratios().ratios;
        assert_eq!(
            view.ratio_compliance.effective_ratios.commons_pct as u8,
            r.commons_pct
        );
        assert_eq!(
            view.ratio_compliance.effective_ratios.dwelling_pct as u8,
            r.dwelling_pct
        );
    }

    // --- reader 1: total_raw_bytes from latest system-sample observation ----

    #[test]
    fn total_raw_bytes_reads_latest_system_sample_disk_total() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_system_sample(&mut conn, "peer:remote-cap", 1_000, 50_000_000_000);
        insert_system_sample(&mut conn, "peer:remote-cap", 2_000, 60_000_000_000);
        let total = query_latest_total_raw_bytes(&mut conn, "peer:remote-cap").unwrap();
        assert_eq!(
            total, 60_000_000_000,
            "latest sample's disk_total_bytes wins"
        );
    }

    // --- reader 2: pledge aggregation from metadata_json --------------------

    #[test]
    fn aggregate_pledges_sums_dwelling_from_metadata_json() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_replicates_dwelling(
            &mut conn,
            "comm-1",
            "peer:prov",
            "active",
            "hub:B",
            ProviderRole::StewardMutual,
            5_000_000_000,
        );
        // cancelled pledge must be excluded
        insert_replicates_dwelling(
            &mut conn,
            "comm-x",
            "peer:prov",
            "cancelled",
            "hub:C",
            ProviderRole::StewardMutual,
            9_000_000_000,
        );
        let (dwelling, collective, commons, by_rcpt) =
            aggregate_pledges_by_tier(&mut conn, "peer:prov").unwrap();
        assert_eq!(dwelling, 5_000_000_000);
        assert_eq!(collective, 0);
        assert_eq!(commons, 0);
        assert_eq!(by_rcpt.len(), 1);
        assert_eq!(by_rcpt[0].tier, Tier::Dwelling);
        assert_eq!(by_rcpt[0].recipient_hub_id, "hub:B");
        assert_eq!(by_rcpt[0].commitment_cid, "comm-1");
        assert_eq!(by_rcpt[0].capacity_bytes, 5_000_000_000);
    }

    #[test]
    fn aggregate_pledges_maps_collective_steward_to_collective_tier() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_replicates_dwelling(
            &mut conn,
            "comm-c",
            "peer:prov",
            "active",
            "hub:K",
            ProviderRole::CollectiveSteward,
            3_000_000_000,
        );
        let (dwelling, collective, _commons, by_rcpt) =
            aggregate_pledges_by_tier(&mut conn, "peer:prov").unwrap();
        assert_eq!(dwelling, 0);
        assert_eq!(collective, 3_000_000_000);
        assert_eq!(by_rcpt[0].tier, Tier::Collective);
    }

    // --- reader 3: unique held bytes via shard_manifests join ---------------

    #[test]
    fn unique_shard_bytes_sums_held_via_shard_manifests_deduped() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_inventory(&mut conn, "peer:hold", "b1");
        insert_inventory(&mut conn, "peer:hold", "b2");
        insert_inventory(&mut conn, "peer:other", "b3");
        insert_manifest(&mut conn, "c1", "b1", 100);
        insert_manifest(&mut conn, "c1-alt", "b1", 100); // same blob_hash -> counted once
        insert_manifest(&mut conn, "c2", "b2", 200);
        insert_manifest(&mut conn, "c3", "b3", 999); // other peer's blob -> excluded
        let held = compute_unique_shard_bytes(&mut conn, "peer:hold").unwrap();
        assert_eq!(held, 300);
    }

    #[test]
    fn unique_shard_bytes_ignores_held_blobs_without_manifest() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_inventory(&mut conn, "peer:hold", "orphan");
        let held = compute_unique_shard_bytes(&mut conn, "peer:hold").unwrap();
        assert_eq!(held, 0);
    }

    // --- end-to-end: full view assembled from real sources ------------------

    #[test]
    fn compute_peer_capacity_assembles_real_values() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_system_sample(&mut conn, "peer:full", 1_000, 100_000_000_000);
        insert_replicates_dwelling(
            &mut conn,
            "comm-1",
            "peer:full",
            "active",
            "hub:B",
            ProviderRole::StewardMutual,
            5_000_000_000,
        );
        insert_inventory(&mut conn, "peer:full", "b1");
        insert_manifest(&mut conn, "c1", "b1", 2_000_000_000);
        let view = compute_peer_capacity(&mut conn, "peer:full").unwrap();
        assert_eq!(view.total_raw_bytes, 100_000_000_000);
        assert_eq!(view.pledges.dwelling_bytes, 5_000_000_000);
        assert_eq!(view.pledges.total_pledged_bytes, 5_000_000_000);
        assert_eq!(view.actually_held.unique_shard_bytes, 2_000_000_000);
        assert_eq!(view.actually_held.free_bytes_remaining, 98_000_000_000);
    }
}
