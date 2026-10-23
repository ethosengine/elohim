//! The connections this device accepted, by the authenticated node id, and the ones it refused
//! (spec §3.1: a Contest takes effect with no restart).
//!
//! The tier and the gossip ALPN record each connection they accept here. When a roster reload
//! leaves a node id unadmitted, [`ConnectionBook::close_unadmitted`] closes every open connection
//! of that node with [`NOT_ADMITTED`], so a retired device keeps nothing it had open; its new
//! connections then meet the same refusal a stranger's do. Every refusal is counted per node id,
//! and the daemon persists the counts in its head table, where `epr peer status` reads them.

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use iroh::endpoint::{Connection, VarInt};
use iroh::NodeId;

use crate::admission::{node_id_hex, Admission};

/// The QUIC application code a connection from a node id this device does not admit is closed
/// with: a stranger's at once, an admitted device's when a reload drops it.
pub const NOT_ADMITTED: u32 = 1;

/// Open connections by node id, and refusals counted per node id. `Clone` shares the book.
#[derive(Debug, Clone, Default)]
pub struct ConnectionBook {
    inner: Arc<Mutex<Book>>,
}

#[derive(Debug, Default)]
struct Book {
    open: HashMap<NodeId, Vec<Connection>>,
    refused: BTreeMap<String, u64>,
}

impl ConnectionBook {
    /// Keep `connection`, accepted from the admitted `node`, so a reload can close it.
    pub fn admitted(&self, node: NodeId, connection: &Connection) {
        if let Ok(mut book) = self.inner.lock() {
            let open = book.open.entry(node).or_default();
            open.retain(|kept| kept.close_reason().is_none());
            open.push(connection.clone());
        }
    }

    /// Count one connection refused from `node`.
    pub fn refused(&self, node: NodeId) {
        if let Ok(mut book) = self.inner.lock() {
            *book.refused.entry(node_id_hex(&node)).or_default() += 1;
        }
    }

    /// Close every open connection of a node id `admission` no longer admits, counting each as
    /// refused. Returns how many were closed.
    pub fn close_unadmitted(&self, admission: &Admission) -> usize {
        let Ok(mut book) = self.inner.lock() else {
            return 0;
        };
        let dropped: Vec<NodeId> = book
            .open
            .keys()
            .filter(|node| admission.admits(node).is_none())
            .copied()
            .collect();
        let mut closed = 0;
        for node in dropped {
            for connection in book.open.remove(&node).unwrap_or_default() {
                if connection.close_reason().is_none() {
                    connection.close(VarInt::from_u32(NOT_ADMITTED), b"not admitted");
                    *book.refused.entry(node_id_hex(&node)).or_default() += 1;
                    closed += 1;
                }
            }
        }
        closed
    }

    /// Refused connections so far, per node id (64 lowercase hex).
    pub fn refused_counts(&self) -> BTreeMap<String, u64> {
        self.inner
            .lock()
            .map(|book| book.refused.clone())
            .unwrap_or_default()
    }
}
