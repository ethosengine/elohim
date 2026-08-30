//! Content service - business logic for content operations
//!
//! Wraps the content repository with validation, event emission,
//! and cross-entity orchestration.

use std::sync::Arc;

use crate::db::content_diesel::ContentProjectionPatch;
use crate::db::{self, content_diesel, context::AppContext, DbPool};
use crate::error::StorageError;
use crate::generated_enums::{ALL_CONTENT_FORMATS, ALL_CONTENT_TYPES, ALL_REACH_LEVELS};
use crate::hc_client::HcClient;
use crate::services::conductor_writes;

use super::events::{EventBus, StorageEvent};

/// Content service for business logic
pub struct ContentService {
    pool: DbPool,
    ctx: AppContext,
    events: Arc<EventBus>,
}

impl ContentService {
    /// Create a new content service
    pub fn new(pool: DbPool, ctx: AppContext, events: Arc<EventBus>) -> Self {
        Self { pool, ctx, events }
    }

    /// Get a connection from the pool
    fn conn(
        &self,
    ) -> Result<
        diesel::r2d2::PooledConnection<diesel::r2d2::ConnectionManager<diesel::SqliteConnection>>,
        StorageError,
    > {
        self.pool
            .get()
            .map_err(|e| StorageError::Internal(format!("Pool error: {}", e)))
    }

    // =========================================================================
    // Read Operations
    // =========================================================================

    /// Get content by ID — INTERNAL use only.
    ///
    /// Passes `require_provenance: false` so internal callers (update merge,
    /// relationship existence check, delete_cascade) can see pre-drain rows.
    /// The external HTTP boundary (`handle_db_content_by_id`) does NOT route
    /// through this method — it calls `content_diesel::get_content_with_tags`
    /// directly with `require_provenance: true`.
    pub fn get(&self, id: &str) -> Result<Option<crate::db::models::Content>, StorageError> {
        let mut conn = self.conn()?;
        content_diesel::get_content(
            &mut conn,
            &self.ctx,
            id,
            content_diesel::MinTrust::Invisible,
        )
    }

    /// List content with filters — INTERNAL use only.
    ///
    /// Passes `require_provenance: false`. The external HTTP boundary
    /// (`handle_db_content_list`) does NOT route through this method.
    #[allow(dead_code)]
    pub fn list(
        &self,
        query: &content_diesel::ContentQuery,
    ) -> Result<Vec<crate::db::models::ContentWithTags>, StorageError> {
        let mut conn = self.conn()?;
        content_diesel::list_content(
            &mut conn,
            &self.ctx,
            query,
            content_diesel::MinTrust::Invisible,
        )
    }

    /// Get content by tag — INTERNAL use only.
    #[allow(dead_code)]
    pub fn get_by_tag(
        &self,
        tag: &str,
        limit: u32,
    ) -> Result<Vec<crate::db::models::ContentWithTags>, StorageError> {
        let mut conn = self.conn()?;
        content_diesel::get_content_by_tag(&mut conn, &self.ctx, tag, limit as i64)
    }

    /// Search content by text — INTERNAL use only.
    #[allow(dead_code)]
    pub fn search(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<crate::db::models::ContentWithTags>, StorageError> {
        self.list(&content_diesel::ContentQuery {
            search: Some(query.to_string()),
            limit: limit as i64,
            ..Default::default()
        })
    }

    // =========================================================================
    // Write Operations
    // =========================================================================

    /// Create a single content item with validation
    pub fn create(
        &self,
        input: content_diesel::CreateContentInput,
    ) -> Result<crate::db::models::ContentWithTags, StorageError> {
        // Validate required fields
        self.validate_content(&input)?;

        // Create content
        let mut conn = self.conn()?;
        let result = content_diesel::create_content(&mut conn, &self.ctx, input)?;

        // Emit event
        self.events.emit(StorageEvent::ContentCreated {
            id: result.content.id.clone(),
            title: result.content.title.clone(),
            content_type: Some(result.content.content_type.clone()),
        });

        Ok(result)
    }

    /// Bulk create content items (for seeding)
    pub fn bulk_create(
        &self,
        items: Vec<content_diesel::CreateContentInput>,
    ) -> Result<content_diesel::BulkResult, StorageError> {
        // Validate all items first
        for (i, item) in items.iter().enumerate() {
            if let Err(e) = self.validate_content(item) {
                return Err(StorageError::InvalidInput(format!("item[{}]: {}", i, e)));
            }
        }

        let ids: Vec<String> = items.iter().map(|i| i.id.clone()).collect();

        // Perform bulk create
        let mut conn = self.conn()?;
        let result = content_diesel::bulk_create_content(&mut conn, &self.ctx, items)?;

        // Emit event if any items were inserted
        if result.inserted > 0 {
            self.events.emit(StorageEvent::ContentBulkCreated {
                count: result.inserted as usize,
                ids,
            });
        }

        Ok(result)
    }

    /// Partially update a content item (PATCH semantics) — metadata-only /
    /// non-notarized fields.
    ///
    /// The `view.metadata` field is shallow-merged with existing metadata:
    /// only the keys present in the patch object are overwritten.
    ///
    /// Notarized fields (blob_hash, reach) are NOT written here — they are
    /// authored through the conductor by [`Self::update_via_conductor`] so the
    /// Holochain DHT witnesses a single head (in context with the network) that
    /// gossips to every peer. There is no diesel-direct "amber" write: a
    /// per-peer, un-witnessed head could never derive green (green == DHT
    /// witnessed / anchored) and would diverge across backends. The amber *tier*
    /// survives as a derived read signal; cross-peer amber convergence is healed
    /// by the DocStore reverse-projection
    /// (`sync::projector::reverse_project_content_doc`), which writes
    /// `crdt_converged_at` at the db layer directly — not through this method.
    pub fn update(
        &self,
        id: &str,
        view: crate::views::UpdateContentInputView,
    ) -> Result<crate::db::models::ContentWithTags, StorageError> {
        let mut conn = self.conn()?;

        // Compute merged metadata_json before entering the DB layer
        let merged_metadata_json =
            if let Some(patch_meta) = &view.metadata {
                let existing = content_diesel::get_content(
                    &mut conn,
                    &self.ctx,
                    id,
                    content_diesel::MinTrust::Invisible,
                )?
                .ok_or_else(|| StorageError::NotFound(format!("Content not found: {}", id)))?;

                let existing_meta: serde_json::Value = existing
                    .metadata_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

                let patch_value = patch_meta.0.clone();
                let merged = match (existing_meta, patch_value) {
                    (serde_json::Value::Object(mut base), serde_json::Value::Object(patch)) => {
                        for (k, v) in patch {
                            base.insert(k, v);
                        }
                        serde_json::Value::Object(base)
                    }
                    (_, patch) => patch, // fallback: replace entirely
                };

                Some(serde_json::to_string(&merged).map_err(|e| {
                    StorageError::Internal(format!("Metadata serialize error: {}", e))
                })?)
            } else {
                None
            };

        let input = content_diesel::UpdateContentInput {
            id: id.to_string(),
            title: view.title,
            description: view.description,
            content_body: view.content_body,
            content_format: view.content_format,
            metadata_json: merged_metadata_json,
            tags: view.tags,
            reach: view.reach,
            blob_hash: view.blob_hash,
            server_blob_hash: view.server_blob_hash,
            p2p_published_at: view.p2p_published_at,
            // The HTTP PATCH path never stamps the amber tier — that is the
            // DocStore reverse-projection's job (drift-heal). Always None here.
            crdt_converged_at: None,
        };

        let result = content_diesel::update_content(&mut conn, &self.ctx, input)?;

        self.events.emit(StorageEvent::ContentUpdated {
            id: result.content.id.clone(),
        });

        Ok(result)
    }

    /// Substrate-correct PATCH path: round-trip the patch through the local
    /// conductor's content_store zome so DHT gossip propagates the entry to
    /// every alpha peer.
    ///
    /// Per substrate-rea-replication-fix Task 8c, this branches on whether
    /// the row already has a DHT anchor:
    ///
    /// 1. **Lazy-migration bootstrap** (`dht_anchor_hash IS NULL`). The row
    ///    was created via `bulk_create_content` during seeding and never
    ///    published to the DHT. First PATCH must publish — we construct a
    ///    full `CreateContentInput` from the existing SQL row + patch
    ///    applied, call `create_content` on the zome, then wait for the
    ///    `ContentCommitted` signal to project the anchor + patch into SQL.
    ///
    /// 2. **Standard update** (`dht_anchor_hash IS NOT NULL`). The DHT
    ///    already has a prior entry. Send only the patched fields via
    ///    `update_content`, which fetches the prev entry, applies the patch
    ///    preserving absent fields, and writes the update via `update_entry`.
    ///
    /// Both paths converge on `ContentCommitted` → `upsert_with_anchor` →
    /// local SQL row with the new `dht_anchor_hash` (and patched fields).
    /// The bounded poll below sees the projection land in ~tens of ms via
    /// the in-process signal subscriber wired in Task 6.5.
    ///
    /// Field naming: HTTP wire / storage diesel use `blob_hash`, DNA entry
    /// uses `blob_cid` (Phase 0 refactor — same SHA256 SemEantically).
    /// Translation happens here.
    /// `election` decides whether the committed action also becomes this row's
    /// NOTARY-DECLARED head, and is required at every call site rather than
    /// defaulted — see [`content_diesel::HeadElection`]. Request-borne
    /// re-notarization declares; the heal-class re-author sweeps preserve.
    pub async fn update_via_conductor(
        &self,
        hc: &Arc<HcClient>,
        id: &str,
        view: crate::views::UpdateContentInputView,
        election: content_diesel::HeadElection,
    ) -> Result<crate::db::models::ContentWithTags, StorageError> {
        let existing = {
            let mut conn = self.conn()?;
            content_diesel::get_content_with_tags(
                &mut conn,
                &self.ctx,
                id,
                content_diesel::MinTrust::Invisible,
            )?
            .ok_or_else(|| StorageError::NotFound(format!("Content not found: {}", id)))?
        };

        // Refuse a reach change the substrate cannot carry, BEFORE any write —
        // a partial apply followed by a refusal would diverge SQL from the DHT.
        // See `reach_patch_refusal` for why the anchored path cannot honor it.
        if let Some(refusal) = reach_patch_refusal(
            id,
            existing.content.dht_anchor_hash.as_deref(),
            &existing.content.reach,
            view.reach.as_deref(),
        ) {
            return Err(refusal);
        }

        // Merge metadata (same logic as the legacy `update` method above).
        let merged_metadata_json =
            if let Some(patch_meta) = &view.metadata {
                let existing_meta: serde_json::Value = existing
                    .content
                    .metadata_json
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));
                let patch_value = patch_meta.0.clone();
                let merged = match (existing_meta, patch_value) {
                    (serde_json::Value::Object(mut base), serde_json::Value::Object(patch)) => {
                        for (k, v) in patch {
                            base.insert(k, v);
                        }
                        serde_json::Value::Object(base)
                    }
                    (_, patch) => patch,
                };
                Some(serde_json::to_string(&merged).map_err(|e| {
                    StorageError::Internal(format!("Metadata serialize error: {}", e))
                })?)
            } else {
                None
            };

        // The DNA target field is `blob_cid`. View carries `blob_hash`.
        let new_blob_cid = view.blob_hash.clone();

        // Built unconditionally: the NULL-anchor branch publishes it, and the
        // stale-anchor heal below re-publishes it when the conductor has lost
        // the entry the anchor points at (post-DHT-reset class, 2026-06-10).
        // Lazy-migration bootstrap: publish the full entry, applying patch
        // on top of the existing SQL row's data. Fields not patched fall
        // through to the existing row's values.
        let bootstrap = {
            lamad_types::CreateContentInput {
                id: existing.content.id.clone(),
                content_type: existing.content.content_type.clone(),
                title: view
                    .title
                    .clone()
                    .unwrap_or_else(|| existing.content.title.clone()),
                description: view
                    .description
                    .clone()
                    .unwrap_or_else(|| existing.content.description.clone().unwrap_or_default()),
                summary: None,
                content: view
                    .content_body
                    .clone()
                    .or_else(|| existing.content.content_body.clone())
                    .unwrap_or_default(),
                content_format: view
                    .content_format
                    .clone()
                    .unwrap_or_else(|| existing.content.content_format.clone()),
                tags: view.tags.clone().unwrap_or_else(|| existing.tags.clone()),
                source_path: None,
                related_node_ids: Vec::new(),
                reach: view
                    .reach
                    .clone()
                    .unwrap_or_else(|| existing.content.reach.clone()),
                estimated_minutes: None,
                thumbnail_url: None,
                metadata_json: merged_metadata_json
                    .clone()
                    .or_else(|| existing.content.metadata_json.clone())
                    .unwrap_or_else(|| "{}".to_string()),
                blob_cid: new_blob_cid
                    .clone()
                    .or_else(|| existing.content.blob_cid.clone())
                    .or_else(|| existing.content.blob_hash.clone()),
                content_size_bytes: existing.content.content_size_bytes.map(|n| n as u64),
                content_hash: existing.content.blob_hash.clone(),
            }
        };

        let output_bytes = if existing.content.dht_anchor_hash.is_none() {
            conductor_writes::call_create_content(hc, &bootstrap).await?
        } else {
            // Standard update: only patched fields cross the wire.
            // REACH (reach-floor Task 6, landed 2026-08-30): `reach` IS threaded through
            // this path now — the content_store zome's `update_content` accepts a reach
            // patch and re-notarizes the entry with it, so a reach change on a
            // live-anchored entry is a real substrate move (the projection then stamps
            // the committed grade). `reach_patch_refusal` (called above) only refuses
            // when the conductor bridge cannot carry the change; on this arm it can.
            let patch = lamad_types::UpdateContentInput {
                id: id.to_string(),
                blob_cid: new_blob_cid.clone(),
                // stageSpaBlobs sends blob_hash only; content_size_bytes/content_hash
                // come from the entry's existing fields. Phase 1 PATCH surface is
                // blob_cid-only — title/description/metadata go through the legacy
                // diesel `update` for now (those bypass the substrate; substrate
                // migration for them is a follow-up sweep).
                content_size_bytes: None,
                content_hash: None,
                title: view.title.clone(),
                description: view.description.clone(),
                metadata_json: merged_metadata_json.clone(),
                reach: view.reach.clone(),
            };
            match conductor_writes::call_update_content(hc, &patch).await {
                Ok(bytes) => bytes,
                // STALE-ANCHOR HEAL (2026-06-10): the SQL row carries a
                // dht_anchor_hash but the conductor has no entry behind it —
                // the anchor predates a conductor/DHT reset (RESET_STORAGE
                // wiped DHT state while the SQL projection persisted). The
                // zome's update_content errors "no Content entry found for
                // id"; re-publish the full entry from the SQL row + patch
                // (identical to the NULL-anchor bootstrap); the resulting
                // ContentCommitted projection overwrites the stale anchor.
                // Without this, every post-reset PATCH 503s forever and the
                // EPR-routed mounts (the landing a human visits) stay 404.
                Err(e) if e.to_string().contains("no Content entry found") => {
                    tracing::warn!(
                        id = %id,
                        "update_via_conductor: stale dht_anchor_hash (no DHT entry behind it) — healing via create_content re-publish"
                    );
                    conductor_writes::call_create_content(hc, &bootstrap).await?
                }
                Err(e) => return Err(e),
            }
        };

        // Eagerly project the SQL row from the zome output (Gap-F fix).
        //
        //    The conductor returned successfully and gave us the committed
        //    Content entry + ActionHash. We project synchronously here using
        //    the same upsert_with_anchor the signal handler calls, with the
        //    same field mapping (ContentEntry → ContentProjectionPatch).
        //    This is idempotent with the async signal: both derive the same
        //    patch fields from the same committed entry, so a later signal
        //    arrival produces the same SQL row.
        //
        //    action_hash string form: holo_hash ActionHash Display → "uhCkk…"
        //    base32 form, identical to what the signal carries.
        let output =
            rmp_serde::from_slice::<lamad_types::ContentOutput>(&output_bytes).map_err(|e| {
                StorageError::Internal(format!(
                    "conductor returned success for content write but output \
                     could not be decoded as ContentOutput: {e}"
                ))
            })?;
        let action_hash_str = format!("{}", output.action_hash);
        let oc = &output.content;
        let size_i32 = oc
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
        let patch = ContentProjectionPatch {
            blob_cid: oc.blob_cid.clone(),
            content_size_bytes: size_i32,
            title: Some(oc.title.clone()),
            description: Some(oc.description.clone()),
            content_type: Some(oc.content_type.clone()),
            content_format: Some(oc.content_format.clone()),
            reach: Some(oc.reach.clone()),
            metadata_json: Some(oc.metadata_json.clone()),
        };
        {
            let mut conn = self.conn()?;
            content_diesel::upsert_with_anchor(
                &mut conn,
                &self.ctx,
                id,
                patch,
                &action_hash_str,
                election,
            )?;
        }

        // `server_blob_hash` is a deploy-projection field, not part of the
        // notarized content entry — the conductor round-trip + ContentProjectionPatch
        // above do not carry it. If this PATCH also set serverBlobHash (e.g. a
        // combined {blobHash, serverBlobHash} body that routed here via
        // patch_needs_conductor), persist it diesel-direct so it isn't dropped.
        // No-clobber: only server_blob_hash is set; update_content preserves all
        // other fields (the just-projected anchor/blob_cid included).
        if view.server_blob_hash.is_some() {
            let mut conn = self.conn()?;
            let server_patch = content_diesel::UpdateContentInput {
                id: id.to_string(),
                server_blob_hash: view.server_blob_hash.clone(),
                ..Default::default()
            };
            content_diesel::update_content(&mut conn, &self.ctx, server_patch)?;
        }

        // Mirror the COMMITTED reach into SQL when the re-publish carried a
        // reach change the projection would otherwise drop. See
        // `reach_mirror_after_renotarize` for why the projection drops it and
        // why this stays on the request-borne path only.
        if let Some(committed_reach) = reach_mirror_after_renotarize(
            view.reach.as_deref(),
            &output.content.reach,
            &existing.content.reach,
        ) {
            let mut conn = self.conn()?;
            let reach_patch = content_diesel::UpdateContentInput {
                id: id.to_string(),
                reach: Some(committed_reach),
                ..Default::default()
            };
            content_diesel::update_content(&mut conn, &self.ctx, reach_patch)?;
        }

        let updated = {
            let mut conn = self.conn()?;
            content_diesel::get_content_with_tags(
                &mut conn,
                &self.ctx,
                id,
                content_diesel::MinTrust::Invisible,
            )?
            .ok_or_else(|| {
                StorageError::Internal(format!(
                    "content {id} projection written but row missing on re-read — \
                         this should not happen; check upsert_with_anchor for id"
                ))
            })?
        };

        self.events
            .emit(StorageEvent::ContentUpdated { id: id.to_string() });
        Ok(updated)
    }

    /// Recover and project the DHT anchor for content that is ALREADY committed
    /// to THIS conductor's DHT view but whose local SQL row is still
    /// NULL-anchor.
    ///
    /// The idempotent-success recovery for the reanchor backfill: when a
    /// re-author via `create_content` is refused with "… already exists. Use
    /// update_content …", the entry IS on the DHT — its anchored state is
    /// already reachable. Read the committed entry back (`get_content_by_id`,
    /// which recovers the `ActionHash` from the IdToContent link) and project it
    /// through the SAME `upsert_with_anchor` path a `ContentCommitted` signal
    /// uses, stamping the REAL `dht_anchor_hash`. The row then leaves the
    /// NULL-anchor candidate set permanently — ending the per-boot re-thrash
    /// that saturated adam's Cache-DB read pool.
    ///
    /// `Ok(true)` when the anchor was recovered and stamped. `Ok(false)` when
    /// the conductor unexpectedly has no entry for the id (the "already exists"
    /// claim could not be corroborated) — the caller keeps the row as a
    /// retryable failure, never fabricating an anchor.
    ///
    /// `election` is required for the same reason as on
    /// [`Self::update_via_conductor`]: recovering an anchor is a heal-class act,
    /// and its only caller (the re-anchor sweep) passes
    /// `PreserveExistingDeclaration` so a recovery can never crown a head.
    pub async fn project_existing_anchor(
        &self,
        hc: &Arc<HcClient>,
        id: &str,
        election: content_diesel::HeadElection,
    ) -> Result<bool, StorageError> {
        let Some(output) = conductor_writes::get_content_by_id(hc, id).await? else {
            return Ok(false);
        };

        // Mirror the projection the create/update path runs (upsert_with_anchor
        // with the committed ActionHash), so a recovered anchor is byte-identical
        // to one stamped by a fresh ContentCommitted signal. Field mapping is
        // identical to the eager-projection block in `update_via_conductor`.
        let action_hash_str = format!("{}", output.action_hash);
        let oc = &output.content;
        let size_i32 = oc
            .content_size_bytes
            .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
        let patch = ContentProjectionPatch {
            blob_cid: oc.blob_cid.clone(),
            content_size_bytes: size_i32,
            title: Some(oc.title.clone()),
            description: Some(oc.description.clone()),
            content_type: Some(oc.content_type.clone()),
            content_format: Some(oc.content_format.clone()),
            reach: Some(oc.reach.clone()),
            metadata_json: Some(oc.metadata_json.clone()),
        };
        {
            let mut conn = self.conn()?;
            content_diesel::upsert_with_anchor(
                &mut conn,
                &self.ctx,
                id,
                patch,
                &action_hash_str,
                election,
            )?;
        }
        Ok(true)
    }

    /// Delete content by ID
    pub fn delete(&self, id: &str) -> Result<bool, StorageError> {
        let mut conn = self.conn()?;
        let deleted = content_diesel::delete_content(&mut conn, &self.ctx, id)?;

        if deleted {
            self.events
                .emit(StorageEvent::ContentDeleted { id: id.to_string() });
        }

        Ok(deleted)
    }

    /// Delete content and cascade to relationships
    ///
    /// This is the preferred delete method as it maintains referential integrity.
    pub fn delete_cascade(&self, id: &str) -> Result<bool, StorageError> {
        // First check if content exists
        let exists = self.get(id)?.is_some();
        if !exists {
            return Ok(false);
        }

        let mut conn = self.conn()?;
        // Delete relationships where this content is source or target
        let _ =
            db::relationships_diesel::delete_relationships_for_content(&mut conn, &self.ctx, id);
        // Then delete content
        content_diesel::delete_content(&mut conn, &self.ctx, id)?;

        self.events
            .emit(StorageEvent::ContentDeleted { id: id.to_string() });

        Ok(true)
    }

    // =========================================================================
    // Validation
    // =========================================================================

    /// Validate content input
    fn validate_content(
        &self,
        input: &content_diesel::CreateContentInput,
    ) -> Result<(), StorageError> {
        if input.id.is_empty() {
            return Err(StorageError::InvalidInput("id is required".into()));
        }

        if input.id.len() > 255 {
            return Err(StorageError::InvalidInput(
                "id must be <= 255 characters".into(),
            ));
        }

        if input.title.is_empty() {
            return Err(StorageError::InvalidInput("title is required".into()));
        }

        if input.title.len() > 500 {
            return Err(StorageError::InvalidInput(
                "title must be <= 500 characters".into(),
            ));
        }

        // Validate content_type — from protocol schema (generated_enums), permissive
        if !ALL_CONTENT_TYPES.contains(&input.content_type.as_str())
            && !input.content_type.starts_with("custom:")
        {
            // Allow custom types with prefix
            // Just warn, don't reject - be permissive
        }

        // Validate content_format — from protocol schema (generated_enums) + storage-specific extensions
        const STORAGE_EXTRA_FORMATS: &[&str] =
            &["yaml", "toml", "latex", "asciidoc", "iframe", "embed"];
        if !ALL_CONTENT_FORMATS.contains(&input.content_format.as_str())
            && !STORAGE_EXTRA_FORMATS.contains(&input.content_format.as_str())
        {
            return Err(StorageError::InvalidInput(format!(
                "content_format '{}' is not valid. Valid formats: {:?} (+ storage: {:?})",
                input.content_format, ALL_CONTENT_FORMATS, STORAGE_EXTRA_FORMATS
            )));
        }

        // Validate reach level — from protocol schema (generated_enums) + legacy for backward compat
        // 'local' removed 2026-06-27: it is not a content-visibility reach
        // (it belongs to the relationship/distance vocabulary) and the DNA
        // rejects it; accepting it here only let invalid rows reach the
        // conductor. See genesis check-reach-drift + reanchor_backfill guard.
        const LEGACY_REACH_LEVELS: &[&str] = &["regional", "invited", "federated"];
        if !ALL_REACH_LEVELS.contains(&input.reach.as_str())
            && !LEGACY_REACH_LEVELS.contains(&input.reach.as_str())
        {
            return Err(StorageError::InvalidInput(format!(
                "reach '{}' is not valid. Valid values: {:?} (+ legacy: {:?})",
                input.reach, ALL_REACH_LEVELS, LEGACY_REACH_LEVELS
            )));
        }

        // Validate metadata_json is valid JSON if provided
        if let Some(ref json_str) = input.metadata_json {
            if !json_str.is_empty() {
                serde_json::from_str::<serde_json::Value>(json_str).map_err(|e| {
                    StorageError::InvalidInput(format!("metadata_json is not valid JSON: {}", e))
                })?;
            }
        }

        Ok(())
    }

    // =========================================================================
    // Stats
    // =========================================================================

    /// Get content count by type
    pub fn get_stats(&self) -> Result<ContentStats, StorageError> {
        let mut conn = self.conn()?;
        let total = content_diesel::content_count(&mut conn, &self.ctx)? as u64;

        // For by_type stats, use a simplified approach
        // The Diesel module doesn't have a group-by-type function,
        // so we return total count with empty by_type map
        Ok(ContentStats {
            total_count: total,
            by_type: std::collections::HashMap::new(),
        })
    }
}

/// The reach value a request-borne re-notarization must mirror into SQL, or
/// `None` when there is nothing to mirror.
///
/// `ContentProjectionPatch` CARRIES `reach`, but `apply_content_patch_fields`
/// writes it only on the defensive-INSERT branch — on the UPDATE branch (every
/// existing row) it is silently dropped. So a reach change that legitimately
/// reached the DHT through the bootstrap/stale-anchor re-publish left the local
/// row on its OLD grade: a projection DIVERGING from the entry it had just
/// notarized, which is worse than the silent no-op `reach_patch_refusal`
/// covers.
///
/// Mirror the COMMITTED reach (what the conductor returned), never the
/// requested one — SQL must say what the DHT says, not what the caller hoped.
///
/// Deliberately narrow to this request-borne path. Teaching the SHARED
/// signal-projection path to re-project reach for every anchored row would
/// re-grade the whole corpus off DHT entries in one deploy — a corpus-wide
/// product decision, not a defect fix (see
/// `genesis/data/timeline/backlog/2026-08-21-seed-doorway-unauthored-reach-default.md`).
fn reach_mirror_after_renotarize(
    requested_reach: Option<&str>,
    committed_reach: &str,
    row_reach: &str,
) -> Option<String> {
    requested_reach?;
    (committed_reach != row_reach).then(|| committed_reach.to_string())
}

/// Decide whether a reach-carrying PATCH must be REFUSED rather than answered
/// with a success it cannot honor.
///
/// `reach` is a DNA-notarized, DHT-witnessed field. Two write paths exist, and
/// only one of them can carry it:
///
/// - **NULL / stale `dht_anchor_hash`** → `update_via_conductor` re-publishes
///   the whole entry through `create_content`, which DOES carry `reach`. The
///   patch genuinely applies; nothing to refuse.
/// - **Live `dht_anchor_hash`** → the standard update path calls the
///   content_store zome's `update_content`, whose input struct has NO reach
///   field (`lamad_types::UpdateContentInput`). The conductor round-trip then
///   returns the entry with its ORIGINAL reach, and the projection re-stamps
///   that unchanged value. The write reported 200, bumped `updated_at`, and
///   changed nothing — the worse half of a silent no-op.
///
/// Making reach patchable on an anchored entry needs a zome change
/// (reach-floor-foundation-plan.md Task 6), which is a DNA-hash-moving
/// integrity concern and not this layer's to invent. Until it lands, the honest
/// answer is a refusal.
///
/// A re-send of the SAME reach is not a change and is never refused — the
/// deploy PATCH sends `{blobHash, reach}` together on every deploy and must
/// keep working.
fn reach_patch_refusal(
    id: &str,
    dht_anchor_hash: Option<&str>,
    current_reach: &str,
    requested_reach: Option<&str>,
) -> Option<StorageError> {
    let requested = requested_reach?;
    if requested == current_reach {
        return None;
    }
    // Reach-floor Task 6 (landed 2026-08-30): the content_store zome's
    // `update_content` now carries a `reach` field, so a reach change on a
    // DHT-anchored entry re-notarizes through the conductor — the silent-no-op
    // class this refusal existed for is closed at the substrate. Anchored and
    // unanchored rows both accept the patch; the conductor round-trip is the
    // gate (a refused/failed zome call still surfaces as an error, never a 200
    // that changed nothing).
    let _ = (id, dht_anchor_hash);
    None
}

/// Content statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct ContentStats {
    pub total_count: u64,
    pub by_type: std::collections::HashMap<String, i64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: Tests would require setting up a test database
    // For now, just test validation logic

    #[test]
    fn test_validate_empty_id() {
        let _events = Arc::new(EventBus::new());
        // Can't test without a database connection, but validation is straightforward
    }

    #[test]
    fn test_content_service_has_update_method() {
        fn _assert_update_exists(
            _s: &ContentService,
            _id: &str,
            _v: crate::views::UpdateContentInputView,
        ) {
            // If this compiles, the method exists
        }
    }

    // ---------------------------------------------------------------------
    // Reach-patch refusal (the silent-no-op fix)
    //
    // A reach-carrying PATCH on a DHT-ANCHORED row used to answer 200 while
    // changing nothing: the standard-update branch cannot carry `reach` (the
    // content_store zome's `update_content` has no reach field), and the
    // post-write projection then re-stamped the entry's UNCHANGED reach. The
    // guard is real and deliberate — so it must be loud, not silent.
    // ---------------------------------------------------------------------

    const ANCHOR: &str = "uhCkk87JiPmogR2U93nUTJLa9kolkN5GhYwqSKen9lMZLm3JdxGwY";

    #[test]
    fn reach_change_on_anchored_row_is_now_carried_not_refused() {
        // Reach-floor Task 6 (2026-08-30): the zome's `update_content` carries
        // `reach`, so the anchored-row refusal is retired — the patch proceeds
        // to the conductor, which re-notarizes with the new reach. The refusal
        // fn stays as the seam (a future substrate that cannot carry a change
        // refuses HERE, before any write), but today it must pass this through.
        assert!(reach_patch_refusal(
            "community-garden-club",
            Some(ANCHOR),
            "public",
            Some("community"),
        )
        .is_none());
    }

    #[test]
    fn idempotent_reach_resend_is_not_refused() {
        // The deploy PATCH sends {blobHash, reach} together and re-sends the
        // SAME reach on every deploy. Refusing that would 400 every deploy.
        assert!(reach_patch_refusal("c1", Some(ANCHOR), "public", Some("public")).is_none());
    }

    #[test]
    fn unanchored_row_accepts_a_reach_change() {
        // A NULL anchor takes the bootstrap branch, which re-publishes the whole
        // entry via `create_content` — that call DOES carry reach, so the patch
        // genuinely applies and must not be refused.
        assert!(reach_patch_refusal("c1", None, "public", Some("community")).is_none());
    }

    #[test]
    fn reach_mirror_writes_the_committed_grade_when_the_republish_carried_one() {
        // Unanchored row: the bootstrap re-publish DID carry reach to the DHT,
        // but the projection drops it on the UPDATE branch — so the mirror must
        // write the COMMITTED value, not the requested one and not nothing.
        assert_eq!(
            reach_mirror_after_renotarize(Some("community"), "community", "public"),
            Some("community".to_string())
        );
    }

    #[test]
    fn reach_mirror_is_silent_when_nothing_asked_or_nothing_moved() {
        // No reach in the patch → never touch the column.
        assert_eq!(
            reach_mirror_after_renotarize(None, "public", "public"),
            None
        );
        // The conductor committed the grade the row already had → no write.
        assert_eq!(
            reach_mirror_after_renotarize(Some("public"), "public", "public"),
            None
        );
    }

    #[test]
    fn reach_mirror_trusts_the_conductor_over_the_request() {
        // The caller asked for `intimate`; the committed entry says `community`.
        // SQL must say what the DHT says.
        assert_eq!(
            reach_mirror_after_renotarize(Some("intimate"), "community", "public"),
            Some("community".to_string())
        );
    }

    #[test]
    fn patch_without_reach_is_never_refused() {
        assert!(reach_patch_refusal("c1", Some(ANCHOR), "public", None).is_none());
    }
}
