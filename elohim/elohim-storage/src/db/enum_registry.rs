//! Enum Registry CRUD operations
//!
//! Extensible vocabulary for protocol enums (Category C operational).
//! Seeded from JSON Schema on startup, extended via API.

use diesel::prelude::*;
use tracing::debug;

use super::context::AppContext;
use super::diesel_schema::enum_registry;
use super::models::{current_timestamp, EnumRegistryEntry, NewEnumRegistryEntry};
use crate::error::StorageError;

/// List all registered values for a given enum name
pub fn list_enum_values(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    enum_name: &str,
) -> Result<Vec<EnumRegistryEntry>, StorageError> {
    debug!(enum_name, "Listing enum registry values");

    enum_registry::table
        .filter(enum_registry::app_id.eq(&ctx.app_id))
        .filter(enum_registry::enum_name.eq(enum_name))
        .order(enum_registry::id.asc())
        .load::<EnumRegistryEntry>(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to list enum values: {}", e)))
}

/// Register a new enum value (idempotent — ignores duplicates)
pub fn register_enum_value(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    enum_name: &str,
    enum_value: &str,
    tier: &str,
    added_by: Option<&str>,
) -> Result<(), StorageError> {
    debug!(enum_name, enum_value, tier, "Registering enum value");

    let new = NewEnumRegistryEntry {
        app_id: ctx.app_id.clone(),
        enum_name: enum_name.to_string(),
        enum_value: enum_value.to_string(),
        tier: tier.to_string(),
        added_by: added_by.map(|s| s.to_string()),
        created_at: current_timestamp(),
    };

    diesel::insert_or_ignore_into(enum_registry::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to register enum value: {}", e)))?;

    Ok(())
}
