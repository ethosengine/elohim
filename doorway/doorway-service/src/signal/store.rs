//! Signal connection store
//!
//! Manages active WebSocket connections indexed by public key.
//! Provides thread-safe lookup for message forwarding.

use dashmap::DashMap;
use futures_util::stream::SplitSink;
use hyper_tungstenite::WebSocketStream;
use hyper_util::rt::TokioIo;
use std::net::Ipv6Addr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;
use tracing::debug;

use super::bus::{MemSignalBus, SignalBus};
use super::bus_mongo::MongoSignalBus;
use super::PubKey;
use crate::db::mongo::MongoClient;

/// Type alias for the WebSocket write half
pub type WsSink =
    Arc<Mutex<SplitSink<WebSocketStream<TokioIo<hyper::upgrade::Upgraded>>, Message>>>;

/// Connection entry in the store
struct ConnectionEntry {
    /// WebSocket write half for sending messages
    write: WsSink,
    /// Client IP address (for rate limiting)
    #[allow(dead_code)]
    ip: Arc<Ipv6Addr>,
}

/// Signal connection store
///
/// Thread-safe store for active signal connections, indexed by public key.
pub struct SignalStore {
    /// Active connections indexed by public key
    connections: DashMap<PubKey, ConnectionEntry>,
    /// Current connection count
    count: AtomicUsize,
    /// Maximum allowed connections
    max_connections: usize,
    /// Cross-relay signal bus — where frames for non-locally-connected dests are
    /// published so a sibling relay can deliver them (D2). Mem (lone-pod no-op) or
    /// the shared Mongo bus, selected exactly like `BootstrapStore`'s k2 backend.
    bus: Arc<dyn SignalBus>,
}

impl SignalStore {
    /// Create a new signal store with the given capacity.
    ///
    /// Safe-by-default: the signal bus is the in-process [`MemSignalBus`] (a lone
    /// pod has no siblings, so cross-relay forwarding is inertly a no-op) UNLESS a
    /// `mongo` client is provided AND `BOOTSTRAP_MONGODB_DB` is set — then the
    /// shared [`MongoSignalBus`] on that same shared DB, so a domain's relays
    /// forward handshake frames to each other. `relay_id` MUST identify one
    /// running relay instance (not a logical doorway shared by replicas): it
    /// tags this relay's publishes so only that instance filters them back out
    /// on drain (loop-free).
    pub fn new(max_connections: usize, mongo: Option<&MongoClient>, relay_id: String) -> Self {
        let bus: Arc<dyn SignalBus> = match (mongo, std::env::var("BOOTSTRAP_MONGODB_DB").ok()) {
            (Some(m), Some(db)) if !db.is_empty() => {
                Arc::new(MongoSignalBus::new(m, &db, relay_id))
            }
            _ => Arc::new(MemSignalBus::new(relay_id)),
        };
        Self {
            connections: DashMap::with_capacity(max_connections),
            count: AtomicUsize::new(0),
            max_connections,
            bus,
        }
    }

    /// The cross-relay signal bus (relay loop publishes local-misses here).
    pub fn bus(&self) -> &Arc<dyn SignalBus> {
        &self.bus
    }

    /// Check if the store is at capacity
    pub fn is_at_capacity(&self) -> bool {
        self.count.load(Ordering::Relaxed) >= self.max_connections
    }

    /// Get the current connection count
    pub fn connection_count(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Insert a new connection
    ///
    /// If a connection with the same pubkey exists, it will be replaced.
    pub fn insert(&self, pk: PubKey, write: WsSink, ip: Arc<Ipv6Addr>) {
        let entry = ConnectionEntry { write, ip };

        // Check if we're replacing an existing connection
        let was_present = self.connections.insert(pk.clone(), entry).is_some();

        if !was_present {
            self.count.fetch_add(1, Ordering::Relaxed);
        }

        debug!(
            "Signal store: inserted {:?}, count={}",
            pk,
            self.count.load(Ordering::Relaxed)
        );
    }

    /// Remove a connection by public key
    pub fn remove(&self, pk: &PubKey) {
        if self.connections.remove(pk).is_some() {
            self.count.fetch_sub(1, Ordering::Relaxed);
            debug!(
                "Signal store: removed {:?}, count={}",
                pk,
                self.count.load(Ordering::Relaxed)
            );
        }
    }

    /// Get the write half of a connection by public key
    pub fn get(&self, pk: &PubKey) -> Option<WsSink> {
        self.connections
            .get(pk)
            .map(|entry| Arc::clone(&entry.write))
    }

    /// Check if a public key is connected
    pub fn contains(&self, pk: &PubKey) -> bool {
        self.connections.contains_key(pk)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use std::net::Ipv6Addr;

    fn make_pk(byte: u8) -> PubKey {
        PubKey(Arc::new([byte; 32]))
    }

    #[test]
    fn test_store_capacity() {
        let store = SignalStore::new(2, None, "test-relay".to_string());

        assert!(!store.is_at_capacity());
        assert_eq!(store.connection_count(), 0);
    }

    #[test]
    fn test_store_contains() {
        let store = SignalStore::new(10, None, "test-relay".to_string());
        let pk = make_pk(1);

        assert!(!store.contains(&pk));
    }
}
