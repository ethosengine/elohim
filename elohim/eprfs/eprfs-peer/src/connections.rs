//! The connections this device accepted, by the authenticated node id, and the ones it refused
//! (spec §3.1: a Contest takes effect with no restart).
//!
//! The tier and the gossip ALPN record each connection they accept here. When a roster reload
//! leaves a node id unadmitted, [`ConnectionBook::close_unadmitted`] closes every open connection
//! of that node with [`NOT_ADMITTED`], so a retired device keeps nothing it had open; its new
//! connections then meet the same refusal a stranger's do. A refusal is counted per node id only
//! for node ids the roster names (its devices, admitted or contested); every other node id — free
//! to mint — folds into one strangers total, so a stranger cannot grow this map. The daemon
//! persists both in its head table, where `epr peer status` reads them.

use std::collections::{BTreeMap, BTreeSet, HashMap};
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
    strangers: u64,
    /// Node ids (64 lowercase hex) the roster names: the only ones counted one by one.
    named: BTreeSet<String>,
}

impl Book {
    fn count(&mut self, node: &NodeId) {
        let hex = node_id_hex(node);
        if self.named.contains(&hex) {
            let n = self.refused.entry(hex).or_default();
            *n = n.saturating_add(1);
        } else {
            self.strangers = self.strangers.saturating_add(1);
        }
    }
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

    /// The node ids the roster names (64 lowercase hex), replacing the last set: refusals of
    /// these are counted one by one, every other node id's as a stranger's.
    pub fn name(&self, nodes: impl IntoIterator<Item = String>) {
        if let Ok(mut book) = self.inner.lock() {
            book.named = nodes.into_iter().collect();
        }
    }

    /// Count one connection refused from `node`.
    pub fn refused(&self, node: NodeId) {
        if let Ok(mut book) = self.inner.lock() {
            book.count(&node);
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
                    book.count(&node);
                    closed += 1;
                }
            }
        }
        closed
    }

    /// Refused connections so far, per roster-named node id (64 lowercase hex).
    pub fn refused_counts(&self) -> BTreeMap<String, u64> {
        self.inner
            .lock()
            .map(|book| book.refused.clone())
            .unwrap_or_default()
    }

    /// Refused connections so far from node ids the roster does not name.
    pub fn refused_strangers(&self) -> u64 {
        self.inner
            .lock()
            .map(|book| book.strangers)
            .unwrap_or_default()
    }
}
