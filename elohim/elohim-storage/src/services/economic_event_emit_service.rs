//! Bounds-validated EconomicEvent emit service.
//!
//! # Purpose
//!
//! This is the **data-transaction gate** for the EPR acquisition Slice 2a.
//! Every notarised "I am providing this content" event is checked against its
//! commitment's bounds (the 7-check) BEFORE it reaches the conductor.  A
//! revoked or out-of-bounds event is refused at the source — the conductor is
//! never contacted.
//!
//! This keeps mutual hosting free/in-kind within reach (not PirateBay) and
//! lets ValueFlows hook into produced content with contributor attribution.
//!
//! # Binding strategy — `fulfills` ≠ `bounded_by`
//!
//! [`CreateReaEconomicEventInput`] has **no `bounded_by` field**.  Two distinct
//! bindings, driven by two distinct fields:
//!
//! 1. **`fulfills` (structural)** — `fulfills: vec![content_store_commitment_cid]`
//!    causes the coordinator zome to create `EventFulfillsCommitment` DHT links
//!    (content_store zome, ~:12124).  This is a COUNTERPARTY relationship: a
//!    content-store commitment the event settles, the on-chain link the VF
//!    resolver uses.  A **pure provide** (ProvideAnnounce, no counterparty) sets
//!    `content_store_commitment_cid = None` → `fulfills == []` (no link).
//! 2. **`bounded_by` (annotation)** — `metadata_json: {"bounded_by": commitment_cid}`
//!    always carries the Mishpat `commitment_cid` (the `replicates-commons` /
//!    `delegates-compute` bounds the event is gated against).  This is present
//!    for BOTH pure provides and counterparty exchanges — it is the bounds
//!    reference, not a fulfilment, and is independent of `fulfills`.
//!
//! # CommitmentFetcher production status
//!
//! [`ConductorCommitmentFetcher`] exists as a stub (Sprint 1 dependency: the
//! Mishpat::Commitment entry type and `get_commitment` coordinator function are
//! not yet wired).  Callers **inject the fetcher** so tests use
//! [`MockCommitmentFetcher`] and production callers can upgrade to the real
//! fetcher when Sprint 1 lands.  This is the same injection pattern used by
//! `bounds_validator.rs` and `republish_epr_validator.rs`.

use std::sync::Arc;

use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::services::bounds_validator::{self, BoundsViolation, EventForValidation};
use crate::services::commitment_fetcher::CommitmentFetcher;
use crate::services::conductor_writes;
use crate::services::rate_history::RateHistory;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// All caller-supplied parameters for a single emit.
///
/// The service derives `EventForValidation` from these fields and — on a
/// clean 7-check — builds `CreateReaEconomicEventInput` to send to the
/// conductor.
#[derive(Debug, Clone)]
pub struct EmitEconomicEventInput {
    /// Unique identifier for this economic event.
    pub id: String,
    /// REA action string, e.g. `"provide-content"`.
    pub action: String,
    /// Agent CID of the provider (performer of the action).
    pub provider: String,
    /// Agent CID or identifier of the receiver.
    pub receiver: String,
    /// ISO-8601 timestamp at which the event occurred.
    pub has_point_in_time: String,
    /// CID of the `Mishpat::Commitment` that bounds this event.
    pub commitment_cid: String,
    /// CID of the **content-store** commitment this event *fulfills*, if any.
    ///
    /// For a counterparty exchange this is `Some(cid)` and drives the structural
    /// `fulfills` binding (the coordinator zome creates `EventFulfillsCommitment`
    /// DHT links). For a **pure provide** (a ProvideAnnounce with no
    /// counterparty) this is `None` → `fulfills == []` (no spurious link). The
    /// `bounded_by` annotation always remains [`Self::commitment_cid`] (the
    /// Mishpat commitment), independent of this field.
    pub content_store_commitment_cid: Option<String>,
    /// EPR identifier targeted by this event.
    pub target_epr_id: String,
    /// Reach level claimed by the event (must be ≤ `bounds.reach_ceiling`).
    pub reach: String,
}

/// Error returned by [`emit`].
#[derive(Debug)]
pub enum EmitError {
    /// One of the 7 bounds checks failed.  The event was NOT sent to the
    /// conductor.
    Bounds(BoundsViolation),
    /// The conductor write failed after bounds checks passed.
    Conductor(StorageError),
}

impl std::fmt::Display for EmitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmitError::Bounds(v) => {
                write!(f, "bounds violation [{}]: {}", v.commitment_cid, v.summary)
            }
            EmitError::Conductor(e) => write!(f, "conductor error: {e}"),
        }
    }
}

impl std::error::Error for EmitError {}

// ---------------------------------------------------------------------------
// Pure helper — testable without a live conductor
// ---------------------------------------------------------------------------

/// Build the `CreateReaEconomicEventInput` from emit intent.
///
/// This is a pure function with no I/O: it can be unit-tested by calling it
/// directly without touching bounds validation or the conductor.  The emit
/// function calls it only after bounds checks pass.
pub fn build_event_input(
    input: &EmitEconomicEventInput,
) -> shefa_types::CreateReaEconomicEventInput {
    // `fulfills` carries the structural DHT binding: the coordinator zome
    // creates EventFulfillsCommitment links from these commitment IDs. A pure
    // provide has no counterparty content-store commitment, so fulfills is empty.
    let fulfills = input
        .content_store_commitment_cid
        .clone()
        .map(|c| vec![c])
        .unwrap_or_default();

    // `metadata_json` carries the human-readable annotation so diagnostics
    // and projection consumers can resolve the bounded_by reference without
    // traversing DHT links. bounded_by is ALWAYS the Mishpat commitment_cid,
    // independent of fulfills (which is the content-store counterparty, if any).
    let metadata_json = Some(serde_json::json!({"bounded_by": input.commitment_cid}).to_string());

    shefa_types::CreateReaEconomicEventInput {
        id: input.id.clone(),
        action: input.action.clone(),
        provider: input.provider.clone(),
        receiver: input.receiver.clone(),
        resource_classified_as: vec![],
        resource_quantity_value: None,
        resource_quantity_unit: None,
        effort_quantity_value: None,
        effort_quantity_unit: None,
        has_point_in_time: input.has_point_in_time.clone(),
        fulfills,
        realization_of: None,
        lamad_event_type: None,
        note: None,
        metadata_json,
        observation_refs: None,
    }
}

// ---------------------------------------------------------------------------
// Main service function
// ---------------------------------------------------------------------------

/// Emit a bounds-validated `EconomicEvent` through the conductor.
///
/// # Gate semantics
///
/// 1. Builds [`EventForValidation`] from `input`.
/// 2. Calls [`bounds_validator::validate`] — **on failure**, returns
///    `Err(EmitError::Bounds(..))` immediately; the conductor is never called.
/// 3. On validation pass, calls
///    [`conductor_writes::call_create_rea_economic_event`] and returns the
///    raw action-hash bytes from the conductor.
///
/// # CommitmentFetcher injection
///
/// `fetcher` is generic over [`CommitmentFetcher`]; callers inject:
/// - [`MockCommitmentFetcher`] in tests
/// - [`ConductorCommitmentFetcher`] (stub) in production until Sprint 1
///   wires the Mishpat zome
pub async fn emit<F: CommitmentFetcher, R: RateHistory>(
    input: &EmitEconomicEventInput,
    hc: &Arc<HcClient>,
    fetcher: &F,
    rate_history: &R,
) -> Result<Vec<u8>, EmitError> {
    // Step 1 — project to the narrow EventForValidation shape the validator needs.
    let event_for_validation = EventForValidation {
        action: input.action.clone(),
        performer: input.provider.clone(),
        bounded_by: input.commitment_cid.clone(),
        target_epr_id: input.target_epr_id.clone(),
        reach: input.reach.clone(),
        signed_at: input.has_point_in_time.clone(),
    };

    // Step 2 — run the 7-check BEFORE contacting the conductor.
    bounds_validator::validate(&event_for_validation, fetcher, rate_history)
        .await
        .map_err(EmitError::Bounds)?;

    // Step 3 — validation passed; build the conductor input and emit.
    let conductor_input = build_event_input(input);
    conductor_writes::call_create_rea_economic_event(hc, &conductor_input)
        .await
        .map_err(EmitError::Conductor)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::bounds_validator;
    use crate::services::commitment_fetcher::{CommitmentRecord, MockCommitmentFetcher};
    use crate::services::rate_history::MockRateHistory;
    use elohim_views::bounds::ViolationKind;

    /// Active commitment fixture that passes all 7 checks for the sample event.
    fn active_commitment() -> CommitmentRecord {
        CommitmentRecord {
            cid: "commitment-cid-abc".into(),
            action: "delegates-compute".into(),
            scope: "provide-content".into(),
            provider: "agent:provider-x".into(),
            recipient: "agent:receiver-y".into(),
            bounds: serde_json::json!({
                "epr_scope": ["epr:lamad-spa"],
                "reach_ceiling": "commons",
                "rate_per_hour": 30,
                "rotation_ttl_days": 90
            }),
            valid_from: "2026-05-01T00:00:00Z".into(),
            valid_until: "2026-08-01T00:00:00Z".into(),
            revoked_at: None,
        }
    }

    /// Emit intent fixture matching the active commitment.
    fn sample_input() -> EmitEconomicEventInput {
        EmitEconomicEventInput {
            id: "event-001".into(),
            action: "provide-content".into(),
            provider: "agent:provider-x".into(),
            receiver: "agent:receiver-y".into(),
            has_point_in_time: "2026-05-28T12:00:00Z".into(),
            commitment_cid: "commitment-cid-abc".into(),
            content_store_commitment_cid: Some("commitment-cid-abc".into()),
            target_epr_id: "epr:lamad-spa".into(),
            reach: "commons".into(),
        }
    }

    // -----------------------------------------------------------------------
    // T1: revoked commitment → bounds_validator returns CommitmentRevoked
    //     BEFORE any conductor interaction.
    //
    // The `emit` function calls `bounds_validator::validate` FIRST and returns
    // `Err(EmitError::Bounds(..))` on any violation, so the conductor is
    // structurally unreachable on the error path.  This test calls
    // `bounds_validator::validate` directly — the same call `emit` makes as
    // its first I/O action — to prove that a revoked commitment is refused
    // before any conductor call could occur.
    //
    // Using the validator directly avoids needing a live `HcClient` (which
    // requires an async websocket connection to a real conductor) while still
    // verifying the gate logic that `emit` relies on.
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn emit_refuses_revoked_commitment_before_conductor() {
        let mut c = active_commitment();
        c.revoked_at = Some("2026-05-15T00:00:00Z".into());

        let fetcher = MockCommitmentFetcher::new();
        fetcher.seed(&c.cid.clone(), c);
        let rate = MockRateHistory::new();

        // Build the same EventForValidation that `emit` would build from the
        // sample input — this is the exact first step inside `emit`.
        let input = sample_input();
        let event_for_validation = EventForValidation {
            action: input.action.clone(),
            performer: input.provider.clone(),
            bounded_by: input.commitment_cid.clone(),
            target_epr_id: input.target_epr_id.clone(),
            reach: input.reach.clone(),
            signed_at: input.has_point_in_time.clone(),
        };

        // Call validate — this is exactly what `emit` does before it would
        // ever contact the conductor.
        let result = bounds_validator::validate(&event_for_validation, &fetcher, &rate).await;

        match result {
            Err(v) => {
                assert_eq!(
                    v.kind,
                    ViolationKind::CommitmentRevoked,
                    "expected CommitmentRevoked, got {:?}",
                    v.kind
                );
                // Confirm the commitment_cid is carried through for diagnostics.
                assert_eq!(v.commitment_cid, "commitment-cid-abc");
            }
            Ok(_) => panic!("expected bounds violation for revoked commitment"),
        }
    }

    // -----------------------------------------------------------------------
    // T2: pure builder — fulfills + metadata_json carry the commitment binding
    //
    // Tests `build_event_input` directly (no I/O), asserting the two-pronged
    // binding strategy described in the module doc.
    // -----------------------------------------------------------------------
    #[test]
    fn emit_builds_fulfills_and_metadata() {
        let input = sample_input();
        let built = build_event_input(&input);

        // Structural binding: the coordinator zome creates
        // EventFulfillsCommitment DHT links from `fulfills`.
        assert_eq!(
            built.fulfills,
            vec!["commitment-cid-abc".to_string()],
            "fulfills must carry the commitment CID"
        );

        // Annotation binding: metadata_json carries bounded_by for diagnostics
        // and projection consumers.
        let meta_json = built.metadata_json.expect("metadata_json must be set");
        let meta: serde_json::Value =
            serde_json::from_str(&meta_json).expect("metadata_json must be valid JSON");
        assert_eq!(
            meta.get("bounded_by").and_then(|v| v.as_str()),
            Some("commitment-cid-abc"),
            "metadata_json must annotate bounded_by with the commitment CID"
        );

        // Scalar fields pass through unchanged.
        assert_eq!(built.id, input.id);
        assert_eq!(built.action, input.action);
        assert_eq!(built.provider, input.provider);
        assert_eq!(built.receiver, input.receiver);
        assert_eq!(built.has_point_in_time, input.has_point_in_time);
    }

    // -----------------------------------------------------------------------
    // T7: pure provide — content_store_commitment_cid=None ⇒ fulfills==[]
    //
    // A ProvideAnnounce has NO counterparty commitment to fulfill, but is still
    // bounded_by the replicates-commons Mishpat CID (metadata annotation). The
    // builder must produce an EMPTY fulfills (no spurious EventFulfillsCommitment
    // link) while keeping bounded_by = commitment_cid.
    // -----------------------------------------------------------------------
    #[test]
    fn pure_provide_has_empty_fulfills_but_keeps_bounded_by() {
        let mut input = sample_input();
        input.content_store_commitment_cid = None; // pure provide — no counterparty

        let built = build_event_input(&input);

        // No counterparty commitment ⇒ no EventFulfillsCommitment link.
        assert!(
            built.fulfills.is_empty(),
            "pure provide must have empty fulfills, got {:?}",
            built.fulfills
        );

        // bounded_by annotation still carries the Mishpat commitment CID.
        let meta_json = built.metadata_json.expect("metadata_json must be set");
        let meta: serde_json::Value =
            serde_json::from_str(&meta_json).expect("metadata_json must be valid JSON");
        assert_eq!(
            meta.get("bounded_by").and_then(|v| v.as_str()),
            Some("commitment-cid-abc"),
            "bounded_by must remain the Mishpat commitment_cid even for a pure provide"
        );

        // The built input must still be a structurally valid CreateReaEconomicEventInput
        // (scalar fields pass through unchanged).
        assert_eq!(built.id, input.id);
        assert_eq!(built.action, input.action);
        assert_eq!(built.provider, input.provider);
    }

    #[test]
    fn fulfilling_provide_carries_content_store_cid_in_fulfills() {
        let mut input = sample_input();
        input.content_store_commitment_cid = Some("content-store-cid-xyz".to_string());

        let built = build_event_input(&input);

        // A fulfilling event puts the content-store commitment in fulfills…
        assert_eq!(
            built.fulfills,
            vec!["content-store-cid-xyz".to_string()],
            "fulfills must carry the content_store_commitment_cid when present"
        );
        // …while bounded_by stays the Mishpat commitment_cid.
        let meta_json = built.metadata_json.expect("metadata_json must be set");
        let meta: serde_json::Value = serde_json::from_str(&meta_json).unwrap();
        assert_eq!(
            meta.get("bounded_by").and_then(|v| v.as_str()),
            Some("commitment-cid-abc")
        );
    }
}
