//! Iroh-gossip receive loop — the receive-side counterpart to
//! [`crate::p2p_iroh::dual_publish::IrohGossipPublisher`] (which only sends).
//!
//! The publisher subscribes to a topic on first *publish* and drops the
//! [`iroh_gossip::api::GossipReceiver`]. That leaves the iroh plane deaf: a
//! peer's inventory / identity-binding / revocation gossip arriving over iroh
//! was silently discarded. This module fixes that by proactively subscribing
//! to the consumer-grade inbound topics at startup, KEEPING each
//! `GossipReceiver`, and dispatching every received message through the same
//! transport-neutral [`crate::p2p::gossip_dispatch::handle_gossip`] the libp2p
//! plane uses.
//!
//! ## Dual-mode dedup
//!
//! In `TransportBackend::Dual` mode the caller passes the libp2p node's
//! `Arc<DedupLru>`, so a revocation/announce arriving on both planes is
//! processed once. In pure-`Iroh` mode a fresh cache is used.
//!
//! ## Scope
//!
//! Only the fixed inbound topic set below is subscribed. Reach-scoped EPR
//! announce topics (`elohim/<pillar>/<reach>...`) are dynamic and NOT
//! subscribed over iroh (they are dedup+log-only on the libp2p plane and
//! Phase-3-deferred for projection) — a documented Dual-mode asymmetry.
//! Inventory work crosses into the same bounded command queue as libp2p via
//! [`crate::p2p_iroh::inventory_fetch::IrohInventoryFetch`]. The receive task
//! never dials blob bytes itself.

use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo;
use crate::p2p::dedup::DedupLru;
use crate::p2p::gossip_dispatch::{handle_gossip, GossipDispatchCtx, GossipSource};
use crate::p2p_iroh::gossip::{GossipEvent, IrohGossip};
use crate::p2p_iroh::inventory_fetch::IrohInventoryFetch;
use crate::p2p_iroh::peer_book::IrohPeerBook;
use iroh::{Endpoint, NodeId};
use std::collections::HashSet;

/// The consumer-grade inbound gossip topics subscribed over the iroh plane.
/// Must include the "must-receive" set: inventory, identity-binding, and
/// recovery/revocation.
const INBOUND_TOPICS: &[&str] = &[
    crate::p2p::inventory_gossip::INVENTORY_TOPIC,
    // Read only to report its publishers unavailable; bodies are never decoded.
    crate::p2p::inventory_gossip::LEGACY_INVENTORY_TOPIC,
    crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC,
    crate::p2p::RECOVERY_INVITATION_TOPIC,
    crate::p2p::RECOVERY_REVOCATION_TOPIC,
    crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION,
    crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC,
    crate::p2p::salvage_gossip::SALVAGE_CAPACITY_TOPIC,
    crate::p2p::custody_announce::CUSTODY_ANNOUNCE_TOPIC,
    crate::p2p::transport_manifest_gossip::TRANSPORT_MANIFEST_TOPIC,
];

/// Everything the per-topic receive loop needs besides the topic itself.
#[derive(Clone)]
pub struct IrohReceiveDeps {
    pub gossip: IrohGossip,
    pub endpoint: Endpoint,
    /// The dialable-peer book. Each topic loop watches it and joins every
    /// new peer on its topic — this is what turns "subscribed with empty
    /// bootstrap" into an actual gossip mesh.
    pub book: IrohPeerBook,
    pub db_pool: Option<crate::db::DbPool>,
    pub dedup: Arc<DedupLru>,
    pub agent_info_tx: Option<tokio::sync::mpsc::Sender<ConductorAgentInfo>>,
    pub inventory_fetch: IrohInventoryFetch,
}

/// Spawn one receive task per inbound topic. Each task subscribes to its topic
/// (keeping BOTH halves — the sender is what `join_peers` hangs off), then
/// dispatches received messages through the transport-neutral handler and
/// joins every peer the book learns, until the gossip subsystem shuts down.
///
/// Non-blocking: returns immediately after spawning the tasks.
pub fn spawn_iroh_gossip_receive(deps: IrohReceiveDeps) {
    // T2: publish the blob plane's iroh leg. This is the single production
    // site holding BOTH the endpoint and the peer book, and it runs exactly
    // once whenever an iroh node exists — so `iroh_fetch_leg()` is `Some`
    // in `dual`/`iroh` mode and `None` (no iroh leg ever constructed) in
    // `libp2p` mode. Blob heal-on-read reads it from
    // `p2p::blob_swarm::race_fetch_dual`.
    super::register_iroh_fetch_leg(deps.endpoint.clone(), deps.book.clone());
    info!(
        topics = INBOUND_TOPICS.len(),
        "iroh gossip receive: subscribing to inbound topics (Dual-plane receive lit)"
    );
    for &topic in INBOUND_TOPICS {
        let deps = deps.clone();
        tokio::spawn(async move {
            run_topic_receive(deps, topic).await;
        });
    }
}

/// Add every book peer we have not yet joined on this topic to the gossip
/// membership. Returns how many were newly joined.
async fn join_new_peers(
    deps: &IrohReceiveDeps,
    topic: &'static str,
    sender: &iroh_gossip::api::GossipSender,
    joined: &mut HashSet<NodeId>,
) -> usize {
    let me = deps.endpoint.node_id();
    let mut fresh: Vec<NodeId> = Vec::new();
    for entry in deps.book.snapshot(Some(&me)) {
        let id = entry.addr.node_id;
        if joined.contains(&id) {
            continue;
        }
        // Teach the endpoint the dial target first; join_peers only carries
        // NodeIds and would otherwise have nothing to connect to.
        if let Err(e) = deps.endpoint.add_node_addr(entry.addr.clone()) {
            debug!(topic = %topic, node = %id, error = ?e, "iroh gossip: add_node_addr refused");
        }
        fresh.push(id);
    }
    if fresh.is_empty() {
        return 0;
    }
    match sender.join_peers(fresh.clone()).await {
        Ok(()) => {
            for id in &fresh {
                joined.insert(*id);
            }
            info!(topic = %topic, peers = fresh.len(), "iroh gossip: joining peers from the book");
            fresh.len()
        }
        Err(e) => {
            warn!(topic = %topic, error = ?e, "iroh gossip: join_peers failed — will retry on next book change");
            0
        }
    }
}

/// Subscribe to one topic, dispatch received messages, and keep the
/// membership fed from the book, until the stream ends.
async fn run_topic_receive(deps: IrohReceiveDeps, topic: &'static str) {
    // Empty bootstrap at subscribe time: the book is (usually) empty at boot.
    // Membership is fed below as manifests arrive.
    let (sender, mut receiver) = match deps.gossip.subscribe(topic, vec![]).await {
        Ok(pair) => {
            debug!(topic = %topic, "iroh gossip receive: subscribed");
            pair
        }
        Err(e) => {
            warn!(topic = %topic, error = ?e, "iroh gossip receive: subscribe failed — topic deaf");
            return;
        }
    };
    let self_node_id = deps.endpoint.node_id().to_string();
    let mut joined: HashSet<NodeId> = HashSet::new();
    let mut book_rx = deps.book.subscribe();
    // Peers learned before this task subscribed.
    join_new_peers(&deps, topic, &sender, &mut joined).await;

    loop {
        tokio::select! {
            changed = book_rx.changed() => {
                if changed.is_err() {
                    // Book dropped — node is shutting down.
                    break;
                }
                join_new_peers(&deps, topic, &sender, &mut joined).await;
            }
            item = receiver.next() => {
                let Some(item) = item else { break };
                match item {
                    Ok(GossipEvent::Received(msg)) => {
                        crate::metrics::inc_iroh_gossip_received(topic);
                        let source = GossipSource::Iroh {
                            node: msg.delivered_from.to_string(),
                        };
                        let ctx = GossipDispatchCtx {
                            db_pool: deps.db_pool.as_ref(),
                            dedup: &deps.dedup,
                            agent_info_inbound_tx: deps.agent_info_tx.as_ref(),
                            inventory_fetch: Some(&deps.inventory_fetch),
                            transport_manifest_sink: Some(&deps.book),
                            self_iroh_node_id: Some(self_node_id.as_str()),
                        };
                        handle_gossip(&ctx, topic, &msg.content, &source);
                    }
                    Ok(GossipEvent::NeighborUp(node)) => {
                        crate::metrics::inc_iroh_gossip_neighbor("up");
                        info!(topic = %topic, node = %node, "iroh gossip: neighbor up");
                    }
                    Ok(GossipEvent::NeighborDown(node)) => {
                        crate::metrics::inc_iroh_gossip_neighbor("down");
                        info!(topic = %topic, node = %node, "iroh gossip: neighbor down");
                        // Let the next book change re-join it.
                        joined.remove(&node);
                    }
                    Ok(GossipEvent::Lagged) => {
                        warn!(topic = %topic, "iroh gossip receive: lagged — some messages missed");
                    }
                    Err(e) => {
                        warn!(topic = %topic, error = ?e, "iroh gossip receive: stream error — stopping topic loop");
                        break;
                    }
                }
            }
        }
    }
    debug!(topic = %topic, "iroh gossip receive: topic loop ended");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inbound_topics_cover_must_receive_set() {
        // The task's consumer-grade-must-receive set: inventory,
        // identity-binding, recovery/revocation. Guard against accidental
        // removal.
        assert!(INBOUND_TOPICS.contains(&crate::p2p::inventory_gossip::INVENTORY_TOPIC));
        assert!(INBOUND_TOPICS.contains(&crate::p2p::inventory_gossip::LEGACY_INVENTORY_TOPIC));
        assert!(
            INBOUND_TOPICS.contains(&crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC)
        );
        assert!(INBOUND_TOPICS.contains(&crate::p2p::RECOVERY_INVITATION_TOPIC));
        assert!(INBOUND_TOPICS.contains(&crate::p2p::RECOVERY_REVOCATION_TOPIC));
        assert!(INBOUND_TOPICS.contains(&crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION));
    }

    #[test]
    fn inbound_topics_are_unique() {
        let mut seen = std::collections::HashSet::new();
        for t in INBOUND_TOPICS {
            assert!(seen.insert(*t), "duplicate inbound topic: {t}");
        }
    }

    /// The iroh plane has no transport-level origin, so admission rests on the envelope
    /// alone: an inventory page signed by one node's key but naming another node's id
    /// as its holder projects nothing, whichever field the forger falsifies, and the
    /// holder's own page over the same plane lands as an end-to-end row.
    #[test]
    fn forged_iroh_inventory_holder_is_refused() {
        use crate::p2p::inventory_gossip::{
            BlobAddress, InventoryDelta, InventoryEnvelope, InventoryLevel, INVENTORY_TOPIC,
        };

        let pool = crate::test_util::test_pool();
        let victim = elohim_epr::AgentKeypair::from_secret(&[51u8; 32]).unwrap();
        let forger = elohim_epr::AgentKeypair::from_secret(&[52u8; 32]).unwrap();
        let victim_node = hex::encode(victim.public_key_bytes());
        let forger_node = hex::encode(forger.public_key_bytes());
        {
            // Each node's agent cross-signs its libp2p PeerId, and that PeerId's key and
            // the node's iroh key link each other — the only chain that binds an iroh
            // author (`p2p::transport_authority`).
            let mut conn = pool.get().unwrap();
            for (agent, iroh_secret) in [
                ("uhCAkIrohVictim", [51u8; 32]),
                ("uhCAkIrohForger", [52u8; 32]),
            ] {
                let libp2p_secret = iroh_secret.map(|b| b ^ 0x5a);
                crate::test_util::bind_cross_signed(&mut conn, agent, libp2p_secret);
                crate::test_util::link_transports(&mut conn, libp2p_secret, iroh_secret);
            }
        }
        let cid = elohim_epr::address::compute_bytes_cid(b"a payload the victim never held");
        let page = |sequence| InventoryDelta {
            added: vec![BlobAddress::from(cid)],
            hints: vec![],
            removed: vec![],
            sequence,
            emitted_at: 1_700_000_000_000_000,
        };
        let hosts = || {
            let mut conn = pool.get().unwrap();
            crate::db::peer_blob_inventory::lookup_hosts(
                &mut conn,
                &cid.to_string(),
                "2000-01-01T00:00:00Z",
            )
            .unwrap()
        };
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = GossipDispatchCtx {
            db_pool: Some(&pool),
            dedup: &dedup,
            agent_info_inbound_tx: None,
            inventory_fetch: None,
            transport_manifest_sink: None,
            self_iroh_node_id: None,
        };
        let delivered_by = |node: &str| GossipSource::Iroh {
            node: node.to_string(),
        };

        // The forger signs with its own key and names the victim as the author…
        let as_victim_author =
            InventoryEnvelope::sign(&forger, &victim_node, InventoryLevel::Full, page(1));
        // …or names the victim's key as the holder while signing with its own.
        let mut as_victim_holder =
            InventoryEnvelope::sign(&forger, &forger_node, InventoryLevel::Full, page(2));
        as_victim_holder.holder =
            elohim_epr::signature::Multikey::ed25519(&victim.public_key_bytes());
        for forged in [as_victim_author, as_victim_holder] {
            // Whoever delivered it, the delivering node is not evidence.
            for relay in [&victim_node, &forger_node] {
                handle_gossip(
                    &ctx,
                    INVENTORY_TOPIC,
                    &forged.to_bytes(),
                    &delivered_by(relay),
                );
            }
        }
        assert!(
            hosts().is_empty(),
            "a page the victim did not sign projects no holder"
        );

        // The victim's own page, relayed by anyone, is authenticated end to end.
        let genuine = InventoryEnvelope::sign(&victim, &victim_node, InventoryLevel::Full, page(3));
        handle_gossip(
            &ctx,
            INVENTORY_TOPIC,
            &genuine.to_bytes(),
            &delivered_by(&forger_node),
        );
        let rows = hosts();
        assert_eq!(rows.len(), 1, "exactly the victim's own row, got {rows:?}");
        assert_eq!(rows[0].peer_id, victim_node);
    }
}
