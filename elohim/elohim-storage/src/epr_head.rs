//! EPR Head derivation from local DB projection.
//!
//! This module contains [`derive_epr_head`], the single canonical path for
//! constructing an [`EprHead`] from the local SQLite read projection.
//!
//! ## Why this helper exists
//!
//! Three production call sites previously constructed [`EprHead`] inline:
//!
//! 1. `http.rs` — `handle_get_epr_head` (GET /epr-head/{id})
//! 2. `p2p/mod.rs` — `resolve_epr_head_locally` (P2P request-response handler)
//!
//! Each site duplicated the field-mapping from `content` rows → struct slots,
//! creating drift risk. This helper centralises that mapping into one code
//! path so any future field addition (e.g. `relationships` from link queries)
//! is done exactly once.
//!
//! ## `gate_provenance` parameter
//!
//! When `true`, only content rows that have been notarised on Holochain DHT
//! **or** published to libp2p Kad are visible — identical to the filter in
//! `db::content_diesel::get_content_with_tags` with `require_provenance=true`.
//!
//! External HTTP handlers pass `true` (never surface pre-publish rows to
//! callers outside the node).  Internal P2P drain-loop paths pass `false` so
//! unpublished rows remain accessible for replication.
//!
//! ## `enrich_pillars` parameter
//!
//! When `true`, the helper issues additional queries to populate:
//!
//! - `shefa.stewards` / `shefa.allocations` — from `stewardship_allocations`
//!
//! The HTTP handler leaves shefa empty (no enrichment needed for the public
//! metadata surface). The P2P handler enriches it so peers receive full
//! stewardship context alongside the content envelope.
//!
//! `qahal.attestation_requirements` is populated downstream (not here) from
//! `PREREQUISITE` content-graph edges by the enrich=true caller, which holds
//! the graph engine; this helper stays diesel-only.
//!
//! ## Future work — projector-derived contexts (Phase 4)
//!
//! The EPR Phase 2B plan (`genesis/docs/superpowers/plans/2026-04-24-epr-phase-2b-plan.md`
//! lines 1009–1051) describes using the projector controller's pillar mapping
//! to derive `EprLamadContext` / `EprShefaContext` / `EprQahalContext` from
//! manifest-graph resolvers. That is aspirational for **Phase 4**, where the
//! projector handles typed structured mappings (beyond the current flat
//! string→string column projections). This helper is the centralization
//! precondition for that future work — once Phase 4 lands, the enrichment
//! branches here will be replaced by a projector dispatch call.

use diesel::prelude::*;

use crate::db;
use crate::epr_codec::{EprHead, EprLamadContext, EprQahalContext, EprShefaContext};
use crate::error::StorageError;
use crate::views_convert::epr::EprHeadView;

/// Derive an [`EprHead`] from the local SQLite read projection for the given
/// content `id`.
///
/// Returns `Ok(None)` when the content row does not exist (or is filtered out
/// by `gate_provenance`).  Returns `Err` only on DB access failures.
///
/// # Parameters
///
/// - `conn` — mutable reference to an open SQLite connection
/// - `app_ctx` — multi-tenant app scope (use [`db::AppContext::default_lamad`]
///   for learning content)
/// - `id` — stable content identifier (slug)
/// - `gate_provenance` — when `true`, exclude rows that have neither a
///   `dht_anchor_hash` nor a `p2p_published_at` timestamp
/// - `enrich_pillars` — when `true`, issue extra queries to populate
///   `shefa` stewardship data and `qahal` attestation requirements
pub fn derive_epr_head(
    conn: &mut SqliteConnection,
    app_ctx: &db::AppContext,
    id: &str,
    gate_provenance: bool,
    enrich_pillars: bool,
) -> Result<Option<EprHead>, StorageError> {
    // Map the passthrough `gate_provenance` bool onto the tri-state trust gate
    // with the fixed migration mapping (`true → Amber`, `false → Invisible`).
    // `derive_epr_head`'s public bool signature is preserved (A2 scope).
    let min_trust = if gate_provenance {
        db::content_diesel::MinTrust::Amber
    } else {
        db::content_diesel::MinTrust::Invisible
    };
    let content_opt = db::content_diesel::get_content_with_tags(conn, app_ctx, id, min_trust)?;

    let content_with_tags = match content_opt {
        Some(cwt) => cwt,
        None => return Ok(None),
    };

    let content = &content_with_tags.content;

    // ── Lamad — knowledge pillar ────────────────────────────────────────────
    let lamad = EprLamadContext {
        title: content.title.clone(),
        content_type: content.content_type.clone(),
        description: content.description.clone(),
        content_format: Some(content.content_format.clone()),
        tags: content_with_tags.tags.clone(),
    };

    // ── Shefa — value pillar ────────────────────────────────────────────────
    let shefa = if enrich_pillars {
        match db::stewardship_allocations::get_allocations_for_content(conn, app_ctx, &content.id) {
            Ok(allocations) if !allocations.is_empty() => EprShefaContext {
                stewards: allocations
                    .iter()
                    .map(|a| a.steward_presence_id.clone())
                    .collect(),
                allocations: allocations
                    .iter()
                    .map(|a| a.allocation_ratio as f64)
                    .collect(),
            },
            _ => EprShefaContext {
                stewards: vec![],
                allocations: vec![],
            },
        }
    } else {
        EprShefaContext {
            stewards: vec![],
            allocations: vec![],
        }
    };

    // ── Qahal — governance pillar ───────────────────────────────────────────
    //
    // `attestation_requirements` is left empty here. The prerequisite
    // requirement now lives as `PREREQUISITE` content-graph edges, not the
    // (dropped) content-attestations table. The single enrich=true caller
    // (`EprService::resolve_epr_head_locally`) holds the graph engine and fills
    // this field from `query_direct_prerequisites` after derivation, preserving
    // the legacy wire shape `"prerequisite-mastery:{prereq_id}"`. This helper
    // stays diesel-only (no graph dependency) for the HTTP enrich=false path.
    let qahal = EprQahalContext {
        reach: Some(content.reach.clone()),
        layer: None,
        attestation_requirements: vec![],
    };

    Ok(Some(EprHead {
        version: 1,
        id: content.id.clone(),
        content: content.blob_cid.clone().unwrap_or_default(),
        lamad,
        shefa,
        qahal,
        relationships: vec![],
        author: content.created_by.clone(),
        // `updated` is `declared_head_at` — the DHT declaration act's own
        // Timestamp — NEVER `content.updated_at`, the local row mtime bumped
        // by every stamp including no-ops (the live incident recorded at
        // `content_diesel.rs`'s `StampOutcome::Refreshed` doc comment:
        // `updated_at` advanced 05:33:54 → 05:49:00 on an unchanged head).
        // A head whose address includes the mtime names a peer's moment, not
        // the head. NULL declared_head_at omits the field entirely (C4
        // honest absence) rather than substituting any local time.
        // See epr-head-envelope-design.md §4–5 (Option A).
        updated: content.declared_head_at.and_then(render_declared_head_at),
    }))
}

/// Render `declared_head_at` (microseconds since the Unix epoch — the zome
/// `Timestamp` of the declaration act behind `declared_head_action_hash`) as
/// RFC3339 UTC, seconds precision. Mirrors the rendering
/// `Envelope::canonical_bytes` already uses for `issuedAt`
/// (`elohim/epr/src/envelope.rs`).
///
/// Returns `None` only if the microsecond value is outside chrono's
/// representable range — defensive; no real DHT-carried Timestamp should
/// ever hit this, and a `None` here composes into the same honest-absence
/// path as a NULL `declared_head_at`.
fn render_declared_head_at(declared_head_at_micros: i64) -> Option<String> {
    chrono::DateTime::<chrono::Utc>::from_timestamp_micros(declared_head_at_micros)
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Secs, true))
}

/// Derive an [`EprHead`], encode it, and stamp the resulting `cid` onto the
/// JSON response wrapper [`EprHeadView`] — the single pure (DB-only, no
/// `Request`) path for composing the HTTP JSON body. Never enriches pillars
/// (matches the HTTP handler's historical `enrich_pillars=false`) and never
/// touches `distribution` — that is purely operational (Category C) and is
/// served separately at `GET /api/v1/blob/{hash}/distribution/summary`
/// (epr-head-envelope-design.md, Option A).
///
/// Returns `Ok(None)` when the content row does not exist (or fails the
/// provenance gate), mirroring [`derive_epr_head`]. Encoding failure (should
/// not occur for any row that derived successfully) collapses to an absent
/// `cid` rather than an error — the canonical bytes are still correct, only
/// the convenience field is missing.
pub fn compose_head_view(
    conn: &mut SqliteConnection,
    app_ctx: &db::AppContext,
    id: &str,
    gate_provenance: bool,
) -> Result<Option<EprHeadView>, StorageError> {
    let head = match derive_epr_head(conn, app_ctx, id, gate_provenance, false)? {
        Some(h) => h,
        None => return Ok(None),
    };

    let mut view: EprHeadView = head.clone().into();
    if let Ok((_bytes, cid)) = crate::epr_codec::encode_epr_head(&head) {
        view.cid = Some(cid.to_string());
    }

    Ok(Some(view))
}
