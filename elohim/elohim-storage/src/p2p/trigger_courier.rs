//! The node's half of the adoption trigger's courier path
//! ([`crate::services::courier_obey`]): an owned head-record fetcher over the
//! reconcile peers, and byte presence over this node's blob store.

use std::sync::Arc;

use seam_contracts::Answer;
use tokio::sync::mpsc;

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::p2p::head_record_client::PeerHeadRecordFetcher;
use crate::p2p::reconcile_peers::ReconcilePeers;
use crate::p2p::P2PCommand;
use crate::services::courier_obey::BytePresence;
use crate::services::head_adoption::{CarriedHeadRecord, HeadRecordFetcher};

/// [`PeerHeadRecordFetcher`] that owns its peer source, so a long-lived worker
/// can hold it.
pub struct OwnedPeerHeadRecordFetcher(pub Arc<dyn ReconcilePeers>);

#[async_trait::async_trait]
impl HeadRecordFetcher for OwnedPeerHeadRecordFetcher {
    async fn fetch(&self, peer_id: &str, content_id: &str) -> Answer<CarriedHeadRecord> {
        PeerHeadRecordFetcher::new(self.0.as_ref())
            .fetch(peer_id, content_id)
            .await
    }
}

/// Byte presence over this node's blob store and shard manifests; requests go
/// to the P2P event loop as [`P2PCommand::HealBlobBytes`].
pub struct NodeBytePresence {
    pub blob_store: Arc<BlobStore>,
    pub pool: DbPool,
    pub command_tx: mpsc::Sender<P2PCommand>,
}

/// `sha256-<hex>` for any address form the blob plane accepts, or `None`.
fn legacy_hash(address: &str) -> Option<String> {
    BlobStore::parse_content_address(address)
        .ok()
        .map(|hex| format!("sha256-{hex}"))
}

#[async_trait::async_trait]
impl BytePresence for NodeBytePresence {
    async fn held(&self, address: &str) -> bool {
        let Some(hash) = legacy_hash(address) else {
            return false;
        };
        if self.blob_store.exists(&hash).await {
            return true;
        }
        let manifest = self.pool.get().ok().and_then(|mut conn| {
            crate::db::shard_manifests::get_manifest_by_blob_hash(&mut conn, &hash)
                .ok()
                .flatten()
                .and_then(|row| crate::db::shard_manifests::hydrate_manifest(&row).ok())
        });
        match manifest {
            Some(m) => self.blob_store.holds_as_shards(&m).await,
            None => false,
        }
    }

    fn request(&self, origin: &str, address: &str, holder: &str) {
        let Some(hash) = legacy_hash(address) else {
            return;
        };
        // try_send: a full command queue drops the request; the trigger's ladder
        // re-checks presence and asks again on its next rung.
        let _ = self.command_tx.try_send(P2PCommand::HealBlobBytes {
            origin: origin.to_string(),
            hash,
            holder_hint: Some(holder.to_string()),
        });
    }
}
