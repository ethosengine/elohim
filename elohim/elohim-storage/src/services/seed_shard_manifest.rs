//! Operator/seed write path for shard manifests + shard locations.
//!
//! ## Why this exists (the honesty boundary)
//!
//! The resilience card's `distributionState` + `stewardingCollectives` columns
//! light from two operational tables:
//!   - `shard_manifests` — "this content entered the distribution plane"
//!     (drives `distributionState: measured`)
//!   - `shard_locations`  — "this shard is held by this peer" (agent-keyed;
//!     joins to `humans.agent_pub_key` → `humans.household_id` to drive
//!     `stewardingCollectives`)
//!
//! Those rows are written by the **real** distribution path (`POST /db/content`
//! with a `blobCid` → `record_manifest_from_bytes` + `distribute_shards`, and
//! the boot-time `shard_manifest_backfill`). That path is the source of truth for
//! genuinely-distributed content and requires live peers + active provide
//! commitments + the in-pod conductor's cells enabled.
//!
//! This module is the **deterministic seed/demo lever** for that same shape: an
//! operator-gated write that creates a manifest + agent-keyed location rows for a
//! content WITHOUT waiting on full P2P gossip. It is for blob-backed demo content
//! and acceptance tests on a thin mesh.
//!
//! ### The honesty contract — do not light what is not distributed
//!
//! Lighting these columns is a CLAIM that the content is held by the named
//! households. This module must therefore:
//!   1. Be **off by default** — gated behind `ALLOW_SEED_SHARD_MANIFEST=1`. In
//!      production (flag unset) the endpoint refuses, so a real deployment can
//!      never silently fabricate distribution. (cf. the felt-surface honesty
//!      discipline: `distributionState: unmeasured` is the truthful default for
//!      inline-body content; this lever only flips it when an operator opts in.)
//!   2. Mark seeded location rows with `status = "seeded"` (not `"announced"` /
//!      `"verified"`), so a row's provenance is legible: a seeded row says
//!      "an operator asserted this", a distributed row says "the substrate
//!      observed a push ack". Both join identically (the resilience snapshot does
//!      not filter on status today), but the status column keeps the audit trail
//!      honest.
//!
//! Category C (operational): no new DHT entity, no new table, no coordinator
//! function, no signal. An explicit operator write to existing operational
//! projection tables. Identity is agent-keyed (`peer_id = steward's
//! agent_pub_key`) so the rows join to `humans` exactly like distributed rows.

use diesel::prelude::*;

use elohim_epr::address::ByteCid;

use crate::blob_store::ChunkRef;
use crate::db::models::NewShardLocation;
use crate::db::{shard_locations, shard_manifests};
use crate::sharding::{ChunkerDecl, ErasureDecl, ShardManifest};
use crate::StorageError;

/// Status marker for operator-seeded shard locations. Distinct from the runtime
/// `announced` / `verified` statuses so a seeded row's provenance is legible.
pub const SEEDED_STATUS: &str = "seeded";

/// One steward to place the (synthetic) shards on. `peer_id` MUST be the
/// steward's Holochain `agent_pub_key` (`uhCAka…`) — the join key shared by
/// `humans.agent_pub_key`, `peer_statuses.peer_id`, and the runtime
/// `distribute_shards` write path. A libp2p PeerId (`12D3KooW…`) will NOT join.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SeedSteward {
    /// The steward's `agent_pub_key`.
    pub peer_id: String,
}

/// Input for a deterministic shard-manifest + locations seed.
#[derive(Debug, Clone)]
pub struct SeedShardManifestInput {
    /// App instance scope (e.g. "lamad").
    pub h_app_id: String,
    /// Content the manifest is for. Must already exist as a content row for the
    /// join names to render, but we do not require it here (the manifest is the
    /// gate the snapshot reads).
    pub content_id: String,
    /// The content's reach (recorded verbatim on the manifest). Defaults to
    /// "commons" at the handler if absent.
    pub reach: String,
    /// Explicit custody units (byte CIDs). When empty, the synthetic manifest
    /// names the payload itself as its one unit.
    pub shard_cids: Vec<String>,
    /// The payload's byte CID. When `shard_cids` is empty this is also the single
    /// unit; when absent and `shard_cids` is non-empty, the first unit is used.
    pub blob_cid: Option<String>,
    /// Stewards to place every shard on. Each becomes one `shard_locations` row
    /// per shard hash. Identity is agent-keyed.
    pub stewards: Vec<SeedSteward>,
    /// Total content size in bytes (best-effort; recorded on the manifest).
    pub total_size_bytes: i64,
}

/// What the seed wrote — returned for the response body + tests.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedShardManifestReport {
    pub content_id: String,
    /// The payload's byte CID the manifest records.
    pub blob_cid: String,
    /// Custody units the manifest names.
    pub unit_count: usize,
    /// Number of `shard_locations` rows written (shards × stewards).
    pub locations_written: usize,
    /// The stewards' agent keys the shards were placed on (for legibility).
    pub stewards: Vec<String>,
    /// Always true — a reminder in the response body that this is a seeded claim.
    pub seeded: bool,
}

/// Validate + apply the seed within a single connection.
///
/// Idempotent: the manifest upsert replaces in place, and location upserts
/// insert-or-update by `(shard_cid, peer_id)`. Running twice yields the same rows.
pub fn apply(
    conn: &mut SqliteConnection,
    input: &SeedShardManifestInput,
) -> Result<SeedShardManifestReport, StorageError> {
    if input.content_id.trim().is_empty() {
        return Err(StorageError::InvalidInput("content_id is required".into()));
    }
    if input.stewards.is_empty() {
        return Err(StorageError::InvalidInput(
            "at least one steward (agent_pub_key) is required — an empty steward set would write a manifest with zero locations, lighting distributionState but not stewardingCollectives".into(),
        ));
    }
    for s in &input.stewards {
        if s.peer_id.trim().is_empty() {
            return Err(StorageError::InvalidInput(
                "steward peer_id (agent_pub_key) must be non-empty".into(),
            ));
        }
    }

    // Resolve the custody-unit set + payload CID. Every address is a byte CID,
    // admitted by full decode; anything else refuses the seed.
    let parse = |text: &str| {
        ByteCid::parse(text.trim())
            .map_err(|e| StorageError::InvalidInput(format!("not a byte CID: {text} ({e})")))
    };
    let (units, blob_cid): (Vec<ByteCid>, ByteCid) =
        match (input.shard_cids.is_empty(), input.blob_cid.as_deref()) {
            // Explicit units given: blob_cid = provided, else the first unit.
            (false, Some(bc)) => (
                input
                    .shard_cids
                    .iter()
                    .map(|u| parse(u))
                    .collect::<Result<_, _>>()?,
                parse(bc)?,
            ),
            (false, None) => {
                let units: Vec<ByteCid> = input
                    .shard_cids
                    .iter()
                    .map(|u| parse(u))
                    .collect::<Result<_, _>>()?;
                let first = units[0];
                (units, first)
            }
            // No explicit units: the payload is its own single unit.
            (true, Some(bc)) => {
                let cid = parse(bc)?;
                (vec![cid], cid)
            }
            (true, None) => {
                return Err(StorageError::InvalidInput(
                    "either shardCids or blobCid is required".into(),
                ))
            }
        };

    let reach = if input.reach.trim().is_empty() {
        "commons"
    } else {
        input.reach.trim()
    };

    // A seeded manifest declares the epoch-0 chunker and no erasure; its units are
    // the declared chunks, the declared size split across them (best-effort).
    let unit_count = units.len() as u64;
    let byte_len = u64::try_from(input.total_size_bytes.max(0))
        .unwrap_or(0)
        .max(unit_count);
    let base = byte_len / unit_count;
    let chunks: Vec<ChunkRef> = units
        .iter()
        .enumerate()
        .map(|(i, cid)| {
            let extra = if i as u64 + 1 == unit_count {
                byte_len % unit_count
            } else {
                0
            };
            ChunkRef {
                cid: *cid,
                len: u32::try_from(base + extra).unwrap_or(u32::MAX),
            }
        })
        .collect();
    let now = chrono::Utc::now().to_rfc3339();
    let manifest = ShardManifest {
        blob_cid,
        byte_len,
        mime_type: "application/octet-stream".to_string(),
        chunker: ChunkerDecl::of(elohim_epr::profile::BYTE_PROFILE_0.chunker),
        chunks,
        erasure: ErasureDecl::of(elohim_epr::profile::Erasure::None),
        stripes: Vec::new(),
        reach: reach.to_string(),
        author_id: None,
        created_at: now,
        verified_at: None,
    };
    manifest
        .validate()
        .map_err(|e| StorageError::InvalidInput(format!("seeded manifest shape refused: {e}")))?;
    let row = shard_manifests::record_generated_manifest(
        conn,
        &input.content_id,
        &input.h_app_id,
        &manifest,
    )?;

    // One location row per (unit, steward), agent-keyed, status="seeded".
    let mut locations_written = 0usize;
    for unit in &units {
        let unit_text = unit.to_string();
        for steward in &input.stewards {
            let location = NewShardLocation {
                shard_cid: &unit_text,
                peer_id: &steward.peer_id,
                h_app_id: &input.h_app_id,
                status: SEEDED_STATUS,
            };
            shard_locations::upsert_location(conn, &location)?;
            locations_written += 1;
        }
    }

    Ok(SeedShardManifestReport {
        content_id: input.content_id.clone(),
        blob_cid: row.blob_cid,
        unit_count: units.len(),
        locations_written,
        stewards: input.stewards.iter().map(|s| s.peer_id.clone()).collect(),
        seeded: true,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::PooledConn;

    /// Fresh in-memory DB with all migrations applied — held alive by the pool
    /// so the named in-memory database is not dropped between checkouts.
    struct TestDb {
        _pool: crate::db::DbPool,
        conn: PooledConn,
    }
    impl std::ops::Deref for TestDb {
        type Target = SqliteConnection;
        fn deref(&self) -> &SqliteConnection {
            // PooledConn → SqliteConnection (r2d2 PooledConnection derefs).
            &self.conn
        }
    }
    impl std::ops::DerefMut for TestDb {
        fn deref_mut(&mut self) -> &mut SqliteConnection {
            &mut self.conn
        }
    }

    fn test_conn() -> TestDb {
        let pool = crate::test_util::test_pool();
        let conn = pool.get().expect("test pool connection");
        TestDb { _pool: pool, conn }
    }

    fn input(stewards: &[&str]) -> SeedShardManifestInput {
        SeedShardManifestInput {
            h_app_id: "lamad".into(),
            content_id: "summer-1974".into(),
            reach: "intimate".into(),
            shard_cids: vec![],
            blob_cid: Some("bafkr4igzdylhmq4rythogpcv6upov3znvbufxh6dbjd3rh5cb6iyqvli4u".into()),
            stewards: stewards
                .iter()
                .map(|s| SeedSteward {
                    peer_id: s.to_string(),
                })
                .collect(),
            total_size_bytes: 4096,
        }
    }

    #[test]
    fn empty_stewards_is_rejected() {
        let mut conn = test_conn();
        let err = apply(&mut conn, &input(&[])).unwrap_err();
        assert!(matches!(err, StorageError::InvalidInput(_)), "{err:?}");
    }

    #[test]
    fn empty_content_id_is_rejected() {
        let mut conn = test_conn();
        let mut i = input(&["uhCAka-adam"]);
        i.content_id = "".into();
        let err = apply(&mut conn, &i).unwrap_err();
        assert!(matches!(err, StorageError::InvalidInput(_)), "{err:?}");
    }

    #[test]
    fn no_shards_and_no_blob_is_rejected() {
        let mut conn = test_conn();
        let mut i = input(&["uhCAka-adam"]);
        i.shard_cids = vec![];
        i.blob_cid = None;
        let err = apply(&mut conn, &i).unwrap_err();
        assert!(matches!(err, StorageError::InvalidInput(_)), "{err:?}");
    }

    #[test]
    fn synthetic_single_unit_names_the_payload_itself() {
        let mut conn = test_conn();
        let report = apply(&mut conn, &input(&["uhCAka-adam", "uhCAka-matthew"])).unwrap();
        assert_eq!(
            report.blob_cid,
            "bafkr4igzdylhmq4rythogpcv6upov3znvbufxh6dbjd3rh5cb6iyqvli4u"
        );
        assert_eq!(report.unit_count, 1);
        // 1 shard × 2 stewards = 2 location rows.
        assert_eq!(report.locations_written, 2);
        assert!(report.seeded);

        // The manifest is readable and drives distributionState=measured.
        let m = shard_manifests::get_manifest(&mut conn, "lamad", "summer-1974")
            .unwrap()
            .expect("manifest exists");
        assert_eq!(m.reach, "intimate");
        let hashes = shard_manifests::custody_unit_cids(&m);
        assert_eq!(
            hashes,
            vec!["bafkr4igzdylhmq4rythogpcv6upov3znvbufxh6dbjd3rh5cb6iyqvli4u".to_string()]
        );

        // Location rows are agent-keyed with status="seeded".
        let locs = shard_locations::get_locations_for_shard(
            &mut conn,
            "bafkr4igzdylhmq4rythogpcv6upov3znvbufxh6dbjd3rh5cb6iyqvli4u",
        )
        .unwrap();
        assert_eq!(locs.len(), 2);
        assert!(locs.iter().all(|l| l.status == "seeded"));
        let mut keys: Vec<&str> = locs.iter().map(|l| l.peer_id.as_str()).collect();
        keys.sort();
        assert_eq!(keys, vec!["uhCAka-adam", "uhCAka-matthew"]);
    }

    #[test]
    fn explicit_shard_cids_produce_per_shard_per_steward_rows() {
        let mut conn = test_conn();
        let mut i = input(&["uhCAka-adam", "uhCAka-matthew"]);
        i.shard_cids = vec![
            "bafkr4iah32kcnkgp6px7cqivintyn2ep4uxauk3icf2ggh62jhfmgfyeia".into(),
            "bafkr4igemkzkc76jory35nc4cxocvoop2f34ocuzh5vkzzhvmb3lejjph4".into(),
            "bafkr4id3bnalc4t4rc2ep3uike7v5kdo6lov3sqaequvpjqrom4455olwi".into(),
        ];
        i.blob_cid = Some("bafkr4ibd7avcsuzi4eliah6f5o43qtb4de6tvdzmgfp7i6x3autnahy7sq".into());
        let report = apply(&mut conn, &i).unwrap();
        assert_eq!(report.unit_count, 3);
        // 3 shards × 2 stewards = 6 location rows.
        assert_eq!(report.locations_written, 6);
    }

    #[test]
    fn apply_is_idempotent() {
        let mut conn = test_conn();
        let i = input(&["uhCAka-adam"]);
        let r1 = apply(&mut conn, &i).unwrap();
        let r2 = apply(&mut conn, &i).unwrap();
        assert_eq!(r1, r2);
        // Still exactly one location row after the second apply.
        let locs = shard_locations::get_locations_for_shard(
            &mut conn,
            "bafkr4igzdylhmq4rythogpcv6upov3znvbufxh6dbjd3rh5cb6iyqvli4u",
        )
        .unwrap();
        assert_eq!(locs.len(), 1);
    }
}
