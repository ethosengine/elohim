//! `IrohPeerBook` — the in-memory set of iroh peers this node can dial.
//!
//! The iroh `Endpoint` only dials peers whose `NodeAddr` it has been told
//! about (`add_node_addr`) or can resolve through a discovery service. With
//! sovereign defaults (no pkarr resolver, no n0) that set is EMPTY at boot —
//! which is why the iroh plane was an idle listener on every dual-mode node
//! until 2026-08-23: every responder mounted, nothing ever dialed.
//!
//! This book is the single place "who can I reach over iroh" lives at
//! runtime. Producers (the `elohim/transport/manifest` gossip receive arm,
//! later pkarr/doorway discovery) `upsert` entries; consumers (the gossip
//! joiner, the iroh sync-round driver, blob/shard fetch) `snapshot` it and
//! `subscribe` to change notifications. The durable projection of the same
//! facts is `peer_transport_manifest` (see [`super::peer_map`]) — the book is
//! the hot working set, the table is the record.
//!
//! Invariant: an entry is only ever inserted from a manifest whose signature
//! verified against `node_id` (see
//! `crate::p2p::transport_manifest_gossip`). A peer can therefore only ever
//! advertise addresses for a NodeId it holds the key for — no third party can
//! redirect our dials.

use std::collections::BTreeMap;
use std::sync::{Arc, RwLock};

use iroh::{NodeAddr, NodeId};
use tokio::sync::watch;

/// One dialable iroh peer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrohPeerEntry {
    /// Full dial target (node id + relay url + direct addresses).
    pub addr: NodeAddr,
    /// The agent CID the peer announced itself under (DHT-anchored
    /// identity). `None` when the announcer did not bind one.
    pub agent_cid: Option<String>,
    /// The peer's libp2p PeerId (base58), when it runs dual — lets the
    /// cross-plane dedup and the `/p2p/status` projection join both legs.
    pub libp2p_peer_id: Option<String>,
    /// Wall-clock ms of the announcement this entry came from. A later
    /// announcement replaces an earlier one; an older one never does.
    pub announced_at_ms: i64,
}

#[derive(Debug)]
struct Inner {
    peers: RwLock<BTreeMap<NodeId, IrohPeerEntry>>,
    /// Bumped on every mutation; consumers `subscribe()` and `changed().await`.
    version_tx: watch::Sender<u64>,
}

/// Cloneable handle to the shared book.
#[derive(Debug, Clone)]
pub struct IrohPeerBook {
    inner: Arc<Inner>,
}

impl Default for IrohPeerBook {
    fn default() -> Self {
        Self::new()
    }
}

impl IrohPeerBook {
    pub fn new() -> Self {
        let (version_tx, _rx) = watch::channel(0u64);
        Self {
            inner: Arc::new(Inner {
                peers: RwLock::new(BTreeMap::new()),
                version_tx,
            }),
        }
    }

    /// Insert or refresh a peer. Returns `true` when the book changed
    /// (new peer, or a fresher announcement for a known peer). An
    /// announcement older than the one already held is ignored.
    pub fn upsert(&self, entry: IrohPeerEntry) -> bool {
        let node_id = entry.addr.node_id;
        let changed = {
            let mut peers = self.inner.peers.write().expect("peer book poisoned");
            match peers.get(&node_id) {
                Some(existing) if existing.announced_at_ms > entry.announced_at_ms => false,
                Some(existing) if *existing == entry => false,
                _ => {
                    peers.insert(node_id, entry);
                    true
                }
            }
        };
        if changed {
            self.inner.version_tx.send_modify(|v| *v += 1);
        }
        changed
    }

    /// Drop a peer (e.g. after repeated dial failures). Returns `true` if it
    /// was present.
    pub fn remove(&self, node_id: &NodeId) -> bool {
        let removed = self
            .inner
            .peers
            .write()
            .expect("peer book poisoned")
            .remove(node_id)
            .is_some();
        if removed {
            self.inner.version_tx.send_modify(|v| *v += 1);
        }
        removed
    }

    pub fn get(&self, node_id: &NodeId) -> Option<IrohPeerEntry> {
        self.inner
            .peers
            .read()
            .expect("peer book poisoned")
            .get(node_id)
            .cloned()
    }

    /// All known peers, NodeId-ordered. Never includes `exclude` (pass our
    /// own NodeId so a node never dials itself).
    pub fn snapshot(&self, exclude: Option<&NodeId>) -> Vec<IrohPeerEntry> {
        self.inner
            .peers
            .read()
            .expect("peer book poisoned")
            .iter()
            .filter(|(id, _)| Some(*id) != exclude)
            .map(|(_, e)| e.clone())
            .collect()
    }

    pub fn node_ids(&self, exclude: Option<&NodeId>) -> Vec<NodeId> {
        self.snapshot(exclude)
            .into_iter()
            .map(|e| e.addr.node_id)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.inner.peers.read().expect("peer book poisoned").len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// A receiver that wakes (`changed().await`) whenever the book mutates.
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.inner.version_tx.subscribe()
    }
}

/// Convert a verified announcement into a dial target. `None` when the
/// NodeId or relay URL does not parse (verification already checked the
/// hex shape and the direct addresses, so this is belt-and-braces).
pub fn entry_from_announcement(
    ann: &crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement,
) -> Option<IrohPeerEntry> {
    let node_id: NodeId = ann.iroh_node_id.parse().ok()?;
    let mut addr = NodeAddr::new(node_id).with_direct_addresses(ann.direct_socket_addrs());
    if let Some(relay) = ann.iroh_relay_url.as_deref() {
        addr = addr.with_relay_url(relay.parse().ok()?);
    }
    Some(IrohPeerEntry {
        addr,
        agent_cid: ann.agent_cid.clone(),
        libp2p_peer_id: ann.libp2p_peer_id.clone(),
        announced_at_ms: ann.announced_at_ms,
    })
}

impl crate::p2p::gossip_dispatch::TransportManifestSink for IrohPeerBook {
    fn accept(
        &self,
        ann: &crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement,
    ) -> bool {
        let Some(entry) = entry_from_announcement(ann) else {
            return false;
        };
        let changed = self.upsert(entry);
        crate::metrics::set_iroh_peers_known(self.len());
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use iroh::SecretKey;

    #[test]
    fn announcement_becomes_a_dialable_entry() {
        use crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement;
        let secret = [3u8; 32];
        let ann = TransportManifestAnnouncement::sign(
            &secret,
            vec!["127.0.0.1:10702".into(), "[::1]:10702".into()],
            None,
            Some("agent-j".into()),
            Some("12D3KooWj".into()),
            vec!["sync".into()],
            42,
        );
        assert_eq!(ann.verify(), Ok(()));
        let entry = entry_from_announcement(&ann).expect("parses");
        assert_eq!(entry.addr.node_id.to_string(), ann.iroh_node_id);
        assert_eq!(entry.addr.direct_addresses.len(), 2);
        assert_eq!(entry.agent_cid.as_deref(), Some("agent-j"));
        let book = IrohPeerBook::new();
        use crate::p2p::gossip_dispatch::TransportManifestSink;
        assert!(book.accept(&ann));
        assert!(!book.accept(&ann));
        assert_eq!(book.len(), 1);
    }

    fn entry(key: &SecretKey, at: i64, port: u16) -> IrohPeerEntry {
        IrohPeerEntry {
            addr: NodeAddr::new(key.public())
                .with_direct_addresses([([127, 0, 0, 1], port).into()]),
            agent_cid: Some("agent-a".into()),
            libp2p_peer_id: None,
            announced_at_ms: at,
        }
    }

    #[test]
    fn upsert_is_monotone_on_announced_at() {
        let book = IrohPeerBook::new();
        let mut rng = rand::rngs::OsRng;
        let key = SecretKey::generate(&mut rng);
        assert!(book.upsert(entry(&key, 10, 1)));
        // same content, same time: no change
        assert!(!book.upsert(entry(&key, 10, 1)));
        // older announcement never replaces
        assert!(!book.upsert(entry(&key, 5, 2)));
        assert_eq!(
            book.get(&key.public()).unwrap().addr.direct_addresses.len(),
            1
        );
        // fresher one does
        assert!(book.upsert(entry(&key, 20, 3)));
        let got = book.get(&key.public()).unwrap();
        assert!(got
            .addr
            .direct_addresses
            .contains(&([127, 0, 0, 1], 3u16).into()));
    }

    #[test]
    fn snapshot_excludes_self_and_version_bumps_on_change() {
        let book = IrohPeerBook::new();
        let mut rx = book.subscribe();
        let mut rng = rand::rngs::OsRng;
        let me = SecretKey::generate(&mut rng);
        let other = SecretKey::generate(&mut rng);
        book.upsert(entry(&me, 1, 1));
        book.upsert(entry(&other, 1, 2));
        assert_eq!(book.len(), 2);
        assert_eq!(book.snapshot(Some(&me.public())).len(), 1);
        assert!(rx.has_changed().unwrap());
        rx.mark_unchanged();
        assert!(!book.upsert(entry(&other, 1, 2)));
        assert!(!rx.has_changed().unwrap());
        assert!(book.remove(&other.public()));
        assert!(rx.has_changed().unwrap());
    }
}
