//! Iroh inventory-receive bridge into the shared bounded fetch-work queue.
//!
//! This module does not fetch bytes over iroh. In dual mode it submits the
//! same [`crate::p2p::P2PCommand::InventoryAdvertisement`] drained by the
//! libp2p event loop; that loop remains the sole owner of commitment scoring
//! and byte-fetch scheduling until the dedicated iroh blob-fetch cut lands.
//! In pure-iroh mode no such consumer exists yet, so the bridge counts and
//! warns about the explicit absence instead of silently discarding the work.

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::p2p::gossip_dispatch::InventoryFetch;
use crate::p2p::inventory_gossip::BlobHint;
use crate::p2p::P2PCommand;

/// Process-lifetime counters for the iroh → fetch-work bridge.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InventoryFetchBridgeStats {
    pub enqueued: u64,
    pub unavailable: u64,
    pub saturated: u64,
    pub closed: u64,
}

#[derive(Debug, Default)]
struct BridgeCounters {
    enqueued: AtomicU64,
    unavailable: AtomicU64,
    saturated: AtomicU64,
    closed: AtomicU64,
}

/// Cloneable hook installed in every iroh gossip receive task.
#[derive(Clone)]
pub struct IrohInventoryFetch {
    command_tx: Option<mpsc::Sender<P2PCommand>>,
    counters: Arc<BridgeCounters>,
}

impl IrohInventoryFetch {
    pub fn new(command_tx: Option<mpsc::Sender<P2PCommand>>) -> Self {
        Self {
            command_tx,
            counters: Arc::new(BridgeCounters::default()),
        }
    }

    pub fn stats(&self) -> InventoryFetchBridgeStats {
        InventoryFetchBridgeStats {
            enqueued: self.counters.enqueued.load(Ordering::Relaxed),
            unavailable: self.counters.unavailable.load(Ordering::Relaxed),
            saturated: self.counters.saturated.load(Ordering::Relaxed),
            closed: self.counters.closed.load(Ordering::Relaxed),
        }
    }

    fn enqueue(&self, work: P2PCommand, source_peer_id: &str, item_count: usize) {
        let Some(command_tx) = self.command_tx.as_ref() else {
            self.counters.unavailable.fetch_add(1, Ordering::Relaxed);
            warn!(
                target: "elohim_storage::inventory",
                peer_id = %source_peer_id,
                count = item_count,
                "iroh inventory learned but no byte-fetch queue exists in pure-iroh mode"
            );
            return;
        };

        match command_tx.try_send(work) {
            Ok(()) => {
                self.counters.enqueued.fetch_add(1, Ordering::Relaxed);
                debug!(
                    target: "elohim_storage::inventory",
                    peer_id = %source_peer_id,
                    count = item_count,
                    "iroh inventory fetch work queued for the shared scorer"
                );
            }
            Err(mpsc::error::TrySendError::Full(_)) => {
                self.counters.saturated.fetch_add(1, Ordering::Relaxed);
                warn!(
                    target: "elohim_storage::inventory",
                    peer_id = %source_peer_id,
                    count = item_count,
                    "iroh inventory fetch work dropped: bounded command queue saturated"
                );
            }
            Err(mpsc::error::TrySendError::Closed(_)) => {
                self.counters.closed.fetch_add(1, Ordering::Relaxed);
                warn!(
                    target: "elohim_storage::inventory",
                    peer_id = %source_peer_id,
                    count = item_count,
                    "iroh inventory fetch work dropped: command queue closed"
                );
            }
        }
    }
}

impl InventoryFetch for IrohInventoryFetch {
    fn score_and_enqueue(
        &self,
        _conn: &mut diesel::SqliteConnection,
        source_peer_id: &str,
        hints: &[BlobHint],
        hashes: &[String],
    ) {
        self.enqueue(
            P2PCommand::InventoryAdvertisement {
                source_peer_id: source_peer_id.to_string(),
                hints: hints.to_vec(),
                hashes: hashes.to_vec(),
            },
            source_peer_id,
            hashes.len(),
        );
    }

    fn request_snapshot_for_gap(&self, peer_id: &str) {
        let Ok(peer_id_parsed) = peer_id.parse::<libp2p::PeerId>() else {
            self.counters.unavailable.fetch_add(1, Ordering::Relaxed);
            warn!(
                target: "elohim_storage::inventory",
                peer_id,
                "iroh inventory delta gap cannot request a libp2p snapshot for this peer id"
            );
            return;
        };
        self.enqueue(
            P2PCommand::SnapshotRequest {
                peer_id: peer_id_parsed,
            },
            peer_id,
            1,
        );
    }
}
