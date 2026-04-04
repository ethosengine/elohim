//! Shard location CRUD operations using Diesel.
//!
//! Shard locations track which peers hold which shards, with verification
//! timestamps. This is per-peer local state (Category C), rebuilt from
//! shard protocol ack events — not DHT-notarized.

use diesel::prelude::*;

use super::diesel_schema::shard_locations;
use super::models::{NewShardLocation, ShardLocationRow};
use crate::StorageError;

pub fn upsert_location(
    conn: &mut SqliteConnection,
    location: &NewShardLocation,
) -> Result<(), StorageError> {
    diesel::replace_into(shard_locations::table)
        .values(location)
        .execute(conn)?;
    Ok(())
}

pub fn get_locations_for_shard(
    conn: &mut SqliteConnection,
    shard_hash: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    shard_locations::table
        .filter(shard_locations::shard_hash.eq(shard_hash))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn get_locations_for_peer(
    conn: &mut SqliteConnection,
    peer_id: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    shard_locations::table
        .filter(shard_locations::peer_id.eq(peer_id))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn get_locations_for_content(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    content_id: &str,
) -> Result<Vec<ShardLocationRow>, StorageError> {
    use super::diesel_schema::shard_manifests;

    let manifest = shard_manifests::table
        .filter(shard_manifests::content_id.eq(content_id))
        .filter(shard_manifests::h_app_id.eq(h_app_id))
        .first::<super::models::ShardManifestRow>(conn)
        .optional()?;

    let Some(manifest) = manifest else {
        return Ok(vec![]);
    };

    let shard_hashes: Vec<String> =
        serde_json::from_str(&manifest.shard_hashes_json).unwrap_or_default();

    if shard_hashes.is_empty() {
        return Ok(vec![]);
    }

    shard_locations::table
        .filter(shard_locations::shard_hash.eq_any(&shard_hashes))
        .load(conn)
        .map_err(StorageError::from)
}

pub fn mark_lost(
    conn: &mut SqliteConnection,
    shard_hash: &str,
    peer_id: &str,
) -> Result<(), StorageError> {
    diesel::update(
        shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard_hash))
            .filter(shard_locations::peer_id.eq(peer_id)),
    )
    .set(shard_locations::status.eq("lost"))
    .execute(conn)?;
    Ok(())
}

pub fn update_verified(
    conn: &mut SqliteConnection,
    shard_hash: &str,
    peer_id: &str,
) -> Result<(), StorageError> {
    let now = chrono::Utc::now().to_rfc3339();
    diesel::update(
        shard_locations::table
            .filter(shard_locations::shard_hash.eq(shard_hash))
            .filter(shard_locations::peer_id.eq(peer_id)),
    )
    .set((
        shard_locations::status.eq("verified"),
        shard_locations::last_verified.eq(&now),
    ))
    .execute(conn)?;
    Ok(())
}
