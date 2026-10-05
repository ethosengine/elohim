//! The release ledger (Category C, local): which releases of each followed
//! channel this peer has verified, in the order it first saw them.
//!
//! A channel's content row holds only its current head, so this is the only
//! place a peer can read "the ten latest releases". The release retention pass
//! (`services::release_adoption::retention`) is its one reader.
//!
//! Also here: the blob bookkeeping a peer drops when it lets a release's bytes
//! go ([`plan_forget_blob`], [`forget_blob_rows`]), and the two reads that say
//! a blob is still in use ([`content_rows_naming`], [`live_custody_pledges_naming`]).

use std::collections::HashSet;

use diesel::prelude::*;
use diesel::sqlite::SqliteConnection;
use serde::{Deserialize, Serialize};

use super::diesel_schema::{
    content, peer_blob_inventory, rea_commitments, release_ledger, shard_locations, shard_manifests,
};
use super::models::current_timestamp;
use crate::error::StorageError;

/// One artifact of a recorded release.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LedgerArtifact {
    /// Hex sha2-256 of the bytes.
    pub sha256: String,
    pub blob_cid: String,
    pub bytes: u64,
    pub filename: String,
}

impl LedgerArtifact {
    /// The blob store's own spelling of this artifact's hash.
    pub fn blob_hash(&self) -> String {
        format!("sha256-{}", self.sha256.to_ascii_lowercase())
    }
}

/// One recorded release.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerRelease {
    pub seq: i32,
    pub channel_id: String,
    pub release_cid: String,
    pub artifact_class: String,
    pub artifacts: Vec<LedgerArtifact>,
    pub first_seen_at: String,
}

#[derive(Queryable)]
struct LedgerRow {
    seq: i32,
    channel_id: String,
    release_cid: String,
    artifact_class: String,
    artifacts_json: String,
    first_seen_at: String,
}

impl From<LedgerRow> for LedgerRelease {
    fn from(row: LedgerRow) -> Self {
        Self {
            seq: row.seq,
            channel_id: row.channel_id,
            release_cid: row.release_cid,
            artifact_class: row.artifact_class,
            // A row whose artifact list cannot be read names no bytes, so the
            // retention pass can release nothing on its account.
            artifacts: serde_json::from_str(&row.artifacts_json).unwrap_or_default(),
            first_seen_at: row.first_seen_at,
        }
    }
}

/// Record a verified release. A release already recorded keeps its place:
/// first-seen order never moves. Returns whether a row was written.
pub fn record_release(
    conn: &mut SqliteConnection,
    channel_id: &str,
    release_cid: &str,
    artifact_class: &str,
    artifacts: &[LedgerArtifact],
) -> Result<bool, StorageError> {
    let artifacts_json = serde_json::to_string(artifacts)?;
    let written = diesel::insert_or_ignore_into(release_ledger::table)
        .values((
            release_ledger::channel_id.eq(channel_id),
            release_ledger::release_cid.eq(release_cid),
            release_ledger::artifact_class.eq(artifact_class),
            release_ledger::artifacts_json.eq(artifacts_json),
            release_ledger::first_seen_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Database(format!("record_release: {e}")))?;
    Ok(written > 0)
}

/// Every recorded release, newest first within each channel.
pub fn list_releases(conn: &mut SqliteConnection) -> Result<Vec<LedgerRelease>, StorageError> {
    let rows: Vec<LedgerRow> = release_ledger::table
        .order((release_ledger::channel_id.asc(), release_ledger::seq.desc()))
        .load(conn)
        .map_err(|e| StorageError::Database(format!("list_releases: {e}")))?;
    Ok(rows.into_iter().map(LedgerRelease::from).collect())
}

/// Drop one release from the ledger, once its bytes have been let go.
pub fn remove_release(conn: &mut SqliteConnection, seq: i32) -> Result<(), StorageError> {
    diesel::delete(release_ledger::table.filter(release_ledger::seq.eq(seq)))
        .execute(conn)
        .map(|_| ())
        .map_err(|e| StorageError::Database(format!("remove_release: {e}")))
}

/// How many content rows name `blob_hash` as something they serve: the browser
/// bundle, the server bundle, the blob's CID, or anywhere in their metadata
/// (which is where a channel row carries its current release manifest).
///
/// Deliberately across every app scope and deliberately loose: a hash that
/// appears anywhere in any row counts as in use.
pub fn content_rows_naming(
    conn: &mut SqliteConnection,
    artifact: &LedgerArtifact,
) -> Result<i64, StorageError> {
    let blob_hash = artifact.blob_hash();
    let hex_anywhere = format!("%{}%", artifact.sha256.to_ascii_lowercase());
    let cid_anywhere = format!("%{}%", artifact.blob_cid);
    let mut query = content::table
        .filter(
            content::blob_hash
                .eq(&blob_hash)
                .or(content::server_blob_hash.eq(&blob_hash))
                .or(content::metadata_json.like(&hex_anywhere)),
        )
        .into_boxed();
    if !artifact.blob_cid.is_empty() {
        query = query
            .or_filter(content::blob_cid.eq(&artifact.blob_cid))
            .or_filter(content::metadata_json.like(cid_anywhere));
    }
    query
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Database(format!("content_rows_naming: {e}")))
}

/// How many custody commitments still oblige someone to hold `blob_hash`.
/// While one stands, the custody pass would fetch the bytes straight back.
pub fn live_custody_pledges_naming(
    conn: &mut SqliteConnection,
    blob_hash: &str,
) -> Result<i64, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::action.eq("custody-blob"))
        .filter(rea_commitments::resource_classified_as.eq(blob_hash))
        .filter(rea_commitments::state.ne_all(super::models::commitment_withdrawn_states::ALL))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Database(format!("live_custody_pledges_naming: {e}")))
}

/// What letting go of one blob touches.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForgetBlobPlan {
    pub blob_hash: String,
    /// Blob-store files to delete: the blob itself and every shard of it that
    /// no other blob's manifest names.
    pub files: Vec<String>,
    /// The blob's BLAKE3 alias in the iroh store, when one was recorded.
    pub blake3: Option<String>,
}

/// Read what letting go of `blob_hash` touches, changing nothing.
///
/// A shard hash is content-addressed, so another blob may legitimately share
/// one. A shard is listed only when no manifest other than this blob's own
/// names it, across every app scope.
pub fn plan_forget_blob(
    conn: &mut SqliteConnection,
    blob_hash: &str,
) -> Result<ForgetBlobPlan, StorageError> {
    let manifests: Vec<(String, String)> = shard_manifests::table
        .select((
            shard_manifests::blob_hash,
            shard_manifests::shard_hashes_json,
        ))
        .load(conn)
        .map_err(|e| StorageError::Database(format!("plan_forget_blob: {e}")))?;
    let mut own: HashSet<String> = HashSet::from([blob_hash.to_string()]);
    let mut others: HashSet<String> = HashSet::new();
    for (manifest_blob, json) in &manifests {
        let hashes: Vec<String> = serde_json::from_str(json).unwrap_or_default();
        if manifest_blob == blob_hash {
            own.extend(hashes);
        } else {
            others.extend(hashes);
            others.insert(manifest_blob.clone());
        }
    }
    let mut files: Vec<String> = own.into_iter().filter(|h| !others.contains(h)).collect();
    files.sort();
    let blake3 = super::peer_blob_inventory::lookup_blake3_for_sha256(conn, blob_hash)?;
    Ok(ForgetBlobPlan {
        blob_hash: blob_hash.to_string(),
        files,
        blake3,
    })
}

/// Drop this peer's own record of holding a blob whose files are already gone:
/// its manifests, the location rows of its unshared shards, and its
/// self-custody inventory rows. Run AFTER the files are deleted, so a crash in
/// between leaves rows that the next pass finishes rather than files nothing
/// names.
pub fn forget_blob_rows(
    conn: &mut SqliteConnection,
    plan: &ForgetBlobPlan,
) -> Result<(), StorageError> {
    conn.transaction(|conn| {
        diesel::delete(
            shard_manifests::table.filter(shard_manifests::blob_hash.eq(&plan.blob_hash)),
        )
        .execute(conn)?;
        diesel::delete(
            shard_locations::table.filter(shard_locations::shard_hash.eq_any(&plan.files)),
        )
        .execute(conn)?;
        diesel::delete(
            peer_blob_inventory::table
                .filter(peer_blob_inventory::blob_hash.eq_any(&plan.files))
                .filter(peer_blob_inventory::source.eq("self-custody")),
        )
        .execute(conn)?;
        Ok::<(), diesel::result::Error>(())
    })
    .map_err(|e| StorageError::Database(format!("forget_blob_rows: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(hex: &str) -> LedgerArtifact {
        LedgerArtifact {
            sha256: hex.to_string(),
            blob_cid: format!("bafkrei{hex}"),
            bytes: 7,
            filename: format!("{hex}.zip"),
        }
    }

    #[test]
    fn a_release_seen_again_keeps_its_place_and_its_artifacts() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        assert!(record_release(&mut conn, "ch", "r1", "app-bundle", &[artifact("aa")]).unwrap());
        assert!(record_release(&mut conn, "ch", "r2", "app-bundle", &[artifact("bb")]).unwrap());
        // The head moved back to r1 and the controller verified it again.
        assert!(!record_release(&mut conn, "ch", "r1", "app-bundle", &[artifact("cc")]).unwrap());

        let ledger = list_releases(&mut conn).unwrap();
        let order: Vec<&str> = ledger.iter().map(|r| r.release_cid.as_str()).collect();
        assert_eq!(order, ["r2", "r1"], "newest first, and r1 did not move");
        assert_eq!(ledger[1].artifacts, [artifact("aa")]);
    }

    #[test]
    fn the_same_release_cid_on_two_channels_is_two_rows() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        assert!(record_release(&mut conn, "a", "r1", "app-bundle", &[]).unwrap());
        assert!(record_release(&mut conn, "b", "r1", "app-bundle", &[]).unwrap());
        assert_eq!(list_releases(&mut conn).unwrap().len(), 2);
    }

    #[test]
    fn a_blob_named_only_inside_a_row_s_metadata_counts_as_in_use() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let served = artifact("deadbeef");
        assert_eq!(content_rows_naming(&mut conn, &served).unwrap(), 0);
        diesel::sql_query(
            "INSERT INTO content (id, h_app_id, title, content_type, content_format, \
             metadata_json) VALUES ('app', 'lamad', 'App', 'app', 'html5-app', \
             '{\"serverBlobHash\":\"sha256-deadbeef\"}')",
        )
        .execute(&mut conn)
        .unwrap();
        assert_eq!(content_rows_naming(&mut conn, &served).unwrap(), 1);
        assert_eq!(
            content_rows_naming(&mut conn, &artifact("0123")).unwrap(),
            0
        );
    }
}
