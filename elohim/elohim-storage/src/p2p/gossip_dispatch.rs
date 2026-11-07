//! Transport-neutral gossip receive dispatch.
//!
//! The per-topic gossip receive logic (decode → structural-verify → dedup →
//! DB projection) is transport-portable: it operates only on the topic name,
//! the raw MessagePack payload bytes, and a source label for logging. This
//! module hosts that logic so BOTH transports call one handler:
//!
//! - the **libp2p** gossipsub arm (`P2PNode::handle_behaviour_event`, the
//!   `GossipsubEvent::Message` case) builds a [`GossipDispatchCtx`] borrowing
//!   the node's `db_pool` / `dedup` / `agent_info_inbound_tx` and passes the
//!   node itself as the [`InventoryFetch`] hook, then calls [`handle_gossip`];
//! - the **iroh** receive loop (`crate::p2p_iroh::gossip_receive`) subscribes
//!   to the inbound topics, keeps the `GossipReceiver`, and calls the same
//!   [`handle_gossip`] with an inventory hook that queues into the libp2p
//!   event loop in dual mode.
//!
//! ## Dedup is cross-plane
//!
//! In `TransportBackend::Dual` mode the two transports share ONE
//! [`crate::p2p::dedup::DedupLru`] (the iroh receive loop is handed the libp2p
//! node's `Arc<DedupLru>`), so a revocation/announce arriving on both planes is
//! processed once.
//!
//! ## Inventory active-fetch
//!
//! Inventory DB projection (the "did this peer's inventory land" consumer
//! signal — `peer_blob_inventory`) is transport-neutral and runs on both
//! planes. The commitment-driven active blob-FETCH and the delta-gap
//! snapshot-request are libp2p-command-path operations, exposed here via the
//! optional [`InventoryFetch`] hook. Both receive planes enqueue the same
//! [`crate::p2p::P2PCommand::InventoryAdvertisement`] work in dual mode; the
//! event loop owns scoring and the existing libp2p byte-fetch race. Pure iroh
//! mode has no byte-fetch consumer yet, so its bridge records and logs that
//! explicit absence. Byte replication over iroh remains Lane T2 — see
//! `project_inventory_exchange_not_byte_replication`.

use tracing::{debug, info, warn};

use crate::p2p::inventory_gossip::BlobHint;

/// Where a gossip message arrived from (logging + which command path is live).
#[derive(Debug, Clone)]
pub enum GossipSource {
    /// Received over the libp2p gossipsub plane.
    Libp2p {
        /// `propagation_source` PeerId (base58): the neighbour that relayed it.
        /// Logging only — never an authority.
        peer: String,
        /// gossipsub `message_id` — for logging only.
        message_id: String,
        /// gossipsub `message.source`: the author, authenticated by gossipsub's
        /// `Signed` validation. The only libp2p author witness consumers bind to.
        author: Option<String>,
    },
    /// Received over the iroh-gossip plane.
    Iroh {
        /// `delivered_from` NodeId — for logging only.
        node: String,
    },
}

impl std::fmt::Display for GossipSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GossipSource::Libp2p { peer, .. } => write!(f, "{peer}"),
            GossipSource::Iroh { node } => write!(f, "iroh:{node}"),
        }
    }
}

/// Inventory operations injected as a hook so transport-neutral dispatch stays
/// free of a concrete event loop. Both planes queue the same work in dual mode.
pub trait InventoryFetch {
    /// Score advertised blobs against active `replicates-dwelling` commitments
    /// and enqueue HIGH-priority fetches (libp2p blob-fetch machinery).
    fn score_and_enqueue(
        &self,
        conn: &mut diesel::SqliteConnection,
        source_peer_id: &str,
        hints: &[BlobHint],
        cids: &[String],
    );

    /// Reconcile with a peer (`inventory_recon`) after a gap the reorder window could
    /// not close; the session re-bases that peer's cursor.
    fn request_recon(&self, peer_id: &str);
}

/// Where a VERIFIED `elohim/transport/manifest` announcement goes so the
/// iroh plane can dial the announcer. `None` on libp2p-only builds (the
/// durable `peer_transport_manifest` row is still written). Implemented by
/// `p2p_iroh::IrohPeerBook` under the `p2p-iroh` feature.
pub trait TransportManifestSink {
    /// Called only after `TransportManifestAnnouncement::verify` passed.
    /// Returns `true` if the sink's state changed (new/fresher peer).
    fn accept(
        &self,
        ann: &crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement,
    ) -> bool;
}

/// Borrowed context the transport-neutral gossip handler needs. Built per
/// dispatch call by whichever transport is delivering the message.
pub struct GossipDispatchCtx<'a> {
    /// Content DB pool (projection target). `None` disables persistence.
    pub db_pool: Option<&'a crate::db::DbPool>,
    /// Cross-plane dedup cache (shared across transports in Dual mode).
    pub dedup: &'a crate::p2p::dedup::DedupLru,
    /// Bounded sender for inbound `ConductorAgentInfo` envelopes (step-zero
    /// discovery). `None` when the feature flag is off.
    pub agent_info_inbound_tx: Option<
        &'a tokio::sync::mpsc::Sender<crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo>,
    >,
    /// Inventory hook. `Some` on both receive planes when they are active.
    pub inventory_fetch: Option<&'a dyn InventoryFetch>,
    /// Verified transport-manifest announcements land here (the iroh peer
    /// book). `None` when this build has no iroh plane.
    pub transport_manifest_sink: Option<&'a dyn TransportManifestSink>,
    /// This node's own iroh NodeId (hex), so its own announcement — which
    /// gossip echoes back — is counted as `self` and never dialed.
    pub self_iroh_node_id: Option<&'a str>,
}

/// Dispatch a single received gossip message by topic. Transport-neutral:
/// identical DB effects regardless of which plane delivered it. Behavior is
/// byte-identical to the pre-refactor libp2p gossipsub arm when
/// `ctx.inventory_fetch` is `Some(node)`.
pub fn handle_gossip(ctx: &GossipDispatchCtx<'_>, topic: &str, data: &[u8], source: &GossipSource) {
    use crate::p2p::{RECOVERY_INVITATION_TOPIC, RECOVERY_REVOCATION_TOPIC};

    if topic == RECOVERY_INVITATION_TOPIC {
        match crate::p2p::recovery_invitation::RecoveryInvitation::from_bytes(data) {
            Ok(inv) => info!(
                target: "elohim_storage::recovery",
                from = %source,
                request_hash = %inv.request_hash,
                human_id = %inv.human_id,
                "Received recovery invitation"
            ),
            Err(e) => warn!(
                target: "elohim_storage::recovery",
                from = %source,
                error = ?e,
                "Failed to decode RecoveryInvitation"
            ),
        }
    } else if topic == crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC {
        // Receive an identity binding from a peer and upsert into
        // peer_identity_bindings with source='gossip'. Structural verification
        // only (Stage 1). Non-authoritative writer: must NOT clobber a
        // `superseded_by` previously set by the DHT-arrival path.
        match crate::p2p::identity_binding_gossip::IdentityBindingGossip::from_bytes(data) {
            Ok(payload) => match payload.verify_structural() {
                Err(reason) => warn!(
                    target: "elohim_storage::identity",
                    from = %source,
                    reason = %reason,
                    "IdentityBindingGossip failed structural verify — dropped"
                ),
                Ok(()) => {
                    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                    let row = crate::p2p::identity_binding_gossip::binding_row_from_gossip(
                        &payload, &now_iso,
                    );
                    match ctx.db_pool {
                        Some(pool) => match pool.get() {
                            Ok(mut conn) => {
                                match crate::db::peer_identity_bindings::upsert_preserving_supersession(
                                    &mut conn, &row,
                                ) {
                                    Ok(()) => info!(
                                        target: "elohim_storage::identity",
                                        from = %source,
                                        peer_id = %payload.peer_id,
                                        agent_cid = %payload.agent_cid,
                                        "IdentityBindingGossip upserted with source='gossip'"
                                    ),
                                    Err(e) => warn!(
                                        target: "elohim_storage::identity",
                                        from = %source,
                                        peer_id = %payload.peer_id,
                                        error = %e,
                                        "IdentityBindingGossip db upsert failed"
                                    ),
                                }
                            }
                            Err(e) => warn!(
                                target: "elohim_storage::identity",
                                peer_id = %payload.peer_id,
                                error = %e,
                                "IdentityBindingGossip: db pool exhausted"
                            ),
                        },
                        None => debug!(
                            target: "elohim_storage::identity",
                            peer_id = %payload.peer_id,
                            "IdentityBindingGossip: no db_pool configured, skipping persistence"
                        ),
                    }
                }
            },
            Err(e) => warn!(
                target: "elohim_storage::identity",
                from = %source,
                error = ?e,
                "Failed to decode IdentityBindingGossip"
            ),
        }
    } else if topic == crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC {
        // Step-zero substrate gossip — decode, structural verify, try_send into
        // the bounded mpsc. Channel-full drops are safe: the 60s publisher
        // heartbeat re-delivers.
        match crate::p2p::conductor_agent_info_gossip::ConductorAgentInfo::from_bytes(data) {
            Ok(payload) => {
                if let Err(reason) = payload.verify_structural() {
                    debug!(
                        target: "elohim_storage::agent_info",
                        from = %source,
                        reason = %reason,
                        "ConductorAgentInfo failed structural verify — dropped"
                    );
                } else if let Some(tx) = ctx.agent_info_inbound_tx {
                    if tx.try_send(payload).is_err() {
                        debug!(
                            target: "elohim_storage::agent_info",
                            from = %source,
                            "agent_info inbound queue full — dropped (heartbeat will re-deliver)"
                        );
                    }
                }
                // else: feature flag off, no sender wired — silently ignore
            }
            Err(e) => debug!(
                target: "elohim_storage::agent_info",
                from = %source,
                error = %e,
                "ConductorAgentInfo decode failed — dropped"
            ),
        }
    } else if topic == RECOVERY_REVOCATION_TOPIC {
        // M4 subscribe/log stub. Dedup on the synthetic `KeyRevocation:{id}` key
        // so a revocation arriving on multiple channels/planes is dropped once.
        handle_revocation_message(ctx, data, source, RECOVERY_REVOCATION_TOPIC, false);
    } else if topic == crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION {
        // Canonical integrity-revocation topic. BOTH arms route to the same
        // handler with the same dedup key (cross-channel dedup contract, D.6).
        handle_revocation_message(
            ctx,
            data,
            source,
            crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION,
            true,
        );
    } else if topic == crate::p2p::inventory_gossip::INVENTORY_TOPIC
        || topic == crate::p2p::inventory_gossip::LEGACY_INVENTORY_TOPIC
    {
        handle_inventory(ctx, topic, data, source);
    } else if topic == crate::p2p::salvage_gossip::SALVAGE_CAPACITY_TOPIC {
        handle_salvage(ctx, data, source);
    } else if topic == crate::p2p::custody_announce::CUSTODY_ANNOUNCE_TOPIC {
        handle_custody_announce(ctx, data, source);
    } else if topic == crate::p2p::transport_manifest_gossip::TRANSPORT_MANIFEST_TOPIC {
        handle_transport_manifest(ctx, data, source);
    } else if topic.starts_with("elohim/") {
        // Per-pillar reach-scoped EPR announce topics carry a msgpack-encoded
        // CID string (announce-only). Stage 1: decode CID + dedup only.
        match rmp_serde::from_slice::<String>(data) {
            Ok(cid) => {
                if !ctx.dedup.insert(&cid) {
                    debug!(
                        target: "elohim_storage::dedup",
                        from = %source,
                        topic = %topic,
                        cid = %cid,
                        "duplicate gossip announce — dropped"
                    );
                } else {
                    debug!(
                        target: "elohim_storage::epr",
                        from = %source,
                        topic = %topic,
                        cid = %cid,
                        "gossip EPR announce (deduped; full fetch deferred to Phase 3+)"
                    );
                }
            }
            Err(e) => debug!(
                target: "elohim_storage::epr",
                from = %source,
                topic = %topic,
                error = %e,
                "gossip EPR announce: msgpack decode failed"
            ),
        }
    } else {
        debug!(topic = %topic, "Gossipsub message on untracked topic");
    }
}

/// Shared revocation handler (both the legacy `recovery.revocation` and the
/// canonical `elohim/integrity/revocation` topics route here). Synthetic dedup
/// key `KeyRevocation:{revocation_id}` — the namespace prefix prevents
/// cross-kind collisions.
fn handle_revocation_message(
    ctx: &GossipDispatchCtx<'_>,
    data: &[u8],
    source: &GossipSource,
    topic: &str,
    canonical: bool,
) {
    match crate::p2p::recovery_revocation::RecoveryRevocationMessage::from_bytes(data) {
        Ok(msg) => {
            let dedup_key = format!("KeyRevocation:{}", msg.revocation_id);
            if !ctx.dedup.insert(&dedup_key) {
                debug!(
                    target: "elohim_storage::dedup",
                    from = %source,
                    revocation_id = %msg.revocation_id,
                    topic = %topic,
                    "duplicate revocation gossip — dropped"
                );
            } else if canonical {
                info!(
                    target: "recovery.revocation.inbound",
                    from = %source,
                    revocation_id = %msg.revocation_id,
                    human_id = %msg.human_id,
                    status = %msg.status,
                    topic = %topic,
                    "Received recovery revocation (canonical topic)"
                );
            } else {
                info!(
                    target: "recovery.revocation.inbound",
                    from = %source,
                    revocation_id = %msg.revocation_id,
                    human_id = %msg.human_id,
                    status = %msg.status,
                    "Received recovery revocation"
                );
            }
        }
        Err(e) => warn!(
            target: "recovery.revocation.inbound",
            from = %source,
            error = ?e,
            topic = %topic,
            "Failed to decode RecoveryRevocationMessage"
        ),
    }
}

/// The holder key a receiver expects for a transport author: admitted only when
/// `peer_transport_manifest` binds the peer (libp2p PeerId or iroh NodeId column), and
/// derived from the peer id itself (both embed the node's ed25519 key).
pub struct ManifestHolderResolver<'c, 'a> {
    conn: &'c std::cell::RefCell<&'a mut diesel::SqliteConnection>,
}

impl crate::p2p::inventory_gossip::HolderResolver for ManifestHolderResolver<'_, '_> {
    fn expected_holder(&self, author_peer: &str) -> Option<elohim_epr::signature::Multikey> {
        bound_agent_for_author(&mut self.conn.borrow_mut(), author_peer)?;
        transport_key(author_peer)
    }
}

/// The `agent_cid` the transport manifest binds to `author_peer`, if any.
pub fn bound_agent_for_author(
    conn: &mut diesel::SqliteConnection,
    author_peer: &str,
) -> Option<String> {
    use crate::db::diesel_schema::peer_transport_manifest::dsl as t;
    use diesel::prelude::*;
    t::peer_transport_manifest
        .filter(
            t::libp2p_peer_id
                .eq(author_peer)
                .or(t::iroh_node_id.eq(author_peer)),
        )
        .select(t::agent_cid)
        .first::<String>(conn)
        .optional()
        .ok()
        .flatten()
}

/// The agent any binding this node holds attributes to a relaying transport id:
/// `peer_transport_manifest` (libp2p or iroh column), else an active
/// `peer_identity_bindings` row (libp2p PeerId).
pub fn relay_agent(conn: &mut diesel::SqliteConnection, transport_id: &str) -> Option<String> {
    if let Some(agent) = bound_agent_for_author(conn, transport_id) {
        return Some(agent);
    }
    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    crate::db::peer_identity_bindings::lookup_active(conn, transport_id, &now_iso)
        .ok()
        .flatten()
        .map(|row| row.agent_cid)
}

/// The ed25519 key a transport peer id embeds: a libp2p PeerId's identity multihash
/// carries the protobuf public key; an iroh NodeId is the key (64 lowercase hex).
pub fn transport_key(author_peer: &str) -> Option<elohim_epr::signature::Multikey> {
    use elohim_epr::signature::Multikey;
    if author_peer.len() == 64 && author_peer.bytes().all(|b| b.is_ascii_hexdigit()) {
        let bytes: [u8; 32] = hex::decode(author_peer).ok()?.try_into().ok()?;
        return Some(Multikey::ed25519(&bytes));
    }
    let peer: libp2p::PeerId = author_peer.parse().ok()?;
    let multihash = peer.as_ref();
    if multihash.code() != 0x00 {
        return None; // a hashed (non-identity) PeerId does not embed its key
    }
    let key = libp2p::identity::PublicKey::try_decode_protobuf(multihash.digest()).ok()?;
    let ed = key.try_into_ed25519().ok()?;
    Some(Multikey::ed25519(&ed.to_bytes()))
}

/// `peer_blob_inventory` + `held_envelopes` behind the transport-neutral sink.
struct DbInventorySink<'c, 'a> {
    conn: &'c std::cell::RefCell<&'a mut diesel::SqliteConnection>,
    /// The witness the arriving plane gives every row this delivery applies.
    witness: crate::db::peer_blob_inventory::InventoryWitness,
}

impl crate::p2p::inventory_gossip::InventorySink for DbInventorySink<'_, '_> {
    fn apply_delta(
        &mut self,
        peer: &str,
        delta: &crate::p2p::inventory_gossip::InventoryDelta,
    ) -> Result<crate::p2p::inventory_gossip::DeltaOutcome, String> {
        apply_delta_row(&mut self.conn.borrow_mut(), peer, delta, self.witness)
    }

    fn retain_held(&mut self, held: crate::p2p::inventory_gossip::HeldEnvelope) {
        let reason = match held.reason {
            crate::p2p::inventory_gossip::HeldReason::Epoch(epoch) => format!("epoch:{epoch}"),
            crate::p2p::inventory_gossip::HeldReason::Scheme(scheme) => format!("scheme:{scheme}"),
        };
        if let Err(e) = crate::db::held_envelopes::retain(
            &mut self.conn.borrow_mut(),
            held.topic,
            &reason,
            &held.bytes,
        ) {
            warn!(target: "elohim_storage::inventory", error = %e, "held envelope not retained");
        }
    }

    fn mark_unavailable(&mut self, peer: &str, reason: &'static str) {
        crate::p2p::inventory_gossip::mark_unavailable(peer, reason);
    }
}

fn apply_delta_row(
    conn: &mut diesel::SqliteConnection,
    peer: &str,
    delta: &crate::p2p::inventory_gossip::InventoryDelta,
    witness: crate::db::peer_blob_inventory::InventoryWitness,
) -> Result<crate::p2p::inventory_gossip::DeltaOutcome, String> {
    use crate::db::peer_blob_inventory::DeltaApplyOutcome as Db;
    use crate::p2p::inventory_gossip::DeltaOutcome;
    let added: Vec<String> = delta.added.iter().map(|a| a.to_string()).collect();
    let removed: Vec<String> = delta.removed.iter().map(|a| a.to_string()).collect();
    let when = crate::p2p::micros_to_iso(delta.emitted_at)
        .unwrap_or_else(|| chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string());
    match crate::db::peer_blob_inventory::apply_delta(
        conn,
        peer,
        &added,
        &removed,
        delta.sequence as i64,
        &when,
        witness,
    ) {
        Ok(Db::Applied) => Ok(DeltaOutcome::Applied),
        Ok(Db::Replay) => Ok(DeltaOutcome::Replay),
        Ok(Db::Gap { expected, .. }) => Ok(DeltaOutcome::Gap {
            expected: expected.max(0) as u64,
        }),
        Err(e) => Err(e.to_string()),
    }
}

/// Apply every held page that is now contiguous with the cursor, in order. Held pages
/// were admitted (signature-verified) before they were stashed; the stash does not
/// keep the plane each arrived on, so each is recorded under the witness every
/// admitted envelope carries on both planes, [`InventoryWitness::EndToEnd`].
///
/// [`InventoryWitness::EndToEnd`]: crate::db::peer_blob_inventory::InventoryWitness::EndToEnd
/// bounded-work: at most the per-peer window (`MAX_PENDING_PER_PEER`) applies per call.
fn flush_held_pages(
    ctx: &GossipDispatchCtx<'_>,
    conn: &mut diesel::SqliteConnection,
    peer_id: &str,
    mut next_seq: i64,
) {
    use crate::p2p::inventory_gossip::DeltaOutcome;
    use crate::p2p::inventory_reorder::with_reorder;
    let mut flushed = 0usize;
    while let Some(held) = with_reorder(|r| r.take_next(peer_id, next_seq)) {
        match apply_delta_row(
            conn,
            peer_id,
            &held.body,
            crate::db::peer_blob_inventory::InventoryWitness::EndToEnd,
        ) {
            Ok(DeltaOutcome::Applied) => {
                flushed += 1;
                crate::metrics::inc_inventory_page("delta", "flushed");
                enqueue_fetch(ctx, conn, peer_id, &held);
                next_seq = held.body.sequence as i64 + 1;
            }
            Ok(other) => {
                debug!(
                    target: "elohim_storage::inventory",
                    peer_id = %peer_id, sequence = held.body.sequence, outcome = ?other,
                    "held inventory page no longer contiguous — left for the next apply"
                );
                break;
            }
            Err(e) => {
                crate::metrics::inc_inventory_page("delta", "failed");
                warn!(target: "elohim_storage::inventory", error = %e, "apply_delta (held page) failed");
                break;
            }
        }
    }
    if flushed > 0 {
        debug!(
            target: "elohim_storage::inventory",
            peer_id = %peer_id, flushed,
            "held inventory pages applied in order"
        );
    }
}

fn enqueue_fetch(
    ctx: &GossipDispatchCtx<'_>,
    conn: &mut diesel::SqliteConnection,
    peer_id: &str,
    envelope: &crate::p2p::inventory_gossip::InventoryEnvelope,
) {
    if envelope.body.added.is_empty() {
        return;
    }
    if let Some(fetch) = ctx.inventory_fetch {
        let added: Vec<String> = envelope.body.added.iter().map(|a| a.to_string()).collect();
        fetch.score_and_enqueue(conn, peer_id, &envelope.body.hints, &added);
    }
}

/// Inventory envelopes (and the legacy topic) into `peer_blob_inventory`. Admission is
/// [`crate::p2p::inventory_gossip::receive`]; this binds it to the DB, the reorder
/// window and the fetch hook.
fn handle_inventory(ctx: &GossipDispatchCtx<'_>, topic: &str, data: &[u8], source: &GossipSource) {
    use crate::p2p::inventory_gossip::{receive, AuthorWitness, Receipt};
    use crate::p2p::inventory_reorder::{with_reorder, StashOutcome};

    let witness = match source {
        GossipSource::Libp2p { author, .. } => AuthorWitness::Gossipsub(author.as_deref()),
        GossipSource::Iroh { .. } => AuthorWitness::EndToEnd,
    };
    let peer_hint = source.to_string();
    let Some(pool) = ctx.db_pool else {
        return;
    };
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            warn!(target: "elohim_storage::inventory", error = %e, "inventory: db pool exhausted");
            return;
        }
    };
    let conn_ref: &mut diesel::SqliteConnection = &mut conn;
    let cell = std::cell::RefCell::new(conn_ref);
    let resolver = ManifestHolderResolver { conn: &cell };
    // libp2p admission required the signed `message.source` to be the author; iroh
    // admission rests on the envelope signature alone.
    let row_witness = match witness {
        AuthorWitness::Gossipsub(_) => {
            crate::db::peer_blob_inventory::InventoryWitness::SignedAuthor
        }
        AuthorWitness::EndToEnd => crate::db::peer_blob_inventory::InventoryWitness::EndToEnd,
    };
    let mut sink = DbInventorySink {
        conn: &cell,
        witness: row_witness,
    };
    let receipt = receive(topic, data, witness, &peer_hint, &resolver, &mut sink);
    let conn: &mut diesel::SqliteConnection = cell.into_inner();
    match receipt {
        Receipt::Applied { peer, envelope } => {
            crate::metrics::inc_inventory_page("delta", "applied");
            debug!(
                target: "elohim_storage::inventory",
                from = %source, peer_id = %peer, sequence = envelope.body.sequence,
                added = envelope.body.added.len(), removed = envelope.body.removed.len(),
                "Inventory delta applied"
            );
            crate::p2p::inventory_gossip::clear_unavailable(&peer);
            enqueue_fetch(ctx, conn, &peer, &envelope);
            flush_held_pages(ctx, conn, &peer, envelope.body.sequence as i64 + 1);
        }
        Receipt::HintsOnly { peer, .. } => {
            crate::metrics::inc_inventory_page("delta", "hints");
            debug!(target: "elohim_storage::inventory", peer_id = %peer, "inventory hints received");
        }
        Receipt::Replay { peer } => {
            crate::metrics::inc_inventory_page("delta", "replay");
            debug!(target: "elohim_storage::inventory", peer_id = %peer, "Inventory delta replay — dropped");
        }
        Receipt::Gap {
            peer,
            expected,
            envelope,
        } => {
            let received = envelope.body.sequence;
            match with_reorder(|r| r.stash(envelope)) {
                StashOutcome::Buffered => {
                    crate::metrics::inc_inventory_page("delta", "buffered");
                    debug!(
                        target: "elohim_storage::inventory",
                        peer_id = %peer, expected, received,
                        "Inventory delta ahead of the cursor — held for in-order apply"
                    );
                }
                StashOutcome::Duplicate => crate::metrics::inc_inventory_page("delta", "duplicate"),
                StashOutcome::Overflow => {
                    crate::metrics::inc_inventory_page("delta", "overflow");
                    warn!(
                        target: "elohim_storage::inventory",
                        peer_id = %peer, expected, received,
                        "Inventory delta gap beyond the reorder window — requesting recon"
                    );
                    if let Some(fetch) = ctx.inventory_fetch {
                        fetch.request_recon(&peer);
                    }
                }
            }
        }
        Receipt::Held(reason) => {
            crate::metrics::inc_inventory_page("delta", "held");
            info!(
                target: "elohim_storage::inventory",
                from = %source, ?reason,
                "Inventory envelope held (unknown epoch or scheme) — retained, not projected"
            );
        }
        Receipt::Refused(reason) => {
            crate::metrics::inc_inventory_page("delta", reason.label());
            warn!(
                target: "elohim_storage::inventory",
                from = %source, ?reason,
                "Inventory envelope refused"
            );
        }
        Receipt::Unavailable { peer } => {
            crate::metrics::inc_inventory_page("legacy", "unavailable");
            debug!(
                target: "elohim_storage::inventory",
                peer_id = %peer,
                "peer publishes on the retired inventory topic — reported unavailable"
            );
        }
        Receipt::SinkFailed(e) => {
            crate::metrics::inc_inventory_page("delta", "failed");
            warn!(target: "elohim_storage::inventory", error = %e, "apply_delta failed");
        }
    }
}

/// Salvage-capacity ads into `salvage_capacity`. Admission is
/// [`crate::p2p::salvage_gossip::admit`] (signature, signer = author, the advertised
/// agent bound to the signer, no contradicting gossipsub source, freshness); an
/// unknown epoch or scheme is held, bounded, never projected; replay is judged by the
/// per-agent `seq` at projection. An unsigned, unbound, stale or replayed ad changes
/// nothing.
fn handle_salvage(ctx: &GossipDispatchCtx<'_>, data: &[u8], source: &GossipSource) {
    use crate::db::salvage_capacity::CapacityApply;
    use crate::p2p::inventory_gossip::AuthorWitness;
    use crate::p2p::salvage_gossip::{admit, SalvageAdmission, SALVAGE_CAPACITY_TOPIC};

    let Some(pool) = ctx.db_pool else {
        return;
    };
    let mut conn = match pool.get() {
        Ok(conn) => conn,
        Err(e) => {
            warn!(target: "elohim_storage::salvage", error = %e, "salvage: db pool exhausted");
            return;
        }
    };
    let witness = match source {
        GossipSource::Libp2p { author, .. } => AuthorWitness::Gossipsub(author.as_deref()),
        GossipSource::Iroh { .. } => AuthorWitness::EndToEnd,
    };
    let now = chrono::Utc::now();
    let verdict = {
        let conn_ref: &mut diesel::SqliteConnection = &mut conn;
        let cell = std::cell::RefCell::new(conn_ref);
        admit(
            data,
            witness,
            &mut |transport_id: &str| bound_agent_for_author(&mut cell.borrow_mut(), transport_id),
            &mut |transport_id: &str| relay_agent(&mut cell.borrow_mut(), transport_id),
            now.timestamp_millis(),
        )
    };
    let (ad, author_peer) = match verdict {
        SalvageAdmission::Admitted { body, author_peer } => (body, author_peer),
        SalvageAdmission::Held { reason, bytes } => {
            if let Err(e) = crate::db::held_envelopes::retain(
                &mut conn,
                SALVAGE_CAPACITY_TOPIC,
                &reason.as_reason(),
                &bytes,
            ) {
                warn!(target: "elohim_storage::salvage", error = %e, "held salvage ad not retained");
            }
            info!(
                target: "elohim_storage::salvage",
                from = %source, ?reason,
                "salvage-capacity ad held (unknown epoch or scheme) — retained, not projected"
            );
            return;
        }
        SalvageAdmission::Refused(reason) => {
            warn!(
                target: "elohim_storage::salvage",
                from = %source, label = reason.label(), %reason,
                "salvage-capacity ad refused"
            );
            return;
        }
    };
    // `last_seen_at` is the signed issue time (never later than this clock), so the
    // pool's freshness window ages the ad from when its author made it.
    let issued = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(ad.issued_at_ms)
        .map_or(now, |issued| issued.min(now));
    let when = issued.format("%Y-%m-%dT%H:%M:%SZ").to_string();
    match crate::db::salvage_capacity::apply_capacity(
        &mut conn,
        &ad.agent_cid,
        i64::try_from(ad.spare_bytes).unwrap_or(i64::MAX),
        &ad.archetype,
        &when,
        i64::try_from(ad.seq).unwrap_or(i64::MAX),
    ) {
        Ok(CapacityApply::Applied) => debug!(
            target: "elohim_storage::salvage",
            from = %source,
            author = %author_peer,
            agent_cid = %ad.agent_cid,
            spare_bytes = ad.spare_bytes,
            archetype = %ad.archetype,
            seq = ad.seq,
            "Salvage-capacity ad applied"
        ),
        Ok(CapacityApply::Replay) => debug!(
            target: "elohim_storage::salvage",
            from = %source,
            agent_cid = %ad.agent_cid,
            seq = ad.seq,
            "Salvage-capacity ad at or below the projected seq — replay, not re-projected"
        ),
        Err(e) => warn!(
            target: "elohim_storage::salvage",
            from = %source,
            error = %e,
            "apply_capacity failed"
        ),
    }
}

/// `elohim/transport/manifest`: verify the announcer's signature against its
/// iroh NodeId, then (a) feed the iroh peer book so the plane can dial it and
/// (b) upsert the durable `peer_transport_manifest` row. Unsigned, stale, or
/// malformed announcements are counted and dropped — never applied.
fn handle_transport_manifest(ctx: &GossipDispatchCtx<'_>, data: &[u8], source: &GossipSource) {
    use crate::p2p::transport_manifest_gossip::TransportManifestAnnouncement;

    let ann = match TransportManifestAnnouncement::from_bytes(data) {
        Ok(a) => a,
        Err(e) => {
            crate::metrics::inc_iroh_manifest_announcement("decode_failed");
            debug!(from = %source, error = %e, "transport manifest decode failed — dropped");
            return;
        }
    };
    if let Err(e) = ann.verify() {
        crate::metrics::inc_iroh_manifest_announcement("bad_signature");
        warn!(
            from = %source, node = %ann.iroh_node_id, reason = %e,
            "transport manifest failed verification — dropped"
        );
        return;
    }
    if ctx.self_iroh_node_id == Some(ann.iroh_node_id.as_str()) {
        crate::metrics::inc_iroh_manifest_announcement("self");
        return;
    }
    // Dedup on the full identity incl. timestamp (exact replay dropped, a
    // fresher re-announce rides through — same rule as custody announce).
    let dedup_key = format!(
        "TransportManifest:{}:{}",
        ann.iroh_node_id, ann.announced_at_ms
    );
    if !ctx.dedup.insert(&dedup_key) {
        crate::metrics::inc_iroh_manifest_announcement("stale");
        return;
    }

    let book_changed = ctx
        .transport_manifest_sink
        .map(|sink| sink.accept(&ann))
        .unwrap_or(false);
    crate::metrics::inc_iroh_manifest_announcement(if book_changed { "accepted" } else { "stale" });
    if book_changed {
        info!(
            from = %source, node = %ann.iroh_node_id,
            addrs = ?ann.iroh_direct_addrs, relay = ?ann.iroh_relay_url,
            agent = ?ann.agent_cid, libp2p = ?ann.libp2p_peer_id,
            user_agent = ?ann.user_agent,
            "iroh peer learned from transport manifest"
        );
    }

    // Durable projection (requires an agent_cid — the manifest table is keyed
    // by it; an announcer without one is dial-only until its binding lands).
    #[cfg(feature = "p2p-iroh")]
    if let (Some(pool), Some(agent_cid)) = (ctx.db_pool, ann.agent_cid.as_deref()) {
        if let Ok(mut conn) = pool.get() {
            let planes: Vec<crate::p2p_iroh::peer_map::Plane> = ann
                .planes
                .iter()
                .filter_map(|p| crate::p2p_iroh::peer_map::Plane::parse(p))
                .collect();
            let relays: Vec<String> = ann.iroh_relay_url.iter().cloned().collect();
            if let Err(e) = crate::p2p_iroh::peer_map::record_iroh_observation(
                &mut conn,
                agent_cid,
                &ann.iroh_node_id,
                &relays,
                &planes,
                ann.announced_at_ms / 1000,
            ) {
                debug!(error = %e, "peer_transport_manifest upsert from transport manifest failed");
            }
        }
    }
}

/// Custody-announcement projection into `shard_locations` as a `peer-announced`
/// row (Category C, cross-pod convergence). Admission
/// ([`crate::p2p::custody_announce::admit`]: signature, signer = author, C1 holder
/// binding) → dedup → resolve THIS node's self `agent_cid` → apply under the
/// never-overwrite-local + monotone-newer rule. A peer telling us about ourselves is
/// dropped (`DroppedSelf`); a claim weaker than an existing row is dropped
/// (`dropped_weaker`).
fn handle_custody_announce(ctx: &GossipDispatchCtx<'_>, data: &[u8], source: &GossipSource) {
    use crate::p2p::custody_announce::{admit, CustodyAdmission, CUSTODY_ANNOUNCE_TOPIC};
    use crate::p2p::inventory_gossip::AuthorWitness;

    crate::metrics::inc_custody_announce("received");

    let Some(pool) = ctx.db_pool else {
        debug!(
            target: "elohim_storage::custody",
            "custody announcement: no db_pool configured, skipping projection"
        );
        return;
    };
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            warn!(
                target: "elohim_storage::custody",
                error = %e,
                "custody announcement: db pool exhausted"
            );
            return;
        }
    };

    // The announcement is signed: the signature is the end-to-end author witness on
    // both planes. C1: the holder is the agent `peer_transport_manifest` binds to the
    // signer's transport identity. On libp2p the gossipsub author is a relay identity
    // that refuses only when bound to a different agent. An unsigned announcement is
    // refused on either plane.
    let witness = match source {
        GossipSource::Libp2p { author, .. } => AuthorWitness::Gossipsub(author.as_deref()),
        GossipSource::Iroh { .. } => AuthorWitness::EndToEnd,
    };
    let verdict = {
        let conn_ref: &mut diesel::SqliteConnection = &mut conn;
        let cell = std::cell::RefCell::new(conn_ref);
        admit(
            data,
            witness,
            &mut |transport_id: &str| bound_agent_for_author(&mut cell.borrow_mut(), transport_id),
            &mut |transport_id: &str| relay_agent(&mut cell.borrow_mut(), transport_id),
        )
    };
    let (ann, author_peer) = match verdict {
        CustodyAdmission::Admitted { body, author_peer } => (body, author_peer),
        CustodyAdmission::Held { reason, bytes } => {
            crate::metrics::inc_custody_announce("held");
            if let Err(e) = crate::db::held_envelopes::retain(
                &mut conn,
                CUSTODY_ANNOUNCE_TOPIC,
                &reason.as_reason(),
                &bytes,
            ) {
                warn!(target: "elohim_storage::custody", error = %e, "held custody announcement not retained");
            }
            info!(
                target: "elohim_storage::custody",
                from = %source, ?reason,
                "custody announcement held (unknown epoch or scheme) — retained, not projected"
            );
            return;
        }
        CustodyAdmission::Refused(reason) => {
            crate::metrics::inc_custody_announce(reason.label());
            warn!(
                target: "elohim_storage::custody",
                from = %source,
                %reason,
                "custody announcement refused"
            );
            return;
        }
    };

    // B (fix in A's handler): dedup only claims that passed the author binding,
    // so an unbound relay of the same bytes cannot consume the key first and
    // shadow the bound author's claim.
    // Dedup on the FULL identity incl. timestamp: an exact replay is dropped, but
    // a fresher re-announce (newer `announced_at_ms`) is NOT deduped so the
    // monotone refresh can still land.
    let dedup_key = format!(
        "Custody:{}:{}:{}",
        ann.shard_cid, ann.holder_agent_cid, ann.announced_at_ms
    );
    if !ctx.dedup.insert(&dedup_key) {
        debug!(
            target: "elohim_storage::dedup",
            from = %source,
            shard_cid = %ann.shard_cid,
            holder = %ann.holder_agent_cid,
            "duplicate custody announcement — dropped"
        );
        return;
    }

    // Resolve THIS node's own agent_cid from the active session (best-effort);
    // when known it powers the self-drop guard. `None` still projects safely —
    // a self-claim about a shard we actually hold hits our stronger self-held /
    // announced row and is dropped as `dropped_weaker` regardless.
    let self_agent_cid = crate::reconcile::custody::resolve_self_agent_cid(&mut conn, None);

    match crate::db::shard_locations::apply_peer_announced(
        &mut conn,
        &ann.shard_cid.to_string(),
        &ann.holder_agent_cid,
        &ann.h_app_id,
        ann.announced_at_ms,
        self_agent_cid.as_deref(),
    ) {
        Ok(outcome) => {
            use crate::db::shard_locations::PeerAnnounceOutcome as O;
            let label = match outcome {
                O::Inserted | O::Refreshed => "applied",
                O::DroppedSelf => "dropped_self",
                O::SkippedStronger | O::SkippedStale | O::NotAgentCid => "dropped_weaker",
            };
            crate::metrics::inc_custody_announce(label);
            debug!(
                target: "elohim_storage::custody",
                from = %source,
                shard_cid = %ann.shard_cid,
                holder = %ann.holder_agent_cid,
                status = %ann.status,
                ?outcome,
                author = %author_peer,
                "custody announcement projected"
            );
        }
        Err(e) => warn!(
            target: "elohim_storage::custody",
            from = %source,
            error = %e,
            "apply_peer_announced failed"
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Topic routing table: the handler must recognise every consumer-grade
    /// inbound topic and route unknowns to the untracked arm. This is the
    /// refactor-safety guard — it exercises the same topic-name dispatch the
    /// old inline `if/else if` chain performed, without needing a live swarm.
    #[test]
    fn known_topics_are_routed_not_untracked() {
        // Garbage payloads: the point is that each KNOWN topic is decoded
        // (and fails to decode → warn/debug) rather than hitting the
        // "untracked topic" arm. We assert routing via a no-panic no-DB run.
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = GossipDispatchCtx {
            db_pool: None,
            dedup: &dedup,
            agent_info_inbound_tx: None,
            inventory_fetch: None,
            transport_manifest_sink: None,
            self_iroh_node_id: None,
        };
        let src = GossipSource::Iroh {
            node: "test-node".to_string(),
        };
        for topic in [
            crate::p2p::RECOVERY_INVITATION_TOPIC,
            crate::p2p::identity_binding_gossip::IDENTITY_BINDING_TOPIC,
            crate::p2p::conductor_agent_info_gossip::CONDUCTOR_AGENT_INFO_TOPIC,
            crate::p2p::RECOVERY_REVOCATION_TOPIC,
            crate::p2p::topics::TOPIC_INTEGRITY_REVOCATION,
            crate::p2p::inventory_gossip::INVENTORY_TOPIC,
            crate::p2p::salvage_gossip::SALVAGE_CAPACITY_TOPIC,
            crate::p2p::custody_announce::CUSTODY_ANNOUNCE_TOPIC,
            "elohim/lamad/public",
            "some.unknown.topic",
        ] {
            // Must not panic on any topic with arbitrary bytes.
            handle_gossip(&ctx, topic, b"\x00\x01\x02", &src);
        }
    }

    #[test]
    fn epr_announce_dedups_by_cid() {
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = GossipDispatchCtx {
            db_pool: None,
            dedup: &dedup,
            agent_info_inbound_tx: None,
            inventory_fetch: None,
            transport_manifest_sink: None,
            self_iroh_node_id: None,
        };
        let src = GossipSource::Iroh {
            node: "n".to_string(),
        };
        // msgpack-encoded CID string
        let payload = rmp_serde::to_vec("bafyreiabc").unwrap();
        // First insert is novel, second is a duplicate — both must be safe.
        handle_gossip(&ctx, "elohim/lamad/public", &payload, &src);
        assert!(
            !dedup.insert("bafyreiabc"),
            "cid should already be in the dedup cache after first announce"
        );
    }

    #[cfg(feature = "p2p-iroh")]
    #[test]
    fn transport_manifest_feeds_the_peer_book_and_the_manifest_row_only_when_verified() {
        use crate::p2p::transport_manifest_gossip::{
            TransportManifestAnnouncement, TRANSPORT_MANIFEST_TOPIC,
        };
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let book = crate::p2p_iroh::IrohPeerBook::new();
        let ctx = GossipDispatchCtx {
            db_pool: Some(&pool),
            dedup: &dedup,
            agent_info_inbound_tx: None,
            inventory_fetch: None,
            transport_manifest_sink: Some(&book),
            self_iroh_node_id: Some("00".repeat(32).leak()),
        };
        let src = GossipSource::Libp2p {
            peer: "12D3KooWpeer".to_string(),
            message_id: "m1".to_string(),
            author: Some("12D3KooWpeer".to_string()),
        };
        let secret = [5u8; 32];
        let ann = TransportManifestAnnouncement::sign(
            &secret,
            vec!["127.0.0.1:10703".into()],
            None,
            Some("uhCAkJames".into()),
            Some("12D3KooWjames".into()),
            vec!["sync".into(), "blob".into()],
            1_700_000_000_000,
        );

        // Verified announcement: book gains the peer, manifest row is written.
        handle_gossip(
            &ctx,
            TRANSPORT_MANIFEST_TOPIC,
            &ann.to_bytes().unwrap(),
            &src,
        );
        assert_eq!(book.len(), 1);
        let got = book
            .get(&ann.iroh_node_id.parse().unwrap())
            .expect("in book");
        assert_eq!(got.agent_cid.as_deref(), Some("uhCAkJames"));
        assert_eq!(got.user_agent, None);
        let mut conn = pool.get().unwrap();
        let row = crate::p2p_iroh::peer_map::lookup_by_iroh_node_id(&mut conn, &ann.iroh_node_id)
            .expect("query")
            .expect("manifest row projected");
        assert_eq!(row.agent_cid, "uhCAkJames");

        // Exact replay: deduped, nothing changes.
        handle_gossip(
            &ctx,
            TRANSPORT_MANIFEST_TOPIC,
            &ann.to_bytes().unwrap(),
            &src,
        );
        assert_eq!(book.len(), 1);

        // Tampered (address swapped after signing): refused — book unchanged.
        let mut forged = ann.clone();
        forged.iroh_direct_addrs = vec!["10.9.9.9:1".into()];
        forged.announced_at_ms += 1;
        handle_gossip(
            &ctx,
            TRANSPORT_MANIFEST_TOPIC,
            &forged.to_bytes().unwrap(),
            &src,
        );
        let still = book.get(&ann.iroh_node_id.parse().unwrap()).unwrap();
        assert!(still
            .addr
            .direct_addresses
            .contains(&"127.0.0.1:10703".parse().unwrap()));
        assert_eq!(still.addr.direct_addresses.len(), 1);
    }

    /// A transport identity (libp2p PeerId) and its signer for `secret`.
    fn custody_signer(secret: [u8; 32]) -> (elohim_epr::AgentKeypair, String) {
        let peer = libp2p::identity::Keypair::ed25519_from_bytes(secret)
            .unwrap()
            .public()
            .to_peer_id()
            .to_string();
        (
            elohim_epr::AgentKeypair::from_secret(&secret).unwrap(),
            peer,
        )
    }

    /// `peer_transport_manifest` binds `libp2p_peer` to `agent`.
    fn bind_agent(pool: &crate::db::DbPool, agent: &str, libp2p_peer: &str) {
        use crate::db::diesel_schema::peer_transport_manifest::dsl as t;
        use diesel::prelude::*;
        let mut conn = pool.get().expect("conn");
        diesel::insert_into(t::peer_transport_manifest)
            .values((
                t::agent_cid.eq(agent),
                t::libp2p_peer_id.eq(libp2p_peer),
                t::discovery_methods_json.eq("[]"),
                t::capability_level.eq(1),
                t::last_observed.eq(1_i64),
            ))
            .execute(&mut conn)
            .expect("bind author");
    }

    /// `peer_transport_manifest` binds iroh NodeId `node` to `agent` and no libp2p
    /// PeerId — the shape the transport-manifest gossip projects in a household.
    fn bind_iroh_agent(pool: &crate::db::DbPool, agent: &str, node: &str) {
        use crate::db::diesel_schema::peer_transport_manifest::dsl as t;
        use diesel::prelude::*;
        let mut conn = pool.get().expect("conn");
        diesel::insert_into(t::peer_transport_manifest)
            .values((
                t::agent_cid.eq(agent),
                t::iroh_node_id.eq(node),
                t::discovery_methods_json.eq("[]"),
                t::capability_level.eq(5),
                t::last_observed.eq(1_i64),
            ))
            .execute(&mut conn)
            .expect("bind iroh author");
    }

    fn custody_claim(
        shard: elohim_epr::address::ByteCid,
        holder: &str,
    ) -> crate::p2p::custody_announce::CustodyAnnouncement {
        crate::p2p::custody_announce::CustodyAnnouncement {
            shard_cid: shard,
            holder_agent_cid: holder.to_string(),
            h_app_id: "lamad".to_string(),
            status: "self-held".to_string(),
            announced_at_ms: 1_700_000_000_000,
        }
    }

    fn custody_rows(
        pool: &crate::db::DbPool,
        shard: &elohim_epr::address::ByteCid,
    ) -> Vec<crate::db::models::ShardLocationRow> {
        let mut conn = pool.get().expect("conn");
        crate::db::shard_locations::get_locations_for_shard(&mut conn, &shard.to_string()).unwrap()
    }

    fn libp2p_from(author: Option<&str>) -> GossipSource {
        GossipSource::Libp2p {
            peer: "12D3KooWRelay".to_string(),
            message_id: "m1".to_string(),
            author: author.map(str::to_string),
        }
    }

    fn iroh_from(node: &str) -> GossipSource {
        GossipSource::Iroh {
            node: node.to_string(),
        }
    }

    fn custody_ctx<'a>(
        pool: &'a crate::db::DbPool,
        dedup: &'a crate::p2p::dedup::DedupLru,
    ) -> GossipDispatchCtx<'a> {
        GossipDispatchCtx {
            db_pool: Some(pool),
            dedup,
            agent_info_inbound_tx: None,
            inventory_fetch: None,
            transport_manifest_sink: None,
            self_iroh_node_id: None,
        }
    }

    // ── R1: salvage-capacity ads are signed, bound, fresh and replay-ordered ──

    fn salvage_ad(
        agent: &str,
        seq: u64,
        spare: u64,
    ) -> crate::p2p::salvage_gossip::SalvageCapacityAd {
        crate::p2p::salvage_gossip::SalvageCapacityAd {
            agent_cid: agent.to_string(),
            spare_bytes: spare,
            archetype: "node".to_string(),
            seq,
            issued_at_ms: chrono::Utc::now().timestamp_millis(),
        }
    }

    fn salvage_row(
        pool: &crate::db::DbPool,
        agent: &str,
    ) -> Option<crate::db::models::SalvageCapacityRow> {
        let mut conn = pool.get().expect("conn");
        crate::db::salvage_capacity::lookup_capacity(&mut conn, agent).unwrap()
    }

    fn salvage_row_count(pool: &crate::db::DbPool) -> i64 {
        use crate::db::diesel_schema::salvage_capacity::dsl as t;
        use diesel::prelude::*;
        let mut conn = pool.get().expect("conn");
        t::salvage_capacity.count().get_result(&mut conn).unwrap()
    }

    #[test]
    fn unsigned_salvage_ad_is_refused_and_projects_nothing() {
        use crate::p2p::salvage_gossip::SALVAGE_CAPACITY_TOPIC;
        #[derive(serde::Serialize)]
        struct Stage1Ad {
            agent_cid: String,
            spare_bytes: u64,
            archetype: String,
            seq: u64,
            signature: Vec<u8>,
        }
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        let (_, peer) = custody_signer([41u8; 32]);
        bind_agent(&pool, "uhCAkSalvager", &peer);
        // The retired Stage-1 wire: a placeholder null-byte "signature", nothing signed.
        let unsigned = rmp_serde::to_vec_named(&Stage1Ad {
            agent_cid: "uhCAkSalvager".to_string(),
            spare_bytes: 1 << 30,
            archetype: "node".to_string(),
            seq: 1,
            signature: vec![0x00],
        })
        .unwrap();
        // Even the bare body, without any envelope, from the bound author itself.
        let body_only = rmp_serde::to_vec_named(&salvage_ad("uhCAkSalvager", 2, 1 << 30)).unwrap();
        for bytes in [&unsigned, &body_only] {
            handle_gossip(
                &ctx,
                SALVAGE_CAPACITY_TOPIC,
                bytes,
                &libp2p_from(Some(&peer)),
            );
            handle_gossip(&ctx, SALVAGE_CAPACITY_TOPIC, bytes, &iroh_from("neighbour"));
        }
        assert_eq!(salvage_row_count(&pool), 0, "an unsigned ad never lands");
    }

    #[test]
    fn salvage_ad_from_unbound_source_is_refused() {
        use crate::p2p::salvage_gossip::{SignedSalvageAd, SALVAGE_CAPACITY_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);

        // A validly signed ad whose signer no manifest binds to any agent.
        let (unbound_key, unbound_peer) = custody_signer([42u8; 32]);
        let bytes = SignedSalvageAd::sign(
            &unbound_key,
            &unbound_peer,
            salvage_ad("uhCAkUnboundSalvager", 1, 4096),
        )
        .to_bytes()
        .unwrap();
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &bytes,
            &libp2p_from(Some(&unbound_peer)),
        );
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &bytes,
            &iroh_from("neighbour"),
        );
        assert!(salvage_row(&pool, "uhCAkUnboundSalvager").is_none());

        // A bound signer's ad relayed under a gossipsub source this node binds to a
        // DIFFERENT agent contradicts the ad and is refused.
        let (key, peer) = custody_signer([43u8; 32]);
        bind_agent(&pool, "uhCAkBoundSalvager", &peer);
        let (_, other_peer) = custody_signer([44u8; 32]);
        bind_agent(&pool, "uhCAkSomeoneElse", &other_peer);
        let bytes = SignedSalvageAd::sign(&key, &peer, salvage_ad("uhCAkBoundSalvager", 1, 4096))
            .to_bytes()
            .unwrap();
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &bytes,
            &libp2p_from(Some(&other_peer)),
        );
        assert!(salvage_row(&pool, "uhCAkBoundSalvager").is_none());

        // The same bytes from the bound author itself project.
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &bytes,
            &libp2p_from(Some(&peer)),
        );
        assert_eq!(
            salvage_row(&pool, "uhCAkBoundSalvager")
                .unwrap()
                .spare_bytes,
            4096
        );
        assert_eq!(salvage_row_count(&pool), 1);
    }

    #[test]
    fn replayed_salvage_ad_does_not_reproject() {
        use crate::p2p::salvage_gossip::{SignedSalvageAd, SALVAGE_CAPACITY_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        let (key, peer) = custody_signer([45u8; 32]);
        bind_agent(&pool, "uhCAkSalvager", &peer);
        let sign = |seq, spare| {
            SignedSalvageAd::sign(&key, &peer, salvage_ad("uhCAkSalvager", seq, spare))
                .to_bytes()
                .unwrap()
        };
        let older = sign(10, 100);
        let newer = sign(11, 200);
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &older,
            &libp2p_from(Some(&peer)),
        );
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &newer,
            &iroh_from("neighbour"),
        );
        let applied = salvage_row(&pool, "uhCAkSalvager").unwrap();
        assert_eq!((applied.seq, applied.spare_bytes), (11, 200));

        // Replays — the older ad, and the newest one again, on either plane — change
        // nothing: not the spare bytes, not the seq, not the freshness stamp.
        handle_gossip(&ctx, SALVAGE_CAPACITY_TOPIC, &older, &iroh_from("relay"));
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &older,
            &libp2p_from(Some(&peer)),
        );
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &newer,
            &libp2p_from(Some(&peer)),
        );
        let after = salvage_row(&pool, "uhCAkSalvager").unwrap();
        assert_eq!(
            (after.seq, after.spare_bytes, after.last_seen_at),
            (applied.seq, applied.spare_bytes, applied.last_seen_at)
        );
    }

    #[test]
    fn forged_iroh_salvage_ad_naming_another_holder_is_refused() {
        use crate::p2p::salvage_gossip::{SignedSalvageAd, SALVAGE_CAPACITY_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        // Victim and forger are both bound by iroh NodeId, as in a household.
        let victim = elohim_epr::AgentKeypair::from_secret(&[46u8; 32]).unwrap();
        let victim_node = hex::encode(victim.public_key_bytes());
        bind_iroh_agent(&pool, "uhCAkVictim", &victim_node);
        let forger = elohim_epr::AgentKeypair::from_secret(&[47u8; 32]).unwrap();
        let forger_node = hex::encode(forger.public_key_bytes());
        bind_iroh_agent(&pool, "uhCAkForger", &forger_node);

        // The forger signs with its own key and names the victim as the advertiser…
        let as_victim_agent =
            SignedSalvageAd::sign(&forger, &forger_node, salvage_ad("uhCAkVictim", 1, 1 << 40));
        // …or claims the victim's NodeId as author while signing with its own key.
        let as_victim_author =
            SignedSalvageAd::sign(&forger, &victim_node, salvage_ad("uhCAkVictim", 2, 1 << 40));
        for forged in [as_victim_agent, as_victim_author] {
            handle_gossip(
                &ctx,
                SALVAGE_CAPACITY_TOPIC,
                &forged.to_bytes().unwrap(),
                &iroh_from(&victim_node),
            );
        }
        assert!(
            salvage_row(&pool, "uhCAkVictim").is_none(),
            "no ad the victim did not sign may place it in the salvage pool"
        );

        // The victim's own signed ad over iroh is end-to-end authenticated and lands.
        let genuine =
            SignedSalvageAd::sign(&victim, &victim_node, salvage_ad("uhCAkVictim", 3, 4096));
        handle_gossip(
            &ctx,
            SALVAGE_CAPACITY_TOPIC,
            &genuine.to_bytes().unwrap(),
            &iroh_from(&forger_node),
        );
        assert_eq!(salvage_row(&pool, "uhCAkVictim").unwrap().spare_bytes, 4096);
    }

    #[test]
    fn unknown_scheme_salvage_ad_is_held_not_projected() {
        use crate::p2p::salvage_gossip::{SignedSalvageAd, SALVAGE_CAPACITY_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        let (key, peer) = custody_signer([48u8; 32]);
        bind_agent(&pool, "uhCAkSalvager", &peer);
        let mut unknown_scheme =
            SignedSalvageAd::sign(&key, &peer, salvage_ad("uhCAkSalvager", 1, 4096));
        unknown_scheme.sig.scheme = 77;
        let mut unknown_epoch =
            SignedSalvageAd::sign(&key, &peer, salvage_ad("uhCAkSalvager", 2, 4096));
        unknown_epoch.epoch = 3;
        for held in [unknown_scheme, unknown_epoch] {
            handle_gossip(
                &ctx,
                SALVAGE_CAPACITY_TOPIC,
                &held.to_bytes().unwrap(),
                &libp2p_from(Some(&peer)),
            );
        }
        assert!(
            salvage_row(&pool, "uhCAkSalvager").is_none(),
            "held, never projected"
        );
        let mut conn = pool.get().unwrap();
        assert_eq!(
            crate::db::held_envelopes::count(&mut conn).unwrap(),
            2,
            "both kept opaque in the bounded held table"
        );
    }

    #[test]
    fn custody_announce_projects_a_peer_announced_row_and_dedups_replay() {
        use crate::p2p::custody_announce::{SignedCustodyAnnouncement, CUSTODY_ANNOUNCE_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        let (keypair, peer) = custody_signer([21u8; 32]);
        bind_agent(&pool, "uhCAkRemoteHolder", &peer);
        let shard = elohim_epr::address::compute_bytes_cid(b"shard xyz");
        let bytes = SignedCustodyAnnouncement::sign(
            &keypair,
            &peer,
            custody_claim(shard, "uhCAkRemoteHolder"),
        )
        .to_bytes()
        .expect("encode");

        // The bound signer's claim, relayed by another peer, projects one row.
        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &libp2p_from(Some(&peer)),
        );
        let rows = custody_rows(&pool, &shard);
        assert_eq!(rows.len(), 1, "one peer-announced row projected");
        assert_eq!(rows[0].peer_id, "uhCAkRemoteHolder");
        assert_eq!(
            rows[0].status,
            crate::db::shard_locations::PEER_ANNOUNCED_STATUS
        );

        // An exact replay (on either plane) is deduped — still exactly one row.
        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &libp2p_from(Some(&peer)),
        );
        handle_gossip(&ctx, CUSTODY_ANNOUNCE_TOPIC, &bytes, &iroh_from("relay"));
        assert_eq!(custody_rows(&pool, &shard).len(), 1, "replay deduped");
    }

    /// The regression: a household whose peers meet over iroh never projected
    /// remote custody. A signed claim delivered over iroh — no gossipsub author —
    /// is authenticated by its signature and projects.
    #[test]
    fn signed_announcement_over_iroh_is_applied() {
        use crate::p2p::custody_announce::{SignedCustodyAnnouncement, CUSTODY_ANNOUNCE_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        // A dual node signs with its iroh key as its NodeId; the receiver's manifest
        // binds only that NodeId (no libp2p PeerId), as in the household.
        let keypair = elohim_epr::AgentKeypair::from_secret(&[22u8; 32]).unwrap();
        let node = hex::encode(keypair.public_key_bytes());
        bind_iroh_agent(&pool, "uhCAkIrohHolder", &node);
        let shard = elohim_epr::address::compute_bytes_cid(b"iroh shard");
        let bytes = SignedCustodyAnnouncement::sign(
            &keypair,
            &node,
            custody_claim(shard, "uhCAkIrohHolder"),
        )
        .to_bytes()
        .unwrap();

        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &iroh_from("neighbour"),
        );
        let rows = custody_rows(&pool, &shard);
        assert_eq!(rows.len(), 1, "the iroh-delivered signed claim projects");
        assert_eq!(rows[0].peer_id, "uhCAkIrohHolder");
    }

    /// No C1 regression: an unsigned claim — the pre-envelope v2 wire — projects
    /// nothing on either plane, even from a bound gossipsub author.
    #[test]
    fn unsigned_announcement_is_refused_on_both_planes() {
        use crate::p2p::custody_announce::CUSTODY_ANNOUNCE_TOPIC;
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        let (_, peer) = custody_signer([23u8; 32]);
        bind_agent(&pool, "uhCAkUnsignedHolder", &peer);
        let shard = elohim_epr::address::compute_bytes_cid(b"unsigned shard");
        let bytes = custody_claim(shard, "uhCAkUnsignedHolder")
            .to_bytes()
            .unwrap();

        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &libp2p_from(Some(&peer)),
        );
        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &iroh_from("neighbour"),
        );
        assert!(
            custody_rows(&pool, &shard).is_empty(),
            "unsigned claims never land"
        );
    }

    /// On libp2p a gossipsub author bound to a DIFFERENT agent than the signer
    /// contradicts the claim and is refused, without consuming the dedup key the
    /// genuine delivery needs.
    #[test]
    fn libp2p_author_mismatch_is_refused() {
        use crate::p2p::custody_announce::{SignedCustodyAnnouncement, CUSTODY_ANNOUNCE_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        let (keypair, peer) = custody_signer([24u8; 32]);
        let (_, stranger) = custody_signer([25u8; 32]);
        bind_agent(&pool, "uhCAkMismatchHolder", &peer);
        bind_agent(&pool, "uhCAkStranger", &stranger);
        let shard = elohim_epr::address::compute_bytes_cid(b"mismatch shard");
        let bytes = SignedCustodyAnnouncement::sign(
            &keypair,
            &peer,
            custody_claim(shard, "uhCAkMismatchHolder"),
        )
        .to_bytes()
        .unwrap();

        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &libp2p_from(Some(&stranger)),
        );
        assert!(
            custody_rows(&pool, &shard).is_empty(),
            "author mismatch projects nothing"
        );

        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &libp2p_from(Some(&peer)),
        );
        assert_eq!(
            custody_rows(&pool, &shard).len(),
            1,
            "the signer's delivery lands"
        );
    }

    /// The household shape (2026-11-06, matthew/jessica refusing james): a claim
    /// signed with james's iroh key, the manifest binding only his NodeId, arriving
    /// over gossipsub under his libp2p PeerId that no binding names. The relay source
    /// does not contradict the claim, so it projects; a source an identity binding
    /// attributes to another agent refuses.
    #[test]
    fn an_iroh_signed_claim_relayed_over_gossipsub_projects_unless_the_relay_is_another_agent() {
        use crate::p2p::custody_announce::{SignedCustodyAnnouncement, CUSTODY_ANNOUNCE_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&pool, &dedup);
        let keypair = elohim_epr::AgentKeypair::from_secret(&[27u8; 32]).unwrap();
        let node = hex::encode(keypair.public_key_bytes());
        bind_iroh_agent(&pool, "uhCAkJames", &node);
        {
            let mut conn = pool.get().unwrap();
            crate::db::peer_identity_bindings::upsert(
                &mut conn,
                &crate::db::models::NewPeerIdentityBindingRow {
                    peer_id: "12D3KooWMallory".into(),
                    agent_cid: "uhCAkMallory".into(),
                    dht_anchor_hash: "binding-anchor".into(),
                    valid_from: "2026-01-01T00:00:00Z".into(),
                    valid_until: None,
                    observed_at: "2026-01-01T00:00:00Z".into(),
                    source: "dht".into(),
                    device_archetype: "node".into(),
                    superseded_by: None,
                    signature: String::new(),
                    proof_status: crate::p2p::binding_proof_wire::BindingProofStatus::unverified(),
                },
            )
            .unwrap();
        }
        let claim = |text: &[u8]| {
            let shard = elohim_epr::address::compute_bytes_cid(text);
            let bytes = SignedCustodyAnnouncement::sign(
                &keypair,
                &node,
                custody_claim(shard, "uhCAkJames"),
            )
            .to_bytes()
            .unwrap();
            (shard, bytes)
        };

        let (shard, bytes) = claim(b"relayed by an unbound peer");
        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &libp2p_from(Some("12D3KooWJamesLibp2p")),
        );
        let rows = custody_rows(&pool, &shard);
        assert_eq!(rows.len(), 1, "an unbound relay source does not refuse");
        assert_eq!(rows[0].peer_id, "uhCAkJames");

        let (shard, bytes) = claim(b"relayed by another agent's peer");
        handle_gossip(
            &ctx,
            CUSTODY_ANNOUNCE_TOPIC,
            &bytes,
            &libp2p_from(Some("12D3KooWMallory")),
        );
        assert!(
            custody_rows(&pool, &shard).is_empty(),
            "a relay bound to another agent contradicts the claim"
        );
    }

    /// The push-ack flow end to end: the distributor pushes a shard with its app
    /// scope; the HOLDER stores it, records its own `self-held` row, signs its claim
    /// with its transport key and publishes it; another peer receiving that claim
    /// over iroh projects the holder — the claim the distributor could never author.
    #[tokio::test]
    async fn push_ack_custody_is_claimed_by_the_holder_and_projected_by_peers() {
        use crate::p2p::custody_announce::{
            holder_claim_after_store, publish_custody_claim, CUSTODY_ANNOUNCE_TOPIC,
        };
        use crate::p2p::shard_protocol::{ShardRequest, ShardResponse};
        use crate::services::custody_standing::Requester;
        use crate::services::gossip_flood::{GossipPublisher, PublishError};
        use crate::shard_service::{HeldShardSink, ShardService};
        use std::sync::{Arc, Mutex};

        struct Reported(Mutex<Vec<(elohim_epr::address::ByteCid, String)>>);
        impl HeldShardSink for Reported {
            fn held(&self, cid: elohim_epr::address::ByteCid, h_app_id: String) {
                self.0.lock().unwrap().push((cid, h_app_id));
            }
        }
        struct Published(Mutex<Vec<(String, Vec<u8>)>>);
        impl GossipPublisher for Published {
            fn publish(&self, topic: &str, payload: Vec<u8>) -> Result<(), PublishError> {
                self.0.lock().unwrap().push((topic.to_string(), payload));
                Ok(())
            }
        }

        // Holder: stores the pushed bytes and learns the scope.
        let dir = tempfile::tempdir().unwrap();
        let pantry = Arc::new(
            crate::blob_store::BlobStore::new(dir.path().to_path_buf())
                .await
                .unwrap(),
        );
        let holder_pool = crate::test_util::test_pool();
        let service = ShardService::new(pantry.clone(), Some(holder_pool.clone()));
        let reported = Arc::new(Reported(Mutex::new(Vec::new())));
        service.set_held_shard_sink(reported.clone());
        let data = b"a placed shard".to_vec();
        let shard = elohim_epr::address::compute_bytes_cid(&data);
        let response = service
            .handle(
                &Requester::libp2p("12D3KooWDistributor"),
                ShardRequest::Push {
                    cid: shard,
                    data,
                    h_app_id: Some("lamad".to_string()),
                },
            )
            .await;
        assert!(matches!(response, ShardResponse::PushAck), "{response:?}");
        let (cid, scope) = reported
            .0
            .lock()
            .unwrap()
            .pop()
            .expect("holder learns the scope");

        // The holder records and signs its own claim.
        // The pantry is probed `Verified` just below; the record reads that verdict.
        struct Pantry;
        impl crate::reconcile::custody::LocalBlobStore for Pantry {
            fn has(&self, _cid: &elohim_epr::address::ByteCid) -> bool {
                true
            }
        }
        assert_eq!(
            pantry.exists(&cid).await,
            crate::blob_store::Exists::Verified
        );
        let (keypair, holder_peer) = custody_signer([26u8; 32]);
        let claim = {
            let mut conn = holder_pool.get().unwrap();
            holder_claim_after_store(
                &mut conn,
                &Pantry,
                &cid,
                "uhCAkPushHolder",
                &scope,
                1_700_000_000_000,
            )
            .unwrap()
            .expect("self-held recorded")
        };
        let own = custody_rows(&holder_pool, &shard);
        assert_eq!(own.len(), 1);
        assert_eq!(
            own[0].status,
            crate::services::self_stewardship::SELF_HELD_STATUS
        );
        let published = Published(Mutex::new(Vec::new()));
        assert!(publish_custody_claim(
            Some(&keypair),
            &holder_peer,
            &published,
            claim
        ));
        let (topic, bytes) = published.0.lock().unwrap().pop().expect("claim published");
        assert_eq!(topic, CUSTODY_ANNOUNCE_TOPIC);

        // The distributor (or any peer) projects the holder from the signed claim.
        let peer_pool = crate::test_util::test_pool();
        bind_agent(&peer_pool, "uhCAkPushHolder", &holder_peer);
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = custody_ctx(&peer_pool, &dedup);
        handle_gossip(&ctx, &topic, &bytes, &iroh_from("neighbour"));
        let rows = custody_rows(&peer_pool, &shard);
        assert_eq!(
            rows.len(),
            1,
            "the holder's claim projects at the distributor"
        );
        assert_eq!(rows[0].peer_id, "uhCAkPushHolder");
        assert_eq!(rows[0].h_app_id, "lamad");
    }

    #[test]
    fn transport_key_is_the_key_a_peer_id_embeds() {
        use elohim_epr::signature::Multikey;
        let kp = libp2p::identity::Keypair::ed25519_from_bytes([7u8; 32]).unwrap();
        let ed = kp.public().try_into_ed25519().unwrap().to_bytes();
        let peer = kp.public().to_peer_id().to_string();
        assert_eq!(transport_key(&peer), Some(Multikey::ed25519(&ed)));
        assert_eq!(
            transport_key(&hex::encode(ed)),
            Some(Multikey::ed25519(&ed))
        );
        assert_eq!(transport_key("not-a-peer"), None);
    }

    /// End to end on libp2p: a delta signed by the key the author's PeerId embeds,
    /// from an author the transport manifest binds, projects into
    /// `peer_blob_inventory`; the same bytes claimed by another author project
    /// nothing.
    #[test]
    fn a_bound_authors_signed_delta_projects_and_a_relabelled_one_does_not() {
        use crate::p2p::inventory_gossip::{
            BlobAddress, InventoryDelta, InventoryEnvelope, InventoryLevel, INVENTORY_TOPIC,
        };
        use diesel::prelude::*;
        let secret = [7u8; 32];
        let peer = libp2p::identity::Keypair::ed25519_from_bytes(secret)
            .unwrap()
            .public()
            .to_peer_id()
            .to_string();
        let pool = crate::test_util::test_pool();
        {
            use crate::db::diesel_schema::peer_transport_manifest::dsl as t;
            let mut conn = pool.get().unwrap();
            diesel::insert_into(t::peer_transport_manifest)
                .values((
                    t::agent_cid.eq("uhCAkInventoryHolder"),
                    t::libp2p_peer_id.eq(peer.as_str()),
                    t::discovery_methods_json.eq("[]"),
                    t::capability_level.eq(1),
                    t::last_observed.eq(1_i64),
                ))
                .execute(&mut conn)
                .unwrap();
        }
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = GossipDispatchCtx {
            db_pool: Some(&pool),
            dedup: &dedup,
            agent_info_inbound_tx: None,
            inventory_fetch: None,
            transport_manifest_sink: None,
            self_iroh_node_id: None,
        };
        let cid = elohim_epr::address::compute_bytes_cid(b"advertised payload");
        let keypair = elohim_epr::AgentKeypair::from_secret(&secret).unwrap();
        let envelope = InventoryEnvelope::sign(
            &keypair,
            &peer,
            InventoryLevel::Full,
            InventoryDelta {
                added: vec![BlobAddress::from(cid)],
                hints: vec![],
                removed: vec![],
                sequence: 1,
                emitted_at: 1_700_000_000_000_000,
            },
        );
        let hosts = || {
            let mut conn = pool.get().unwrap();
            crate::db::peer_blob_inventory::lookup_hosts(
                &mut conn,
                &cid.to_string(),
                "2000-01-01T00:00:00Z",
            )
            .unwrap()
        };
        let from = |author: &str| GossipSource::Libp2p {
            peer: "12D3KooWRelay".to_string(),
            message_id: "m".to_string(),
            author: Some(author.to_string()),
        };

        handle_gossip(
            &ctx,
            INVENTORY_TOPIC,
            &envelope.to_bytes(),
            &from("12D3KooWOther"),
        );
        assert!(hosts().is_empty(), "author mismatch projects nothing");

        handle_gossip(&ctx, INVENTORY_TOPIC, &envelope.to_bytes(), &from(&peer));
        let rows = hosts();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].peer_id, peer);
    }

    #[test]
    fn gossip_source_display() {
        let l = GossipSource::Libp2p {
            peer: "12D3Koo".to_string(),
            message_id: "m1".to_string(),
            author: None,
        };
        let i = GossipSource::Iroh {
            node: "node123".to_string(),
        };
        assert_eq!(format!("{l}"), "12D3Koo");
        assert_eq!(format!("{i}"), "iroh:node123");
    }
}
