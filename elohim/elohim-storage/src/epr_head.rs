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
//! - `qahal.attestation_requirements` — from `content_attestations`
//!
//! The HTTP handler leaves both empty (no enrichment needed for the public
//! metadata surface). The P2P handler enriches them so peers receive full
//! governance context alongside the content envelope.
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
    let content_opt =
        db::content_diesel::get_content_with_tags(conn, app_ctx, id, gate_provenance)?;

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
    let qahal = if enrich_pillars {
        let mut attestation_requirements = Vec::new();
        if let Ok(atts) =
            db::content_attestations::query_attestations_for_content(conn, &content.id)
        {
            for att in &atts {
                if att.is_revoked == 0 {
                    let req = if let Some(ref evidence) = att.evidence {
                        format!("{}:{}", att.attestation_type, evidence)
                    } else {
                        att.attestation_type.clone()
                    };
                    attestation_requirements.push(req);
                }
            }
        }
        EprQahalContext {
            reach: Some(content.reach.clone()),
            layer: None,
            attestation_requirements,
        }
    } else {
        EprQahalContext {
            reach: Some(content.reach.clone()),
            layer: None,
            attestation_requirements: vec![],
        }
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
        updated: Some(content.updated_at.clone()),
    }))
}
