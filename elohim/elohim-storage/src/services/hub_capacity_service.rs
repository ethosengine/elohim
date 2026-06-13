//! Computes HubCapacityView per spec §7.2. Hub is a *role* (per
//! project_hub_archetype_abstraction memory); substrate stays kind-agnostic.
//!
//! Wave 2 T3: `resolve_hub_members` now resolves the hub to its member DEVICES
//! via `stewarded_nodes.household_id`. `classify_hub` is derived from
//! `collectives.governance_layer` (NOT a string-prefix scheme). `display_label`
//! comes from `Collective.name`.
//!
//! ## Identity caveat (KNOWN LIMITATION — T3 scope)
//!
//! `compute_peer_capacity` uses the same `peer_cid` argument for BOTH:
//!   - raw-capacity (keyed on libp2p peer_id via `system_metrics.observer_cid`)
//!   - pledges       (keyed on agent_cid via `rea_commitments.provider`)
//!
//! Members returned by `resolve_hub_members` are `stewarded_nodes.id` values
//! (device/node IDs), which are the correct key for raw-capacity and held-byte
//! queries. However, pledge reads will return 0 for these node IDs unless the
//! node ID happens to equal the agent CID used as `rea_commitments.provider`.
//!
//! A clean mapping exists via `peer_identity_bindings` (peer_id → agent_cid),
//! but applying it would require either:
//!   a) splitting `compute_peer_capacity` into separate raw-cap vs pledge
//!      functions, or
//!   b) pre-resolving each node_id to its agent_cid and passing BOTH into an
//!      extended signature.
//!
//! This is deferred to a dedicated T3-followup item. The held + raw-capacity
//! bars (which light up today) aggregate correctly; pledges read 0 until the
//! mapping is wired.

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;

use crate::db::models::governance_layers;
use crate::error::StorageError;
use crate::services::hub_resolver;
use crate::services::peer_capacity_service::compute_peer_capacity;
use elohim_views::hub_capacity::{HubCapacityAggregate, HubCapacityView, HubKind};

pub fn compute_hub_capacity(
    conn: &mut SqliteConnection,
    hub_id: &str,
) -> Result<HubCapacityView, StorageError> {
    let member_peer_cids = resolve_hub_members(conn, hub_id)?;
    let (hub_kind, display_label) = classify_hub_with_label(conn, hub_id, &member_peer_cids)?;

    if member_peer_cids.is_empty() {
        return Ok(HubCapacityView {
            hub_id: hub_id.to_string(),
            hub_kind,
            display_label,
            member_device_count: 0,
            capacity: None,
        });
    }

    let mut capacity_aggregate = HubCapacityAggregate::default();
    for peer_cid in &member_peer_cids {
        let pv = compute_peer_capacity(conn, peer_cid)?;
        capacity_aggregate.total_raw_bytes += pv.total_raw_bytes;
        capacity_aggregate.pledges.dwelling_bytes += pv.pledges.dwelling_bytes;
        capacity_aggregate.pledges.collective_bytes += pv.pledges.collective_bytes;
        capacity_aggregate.pledges.commons_bytes += pv.pledges.commons_bytes;
        capacity_aggregate.pledges.total_pledged_bytes += pv.pledges.total_pledged_bytes;
        capacity_aggregate.actually_held.unique_shard_bytes += pv.actually_held.unique_shard_bytes;
        capacity_aggregate.actually_held.free_bytes_remaining +=
            pv.actually_held.free_bytes_remaining;
    }

    Ok(HubCapacityView {
        hub_id: hub_id.to_string(),
        hub_kind,
        display_label,
        member_device_count: member_peer_cids.len() as i32,
        capacity: Some(capacity_aggregate),
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal helpers
// ─────────────────────────────────────────────────────────────────────────────

/// Resolve the member device IDs for `hub_id`.
///
/// Members of a hub are its DEVICES — capacity lives on devices.
///
/// Steps:
/// 1. Normalize `hub_id` to a slug: if it looks like a `collective:{hash}` CID,
///    call `hub_resolver::cid_to_slug`; otherwise treat it as already a slug.
/// 2. Query `stewarded_nodes.household_id == slug` and return the device IDs
///    (i.e. `stewarded_nodes.id` values), which are the keys consumed by
///    `compute_peer_capacity`.
///
/// Returns an empty Vec when:
/// - A `collective:` CID cannot be resolved to a slug (hub unknown).
/// - No `stewarded_nodes` rows carry `household_id == slug`.
///
/// This is correct (not an error): an empty member list means the aggregate
/// is zero rather than an error.
fn resolve_hub_members(
    conn: &mut SqliteConnection,
    hub_id: &str,
) -> Result<Vec<String>, StorageError> {
    use crate::db::diesel_schema::stewarded_nodes as t;

    // Step 1: normalise hub_id → slug.
    let slug = if hub_id.starts_with("collective:") {
        match hub_resolver::cid_to_slug(conn, hub_id)? {
            Some(s) => s,
            // CID present but not yet projected → zero members, no error.
            None => return Ok(vec![]),
        }
    } else {
        // Treat as a slug / legacy string key directly.
        hub_id.to_string()
    };

    // Step 2: all devices registered under this household slug.
    let node_ids: Vec<String> = t::table
        .filter(t::household_id.eq(&slug))
        .select(t::id)
        .load::<String>(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "hub_capacity_service resolve_hub_members stewarded_nodes: {e}"
            ))
        })?;

    Ok(node_ids)
}

/// Classify the hub and return its display label in one pass (one collective
/// lookup shared across both outputs).
///
/// Classification rules (per HubKind doc in elohim-views/infrastructure.rs):
/// - `Dwelling`   → `collectives.governance_layer == 'family'`
/// - `Collective` → other governance_layer, with 2+ members
/// - `Computed`   → no collective row found, OR members ≤ 1
///
/// `display_label` is populated from `Collective.name` when the collective row
/// is found; `None` otherwise.
fn classify_hub_with_label(
    conn: &mut SqliteConnection,
    hub_id: &str,
    members: &[String],
) -> Result<(HubKind, Option<String>), StorageError> {
    use crate::db::diesel_schema::collectives;

    // Resolve hub_id → slug (same normalisation as resolve_hub_members).
    let slug: Option<String> = if hub_id.starts_with("collective:") {
        hub_resolver::cid_to_slug(conn, hub_id)?
    } else {
        Some(hub_id.to_string())
    };

    let slug = match slug {
        None => return Ok((HubKind::Computed, None)),
        Some(s) => s,
    };

    // Look up the collective row by slug.
    let row: Option<(String, String)> = collectives::table
        .filter(collectives::id.eq(&slug))
        .select((collectives::governance_layer, collectives::name))
        .first::<(String, String)>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Internal(format!(
                "hub_capacity_service classify_hub collectives lookup: {e}"
            ))
        })?;

    let (governance_layer, name) = match row {
        None => return Ok((HubKind::Computed, None)),
        Some(r) => r,
    };

    let kind = if governance_layer == governance_layers::FAMILY {
        HubKind::Dwelling
    } else if members.len() >= 2 {
        HubKind::Collective
    } else {
        HubKind::Computed
    };

    Ok((kind, Some(name)))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::collectives::{create_collective, CreateCollectiveInput};
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:hub_cap_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn ctx() -> crate::db::context::AppContext {
        crate::db::context::AppContext::new("test-app")
    }

    /// Seed a collective row (governance_layer defaults to 'family').
    fn seed_collective(
        conn: &mut SqliteConnection,
        slug: &str,
        name: &str,
        governance_layer: &str,
        cid: Option<&str>,
    ) {
        let app_ctx = ctx();
        let input = CreateCollectiveInput {
            id: slug.to_string(),
            name: name.to_string(),
            description: None,
            governance_layer: governance_layer.to_string(),
            constitutional_parent_id: None,
            reach: "private".to_string(),
            region: None,
            metadata_json: None,
            created_by: None,
        };
        create_collective(conn, &app_ctx, &input).expect("seed collective");

        if let Some(c) = cid {
            use crate::db::diesel_schema::collectives;
            diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq(&app_ctx.h_app_id))
                    .filter(collectives::id.eq(slug)),
            )
            .set(collectives::collective_cid.eq(Some(c)))
            .execute(conn)
            .expect("set collective_cid");
        }
    }

    /// Seed a stewarded_node row with household_id set.
    fn seed_device(conn: &mut SqliteConnection, node_id: &str, household_id: &str) {
        use crate::db::diesel_schema::stewarded_nodes as t;
        use crate::db::models::current_timestamp;

        let now = current_timestamp();
        diesel::insert_or_ignore_into(t::table)
            .values((
                t::id.eq(node_id),
                t::display_name.eq(node_id),
                t::claim_status.eq("claimed"),
                t::cpu_cores.eq(4),
                t::memory_gb.eq(8),
                t::storage_tb.eq(1.0_f64),
                t::bandwidth_mbps.eq(100),
                t::steward_tier.eq("caretaker"),
                t::custodian_opt_in.eq(1),
                t::h_app_id.eq("test-app"),
                t::household_id.eq(Some(household_id)),
                t::can_steward.eq(1),
                t::can_infer.eq(0),
                t::can_doorway.eq(0),
                t::created_at.eq(&now),
                t::updated_at.eq(&now),
            ))
            .execute(conn)
            .expect("seed device");
    }

    // ─── T3-1: resolve_hub_members via slug ───────────────────────────────────

    /// Hub referenced by slug → returns devices with household_id == slug.
    #[test]
    fn resolve_hub_members_by_slug_returns_devices() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", "Dowell Family", "family", None);
        seed_device(&mut conn, "node-alice", "family-dowell");
        seed_device(&mut conn, "node-bob", "family-dowell");

        let mut members = resolve_hub_members(&mut conn, "family-dowell").expect("no error");
        members.sort();

        assert_eq!(members, vec!["node-alice", "node-bob"]);
    }

    // ─── T3-2: resolve_hub_members via collective: CID ────────────────────────

    /// Hub referenced by `collective:` CID → normalised to slug, devices returned.
    #[test]
    fn resolve_hub_members_by_cid_normalises_to_slug() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            "Dowell Family",
            "family",
            Some("collective:uhCkkT3Test001"),
        );
        seed_device(&mut conn, "node-alice", "family-dowell");

        let members =
            resolve_hub_members(&mut conn, "collective:uhCkkT3Test001").expect("no error");
        assert_eq!(members, vec!["node-alice"]);
    }

    // ─── T3-3: unknown CID → empty (no error) ─────────────────────────────────

    #[test]
    fn resolve_hub_members_unknown_cid_returns_empty() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let members = resolve_hub_members(&mut conn, "collective:nobody").expect("no error");
        assert!(members.is_empty(), "unknown CID → empty Vec, not an error");
    }

    // ─── T3-4: classify_hub — family governance_layer → Dwelling ──────────────

    #[test]
    fn classify_hub_family_governance_layer_is_dwelling() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            "Dowell Family",
            "family",
            Some("collective:uhCkkT3Test002"),
        );
        seed_device(&mut conn, "node-alice", "family-dowell");

        let (kind, label) =
            classify_hub_with_label(&mut conn, "family-dowell", &["node-alice".to_string()])
                .expect("no error");

        assert_eq!(
            kind,
            HubKind::Dwelling,
            "family governance_layer → Dwelling"
        );
        assert_eq!(label.as_deref(), Some("Dowell Family"), "label from name");
    }

    // ─── T3-5: classify_hub — non-family with members → Collective ────────────

    #[test]
    fn classify_hub_community_governance_with_members_is_collective() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "bay-runners",
            "Bay Area Dawn Runners",
            "community",
            None,
        );
        seed_device(&mut conn, "node-runner-a", "bay-runners");
        seed_device(&mut conn, "node-runner-b", "bay-runners");

        let members = vec!["node-runner-a".to_string(), "node-runner-b".to_string()];
        let (kind, label) =
            classify_hub_with_label(&mut conn, "bay-runners", &members).expect("no error");

        assert_eq!(
            kind,
            HubKind::Collective,
            "non-family with ≥2 members → Collective"
        );
        assert_eq!(
            label.as_deref(),
            Some("Bay Area Dawn Runners"),
            "label from collective name"
        );
    }

    // ─── T3-6: classify_hub — no collective row → Computed ───────────────────

    #[test]
    fn classify_hub_no_collective_row_is_computed() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // No collective row seeded — hub_id is an arbitrary peer id.
        let (kind, label) =
            classify_hub_with_label(&mut conn, "peer:solo-device", &[]).expect("no error");

        assert_eq!(kind, HubKind::Computed, "no collective row → Computed");
        assert!(label.is_none(), "no label when collective absent");
    }

    // ─── T3-7: classify_hub by CID → Dwelling ────────────────────────────────

    #[test]
    fn classify_hub_by_cid_resolves_to_dwelling() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            "Dowell Family",
            "family",
            Some("collective:uhCkkT3Test003"),
        );
        seed_device(&mut conn, "node-alice", "family-dowell");

        let (kind, _) = classify_hub_with_label(
            &mut conn,
            "collective:uhCkkT3Test003",
            &["node-alice".to_string()],
        )
        .expect("no error");

        assert_eq!(
            kind,
            HubKind::Dwelling,
            "CID-referenced family hub → Dwelling"
        );
    }

    // ─── T3-8: aggregate via compute_hub_capacity — slug path ─────────────────

    /// Full integration: compute_hub_capacity via slug returns non-None capacity
    /// with the correct member_device_count.
    #[test]
    fn compute_hub_capacity_aggregate_slug_path() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", "Dowell Family", "family", None);
        seed_device(&mut conn, "node-alice", "family-dowell");
        seed_device(&mut conn, "node-bob", "family-dowell");

        let view = compute_hub_capacity(&mut conn, "family-dowell").expect("no error");

        assert_eq!(view.hub_kind, HubKind::Dwelling);
        assert_eq!(view.member_device_count, 2);
        assert!(view.capacity.is_some(), "capacity aggregate present");
        assert_eq!(view.display_label.as_deref(), Some("Dowell Family"));
    }

    // ─── T3-9: aggregate via compute_hub_capacity — CID path ──────────────────

    #[test]
    fn compute_hub_capacity_aggregate_cid_path() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            "Dowell Family",
            "family",
            Some("collective:uhCkkT3Test004"),
        );
        seed_device(&mut conn, "node-alice", "family-dowell");

        let view = compute_hub_capacity(&mut conn, "collective:uhCkkT3Test004").expect("no error");

        assert_eq!(view.hub_kind, HubKind::Dwelling);
        assert_eq!(view.member_device_count, 1);
        assert!(view.capacity.is_some());
        assert_eq!(view.display_label.as_deref(), Some("Dowell Family"));
    }

    // ─── T3-10: unknown hub → zero aggregate, no error ────────────────────────

    #[test]
    fn compute_hub_capacity_unknown_hub_zeroed_aggregate() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let view = compute_hub_capacity(&mut conn, "collective:nobody").expect("no error");

        assert_eq!(view.member_device_count, 0);
        assert!(view.capacity.is_none(), "no capacity when no members");
    }

    // ─── T3-11: Backward-compat — existing single-device peer test ───────────
    //
    // The old stub returned `vec![hub_id]` for any input, so a peer_id like
    // "peer:solo" would count itself as its own member and yield Computed/1.
    // The new code correctly returns an EMPTY member list for "peer:solo" (no
    // collective row, no stewarded_nodes row with that household_id), so the
    // aggregate is now 0/None — which is substrate-honest. The old stub behaviour
    // was the stub-artifact, not contract. This test reflects the correct
    // post-T3 behaviour.
    #[test]
    fn single_device_peer_id_no_collective_row_returns_empty() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // No collective, no stewarded_node with household_id == "peer:solo".
        let view = compute_hub_capacity(&mut conn, "peer:solo").expect("no error");
        // After T3: no collective row → Computed, 0 members.
        assert_eq!(view.hub_kind, HubKind::Computed);
        assert_eq!(view.member_device_count, 0);
        assert!(view.capacity.is_none());
    }

    // ─── T3-12: dwelling: prefix is now handled by governance_layer, not prefix ─

    /// The old classify_hub classified "dwelling:X" via string prefix as Dwelling.
    /// The new code ignores prefixes entirely — classification comes from the DB.
    /// A "dwelling:X" hub with no collective row is Computed (substrate-honest).
    #[test]
    fn dwelling_prefix_without_collective_row_is_computed() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        // No collective row seeded for "dwelling:smith-family".
        let view = compute_hub_capacity(&mut conn, "dwelling:smith-family").expect("no error");
        assert_eq!(view.hub_kind, HubKind::Computed);
    }
}
