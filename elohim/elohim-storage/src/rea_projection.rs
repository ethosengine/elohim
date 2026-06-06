//! REA Projection Signal Handler
//!
//! Receives post-commit signals from the Holochain conductor and projects
//! REA entries (Agreement, Commitment, EconomicEvent) into SQLite storage
//! with dht_anchor_hash for cryptographic verification.
//!
//! DHT is the truth. Storage is the index. This handler bridges them.
//!
//! ## Wire format
//!
//! The DNA's `ProjectionSignal` enum uses `#[serde(tag = "type", content =
//! "payload")]` (adjacent tagging). The payload variants carry the FULL
//! DHT entry — `Agreement`, `Commitment`, `EconomicEvent` — not a
//! pre-converted projection input. This module mirrors that wire shape via
//! the `*Entry` structs and does the projection-input conversion inside
//! `handle_rea_signal` (parsing `*_json` fields, downcasting f64→f32, etc.).
//!
//! ## Signal Flow
//!
//! 1. Coordinator zome commits an REA entry to the DHT
//! 2. Post-commit hook emits a `ProjectionSignal` with the action_hash + entry
//! 3. `HcClient::subscribe_rea_projection_signals` (main.rs) receives + decodes
//! 4. This handler converts the entry shape → CreateInput → upsert_with_anchor
//! 5. If the row already exists (optimistic pre-write), it sets dht_anchor_hash
//! 6. If new (DHT-first write), it inserts with anchor

use chrono::Utc;
use serde::Deserialize;
use tracing::{info, warn};

use crate::db::agreements::{self, CreateAgreementInput};
use crate::db::content_diesel::{self, ContentProjectionPatch};
use crate::db::context::AppContext;
use crate::db::economic_events::{self, CreateEconomicEventInput};
use crate::db::rea_commitments::{self, CreateReaCommitmentInput};
use crate::db::DbPool;
use crate::error::StorageError;

// ============================================================================
// Signal Types — mirror DNA-side ProjectionSignal exactly
//
// The DNA's ProjectionSignal uses #[serde(tag = "type", content = "payload")]
// (adjacent tagging). Variants embed the FULL DHT entry — Agreement,
// Commitment, EconomicEvent. The *Entry structs below must match the
// integrity-zome entry shapes field-for-field (see
// elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs).
//
// Any drift here is silently fatal: serde_json::from_value returns Err,
// the subscriber logs at debug, and the signal is dropped. Symptom is
// "REA commitment X written via conductor but projection did not land" —
// the in-process bounded poll times out at 1s.
// ============================================================================

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum ReaProjectionSignal {
    AgreementCommitted {
        action_hash: String,
        #[serde(default)]
        entry_hash: Option<String>,
        agreement: AgreementEntry,
        #[serde(default)]
        author: Option<String>,
    },
    ReaCommitmentCommitted {
        action_hash: String,
        #[serde(default)]
        entry_hash: Option<String>,
        commitment: CommitmentEntry,
        #[serde(default)]
        author: Option<String>,
    },
    ReaEconomicEventCommitted {
        action_hash: String,
        #[serde(default)]
        entry_hash: Option<String>,
        event: EconomicEventEntry,
        #[serde(default)]
        author: Option<String>,
    },
    /// Lamad Content entry committed. Carries the full Content entry shape
    /// for projection into the local SQL `content` table with anchor.
    /// Fires from the DNA post_commit for both create_content (initial
    /// publish) and update_content (e.g. blob_cid patches from stageSpaBlobs).
    ContentCommitted {
        action_hash: String,
        #[serde(default)]
        entry_hash: Option<String>,
        content: ContentEntry,
        #[serde(default)]
        author: Option<String>,
    },
}

/// Mirror of DNA `Agreement` entry shape.
#[derive(Debug, Clone, Deserialize)]
pub struct AgreementEntry {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

/// Mirror of DNA `Commitment` entry shape. Field names/types match the
/// integrity zome exactly — including `_json` suffixes on multi-value
/// fields and `f64` quantities (storage downcasts to f32 in the handler).
#[derive(Debug, Clone, Deserialize)]
pub struct CommitmentEntry {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    /// JSON-encoded `Vec<String>` on the wire. Decoded in the handler.
    #[serde(default)]
    pub resource_classified_as_json: Option<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f64>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f64>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub agreed_in: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub satisfies: Option<String>,
    /// JSON-encoded scope list on the wire. Decoded in the handler.
    #[serde(default)]
    pub in_scope_of_json: Option<String>,
    #[serde(default)]
    pub finished: bool,
    #[serde(default)]
    pub state: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
    #[serde(default)]
    pub updated_at: Option<String>,
}

/// Mirror of DNA `Content` entry shape (lamad zome content_store).
///
/// Field naming note: the DNA's blob field is `blob_cid` (Phase 0 refactor
/// per substrate-rea-replication-fix Addendum 5). The storage projection
/// mirrors that to both `blob_cid` AND the legacy `blob_hash` SQL column
/// inside upsert_with_anchor.
#[derive(Debug, Clone, Deserialize)]
pub struct ContentEntry {
    pub id: String,
    pub content_type: String,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub content: String,
    pub content_format: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub related_node_ids: Vec<String>,
    #[serde(default)]
    pub author_id: Option<String>,
    pub reach: String,
    #[serde(default)]
    pub trust_score: f64,
    #[serde(default)]
    pub estimated_minutes: Option<u32>,
    #[serde(default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub metadata_json: String,
    #[serde(default)]
    pub created_at: String,
    #[serde(default)]
    pub updated_at: String,
    #[serde(default)]
    pub schema_version: u32,
    #[serde(default)]
    pub validation_status: String,
    #[serde(default)]
    pub blob_cid: Option<String>,
    #[serde(default)]
    pub content_size_bytes: Option<u64>,
    #[serde(default)]
    pub content_hash: Option<String>,
}

/// Mirror of DNA `EconomicEvent` entry shape.
#[derive(Debug, Clone, Deserialize)]
pub struct EconomicEventEntry {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub to_resource_inventoried_as: Option<String>,
    #[serde(default)]
    pub resource_classified_as_json: Option<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f64>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f64>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_point_in_time: Option<String>,
    #[serde(default)]
    pub has_duration: Option<String>,
    #[serde(default)]
    pub input_of: Option<String>,
    #[serde(default)]
    pub output_of: Option<String>,
    #[serde(default)]
    pub fulfills_json: Option<String>,
    #[serde(default)]
    pub realization_of: Option<String>,
    #[serde(default)]
    pub satisfies_json: Option<String>,
    #[serde(default)]
    pub in_scope_of_json: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub triggered_by: Option<String>,
    #[serde(default)]
    pub at_location: Option<String>,
    #[serde(default)]
    pub image: Option<String>,
    #[serde(default)]
    pub lamad_event_type: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub created_at: Option<String>,
}

// ============================================================================
// Helpers
// ============================================================================

/// Parse a JSON-encoded `Vec<String>` field from the DNA entry. Empty
/// string or invalid JSON → empty Vec. Used for the `_json` resource and
/// scope fields. Drops empty entries so downstream code can treat `is_empty`
/// as "no value".
///
/// Public so the eager-projection path in the service layer can share the
/// same logic without duplicating it (Gap-F fix — see
/// `services/rea_commitment_service.rs` and `services/content_service.rs`).
pub fn parse_json_strings(raw: Option<&str>) -> Vec<String> {
    let s = match raw {
        Some(s) if !s.is_empty() => s,
        _ => return Vec::new(),
    };
    serde_json::from_str::<Vec<String>>(s).unwrap_or_else(|_| Vec::new())
}

/// Take the first element of a parsed JSON Vec<String>, or None.
/// Storage's CreateReaCommitmentInput stores resource_classified_as and
/// in_scope_of as single-value `Option<String>` columns; downstream readers
/// can reconstruct multi-value via the DHT entry if needed.
///
/// Public so the eager-projection path in the service layer can share the
/// same logic (Gap-F fix).
pub fn first_or_none(v: Vec<String>) -> Option<String> {
    v.into_iter().find(|s| !s.is_empty())
}

// ============================================================================
// Signal Handler
// ============================================================================

/// Handle an incoming REA projection signal from the conductor.
///
/// Main entry point — called from the signal dispatch loop. Acquires a DB
/// connection from the pool and upserts the projection row with dht_anchor_hash.
pub fn handle_rea_signal(
    signal: ReaProjectionSignal,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<(), StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("Pool error: {e}")))?;

    match signal {
        ReaProjectionSignal::AgreementCommitted {
            action_hash,
            agreement,
            ..
        } => {
            info!(id = %agreement.id, hash = %action_hash, "Projecting Agreement from DHT");
            let input = CreateAgreementInput {
                id: Some(agreement.id),
                name: agreement.name,
                note: agreement.note,
                // Agreement DNA entry has no metadata_json; projection writes None.
                metadata_json: None,
            };
            agreements::upsert_agreement(&mut conn, ctx, input, Some(&action_hash))?;
        }
        ReaProjectionSignal::ReaCommitmentCommitted {
            action_hash,
            commitment,
            ..
        } => {
            info!(id = %commitment.id, hash = %action_hash, "Projecting Commitment from DHT");
            let classified = first_or_none(parse_json_strings(
                commitment.resource_classified_as_json.as_deref(),
            ));
            let in_scope_of =
                first_or_none(parse_json_strings(commitment.in_scope_of_json.as_deref()));
            let input = CreateReaCommitmentInput {
                id: Some(commitment.id),
                action: commitment.action,
                provider: commitment.provider,
                receiver: commitment.receiver,
                resource_conforms_to: commitment.resource_conforms_to,
                resource_classified_as: classified,
                resource_quantity_value: commitment.resource_quantity_value.map(|v| v as f32),
                resource_quantity_unit: commitment.resource_quantity_unit,
                effort_quantity_value: commitment.effort_quantity_value.map(|v| v as f32),
                effort_quantity_unit: commitment.effort_quantity_unit,
                has_beginning: commitment.has_beginning,
                has_end: commitment.has_end,
                due: commitment.due,
                clause_of: commitment.clause_of,
                in_scope_of,
                medium_of_exchange_id: None,
                note: commitment.note,
                metadata_json: commitment.metadata_json,
                // Projection of an already-committed entry — supersession (if any)
                // already happened on the originating create; never re-run it here.
                supersedes: None,
            };
            rea_commitments::upsert_with_anchor(&mut conn, ctx, input, Some(&action_hash))?;
        }
        ReaProjectionSignal::ReaEconomicEventCommitted {
            action_hash, event, ..
        } => {
            info!(id = %event.id, hash = %action_hash, "Projecting EconomicEvent from DHT");

            // Phase 4 T4 — side-projection: if action='ack-projection', also
            // write into the projection_events operational log. Self-filtering:
            // other EconomicEvent actions (custody-blob, serve-blob) are ignored.
            let classified = parse_json_strings(event.resource_classified_as_json.as_deref());
            {
                let first_resource = classified.first().cloned().unwrap_or_default();
                let emitted_at = event
                    .has_point_in_time
                    .clone()
                    .unwrap_or_else(|| Utc::now().to_rfc3339());
                if let Err(e) = crate::p2p::projection_ack_handler::handle_projection_ack_sync(
                    pool,
                    &event.action,
                    &event.provider,
                    &first_resource,
                    &action_hash,
                    &emitted_at,
                ) {
                    warn!(
                        target = "rea_projection",
                        error = %e,
                        "projection_ack side-projection failed (non-fatal)"
                    );
                }
            }

            let input = CreateEconomicEventInput {
                id: Some(event.id),
                action: event.action,
                provider: event.provider,
                receiver: event.receiver,
                resource_conforms_to: event.resource_conforms_to,
                resource_inventoried_as: event.resource_inventoried_as,
                resource_classified_as: classified,
                resource_quantity_value: event.resource_quantity_value.map(|v| v as f32),
                resource_quantity_unit: event.resource_quantity_unit,
                effort_quantity_value: event.effort_quantity_value.map(|v| v as f32),
                effort_quantity_unit: event.effort_quantity_unit,
                has_point_in_time: event.has_point_in_time,
                has_duration: event.has_duration,
                input_of: event.input_of,
                output_of: event.output_of,
                lamad_event_type: event.lamad_event_type,
                // The CreateEconomicEventInput surface has app-specific link fields
                // (content_id, contributor_presence_id, path_id) that are not on
                // the DNA wire — leave None; the HTTP write path sets them.
                content_id: None,
                contributor_presence_id: None,
                path_id: None,
                triggered_by: event.triggered_by,
                note: event.note,
                metadata_json: event.metadata_json,
                at_location: event.at_location,
                scope_collab_cid: None,
            };
            economic_events::upsert_with_anchor(&mut conn, ctx, input, Some(&action_hash))?;
        }
        ReaProjectionSignal::ContentCommitted {
            action_hash,
            content,
            ..
        } => {
            info!(
                id = %content.id,
                hash = %action_hash,
                blob_cid = ?content.blob_cid,
                "Projecting Content from DHT"
            );
            // u64 → i32 with saturating cast. Real content sizes fit in
            // i32 with room to spare (max ~2.1 GB per blob; doorway proxy
            // caps far lower); the cast is defensive against malformed
            // entries claiming absurd sizes.
            let size_i32 = content
                .content_size_bytes
                .map(|n| i32::try_from(n).unwrap_or(i32::MAX));
            let patch = ContentProjectionPatch {
                blob_cid: content.blob_cid,
                content_size_bytes: size_i32,
                title: Some(content.title),
                description: Some(content.description),
                content_type: Some(content.content_type),
                content_format: Some(content.content_format),
                reach: Some(content.reach),
                metadata_json: Some(content.metadata_json),
            };
            content_diesel::upsert_with_anchor(&mut conn, ctx, &content.id, patch, &action_hash)?;
        }
    }

    Ok(())
}

/// Try to parse and handle a raw signal payload as an REA projection signal.
/// Returns Ok(true) if handled, Ok(false) if not an REA signal, Err on failure.
pub fn try_handle_signal(
    raw_payload: &[u8],
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<bool, StorageError> {
    match serde_json::from_slice::<ReaProjectionSignal>(raw_payload) {
        Ok(signal) => {
            handle_rea_signal(signal, pool, ctx)?;
            Ok(true)
        }
        Err(_) => {
            // Not an REA projection signal — caller should try other handlers
            Ok(false)
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference fixture: a JSON shape matching what the DNA's
    /// `ProjectionSignal::ReaCommitmentCommitted` emits over the wire
    /// (adjacent tagging — tag + payload). This test guards against
    /// silent breakage of the substrate-correct write path.
    #[test]
    fn decode_rea_commitment_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "ReaCommitmentCommitted",
            "payload": {
                "action_hash": "uhCkk-abc123",
                "entry_hash": "uhCEk-def456",
                "commitment": {
                    "id": "doorway:test|epr:test-app",
                    "action": "project-epr",
                    "provider": "doorway:test-doorway",
                    "receiver": "epr:test-app",
                    "resource_conforms_to": null,
                    "resource_inventoried_as": null,
                    "resource_classified_as_json": "[]",
                    "resource_quantity_value": null,
                    "resource_quantity_unit": null,
                    "effort_quantity_value": null,
                    "effort_quantity_unit": null,
                    "has_point_in_time": null,
                    "has_beginning": null,
                    "has_end": null,
                    "due": null,
                    "clause_of": null,
                    "agreed_in": null,
                    "input_of": null,
                    "output_of": null,
                    "satisfies": null,
                    "in_scope_of_json": "[\"doorway:test-doorway|epr:test-app\"]",
                    "finished": false,
                    "state": "proposed",
                    "note": null,
                    "metadata_json": "{}",
                    "created_at": "2026-05-26T12:00:00Z",
                    "updated_at": "2026-05-26T12:00:00Z"
                },
                "author": "uhCAk-xyz789"
            }
        });

        let signal: ReaProjectionSignal = serde_json::from_value(wire)
            .expect("DNA wire shape must decode into ReaProjectionSignal");

        match signal {
            ReaProjectionSignal::ReaCommitmentCommitted {
                action_hash,
                commitment,
                ..
            } => {
                assert_eq!(action_hash, "uhCkk-abc123");
                assert_eq!(commitment.id, "doorway:test|epr:test-app");
                assert_eq!(commitment.action, "project-epr");
                assert_eq!(
                    commitment.in_scope_of_json.as_deref(),
                    Some("[\"doorway:test-doorway|epr:test-app\"]")
                );
                assert!(!commitment.finished);
            }
            other => panic!("unexpected variant: {other:?}"),
        }
    }

    #[test]
    fn decode_agreement_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "AgreementCommitted",
            "payload": {
                "action_hash": "uhCkk-ag1",
                "entry_hash": "uhCEk-ag2",
                "agreement": {
                    "id": "agreement-001",
                    "name": "Test Agreement",
                    "note": null,
                    "created_at": "2026-05-26T12:00:00Z"
                },
                "author": "uhCAk-author"
            }
        });

        let signal: ReaProjectionSignal = serde_json::from_value(wire).unwrap();
        match signal {
            ReaProjectionSignal::AgreementCommitted {
                action_hash,
                agreement,
                ..
            } => {
                assert_eq!(action_hash, "uhCkk-ag1");
                assert_eq!(agreement.id, "agreement-001");
                assert_eq!(agreement.name.as_deref(), Some("Test Agreement"));
            }
            _ => panic!("expected AgreementCommitted"),
        }
    }

    #[test]
    fn decode_economic_event_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "ReaEconomicEventCommitted",
            "payload": {
                "action_hash": "uhCkk-ee1",
                "entry_hash": "uhCEk-ee2",
                "event": {
                    "id": "event-001",
                    "action": "ack-projection",
                    "provider": "doorway:test",
                    "receiver": "epr:test",
                    "resource_conforms_to": null,
                    "resource_inventoried_as": null,
                    "to_resource_inventoried_as": null,
                    "resource_classified_as_json": "[\"doorway:test|epr:test\"]",
                    "resource_quantity_value": null,
                    "resource_quantity_unit": null,
                    "effort_quantity_value": null,
                    "effort_quantity_unit": null,
                    "has_point_in_time": "2026-05-26T12:00:00Z",
                    "has_duration": null,
                    "input_of": null,
                    "output_of": null,
                    "fulfills_json": "[]",
                    "realization_of": null,
                    "satisfies_json": "[]",
                    "in_scope_of_json": "[]",
                    "note": null,
                    "state": "settled",
                    "triggered_by": null,
                    "at_location": null,
                    "image": null,
                    "lamad_event_type": null,
                    "metadata_json": "{}",
                    "created_at": "2026-05-26T12:00:00Z"
                },
                "author": "uhCAk-author"
            }
        });

        let signal: ReaProjectionSignal = serde_json::from_value(wire).unwrap();
        match signal {
            ReaProjectionSignal::ReaEconomicEventCommitted { event, .. } => {
                assert_eq!(event.id, "event-001");
                assert_eq!(event.action, "ack-projection");
                assert_eq!(
                    event.resource_classified_as_json.as_deref(),
                    Some("[\"doorway:test|epr:test\"]")
                );
            }
            _ => panic!("expected ReaEconomicEventCommitted"),
        }
    }

    #[test]
    fn decode_content_signal_from_dna_wire_shape() {
        let wire = serde_json::json!({
            "type": "ContentCommitted",
            "payload": {
                "action_hash": "uhCkk-content-anchor",
                "entry_hash": "uhCEk-content-entry",
                "content": {
                    "id": "elohim-host-landing",
                    "content_type": "html5-app",
                    "title": "Elohim Host Landing",
                    "description": "The hosted landing page bundle.",
                    "summary": null,
                    "content": "",
                    "content_format": "html5-app",
                    "tags": ["landing", "spa"],
                    "source_path": null,
                    "related_node_ids": [],
                    "author_id": null,
                    "reach": "commons",
                    "trust_score": 1.0,
                    "estimated_minutes": null,
                    "thumbnail_url": null,
                    "metadata_json": "{}",
                    "created_at": "2026-05-26T12:00:00Z",
                    "updated_at": "2026-05-26T13:30:00Z",
                    "schema_version": 1,
                    "validation_status": "Valid",
                    "blob_cid": "sha256-deadbeefcafe1234",
                    "content_size_bytes": 4096,
                    "content_hash": "sha256-deadbeefcafe1234"
                },
                "author": "uhCAk-author"
            }
        });

        let signal: ReaProjectionSignal =
            serde_json::from_value(wire).expect("DNA wire shape for ContentCommitted must decode");

        match signal {
            ReaProjectionSignal::ContentCommitted {
                action_hash,
                content,
                ..
            } => {
                assert_eq!(action_hash, "uhCkk-content-anchor");
                assert_eq!(content.id, "elohim-host-landing");
                assert_eq!(content.blob_cid.as_deref(), Some("sha256-deadbeefcafe1234"));
                assert_eq!(content.content_size_bytes, Some(4096));
                assert_eq!(content.tags.len(), 2);
                assert!((content.trust_score - 1.0).abs() < f64::EPSILON);
            }
            _ => panic!("expected ContentCommitted variant"),
        }
    }

    #[test]
    fn parse_json_strings_handles_empty_and_invalid() {
        assert_eq!(parse_json_strings(None), Vec::<String>::new());
        assert_eq!(parse_json_strings(Some("")), Vec::<String>::new());
        assert_eq!(parse_json_strings(Some("[]")), Vec::<String>::new());
        assert_eq!(parse_json_strings(Some("not json")), Vec::<String>::new());
        assert_eq!(
            parse_json_strings(Some("[\"a\", \"b\"]")),
            vec!["a".to_string(), "b".to_string()]
        );
    }

    #[test]
    fn first_or_none_picks_first_non_empty() {
        assert_eq!(first_or_none(vec![]), None);
        assert_eq!(first_or_none(vec!["".to_string()]), None);
        assert_eq!(first_or_none(vec!["x".to_string()]), Some("x".to_string()));
        assert_eq!(
            first_or_none(vec!["a".to_string(), "b".to_string()]),
            Some("a".to_string())
        );
    }

    /// Old internally-tagged shape (what storage USED to expect) must FAIL
    /// to decode — guards against accidental revert of the wire-shape fix.
    #[test]
    fn old_internal_tagging_shape_fails_to_decode() {
        let stale_wire = serde_json::json!({
            "type": "ReaCommitmentCommitted",
            "action_hash": "uhCkk-abc",
            "commitment": {
                "id": "x",
                "action": "y",
                "provider": "p",
                "receiver": "r"
            }
        });
        let result: Result<ReaProjectionSignal, _> = serde_json::from_value(stale_wire);
        assert!(
            result.is_err(),
            "internally-tagged wire shape must NOT decode (would mean storage drifted away from DNA again)"
        );
    }
}
