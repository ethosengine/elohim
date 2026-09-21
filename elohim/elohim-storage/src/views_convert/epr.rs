//! EPR-domain Wire→View converters.
//!
//! Converts EPR Head input/output between wire JSON shapes (camelCase) and the
//! internal `crate::epr_codec` types. These are NOT DB-model projections — they
//! are protocol-codec conversions that live at the HTTP boundary.

use serde::{Deserialize, Serialize};

use crate::epr_codec::{
    EprHead, EprLamadContext, EprQahalContext, EprRelationship, EprShefaContext,
};

// ============================================================================
// EPR Head Input Views (TypeScript → EprHead)
// ============================================================================

/// EPR Head input — accepts camelCase JSON from TypeScript clients.
/// Converts to `EprHead` for DAG-CBOR encoding.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadInputView {
    pub version: Option<u32>,
    pub id: String,
    pub content: String,
    pub lamad: EprLamadContextInputView,
    pub shefa: Option<EprShefaContextInputView>,
    pub qahal: Option<EprQahalContextInputView>,
    #[serde(default)]
    pub relationships: Vec<EprRelationshipInputView>,
    pub author: Option<String>,
    pub updated: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprLamadContextInputView {
    pub title: String,
    pub content_type: String,
    pub description: Option<String>,
    pub content_format: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprShefaContextInputView {
    #[serde(default)]
    pub stewards: Vec<String>,
    #[serde(default)]
    pub allocations: Vec<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprQahalContextInputView {
    pub reach: Option<String>,
    pub layer: Option<String>,
    #[serde(default)]
    pub attestation_requirements: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EprRelationshipInputView {
    #[serde(rename = "type")]
    pub rel_type: String,
    pub target: String,
    pub target_cid: Option<String>,
}

impl From<EprHeadInputView> for EprHead {
    fn from(v: EprHeadInputView) -> Self {
        Self {
            version: v.version.unwrap_or(1),
            id: v.id,
            content: v.content,
            lamad: EprLamadContext {
                title: v.lamad.title,
                content_type: v.lamad.content_type,
                description: v.lamad.description,
                content_format: v.lamad.content_format,
                tags: v.lamad.tags,
            },
            shefa: v.shefa.map_or_else(
                || EprShefaContext {
                    stewards: vec![],
                    allocations: vec![],
                },
                |s| EprShefaContext {
                    stewards: s.stewards,
                    allocations: s.allocations,
                },
            ),
            qahal: v.qahal.map_or_else(
                || EprQahalContext {
                    reach: None,
                    layer: None,
                    attestation_requirements: vec![],
                },
                |q| EprQahalContext {
                    reach: q.reach,
                    layer: q.layer,
                    attestation_requirements: q.attestation_requirements,
                },
            ),
            relationships: v
                .relationships
                .into_iter()
                .map(|r| EprRelationship {
                    rel_type: r.rel_type,
                    target: r.target,
                    target_cid: r.target_cid,
                })
                .collect(),
            author: v.author,
            updated: v.updated,
        }
    }
}

// ============================================================================
// EPR Head Output View (EprHead → TypeScript)
// ============================================================================

/// EPR Head response — camelCase output for TypeScript clients.
///
/// **The body is a function of the declared head alone.** This wrapper adds
/// exactly one field beyond the canonical [`EprHead`] (in `epr_codec`): `cid`,
/// the address of that head's dag-cbor bytes. It carries no operational
/// (Category C) fields — no per-peer distribution/replica facts, no local row
/// mtime. Two peers that agree on the declared head therefore serve
/// byte-identical bodies and mint identical `cid`s, regardless of local state
/// (row mtime, gossip-observed inventory). Per-peer distribution facts
/// (replica counts, projector counts, diversity, caller role) live at the
/// sibling route `GET /api/v1/blob/{hash}/distribution/summary`
/// (`api/blob.rs`), which genuinely — and correctly — differs between honest
/// peers; a view whose whole point is to be the same everywhere must never
/// carry a fact two honest peers can honestly disagree on.
/// See `genesis/a2o/reports/recovery/serving-edge-20260920/epr-head-envelope-design.md`
/// (Option A) for the full design and the census that found zero in-tree
/// consumers of the removed `distribution` field.
///
/// The DAG-CBOR encoding path in `handle_get_epr_head` serializes the
/// canonical [`EprHead`] struct directly (no `cid`, structurally no
/// `distribution`); this JSON view is the only place `cid` is surfaced.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EprHeadView {
    pub version: u32,
    pub id: String,
    pub content: String,
    pub lamad: EprLamadContext,
    pub shefa: EprShefaContext,
    pub qahal: EprQahalContext,
    pub relationships: Vec<EprRelationship>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,
    /// CID of the DAG-CBOR encoded head (set after encoding)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cid: Option<String>,
}

impl From<EprHead> for EprHeadView {
    fn from(h: EprHead) -> Self {
        Self {
            version: h.version,
            id: h.id,
            content: h.content,
            lamad: h.lamad,
            shefa: h.shefa,
            qahal: h.qahal,
            relationships: h.relationships,
            author: h.author,
            updated: h.updated,
            cid: None,
        }
    }
}
