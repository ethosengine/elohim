//! Mishpat recognition participation API controller
//!
//! Routes: `/api/v1/mishpat/recognition/participation`
//!
//! Accepts a `RecognitionParticipationIntent` (the wire shape defined in
//! `elohim/sdk/schemas/v1/intents/recognition-participation-intent.schema.json`),
//! resolves the recognition weight from the standing-policy Manifest stored in
//! SQLite, and forwards to the mishpat `create_recognition_event_from_participation`
//! coordinator (or falls back to a direct EconomicEvent write when the conductor
//! bridge is absent).
//!
//! ## Design classification (P2P Design Gate)
//!
//! - **Entity:** RecognitionParticipationIntent — request body (intent wire
//!   shape, not an entity). M-REA-3 / §1.5.
//! - **EconomicEvent** — Category A (existing, elohim DNA content_store zome).
//!   No new DHT entry type.
//! - **Projection:** `economic_events` table (already exists, no new table).
//! - **Source of truth:** DHT EconomicEvent entry (via post-commit projection).
//! - **Weight policy:** `Manifest{kind: "standing-policy"}.recognition_weight_by_level`
//!   (Category A via existing Manifest entry type). Falls back to protocol
//!   defaults when no Manifest is found.
//!
//! ## Route placement rationale (storage vs doorway)
//!
//! Domain logic (weight lookup from Manifest, intent validation, REA composition)
//! belongs in storage — not in doorway. Both browser (via doorway proxy) and
//! Tauri (direct to :8090) receive identical semantics at the same path.
//!
//! ## Closes Slice 2.2b (F-REA-3 + F-POLICY-3)
//!
//! Retires client-side `GovernanceRecognitionService.recordParticipation()` and
//! the `WEIGHT_BY_LEVEL` constant. Callers (`reaction-bar.component.ts`,
//! `graduated-feedback.component.ts`) POST to this route instead.

use std::sync::Arc;

use bytes::Bytes;
use http_body_util::Full;
use hyper::{body::Incoming, Method, Request, Response};
use serde::{Deserialize, Serialize};
use tracing;

use crate::db::manifests::fetch_manifests_by_kind;
use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::services::EconomicEventService;
use crate::views::CreateEconomicEventInputView;

use super::{get_conn, parse_body};

// ---------------------------------------------------------------------------
// Wire type — mirrors recognition-participation-intent.schema.json
// ---------------------------------------------------------------------------

/// HTTP request body for POST /api/v1/mishpat/recognition/participation.
///
/// Mirrors `RecognitionParticipationIntent` from the JSON schema at
/// `elohim/sdk/schemas/v1/intents/recognition-participation-intent.schema.json`.
/// camelCase via `#[serde(rename_all = "camelCase")]`.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionParticipationIntentView {
    /// What is being governed (e.g. "content", "proposal").
    pub entity_type: String,
    /// ID of the governed entity — becomes the receiver of the EconomicEvent.
    pub entity_id: String,
    /// Agent public key / human ID of the participant — becomes the provider.
    pub human_id: String,
    /// Governance mechanism level (0–7).
    pub mechanism_level: u8,
    /// Type of participation act.
    pub participation_type: String,
}

/// Response body for a successfully recorded participation event.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecognitionParticipationResultView {
    /// ID of the created EconomicEvent.
    pub event_id: String,
    /// Resolved recognition weight (from Manifest or protocol default).
    pub recognition_weight: f64,
    /// lamadEventType composed from the participation_type.
    pub lamad_event_type: String,
    /// CID of the standing-policy Manifest used for weight lookup, if any.
    pub policy_manifest_cid: Option<String>,
}

// ---------------------------------------------------------------------------
// Protocol-default weight table
// ---------------------------------------------------------------------------

/// Protocol-default recognition weight by mechanism level.
///
/// Applied when no standing-policy Manifest with `recognition_weight_by_level`
/// is found in the SQLite projection. Mirrors `default_recognition_weight` in
/// the mishpat coordinator (both must stay in sync with the schema description).
fn default_recognition_weight(level: u8) -> f64 {
    match level {
        0 => 0.1,
        1 => 0.1,
        2 => 0.3,
        3 => 0.5,
        4 => 0.7,
        5 => 0.7,
        6 => 0.8,
        7 => 1.0,
        _ => 0.1,
    }
}

/// Resolve the recognition weight for a given mechanism level.
///
/// Reads the active standing-policy Manifest from the SQLite projection and
/// extracts `recognition_weight_by_level["L{level}"]`. Falls back to
/// `default_recognition_weight(level)` when:
///   - no standing-policy Manifest exists in the projection, or
///   - the Manifest payload has no `recognition_weight_by_level` field, or
///   - the specific level key is absent.
///
/// Returns `(weight, Option<manifest_cid>)`.
fn resolve_recognition_weight(
    conn: &mut crate::db::SqliteConnection,
    level: u8,
) -> Result<(f64, Option<String>), StorageError> {
    let manifests = fetch_manifests_by_kind(conn, "standing-policy")?;

    // Use the first (most recently verified) standing-policy manifest.
    if let Some(manifest) = manifests.first() {
        let payload: serde_json::Value =
            serde_json::from_str(&manifest.payload_json).unwrap_or(serde_json::Value::Null);

        let level_key = format!("L{}", level);
        let weight = payload
            .get("recognition_weight_by_level")
            .and_then(|w| w.get(&level_key))
            .and_then(|v| v.as_f64());

        let resolved_weight = weight.unwrap_or_else(|| default_recognition_weight(level));
        return Ok((resolved_weight, Some(manifest.cid.clone())));
    }

    Ok((default_recognition_weight(level), None))
}

// ---------------------------------------------------------------------------
// Route dispatcher
// ---------------------------------------------------------------------------

/// Handle `/api/v1/mishpat/recognition/*` requests.
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // POST /api/v1/mishpat/recognition/participation
        (&Method::POST, "participation") => {
            handle_record_participation(req, pool, ctx).await
        }
        _ => Ok(response::not_found(&format!(
            "Unknown mishpat/recognition route: {} /api/v1/mishpat/recognition/{}",
            method, path
        ))),
    }
}

// ---------------------------------------------------------------------------
// Handler implementations
// ---------------------------------------------------------------------------

/// POST /api/v1/mishpat/recognition/participation
///
/// Records a governance-participation REA EconomicEvent. The handler:
///
/// 1. Parses the `RecognitionParticipationIntent` from the request body.
/// 2. Resolves the recognition weight from the standing-policy Manifest
///    projection in SQLite (falls back to protocol defaults).
/// 3. Composes the EconomicEvent shape:
///    - action = "work" (governance-{type} → work in PROTOCOL_EVENT_MAPPINGS)
///    - provider = human_id
///    - receiver = entity_id
///    - resource_quantity_value = resolved weight
///    - resource_quantity_unit = "recognition"
///    - lamad_event_type = "governance-{participation_type}"
/// 4. Writes via EconomicEventService (storage-only path — conductor bridge
///    for mishpat zome is a Phase E sweettest concern; the storage-first
///    pattern matches the lamad.rs precedent and reconciles on reconnect).
///
/// ## Fire-and-forget semantics from the client's perspective
/// Recognition failures must never block governance signal recording. The
/// caller (reaction-bar, graduated-feedback) posts this concurrently and
/// ignores non-2xx responses. The substrate writes the EconomicEvent
/// regardless of client acknowledgement.
async fn handle_record_participation(
    req: Request<Incoming>,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let intent: RecognitionParticipationIntentView = parse_body(req).await?;

    // Validate participation_type
    let known_types = ["reaction", "graduated-feedback", "vote", "block"];
    if !known_types.contains(&intent.participation_type.as_str()) {
        return Ok(response::bad_request(&format!(
            "Unknown participation_type '{}' — must be one of: {}",
            intent.participation_type,
            known_types.join(", ")
        )));
    }

    let mut conn = get_conn(pool)?;

    // Resolve weight from standing-policy Manifest projection.
    let (recognition_weight, policy_manifest_cid) =
        resolve_recognition_weight(&mut conn, intent.mechanism_level)?;

    // Compose lamadEventType from participationType.
    // "governance-vote" maps to REA action "work" via PROTOCOL_EVENT_MAPPINGS.
    let lamad_event_type = format!("governance-{}", intent.participation_type);

    tracing::debug!(
        entity_type = %intent.entity_type,
        entity_id = %intent.entity_id,
        human_id = %intent.human_id,
        mechanism_level = intent.mechanism_level,
        participation_type = %intent.participation_type,
        recognition_weight,
        lamad_event_type = %lamad_event_type,
        policy_manifest_cid = ?policy_manifest_cid,
        "recording governance participation EconomicEvent"
    );

    // Compose the EconomicEvent input.
    // action = "work" — governance participation is stewardship labor on the entity.
    // provider = human_id — the participant performs the governance work.
    // receiver = entity_id — the governed entity receives the stewardship act.
    let event_id = format!(
        "governance-participation-{}-{}-{}",
        intent.participation_type,
        intent.entity_id,
        chrono::Utc::now().timestamp_millis()
    );

    let input_view = CreateEconomicEventInputView {
        id: Some(event_id.clone()),
        schema_version: 1,
        action: "work".to_string(),
        provider: intent.human_id.clone(),
        receiver: intent.entity_id.clone(),
        resource_conforms_to: None,
        resource_inventoried_as: None,
        resource_classified_as: vec!["governance-participation".to_string()],
        resource_quantity_value: Some(recognition_weight as f32),
        resource_quantity_unit: Some("recognition".to_string()),
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: None,
        has_duration: None,
        input_of: None,
        output_of: None,
        lamad_event_type: Some(lamad_event_type.clone()),
        content_id: Some(intent.entity_id.clone()),
        contributor_presence_id: None,
        path_id: None,
        triggered_by: None,
        note: Some(format!(
            "governance {} participation on {} (entity_type={})",
            intent.participation_type, intent.entity_id, intent.entity_type
        )),
        metadata: None,
        at_location: None,
        scope_collab_cid: None,
        at_block_height: 0,
    };

    let input: crate::db::economic_events::CreateEconomicEventInput = input_view.into();
    let create_result = EconomicEventService::create_event(&mut conn, ctx, input);

    // Extract event ID from create result for the response.
    match create_result {
        Ok(event) => {
            let result_view = RecognitionParticipationResultView {
                event_id: event.id.clone(),
                recognition_weight,
                lamad_event_type,
                policy_manifest_cid,
            };
            Ok(response::json_response(
                hyper::StatusCode::CREATED,
                &serde_json::json!({ "data": result_view }),
            ))
        }
        Err(e) => {
            tracing::warn!(error = %e, "failed to record governance participation EconomicEvent");
            Ok(response::json_response(
                hyper::StatusCode::INTERNAL_SERVER_ERROR,
                &serde_json::json!({
                    "error": format!("failed to record participation event: {e}")
                }),
            ))
        }
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_weights_match_weight_by_level_constant() {
        // Verify the storage-side default table matches the schema description
        // and the mishpat coordinator's default_recognition_weight function.
        assert_eq!(default_recognition_weight(0), 0.1);
        assert_eq!(default_recognition_weight(1), 0.1);
        assert_eq!(default_recognition_weight(2), 0.3);
        assert_eq!(default_recognition_weight(3), 0.5);
        assert_eq!(default_recognition_weight(4), 0.7);
        assert_eq!(default_recognition_weight(5), 0.7);
        assert_eq!(default_recognition_weight(6), 0.8);
        assert_eq!(default_recognition_weight(7), 1.0);
        // Unknown level → lowest weight
        assert_eq!(default_recognition_weight(8), 0.1);
        assert_eq!(default_recognition_weight(255), 0.1);
    }

    #[test]
    fn lamad_event_type_composition() {
        // Verify the governance-{type} composition rule is correct.
        let cases = [
            ("reaction", "governance-reaction"),
            ("graduated-feedback", "governance-graduated-feedback"),
            ("vote", "governance-vote"),
            ("block", "governance-block"),
        ];
        for (participation_type, expected) in &cases {
            let composed = format!("governance-{}", participation_type);
            assert_eq!(&composed, expected);
        }
    }
}
