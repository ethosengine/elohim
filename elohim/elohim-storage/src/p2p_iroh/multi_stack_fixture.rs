/*!
 * Multi-stack fixture for cross-transport recovery e2e (gate #5).
 *
 * Extends `TwoNodeFixture` to a 5-node mesh with per-node transport profile
 * assignment. Gates iroh-only / libp2p-only / dual-stack participation per
 * node so the cross-stack scenarios can exercise the share-holder transport-mix
 * matrix from:
 *   genesis/a2o/features/auth/recovery/cross-stack/recovery-cross-stack-transport.feature
 *
 * ## Architecture
 *
 * Each [`NodeSlot`] owns whichever transports its [`TransportProfile`] declares:
 *
 *   - `IrohOnly`   — a real `IrohNode` running loopback (no relay); no libp2p handle.
 *   - `Libp2pOnly` — a `P2PHandle::for_testing()` stub (no iroh node).
 *   - `Dual`       — both: a real `IrohNode` **and** a `P2PHandle::for_testing()` stub.
 *
 * In production, `DualGossipPublisher` fans out gossip to both stacks simultaneously
 * (Plan 4, permanent post-cutover per spec gate #4). In this test fixture the libp2p
 * side is a `CaptureMock`-style stub — enough for cross-stack delivery assertions
 * via `DualGossipPublisher`, which requires no live swarm.
 *
 * The iroh side IS a live `IrohNode` so gossip topics can actually traverse QUIC.
 *
 * ## Blocked dependencies
 *
 * `peer_transport_manifest_for` is gated on Plan 1 (`PeerTransportManifest`).
 * The type exists (src/p2p_iroh/peer_map.rs) but the in-memory construction
 * path from a test profile has been wired via the existing test constructors
 * (`PeerTransportManifest::iroh_capable_for_test()`, `::libp2p_only_for_test()`).
 *
 * `publish_recovery_invitation_dual` and `publish_recovery_revocation_dual` do
 * not yet exist as standalone methods — callers use
 * `P2PHandle::publish_recovery_invitation` / `publish_recovery_revocation` which
 * internally route through `DualGossipPublisher`. These methods are the Plan 4
 * dependency; the Rust integration test (Task 7) drives the handle directly
 * via the `DualGossipPublisher` mock pattern from `iroh_gossip_cross_stack_e2e.rs`.
 *
 * Spec:  genesis/docs/superpowers/specs/2026-05-08-iroh-libp2p-complementarity.md (line 514)
 * Plan:  genesis/docs/superpowers/plans/2026-05-10-iroh-recovery-e2e.md
 */

use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use iroh::{NodeAddr, NodeId};

use crate::p2p::recovery_invitation::RECOVERY_INVITATION_TOPIC;
use crate::p2p::{P2PHandle, RECOVERY_REVOCATION_TOPIC};
use crate::p2p_iroh::{
    parity_harness::loopback_config,
    peer_map::{PeerTransportManifest, Plane},
    IrohNode,
};
use crate::services::gossip_flood::{GossipPublisher, PublishError};

// ────────────────────────────────────────────────────────────────────────────
// Transport profile
// ────────────────────────────────────────────────────────────────────────────

/// Per-node transport capability for the 5-node cross-stack test mesh.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportProfile {
    /// Node speaks iroh gossip only. `libp2p_handle` is a no-op stub.
    IrohOnly,
    /// Node speaks libp2p gossipsub only. No live iroh node is started.
    Libp2pOnly,
    /// Node speaks both. Recovery invitation arrives on whichever transport it
    /// was sent on; the dual publisher fans out to both simultaneously.
    Dual,
}

// ────────────────────────────────────────────────────────────────────────────
// Capture mock (libp2p stub for tests)
// ────────────────────────────────────────────────────────────────────────────

/// Captured gossip call: (topic_name, payload_bytes).
type CapturedCall = (String, Vec<u8>);

/// Thread-safe capture of (topic, payload) calls. Simulates a libp2p
/// gossipsub subscriber sink — structurally identical to what
/// `DualGossipPublisher` treats as its `libp2p_pub` half.
#[derive(Clone, Default)]
pub struct CaptureMock {
    calls: Arc<Mutex<Vec<CapturedCall>>>,
}

impl CaptureMock {
    pub fn new() -> Self {
        Self::default()
    }

    /// Return all captured calls (topic, payload).
    pub fn calls(&self) -> Vec<(String, Vec<u8>)> {
        self.calls.lock().unwrap().clone()
    }

    /// Drain all captured calls and clear the buffer.
    pub fn drain(&self) -> Vec<(String, Vec<u8>)> {
        std::mem::take(&mut self.calls.lock().unwrap())
    }

    /// Count captures for a specific topic.
    pub fn count_for_topic(&self, topic: &str) -> usize {
        self.calls().iter().filter(|(t, _)| t == topic).count()
    }
}

impl GossipPublisher for CaptureMock {
    fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
        self.calls
            .lock()
            .unwrap()
            .push((topic.to_string(), payload));
        Ok(())
    }
}

// ────────────────────────────────────────────────────────────────────────────
// NodeSlot
// ────────────────────────────────────────────────────────────────────────────

/// A single peer in the 5-node recovery cross-stack mesh.
pub struct NodeSlot {
    /// Friendly name (e.g., "Ben", "Cara") — matches the scenario personas.
    pub name: String,
    /// This node's transport capability.
    pub profile: TransportProfile,
    /// Live iroh node — `Some` for `IrohOnly` and `Dual`; `None` for `Libp2pOnly`.
    pub iroh: Option<IrohNode>,
    /// libp2p stub capture — `Some` for `Libp2pOnly` and `Dual`; `None` for `IrohOnly`.
    ///
    /// In production this would be a real libp2p swarm connection. In this fixture
    /// it is a `CaptureMock` that records gossip publish calls for assertion.
    pub libp2p_capture: Option<CaptureMock>,
    /// Stub `P2PHandle` (fire-and-forget command sink). Used so code paths that
    /// call `P2PHandle::publish_recovery_invitation` / `publish_recovery_revocation`
    /// don't need a live swarm — they drain into `command_tx` which the stub task
    /// discards (same pattern as `P2PHandle::for_testing()`).
    pub p2p_handle: P2PHandle,
}

impl NodeSlot {
    /// `true` when the node is reachable on the iroh transport.
    pub fn has_iroh(&self) -> bool {
        self.iroh.is_some()
    }

    /// `true` when the node is reachable on the libp2p transport.
    pub fn has_libp2p(&self) -> bool {
        self.libp2p_capture.is_some()
    }

    /// Count gossip receipts on the libp2p capture for the given topic.
    pub fn libp2p_receipts_for(&self, topic: &str) -> usize {
        self.libp2p_capture
            .as_ref()
            .map(|c| c.count_for_topic(topic))
            .unwrap_or(0)
    }
}

// ────────────────────────────────────────────────────────────────────────────
// MultiStackFixture
// ────────────────────────────────────────────────────────────────────────────

/// 5-node (extendable) cross-stack fixture.
///
/// Spins up each slot per its [`TransportProfile`] and cross-bootstraps the
/// iroh nodes so they can find each other via loopback (no relay required).
///
/// ## Usage (in `#[tokio::test]`)
///
/// ```rust,no_run
/// # #[cfg(feature = "p2p-iroh")]
/// # async fn example() -> anyhow::Result<()> {
/// use tempfile::tempdir;
/// use elohim_storage::p2p_iroh::multi_stack_fixture::{MultiStackFixture, TransportProfile};
///
/// let dirs: Vec<_> = (0..5).map(|_| tempdir().unwrap()).collect();
/// let profiles = vec![
///     ("Ben".into(),  TransportProfile::IrohOnly,   dirs[0].path()),
///     ("Cara".into(), TransportProfile::IrohOnly,   dirs[1].path()),
///     ("Dan".into(),  TransportProfile::Libp2pOnly,  dirs[2].path()),
///     ("Evan".into(), TransportProfile::Libp2pOnly,  dirs[3].path()),
///     ("Faye".into(), TransportProfile::Dual,        dirs[4].path()),
/// ];
/// let fixture = MultiStackFixture::new(
///     &profiles.iter().map(|(n, p, d)| (n.as_str(), *p, *d)).collect::<Vec<_>>(),
///     Vec::new,
/// ).await?;
/// # fixture.shutdown().await?;
/// # Ok(())
/// # }
/// ```
pub struct MultiStackFixture {
    pub nodes: Vec<NodeSlot>,
    /// Pre-resolved `NodeAddr` for every slot that has an iroh side.
    /// Index corresponds to the same slot position in `nodes`.
    pub iroh_addrs: Vec<Option<NodeAddr>>,
}

impl MultiStackFixture {
    /// Construct the fixture.
    ///
    /// `slots` — slice of `(name, profile, data_dir)`. Each entry becomes one [`NodeSlot`].
    ///
    /// `protocol_factory` — called once per iroh node to register custom-ALPN handlers
    /// (same pattern as `TwoNodeFixture::new`). For recovery e2e tests this is typically
    /// `Vec::new` — no extra custom ALPNs needed beyond blob + gossip (already registered
    /// by `IrohNode::start_with_protocols`).
    pub async fn new(
        slots: &[(&str, TransportProfile, &Path)],
        protocol_factory: impl Fn() -> Vec<crate::p2p_iroh::node::AlpnRegistration>,
    ) -> Result<Self> {
        let mut nodes = Vec::with_capacity(slots.len());
        let mut iroh_addrs: Vec<Option<NodeAddr>> = Vec::with_capacity(slots.len());

        // Pass 1: start each node (iroh nodes start in parallel; libp2p stubs are instant).
        for (name, profile, dir) in slots {
            let (iroh_node, addr) = match profile {
                TransportProfile::IrohOnly | TransportProfile::Dual => {
                    let node =
                        IrohNode::start_with_protocols(loopback_config(dir), (protocol_factory)())
                            .await?;
                    let addr = node.node_addr().await?;
                    (Some(node), Some(addr))
                }
                TransportProfile::Libp2pOnly => (None, None),
            };

            let libp2p_capture = match profile {
                TransportProfile::Libp2pOnly | TransportProfile::Dual => Some(CaptureMock::new()),
                TransportProfile::IrohOnly => None,
            };

            let p2p_handle = P2PHandle::for_testing();

            iroh_addrs.push(addr);
            nodes.push(NodeSlot {
                name: name.to_string(),
                profile: *profile,
                iroh: iroh_node,
                libp2p_capture,
                p2p_handle,
            });
        }

        // Pass 2: cross-bootstrap iroh nodes.
        //
        // Each iroh-capable node needs to:
        //   (a) know the NodeAddr (IP + port) of every other iroh node so QUIC
        //       dials succeed (relays are disabled in loopback mode).
        //   (b) subscribe to recovery topics with the other nodes as bootstrap
        //       peers so iroh-gossip builds the overlay.
        //
        // Pattern mirrors `iroh_gossip_parity.rs:59-71`:
        //   endpoint.add_node_addr(peer_addr)?;
        //   gossip.subscribe(topic, vec![peer_node_id]).await?;

        // Collect (NodeId, NodeAddr) for every iroh-capable node.
        let iroh_id_addrs: Vec<(NodeId, NodeAddr)> = nodes
            .iter()
            .zip(iroh_addrs.iter())
            .filter_map(|(slot, addr_opt)| {
                addr_opt
                    .as_ref()
                    .zip(slot.iroh.as_ref().map(|n| n.node_id()))
                    .map(|(addr, id)| (id, addr.clone()))
            })
            .collect();

        for slot in &nodes {
            if let Some(iroh_node) = &slot.iroh {
                let self_id = iroh_node.node_id();

                // Register every other iroh peer's address in this node's endpoint.
                let mut other_ids = Vec::new();
                for (peer_id, peer_addr) in &iroh_id_addrs {
                    if *peer_id != self_id {
                        iroh_node.endpoint().add_node_addr(peer_addr.clone())?;
                        other_ids.push(*peer_id);
                    }
                }

                // Subscribe to recovery topics so this node receives gossip.
                // Both topics are permanent dual-publish per spec gate #4 (line 513).
                let _ = iroh_node
                    .gossip()
                    .subscribe(RECOVERY_INVITATION_TOPIC, other_ids.clone())
                    .await?;
                let _ = iroh_node
                    .gossip()
                    .subscribe(RECOVERY_REVOCATION_TOPIC, other_ids)
                    .await?;
            }
        }

        Ok(Self { nodes, iroh_addrs })
    }

    /// Look up a slot by name (case-insensitive). Panics if not found — tests
    /// should only reference slots that were declared at construction time.
    pub fn slot(&self, name: &str) -> &NodeSlot {
        self.nodes
            .iter()
            .find(|s| s.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("MultiStackFixture: no slot named '{name}'"))
    }

    /// Return the `PeerTransportManifest` for a named slot, derived from its
    /// `TransportProfile`. Used by the Rust integration test (Task 7) to resolve
    /// "which transport does share-holder X support?" without duplicating the
    /// test data.
    ///
    /// # TODO(plan-1)
    /// The in-memory construction path reuses the existing test constructors on
    /// `PeerTransportManifest`. If Plan 1 adds richer constructor APIs, update
    /// this method to use them. The `agent_cid` is set to `"test-{name}"` — in
    /// production this would be the real CID from the peer's `AgentPeerBinding`.
    pub fn peer_transport_manifest_for(&self, name: &str) -> PeerTransportManifest {
        let slot = self.slot(name);
        match slot.profile {
            TransportProfile::IrohOnly => {
                let node_id = slot
                    .iroh
                    .as_ref()
                    .map(|n| n.node_id().to_string())
                    .unwrap_or_else(|| format!("test-iroh-{name}"));
                PeerTransportManifest {
                    agent_cid: format!("test-{}", slot.name.to_lowercase()),
                    libp2p: None,
                    iroh: Some(crate::p2p_iroh::peer_map::IrohTransportProfile {
                        node_id,
                        relays: vec![],
                        supports: vec![
                            Plane::Gossip.as_str().to_string(),
                            Plane::Blob.as_str().to_string(),
                        ],
                    }),
                    discovery: vec![],
                    capability_level: 5,
                    last_observed: 1746878400,
                }
            }
            TransportProfile::Libp2pOnly => {
                let peer_id = format!("12D3KooWTest{}", slot.name);
                PeerTransportManifest {
                    agent_cid: format!("test-{}", slot.name.to_lowercase()),
                    libp2p: Some(crate::p2p_iroh::peer_map::Libp2pTransportProfile {
                        peer_id,
                        addrs: vec![],
                        supports: vec![
                            Plane::Gossip.as_str().to_string(),
                            Plane::Blob.as_str().to_string(),
                        ],
                    }),
                    iroh: None,
                    discovery: vec![],
                    capability_level: 5,
                    last_observed: 1746878400,
                }
            }
            TransportProfile::Dual => {
                // TODO(plan-1): PeerTransportManifest::iroh_capable_for_test() sets a
                // generic node_id. Replace with the actual live iroh node_id once Plan 1
                // lands richer constructor support.
                let node_id = slot
                    .iroh
                    .as_ref()
                    .map(|n| n.node_id().to_string())
                    .unwrap_or_else(|| format!("test-iroh-dual-{name}"));
                let peer_id = format!("12D3KooWDual{}", slot.name);
                PeerTransportManifest {
                    agent_cid: format!("test-{}", slot.name.to_lowercase()),
                    libp2p: Some(crate::p2p_iroh::peer_map::Libp2pTransportProfile {
                        peer_id,
                        addrs: vec![],
                        supports: vec![
                            Plane::Gossip.as_str().to_string(),
                            Plane::Blob.as_str().to_string(),
                        ],
                    }),
                    iroh: Some(crate::p2p_iroh::peer_map::IrohTransportProfile {
                        node_id,
                        relays: vec![],
                        supports: vec![
                            Plane::Gossip.as_str().to_string(),
                            Plane::Blob.as_str().to_string(),
                        ],
                    }),
                    discovery: vec![],
                    capability_level: 5,
                    last_observed: 1746878400,
                }
            }
        }
    }

    /// Shut down all iroh nodes cleanly. Call this at the end of each test.
    pub async fn shutdown(self) -> Result<()> {
        for slot in self.nodes {
            if let Some(iroh) = slot.iroh {
                iroh.shutdown().await?;
            }
        }
        Ok(())
    }
}
