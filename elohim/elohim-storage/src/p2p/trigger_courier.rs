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

/// Byte presence over this node's blob store and shard manifests. Requests go
/// to the libp2p event loop as [`P2PCommand::HealBlobBytes`] when that plane
/// exists, else over the iroh fetch leg to the courier directly.
pub struct NodeBytePresence {
    pub blob_store: Arc<BlobStore>,
    pub pool: DbPool,
    /// The libp2p plane's command queue; `None` on a pure-iroh node.
    pub command_tx: Option<mpsc::Sender<P2PCommand>>,
    /// This node's steward identity, which books an iroh-fetched blob the way
    /// every other fetch is booked (`finalize_fetch_success`).
    pub self_cid: String,
}

/// Iroh byte requests in flight at once. A request past the bound is dropped:
/// the trigger re-checks presence on its next rung and asks again.
#[cfg(feature = "p2p-iroh")]
static IROH_BYTE_REQUESTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);

/// One dial, one answer: the same per-peer bound the heal-on-read race uses.
#[cfg(feature = "p2p-iroh")]
const IROH_BYTE_REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);

impl NodeBytePresence {
    /// Fetch `hash` from `holder` over the iroh leg and book it. Pure-iroh
    /// nodes have no libp2p event loop to send `HealBlobBytes` to; without
    /// this, the bytes gate would hold a head move until the sweep.
    #[cfg(feature = "p2p-iroh")]
    fn request_over_iroh(&self, origin: &str, hash: String, holder: &str) {
        let Ok(permit) = IROH_BYTE_REQUESTS.try_acquire() else {
            return;
        };
        let (blob_store, pool, self_cid) = (
            self.blob_store.clone(),
            self.pool.clone(),
            self.self_cid.clone(),
        );
        let (origin, holder) = (origin.to_string(), holder.to_string());
        tokio::spawn(async move {
            let _permit = permit;
            // No libp2p plane: an inert sender, and no peer counts as connected,
            // so only the iroh leg is planned.
            let (inert, _rx) = mpsc::channel(1);
            let outcome = crate::p2p::blob_swarm::race_fetch_dual(
                &hash,
                vec![holder],
                &inert,
                |_| false,
                1,
                IROH_BYTE_REQUEST_TIMEOUT,
            )
            .await;
            let crate::p2p::blob_fetch::FetchOutcome::Hit { bytes, source_peer } = outcome else {
                tracing::debug!(origin = %origin, hash = %hash, "iroh byte request: no bytes");
                return;
            };
            let Ok(mut conn) = pool.get() else {
                return;
            };
            if let Err(error) = crate::p2p::blob_fetch::finalize_fetch_success(
                &mut conn,
                &hash,
                &source_peer,
                &bytes,
                &self_cid,
                &blob_store,
            )
            .await
            {
                tracing::warn!(origin = %origin, hash = %hash, error = %error, "iroh byte request: store failed");
            }
        });
    }
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
        let Some(command_tx) = self.command_tx.as_ref() else {
            #[cfg(feature = "p2p-iroh")]
            self.request_over_iroh(origin, hash, holder);
            return;
        };
        // try_send: a full command queue drops the request; the trigger's ladder
        // re-checks presence and asks again on its next rung.
        let _ = command_tx.try_send(P2PCommand::HealBlobBytes {
            origin: origin.to_string(),
            hash,
            holder_hint: Some(holder.to_string()),
        });
    }
}
