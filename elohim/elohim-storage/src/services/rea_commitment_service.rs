//! REA Commitment service — business logic
//!
//! ## Substrate write paths (post 2026-05-26-substrate-rea-replication-fix.md)
//!
//! - `action == "project-epr"` → conductor-first round-trip via
//!   `conductor_writes::call_create_rea_commitment`. Closes the cross-peer
//!   replication gap that produced `/lamad` → 404 and stale x-content-address
//!   on alpha. Per `elohim/holochain/dna/CLAUDE.md` gospel: notarized types
//!   MUST round-trip through the local conductor so the post-commit signal
//!   fires and Holochain DHT gossip propagates the entry to other peers.
//!
//! - All other actions → diesel-direct (legacy, scoped out of this migration).
//!   The wire-shape divergence between `db::CreateReaCommitmentInput` and
//!   `shefa_types::CreateReaCommitmentInput` (`medium_of_exchange_id` is
//!   storage-only; `in_scope_of` is `Option<String>` vs `Vec<String>`; f32
//!   vs f64 numeric width) means a blanket migration would lose data for
//!   non-project-epr actions. Those migrate in a follow-up sprint.

use std::sync::Arc;
use std::time::Duration;

use diesel::SqliteConnection;

use crate::db::context::AppContext;
use crate::db::rea_commitments::{
    self, CreateReaCommitmentInput, ReaCommitmentQuery, UpdateReaCommitmentState,
    PROJECT_EPR_ACTION,
};
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::conductor_writes;
use crate::services::events::{EventBus, StorageEvent};
use crate::views::ReaCommitmentView;

pub struct ReaCommitmentService;

impl ReaCommitmentService {
    pub async fn create(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateReaCommitmentInput,
        events: Option<&EventBus>,
        hc_lamad: Option<&Arc<HcClient>>,
    ) -> Result<ReaCommitmentView, StorageError> {
        if input.action == PROJECT_EPR_ACTION {
            return Self::create_via_conductor(conn, ctx, input, events, hc_lamad).await;
        }
        // Legacy diesel-direct path — preserved for non-project-epr actions
        // pending follow-up migration. See module docs.
        Self::create_via_diesel(conn, ctx, input, events)
    }

    fn create_via_diesel(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateReaCommitmentInput,
        events: Option<&EventBus>,
    ) -> Result<ReaCommitmentView, StorageError> {
        let commitment = rea_commitments::create_commitment(conn, ctx, input)?;
        if let Some(bus) = events {
            if commitment.action == PROJECT_EPR_ACTION {
                bus.emit(StorageEvent::ProjectionRegistered {
                    commitment_id: commitment.id.clone(),
                });
            }
        }
        Ok(ReaCommitmentView::from(commitment))
    }

    async fn create_via_conductor(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        input: CreateReaCommitmentInput,
        events: Option<&EventBus>,
        hc_lamad: Option<&Arc<HcClient>>,
    ) -> Result<ReaCommitmentView, StorageError> {
        let hc = hc_lamad.ok_or_else(|| {
            StorageError::Conductor(
                "lamad bridge unavailable — required for project-epr commitments".into(),
            )
        })?;

        // Ensure id is concrete — DNA wire shape requires it. Mirrors the
        // storage-side fallback (db::rea_commitments::create_commitment also
        // generates UUID on None).
        let id = input
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

        let shefa_input = to_shefa_input(&id, &input);

        // 1. Round-trip through the conductor. The post-commit handler at
        //    elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:10768
        //    emits ProjectionSignal::ReaCommitmentCommitted, which the
        //    in-process signal subscriber routes to
        //    rea_projection::project_signal — upsert with dht_anchor_hash.
        let _output_bytes = conductor_writes::call_create_rea_commitment(hc, &shefa_input).await?;

        // 2. Poll local SQL for the projection. The signal pipeline is
        //    in-process so latency is typically <100ms; 20 × 50ms gives a
        //    generous 1s ceiling. Tighten post-soak if needed.
        for _ in 0..20 {
            if let Some(commitment) = rea_commitments::get_commitment(conn, ctx, &id)? {
                if let Some(bus) = events {
                    bus.emit(StorageEvent::ProjectionRegistered {
                        commitment_id: commitment.id.clone(),
                    });
                }
                return Ok(ReaCommitmentView::from(commitment));
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Err(StorageError::Timeout(format!(
            "REA commitment {} written via conductor but projection did not \
             land in local SQL within 1s — retryable (transient post-commit latency); \
             if persistent, check the rea_projection subscriber + post-commit signal pipeline",
            id
        )))
    }

    pub fn get_by_id(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
    ) -> Result<Option<ReaCommitmentView>, StorageError> {
        rea_commitments::get_commitment(conn, ctx, id).map(|opt| opt.map(ReaCommitmentView::from))
    }

    pub fn list(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        query: &ReaCommitmentQuery,
    ) -> Result<Vec<ReaCommitmentView>, StorageError> {
        rea_commitments::list_commitments(conn, ctx, query)
            .map(|v| v.into_iter().map(ReaCommitmentView::from).collect())
    }

    pub fn get_by_agent(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        agent_id: &str,
        limit: i64,
    ) -> Result<Vec<ReaCommitmentView>, StorageError> {
        rea_commitments::get_commitments_for_agent(conn, ctx, agent_id, limit)
            .map(|v| v.into_iter().map(ReaCommitmentView::from).collect())
    }

    /// Update commitment state.
    ///
    /// Like `create`, branches on the underlying commitment's action: for
    /// project-epr we round-trip through the conductor's
    /// content_store::update_rea_commitment_state coordinator (Task 6 of
    /// the substrate-rea-replication-fix plan); other actions take the
    /// legacy diesel-direct path.
    pub async fn update_state(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
        update: &UpdateReaCommitmentState,
        events: Option<&EventBus>,
        hc_lamad: Option<&Arc<HcClient>>,
    ) -> Result<ReaCommitmentView, StorageError> {
        // Need to know the existing commitment's action to decide path.
        let existing = rea_commitments::get_commitment(conn, ctx, id)?
            .ok_or_else(|| StorageError::NotFound(format!("commitment {} not found", id)))?;

        if existing.action == PROJECT_EPR_ACTION {
            return Self::update_state_via_conductor(conn, ctx, id, update, events, hc_lamad).await;
        }

        // Legacy diesel-direct path.
        let commitment = rea_commitments::update_commitment_state(conn, ctx, id, update)?;
        if let Some(bus) = events {
            if commitment.action == PROJECT_EPR_ACTION && update.state == "cancelled" {
                bus.emit(StorageEvent::ProjectionRevoked {
                    commitment_id: commitment.id.clone(),
                });
            }
        }
        Ok(ReaCommitmentView::from(commitment))
    }

    async fn update_state_via_conductor(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
        update: &UpdateReaCommitmentState,
        events: Option<&EventBus>,
        hc_lamad: Option<&Arc<HcClient>>,
    ) -> Result<ReaCommitmentView, StorageError> {
        let hc = hc_lamad.ok_or_else(|| {
            StorageError::Conductor(
                "lamad bridge unavailable — required for project-epr commitment updates".into(),
            )
        })?;

        let input = shefa_types::UpdateReaCommitmentStateInput {
            id: id.to_string(),
            state: update.state.clone(),
            finished: update.finished,
        };

        let _output = conductor_writes::call_update_rea_commitment_state(hc, &input).await?;

        // Poll for projection to reflect the new state. Same bounded wait
        // shape as create_via_conductor.
        for _ in 0..20 {
            if let Some(commitment) = rea_commitments::get_commitment(conn, ctx, id)? {
                if commitment.state == update.state {
                    if let Some(bus) = events {
                        if commitment.action == PROJECT_EPR_ACTION && update.state == "cancelled" {
                            bus.emit(StorageEvent::ProjectionRevoked {
                                commitment_id: commitment.id.clone(),
                            });
                        }
                    }
                    return Ok(ReaCommitmentView::from(commitment));
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        Err(StorageError::Timeout(format!(
            "REA commitment {} state update written via conductor but \
             projection did not reflect state={} within 1s — retryable (transient)",
            id, update.state
        )))
    }
}

/// Bridge the storage-layer input shape (Option<String> id, single-string
/// scope fields, includes medium_of_exchange_id) to the DNA wire shape
/// (required id, Vec<String> scope fields, no medium_of_exchange_id).
///
/// For project-epr action specifically, the dropped fields
/// (medium_of_exchange_id, resource_conforms_to) are not used — verified
/// by reading the seed-projections.ts canonical payload shape.
fn to_shefa_input(
    id: &str,
    storage: &CreateReaCommitmentInput,
) -> shefa_types::CreateReaCommitmentInput {
    shefa_types::CreateReaCommitmentInput {
        id: id.to_string(),
        action: storage.action.clone(),
        provider: storage.provider.clone(),
        receiver: storage.receiver.clone(),
        resource_classified_as: storage
            .resource_classified_as
            .clone()
            .map(|s| vec![s])
            .unwrap_or_default(),
        resource_quantity_value: storage.resource_quantity_value.map(|v| v as f64),
        resource_quantity_unit: storage.resource_quantity_unit.clone(),
        effort_quantity_value: storage.effort_quantity_value.map(|v| v as f64),
        effort_quantity_unit: storage.effort_quantity_unit.clone(),
        has_beginning: storage.has_beginning.clone(),
        has_end: storage.has_end.clone(),
        due: storage.due.clone(),
        clause_of: storage.clause_of.clone(),
        in_scope_of: storage
            .in_scope_of
            .clone()
            .map(|s| vec![s])
            .unwrap_or_default(),
        note: storage.note.clone(),
        metadata_json: storage.metadata_json.clone(),
    }
}
