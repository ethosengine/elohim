//! Shard manifest CRUD operations using Diesel.
//!
//! Shard manifests track the encoding state for each content item —
//! what erasure coding strategy was used, the resulting shard hashes,
//! and sizes. This is per-peer local state (Category C), not DHT-notarized.

use diesel::prelude::*;

use super::diesel_schema::shard_manifests;
use super::models::{NewShardManifest, ShardManifestRow};
use crate::StorageError;

/// Encode `data` into a shard manifest and upsert it for `(content_id, h_app_id)`.
///
/// Single source of truth for "blob bytes → persisted shard manifest". Both the
/// `POST /db/content` create path (`http.rs`) and the startup backfill
/// (`services::shard_manifest_backfill`) converge here so the encoding +
/// persistence shape never drifts between them.
///
/// `blob_cid` is the optional content-addressed CID captured on the content row;
/// `content_format` becomes the manifest mime_type; `reach` is recorded verbatim.
#[allow(clippy::too_many_arguments)]
pub fn record_manifest_from_bytes(
    conn: &mut SqliteConnection,
    content_id: &str,
    h_app_id: &str,
    blob_hash: &str,
    blob_cid: Option<&str>,
    data: &[u8],
    content_format: &str,
    reach: &str,
) -> Result<ShardManifestRow, StorageError> {
    let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());
    let manifest = encoder
        .create_manifest(data, content_format, reach)
        .map_err(StorageError::from)?;

    let shard_hashes_json = serde_json::to_string(&manifest.shard_hashes)?;

    let new_manifest = NewShardManifest {
        content_id,
        h_app_id,
        blob_hash,
        blob_cid,
        encoding: &manifest.encoding,
        data_shard_count: manifest.data_shards as i32,
        parity_shard_count: (manifest.total_shards - manifest.data_shards) as i32,
        shard_hashes_json: &shard_hashes_json,
        total_size_bytes: manifest.total_size as i64,
        shard_size_bytes: manifest.shard_size as i64,
        mime_type: &manifest.mime_type,
        reach,
    };

    upsert_manifest(conn, &new_manifest)
}

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
