//! Attention-tending API controller — M-REA-2.
//!
//! Routes: `POST /api/v1/attention/tending`
//!
//! Accepts an `AttentionTendingIntentView` (the wire shape from
//! `elohim/sdk/schemas/v1/intents/attention-tending-intent.schema.json`) and
//! proxies to the existing `create_attention_tending` coordinator on the
//! `content_store` zome. The client sends intent — the substrate writes the
//! private source-chain entry.
//!
//! ## Design classification (P2P Design Gate — §1.5)
//!
//! - **Entity:** AttentionTending — Category B (Agent-Scoped, private source chain).
//!   `Visibility::Private` — entries never gossiped to DHT. The coordinator
//!   already exists at `content_store/src/attention_tending.rs`.
//! - **No new DHT entry type needed.** No new projection table needed.
//! - **dwell_qualification_ms:** the proxy route evaluates `elapsed_ms` from
//!   the intent against the tending-policy Manifest value (default 3000).
//!   If `elapsed_ms < dwell_qualification_ms`, the route returns 200 without
//!   calling the coordinator (not a qualified tending event). This moves the
//!   client-side `DWELL_THRESHOLD_MS = 3000` constant (F-POLICY-4) to the
//!   substrate.
//! - **signer_pubkey:** intentionally absent from the intent — the coordinator
//!   derives it from `agent_info()?.agent_initial_pubkey`. Mirrors the T8 I1
//!   fix on FeedbackSignal; prevents caller-side spoofing.
//!
//! ## Substrate re-tending / deduplication
//!
//! The coordinator's `refresh_tending_ttl` path appends a new timestamp to
//! an existing `AttentionTending` entry's `tended_at` vec rather than creating
//! a duplicate entry. The route always calls `create_attention_tending` — the
//! substrate handles deduplication per the docstring at attention_tending.rs:30-34.
//! The client's `sessionViewed` Set is no longer needed.
//!
//! ## Route placement
//!
//! Lives in `elohim-storage/src/api/` (not `doorway`) for the same reason as
//! `api/lamad.rs` — the dwell-qualification logic IS domain logic; doorway is
//! a thin web2 bridge with no domain logic. Both browser (via doorway proxy at
//! `/api/v1/attention/tending`) and Tauri (direct to :8090) receive identical
//! semantics.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tracing;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::hc_client_registry::HcClientRegistry;
use crate::services::response;

use super::parse_body;

// ---------------------------------------------------------------------------
// Wire input — HTTP boundary
// ---------------------------------------------------------------------------

/// Incoming intent from the client.
///
/// Aligns with `intents/attention-tending-intent.schema.json`.
/// `signer_pubkey` is intentionally absent — coordinator derives it
/// from `agent_info()?.agent_initial_pubkey`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionTendingIntentView {
    /// JSON-encoded FilterSubject. Coordinator gates on parse failure.
    pub filter_subject_json: String,
    /// One of: values-forward / fatigue / scope-mismatch / safety.
    pub classification: String,
    /// Optional human-readable reason.
    pub reason: Option<String>,
    /// TTL in seconds; minimum 3600 enforced by integrity zome floor.
    pub ttl_seconds: u64,
    /// JSON-encoded ContextScope. Coordinator gates on parse failure.
    pub context_json: String,
    /// Elapsed milliseconds — compared against tending-policy Manifest's
    /// dwell_qualification_ms before forwarding to the coordinator.
    /// Replaces the client-side DWELL_THRESHOLD_MS = 3000 constant.
    pub elapsed_ms: u64,
}

// ---------------------------------------------------------------------------
// Zome wire type mirror — CreateAttentionTendingInput
// ---------------------------------------------------------------------------
//
// Mirrors `CreateAttentionTendingInput` in content_store/src/attention_tending.rs.
// Field names MUST match exactly (MessagePack-serialized). Wire-internal only —
// does NOT cross the HTTP boundary (use AttentionTendingIntentView for that).

#[derive(Debug, Clone, Serialize)]
struct ZomeCreateAttentionTendingInput {
    pub filter_subject_json: String,
    pub classification: String,
    pub reason: Option<String>,
    pub ttl_seconds: u64,
    pub context_json: String,
}

impl From<&AttentionTendingIntentView> for ZomeCreateAttentionTendingInput {
    fn from(v: &AttentionTendingIntentView) -> Self {
        Self {
            filter_subject_json: v.filter_subject_json.clone(),
            classification: v.classification.clone(),
            reason: v.reason.clone(),
            ttl_seconds: v.ttl_seconds,
            context_json: v.context_json.clone(),
        }
    }
}

// ---------------------------------------------------------------------------
// Response — thin acknowledgement (private entry: no DHT hash to return)
// ---------------------------------------------------------------------------

/// Response from POST /api/v1/attention/tending.
///
/// `actionHash` is the private source-chain action hash returned by the
/// coordinator. The entry is never gossiped; the hash is opaque to peers.
/// Clients may use it for `refresh_tending_ttl` calls. When the route
/// short-circuits (elapsed_ms < dwell_qualification_ms), `accepted` is false
/// and `actionHash` is None.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AttentionTendingAckView {
    /// Whether the tending was forwarded to the coordinator.
    pub accepted: bool,
    /// Hex-encoded action hash when accepted via coordinator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action_hash: Option<String>,
    /// Reason when not accepted (e.g. "below_dwell_threshold").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skip_reason: Option<String>,
}

// ---------------------------------------------------------------------------
// Default dwell threshold (substrate policy — overridden by tending-policy Manifest)
// ---------------------------------------------------------------------------

/// Protocol-default dwell qualification in milliseconds.
///
/// The substrate-correct value lives in Manifest{kind: tending-policy,
/// payload_json: {dwell_qualification_ms: N}}. This constant is the
/// offline/pre-manifest fallback so the route is functional before any
/// tending-policy Manifest is authored. Mirrors the client-side value it
/// retires (F-POLICY-4: DWELL_THRESHOLD_MS = 3000 in attention-tracker.service.ts).
const DEFAULT_DWELL_QUALIFICATION_MS: u64 = 3000;

/// Resolve the active dwell qualification threshold.
///
/// Precedence (highest to lowest):
/// 1. tending-policy Manifest value (substrate-canonical).
/// 2. `DEFAULT_DWELL_QUALIFICATION_MS` (protocol floor fallback).
///
/// The Manifest lookup is deferred to Phase M-POLICY-4; for M-REA-2 this
/// always returns the default. When M-POLICY-4 lands, replace this function
/// with a pool query against the `manifests` projection.
fn resolve_dwell_qualification_ms(_pool: &DbPool, _ctx: &AppContext) -> u64 {
    // TODO(M-POLICY-4): query manifests table for tending-policy payload_json
    // and parse dwell_qualification_ms. Until M-POLICY-4 lands, fall back to
    // the protocol default (same value the client had hardcoded).
    DEFAULT_DWELL_QUALIFICATION_MS
}

// ---------------------------------------------------------------------------
// Handler
// ---------------------------------------------------------------------------

/// Handle `/api/v1/attention/*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // POST /api/v1/attention/tending
        (&Method::POST, "tending") => handle_post_tending(req, pool, ctx, hc_registry).await,

        _ => Ok(response::not_found(&format!(
            "Unknown attention route: {} /api/v1/attention/{}",
            method, path
        ))),
    }
}

/// POST /api/v1/attention/tending
///
/// Dwell-qualification gate: if `elapsed_ms < dwell_qualification_ms`, return
/// 200 with `{accepted: false, skipReason: "below_dwell_threshold"}` without
/// calling the coordinator. This moves the client-side threshold policy to the
/// substrate (F-POLICY-4 retirement + F-REA-2 retirement).
///
/// When accepted, forward to `create_attention_tending` on the `content_store`
/// zome. The coordinator derives `signer_pubkey` from `agent_info()` and writes
/// the private source-chain `AttentionTending` entry. No DHT gossip.
async fn handle_post_tending(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let intent: AttentionTendingIntentView = parse_body(req).await?;

    // --- Dwell-qualification gate (substrate policy, not client policy) ---
    let dwell_ms = resolve_dwell_qualification_ms(pool, ctx);
    if intent.elapsed_ms < dwell_ms {
        tracing::debug!(
            elapsed_ms = intent.elapsed_ms,
            dwell_qualification_ms = dwell_ms,
            "attention tending below dwell threshold — skipping coordinator call"
        );
        let ack = AttentionTendingAckView {
            accepted: false,
            action_hash: None,
            skip_reason: Some("below_dwell_threshold".to_string()),
        };
        return Ok(response::json_response(StatusCode::OK, &ack));
    }

    // --- Conductor-first path: call create_attention_tending coordinator ---
    if let Some(hc) = hc_registry.and_then(|r| r.lamad.clone()) {
        let zome_input = ZomeCreateAttentionTendingInput::from(&intent);
        let payload = rmp_serde::to_vec_named(&zome_input)
            .map_err(|e| StorageError::Conductor(format!("encode attention tending: {e}")))?;

        match hc
            .call_zome("content_store", "create_attention_tending", payload)
            .await
        {
            Ok(resp_bytes) => {
                // The coordinator returns an ActionHash (serialized as bytes).
                // Decode to a hex string for the HTTP response.
                let action_hash_hex: String = rmp_serde::from_slice::<Vec<u8>>(&resp_bytes)
                    .map(hex::encode)
                    .unwrap_or_else(|_| "unknown".to_string());

                let ack = AttentionTendingAckView {
                    accepted: true,
                    action_hash: Some(action_hash_hex),
                    skip_reason: None,
                };
                return Ok(response::json_response(StatusCode::CREATED, &ack));
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "content_store zome unreachable for create_attention_tending — \
                     private entry cannot fall back to storage (no projection table). \
                     Client will retry on next mount."
                );
                // AttentionTending is private (Visibility::Private) and never
                // written to storage — there is no projection table to fall back
                // to. Return 503 so the client can retry later rather than
                // silently dropping the tending record.
                return Err(StorageError::Conductor(
                    "create_attention_tending: conductor unavailable".to_string(),
                ));
            }
        }
    }

    // --- No conductor configured (e.g. storage-only test mode) ---
    tracing::debug!("no conductor configured for attention tending — returning accepted:false");
    let ack = AttentionTendingAckView {
        accepted: false,
        action_hash: None,
        skip_reason: Some("no_conductor".to_string()),
    };
    Ok(response::json_response(StatusCode::OK, &ack))
}
