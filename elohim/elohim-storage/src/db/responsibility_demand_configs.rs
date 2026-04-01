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
