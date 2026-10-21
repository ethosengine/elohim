//! `BlobStoreTier`: the peer's own [`BlobStore`] as an eprfs tier (interfaces spec §2.A.6).
//!
//! A leaf store. It answers from what it holds, treats `FetchIfMissing` as `LocalOnly` (contract
//! 11: only the chain honours the policy), and inherits eprfs-core's default `fetch_at`,
//! `fetch_blobs` and depth-0 queries, so a batch is answered item by item with each CID fetched
//! once and verified before a sink sees it.
//!
//! # Preconditions (spec §12.3 rule 5, §17 slice 2, finding A1)
//!
//! - **dag-cbor objects.** `BlobStore` files every object under the sha2-256 of its bytes, so a
//!   tree node (`bafyrei…`, codec 0x71) and a raw leaf (`bafkrei…`, 0x55) are both held; the CID's
//!   codec decides how bytes are verified and how they are written (`store_dag_cbor` or `store`).
//! - **Off the `/blob/{cid}` route.** That route serves any blob no content row references to an
//!   anonymous caller (`blob_reach.rs`, honest absence), so a tree object held in the route's own
//!   store would be served past its root's reach. The tier is therefore built only over a store of
//!   its own, in [`TIER_SUBDIR`] beside the route's store ([`BlobStoreTier::dedicated`]): the only
//!   door to its bytes is a `TierEndpoint` fronted by a [`super::RootReachGate`]. The cost is that
//!   a leaf also published as public content is held twice.
//! - **The sha256-to-iroh alias** (spec §17 slice 2) is the iroh blob plane's precondition, not
//!   this tier's: this tier never speaks iroh.

use std::path::Path;
use std::sync::Arc;

use async_trait::async_trait;
use bytes::Bytes;
use eprfs_core::{
    AttestationDraft, BlobCid, BlobHandle, BlobPresence, EprRecord, EprRef, EprfsError,
    EprfsStorage, FetchPolicy, Result,
};

use crate::blob_store::BlobStore;
use crate::epr_codec::DAG_CBOR_CODEC;
use crate::error::StorageError;

/// The directory, under the storage root, that holds the tier's own store.
pub const TIER_SUBDIR: &str = "eprfs-tier";

const RAW_CODEC: u64 = 0x55;

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

    fn sha256_name(cid: &BlobCid) -> String {
        let digest = cid.as_cid().hash().digest();
        format!("sha256-{}", hex::encode(digest))
    }
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
        match self.store.size(&Self::sha256_name(cid)).await {
            Ok(_) => Ok(BlobPresence::Local),
            Err(StorageError::NotFound(_) | StorageError::BlobNotFound(_)) => {
                Ok(BlobPresence::Missing)
            }
            Err(error) => Err(storage_error(cid, error)),
        }
    }

    /// The held bytes, verified against `cid`'s own codec. Both policies answer from this store
    /// alone (contract 11). Bytes that fail their address are `Corrupt`, never returned.
    async fn fetch_blob(&self, cid: &BlobCid, _policy: FetchPolicy) -> Result<BlobHandle> {
        let bytes = self
            .store
            .get(&Self::sha256_name(cid))
            .await
            .map_err(|e| storage_error(cid, e))?;
        if !cid.verifies(&bytes) {
            return Err(EprfsError::Corrupt(cid.clone()));
        }
        Ok(BlobHandle {
            cid: cid.clone(),
            bytes: Bytes::from(bytes),
        })
    }

    /// Stores raw bytes and returns their raw-codec CID.
    async fn put_blob(&self, bytes: Bytes) -> Result<BlobCid> {
        self.store
            .store(&bytes)
            .await
            .map_err(|e| EprfsError::Storage(e.to_string()))?;
        Ok(BlobCid::compute_raw(&bytes))
    }

    /// Stores bytes under an explicit raw or dag-cbor CID, after checking they are its bytes.
    async fn put_blob_verified(&self, cid: &BlobCid, bytes: Bytes) -> Result<()> {
        if !cid.verifies(&bytes) {
            return Err(EprfsError::Storage(format!(
                "refusing to store bytes that do not verify against {cid}"
            )));
        }
        let stored = match cid.as_cid().codec() {
            RAW_CODEC => self.store.store(&bytes).await,
            DAG_CBOR_CODEC => self.store.store_dag_cbor(&bytes).await,
            other => {
                return Err(EprfsError::Storage(format!(
                    "codec {other:#x} is not held by the blob-store tier"
                )))
            }
        };
        stored
            .map(|_| ())
            .map_err(|e| EprfsError::Storage(e.to_string()))
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
    use eprfs_core::{
        BatchAnswer, BatchItem, CollectSink, ItemAbsence, TreeEntry, TreeEntryKind, TreeNode,
        TreePath,
    };
    use eprfs_serve::testing::assert_leaf_contract;

    async fn tier() -> (tempfile::TempDir, BlobStoreTier) {
        let dir = tempfile::tempdir().unwrap();
        let tier = BlobStoreTier::dedicated(dir.path()).await.unwrap();
        (dir, tier)
    }

    #[tokio::test]
    async fn the_blob_store_tier_passes_the_leaf_contract() {
        let (_dir, tier) = tier().await;
        let held_bytes = Bytes::from_static(b"held by the blob-store tier");
        let held = BlobCid::compute_raw(&held_bytes);
        tier.put_blob_verified(&held, held_bytes).await.unwrap();
        let missing = BlobCid::compute_raw(b"never stored");
        assert_leaf_contract(&tier, &held, &missing).await;
    }

    #[tokio::test]
    async fn a_dag_cbor_tree_node_round_trips_under_its_own_codec() {
        let (_dir, tier) = tier().await;
        let leaf = Bytes::from_static(b"a leaf");
        let leaf_cid = tier.put_blob(leaf.clone()).await.unwrap();
        assert_eq!(leaf_cid, BlobCid::compute_raw(&leaf));
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
        let cid = BlobCid::compute_raw(&real);
        assert!(tier
            .put_blob_verified(&cid, Bytes::from_static(b"other bytes"))
            .await
            .is_err());
        assert_eq!(tier.has_blob(&cid).await.unwrap(), BlobPresence::Missing);

        // A held object corrupted on disk reads as the typed `Corrupt`, and a batch names it
        // `Integrity`, never an arrival.
        tier.put_blob_verified(&cid, real).await.unwrap();
        let name = BlobStoreTier::sha256_name(&cid);
        let hex = name.trim_start_matches("sha256-");
        let path = dir
            .path()
            .join(TIER_SUBDIR)
            .join("blobs")
            .join(&hex[..4])
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
        let cid = BlobCid::compute_raw(&bytes);
        tier.put_blob_verified(&cid, bytes).await.unwrap();
        assert!(
            !route.exists(&BlobStoreTier::sha256_name(&cid)).await,
            "A1: an object the tier holds is not reachable through the /blob route's store"
        );
        assert_eq!(tier.store().root_dir(), dir.path().join(TIER_SUBDIR));
    }

    #[tokio::test]
    async fn an_unreadable_store_is_not_a_missing_blob() {
        let (dir, tier) = tier().await;
        let bytes = Bytes::from_static(b"presence is observed, not guessed");
        let cid = BlobCid::compute_raw(&bytes);
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
