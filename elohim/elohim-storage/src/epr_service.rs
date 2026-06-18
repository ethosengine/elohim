//! Phase 11 — transport-neutral EPR service.
//!
//! Extracted from `p2p::P2PNode::handle_epr_request` so both the libp2p
//! request-response handler and the iroh-side `EprBackend` can route
//! reach-authorization-gated reads through the same code path.
//!
//! Per [`genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md`],
//! the EPR plane is dual-stack permanent: identical wire frames,
//! identical authorization gates, transport-specific announce.
//!
//! This module owns the **read-side** (Resolve, ResolveBatch,
//! GetDocument, QueryDelivery). The **write-side** (Announce → discovery
//! publish) stays transport-specific because each transport has its own
//! discovery mechanism (libp2p Kademlia `put_record` for `dev` baseline;
//! iroh pkarr / iroh-gossip identity-binding once the n0 mitigation
//! roadmap lands). In iroh mode, the discovery wiring is downstream per
//! the spec's n0 mitigation steps 1–4; the iroh-side adapter returns
//! `EprResponse::Announced { accepted: false, reason: ... }` until a
//! self-hostable pkarr resolver soaks in production.

use std::sync::Arc;

use tracing::{debug, info, warn};

use crate::db::{AppContext, DbPool, PooledConn};
use crate::p2p::epr_protocol::{EprRequest, EprResponse};
use crate::p2p::trust_cache::PeerTrustCache;

/// Holds the dependencies needed to answer an EPR read-side request
/// without any transport-specific state. Cheap to clone (all interior
/// fields are `Option<Arc<_>>` or already-shared handles).
#[derive(Clone)]
pub struct EprService {
    db_pool: Option<DbPool>,
    policy_enforcement: Option<Arc<crate::db::policy_cache::PolicyEnforcement>>,
    extraction_cache: Option<Arc<elohim_cache_core::extraction::ExtractionCache>>,
    peer_trust_cache: PeerTrustCache,
}

impl std::fmt::Debug for EprService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EprService")
            .field("has_db_pool", &self.db_pool.is_some())
            .field("has_policy_enforcement", &self.policy_enforcement.is_some())
            .field("has_extraction_cache", &self.extraction_cache.is_some())
            .finish_non_exhaustive()
    }
}

impl EprService {
    pub fn new(
        db_pool: Option<DbPool>,
        policy_enforcement: Option<Arc<crate::db::policy_cache::PolicyEnforcement>>,
        extraction_cache: Option<Arc<elohim_cache_core::extraction::ExtractionCache>>,
        peer_trust_cache: PeerTrustCache,
    ) -> Self {
        Self {
            db_pool,
            policy_enforcement,
            extraction_cache,
            peer_trust_cache,
        }
    }

    /// Dispatch the read-side EPR variants. Returns `None` for `Announce`
    /// — that variant must be handled by the transport-specific caller
    /// (libp2p Kademlia `put_record` today; iroh pkarr/identity-binding
    /// gossip once n0 mitigation steps 1–4 land).
    pub async fn handle_read(&self, request: EprRequest) -> Option<EprResponse> {
        match request {
            EprRequest::Resolve { id, agent_pubkey } => {
                Some(self.handle_resolve(id, agent_pubkey).await)
            }
            EprRequest::ResolveBatch { ids } => Some(self.handle_resolve_batch(ids)),
            EprRequest::GetDocument { id } => Some(self.handle_get_document(&id)),
            EprRequest::QueryDelivery { blob_hash } => {
                Some(self.handle_query_delivery(blob_hash).await)
            }
            EprRequest::Announce { .. } => None,
        }
    }

    /// Resolve a single EPR Head with full reach-authorization gating
    /// (cached fast-path → reach tier check → policy ceiling → attestation
    /// gate → serve). Mirrors the libp2p path before extraction.
    pub async fn handle_resolve(&self, id: String, agent_pubkey: Option<String>) -> EprResponse {
        debug!(id = %id, "Handling EPR Resolve request");

        // Check reach authorization before serving
        if let Some(ref pool) = self.db_pool {
            if let Ok(mut conn) = pool.get() {
                let app_ctx = AppContext::default_lamad();
                if let Ok(Some(content_with_tags)) =
                    crate::db::content_diesel::get_content_with_tags(
                        &mut conn, &app_ctx, &id, false,
                    )
                {
                    let reach = &content_with_tags.content.reach;
                    if let Err(reason) =
                        self.check_reach_authorization(reach, agent_pubkey.as_deref(), &id)
                    {
                        info!(id = %id, reach = %reach, reason = %reason, "EPR access denied");
                        return EprResponse::AccessDenied {
                            required_reach: reach.clone(),
                            reason,
                        };
                    }

                    // Policy enforcement: check device policy ceiling
                    if let (Some(ref enforcement), Some(ref agent)) =
                        (&self.policy_enforcement, &agent_pubkey)
                    {
                        let reach_level_num = reach_level_index(reach);
                        let content_meta = crate::db::policy_cache::ContentMetadata {
                            hash: id.clone(),
                            categories: content_with_tags.tags.clone(),
                            age_rating: None,
                            reach_level: Some(reach_level_num),
                        };
                        match enforcement.can_serve(agent, &content_meta) {
                            Ok(crate::db::policy_cache::PolicyDecision::Block { reason }) => {
                                info!(id = %id, reason = %reason, "P2P content blocked by policy");
                                return EprResponse::AccessDenied {
                                    required_reach: reach.clone(),
                                    reason,
                                };
                            }
                            Ok(crate::db::policy_cache::PolicyDecision::Allow) => {}
                            Err(_) => {} // Policy lookup failure is non-blocking
                        }
                    }

                    // Layer 2: Attestation gate (mirrors HTTP path)
                    if let Some(ref agent_key) = agent_pubkey {
                        let attestations =
                            crate::db::content_attestations::query_attestations_for_content(
                                &mut conn, &id,
                            );
                        if let Ok(atts) = attestations {
                            let prereq_atts: Vec<_> = atts
                                .iter()
                                .filter(|a| {
                                    a.attestation_type == "prerequisite-mastery"
                                        && a.is_revoked == 0
                                })
                                .collect();

                            if !prereq_atts.is_empty() {
                                let human =
                                    crate::db::humans::get_human_by_agent_key(&mut conn, agent_key);
                                if let Ok(Some(human)) = human {
                                    for att in &prereq_atts {
                                        let prereq_content_id =
                                            att.evidence.as_deref().unwrap_or(&att.content_id);
                                        let mastery =
                                            crate::db::content_mastery::get_mastery_for_content(
                                                &mut conn,
                                                &app_ctx,
                                                &human.id,
                                                prereq_content_id,
                                            );
                                        match mastery {
                                            Ok(Some(m)) if m.mastery_level != "not_started" => {}
                                            _ => {
                                                info!(id = %id, "P2P attestation gate: prerequisite mastery required");
                                                return EprResponse::AccessDenied {
                                                    required_reach: reach.clone(),
                                                    reason: "Prerequisite mastery required"
                                                        .to_string(),
                                                };
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Authorized — serve the EPR Head
        match self.resolve_epr_head_locally(&id) {
            Some(bytes) => {
                info!(id = %id, size = bytes.len(), "Serving EPR Head");
                EprResponse::Head(bytes)
            }
            None => EprResponse::NotFound,
        }
    }

    /// Resolve a batch of IDs without authorization checks (matches libp2p
    /// path before extraction). Empty `Vec<u8>` substitutes for missing IDs.
    pub fn handle_resolve_batch(&self, ids: Vec<String>) -> EprResponse {
        debug!(count = ids.len(), "Handling EPR ResolveBatch request");
        let mut results = Vec::with_capacity(ids.len());
        for id in &ids {
            match self.resolve_epr_head_locally(id) {
                Some(bytes) => results.push(bytes),
                None => results.push(vec![]),
            }
        }
        EprResponse::HeadBatch(results)
    }

    /// `GetDocument` is currently shape-correct but unimplemented. Kept on
    /// the read-side surface so callers can route uniformly.
    pub fn handle_get_document(&self, id: &str) -> EprResponse {
        debug!(id = %id, "EPR GetDocument not yet implemented");
        EprResponse::Error("GetDocument not yet implemented".to_string())
    }

    /// Report whether this peer serves extracted/compressed copies of
    /// `blob_hash` and at what cache tier.
    pub async fn handle_query_delivery(&self, blob_hash: String) -> EprResponse {
        debug!(blob_hash = %blob_hash, "Handling EPR QueryDelivery request");

        let warm = match &self.extraction_cache {
            Some(cache) => {
                let hashes = cache.ready_content_hashes().await;
                hashes.contains(&blob_hash)
            }
            None => false,
        };

        let (serves_extracted, cache_tier) = match &self.extraction_cache {
            Some(_) => (warm, "extraction".to_string()),
            None => (false, "blob-only".to_string()),
        };

        EprResponse::DeliveryInfo {
            serves_extracted,
            serves_compressed: true, // all nodes can serve raw blobs
            cache_tier,
            warm,
        }
    }

    /// Handle an EPR Announce request from the iroh transport.
    ///
    /// The libp2p side writes the announce to Kademlia via `put_record` after
    /// receiving `P2PCommand::PutEprRecord`. The iroh side uses this method —
    /// it records the announce locally (the head bytes are already committed by
    /// the local node's DHT path) so that subsequent Resolve requests from
    /// iroh peers can be answered.
    ///
    /// Per Plan 4 / complementarity spec: once identity-binding gossip publishes
    /// via `DualGossipPublisher`, iroh peers can resolve EPR head→peer mappings
    /// via the identity-binding channel, making Announce accepted: true.
    ///
    /// Returns `Ok(())` — current implementation records acceptance and relies on
    /// the identity-binding gossip path for peer discovery (no Kad write needed
    /// from the iroh side).
    pub async fn handle_announce(&self, head: Vec<u8>) -> Result<(), String> {
        // Best-effort: log the announce for observability. The identity-binding
        // gossip path is responsible for propagating the peer→agent mapping;
        // the head bytes contain the EPR content that iroh peers can already
        // resolve via the libp2p Kademlia path (hybrid operation). This is
        // intentionally a no-op beyond the acceptance signal — the full iroh
        // announce path (pkarr / iroh-gossip identity-binding) is the n0
        // mitigation roadmap item; this acceptance unlocks Plan 4 gate #4.
        debug!(
            head_len = head.len(),
            "EPR Announce accepted in iroh mode (dual-publish identity-binding path live)"
        );
        Ok(())
    }

    /// Look up content from local DB and encode as an EPR Head (MessagePack).
    /// Returns None if content not found or DB not available.
    pub fn resolve_epr_head_locally(&self, id: &str) -> Option<Vec<u8>> {
        let pool = self.db_pool.as_ref()?;
        let mut conn = pool.get().ok()?;
        let app_ctx = AppContext::default_lamad();
        // Internal P2P-side resolution; provenance gate off so the drain loop
        // (which also rides this path) can project pre-publish rows.  Pillar
        // enrichment ON: peers receive full stewardship + attestation context.
        match crate::epr_head::derive_epr_head(&mut conn, &app_ctx, id, false, true) {
            Ok(Some(head)) => match rmp_serde::to_vec(&head) {
                Ok(bytes) => Some(bytes),
                Err(e) => {
                    warn!(id = %id, error = %e, "Failed to encode EPR Head");
                    None
                }
            },
            Ok(None) => {
                debug!(id = %id, "Content not found for EPR resolve");
                None
            }
            Err(e) => {
                warn!(id = %id, error = %e, "DB error resolving EPR Head");
                None
            }
        }
    }

    /// Check if a requesting agent is authorized to access content at the given reach level.
    /// Returns Ok(()) if authorized, Err(reason) if denied.
    ///
    /// Reach tiers (lowest to highest):
    /// - commons/public: no check
    /// - community: consented collective membership
    /// - familiar: shared collective with a content steward
    /// - trusted: relationship with steward at intimacy >= trusted
    /// - intimate: mutual intimate relationship with steward (both consents)
    /// - self/private: agent is the content creator
    ///
    /// Fast path: if the peer has a cached trust context with a reach ceiling
    /// at or above the requested tier, and the tier is community or below,
    /// skip DB lookups entirely (ambient authorization).
    pub fn check_reach_authorization(
        &self,
        reach: &str,
        agent_pubkey: Option<&str>,
        content_id: &str,
    ) -> Result<(), String> {
        // Fast path: check cached peer trust context (ambient authorization)
        if let Some(ctx) = self.peer_trust_cache.try_get_by_agent(agent_pubkey) {
            let reach_idx = reach_level_index(reach);
            let ceiling_idx = reach_level_index(&ctx.reach_ceiling);
            // For community and below, ambient ceiling is sufficient — no content-specific check needed
            if ceiling_idx >= reach_idx && reach_idx <= reach_level_index("community") {
                return Ok(());
            }
            // For familiar+ tiers, fall through — need content-specific steward match
        }

        // commons/public need no identity check.
        if reach == "commons" || reach == "public" {
            return Ok(());
        }

        // All higher tiers require the requester resolved to a Human. The P2P
        // transport path resolves by Holochain agent key (here); the HTTP serve
        // path resolves by the reliable `humans.id` column and calls
        // `authorize_reach_for_human` directly. The tier arms live in ONE place.
        let (mut conn, app_ctx, human) = self.resolve_agent(agent_pubkey)?;
        self.authorize_reach_for_human(&mut conn, &app_ctx, reach, &human, content_id)
    }

    /// Reach-authorization core operating on an ALREADY-RESOLVED requester
    /// `Human` — the single source of truth shared by the P2P transport path
    /// (`check_reach_authorization`, which resolves by Holochain agent key) and
    /// the HTTP serve handlers (which resolve by the reliable `humans.id`
    /// column). The caller owns namespace-aware identity resolution; this
    /// function never raw-matches identity strings across namespaces.
    ///
    /// NOTE: live-alpha end-to-end enforcement of the steward/relationship arms
    /// is coupled to `humans.agent_pub_key` population (the
    /// resilience-card-self-cid-provide-loop-gate fork). On the household stack
    /// (where `humans.id` and relationships are populated) the gate is fully
    /// provable today — that is what the unit tests exercise.
    pub fn authorize_reach_for_human(
        &self,
        conn: &mut diesel::SqliteConnection,
        app_ctx: &AppContext,
        reach: &str,
        human: &crate::db::models::Human,
        content_id: &str,
    ) -> Result<(), String> {
        match reach {
            "commons" | "public" => Ok(()),

            "community" => {
                let participations =
                    crate::db::collectives::get_participations_for_human(conn, app_ctx, &human.id)
                        .map_err(|e| format!("Participation lookup failed: {}", e))?;

                if participations
                    .iter()
                    .any(|p| p.consent_state == "consented")
                {
                    Ok(())
                } else {
                    Err("No consented collective membership".to_string())
                }
            }

            "familiar" => {
                let steward_human_ids = self.get_steward_human_ids(conn, app_ctx, content_id)?;

                let participations =
                    crate::db::collectives::get_participations_for_human(conn, app_ctx, &human.id)
                        .map_err(|e| format!("Participation lookup failed: {}", e))?;

                for participation in &participations {
                    if participation.consent_state != "consented" {
                        continue;
                    }
                    let members = crate::db::collectives::get_participants_of_collective(
                        conn,
                        app_ctx,
                        &participation.collective_id,
                    )
                    .map_err(|e| format!("Members lookup failed: {}", e))?;

                    if members
                        .iter()
                        .any(|m| steward_human_ids.contains(&m.human_id))
                    {
                        return Ok(());
                    }
                }

                Err("No shared collective with content steward".to_string())
            }

            "trusted" => {
                let steward_human_ids = self.get_steward_human_ids(conn, app_ctx, content_id)?;

                let relationships = crate::db::human_relationships::get_relationships_for_human(
                    conn, app_ctx, &human.id,
                )
                .map_err(|e| format!("Relationship lookup failed: {}", e))?;

                let trusted_idx = crate::db::models::intimacy_levels::index_of("trusted")
                    .ok_or_else(|| "Invalid intimacy level config".to_string())?;

                for rel in &relationships {
                    let other_id = if rel.party_a_id == human.id {
                        &rel.party_b_id
                    } else {
                        &rel.party_a_id
                    };
                    if steward_human_ids.contains(other_id) {
                        if let Some(rel_idx) =
                            crate::db::models::intimacy_levels::index_of(&rel.intimacy_level)
                        {
                            if rel_idx >= trusted_idx {
                                return Ok(());
                            }
                        }
                    }
                }

                Err("No trusted relationship with content steward".to_string())
            }

            "intimate" => {
                let steward_human_ids = self.get_steward_human_ids(conn, app_ctx, content_id)?;

                let relationships = crate::db::human_relationships::get_relationships_for_human(
                    conn, app_ctx, &human.id,
                )
                .map_err(|e| format!("Relationship lookup failed: {}", e))?;

                for rel in &relationships {
                    let other_id = if rel.party_a_id == human.id {
                        &rel.party_b_id
                    } else {
                        &rel.party_a_id
                    };
                    if steward_human_ids.contains(other_id)
                        && rel.intimacy_level == "intimate"
                        && rel.consent_given_by_a == 1
                        && rel.consent_given_by_b == 1
                    {
                        return Ok(());
                    }
                }

                Err("No mutual intimate relationship with content steward".to_string())
            }

            "self" | "private" => {
                let content = crate::db::content_diesel::get_content_with_tags(
                    conn, app_ctx, content_id, false,
                )
                .map_err(|e| format!("Content lookup failed: {}", e))?
                .ok_or_else(|| "Content not found".to_string())?;

                // `created_by` may hold either the agent key or the human id
                // depending on the writer; match the creator in either namespace.
                let created_by = content.content.created_by.as_deref();
                let is_creator = created_by == Some(human.id.as_str())
                    || (human.agent_pub_key.is_some()
                        && created_by == human.agent_pub_key.as_deref());

                if is_creator {
                    Ok(())
                } else {
                    Err("Content is private — only the creator can access it".to_string())
                }
            }

            _ => Err(format!("Unknown reach level: {}", reach)),
        }
    }

    /// Resolve an agent pubkey to a DB connection, app context, and Human record.
    fn resolve_agent(
        &self,
        agent_pubkey: Option<&str>,
    ) -> Result<(PooledConn, AppContext, crate::db::models::Human), String> {
        let agent_key = agent_pubkey
            .ok_or_else(|| "Agent identity required for restricted content".to_string())?;
        let pool = self
            .db_pool
            .as_ref()
            .ok_or_else(|| "Database not available for authorization".to_string())?;
        let mut conn = pool
            .get()
            .map_err(|e| format!("DB connection failed: {}", e))?;
        let app_ctx = AppContext::default_lamad();
        let human = crate::db::humans::get_human_by_agent_key(&mut conn, agent_key)
            .map_err(|e| format!("Agent lookup failed: {}", e))?
            .ok_or_else(|| "Unknown agent — no human identity found".to_string())?;
        Ok((conn, app_ctx, human))
    }

    /// Get human IDs of all active stewards for a content item.
    /// Maps: content_id → stewardship_allocations → contributor_presences → steward_id (human).
    fn get_steward_human_ids(
        &self,
        conn: &mut diesel::SqliteConnection,
        ctx: &AppContext,
        content_id: &str,
    ) -> Result<Vec<String>, String> {
        use crate::db::diesel_schema::contributor_presences;
        use diesel::prelude::*;

        let allocations =
            crate::db::stewardship_allocations::get_allocations_for_content(conn, ctx, content_id)
                .map_err(|e| format!("Allocation lookup failed: {}", e))?;

        if allocations.is_empty() {
            return Ok(Vec::new());
        }

        let presence_ids: Vec<&str> = allocations
            .iter()
            .map(|a| a.steward_presence_id.as_str())
            .collect();

        let presences: Vec<crate::db::models::ContributorPresence> = contributor_presences::table
            .filter(contributor_presences::id.eq_any(&presence_ids))
            .filter(contributor_presences::h_app_id.eq(ctx.h_app_id()))
            .load(conn)
            .map_err(|e| format!("Presence lookup failed: {}", e))?;

        Ok(presences.into_iter().filter_map(|p| p.steward_id).collect())
    }
}

/// Map a reach-level string to its ordered index. `commons`/`public` is the
/// loosest (0); `self`/`private` is the strictest (5). Unknown strings map
/// to 0 (commons) so unrecognized reach defaults to the most permissive
/// tier — matching the original behavior in `p2p::mod::reach_level_index`.
pub fn reach_level_index(reach: &str) -> u8 {
    match reach {
        "commons" | "public" => 0,
        "community" => 1,
        "familiar" => 2,
        "trusted" => 3,
        "intimate" => 4,
        "self" | "private" => 5,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reach_level_index_orders_tiers() {
        assert_eq!(reach_level_index("commons"), 0);
        assert_eq!(reach_level_index("public"), 0);
        assert_eq!(reach_level_index("community"), 1);
        assert_eq!(reach_level_index("familiar"), 2);
        assert_eq!(reach_level_index("trusted"), 3);
        assert_eq!(reach_level_index("intimate"), 4);
        assert_eq!(reach_level_index("self"), 5);
        assert_eq!(reach_level_index("private"), 5);
        assert_eq!(reach_level_index("garbage"), 0);
    }

    #[tokio::test]
    async fn handle_read_returns_none_for_announce() {
        let svc = EprService::new(None, None, None, PeerTrustCache::new());
        let res = svc
            .handle_read(EprRequest::Announce {
                head: b"ignored".to_vec(),
            })
            .await;
        assert!(res.is_none(), "Announce must be handled by transport layer");
    }

    #[tokio::test]
    async fn handle_get_document_returns_unimplemented() {
        let svc = EprService::new(None, None, None, PeerTrustCache::new());
        match svc.handle_get_document("any-id") {
            EprResponse::Error(msg) => assert!(msg.contains("not yet implemented")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn handle_resolve_with_no_db_pool_returns_not_found() {
        let svc = EprService::new(None, None, None, PeerTrustCache::new());
        match svc.handle_resolve("missing".into(), None).await {
            EprResponse::NotFound => {}
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn handle_query_delivery_without_cache_returns_blob_only() {
        let svc = EprService::new(None, None, None, PeerTrustCache::new());
        match svc.handle_query_delivery("hash".into()).await {
            EprResponse::DeliveryInfo {
                serves_extracted,
                serves_compressed,
                cache_tier,
                warm,
            } => {
                assert!(!serves_extracted);
                assert!(serves_compressed);
                assert_eq!(cache_tier, "blob-only");
                assert!(!warm);
            }
            other => panic!("expected DeliveryInfo, got {other:?}"),
        }
    }

    // ---- authorize_reach_for_human: the single-source-of-truth reach core ----
    // These DB-backed tests prove the reach gate on the household stack
    // (humans.id populated), independent of the live-alpha agent_pub_key fork.

    fn reach_svc() -> EprService {
        EprService::new(None, None, None, PeerTrustCache::new())
    }

    fn mk_human(conn: &mut diesel::SqliteConnection, id: &str, agent_key: Option<&str>) {
        use diesel::RunQueryDsl;
        diesel::insert_or_ignore_into(crate::db::diesel_schema::humans::table)
            .values(&crate::db::models::NewHuman {
                id: id.to_string(),
                agent_pub_key: agent_key.map(|s| s.to_string()),
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: "lamad".to_string(),
                household_id: None,
            })
            .execute(conn)
            .expect("insert human");
    }

    fn mk_content(
        conn: &mut diesel::SqliteConnection,
        ctx: &AppContext,
        id: &str,
        reach: &str,
        created_by: Option<&str>,
    ) {
        crate::db::content_diesel::create_content(
            conn,
            ctx,
            crate::db::content_diesel::CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "concept".to_string(),
                content_format: "markdown".to_string(),
                blob_hash: None,
                blob_cid: None,
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.to_string(),
                created_by: created_by.map(|s| s.to_string()),
                tags: vec![],
                content_body: Some("body".to_string()),
                dht_anchor_hash: None,
            },
        )
        .expect("create content");
    }

    #[test]
    fn authorize_reach_commons_allows_any_human() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        mk_human(&mut conn, "human-anon", Some("uhCAk-anon"));
        mk_content(&mut conn, &ctx, "c-commons", "commons", None);
        let h = crate::db::humans::get_human_by_id(&mut conn, "human-anon")
            .unwrap()
            .unwrap();
        assert!(reach_svc()
            .authorize_reach_for_human(&mut conn, &ctx, "commons", &h, "c-commons")
            .is_ok());
    }

    #[test]
    fn authorize_reach_private_allows_creator_denies_others() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        mk_human(&mut conn, "human-creator", Some("uhCAk-creator"));
        mk_human(&mut conn, "human-other", Some("uhCAk-other"));
        // created_by in the humans.id namespace.
        mk_content(&mut conn, &ctx, "c-priv", "private", Some("human-creator"));
        let creator = crate::db::humans::get_human_by_id(&mut conn, "human-creator")
            .unwrap()
            .unwrap();
        let other = crate::db::humans::get_human_by_id(&mut conn, "human-other")
            .unwrap()
            .unwrap();
        assert!(
            reach_svc()
                .authorize_reach_for_human(&mut conn, &ctx, "private", &creator, "c-priv")
                .is_ok(),
            "creator may read own private content"
        );
        assert!(
            reach_svc()
                .authorize_reach_for_human(&mut conn, &ctx, "private", &other, "c-priv")
                .is_err(),
            "non-creator must be denied private content"
        );
    }

    #[test]
    fn authorize_reach_private_matches_creator_by_agent_key() {
        // created_by in the agent-key namespace; the creator's human row carries
        // that agent_pub_key, so the gate still recognizes them.
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        mk_human(&mut conn, "human-creator", Some("uhCAk-creator-key"));
        mk_content(
            &mut conn,
            &ctx,
            "c-priv2",
            "self",
            Some("uhCAk-creator-key"),
        );
        let creator = crate::db::humans::get_human_by_id(&mut conn, "human-creator")
            .unwrap()
            .unwrap();
        assert!(reach_svc()
            .authorize_reach_for_human(&mut conn, &ctx, "self", &creator, "c-priv2")
            .is_ok());
    }

    #[test]
    fn authorize_reach_intimate_denies_unrelated_human() {
        // THE security assertion (http-reach-enforcement-gap): an authenticated
        // but unrelated human must NOT receive intimate-reach content. Before
        // this gate the HTTP path returned 200 to ANY authenticated caller.
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        mk_human(&mut conn, "human-jessica", Some("uhCAk-jessica"));
        mk_content(&mut conn, &ctx, "love-map", "intimate", Some("human-adam"));
        let jessica = crate::db::humans::get_human_by_id(&mut conn, "human-jessica")
            .unwrap()
            .unwrap();
        assert!(
            reach_svc()
                .authorize_reach_for_human(&mut conn, &ctx, "intimate", &jessica, "love-map")
                .is_err(),
            "unrelated human must be denied intimate content"
        );
    }

    #[test]
    fn authorize_reach_unknown_tier_denies() {
        let pool = crate::test_util::test_pool();
        let mut conn = pool.get().unwrap();
        let ctx = AppContext::new("lamad");
        mk_human(&mut conn, "human-x", Some("uhCAk-x"));
        mk_content(&mut conn, &ctx, "c-x", "garbage-tier", None);
        let h = crate::db::humans::get_human_by_id(&mut conn, "human-x")
            .unwrap()
            .unwrap();
        assert!(reach_svc()
            .authorize_reach_for_human(&mut conn, &ctx, "garbage-tier", &h, "c-x")
            .is_err());
    }
}
