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
        /// `propagation_source` PeerId (base58) — for logging only.
        peer: String,
        /// gossipsub `message_id` — for logging only.
        message_id: String,
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
        hashes: &[String],
    );

    /// Request a fresh snapshot from a peer after a delta sequence gap.
    fn request_snapshot_for_gap(&self, peer_id: &str);
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
    } else if topic == crate::p2p::inventory_gossip::INVENTORY_TOPIC {
        handle_inventory(ctx, data, source);
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

/// Inventory snapshot/delta projection into `peer_blob_inventory`. The
/// commitment-driven active fetch + delta-gap snapshot-request run only when
/// `ctx.inventory_fetch` is `Some` (both active receive planes; see module docs).
fn handle_inventory(ctx: &GossipDispatchCtx<'_>, data: &[u8], source: &GossipSource) {
    use crate::p2p::inventory_gossip::{BlobInventoryDelta, BlobInventorySnapshot};

    if let Ok(snapshot) = BlobInventorySnapshot::from_bytes(data) {
        if let Err(e) = snapshot.verify_structural() {
            warn!(
                target: "elohim_storage::inventory",
                from = %source,
                error = ?e,
                "Inventory snapshot failed structural verify — dropped"
            );
        } else if let Some(pool) = ctx.db_pool {
            match pool.get() {
                Ok(mut conn) => {
                    let now_iso = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                    let when = crate::p2p::micros_to_iso(snapshot.snapshot_at).unwrap_or(now_iso);
                    let hashes_str: Vec<String> = snapshot
                        .hashes
                        .iter()
                        .map(|b| b.as_str().to_string())
                        .collect();
                    match crate::db::peer_blob_inventory::apply_snapshot(
                        &mut conn,
                        &snapshot.peer_id,
                        &hashes_str,
                        snapshot.sequence as i64,
                        &when,
                    ) {
                        Ok(crate::db::peer_blob_inventory::SnapshotApplyOutcome::Applied) => {
                            info!(
                                target: "elohim_storage::inventory",
                                from = %source,
                                peer_id = %snapshot.peer_id,
                                count = snapshot.hashes.len(),
                                sequence = snapshot.sequence,
                                "Inventory snapshot applied"
                            );
                            if let Some(fetch) = ctx.inventory_fetch {
                                fetch.score_and_enqueue(
                                    &mut conn,
                                    &snapshot.peer_id,
                                    &snapshot.hints,
                                    &hashes_str,
                                );
                            }
                        }
                        Ok(crate::db::peer_blob_inventory::SnapshotApplyOutcome::Deduplicated) => {
                            debug!(
                                target: "elohim_storage::inventory",
                                from = %source,
                                peer_id = %snapshot.peer_id,
                                sequence = snapshot.sequence,
                                "Inventory snapshot deduplicated (identical content)"
                            );
                        }
                        Err(e) => warn!(
                            target: "elohim_storage::inventory",
                            from = %source,
                            error = %e,
                            "apply_snapshot failed"
                        ),
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    "inventory: db pool exhausted"
                ),
            }
        }
    } else if let Ok(delta) = BlobInventoryDelta::from_bytes(data) {
        if let Err(e) = delta.verify_structural() {
            warn!(
                target: "elohim_storage::inventory",
                from = %source,
                error = ?e,
                "Inventory delta failed structural verify — dropped"
            );
        } else if let Some(pool) = ctx.db_pool {
            match pool.get() {
                Ok(mut conn) => {
                    let when = crate::p2p::micros_to_iso(delta.emitted_at).unwrap_or_else(|| {
                        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
                    });
                    let added_str: Vec<String> =
                        delta.added.iter().map(|b| b.as_str().to_string()).collect();
                    let removed_str: Vec<String> = delta
                        .removed
                        .iter()
                        .map(|b| b.as_str().to_string())
                        .collect();
                    match crate::db::peer_blob_inventory::apply_delta(
                        &mut conn,
                        &delta.peer_id,
                        &added_str,
                        &removed_str,
                        delta.sequence as i64,
                        &when,
                    ) {
                        Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Applied) => {
                            debug!(
                                target: "elohim_storage::inventory",
                                peer_id = %delta.peer_id,
                                sequence = delta.sequence,
                                added = delta.added.len(),
                                removed = delta.removed.len(),
                                "Inventory delta applied"
                            );
                            if !delta.added.is_empty() {
                                if let Some(fetch) = ctx.inventory_fetch {
                                    fetch.score_and_enqueue(
                                        &mut conn,
                                        &delta.peer_id,
                                        &delta.hints,
                                        &added_str,
                                    );
                                }
                            }
                        }
                        Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Replay) => {
                            debug!(
                                target: "elohim_storage::inventory",
                                peer_id = %delta.peer_id,
                                sequence = delta.sequence,
                                "Inventory delta replay — dropped silently"
                            );
                        }
                        Ok(crate::db::peer_blob_inventory::DeltaApplyOutcome::Gap {
                            expected,
                            received,
                        }) => {
                            warn!(
                                target: "elohim_storage::inventory",
                                peer_id = %delta.peer_id,
                                expected,
                                received,
                                "Inventory delta gap — requesting snapshot"
                            );
                            if let Some(fetch) = ctx.inventory_fetch {
                                fetch.request_snapshot_for_gap(&delta.peer_id);
                            }
                        }
                        Err(e) => warn!(
                            target: "elohim_storage::inventory",
                            error = %e,
                            "apply_delta failed"
                        ),
                    }
                }
                Err(e) => warn!(
                    target: "elohim_storage::inventory",
                    error = %e,
                    "inventory: db pool exhausted"
                ),
            }
        }
    } else {
        debug!(
            target: "elohim_storage::inventory",
            from = %source,
            "Inventory message decoded as neither snapshot nor delta — dropped"
        );
    }
}

/// Salvage-capacity advertisement projection into `salvage_capacity`.
fn handle_salvage(ctx: &GossipDispatchCtx<'_>, data: &[u8], source: &GossipSource) {
    use crate::p2p::salvage_gossip::SalvageCapacityAd;

    match SalvageCapacityAd::from_bytes(data) {
        Ok(ad) => {
            if let Err(e) = ad.verify_structural() {
                warn!(
                    target: "elohim_storage::salvage",
                    from = %source,
                    error = ?e,
                    "Salvage-capacity ad failed structural verify — dropped"
                );
            } else if let Some(pool) = ctx.db_pool {
                match pool.get() {
                    Ok(mut conn) => {
                        let when = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
                        match crate::db::salvage_capacity::apply_capacity(
                            &mut conn,
                            &ad.agent_cid,
                            ad.spare_bytes as i64,
                            &ad.archetype,
                            &when,
                            ad.seq as i64,
                        ) {
                            Ok(()) => debug!(
                                target: "elohim_storage::salvage",
                                from = %source,
                                agent_cid = %ad.agent_cid,
                                spare_bytes = ad.spare_bytes,
                                archetype = %ad.archetype,
                                seq = ad.seq,
                                "Salvage-capacity ad applied"
                            ),
                            Err(e) => warn!(
                                target: "elohim_storage::salvage",
                                from = %source,
                                error = %e,
                                "apply_capacity failed"
                            ),
                        }
                    }
                    Err(e) => warn!(
                        target: "elohim_storage::salvage",
                        error = %e,
                        "salvage: db pool exhausted"
                    ),
                }
            }
        }
        Err(e) => debug!(
            target: "elohim_storage::salvage",
            from = %source,
            error = %e,
            "Salvage-capacity ad failed to decode — dropped"
        ),
    }
}

/// Custody-announcement projection into `shard_locations` as a `peer-announced`
/// row (Category C, cross-pod convergence). Decode → structural verify → dedup →
/// resolve THIS node's self `agent_cid` → apply under the never-overwrite-local +
/// monotone-newer rule. A peer telling us about ourselves is dropped
/// (`DroppedSelf`); a claim weaker than an existing row is dropped
/// (`dropped_weaker`).
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

fn handle_custody_announce(ctx: &GossipDispatchCtx<'_>, data: &[u8], source: &GossipSource) {
    use crate::p2p::custody_announce::CustodyAnnouncement;

    let ann = match CustodyAnnouncement::from_bytes(data) {
        Ok(a) => a,
        Err(e) => {
            debug!(
                target: "elohim_storage::custody",
                from = %source,
                error = %e,
                "CustodyAnnouncement decode failed — dropped"
            );
            return;
        }
    };
    if let Err(reason) = ann.verify_structural() {
        warn!(
            target: "elohim_storage::custody",
            from = %source,
            reason = %reason,
            "CustodyAnnouncement failed structural verify — dropped"
        );
        return;
    }

    // Dedup on the FULL identity incl. timestamp: an exact replay is dropped, but
    // a fresher re-announce (newer `announced_at_ms`) is NOT deduped so the
    // monotone refresh can still land.
    let dedup_key = format!(
        "Custody:{}:{}:{}",
        ann.shard_hash, ann.holder_agent_cid, ann.announced_at_ms
    );
    if !ctx.dedup.insert(&dedup_key) {
        debug!(
            target: "elohim_storage::dedup",
            from = %source,
            shard_hash = %ann.shard_hash,
            holder = %ann.holder_agent_cid,
            "duplicate custody announcement — dropped"
        );
        return;
    }

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

    // Resolve THIS node's own agent_cid from the active session (best-effort);
    // when known it powers the self-drop guard. `None` still projects safely —
    // a self-claim about a shard we actually hold hits our stronger self-held /
    // announced row and is dropped as `dropped_weaker` regardless.
    let self_agent_cid = crate::reconcile::custody::resolve_self_agent_cid(&mut conn, None);

    match crate::db::shard_locations::apply_peer_announced(
        &mut conn,
        &ann.shard_hash,
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
                shard_hash = %ann.shard_hash,
                holder = %ann.holder_agent_cid,
                status = %ann.status,
                ?outcome,
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

    #[test]
    fn custody_announce_projects_a_peer_announced_row_and_dedups_replay() {
        use crate::p2p::custody_announce::{CustodyAnnouncement, CUSTODY_ANNOUNCE_TOPIC};
        let pool = crate::test_util::test_pool();
        let dedup = crate::p2p::dedup::DedupLru::new();
        let ctx = GossipDispatchCtx {
            db_pool: Some(&pool),
            dedup: &dedup,
            agent_info_inbound_tx: None,
            inventory_fetch: None,
            transport_manifest_sink: None,
            self_iroh_node_id: None,
        };
        let src = GossipSource::Iroh {
            node: "peer-a".to_string(),
        };
        let ann = CustodyAnnouncement {
            shard_hash: "sha256-xyz".to_string(),
            holder_agent_cid: "uhCAkRemoteHolder".to_string(),
            h_app_id: "lamad".to_string(),
            status: "self-held".to_string(),
            announced_at_ms: 1_700_000_000_000,
            signature: vec![0x00],
        };
        let bytes = ann.to_bytes().expect("encode");

        // First arrival projects a peer-announced row.
        handle_gossip(&ctx, CUSTODY_ANNOUNCE_TOPIC, &bytes, &src);
        let mut conn = pool.get().expect("conn");
        let rows =
            crate::db::shard_locations::get_locations_for_shard(&mut conn, "sha256-xyz").unwrap();
        assert_eq!(rows.len(), 1, "one peer-announced row projected");
        assert_eq!(rows[0].peer_id, "uhCAkRemoteHolder");
        assert_eq!(
            rows[0].status,
            crate::db::shard_locations::PEER_ANNOUNCED_STATUS
        );

        // An exact replay is deduped — still exactly one row (no churn, no panic).
        handle_gossip(&ctx, CUSTODY_ANNOUNCE_TOPIC, &bytes, &src);
        let rows =
            crate::db::shard_locations::get_locations_for_shard(&mut conn, "sha256-xyz").unwrap();
        assert_eq!(rows.len(), 1, "replay deduped — no duplicate row");
    }

    #[test]
    fn gossip_source_display() {
        let l = GossipSource::Libp2p {
            peer: "12D3Koo".to_string(),
            message_id: "m1".to_string(),
        };
        let i = GossipSource::Iroh {
            node: "node123".to_string(),
        };
        assert_eq!(format!("{l}"), "12D3Koo");
        assert_eq!(format!("{i}"), "iroh:node123");
    }
}
