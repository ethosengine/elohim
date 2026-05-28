//! Lamad intent API controller
//!
//! Routes: `/api/v1/lamad/events`
//!
//! Accepts a high-level `LamadEventIntent` discriminated union and composes
//! the full EconomicEvent (action, provider, receiver) server-side using
//! substrate-known PROTOCOL_EVENT_MAPPINGS. The client sends intent — the
//! substrate decides the REA shape. This eliminates F-REA-1 client-side
//! REA composition.
//!
//! ## Design classification (P2P Design Gate)
//!
//! - **Entity:** LamadEventIntent — request body (intent wire shape, not an entity).
//! - **EconomicEvent** — Category A (existing, elohim DNA content_store zome).
//! - **Projection:** `economic_events` table (already exists, no new table).
//! - **Source of truth:** DHT EconomicEvent entry (via post-commit projection).
//!
//! ## Route placement rationale (storage vs doorway)
//!
//! This route lives in `elohim-storage/src/api/` rather than
//! `doorway/doorway-service/src/routes/`. Doorway is a web2 bridge — it holds
//! no domain logic and its routes are manifest-driven (`project_doorway_manifest_driven_routes`).
//! The REA composition logic (`map_intent_to_rea`, `compose_event_from_intent`) IS
//! domain logic; it belongs in storage alongside every other EconomicEvent handler.
//! Both browser (via doorway proxy) and Tauri (direct to :8090) receive identical
//! semantics at the same path — which is the whole point of the unified storage API.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::{Deserialize, Serialize};
use tracing;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::hc_client_registry::HcClientRegistry;
use crate::services::response::{self, from_create_result};
use crate::services::EconomicEventService;
use crate::views::{CreateEconomicEventInputView, EconomicEventView, LamadEventIntentView};

use super::{get_conn, parse_body};

/// Handle `/api/v1/lamad/*` requests
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
        // POST /api/v1/lamad/events — intent-driven EconomicEvent creation
        (&Method::POST, "events") => handle_emit_event(req, pool, ctx, hc_registry).await,

        _ => Ok(response::not_found(&format!(
            "Unknown lamad route: {} /api/v1/lamad/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Zome wire type mirror — LamadEventIntentInput
// ---------------------------------------------------------------------------
//
// Mirrors the `LamadEventIntentInput` struct added to content_store/src/lib.rs.
// Field names MUST match exactly (MessagePack-serialized). Wire-internal only —
// does NOT cross the HTTP boundary (use `LamadEventIntentView` for that).

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ZomeLamadEventIntentInput {
    pub agent_id: String,
    pub lamad_event_type: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contributor_presence_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_value: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resource_quantity_unit: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
}

impl From<&LamadEventIntentView> for ZomeLamadEventIntentInput {
    fn from(v: &LamadEventIntentView) -> Self {
        Self {
            agent_id: v.agent_id.clone(),
            lamad_event_type: v.lamad_event_type.clone(),
            content_id: v.content_id.clone(),
            path_id: v.path_id.clone(),
            contributor_presence_id: v.contributor_presence_id.clone(),
            resource_quantity_value: v.resource_quantity_value.map(|x| x as f64),
            resource_quantity_unit: v.resource_quantity_unit.clone(),
            note: v.note.clone(),
            // metadata arrives as JsonVal; pre-stringify for the zome boundary
            // (never pass serde_json::Value over SerializedBytes)
            metadata_json: v
                .metadata
                .as_ref()
                .and_then(|m| serde_json::to_string(&m.0).ok()),
        }
    }
}

/// POST /api/v1/lamad/events
///
/// Accepts a `LamadEventIntent` and composes the full EconomicEvent using
/// PROTOCOL_EVENT_MAPPINGS. The client provides only the high-level semantic
/// intent; the substrate fills in action, provider, and receiver.
///
/// Execution path (preference order):
///
/// 1. **Conductor-first (DHT):** when `hc_registry.lamad` is Some, forward the
///    intent to `create_rea_economic_event_from_intent` on the `content_store`
///    zome. The post-commit signal writes the SQLite projection.
///
/// 2. **Storage-only fallback:** when the conductor bridge is absent (Tauri
///    offline mode, early-startup timing, or tests without a conductor), the
///    composed EconomicEvent is written directly to SQLite. The record will be
///    reconciled against the DHT on next conductor reconnect.
///
/// Mapping table mirrors `intent_to_rea_triple` in the content_store zome and
/// the TypeScript PROTOCOL_EVENT_MAPPINGS in `protocol-event-types.ts`.
async fn handle_emit_event(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
    hc_registry: Option<&Arc<HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let intent: LamadEventIntentView = parse_body(req).await?;

    // --- Path 1: conductor-first via content_store zome ---
    if let Some(hc) = hc_registry.and_then(|r| r.lamad.clone()) {
        let zome_input = ZomeLamadEventIntentInput::from(&intent);
        let payload = rmp_serde::to_vec_named(&zome_input)
            .map_err(|e| StorageError::Conductor(format!("encode intent: {e}")))?;
        match hc
            .call_zome(
                "content_store",
                "create_rea_economic_event_from_intent",
                payload,
            )
            .await
        {
            Ok(resp_bytes) => {
                // Decode the ReaEconomicEventOutput from the zome.
                // We only need the event fields for the HTTP response; the
                // post-commit signal handles the SQLite projection async.
                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case")]
                struct ZomeEventOutput {
                    event: ZomeEconomicEvent,
                }
                #[derive(Deserialize)]
                #[serde(rename_all = "snake_case")]
                struct ZomeEconomicEvent {
                    id: String,
                    action: String,
                    provider: String,
                    receiver: String,
                    #[serde(default)]
                    lamad_event_type: Option<String>,
                    #[serde(default)]
                    has_point_in_time: String,
                    #[serde(default)]
                    state: String,
                }
                let out: ZomeEventOutput = rmp_serde::from_slice(&resp_bytes)
                    .map_err(|e| StorageError::Conductor(format!("decode intent output: {e}")))?;
                let e = out.event;
                let view = EconomicEventView {
                    id: e.id,
                    h_app_id: ctx.h_app_id.clone(),
                    action: e.action,
                    provider: e.provider,
                    receiver: e.receiver,
                    resource_conforms_to: None,
                    resource_inventoried_as: None,
                    resource_classified_as: None,
                    resource_quantity_value: None,
                    resource_quantity_unit: None,
                    effort_quantity_value: None,
                    effort_quantity_unit: None,
                    has_point_in_time: e.has_point_in_time,
                    has_duration: None,
                    input_of: None,
                    output_of: None,
                    note: None,
                    state: e.state,
                    triggered_by: None,
                    at_location: None,
                    lamad_event_type: e.lamad_event_type,
                    content_id: intent.content_id.clone(),
                    contributor_presence_id: intent.contributor_presence_id.clone(),
                    path_id: intent.path_id.clone(),
                    scope_collab_cid: None,
                    metadata: None,
                    created_at: String::new(),
                    dht_anchor_hash: None,
                };
                return Ok(crate::services::response::json_response(
                    hyper::StatusCode::CREATED,
                    &view,
                ));
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "content_store zome unreachable — falling back to storage-only write"
                );
                // Fall through to storage-only path below
            }
        }
    }

    // --- Path 2: storage-only fallback (offline / no conductor) ---
    let input_view = compose_event_from_intent(intent)?;
    let input: crate::db::economic_events::CreateEconomicEventInput = input_view.into();
    let mut conn = get_conn(pool)?;

    Ok(from_create_result(EconomicEventService::create_event(
        &mut conn, ctx, input,
    )))
}

/// Compose a `CreateEconomicEventInputView` from a `LamadEventIntentView`
/// using substrate-known PROTOCOL_EVENT_MAPPINGS.
///
/// The mapping defines:
/// - `action` — the REA action verb
/// - `provider` / `receiver` — derived from `agentId` and the contextual ID
///   (contentId, pathId, contributorPresenceId) per event-type semantics.
///
/// Returns `Err` for unknown or unsupported `lamadEventType` values.
pub(crate) fn compose_event_from_intent(
    intent: LamadEventIntentView,
) -> Result<CreateEconomicEventInputView, StorageError> {
    let (action, provider, receiver) = map_intent_to_rea(&intent).ok_or_else(|| {
        StorageError::InvalidInput(format!(
            "Unknown lamadEventType '{}' — not in PROTOCOL_EVENT_MAPPINGS",
            intent.lamad_event_type
        ))
    })?;

    let metadata = intent.metadata.clone();

    Ok(CreateEconomicEventInputView {
        id: None,
        schema_version: 1,
        action,
        provider,
        receiver,
        resource_conforms_to: None,
        resource_inventoried_as: None,
        resource_classified_as: vec![],
        resource_quantity_value: intent.resource_quantity_value,
        resource_quantity_unit: intent.resource_quantity_unit,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: None,
        has_duration: None,
        input_of: None,
        output_of: None,
        lamad_event_type: Some(intent.lamad_event_type),
        content_id: intent.content_id.clone(),
        contributor_presence_id: intent.contributor_presence_id.clone(),
        path_id: intent.path_id.clone(),
        triggered_by: None,
        note: intent.note,
        metadata,
        at_location: None,
        scope_collab_cid: None,
        at_block_height: 0,
    })
}

/// Map `lamadEventType` to `(action, provider, receiver)`.
///
/// Provider is always the initiating agent. Receiver is:
/// - the `contentId` for content-interaction events
/// - the `pathId` for path-progress events
/// - the `contributorPresenceId` for recognition events
/// - the `agentId` itself (self-production) for achievement events
///
/// Returns `None` for unknown event types.
fn map_intent_to_rea(intent: &LamadEventIntentView) -> Option<(String, String, String)> {
    let agent = intent.agent_id.clone();
    let content = intent.content_id.clone().unwrap_or_default();
    let path = intent.path_id.clone().unwrap_or_default();
    let presence = intent.contributor_presence_id.clone().unwrap_or_default();

    match intent.lamad_event_type.as_str() {
        // Attention — agent uses the content resource
        "content-view" => Some(("use".into(), agent.clone(), content)),
        "session-start" => Some(("use".into(), agent.clone(), agent.clone())),
        "session-end" => Some(("use".into(), agent.clone(), agent.clone())),

        // Demonstration — agent produces a completion credential
        "content-complete" => Some(("produce".into(), agent.clone(), agent.clone())),
        "assessment-start" => Some(("use".into(), agent.clone(), content)),
        "assessment-complete" => Some(("produce".into(), agent.clone(), agent.clone())),
        "practice-attempt" => Some(("use".into(), agent.clone(), content)),
        "quiz-submit" => Some(("produce".into(), agent.clone(), agent.clone())),

        // Path progress — agent produces path-level achievements
        "path-step-complete" => Some(("produce".into(), agent.clone(), agent.clone())),
        "path-complete" => Some(("produce".into(), agent.clone(), agent.clone())),

        // Recognition — agent raises recognition for another presence
        "recognition-given" => Some(("raise".into(), agent.clone(), presence)),
        "recognition-received" => Some(("raise".into(), agent.clone(), agent.clone())),
        "affinity-mark" => Some(("raise".into(), agent.clone(), content)),
        "endorsement" => Some(("raise".into(), agent.clone(), content)),
        "citation" => Some(("cite".into(), agent.clone(), content)),
        "recognition-transfer" => Some(("transfer".into(), agent.clone(), presence)),

        // Achievement — credentials earned
        "attestation-grant" => Some(("produce".into(), agent.clone(), agent.clone())),
        "capability-earn" => Some(("produce".into(), agent.clone(), agent.clone())),

        // Creation — agent produces content
        "content-create" => Some(("produce".into(), agent.clone(), content)),
        "path-create" => Some(("produce".into(), agent.clone(), path)),
        "extension-create" => Some(("produce".into(), agent.clone(), content)),
        "map-synthesis" => Some(("produce".into(), agent.clone(), content)),
        "analysis-complete" => Some(("produce".into(), agent.clone(), content)),

        // Stewardship — agent begins steward work
        "stewardship-begin" => Some(("work".into(), agent.clone(), content)),
        "invitation-send" => Some(("deliver-service".into(), agent.clone(), content)),
        "presence-claim" => Some(("accept".into(), agent.clone(), presence)),

        // Governance
        "attestation-revoke" => Some(("modify".into(), agent.clone(), content)),
        "content-flag" => Some(("modify".into(), agent.clone(), content)),
        "governance-vote" => Some(("work".into(), agent.clone(), content)),

        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::views::LamadEventIntentView;

    fn make_intent(lamad_event_type: &str) -> LamadEventIntentView {
        LamadEventIntentView {
            agent_id: "agent-abc".into(),
            lamad_event_type: lamad_event_type.into(),
            content_id: Some("content-123".into()),
            path_id: Some("path-456".into()),
            contributor_presence_id: Some("presence-789".into()),
            resource_quantity_value: None,
            resource_quantity_unit: None,
            metadata: None,
            note: None,
        }
    }

    #[test]
    fn content_view_composes_use_action() {
        let intent = make_intent("content-view");
        let result = compose_event_from_intent(intent).unwrap();
        assert_eq!(result.action, "use");
        assert_eq!(result.provider, "agent-abc");
        assert_eq!(result.receiver, "content-123");
        assert_eq!(result.lamad_event_type, Some("content-view".into()));
    }

    #[test]
    fn content_complete_composes_produce_action_self_receiver() {
        let intent = make_intent("content-complete");
        let result = compose_event_from_intent(intent).unwrap();
        assert_eq!(result.action, "produce");
        assert_eq!(result.provider, "agent-abc");
        assert_eq!(result.receiver, "agent-abc"); // self-production
    }

    #[test]
    fn recognition_given_routes_to_presence() {
        let intent = make_intent("recognition-given");
        let result = compose_event_from_intent(intent).unwrap();
        assert_eq!(result.action, "raise");
        assert_eq!(result.receiver, "presence-789");
    }

    #[test]
    fn path_step_complete_is_self_produced() {
        let intent = make_intent("path-step-complete");
        let result = compose_event_from_intent(intent).unwrap();
        assert_eq!(result.action, "produce");
        assert_eq!(result.provider, "agent-abc");
        assert_eq!(result.receiver, "agent-abc");
    }

    #[test]
    fn unknown_event_type_returns_error() {
        let intent = LamadEventIntentView {
            agent_id: "agent-abc".into(),
            lamad_event_type: "totally-made-up".into(),
            content_id: None,
            path_id: None,
            contributor_presence_id: None,
            resource_quantity_value: None,
            resource_quantity_unit: None,
            metadata: None,
            note: None,
        };
        let result = compose_event_from_intent(intent);
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(msg.contains("totally-made-up"));
    }

    #[test]
    fn all_schema_event_types_map_successfully() {
        // Verify every event type in the JSON schema enum maps without error.
        let schema_event_types = [
            "content-view",
            "content-complete",
            "path-step-complete",
            "path-complete",
            "session-start",
            "session-end",
            "assessment-start",
            "assessment-complete",
            "practice-attempt",
            "quiz-submit",
            "affinity-mark",
            "endorsement",
            "citation",
            "recognition-given",
            "recognition-received",
            "attestation-grant",
            "capability-earn",
            "content-create",
            "path-create",
            "extension-create",
            "map-synthesis",
            "analysis-complete",
            "stewardship-begin",
            "invitation-send",
            "presence-claim",
            "recognition-transfer",
            "attestation-revoke",
            "content-flag",
            "governance-vote",
        ];

        for event_type in &schema_event_types {
            let intent = make_intent(event_type);
            let result = compose_event_from_intent(intent);
            assert!(
                result.is_ok(),
                "Event type '{}' should map to a valid REA action",
                event_type
            );
        }
    }
}
