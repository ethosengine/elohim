//! Writer service for `replicates-dwelling` REA commitments.
//!
//! ## Path decision (storage-direct, DHT anchor backfilled)
//!
//! Per spec §3 (P2P design gate), `replicates-dwelling` rides the EXISTING
//! `Mishpat::Commitment` entry type with `action="replicates-dwelling"` as
//! discriminator — no new DHT entry type. The canonical flow is DHT-first:
//!   Mishpat `create_commitment` → post-commit signal → storage projection.
//!
//! Sprint 3 uses the **storage-direct path** (consistent with
//! `rea_commitment_service` module docs: "legacy diesel-direct path, preserved
//! for non-project-epr actions pending follow-up migration"). This activates
//! BOTH consumers (`replication_prioritizer::active_commitments_for_provider`
//! and `peer_capacity_service::aggregate_pledges_by_tier`) immediately. The
//! `dht_anchor_hash` column is `NULL` until the Mishpat coordinator path is
//! plumbed in a follow-up sprint.
//!
//! ## Validation gate
//!
//! Calls `replicates_dwelling_validator::validate_for_creation` (schema +
//! donut checks) before writing. This is the author-time gate. The full
//! `validate_replicates_dwelling` (schema + donut + bounds_validator) is the
//! event-time gate and requires the commitment to already exist in the
//! conductor — not appropriate here.
//!
//! ## What this activates
//!
//! Writing one row via `create_replicates_dwelling_commitment` makes BOTH
//! downstream consumers light up on the same DB connection:
//! - `replication_prioritizer::active_commitments_for_provider(conn, provider)`
//!   returns a matching `ActiveCommitment`
//! - `peer_capacity_service::aggregate_pledges_by_tier(conn, provider)`
//!   returns the pledged bytes in the correct tier
//!
//! The felt-loop integration test (`replicates_dwelling_felt_loop`) proves
//! this end-to-end.

use diesel::prelude::*;
use diesel::SqliteConnection;

use crate::db::context::AppContext;
use crate::db::diesel_schema::rea_commitments;
use crate::db::models::ReaCommitment;
use crate::error::StorageError;
use crate::services::replicates_dwelling_validator::{
    validate_typed_for_creation, ProviderPledgedState, ValidationError,
};
use elohim_views::replicates_dwelling::{ReplicatesDwellingPayload, ScopeFilter};

/// Typed input for creating a `replicates-dwelling` commitment.
///
/// Mirrors the fields the caller must supply; does not include `id` (generated
/// internally) or `state` (always `"proposed"` at creation per REA convention).
#[derive(Debug, Clone)]
pub struct CreateReplicatesDwellingInput {
    /// Stable hub id of the pledging device/peer.
    pub provider: String,
    /// Hub id acting as the "receiver" for REA accounting purposes
    /// (same as `payload.recipient_dwelling_hub_id`).
    pub receiver: String,
    /// The full validated payload that will be serialized into `metadata_json`.
    pub payload: ReplicatesDwellingPayload,
    /// Optional: Mishpat ActionHash when the caller has already round-tripped
    /// through the conductor. `NULL` when using the storage-direct path.
    pub dht_anchor_hash: Option<String>,
}

/// Errors from the commitment writer.
#[derive(Debug, thiserror::Error)]
pub enum CreateCommitmentError {
    #[error("validation failed: {0}")]
    Validation(#[from] ValidationError),
    #[error("storage error: {0}")]
    Storage(#[from] StorageError),
    #[error("payload serialization failed: {0}")]
    Serialization(String),
}

/// Create a `replicates-dwelling` commitment row.
///
/// Three-step:
///   1. Serialize `input.payload` to JSON (cheap; catches mismatched types early).
///   2. Call `validate_for_creation` (schema + donut; no conductor round-trip).
///   3. `INSERT INTO rea_commitments` via Diesel (storage-direct; `dht_anchor_hash`
///      is `NULL` until the Mishpat coordinator path is wired in a follow-up sprint).
///
/// Returns the newly inserted `ReaCommitment` row. The caller can immediately
/// query both `active_commitments_for_provider` and `aggregate_pledges_by_tier`
/// on the same connection and observe the new commitment.
pub fn create_replicates_dwelling_commitment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateReplicatesDwellingInput,
    provider_state: &ProviderPledgedState,
) -> Result<ReaCommitment, CreateCommitmentError> {
    // 1. Validate at author time (schema + donut; no bounds_validator here).
    //    Works on the typed struct directly — avoids a JSON round-trip that
    //    would produce camelCase keys incompatible with the snake_case validator.
    validate_typed_for_creation(&input.payload, provider_state)?;

    // 2. Serialize payload → JSON for the metadata_json column.
    let metadata_json = serde_json::to_string(&input.payload)
        .map_err(|e| CreateCommitmentError::Serialization(e.to_string()))?;

    // 3. Write the projection row.
    let id = uuid::Uuid::new_v4().to_string();
    let anchor_ref = input.dht_anchor_hash.as_deref();

    diesel::insert_into(rea_commitments::table)
        .values((
            rea_commitments::id.eq(&id),
            rea_commitments::h_app_id.eq(&ctx.h_app_id),
            rea_commitments::action.eq("replicates-dwelling"),
            rea_commitments::provider.eq(&input.provider),
            rea_commitments::receiver.eq(&input.receiver),
            rea_commitments::state.eq("proposed"),
            rea_commitments::finished.eq(0i32),
            rea_commitments::metadata_json.eq(Some(&metadata_json)),
            rea_commitments::dht_anchor_hash.eq(anchor_ref),
        ))
        .execute(conn)
        .map_err(|e| CreateCommitmentError::Storage(StorageError::Database(e.to_string())))?;

    // Re-read the inserted row (mirrors `create_commitment` in db/rea_commitments.rs).
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::id.eq(&id))
        .first::<ReaCommitment>(conn)
        .map_err(|e| {
            CreateCommitmentError::Storage(StorageError::Database(format!(
                "failed to re-read inserted commitment {id}: {e}"
            )))
        })
}

// ---------------------------------------------------------------------------
// Builder helper: turn a flat input into the canonical payload struct.
// ---------------------------------------------------------------------------

/// Build a `ReplicatesDwellingPayload` from the effective constitutional ratios
/// and the supplied fields. The caller must supply `scope_filter` (use
/// `ScopeFilter::default()` for "all kinds").
///
/// `ratio_attestation` is auto-populated from
/// `constitutional_ratio_registry::effective_ratios()` so the caller does not
/// need to wire manifest CIDs manually.
#[allow(clippy::too_many_arguments)]
pub fn build_payload(
    provider_dwelling_hub_id: impl Into<String>,
    recipient_dwelling_hub_id: impl Into<String>,
    capacity_bytes: u64,
    valid_from: impl Into<String>,
    valid_until: impl Into<String>,
    grace_period_days: u32,
    rotation_ttl_days: u32,
    scope_filter: ScopeFilter,
) -> ReplicatesDwellingPayload {
    use crate::services::constitutional_ratio_registry;
    use elohim_views::replicates_dwelling::{ProviderRole, RatioAttestation};

    let provenance = constitutional_ratio_registry::effective_ratios();
    let r = provenance.ratios;

    ReplicatesDwellingPayload {
        action: "replicates-dwelling".to_string(),
        provider_dwelling_hub_id: provider_dwelling_hub_id.into(),
        recipient_dwelling_hub_id: recipient_dwelling_hub_id.into(),
        provider_role: ProviderRole::StewardMutual,
        via_collective_hub_id: None,
        capacity_bytes,
        scope_filter,
        valid_from: valid_from.into(),
        valid_until: valid_until.into(),
        grace_period_days,
        rotation_ttl_days,
        ratio_attestation: RatioAttestation {
            commons_pct: r.commons_pct,
            dwelling_pct: r.dwelling_pct,
            collective_pct: r.collective_pct,
            free_pct: r.free_pct,
            effective_ratio_cid: provenance.manifest_cid,
        },
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::constitutional_ratio_registry;
    use crate::services::peer_capacity_service::compute_peer_capacity;
    use crate::services::replication_prioritizer::active_commitments_for_provider;
    use crate::test_util::test_pool;
    use elohim_views::peer_capacity::Tier;
    use elohim_views::replicates_dwelling::{ProviderRole, RatioAttestation, ScopeFilter};

    fn ctx() -> AppContext {
        AppContext::new("test-app")
    }

    fn fresh_provider_state() -> ProviderPledgedState {
        ProviderPledgedState {
            total_raw_bytes: 100_000_000_000, // 100 GB
            pledged_dwelling_bytes: 0,
            pledged_collective_bytes: 0,
            pledged_commons_bytes: 0,
        }
    }

    /// Build a valid `ReplicatesDwellingPayload` using the manifest's effective
    /// ratios — so it passes `validate_for_creation`'s donut check.
    fn valid_payload(
        provider_hub: &str,
        recipient_hub: &str,
        capacity_bytes: u64,
    ) -> ReplicatesDwellingPayload {
        build_payload(
            provider_hub,
            recipient_hub,
            capacity_bytes,
            "2026-05-29T00:00:00Z",
            "2027-05-29T00:00:00Z",
            14,
            90,
            ScopeFilter {
                epr_kinds: Some(vec!["Content".to_string()]),
                bytes_per_blob_max: Some(100_000_000),
                requires_attestations: None,
                kinds_excluded: None,
            },
        )
    }

    // -----------------------------------------------------------------------
    // Felt-loop test: write one commitment; assert BOTH consumers light up.
    // -----------------------------------------------------------------------

    /// T-KEYSTONE: creating a replicates-dwelling commitment via the writer
    /// activates BOTH consumers — prioritizer AND pledged bar — on the same DB.
    ///
    /// This is the felt-loop proof that closes the producer gap:
    ///   - replication_prioritizer::active_commitments_for_provider returns
    ///     the matching ActiveCommitment
    ///   - peer_capacity_service::aggregate_pledges_by_tier returns the
    ///     pledged bytes in the Dwelling tier
    #[test]
    fn felt_loop_writer_activates_prioritizer_and_pledged_bar() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let provider = "agent:uhCAkProvider-felt-loop";
        let payload = valid_payload("hub:providerA", "hub:recipientB", 30_000_000_000);

        let input = CreateReplicatesDwellingInput {
            provider: provider.to_string(),
            receiver: "hub:recipientB".to_string(),
            payload,
            dht_anchor_hash: None,
        };

        let row = create_replicates_dwelling_commitment(
            &mut conn,
            &ctx(),
            input,
            &fresh_provider_state(),
        )
        .expect("create should succeed");

        assert_eq!(row.action, "replicates-dwelling");
        assert_eq!(row.provider, provider);
        assert_eq!(row.state, "proposed");

        // --- Consumer 1: replication_prioritizer ---
        let commitments =
            active_commitments_for_provider(&mut conn, provider).expect("prioritizer query");
        assert_eq!(
            commitments.len(),
            1,
            "prioritizer must find exactly one active commitment"
        );
        let c = &commitments[0];
        assert_eq!(c.action, "replicates-dwelling");
        assert_eq!(c.recipient_hub_id, "hub:recipientB");
        assert_eq!(
            c.scope_epr_kinds,
            Some(vec!["Content".to_string()]),
            "scope_epr_kinds matches payload"
        );

        // --- Consumer 2: peer_capacity_service (pledged bar) ---
        let view = compute_peer_capacity(&mut conn, provider).expect("peer capacity view");
        assert_eq!(
            view.pledges.dwelling_bytes, 30_000_000_000,
            "pledged bar shows 30 GB dwelling"
        );
        assert_eq!(view.pledges.collective_bytes, 0);
        assert_eq!(view.pledges.commons_bytes, 0);
        assert_eq!(view.pledges.total_pledged_bytes, 30_000_000_000);
        assert_eq!(
            view.pledges.pledges_by_recipient.len(),
            1,
            "one pledge-by-recipient entry"
        );
        let by_rcpt = &view.pledges.pledges_by_recipient[0];
        assert_eq!(by_rcpt.tier, Tier::Dwelling);
        assert_eq!(by_rcpt.capacity_bytes, 30_000_000_000);
        assert_eq!(by_rcpt.recipient_hub_id, "hub:recipientB");
    }

    // -----------------------------------------------------------------------
    // Validation-rejection test: invalid payload produces no DB row.
    // -----------------------------------------------------------------------

    /// T-REJECT: a payload missing a required field is rejected before insert.
    #[test]
    fn invalid_payload_rejected_no_row_written() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let provider = "agent:uhCAkProvider-invalid";

        // Build a payload but with commons_pct=0 (below DNA floor of 10).
        let provenance = constitutional_ratio_registry::effective_ratios();
        let bad_payload = ReplicatesDwellingPayload {
            action: "replicates-dwelling".to_string(),
            provider_dwelling_hub_id: "hub:providerX".to_string(),
            recipient_dwelling_hub_id: "hub:recipientY".to_string(),
            provider_role: ProviderRole::StewardMutual,
            via_collective_hub_id: None,
            capacity_bytes: 10_000_000_000,
            scope_filter: ScopeFilter::default(),
            valid_from: "2026-05-29T00:00:00Z".to_string(),
            valid_until: "2027-05-29T00:00:00Z".to_string(),
            grace_period_days: 14,
            rotation_ttl_days: 90,
            ratio_attestation: RatioAttestation {
                commons_pct: 0, // INVALID: below floor (10)
                dwelling_pct: 70,
                collective_pct: 20,
                free_pct: 10,
                effective_ratio_cid: provenance.manifest_cid.clone(),
            },
        };

        let input = CreateReplicatesDwellingInput {
            provider: provider.to_string(),
            receiver: "hub:recipientY".to_string(),
            payload: bad_payload,
            dht_anchor_hash: None,
        };

        let result = create_replicates_dwelling_commitment(
            &mut conn,
            &ctx(),
            input,
            &fresh_provider_state(),
        );

        assert!(
            matches!(result, Err(CreateCommitmentError::Validation(_))),
            "bad payload must be rejected; got: {result:?}"
        );

        // Assert: no row was written.
        let commitments =
            active_commitments_for_provider(&mut conn, provider).expect("query after rejection");
        assert_eq!(
            commitments.len(),
            0,
            "no commitment row must exist after rejection"
        );

        // Also assert the pledged bar is dark.
        let view =
            compute_peer_capacity(&mut conn, provider).expect("capacity view after rejection");
        assert_eq!(
            view.pledges.dwelling_bytes, 0,
            "pledged bar must remain dark after rejection"
        );
        assert_eq!(view.pledges.pledges_by_recipient.len(), 0);
    }

    // -----------------------------------------------------------------------
    // Donut ceiling test: adding a pledge that pushes dwelling above ceiling
    // is rejected.
    // -----------------------------------------------------------------------

    /// T-CEILING: adding a second pledge that would breach the dwelling ceiling
    /// is rejected even when the first pledge was valid.
    #[test]
    fn ceiling_breach_rejected_second_pledge() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let provider = "agent:uhCAkProvider-ceiling";

        // First pledge: 30 GB on 100 GB device = 30% (within 40% ceiling).
        let payload1 = valid_payload("hub:P", "hub:R1", 30_000_000_000);
        let state1 = fresh_provider_state();

        create_replicates_dwelling_commitment(
            &mut conn,
            &ctx(),
            CreateReplicatesDwellingInput {
                provider: provider.to_string(),
                receiver: "hub:R1".to_string(),
                payload: payload1,
                dht_anchor_hash: None,
            },
            &state1,
        )
        .expect("first pledge should succeed");

        // Second pledge: 20 GB more = 50% total (above 40% ceiling).
        let payload2 = valid_payload("hub:P", "hub:R2", 20_000_000_000);
        let state2 = ProviderPledgedState {
            total_raw_bytes: 100_000_000_000,
            pledged_dwelling_bytes: 30_000_000_000, // existing pledge included
            pledged_collective_bytes: 0,
            pledged_commons_bytes: 0,
        };

        let result = create_replicates_dwelling_commitment(
            &mut conn,
            &ctx(),
            CreateReplicatesDwellingInput {
                provider: provider.to_string(),
                receiver: "hub:R2".to_string(),
                payload: payload2,
                dht_anchor_hash: None,
            },
            &state2,
        );

        assert!(
            matches!(result, Err(CreateCommitmentError::Validation(_))),
            "ceiling breach must be rejected; got: {result:?}"
        );

        // Only the first commitment should be in DB.
        let commitments =
            active_commitments_for_provider(&mut conn, provider).expect("prioritizer query");
        assert_eq!(commitments.len(), 1, "only first pledge persisted");
    }

    // -----------------------------------------------------------------------
    // dht_anchor_hash backfill: writer accepts an optional anchor.
    // -----------------------------------------------------------------------

    /// T-ANCHOR: writer correctly stores a provided dht_anchor_hash.
    #[test]
    fn dht_anchor_hash_stored_when_provided() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let provider = "agent:uhCAkProvider-anchor";
        let payload = valid_payload("hub:P", "hub:Q", 5_000_000_000);
        let anchor = "uhCkk-mock-action-hash-abc123";

        let row = create_replicates_dwelling_commitment(
            &mut conn,
            &ctx(),
            CreateReplicatesDwellingInput {
                provider: provider.to_string(),
                receiver: "hub:Q".to_string(),
                payload,
                dht_anchor_hash: Some(anchor.to_string()),
            },
            &fresh_provider_state(),
        )
        .expect("create with anchor");

        assert_eq!(row.dht_anchor_hash.as_deref(), Some(anchor));

        // Prioritizer prefers dht_anchor_hash as the commitment_cid.
        let commitments =
            active_commitments_for_provider(&mut conn, provider).expect("prioritizer");
        assert_eq!(commitments[0].commitment_cid, anchor);
    }
}
