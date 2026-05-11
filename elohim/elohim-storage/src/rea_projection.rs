//! REA Projection Signal Handler
//!
//! Receives post-commit signals from the Holochain conductor and projects
//! REA entries (Agreement, Commitment, EconomicEvent) into SQLite storage
//! with dht_anchor_hash for cryptographic verification.
//!
//! DHT is the truth. Storage is the index. This handler bridges them.
//!
//! ## Signal Flow
//!
//! 1. Coordinator zome commits an REA entry to the DHT
//! 2. Post-commit hook emits a ProjectionSignal with the action_hash
//! 3. This handler receives the signal and upserts into SQLite
//! 4. If the record already exists (optimistic pre-write), it sets dht_anchor_hash
//! 5. If the record is new (DHT-first write), it inserts with anchor
//!
//! ## Wiring
//!
//! TODO: Wire into conductor client's signal receive loop. The connection_loop
//! in conductor_client.rs currently only handles request/response. To receive
//! signals, the recv_task needs to detect signal messages (distinct from
//! response messages) and route them to `handle_rea_signal`. Alternatively,
//! the HcClient (hc_client.rs) may already have signal handling that can
//! dispatch to this handler.

use chrono::Utc;
use serde::Deserialize;
use tracing::info;

use crate::db::agreements::{self, CreateAgreementInput};
use crate::db::context::AppContext;
use crate::db::economic_events::{self, CreateEconomicEventInput};
use crate::db::rea_commitments::{self, CreateReaCommitmentInput};
use crate::db::DbPool;
use crate::error::StorageError;

// ============================================================================
// Signal Types — must match ProjectionSignal variants from coordinator zome
// ============================================================================

/// Signal payload matching ProjectionSignal variants from content_store coordinator.
/// The conductor serializes these as externally tagged enums.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum ReaProjectionSignal {
    AgreementCommitted {
        action_hash: String,
        agreement: AgreementPayload,
    },
    ReaCommitmentCommitted {
        action_hash: String,
        commitment: CommitmentPayload,
    },
    ReaEconomicEventCommitted {
        action_hash: String,
        event: EconomicEventPayload,
    },
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgreementPayload {
    pub id: String,
    pub name: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommitmentPayload {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_classified_as: Option<String>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<String>,
    pub has_beginning: Option<String>,
    pub has_end: Option<String>,
    pub due: Option<String>,
    pub clause_of: Option<String>,
    pub in_scope_of: Option<String>,
    pub medium_of_exchange_id: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EconomicEventPayload {
    pub id: String,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    pub resource_conforms_to: Option<String>,
    pub resource_inventoried_as: Option<String>,
    pub resource_classified_as: Vec<String>,
    pub resource_quantity_value: Option<f32>,
    pub resource_quantity_unit: Option<String>,
    pub effort_quantity_value: Option<f32>,
    pub effort_quantity_unit: Option<String>,
    pub has_point_in_time: Option<String>,
    pub has_duration: Option<String>,
    pub input_of: Option<String>,
    pub output_of: Option<String>,
    pub lamad_event_type: Option<String>,
    pub content_id: Option<String>,
    pub contributor_presence_id: Option<String>,
    pub path_id: Option<String>,
    pub triggered_by: Option<String>,
    pub note: Option<String>,
    pub metadata_json: Option<String>,
    pub at_location: Option<String>,
}

// ============================================================================
// Signal Handler
// ============================================================================

/// Handle an incoming REA projection signal from the conductor.
///
/// This is the main entry point — call this from the signal dispatch loop.
/// It acquires a DB connection from the pool and upserts the record.
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
        } => {
            info!(id = %agreement.id, hash = %action_hash, "Projecting Agreement from DHT");
            let input = CreateAgreementInput {
                id: Some(agreement.id),
                name: agreement.name,
                note: agreement.note,
                metadata_json: agreement.metadata_json,
            };
            agreements::upsert_agreement(&mut conn, ctx, input, Some(&action_hash))?;
        }
        ReaProjectionSignal::ReaCommitmentCommitted {
            action_hash,
            commitment,
        } => {
            info!(id = %commitment.id, hash = %action_hash, "Projecting Commitment from DHT");
            let input = CreateReaCommitmentInput {
                id: Some(commitment.id),
                action: commitment.action,
                provider: commitment.provider,
                receiver: commitment.receiver,
                resource_conforms_to: commitment.resource_conforms_to,
                resource_classified_as: commitment.resource_classified_as,
                resource_quantity_value: commitment.resource_quantity_value,
                resource_quantity_unit: commitment.resource_quantity_unit,
                effort_quantity_value: commitment.effort_quantity_value,
                effort_quantity_unit: commitment.effort_quantity_unit,
                has_beginning: commitment.has_beginning,
                has_end: commitment.has_end,
                due: commitment.due,
                clause_of: commitment.clause_of,
                in_scope_of: commitment.in_scope_of,
                medium_of_exchange_id: commitment.medium_of_exchange_id,
                note: commitment.note,
                metadata_json: commitment.metadata_json,
            };
            rea_commitments::upsert_with_anchor(&mut conn, ctx, input, Some(&action_hash))?;
        }
        ReaProjectionSignal::ReaEconomicEventCommitted { action_hash, event } => {
            info!(id = %event.id, hash = %action_hash, "Projecting EconomicEvent from DHT");

            // Phase 4 T4 — side-projection: if action='ack-projection', also
            // write into the projection_events operational log. Self-filtering:
            // other EconomicEvent actions (custody-blob, serve-blob) are ignored.
            {
                let first_resource = event
                    .resource_classified_as
                    .first()
                    .cloned()
                    .unwrap_or_default();
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
                    tracing::warn!(
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
                resource_classified_as: event.resource_classified_as,
                resource_quantity_value: event.resource_quantity_value,
                resource_quantity_unit: event.resource_quantity_unit,
                effort_quantity_value: event.effort_quantity_value,
                effort_quantity_unit: event.effort_quantity_unit,
                has_point_in_time: event.has_point_in_time,
                has_duration: event.has_duration,
                input_of: event.input_of,
                output_of: event.output_of,
                lamad_event_type: event.lamad_event_type,
                content_id: event.content_id,
                contributor_presence_id: event.contributor_presence_id,
                path_id: event.path_id,
                triggered_by: event.triggered_by,
                note: event.note,
                metadata_json: event.metadata_json,
                at_location: event.at_location,
            };
            economic_events::upsert_with_anchor(&mut conn, ctx, input, Some(&action_hash))?;
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
