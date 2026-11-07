//! Phase 11 — transport-neutral shard service.
//!
//! Extracted from `p2p::P2PNode::handle_shard_request` so both the
//! libp2p request-response handler and the iroh-side `ShardBackend`
//! can route shard fetch / probe / push / inventory through the same
//! code path.
//!
//! Per [`genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md`],
//! the shard plane is dual-stack permanent. Reed-Solomon coding stays
//! in pure Rust; framing is per-transport. This service holds the
//! transport-neutral state (the blob store + the optional content DB
//! pool) and answers each request variant identically regardless of
//! which transport carried it.
//!
//! Every address is a byte CID ([`ByteCid`], decoded by the protocol codec); the
//! payload store is the one authoritative store (C10) — there is no iroh-blobs copy
//! to fall back to.

use std::sync::Arc;

use tracing::{debug, error, info, warn};

use elohim_epr::address::{compute_bytes_cid, ByteCid};

use crate::blob_store::BlobStore;
use crate::db::DbPool;
use crate::error::StorageError;
use crate::p2p::shard_protocol::{self, ShardRequest, ShardResponse};
use crate::private_reach::{
    private_serve_verdict, PrivateServeVerdict, ServeReason, WithholdReason,
};
use crate::services::custody_standing::{CustodyStanding, Requester, RowFacts};

/// Holds the dependencies needed to answer a shard request.
/// `Clone` is cheap (Arc + Option<DbPool>).
#[derive(Clone)]
pub struct ShardService {
    blob_store: Arc<BlobStore>,
    db_pool: Option<DbPool>,
    /// Station 3b (M9) — resolves the custody facts
    /// [`private_serve_verdict`] decides on. Wired automatically whenever a
    /// content pool exists. Production injects one process-lifetime resolver at
    /// the composition root and shares this service across both transports.
    /// A resolver missing while a pool exists fails closed for blob `Get`.
    custody_standing: Option<Arc<dyn CustodyStanding>>,
    /// Where a stored, scope-carrying push is reported so this node — the holder —
    /// can author its own signed custody claim. Set once after the P2P node exists
    /// (the service is built before it and shared with iroh), hence the cell.
    held_shard_sink: Arc<std::sync::OnceLock<Arc<dyn HeldShardSink>>>,
}

/// Receives each pushed shard this node stored (bytes verified against their CID)
/// together with the app scope the placer named. Production: the P2P node, which
/// records the `self-held` row and gossips the holder-signed claim.
pub trait HeldShardSink: Send + Sync {
    fn held(&self, cid: ByteCid, h_app_id: String);
}

impl std::fmt::Debug for ShardService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShardService")
            .field("has_db_pool", &self.db_pool.is_some())
            .field("reach_gated", &self.custody_standing.is_some())
            .field("claims_held_shards", &self.held_shard_sink.get().is_some())
            .finish_non_exhaustive()
    }
}

impl ShardService {
    pub fn new(blob_store: Arc<BlobStore>, db_pool: Option<DbPool>) -> Self {
        Self {
            blob_store,
            db_pool,
            custody_standing: None,
            held_shard_sink: Arc::new(std::sync::OnceLock::new()),
        }
    }

    /// Install the held-shard sink (first call wins; returns whether it was set).
    pub fn set_held_shard_sink(&self, sink: Arc<dyn HeldShardSink>) -> bool {
        self.held_shard_sink.set(sink).is_ok()
    }

    /// Replace the custody-standing resolver — the test seam.
    pub fn with_custody_standing(mut self, standing: Arc<dyn CustodyStanding>) -> Self {
        self.custody_standing = Some(standing);
        self
    }

    /// The shared custody resolver, if reach-gating is wired. Exposed so the
    /// acquisition path (`store_acquired_record`, station 3b receiver-side
    /// pre-authorization) reuses the SAME process-lifetime resolver — and its
    /// TTL cache / single-flight — rather than constructing a second one.
    pub(crate) fn custody_standing(&self) -> Option<Arc<dyn CustodyStanding>> {
        self.custody_standing.clone()
    }

    /// Dispatch a [`ShardRequest`] on behalf of an identified `requester`.
    ///
    /// The requester's transport identity is what makes the custody gate
    /// possible: before station 3b the libp2p `PeerId` was known at the call
    /// site and discarded, and the iroh `Connection` likewise. Three variants
    /// gate — `ListContent` (omit + count), `GetContent` and `Get` (typed
    /// `reach-withheld` refusal). `Have`, `Push` and `GetManifest` are
    /// unchanged this station: `Have` and `GetManifest` answer about bytes the
    /// requester must already name, and `Push` is inbound.
    pub async fn handle(&self, requester: &Requester, request: ShardRequest) -> ShardResponse {
        match request {
            ShardRequest::Get { cid } => self.handle_get(requester, cid).await,
            ShardRequest::Have { cid } => self.handle_have(cid).await,
            ShardRequest::Push {
                cid,
                data,
                h_app_id,
            } => self.handle_push(requester, cid, data, h_app_id).await,
            ShardRequest::ListContent {
                reach_filter,
                offset,
                limit,
            } => {
                self.handle_list_content(requester, reach_filter, offset, limit)
                    .await
            }
            ShardRequest::GetContent { id } => self.handle_get_content(requester, id).await,
            ShardRequest::GetManifest { cid } => self.handle_get_manifest(cid),
        }
    }

    // ────────────────────────────────────────────────────────────────────
    // Station 3b — the custody gate
    // ────────────────────────────────────────────────────────────────────

    /// The verdict for one row, retaining the grant reason for observability.
    ///
    /// Short-circuits on reach: a non-`private` row never reaches the resolver,
    /// so every other tier keeps exactly its pre-station cost and behaviour.
    async fn row_verdict(
        &self,
        requester: &Requester,
        row: &crate::db::models::Content,
    ) -> PrivateServeVerdict {
        if !crate::private_reach::is_private(&row.reach) {
            return PrivateServeVerdict::Serve(ServeReason::NonPrivate);
        }
        let Some(standing) = self.custody_standing.as_ref() else {
            return PrivateServeVerdict::Withhold(WithholdReason::AuthorityUnavailable);
        };
        let facts = standing
            .facts_for(requester, &RowFacts::from_content(row))
            .await;
        private_serve_verdict(&facts)
    }

    /// Count + log one withhold. The `list_content` site is the only place a
    /// caller cannot see the refusal, which is exactly why it is counted.
    fn record_withhold(
        &self,
        site: &'static str,
        requester: &Requester,
        row_id: &str,
        reason: WithholdReason,
    ) {
        crate::metrics::inc_private_withheld(site, reason);
        info!(
            target: "elohim_storage::reach",
            site = site,
            requester = %requester.label(),
            row = %row_id,
            reason = reason.label(),
            "reach-withheld: a private row was not served to this peer"
        );
    }

    fn record_private_serve(
        &self,
        site: &'static str,
        requester: &Requester,
        row_id: &str,
        reason: ServeReason,
    ) {
        if reason == ServeReason::NonPrivate {
            return;
        }
        debug!(
            target: "elohim_storage::reach",
            site = site,
            requester = %requester.label(),
            row = %row_id,
            reason = reason.label(),
            "reach-granted: a private row was served to this peer"
        );
    }

    /// The blob-byte question, asked the way [`crate::blob_reach`] asks it:
    /// bytes are servable iff SOME referencing content row is servable. A
    /// successful `None` means an empty reference set or a non-private
    /// reference; `Some` retains the private grant reason and row id.
    ///
    /// Reuses `blob_reach::lookup_references` for the reach question and only
    /// pays for a second (bounded, indexed) read when EVERY referencing row is
    /// `private` — the case this station exists for.
    async fn blob_withhold_reason(
        &self,
        requester: &Requester,
        hash: &str,
    ) -> Result<Option<(ServeReason, String)>, WithholdReason> {
        // A peer composed without a content DB cannot hold private reference
        // rows. Preserve the pre-gate blob-serving behaviour for that explicit
        // configuration; fail closed only when configured authority breaks.
        let Some(pool) = self.db_pool.as_ref() else {
            return Ok(None);
        };
        self.custody_standing
            .as_ref()
            .ok_or(WithholdReason::AuthorityUnavailable)?;

        // One address form: the codec admitted the request's byte CID by full
        // decode, and every row records its payload by the same canonical text.
        let candidates = vec![hash.to_string()];

        let (refs, rows) = {
            let mut conn = pool.get().map_err(|error| {
                warn!(error = %error, hash = %hash, "reach authority pool checkout failed");
                WithholdReason::AuthorityUnavailable
            })?;
            let refs =
                crate::blob_reach::lookup_references(&mut conn, &candidates).map_err(|error| {
                    warn!(error = %error, hash = %hash, "reach reference query failed");
                    WithholdReason::AuthorityUnavailable
                })?;
            if refs.is_empty() {
                return Ok(None);
            }
            // Any non-private referencing row already makes these bytes
            // servable through it — refusing here would break a legitimate
            // read while protecting nothing.
            if refs
                .iter()
                .any(|r| !crate::private_reach::is_private(&r.reach))
            {
                return Ok(None);
            }
            let ids: Vec<String> = refs.iter().map(|r| r.content_id.clone()).collect();
            use crate::db::diesel_schema::content::dsl as c;
            use diesel::prelude::*;
            let rows: Vec<crate::db::models::Content> = c::content
                .filter(c::id.eq_any(&ids))
                .select(crate::db::models::Content::as_select())
                .limit(crate::blob_reach::MAX_REFERENCES)
                .load(&mut conn)
                .map_err(|error| {
                    warn!(error = %error, hash = %hash, "reach content-row load failed");
                    WithholdReason::AuthorityUnavailable
                })?;
            (refs, rows)
        };

        let referenced_ids: std::collections::HashSet<&str> = refs
            .iter()
            .map(|reference| reference.content_id.as_str())
            .collect();
        let loaded_ids: std::collections::HashSet<&str> =
            rows.iter().map(|row| row.id.as_str()).collect();
        if referenced_ids != loaded_ids {
            warn!(hash = %hash, references = refs.len(), rows = rows.len(), "reach content-row authority was incomplete");
            return Err(WithholdReason::AuthorityUnavailable);
        }

        // Every referencing row is private: serve iff ANY of them serves.
        let mut first: Option<WithholdReason> = None;
        for row in &rows {
            match self.row_verdict(requester, row).await {
                PrivateServeVerdict::Serve(reason) => {
                    return Ok(Some((reason, row.id.clone())));
                }
                PrivateServeVerdict::Withhold(reason) => {
                    first.get_or_insert(reason);
                }
            }
        }
        Err(first.unwrap_or(WithholdReason::AuthorityUnavailable))
    }

    async fn handle_get(&self, requester: &Requester, cid: ByteCid) -> ShardResponse {
        let hash = cid.to_string();
        debug!(cid = %hash, "Handling shard Get request");
        // Station 3b: the bytes of a blob referenced ONLY by private rows leave
        // this peer only toward the ward or a standing custodian. A refusal is
        // never rendered as NotFound (C4) — a stranger learns "you may not have
        // this", not "this does not exist".
        match self.blob_withhold_reason(requester, &hash).await {
            Ok(Some((reason, row_id))) => self.record_private_serve(
                crate::metrics::PRIVATE_WITHHOLD_SITE_GET_BLOB,
                requester,
                &row_id,
                reason,
            ),
            Ok(None) => {}
            Err(reason) => {
                self.record_withhold(
                    crate::metrics::PRIVATE_WITHHOLD_SITE_GET_BLOB,
                    requester,
                    &hash,
                    reason,
                );
                return ShardResponse::Error(format!("reach-withheld: {reason}"));
            }
        }
        // Size BEFORE bytes. A payload larger than one frame can never be served
        // under this name, so reading and hashing it only buys a refusal. An
        // oversize whole-bytes ask is exactly the question `GetManifest` answers.
        if let Some(len) = self.declared_len(&cid).await {
            if !crate::p2p::shard_protocol::payload_fits_frame(len) {
                let pivot = self.handle_get_manifest(cid);
                info!(
                    cid = %hash,
                    size = len,
                    max = crate::p2p::shard_protocol::SHARD_PAYLOAD_MAX_BYTES,
                    pivoted = !matches!(pivot, ShardResponse::NotFound),
                    "whole bytes exceed one shard frame — answering with the manifest pivot \
                     instead of reading them"
                );
                return pivot;
            }
        }

        // The store verifies the whole payload against its CID before returning a
        // byte (C5); a failure is quarantined, never served.
        match self.blob_store.get(&cid).await {
            Ok(data) => {
                info!(cid = %hash, size = data.len(), "Serving shard");
                ShardResponse::Data(data.to_vec())
            }
            Err(_) => {
                // An unexpired compute attachment answers from its own store, which
                // verifies it against the CID and its lease (C10: the payload store
                // never falls back to it, so every serve site asks it explicitly).
                match crate::compute_payload_store::get(self.blob_store.root_dir(), &hash).await {
                    Ok(bytes) => {
                        info!(cid = %hash, size = bytes.len(), "Serving compute payload chunk");
                        ShardResponse::Data(bytes)
                    }
                    Err(_) => {
                        debug!(cid = %hash, "Shard not found");
                        ShardResponse::NotFound
                    }
                }
            }
        }
    }

    /// The payload size for `cid` without materialising the payload: the store's
    /// own length, else the durable manifest's `byte_len`. `None` means this node
    /// cannot say, and the frame budget is enforced at serialization.
    async fn declared_len(&self, cid: &ByteCid) -> Option<u64> {
        if let Ok(len) = self.blob_store.size(cid).await {
            return Some(len);
        }
        let pool = self.db_pool.as_ref()?;
        let mut conn = pool.get().ok()?;
        crate::db::shard_manifests::resolve_manifest_by_cid(&mut conn, cid).map(|m| m.byte_len)
    }

    /// The composite pivot: a peer that holds a blob only as RS shards (its
    /// whole-bytes `Get` misses) answers with the durable manifest so the
    /// requester can shard-fetch.
    fn handle_get_manifest(&self, cid: ByteCid) -> ShardResponse {
        let Some(pool) = self.db_pool.as_ref() else {
            return ShardResponse::NotFound;
        };
        let Ok(mut conn) = pool.get() else {
            return ShardResponse::NotFound;
        };
        match crate::db::shard_manifests::resolve_manifest_by_cid(&mut conn, &cid) {
            Some(manifest) => {
                info!(cid = %cid, units = manifest.custody_units().len(), "Serving shard manifest");
                ShardResponse::Manifest(Box::new(manifest))
            }
            None => ShardResponse::NotFound,
        }
    }

    async fn handle_have(&self, cid: ByteCid) -> ShardResponse {
        debug!(cid = %cid, "Handling shard Have request");
        // An advertising probe: structural presence, no payload hashing and no
        // quarantine side effect (a requester verifies what it fetches).
        ShardResponse::Have(self.blob_store.present(&cid).await)
    }

    async fn handle_push(
        &self,
        requester: &Requester,
        cid: ByteCid,
        data: Vec<u8>,
        h_app_id: Option<String>,
    ) -> ShardResponse {
        let hash = cid.to_string();
        debug!(cid = %hash, size = data.len(), "Handling shard Push request");
        // The bytes are checked against the CID they were pushed under BEFORE
        // they are stored.
        let actual = compute_bytes_cid(&data);
        if actual != cid {
            warn!(expected = %hash, actual = %actual, "Shard CID mismatch; nothing stored");
            return ShardResponse::Error("CID mismatch".to_string());
        }
        // Under the arrival gate from the record to the store, so the
        // retention pass cannot let these bytes go in between.
        let _arriving = crate::db::blob_arrivals::arrival_gate().read().await;
        // Who placed it, recorded before the bytes: these are another peer's
        // bytes to withdraw, not ours to let go. A record that cannot be
        // written refuses the push, because bytes stored with no record could
        // never be told from ones nobody placed.
        if let Err(e) = self.record_placement(requester, &hash).await {
            error!(hash = %hash, error = %e, "Push refused: placement could not be recorded");
            return ShardResponse::Error(format!("Storage error: {}", e));
        }
        match self.blob_store.store(&data).await {
            Ok(_) => {
                info!(hash = %hash, "Shard stored via P2P push");
                // The holder claims its own custody; the placer cannot (C1).
                if let (Some(h_app_id), Some(sink)) = (h_app_id, self.held_shard_sink.get()) {
                    sink.held(cid, h_app_id);
                }
                ShardResponse::PushAck
            }
            Err(e) => {
                error!(hash = %hash, error = %e, "Failed to store shard");
                ShardResponse::Error(format!("Storage error: {}", e))
            }
        }
    }

    /// Record that `requester` placed `hash` here. The placer is its agent
    /// when the custody resolver knows one, otherwise its transport label: an
    /// unresolved placer still placed it. With no content database there is
    /// nowhere to record and nothing reads records, so the push proceeds.
    async fn record_placement(
        &self,
        requester: &Requester,
        hash: &str,
    ) -> Result<(), StorageError> {
        let Some(pool) = self.db_pool.clone() else {
            return Ok(());
        };
        let agent = match self.custody_standing.as_ref() {
            Some(standing) => standing.resolve_agent(requester).await,
            None => None,
        };
        // This node handing bytes to its own shard service is its own act.
        let own_act = requester.label() == "local";
        let placed_by = agent.unwrap_or_else(|| requester.label());
        let hash = hash.to_string();
        tokio::task::spawn_blocking(move || {
            use crate::db::blob_arrivals::{record_arrival, ArrivalVia};
            let mut conn = pool
                .get()
                .map_err(|e| StorageError::Database(format!("placement record: {e}")))?;
            if own_act {
                record_arrival(&mut conn, &hash, ArrivalVia::SelfPut, None, None)
            } else {
                record_arrival(&mut conn, &hash, ArrivalVia::Placed, Some(&placed_by), None)
            }
        })
        .await
        .map_err(|e| StorageError::Internal(format!("placement record task: {e}")))?
    }

    async fn handle_list_content(
        &self,
        requester: &Requester,
        reach_filter: Option<String>,
        offset: u32,
        limit: u32,
    ) -> ShardResponse {
        // Validate reach_filter against schema-generated constants so
        // unknown strings don't silently return empty results.
        if let Some(ref r) = reach_filter {
            if !crate::generated_enums::CORE_REACH_LEVELS.contains(&r.as_str()) {
                return ShardResponse::Error(format!(
                    "Unknown reach level {:?}. Valid values: {:?}",
                    r,
                    crate::generated_enums::CORE_REACH_LEVELS
                ));
            }
        }
        // ConvergenceAtom::InventoryServe — the local read cost of answering ONE
        // peer's ListContent page. Started before the pool checkout deliberately:
        // waiting for a connection IS the cost when the read pool is saturated,
        // and a timer starting after checkout would report a fast query while the
        // peer waited seconds to be served. Measured 2026-08-20 on matthew:
        // "Database read connection is saturated. Util 1387.50%" — the wait was
        // the whole story and no timer existed to say so.
        let serve_started = std::time::Instant::now();
        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => return ShardResponse::Error("No database pool".to_string()),
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
        };
        let app_ctx = crate::db::AppContext::default_lamad();
        let query = crate::db::content_diesel::ContentQuery {
            reach: reach_filter,
            limit: limit as i64,
            offset: offset as i64,
            ..Default::default()
        };
        // P2P shard inventory — internal peer-to-peer protocol, not
        // web2 HTTP. Peers must see all local rows so replication can
        // cover pre-drain content.
        // Read the page, then DROP the connection before the gate awaits — the
        // custody resolver may make one bounded conductor call, and a pooled
        // diesel connection must never be held across an await point.
        let loaded = crate::db::content_diesel::list_content(
            &mut conn,
            &app_ctx,
            &query,
            crate::db::content_diesel::MinTrust::Invisible,
        )
        .map(|items| {
            let total = crate::db::content_diesel::count_content(
                &mut conn,
                &app_ctx,
                &query,
                crate::db::content_diesel::MinTrust::Invisible,
            )
            .unwrap_or(items.len() as i64) as u64;
            (items, total)
        });
        drop(conn);

        match loaded {
            Ok((items, total)) => {
                // Station 3b: a withheld row is OMITTED from the page and
                // COUNTED. Offsets keep walking the unfiltered set, so `total`
                // may over-report for a gated requester — declared, and cheaper
                // than a second gated count on every page.
                let mut inventory: Vec<shard_protocol::ContentInventoryItem> =
                    Vec::with_capacity(items.len());
                let mut withheld = 0usize;
                for cwt in &items {
                    match self.row_verdict(requester, &cwt.content).await {
                        PrivateServeVerdict::Withhold(reason) => {
                            withheld += 1;
                            self.record_withhold(
                                crate::metrics::PRIVATE_WITHHOLD_SITE_LIST_CONTENT,
                                requester,
                                &cwt.content.id,
                                reason,
                            );
                            continue;
                        }
                        PrivateServeVerdict::Serve(reason) => self.record_private_serve(
                            crate::metrics::PRIVATE_WITHHOLD_SITE_LIST_CONTENT,
                            requester,
                            &cwt.content.id,
                            reason,
                        ),
                    }
                    inventory.push(shard_protocol::ContentInventoryItem {
                        id: cwt.content.id.clone(),
                        title: cwt.content.title.clone(),
                        content_type: cwt.content.content_type.clone(),
                        content_format: cwt.content.content_format.clone(),
                        reach: cwt.content.reach.clone(),
                        blob_cid: cwt.content.blob_cid.clone(),
                        updated_at: cwt.content.updated_at.clone(),
                    });
                }
                let has_more = (offset as u64 + items.len() as u64) < total;
                // Covers BOTH queries — list_content AND count_content. The pair
                // is the real cost: count_content is a full count over the whole
                // corpus (4495 rows on alpha) run on EVERY page, so a 5-page walk
                // pays five full counts. Timing only the list would have hidden
                // half of it.
                crate::metrics::observe_atom_duration(
                    crate::metrics::ConvergenceAtom::InventoryServe,
                    serve_started.elapsed(),
                );
                info!(
                    count = inventory.len(),
                    withheld = withheld,
                    total = total,
                    requester = %requester.label(),
                    elapsed_ms = serve_started.elapsed().as_secs_f64() * 1_000.0,
                    "Serving content inventory"
                );
                ShardResponse::ContentList {
                    items: inventory,
                    total,
                    has_more,
                }
            }
            Err(e) => {
                // Errors are recorded too: a query that FAILS after waiting on a
                // saturated pool still consumed the wait, and omitting it would
                // make the distribution improve exactly as the pool degrades
                // (coordinated omission).
                crate::metrics::observe_atom_duration(
                    crate::metrics::ConvergenceAtom::InventoryServe,
                    serve_started.elapsed(),
                );
                ShardResponse::Error(format!("Content query failed: {}", e))
            }
        }
    }

    async fn handle_get_content(&self, requester: &Requester, id: String) -> ShardResponse {
        let pool = match self.db_pool.as_ref() {
            Some(p) => p,
            None => return ShardResponse::Error("No database pool".to_string()),
        };
        let mut conn = match pool.get() {
            Ok(c) => c,
            Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
        };
        let app_ctx = crate::db::AppContext::default_lamad();
        let loaded = crate::db::content_diesel::get_content_with_tags(
            &mut conn,
            &app_ctx,
            &id,
            crate::db::content_diesel::MinTrust::Invisible,
        );
        drop(conn);
        match loaded {
            Ok(Some(cwt)) => {
                // A refusal is not an absence (C4): never ContentNotFound here.
                match self.row_verdict(requester, &cwt.content).await {
                    PrivateServeVerdict::Withhold(reason) => {
                        self.record_withhold(
                            crate::metrics::PRIVATE_WITHHOLD_SITE_GET_CONTENT,
                            requester,
                            &id,
                            reason,
                        );
                        return ShardResponse::Error(format!("reach-withheld: {reason}"));
                    }
                    PrivateServeVerdict::Serve(reason) => self.record_private_serve(
                        crate::metrics::PRIVATE_WITHHOLD_SITE_GET_CONTENT,
                        requester,
                        &id,
                        reason,
                    ),
                }
                debug!(id = %id, "Serving content record to peer");
                ShardResponse::Content(Box::new(shard_protocol::ContentRecord {
                    id: cwt.content.id,
                    title: cwt.content.title,
                    description: cwt.content.description,
                    content_type: cwt.content.content_type,
                    content_format: cwt.content.content_format,
                    blob_cid: cwt.content.blob_cid,
                    content_size_bytes: cwt.content.content_size_bytes,
                    metadata_json: cwt.content.metadata_json,
                    reach: cwt.content.reach,
                    created_by: cwt.content.created_by,
                    tags: cwt.tags,
                    content_body: cwt.content.content_body,
                }))
            }
            Ok(None) => ShardResponse::ContentNotFound,
            Err(e) => ShardResponse::Error(format!("Content fetch failed: {}", e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::blob_store::{BlobStore, Exists};
    use elohim_epr::address::{compute_bytes_cid, ByteCid};
    use tempfile::tempdir;

    async fn fresh_service() -> ShardService {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        ShardService::new(blob_store, None)
    }

    /// `Have` is an advertising probe: it answers from the store's structure and never
    /// hashes (or quarantines) the payload — a requester verifies what it fetches. A
    /// same-length corruption a hash would catch is left exactly where it was.
    #[tokio::test]
    async fn have_does_not_hash_the_payload() {
        fn find(root: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
            for entry in std::fs::read_dir(root).ok()?.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(found) = find(&path, name) {
                        return Some(found);
                    }
                } else if entry.file_name().to_string_lossy() == name {
                    return Some(path);
                }
            }
            None
        }
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let stored = blob_store.store(b"advertised, not hashed").await.unwrap();
        let path = find(&dir.path().join("blobs"), &stored.cid.to_string()).unwrap();
        let mut bytes = std::fs::read(&path).unwrap();
        bytes[0] ^= 0x01;
        std::fs::write(&path, &bytes).unwrap();
        let svc = ShardService::new(blob_store.clone(), None);
        let have = svc
            .handle(&Requester::local(), ShardRequest::Have { cid: stored.cid })
            .await;
        assert!(matches!(have, ShardResponse::Have(true)), "{have:?}");
        assert_eq!(
            std::fs::read(&path).unwrap(),
            bytes,
            "no quarantine, no rewrite"
        );
        // The verified answer still refuses it.
        assert_eq!(blob_store.exists(&stored.cid).await, Exists::Quarantined);
        let absent = svc
            .handle(&Requester::local(), ShardRequest::Have { cid: stored.cid })
            .await;
        assert!(matches!(absent, ShardResponse::Have(false)), "{absent:?}");
    }

    /// A holder serves a payload under its byte CID, the one spelling the wire
    /// carries (`ShardRequest::Get` decodes it in full).
    #[tokio::test]
    async fn get_by_byte_cid_serves_the_stored_bytes() {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let stored = blob_store.store(b"release artifact bytes").await.unwrap();
        assert_eq!(stored.cid, compute_bytes_cid(b"release artifact bytes"));
        let svc = ShardService::new(blob_store, None);
        match svc
            .handle(&Requester::local(), ShardRequest::Get { cid: stored.cid })
            .await
        {
            ShardResponse::Data(bytes) => assert_eq!(bytes, b"release artifact bytes"),
            other => panic!("the byte CID must serve the stored bytes, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_blob_on_a_peer_without_a_content_db_serves_as_before() {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let stored = blob_store.store(b"db-less peer blob").await.unwrap();
        let svc = ShardService::new(blob_store, None);
        match svc
            .handle(&Requester::local(), ShardRequest::Get { cid: stored.cid })
            .await
        {
            ShardResponse::Data(bytes) => assert_eq!(bytes, b"db-less peer blob"),
            other => panic!("a peer without a content DB must serve as before, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_blob_without_resolver_fails_closed_as_authority_unavailable() {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let stored = blob_store
            .store(b"bytes need a reach authority")
            .await
            .unwrap();
        let service = ShardService::new(blob_store, Some(gate_test_pool()));
        match service
            .handle(&Requester::local(), ShardRequest::Get { cid: stored.cid })
            .await
        {
            ShardResponse::Error(message) => {
                assert_eq!(message, "reach-withheld: authority-unavailable")
            }
            other => panic!("missing resolver must fail closed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn have_unknown_returns_have_false() {
        let svc = fresh_service().await;
        match svc
            .handle(
                &Requester::local(),
                ShardRequest::Have {
                    cid: compute_bytes_cid(b"missing"),
                },
            )
            .await
        {
            ShardResponse::Have(false) => {}
            other => panic!("expected Have(false), got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_content_with_unknown_reach_filter_returns_error() {
        let svc = fresh_service().await;
        match svc
            .handle(
                &Requester::local(),
                ShardRequest::ListContent {
                    reach_filter: Some("super-secret-tier".into()),
                    offset: 0,
                    limit: 10,
                },
            )
            .await
        {
            ShardResponse::Error(msg) => assert!(msg.contains("Unknown reach level")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn list_content_without_db_pool_returns_error() {
        let svc = fresh_service().await;
        match svc
            .handle(
                &Requester::local(),
                ShardRequest::ListContent {
                    reach_filter: None,
                    offset: 0,
                    limit: 10,
                },
            )
            .await
        {
            ShardResponse::Error(msg) => assert!(msg.contains("No database pool")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_content_without_db_pool_returns_error() {
        let svc = fresh_service().await;
        match svc
            .handle(
                &Requester::local(),
                ShardRequest::GetContent {
                    id: "anything".into(),
                },
            )
            .await
        {
            ShardResponse::Error(msg) => assert!(msg.contains("No database pool")),
            other => panic!("expected Error, got {other:?}"),
        }
    }

    // ────────────────────────────────────────────────────────────────
    // Station 3b — the custody gate on the three serve sites
    // ────────────────────────────────────────────────────────────────

    use crate::services::custody_standing::FakeCustodyStanding;

    const WARD: &str = "uhCAkWardAgentKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const CUSTODIAN: &str = "uhCAkCustodianKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
    const STRANGER: &str = "uhCAkStrangerKeyAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";

    fn gate_test_pool() -> crate::db::DbPool {
        use diesel::r2d2::{ConnectionManager, Pool};
        let url = format!(
            "file:shard_gate_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<diesel::SqliteConnection>::new(&url))
            .expect("pool");
        crate::db::run_migrations(&pool).expect("migrations");
        pool
    }

    fn seed_row(pool: &crate::db::DbPool, id: &str, reach: &str, blob_cid: Option<&str>) {
        let mut conn = pool.get().expect("conn");
        crate::db::content_diesel::create_content(
            &mut conn,
            &crate::db::AppContext::default_lamad(),
            crate::db::content_diesel::CreateContentInput {
                id: id.to_string(),
                title: id.to_string(),
                description: None,
                content_type: "issue-report".to_string(),
                content_format: "json".to_string(),
                blob_cid: blob_cid.map(str::to_string),
                content_size_bytes: None,
                metadata_json: None,
                reach: reach.to_string(),
                created_by: Some(WARD.to_string()),
                tags: Vec::new(),
                content_body: None,
                dht_anchor_hash: None,
            },
        )
        .expect("seed content row");
    }

    /// A service holding one `private` witness row (bytes present) and one
    /// `public` row, gated by a fake resolver the test drives.
    async fn gated_service(fake: Arc<FakeCustodyStanding>) -> (ShardService, ByteCid) {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        Box::leak(Box::new(dir));
        let bytes = b"death witness bytes".to_vec();
        let stored = blob_store.store(&bytes).await.unwrap();
        let pool = gate_test_pool();
        seed_row(
            &pool,
            "witness-private",
            "private",
            Some(&stored.cid.to_string()),
        );
        seed_row(&pool, "notice-public", "public", None);
        let svc = ShardService::new(blob_store, Some(pool)).with_custody_standing(fake);
        (svc, stored.cid)
    }

    #[tokio::test]
    async fn a_private_row_is_omitted_from_a_strangers_list_page() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let stranger = Requester::libp2p("12D3KooWStranger");
        fake.bind(&stranger, STRANGER)
            .ward_of_content("witness-private", WARD);
        let (svc, _hash) = gated_service(fake).await;

        match svc
            .handle(
                &stranger,
                ShardRequest::ListContent {
                    reach_filter: None,
                    offset: 0,
                    limit: 50,
                },
            )
            .await
        {
            ShardResponse::ContentList { items, .. } => {
                let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
                assert!(
                    !ids.contains(&"witness-private"),
                    "a stranger must not see the private witness: {ids:?}"
                );
                assert!(
                    ids.contains(&"notice-public"),
                    "the public row must still list — only `private` moves: {ids:?}"
                );
            }
            other => panic!("expected ContentList, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn a_private_row_is_present_in_a_custodians_list_page() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let custodian = Requester::libp2p("12D3KooWCustodian");
        fake.bind(&custodian, CUSTODIAN)
            .ward_of_content("witness-private", WARD)
            .spool_custody(CUSTODIAN, WARD);
        let (svc, _hash) = gated_service(fake).await;

        match svc
            .handle(
                &custodian,
                ShardRequest::ListContent {
                    reach_filter: None,
                    offset: 0,
                    limit: 50,
                },
            )
            .await
        {
            ShardResponse::ContentList { items, .. } => {
                let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
                assert!(
                    ids.contains(&"witness-private"),
                    "a standing custodian must be served the witness: {ids:?}"
                );
            }
            other => panic!("expected ContentList, got {other:?}"),
        }
    }

    /// C4 — a refusal is not an absence. This must never be `ContentNotFound`.
    #[tokio::test]
    async fn get_content_refuses_a_stranger_with_reach_withheld() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let stranger = Requester::libp2p("12D3KooWStranger");
        fake.bind(&stranger, STRANGER)
            .ward_of_content("witness-private", WARD);
        let (svc, _hash) = gated_service(fake).await;

        match svc
            .handle(
                &stranger,
                ShardRequest::GetContent {
                    id: "witness-private".into(),
                },
            )
            .await
        {
            ShardResponse::Error(msg) => {
                assert!(
                    msg.starts_with("reach-withheld: "),
                    "typed refusal expected, got {msg}"
                );
                assert!(
                    msg.contains("no-standing"),
                    "reason must ride the error: {msg}"
                );
            }
            other => panic!("a refusal must never be rendered as absence, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_content_serves_the_ward() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let ward = Requester::libp2p("12D3KooWWard");
        fake.bind(&ward, WARD)
            .ward_of_content("witness-private", WARD);
        let (svc, _hash) = gated_service(fake).await;

        match svc
            .handle(
                &ward,
                ShardRequest::GetContent {
                    id: "witness-private".into(),
                },
            )
            .await
        {
            ShardResponse::Content(record) => assert_eq!(record.id, "witness-private"),
            other => panic!("the ward's own row must come home, got {other:?}"),
        }
    }

    /// The blob leg: bytes referenced ONLY by a private row are refused, and
    /// again never as NotFound.
    #[tokio::test]
    async fn get_blob_refuses_when_every_referencing_row_is_private() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let stranger = Requester::libp2p("12D3KooWStranger");
        fake.bind(&stranger, STRANGER)
            .ward_of_content("witness-private", WARD);
        let (svc, hash) = gated_service(fake).await;
        let digest = hash.to_string();
        // The fake keys the ward by digest as well, mirroring production.
        match svc.handle(&stranger, ShardRequest::Get { cid: hash }).await {
            ShardResponse::Error(msg) => {
                assert!(msg.starts_with("reach-withheld: "), "got {msg}");
            }
            other => panic!("private bytes must be refused, got {other:?} (digest {digest})"),
        }
    }

    #[tokio::test]
    async fn get_blob_serves_the_ward_the_same_bytes() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let ward = Requester::libp2p("12D3KooWWard");
        fake.bind(&ward, WARD)
            .ward_of_content("witness-private", WARD);
        let (svc, hash) = gated_service(fake).await;
        match svc.handle(&ward, ShardRequest::Get { cid: hash }).await {
            ShardResponse::Data(bytes) => assert_eq!(bytes, b"death witness bytes".to_vec()),
            other => panic!("the ward must still get its bytes, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_blob_serves_a_digest_custodian_when_the_ward_is_unresolved() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let custodian = Requester::libp2p("12D3KooWCustodian");
        let (svc, hash) = gated_service(fake.clone()).await;
        let digest = hash.to_string();
        fake.bind(&custodian, CUSTODIAN)
            .blob_custody(CUSTODIAN, &digest);
        match svc
            .handle(&custodian, ShardRequest::Get { cid: hash })
            .await
        {
            ShardResponse::Data(bytes) => assert_eq!(bytes, b"death witness bytes".to_vec()),
            other => panic!("exact digest custody must stand without a ward, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_blob_reference_query_failure_fails_closed() {
        use diesel::r2d2::{ConnectionManager, Pool};
        let url = format!(
            "file:shard_gate_unmigrated_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<diesel::SqliteConnection>::new(&url))
            .unwrap();
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let stored = blob_store
            .store(b"bytes behind unreadable authority")
            .await
            .unwrap();
        let service = ShardService::new(blob_store, Some(pool))
            .with_custody_standing(Arc::new(FakeCustodyStanding::new()));
        match service
            .handle(&Requester::local(), ShardRequest::Get { cid: stored.cid })
            .await
        {
            ShardResponse::Error(message) => {
                assert_eq!(message, "reach-withheld: authority-unavailable")
            }
            other => panic!("query failure must fail closed, got {other:?}"),
        }
    }

    /// The declared scope, at the service level: an unreferenced blob and a
    /// public row's blob serve to anyone, exactly as before the gate.
    #[tokio::test]
    async fn an_unreferenced_blob_still_serves_to_a_stranger() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let stranger = Requester::libp2p("12D3KooWStranger");
        fake.bind(&stranger, STRANGER);
        let (svc, _hash) = gated_service(fake).await;
        let orphan = svc.blob_store.store(b"no row claims these").await.unwrap();
        match svc
            .handle(&stranger, ShardRequest::Get { cid: orphan.cid })
            .await
        {
            ShardResponse::Data(bytes) => assert_eq!(bytes, b"no row claims these".to_vec()),
            other => panic!("honest absence: an unclaimed blob serves, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn every_non_private_reach_is_unchanged_at_all_three_serve_sites() {
        for reach in [
            "public",
            "commons",
            "community",
            "familiar",
            "trusted",
            "intimate",
            "Private",
            "unknown-future-tier",
        ] {
            let dir = tempdir().unwrap();
            let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
            let stored = blob_store.store(reach.as_bytes()).await.unwrap();
            let pool = gate_test_pool();
            let id = format!("row-{reach}");
            seed_row(&pool, &id, reach, Some(&stored.cid.to_string()));
            let fake = Arc::new(FakeCustodyStanding::new());
            let stranger = Requester::libp2p("12D3KooWStranger");
            let service = ShardService::new(blob_store, Some(pool)).with_custody_standing(fake);

            match service
                .handle(
                    &stranger,
                    ShardRequest::ListContent {
                        reach_filter: None,
                        offset: 0,
                        limit: 50,
                    },
                )
                .await
            {
                ShardResponse::ContentList { items, .. } => {
                    assert!(
                        items.iter().any(|item| item.id == id),
                        "ListContent moved {reach}"
                    )
                }
                other => panic!("ListContent failed for {reach}: {other:?}"),
            }
            assert!(matches!(
                service
                    .handle(&stranger, ShardRequest::GetContent { id: id.clone() })
                    .await,
                ShardResponse::Content(_)
            ));
            assert!(matches!(
                service
                    .handle(&stranger, ShardRequest::Get { cid: stored.cid })
                    .await,
                ShardResponse::Data(_)
            ));
        }
    }

    fn arrivals(svc: &ShardService, hash: &str) -> Vec<crate::db::blob_arrivals::Arrival> {
        let mut conn = svc.db_pool.as_ref().unwrap().get().unwrap();
        crate::db::blob_arrivals::arrivals_by_cid(&mut conn)
            .unwrap()
            .remove(hash)
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn a_pushed_shard_records_who_placed_it() {
        use crate::db::blob_arrivals::ArrivalVia;
        let fake = Arc::new(FakeCustodyStanding::new());
        let placer = Requester::libp2p("12D3KooWPlacer");
        fake.bind(&placer, CUSTODIAN);
        let stranger = Requester::iroh("node-with-no-known-agent");
        let (svc, _) = gated_service(fake).await;

        let data = b"a shard another peer places".to_vec();
        let hash = compute_bytes_cid(&data);
        for requester in [&placer, &stranger] {
            let res = svc
                .handle(
                    requester,
                    ShardRequest::Push {
                        cid: hash,
                        data: data.clone(),
                        h_app_id: None,
                    },
                )
                .await;
            assert!(matches!(res, ShardResponse::PushAck), "{res:?}");
        }

        let mut placed: Vec<String> = arrivals(&svc, &hash.to_string())
            .into_iter()
            .inspect(|a| assert_eq!(a.via, ArrivalVia::Placed))
            .map(|a| a.placed_by)
            .collect();
        placed.sort();
        // The resolved agent where one is known; the transport label where not.
        assert_eq!(
            placed,
            vec![
                "iroh:node-with-no-known-agent".to_string(),
                CUSTODIAN.to_string()
            ]
        );
    }

    #[tokio::test]
    async fn a_push_whose_bytes_do_not_match_stores_nothing() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let (svc, _) = gated_service(fake).await;
        let data = b"not what the name says".to_vec();
        let claimed = compute_bytes_cid(b"something else");
        let res = svc
            .handle(
                &Requester::libp2p("12D3KooWPlacer"),
                ShardRequest::Push {
                    cid: claimed,
                    data: data.clone(),
                    h_app_id: None,
                },
            )
            .await;
        assert!(matches!(res, ShardResponse::Error(_)), "{res:?}");
        assert_ne!(svc.blob_store.exists(&claimed).await, Exists::Verified);
        assert_ne!(
            svc.blob_store.exists(&compute_bytes_cid(&data)).await,
            Exists::Verified,
            "the mismatched bytes must not be left on disk under their real hash"
        );
        assert!(arrivals(&svc, &claimed.to_string()).is_empty());
    }

    #[tokio::test]
    async fn bytes_this_node_hands_its_own_shard_service_are_its_own_act() {
        use crate::db::blob_arrivals::ArrivalVia;
        let fake = Arc::new(FakeCustodyStanding::new());
        let (svc, _) = gated_service(fake).await;
        let data = b"handed over in process".to_vec();
        let hash = compute_bytes_cid(&data);
        let res = svc
            .handle(
                &Requester::local(),
                ShardRequest::Push {
                    cid: hash,
                    data,
                    h_app_id: None,
                },
            )
            .await;
        assert!(matches!(res, ShardResponse::PushAck), "{res:?}");
        let recorded = arrivals(&svc, &hash.to_string());
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0].via, ArrivalVia::SelfPut);
    }

    struct CapturingSink(std::sync::Mutex<Vec<(ByteCid, String)>>);

    impl HeldShardSink for CapturingSink {
        fn held(&self, cid: ByteCid, h_app_id: String) {
            self.0.lock().unwrap().push((cid, h_app_id));
        }
    }

    /// The holder learns the scope of a placement it stored, so it can author its own
    /// custody claim; a scope-less push and a push whose bytes do not match report
    /// nothing.
    #[tokio::test]
    async fn a_scoped_push_reports_the_stored_shard_to_the_holder() {
        let dir = tempdir().unwrap();
        let blob_store = Arc::new(BlobStore::new(dir.path().to_path_buf()).await.unwrap());
        let svc = ShardService::new(blob_store, None);
        let sink = Arc::new(CapturingSink(std::sync::Mutex::new(Vec::new())));
        assert!(svc.set_held_shard_sink(sink.clone()));
        let data = b"a placed shard".to_vec();
        let cid = compute_bytes_cid(&data);
        let push = |h_app_id: Option<&str>, cid: ByteCid| ShardRequest::Push {
            cid,
            data: data.clone(),
            h_app_id: h_app_id.map(str::to_string),
        };
        let placer = Requester::libp2p("12D3KooWPlacer");
        let res = svc.handle(&placer, push(None, cid)).await;
        assert!(matches!(res, ShardResponse::PushAck), "{res:?}");
        let res = svc
            .handle(&placer, push(Some("lamad"), compute_bytes_cid(b"other")))
            .await;
        assert!(matches!(res, ShardResponse::Error(_)), "{res:?}");
        assert!(sink.0.lock().unwrap().is_empty());

        let res = svc.handle(&placer, push(Some("lamad"), cid)).await;
        assert!(matches!(res, ShardResponse::PushAck), "{res:?}");
        assert_eq!(*sink.0.lock().unwrap(), vec![(cid, "lamad".to_string())]);
    }

    #[tokio::test]
    async fn push_then_get_round_trips() {
        let fake = Arc::new(FakeCustodyStanding::new());
        let (svc, _) = gated_service(fake).await;
        let data = b"hello shard".to_vec();
        // Compute the hash by storing first to learn the hash, then re-push
        // with that hash to exercise the Push path.
        let stored = svc.blob_store.store(&data).await.unwrap();
        let res = svc
            .handle(
                &Requester::local(),
                ShardRequest::Push {
                    cid: stored.cid,
                    data: data.clone(),
                    h_app_id: None,
                },
            )
            .await;
        match res {
            ShardResponse::PushAck => {}
            other => panic!("expected PushAck, got {other:?}"),
        }

        match svc
            .handle(&Requester::local(), ShardRequest::Get { cid: stored.cid })
            .await
        {
            ShardResponse::Data(bytes) => assert_eq!(bytes, data),
            other => panic!("expected Data, got {other:?}"),
        }
    }
}
