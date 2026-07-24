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
//! `inventory_fetch` is always `None` here: the commitment-driven active blob
//! fetch is a libp2p-command-path operation (see `gossip_dispatch` module docs).

use std::sync::Arc;

use futures::StreamExt;
use tracing::{debug, info, warn};

use crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo;
use crate::p2p::dedup::DedupLru;
use crate::p2p::gossip_dispatch::{handle_gossip, GossipDispatchCtx, GossipSource};
use crate::p2p_iroh::gossip::{GossipEvent, IrohGossip};

/// The consumer-grade inbound gossip topics subscribed over the iroh plane.
/// Must include the "must-receive" set: inventory, identity-binding, and
/// recovery/revocation.
const INBOUND_TOPICS: &[&str] = &[
    crate::p2p::inventory_gossip::INVENTORY_TOPIC,
    crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC,
    crate::p2p::RECOVERY_INVITATION_TOPIC,
    crate::p2p::RECOVERY_REVOCATION_TOPIC,
    crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION,
    crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC,
    crate::p2p::salvage_gossip::SALVAGE_CAPACITY_TOPIC,
    crate::p2p::custody_announce::CUSTODY_ANNOUNCE_TOPIC,
];

/// Spawn one receive task per inbound topic. Each task subscribes to its topic
/// (keeping the receiver), then dispatches received messages through the
/// transport-neutral handler until the gossip subsystem shuts down.
///
/// Non-blocking: returns immediately after spawning the tasks.
pub fn spawn_iroh_gossip_receive(
    gossip: IrohGossip,
    db_pool: Option<crate::db::DbPool>,
    dedup: Arc<DedupLru>,
    agent_info_tx: Option<tokio::sync::mpsc::Sender<ConductorAgentInfo>>,
) {
    info!(
        topics = INBOUND_TOPICS.len(),
        "iroh gossip receive: subscribing to inbound topics (Dual-plane receive lit)"
    );
    for &topic in INBOUND_TOPICS {
        let gossip = gossip.clone();
        let db_pool = db_pool.clone();
        let dedup = dedup.clone();
        let agent_info_tx = agent_info_tx.clone();
        tokio::spawn(async move {
            run_topic_receive(gossip, topic, db_pool, dedup, agent_info_tx).await;
        });
    }
}

/// Subscribe to one topic and dispatch received messages until the stream ends.
async fn run_topic_receive(
    gossip: IrohGossip,
    topic: &'static str,
    db_pool: Option<crate::db::DbPool>,
    dedup: Arc<DedupLru>,
    agent_info_tx: Option<tokio::sync::mpsc::Sender<ConductorAgentInfo>>,
) {
    // Empty bootstrap: we are a subscriber; peers join via the gossip
    // membership layer as they discover the topic.
    let mut receiver = match gossip.subscribe(topic, vec![]).await {
        Ok((_sender, receiver)) => {
            debug!(topic = %topic, "iroh gossip receive: subscribed");
            receiver
        }
        Err(e) => {
            warn!(topic = %topic, error = ?e, "iroh gossip receive: subscribe failed — topic deaf");
            return;
        }
    };

    while let Some(item) = receiver.next().await {
        match item {
            Ok(GossipEvent::Received(msg)) => {
                let source = GossipSource::Iroh {
                    node: msg.delivered_from.to_string(),
                };
                let ctx = GossipDispatchCtx {
                    db_pool: db_pool.as_ref(),
                    dedup: &dedup,
                    agent_info_inbound_tx: agent_info_tx.as_ref(),
                    // Always None on the iroh plane — active fetch is
                    // libp2p-command-path (documented Dual-mode asymmetry).
                    inventory_fetch: None,
                };
                handle_gossip(&ctx, topic, &msg.content, &source);
            }
            Ok(GossipEvent::NeighborUp(node)) => {
                debug!(topic = %topic, node = %node, "iroh gossip receive: neighbor up");
            }
            Ok(GossipEvent::NeighborDown(node)) => {
                debug!(topic = %topic, node = %node, "iroh gossip receive: neighbor down");
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
}
