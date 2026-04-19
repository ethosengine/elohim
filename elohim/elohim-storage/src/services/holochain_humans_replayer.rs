//! Provides a snapshot of (humanId, householdId) pairs from current DHT humans
//! entries, for the household_backfill startup pass.

use crate::StorageError;

/// Returns a point-in-time snapshot of (human_id, household_id) pairs from
/// the DHT humans entries.  Tolerates DHT unavailability.
///
/// Placeholder implementation: returns an empty mapping until the DHT reader
/// hook lands.  The backfill is idempotent and a zero-length mapping is a
/// valid outcome — rows already populated remain correct.
pub async fn snapshot_household_ids<C>(
    _dht: &C,
) -> Result<Vec<(String, String)>, StorageError> {
    // Placeholder: returns empty until the DHT reader hook lands.
    // The backfill is idempotent and a zero-length mapping is a valid outcome.
    Ok(vec![])
}
