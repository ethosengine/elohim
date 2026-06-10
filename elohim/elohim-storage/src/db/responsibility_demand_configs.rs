//! Responsibility demand config CRUD operations using Diesel with app scoping.
//!
//! The config table stores curve parameters per governance layer. Different
//! communities can have different curves based on their social contract health
//! (Robeyns capability insight: obligations scale with accumulated power).
//!
//! Category C (operational): one row per governance layer per app. Not notarized
//! on the DHT — this is operator/council configuration, not agent-centric data.

use diesel::prelude::*;
use uuid::Uuid;

use super::context::AppContext;
use super::diesel_schema::responsibility_demand_configs;
use super::models::{NewResponsibilityDemandConfig, ResponsibilityDemandConfig};
use crate::error::StorageError;

// ============================================================================
// Read Operations
// ============================================================================

/// Find the demand curve config for a specific governance layer — scoped by app.
///
/// Returns `None` if no config has been set for this layer yet (callers should
/// fall back to protocol defaults).
pub fn get_config_for_layer(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    governance_layer: &str,
) -> Result<Option<ResponsibilityDemandConfig>, StorageError> {
    responsibility_demand_configs::table
        .filter(responsibility_demand_configs::h_app_id.eq(&ctx.h_app_id))
        .filter(responsibility_demand_configs::governance_layer.eq(governance_layer))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List all demand curve configs for this app — one per governance layer.
pub fn get_all_configs(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<Vec<ResponsibilityDemandConfig>, StorageError> {
    responsibility_demand_configs::table
        .filter(responsibility_demand_configs::h_app_id.eq(&ctx.h_app_id))
        .order(responsibility_demand_configs::governance_layer.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Write Operations
// ============================================================================

/// Insert a new responsibility demand config — scoped by app.
///
/// The UNIQUE(h_app_id, governance_layer) constraint means this will fail if a
/// config already exists for this layer. Use an upsert pattern at the HTTP layer
/// if update semantics are needed.
///
/// The `id` field on `new` is used as-is if non-empty; a UUID is generated otherwise.
pub fn create_config(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    new: NewResponsibilityDemandConfig<'_>,
) -> Result<ResponsibilityDemandConfig, StorageError> {
    let generated_id;
    let id = if new.id.is_empty() {
        generated_id = Uuid::new_v4().to_string();
        generated_id.as_str()
    } else {
        new.id
    };

    let record = NewResponsibilityDemandConfig {
        id,
        h_app_id: &ctx.h_app_id,
        governance_layer: new.governance_layer,
        dignity_floor: new.dignity_floor,
        median_estimate: new.median_estimate,
        soft_ceiling_multiplier: new.soft_ceiling_multiplier,
        hard_ceiling_multiplier: new.hard_ceiling_multiplier,
        social_contract_health: new.social_contract_health,
        enforcement_active: new.enforcement_active,
        ratified_by: new.ratified_by,
        ratified_at: new.ratified_at,
        dht_anchor_hash: new.dht_anchor_hash,
    };

    diesel::insert_into(responsibility_demand_configs::table)
        .values(&record)
        .execute(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to insert responsibility demand config: {}",
                e
            ))
        })?;

    responsibility_demand_configs::table
        .filter(responsibility_demand_configs::id.eq(id))
        .first(conn)
        .map_err(|e| {
            StorageError::Internal(format!(
                "Failed to fetch inserted responsibility demand config: {}",
                e
            ))
        })
}

/// The dead-seam writeback (spec §1/§11 land 6): a PASSED ratify-limit-gradient
/// tally stamps the projection row. Updates the (h_app_id, governance_layer)
/// row's ratification columns + the ratified dignity_floor.
///
/// Trigger shape (b): called from `attestation_projector::handle_content_signal`
/// after `tally_projector::recompute` returns `computed_status == "reached-quorum"`
/// for a parent whose `governance_kind == "governance-action:ratify-limit-gradient"`.
/// The parent's `parameters_json` carries `governance_layer` and `dignity_floor`;
/// those are parsed at the call site and forwarded here. A parse failure at the
/// call site logs WARN and skips (fail-soft, mirrors CommitmentCommitted arm).
///
/// No new signal pipeline is required — the existing attestation_projector fan-out
/// already runs for all `attestation:*` and `governance-action:*` content_types.
pub fn apply_ratification(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    governance_layer: &str,
    ratified_by: &str,
    dht_anchor_hash: &str,
    dignity_floor: f32,
    ratified_at: &str,
) -> QueryResult<usize> {
    diesel::update(
        responsibility_demand_configs::table
            .filter(responsibility_demand_configs::h_app_id.eq(h_app_id))
            .filter(responsibility_demand_configs::governance_layer.eq(governance_layer)),
    )
    .set((
        responsibility_demand_configs::ratified_by.eq(ratified_by),
        responsibility_demand_configs::ratified_at.eq(ratified_at),
        responsibility_demand_configs::dht_anchor_hash.eq(dht_anchor_hash),
        responsibility_demand_configs::dignity_floor.eq(dignity_floor),
        responsibility_demand_configs::updated_at.eq(ratified_at),
    ))
    .execute(conn)
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::test_pool;

    /// Seed a config row via create_config, call apply_ratification, read back
    /// via get_config_for_layer, assert ratified columns + dignity_floor update.
    #[test]
    fn apply_ratification_stamps_config_row() {
        let pool = test_pool();
        let mut conn = pool.get().expect("connection");

        let ctx = AppContext::new("shefa");

        // Seed a config row using the existing create fn.
        let new = NewResponsibilityDemandConfig {
            id: "test-config-001",
            h_app_id: "shefa",
            governance_layer: "community",
            dignity_floor: 100.0,
            median_estimate: 1000.0,
            soft_ceiling_multiplier: 10.0,
            hard_ceiling_multiplier: 20.0,
            social_contract_health: 0.5,
            enforcement_active: 1,
            ratified_by: None,
            ratified_at: None,
            dht_anchor_hash: None,
        };
        create_config(&mut conn, &ctx, new).expect("create_config failed");

        // Apply ratification writeback.
        let affected = apply_ratification(
            &mut conn,
            "shefa",
            "community",
            "uhCEk-ga-cid",
            "uhCEk-commitment-cid",
            75.0,
            "2026-06-10T04:00:00Z",
        )
        .expect("apply_ratification failed");
        assert_eq!(affected, 1, "expected exactly one row updated");

        // Read back and assert.
        let config = get_config_for_layer(&mut conn, &ctx, "community")
            .expect("get_config_for_layer failed")
            .expect("config row must exist");

        assert_eq!(
            config.ratified_by.as_deref(),
            Some("uhCEk-ga-cid"),
            "ratified_by must be the governance-action CID"
        );
        assert_eq!(
            config.dht_anchor_hash.as_deref(),
            Some("uhCEk-commitment-cid"),
            "dht_anchor_hash must be the commitment CID"
        );
        assert!(
            (config.dignity_floor - 75.0).abs() < f32::EPSILON,
            "dignity_floor must be updated to 75.0, got {}",
            config.dignity_floor
        );
        assert_eq!(
            config.ratified_at.as_deref(),
            Some("2026-06-10T04:00:00Z"),
            "ratified_at must be set"
        );
    }
}
