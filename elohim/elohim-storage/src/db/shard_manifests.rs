//! Shard manifest CRUD operations using Diesel.
//!
//! Shard manifests track the encoding state for each content item —
//! what erasure coding strategy was used, the resulting shard hashes,
//! and sizes. This is per-peer local state (Category C), not DHT-notarized.

use diesel::prelude::*;

use super::diesel_schema::shard_manifests;
use super::models::{NewShardManifest, ShardManifestRow};
use crate::StorageError;

pub fn upsert_manifest(
    conn: &mut SqliteConnection,
    manifest: &NewShardManifest,
) -> Result<ShardManifestRow, StorageError> {
    diesel::replace_into(shard_manifests::table)
        .values(manifest)
        .execute(conn)?;

    shard_manifests::table
        .filter(shard_manifests::content_id.eq(manifest.content_id))
        .filter(shard_manifests::h_app_id.eq(manifest.h_app_id))
        .first(conn)
        .map_err(StorageError::from)
}

pub fn get_manifest(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    content_id: &str,
) -> Result<Option<ShardManifestRow>, StorageError> {
    shard_manifests::table
        .filter(shard_manifests::content_id.eq(content_id))
        .filter(shard_manifests::h_app_id.eq(h_app_id))
        .first(conn)
        .optional()
        .map_err(StorageError::from)
}

pub fn list_manifests_by_encoding(
    conn: &mut SqliteConnection,
    h_app_id: &str,
    encoding: &str,
) -> Result<Vec<ShardManifestRow>, StorageError> {
    shard_manifests::table
        .filter(shard_manifests::h_app_id.eq(h_app_id))
        .filter(shard_manifests::encoding.eq(encoding))
        .load(conn)
        .map_err(StorageError::from)
}
