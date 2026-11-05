//! `BlobStoreTier`: the peer's own [`BlobStore`] as an eprfs tier (interfaces spec §2.A.6).
//!
//! A leaf store. It answers from what it holds, treats `FetchIfMissing` as `LocalOnly` (contract
//! 11: only the chain honours the policy), and inherits eprfs-core's default `fetch_at`,
//! `fetch_blobs` and depth-0 queries, so a batch is answered item by item with each CID fetched
//! once and verified before a sink sees it.
//!
//! # Preconditions (spec §12.3 rule 5, §17 slice 2, finding A1)
//!
//! - **Two lanes, by the CID's own profile.** A tree node (`bafyrei…`, dag-cbor + sha2-256) is a
//!   record and lives on the store's record lane (`store_record`/`get_record`); a leaf is a byte CID
//!   (`bafkr4i…`, raw + BLAKE3-256) and lives in the payload store (`store`/`get`). Any other
//!   profile is not held here. Bytes are verified by the address kernel's `verify_cid_any`, which
//!   dispatches on the CID's declared `(codec, multihash, length)` and fails closed.
//! - **Off the `/blob/{cid}` route.** That route serves any blob no content row references to an
//!   anonymous caller (`blob_reach.rs`, honest absence), so a tree object held in the route's own
//!   store would be served past its root's reach. The tier is therefore built only over a store of
//!   its own, in [`TIER_SUBDIR`] beside the route's store ([`BlobStoreTier::dedicated`]): the only
//!   door to its bytes is a `TierEndpoint` fronted by a [`super::RootReachGate`]. The cost is that
//!   a leaf also published as public content is held twice.
//! - This tier never speaks iroh.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    AttestationDraft, BlobCid, BlobHandle, BlobPresence, EprRecord, EprRef, EprfsError,
    EprfsStorage, FetchPolicy, Result,
};

use elohim_epr::address::{verify_cid_any, ByteCid, Verify};

use crate::blob_store::{BlobStore, Exists};
use crate::epr_codec::DAG_CBOR_CODEC;
use crate::error::StorageError;

/// The directory, under the storage root, that holds the tier's own store.
pub const TIER_SUBDIR: &str = "eprfs-tier";

/// The peer's own store as an eprfs tier. `tier_id` is `own` (the default): a serving
/// `TierEndpoint` stamps its agent key on what it answers for this store.
pub struct BlobStoreTier {
    store: Arc<BlobStore>,
}

impl BlobStoreTier {
    /// Opens (creating it if needed) the tier's own store at `<storage_root>/eprfs-tier`, a store
    /// the `/blob/{cid}` route never reads (A1).
    pub async fn dedicated(
        storage_root: impl AsRef<Path>,
    ) -> std::result::Result<Self, StorageError> {
        let store = BlobStore::new(storage_root.as_ref().join(TIER_SUBDIR)).await?;
        Ok(Self {
            store: Arc::new(store),
        })
    }

    /// The store behind the tier, for the host that composes it (its stats, its disk probe).
    pub fn store(&self) -> &BlobStore {
        &self.store
    }

    /// Where `cid` lives in the store: the record lane, the payload store, or nowhere.
    fn lane(cid: &BlobCid) -> Lane {
        if cid.as_cid().codec() == DAG_CBOR_CODEC {
            return Lane::Record;
        }
        match ByteCid::try_from(*cid.as_cid()) {
            Ok(byte_cid) => Lane::Payload(byte_cid),
            Err(_) => Lane::NotHeld,
        }
    }

    async fn read(&self, cid: &BlobCid) -> std::result::Result<Bytes, StorageError> {
        match Self::lane(cid) {
            Lane::Record => self.store.get_record(cid.as_cid()).await,
            Lane::Payload(byte_cid) => self.store.get(&byte_cid).await,
            Lane::NotHeld => Err(StorageError::NotFound(cid.to_string())),
        }
    }
}

/// The store lane a CID's profile names.
enum Lane {
    Record,
    Payload(ByteCid),
    NotHeld,
}

fn verified(cid: &BlobCid, bytes: &[u8]) -> bool {
    verify_cid_any(cid.as_cid(), bytes) == Verify::Verified
}

/// `StorageError` → `EprfsError`, keeping absence apart from failure (A17): only a not-found is
/// `BlobNotFound`; a reassembled object that fails its hash is `Corrupt`; anything else is a
/// storage failure, which a reader maps to `Unreachable` (absence not established).
fn storage_error(cid: &BlobCid, e: StorageError) -> EprfsError {
    match e {
        StorageError::NotFound(_) | StorageError::BlobNotFound(_) => {
            EprfsError::BlobNotFound(cid.clone())
        }
        StorageError::HashMismatch { .. } => EprfsError::Corrupt(cid.clone()),
        other => EprfsError::Storage(other.to_string()),
    }
}

#[async_trait]
impl EprfsStorage for BlobStoreTier {
    /// A blob store holds bytes, never EPR records.
    async fn resolve_epr(&self, reference: &EprRef) -> Result<EprRecord> {
        Err(EprfsError::EprNotFound(reference.clone()))
    }

    /// Whether the object is held here. Probe through the store's metadata path so only a
    /// confirmed absence is `Missing`; unreadable storage remains an error (A17).
    async fn has_blob(&self, cid: &BlobCid) -> Result<BlobPresence> {
        match Self::lane(cid) {
            // The length read separates a confirmed absence from unreadable storage,
            // which the structural `exists` probe folds together.
            Lane::Payload(byte_cid) => match self.store.size(&byte_cid).await {
                Ok(_) => match self.store.exists(&byte_cid).await {
                    Exists::Verified => Ok(BlobPresence::Local),
                    Exists::Absent | Exists::Quarantined => Ok(BlobPresence::Missing),
                },
                Err(StorageError::NotFound(_) | StorageError::BlobNotFound(_)) => {
                    Ok(BlobPresence::Missing)
                }
                Err(error) => Err(storage_error(cid, error)),
            },
            Lane::Record => match self.store.get_record(cid.as_cid()).await {
                Ok(_) => Ok(BlobPresence::Local),
                Err(StorageError::NotFound(_) | StorageError::BlobNotFound(_)) => {
                    Ok(BlobPresence::Missing)
                }
                Err(error) => Err(storage_error(cid, error)),
            },
            Lane::NotHeld => Ok(BlobPresence::Missing),
        }
    }

    /// The held bytes, verified against `cid`'s own codec. Both policies answer from this store
    /// alone (contract 11). Bytes that fail their address are `Corrupt`, never returned.
    async fn fetch_blob(&self, cid: &BlobCid, _policy: FetchPolicy) -> Result<BlobHandle> {
        let bytes = self.read(cid).await.map_err(|e| storage_error(cid, e))?;
        if !verified(cid, &bytes) {
            return Err(EprfsError::Corrupt(cid.clone()));
        }
        Ok(BlobHandle {
            cid: cid.clone(),
            bytes,
        })
    }

    /// Stores raw bytes and returns their byte CID (raw + BLAKE3-256).
    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid> {
        let stored = self
            .store
            .store(&bytes)
            .await
            .map_err(|e| EprfsError::Storage(e.to_string()))?;
        BlobCid::parse(&stored.cid.to_string()).map_err(|e| EprfsError::Storage(e.to_string()))
    }

    /// Stores bytes under an explicit byte or record CID, after checking they are its bytes.
    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> Result<()> {
        if !verified(cid, &bytes) {
            return Err(EprfsError::Storage(format!(
                "refusing to store bytes that do not verify against {cid}"
            )));
        }
        let stored = match Self::lane(cid) {
            Lane::Payload(_) => self.store.store(&bytes).await.map(|_| ()),
            Lane::Record => self.store.store_record(&bytes).await.map(|_| ()),
            Lane::NotHeld => {
                return Err(EprfsError::Storage(format!(
                    "{cid}: this profile is not held by the blob-store tier"
                )))
            }
        };
        stored.map_err(|e| EprfsError::Storage(e.to_string()))
    }

    /// A blob store publishes no attestations.
    async fn publish_attestation(&self, _draft: AttestationDraft) -> Result<EprRef> {
        Err(EprfsError::Storage(
            "the blob-store tier publishes no attestations".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use elohim_epr::address::compute_bytes_cid;
    use eprfs_core::{
        BatchAnswer, BatchItem, CollectSink, ItemAbsence, TreeEntry, TreeEntryKind, TreeNode,
        TreePath,
    };
    use eprfs_serve::testing::assert_leaf_contract;

    /// A leaf's byte CID (raw + BLAKE3-256), as the tier names what it holds.
    fn leaf(bytes: &[u8]) -> BlobCid {
        BlobCid::parse(&compute_bytes_cid(bytes).to_string()).unwrap()
    }

    async fn tier() -> (tempfile::TempDir, BlobStoreTier) {
        let dir = tempfile::tempdir().unwrap();
        let tier = BlobStoreTier::dedicated(dir.path()).await.unwrap();
        (dir, tier)
    }

    #[tokio::test]
    async fn the_blob_store_tier_passes_the_leaf_contract() {
        let (_dir, tier) = tier().await;
        let held_bytes = Bytes::from_static(b"held by the blob-store tier");
        let held = leaf(&held_bytes);
        tier.put_blob_verified(&held, held_bytes).await.unwrap();
        let missing = leaf(b"never stored");
        assert_leaf_contract(&tier, &held, &missing).await;
    }

    #[tokio::test]
    async fn a_dag_cbor_tree_node_round_trips_under_its_own_codec() {
        let (_dir, tier) = tier().await;
        let leaf_bytes = Bytes::from_static(b"a leaf");
        let leaf_cid = tier.put_blob(leaf_bytes.clone()).await.unwrap();
        assert_eq!(leaf_cid, leaf(&leaf_bytes));
        let node = TreeNode::new(vec![TreeEntry {
            name: b"leaf".to_vec(),
            kind: TreeEntryKind::File {
                blob: leaf_cid.clone().into(),
                executable: false,
            },
        }])
        .unwrap();
        let (node_cid, node_bytes) =
            eprfs_core::encode_tree(&node, &eprfs_core::TreeLimits::default()).unwrap();
        let node_bytes = Bytes::from(node_bytes);
        assert_eq!(node_cid.as_cid().codec(), DAG_CBOR_CODEC);
        tier.put_blob_verified(&node_cid, node_bytes.clone())
            .await
            .unwrap();

        let got = tier
            .fetch_blob(&node_cid, FetchPolicy::LocalOnly)
            .await
            .unwrap();
        assert_eq!(got.bytes, node_bytes);
        assert_eq!(tier.has_blob(&node_cid).await.unwrap(), BlobPresence::Local);

        // The batch path verifies and fans out once per CID, as the default does.
        let mut sink = CollectSink::default();
        let items = [
            BatchItem::new(node_cid.clone(), TreePath::root()),
            BatchItem::new(
                leaf_cid.clone(),
                TreePath::from_slash_bytes(b"leaf").unwrap(),
            ),
        ];
        tier.fetch_blobs(&node_cid, &items, &FetchPolicy::LocalOnly.into(), &mut sink)
            .await
            .unwrap();
        assert!(sink
            .answers
            .iter()
            .all(|(_, a)| matches!(a, BatchAnswer::Arrived { .. })));
    }

    #[tokio::test]
    async fn bytes_that_fail_their_address_are_refused_and_never_returned() {
        let (dir, tier) = tier().await;
        let real = Bytes::from_static(b"the real bytes");
        let cid = leaf(&real);
        assert!(tier
            .put_blob_verified(&cid, Bytes::from_static(b"other bytes"))
            .await
            .is_err());
        assert_eq!(tier.has_blob(&cid).await.unwrap(), BlobPresence::Missing);

        // A held object corrupted on disk reads as the typed `Corrupt`, and a batch names it
        // `Integrity`, never an arrival.
        tier.put_blob_verified(&cid, real).await.unwrap();
        let name = cid.to_string();
        let path = dir
            .path()
            .join(TIER_SUBDIR)
            .join("blobs")
            .join(&name[7..11])
            .join(&name);
        std::fs::write(&path, b"tampered").unwrap();
        match tier.fetch_blob(&cid, FetchPolicy::LocalOnly).await {
            Err(EprfsError::Corrupt(c)) => assert_eq!(c, cid),
            other => panic!("expected Corrupt, got {other:?}"),
        }
        let mut sink = CollectSink::default();
        tier.fetch_blobs(
            &cid,
            &[BatchItem::new(cid.clone(), TreePath::root())],
            &FetchPolicy::LocalOnly.into(),
            &mut sink,
        )
        .await
        .unwrap();
        match &sink.answers[0].1 {
            BatchAnswer::Absent(why) => {
                assert!(matches!(why.why, ItemAbsence::Integrity { .. }), "{why:?}")
            }
            other => panic!("expected Integrity, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn the_tier_store_is_not_the_blob_route_store() {
        let dir = tempfile::tempdir().unwrap();
        let route = BlobStore::new(dir.path()).await.unwrap();
        let tier = BlobStoreTier::dedicated(dir.path()).await.unwrap();
        let bytes = Bytes::from_static(b"a tree object under a private root");
        let cid = leaf(&bytes);
        tier.put_blob_verified(&cid, bytes).await.unwrap();
        assert!(
            route
                .exists(&ByteCid::parse(&cid.to_string()).unwrap())
                .await
                != Exists::Verified,
            "A1: an object the tier holds is not reachable through the /blob route's store"
        );
        assert_eq!(tier.store().root_dir(), dir.path().join(TIER_SUBDIR));
    }

    #[tokio::test]
    async fn an_unreadable_store_is_not_a_missing_blob() {
        let (dir, tier) = tier().await;
        let bytes = Bytes::from_static(b"presence is observed, not guessed");
        let cid = leaf(&bytes);
        assert_eq!(tier.has_blob(&cid).await.unwrap(), BlobPresence::Missing);
        tier.put_blob_verified(&cid, bytes).await.unwrap();
        assert_eq!(tier.has_blob(&cid).await.unwrap(), BlobPresence::Local);

        // ENOTDIR is reliable even for a root test runner, unlike permission-bit changes.
        let blobs = dir.path().join(TIER_SUBDIR).join("blobs");
        std::fs::rename(&blobs, blobs.with_file_name("held-blobs")).unwrap();
        std::fs::write(&blobs, b"not a directory").unwrap();
        assert!(matches!(
            tier.has_blob(&cid).await,
            Err(EprfsError::Storage(_))
        ));
        assert!(matches!(
            tier.fetch_blob(&cid, FetchPolicy::LocalOnly).await,
            Err(EprfsError::Storage(_))
        ));
        let mut sink = CollectSink::default();
        tier.fetch_blobs(
            &cid,
            &[BatchItem::new(cid.clone(), TreePath::root())],
            &FetchPolicy::LocalOnly.into(),
            &mut sink,
        )
        .await
        .unwrap();
        assert!(matches!(
            &sink.answers[0].1,
            BatchAnswer::Absent(why) if matches!(why.why, ItemAbsence::Unreachable { .. })
        ));
    }

    #[tokio::test]
    async fn the_tier_holds_no_records_and_publishes_nothing() {
        let (_dir, tier) = tier().await;
        assert!(matches!(
            tier.resolve_epr(&EprRef::new("epr:x")).await,
            Err(EprfsError::EprNotFound(_))
        ));
        assert!(tier.tier_id().is_own());
    }
}
