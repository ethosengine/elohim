//! Signal bus — cross-relay forwarding for the SBD WebRTC-signal plane.
//!
//! ## Why this exists (the partition it heals)
//!
//! The signal relay ([`super::store::SignalStore`]) holds LIVE in-memory WebSocket
//! write-halves (`DashMap<PubKey, WsSink>`). A relay can only forward a message
//! between two peers that are BOTH connected to the same relay instance. When the
//! genesis pair split across two doorway pods — adam on `signal.elohim.host`,
//! matthew on `signal.doorway-alpha.elohim.host` — adam's handshake message for
//! matthew hit adam's relay, which had no live connection to matthew, so it was
//! silently dropped ([`super::mod`] relay loop, the `store.get(&dest)` local-miss).
//! Two peers that discover each other via the shared bootstrap table still could
//! not complete a WebRTC handshake. That is the `signalShared:false` partition the
//! Tier-C diagnostic surfaces.
//!
//! ## The design (D2 — shared bus, NOT a MongoK2Store-style table)
//!
//! Signal is a LIVE relay, not a persistable PUT/GET table, so the bootstrap fix
//! (share a Mongo table of presence records) does not transfer. Instead: on a
//! local-miss the origin relay PUBLISHES the message to a shared bus; every relay
//! in the domain drains the bus and DELIVERS any message whose dest is connected
//! locally. The bus is the shared backing; the WebSocket sinks stay per-pod.
//!
//! **Loop-free by construction**: only an ORIGIN relay (one that received the
//! message from a live WS client and missed locally) publishes. A relay draining
//! the bus only ever delivers-if-local — it never re-publishes. Each relay also
//! filters out its OWN publishes on drain (`origin_relay` match), so a relay never
//! reprocesses what it emitted.
//!
//! **Bounded blast radius**: the bus broadcasts within a DOMAIN (a household, the
//! commons/genesis peers) — a handful of relays, per the fractal-federation design
//! (`genesis/docs/superpowers/specs/2026-07-09-peer-discovery-fractal-federation-design.md`).
//! Cross-domain signal is Tier-B (federated), never one global bus.
//!
//! **Source of truth**: OPERATIONAL (Category C). A bus message is ephemeral relay
//! routing state — reconstructable (peers retry handshakes), TTL-expired, never
//! notarized, no `dht_anchor_hash`. Keyed only by the ed25519 SIGNAL pubkey (a
//! transport-plane identity — NEVER string-joined against `agent_cid`).

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// A message that could not be delivered locally and was handed to the bus. The
/// `payload` is the full SBD frame with the SENDER pubkey already swapped into the
/// 32-byte header (done by the relay loop before publish), so a sibling relay can
/// deliver it verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BusMessage {
    /// Destination signal pubkey (ed25519) — whom to deliver to if connected locally.
    pub dest: [u8; 32],
    /// The SBD frame to deliver verbatim (sender header already applied).
    pub payload: Vec<u8>,
}

/// The cross-relay signal bus. Mirrors the `K2Store` trait shape: one trait, an
/// in-process `MemSignalBus` (single-pod / dev / tests) and a shared-backed
/// `MongoSignalBus` (the real cross-relay backing) implement it.
#[async_trait::async_trait]
pub trait SignalBus: Send + Sync {
    /// Publish a message this relay could not deliver locally. Tagged with the
    /// publishing relay's id so the publisher filters it back out on drain.
    async fn publish(&self, dest: &[u8; 32], payload: &[u8]);

    /// Drain messages published AFTER `cursor` by OTHER relays. Returns the new
    /// messages and the advanced cursor (pass it back on the next call). Messages
    /// published by THIS relay are excluded (loop-free). The cursor is opaque:
    /// an in-process sequence for `MemSignalBus`, a wall-clock micros for Mongo.
    async fn drain(&self, cursor: i64) -> (Vec<BusMessage>, i64);

    /// Backend name for the `/health dht_backing` diagnostic (`mem` vs `mongo`).
    fn backend_name(&self) -> &'static str;
}

/// Shared in-process log backing one or more `MemSignalBus` handles. In production
/// each pod gets a PRIVATE inner (no siblings → drain is always empty → cross-relay
/// forwarding is inertly a no-op, which is correct for a lone pod). Tests share one
/// inner between two handles with distinct relay ids to exercise cross-relay semantics.
/// One entry in the in-process bus log.
struct MemEntry {
    seq: i64,
    origin_relay: String,
    dest: [u8; 32],
    payload: Vec<u8>,
}

#[derive(Default)]
pub struct MemBusInner {
    log: Mutex<Vec<MemEntry>>,
    seq: AtomicI64,
}

/// In-process signal bus. A lone pod's handle has no siblings, so it never delivers
/// cross-relay (correct). Shareable via `with_shared` for tests.
pub struct MemSignalBus {
    relay_id: String,
    inner: Arc<MemBusInner>,
    /// Cap to bound dev/test growth (the Mongo impl uses a TTL index instead).
    cap: usize,
}

impl MemSignalBus {
    pub fn new(relay_id: impl Into<String>) -> Self {
        Self {
            relay_id: relay_id.into(),
            inner: Arc::new(MemBusInner::default()),
            cap: 4096,
        }
    }

    /// Construct a handle that SHARES an existing inner log — models a second relay
    /// on the same bus. Test-facing (and the honest model of "shared backing").
    pub fn with_shared(relay_id: impl Into<String>, inner: Arc<MemBusInner>) -> Self {
        Self {
            relay_id: relay_id.into(),
            inner,
            cap: 4096,
        }
    }

    pub fn inner(&self) -> Arc<MemBusInner> {
        Arc::clone(&self.inner)
    }
}

#[async_trait::async_trait]
impl SignalBus for MemSignalBus {
    async fn publish(&self, dest: &[u8; 32], payload: &[u8]) {
        let seq = self.inner.seq.fetch_add(1, Ordering::SeqCst) + 1;
        let mut log = self.inner.log.lock().await;
        log.push(MemEntry {
            seq,
            origin_relay: self.relay_id.clone(),
            dest: *dest,
            payload: payload.to_vec(),
        });
        // Bound growth: drop the oldest beyond cap (dev/test only).
        let overflow = log.len().saturating_sub(self.cap);
        if overflow > 0 {
            log.drain(0..overflow);
        }
    }

    async fn drain(&self, cursor: i64) -> (Vec<BusMessage>, i64) {
        let log = self.inner.log.lock().await;
        let mut out = Vec::new();
        let mut new_cursor = cursor;
        for e in log.iter() {
            if e.seq <= cursor {
                continue;
            }
            new_cursor = new_cursor.max(e.seq);
            if e.origin_relay == self.relay_id {
                continue; // never redeliver our own publishes (loop-free)
            }
            out.push(BusMessage {
                dest: e.dest,
                payload: e.payload.clone(),
            });
        }
        (out, new_cursor)
    }

    fn backend_name(&self) -> &'static str {
        "mem"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dest(b: u8) -> [u8; 32] {
        [b; 32]
    }

    /// A sibling relay drains what another relay published (the cross-relay heal),
    /// and the cursor advances so a re-drain returns nothing new.
    #[tokio::test]
    async fn sibling_relay_drains_a_published_message() {
        let inner = Arc::new(MemBusInner::default());
        let relay_a = MemSignalBus::with_shared("relay-a", Arc::clone(&inner));
        let relay_b = MemSignalBus::with_shared("relay-b", inner);

        relay_a.publish(&dest(7), b"handshake-for-matthew").await;

        let (msgs, cursor) = relay_b.drain(0).await;
        assert_eq!(msgs.len(), 1, "sibling must see the published message");
        assert_eq!(msgs[0].dest, dest(7));
        assert_eq!(msgs[0].payload, b"handshake-for-matthew");

        let (again, _) = relay_b.drain(cursor).await;
        assert!(again.is_empty(), "advanced cursor must not re-deliver");
    }

    /// A relay NEVER drains its own publishes — the loop-free invariant.
    #[tokio::test]
    async fn a_relay_never_drains_its_own_publishes() {
        let inner = Arc::new(MemBusInner::default());
        let relay_a = MemSignalBus::with_shared("relay-a", inner);

        relay_a.publish(&dest(1), b"mine").await;
        let (msgs, _) = relay_a.drain(0).await;
        assert!(
            msgs.is_empty(),
            "origin relay must filter out its own publishes (loop-free)"
        );
    }

    /// A lone pod (private inner, no siblings) drains nothing — cross-relay
    /// forwarding is inertly a no-op, which is correct for a single relay.
    #[tokio::test]
    async fn lone_pod_has_no_siblings_and_drains_nothing() {
        let lone = MemSignalBus::new("solo");
        lone.publish(&dest(2), b"x").await;
        let (msgs, _) = lone.drain(0).await;
        assert!(msgs.is_empty(), "a lone pod has no sibling to deliver to");
    }

    /// Multiple messages drain in order and only those strictly after the cursor.
    #[tokio::test]
    async fn drain_returns_only_messages_after_cursor() {
        let inner = Arc::new(MemBusInner::default());
        let a = MemSignalBus::with_shared("a", Arc::clone(&inner));
        let b = MemSignalBus::with_shared("b", inner);

        a.publish(&dest(1), b"one").await;
        let (first, c1) = b.drain(0).await;
        assert_eq!(first.len(), 1);
        a.publish(&dest(2), b"two").await;
        a.publish(&dest(3), b"three").await;
        let (rest, _) = b.drain(c1).await;
        assert_eq!(rest.len(), 2, "only the two published after the cursor");
        assert_eq!(rest[0].dest, dest(2));
        assert_eq!(rest[1].dest, dest(3));
    }

    #[test]
    fn backend_name_is_mem() {
        assert_eq!(MemSignalBus::new("x").backend_name(), "mem");
    }
}
