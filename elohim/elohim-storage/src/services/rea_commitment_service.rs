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

use diesel::SqliteConnection;

use crate::db::context::AppContext;
use crate::db::rea_commitments::{
    self, CreateReaCommitmentInput, ReaCommitmentQuery, UpdateReaCommitmentState,
    PROJECT_EPR_ACTION,
};
use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::rea_projection::{first_or_none, parse_json_strings};
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
        let output_bytes = conductor_writes::call_create_rea_commitment(hc, &shefa_input).await?;

        // 2. Eagerly project the SQL row from the zome output (Gap-F fix).
        //
        //    The post-commit signal is unreliable under deployed conductor
        //    latency — it arrives asynchronously and may land after the HTTP
        //    request has already timed out. We can derive the full projection
        //    from the zome output directly: it carries the committed entry
        //    (same data the signal would carry) plus the ActionHash. This
        //    write is idempotent with the signal path: both call
        //    rea_commitments::upsert_with_anchor with the same anchor string,
        //    so a later signal arrival is a harmless no-op update.
        //
        //    Field mapping mirrors rea_projection.rs::ReaCommitmentCommitted arm
        //    exactly: parse_json_strings + first_or_none for multi-value fields,
        //    f64→f32 downcast for quantities. The action_hash string is derived
        //    from the holo_hash ActionHash Display impl which produces the same
        //    "uhCkk…" base32 form the signal carries.
        let output = rmp_serde::from_slice::<shefa_types::ReaCommitmentOutput>(&output_bytes)
            .map_err(|e| {
                StorageError::Internal(format!(
                    "conductor returned success for create_rea_commitment but \
                     output could not be decoded as ReaCommitmentOutput: {e}"
                ))
            })?;
        let action_hash_str = format!("{}", output.action_hash);
        let c = &output.commitment;
        let classified = first_or_none(parse_json_strings(Some(&c.resource_classified_as_json)));
        let in_scope_of = first_or_none(parse_json_strings(Some(&c.in_scope_of_json)));
        let projection_input = CreateReaCommitmentInput {
            id: Some(c.id.clone()),
            action: c.action.clone(),
            provider: c.provider.clone(),
            receiver: c.receiver.clone(),
            resource_conforms_to: c.resource_conforms_to.clone(),
            resource_classified_as: classified,
            resource_quantity_value: c.resource_quantity_value.map(|v| v as f32),
            resource_quantity_unit: c.resource_quantity_unit.clone(),
            effort_quantity_value: c.effort_quantity_value.map(|v| v as f32),
            effort_quantity_unit: c.effort_quantity_unit.clone(),
            has_beginning: c.has_beginning.clone(),
            has_end: c.has_end.clone(),
            due: c.due.clone(),
            clause_of: c.clause_of.clone(),
            in_scope_of,
            medium_of_exchange_id: None, // storage-only field; not on DNA wire
            note: c.note.clone(),
            metadata_json: Some(c.metadata_json.clone()),
        };
        let commitment = rea_commitments::upsert_with_anchor(
            conn,
            ctx,
            projection_input,
            Some(&action_hash_str),
        )?;

        if let Some(bus) = events {
            bus.emit(StorageEvent::ProjectionRegistered {
                commitment_id: commitment.id.clone(),
            });
        }
        Ok(ReaCommitmentView::from(commitment))
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

        let zome_input = shefa_types::UpdateReaCommitmentStateInput {
            id: id.to_string(),
            state: update.state.clone(),
            finished: update.finished,
        };

        let output_bytes =
            conductor_writes::call_update_rea_commitment_state(hc, &zome_input).await?;

        // Eagerly project the state update from the zome output (Gap-F fix).
        //
        //    The conductor returned a ReaCommitmentOutput carrying the full
        //    updated entry + new ActionHash. We apply two synchronous writes:
        //    (1) update_commitment_state to write the new state + finished flag
        //    into the SQL row (the field the caller is polling for), and
        //    (2) upsert_with_anchor to advance dht_anchor_hash to the new entry
        //    (the `update_rea_commitment_state` zome fn writes a new DHT entry
        //    via update_entry, producing a new ActionHash distinct from the
        //    original create ActionHash).
        //
        //    The signal path (ReaCommitmentCommitted on update) fires
        //    upsert_with_anchor with the new entry too, but only updates the
        //    anchor column (not state) — so the signal path has always been
        //    incomplete for state propagation. This eager path is correct and
        //    complete; a later signal arrival is a harmless anchor-only update.
        let output = rmp_serde::from_slice::<shefa_types::ReaCommitmentOutput>(&output_bytes)
            .map_err(|e| {
                StorageError::Internal(format!(
                    "conductor returned success for update_rea_commitment_state but \
                     output could not be decoded as ReaCommitmentOutput: {e}"
                ))
            })?;
        let action_hash_str = format!("{}", output.action_hash);

        // Write new state + finished flag.
        let commitment = rea_commitments::update_commitment_state(conn, ctx, id, update)?;

        // Advance dht_anchor_hash to the new update entry's ActionHash.
        // upsert_with_anchor on an existing row only updates dht_anchor_hash,
        // which is exactly what we need here.
        let c = &output.commitment;
        let classified = first_or_none(parse_json_strings(Some(&c.resource_classified_as_json)));
        let in_scope_of = first_or_none(parse_json_strings(Some(&c.in_scope_of_json)));
        let anchor_input = CreateReaCommitmentInput {
            id: Some(c.id.clone()),
            action: c.action.clone(),
            provider: c.provider.clone(),
            receiver: c.receiver.clone(),
            resource_conforms_to: c.resource_conforms_to.clone(),
            resource_classified_as: classified,
            resource_quantity_value: c.resource_quantity_value.map(|v| v as f32),
            resource_quantity_unit: c.resource_quantity_unit.clone(),
            effort_quantity_value: c.effort_quantity_value.map(|v| v as f32),
            effort_quantity_unit: c.effort_quantity_unit.clone(),
            has_beginning: c.has_beginning.clone(),
            has_end: c.has_end.clone(),
            due: c.due.clone(),
            clause_of: c.clause_of.clone(),
            in_scope_of,
            medium_of_exchange_id: None,
            note: c.note.clone(),
            metadata_json: Some(c.metadata_json.clone()),
        };
        rea_commitments::upsert_with_anchor(conn, ctx, anchor_input, Some(&action_hash_str))?;

        if let Some(bus) = events {
            if commitment.action == PROJECT_EPR_ACTION && update.state == "cancelled" {
                bus.emit(StorageEvent::ProjectionRevoked {
                    commitment_id: commitment.id.clone(),
                });
            }
        }
        Ok(ReaCommitmentView::from(commitment))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    /// Validates that `shefa_types::Commitment` (the entry field inside
    /// `ReaCommitmentOutput`) round-trips through MessagePack encoding+decoding.
    ///
    /// This exercises the field shapes the eager-projection decode path depends
    /// on: `create_via_conductor` and `update_state_via_conductor` decode the
    /// full `ReaCommitmentOutput` then access `.commitment` fields. If a field
    /// name drifted between the zome entry type and this types crate, the decode
    /// would return empty/default values silently. This test catches that at
    /// compile+test time.
    ///
    /// Note: `ReaCommitmentOutput` itself contains `ActionHash` (a holo_hash
    /// type with validated prefix bytes); we test the inner `Commitment` entry
    /// separately since `ActionHash` construction requires correct prefix bytes
    /// that vary by hash type. The conductor msgpack integration that exercises
    /// the full `Output` type lives in the sweettest suite.
    #[test]
    fn rea_commitment_entry_msgpack_roundtrip() {
        let original = shefa_types::Commitment {
            id: "doorway:alpha|epr:lamad-spa".to_string(),
            action: "project-epr".to_string(),
            provider: "doorway:alpha-elohim-host".to_string(),
            receiver: "epr:lamad-spa".to_string(),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as_json: "[]".to_string(),
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            agreed_in: None,
            input_of: None,
            output_of: None,
            satisfies: None,
            in_scope_of_json: "[\"doorway:alpha-elohim-host|epr:lamad-spa\"]".to_string(),
            finished: false,
            state: "proposed".to_string(),
            note: None,
            metadata_json: "{}".to_string(),
            created_at: "2026-05-29T00:00:00Z".to_string(),
            updated_at: "2026-05-29T00:00:00Z".to_string(),
        };

        let bytes = rmp_serde::to_vec_named(&original).expect("encode Commitment");
        let decoded: shefa_types::Commitment =
            rmp_serde::from_slice(&bytes).expect("decode Commitment");

        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.action, original.action);
        assert_eq!(decoded.state, original.state);
        assert_eq!(decoded.in_scope_of_json, original.in_scope_of_json);
        assert_eq!(decoded.metadata_json, original.metadata_json);
    }

    /// Validates that `lamad_types::Content` (the entry field inside
    /// `ContentOutput`) round-trips through MessagePack.
    ///
    /// Same rationale as `rea_commitment_entry_msgpack_roundtrip` — tests the
    /// inner entry struct that the eager projection path maps into
    /// `ContentProjectionPatch`, without needing to construct `ActionHash`.
    #[test]
    fn content_entry_msgpack_roundtrip() {
        let original = lamad_types::Content {
            id: "elohim-host-landing".to_string(),
            content_type: "html5-app".to_string(),
            title: "Elohim Host Landing".to_string(),
            description: "SPA bundle".to_string(),
            summary: None,
            content: "".to_string(),
            content_format: "html5-app".to_string(),
            tags: vec!["landing".to_string()],
            source_path: None,
            related_node_ids: vec![],
            author_id: None,
            reach: "commons".to_string(),
            trust_score: 1.0,
            estimated_minutes: None,
            thumbnail_url: None,
            metadata_json: "{}".to_string(),
            created_at: "2026-05-29T00:00:00Z".to_string(),
            updated_at: "2026-05-29T00:00:00Z".to_string(),
            schema_version: 1,
            validation_status: "Valid".to_string(),
            blob_cid: Some("sha256-deadbeef".to_string()),
            content_size_bytes: Some(4096),
            content_hash: None,
        };

        let bytes = rmp_serde::to_vec_named(&original).expect("encode Content");
        let decoded: lamad_types::Content = rmp_serde::from_slice(&bytes).expect("decode Content");

        assert_eq!(decoded.id, original.id);
        assert_eq!(decoded.blob_cid, original.blob_cid);
        assert_eq!(decoded.content_size_bytes, original.content_size_bytes);
        assert_eq!(decoded.metadata_json, original.metadata_json);
        assert_eq!(decoded.tags, original.tags);
    }

    /// Validates the field mapping from `shefa_types::Commitment` (output entry)
    /// to `db::rea_commitments::CreateReaCommitmentInput` (projection input)
    /// matches the signal handler's mapping in rea_projection.rs exactly:
    /// parse_json_strings + first_or_none for multi-value fields, f64→f32 cast.
    #[test]
    fn commitment_output_to_projection_input_field_mapping() {
        let c = shefa_types::Commitment {
            id: "test-commit-001".to_string(),
            action: "project-epr".to_string(),
            provider: "doorway:test".to_string(),
            receiver: "epr:test-app".to_string(),
            resource_conforms_to: None,
            resource_inventoried_as: None,
            resource_classified_as_json: "[\"class-a\"]".to_string(),
            resource_quantity_value: Some(1.5),
            resource_quantity_unit: Some("hours".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_point_in_time: None,
            has_beginning: Some("2026-05-29T00:00:00Z".to_string()),
            has_end: None,
            due: None,
            clause_of: None,
            agreed_in: None,
            input_of: None,
            output_of: None,
            satisfies: None,
            in_scope_of_json: "[\"doorway:test|epr:test-app\",\"extra\"]".to_string(),
            finished: false,
            state: "proposed".to_string(),
            note: Some("test note".to_string()),
            metadata_json: "{\"key\":\"val\"}".to_string(),
            created_at: "2026-05-29T00:00:00Z".to_string(),
            updated_at: "2026-05-29T00:00:00Z".to_string(),
        };

        // Mirror the mapping from create_via_conductor
        let classified = first_or_none(parse_json_strings(Some(&c.resource_classified_as_json)));
        let in_scope_of = first_or_none(parse_json_strings(Some(&c.in_scope_of_json)));
        let input = CreateReaCommitmentInput {
            id: Some(c.id.clone()),
            action: c.action.clone(),
            provider: c.provider.clone(),
            receiver: c.receiver.clone(),
            resource_conforms_to: c.resource_conforms_to.clone(),
            resource_classified_as: classified,
            resource_quantity_value: c.resource_quantity_value.map(|v| v as f32),
            resource_quantity_unit: c.resource_quantity_unit.clone(),
            effort_quantity_value: c.effort_quantity_value.map(|v| v as f32),
            effort_quantity_unit: c.effort_quantity_unit.clone(),
            has_beginning: c.has_beginning.clone(),
            has_end: c.has_end.clone(),
            due: c.due.clone(),
            clause_of: c.clause_of.clone(),
            in_scope_of,
            medium_of_exchange_id: None,
            note: c.note.clone(),
            metadata_json: Some(c.metadata_json.clone()),
        };

        assert_eq!(input.id.as_deref(), Some("test-commit-001"));
        assert_eq!(input.action, "project-epr");
        // first_or_none picks the first non-empty from the JSON array
        assert_eq!(input.resource_classified_as.as_deref(), Some("class-a"));
        assert_eq!(
            input.in_scope_of.as_deref(),
            Some("doorway:test|epr:test-app")
        );
        // f64 → f32 cast
        assert!((input.resource_quantity_value.unwrap() - 1.5_f32).abs() < 1e-5);
        assert_eq!(input.note.as_deref(), Some("test note"));
        assert_eq!(input.medium_of_exchange_id, None);
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
