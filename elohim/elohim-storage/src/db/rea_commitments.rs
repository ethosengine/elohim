//! REA Commitment CRUD operations
//!
//! hREA/ValueFlows compatible commitment tracking for the learning economy.
//! Tracks binding promises of future economic activity (compute sharing,
//! content provision, assessment availability, etc.).

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use elohim_views::projection::{
    EprProjectionView, GateHintRef, ProjectionMode, RedirectTemplate, RouteClaimGrant,
    StewardDirectEndpoint,
};

use super::context::AppContext;
use super::diesel_schema::rea_commitments;
use super::models::{NewReaCommitment, ReaCommitment};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating an REA commitment.
///
/// `rename_all = "camelCase"` is REQUIRED: this struct is the direct
/// `parse_body` target for `POST /api/v1/commitments` (see
/// `api/rea_commitments.rs`), and HTTP bodies cross the boundary in camelCase
/// (per the storage CLAUDE.md boundary rule). Without it, camelCase keys like
/// `inScopeOf` / `metadataJson` silently fail to match the snake_case fields and
/// drop to `None` — which left `in_scope_of` null on every project-epr
/// projection, so `find_active_projections` (filters on `in_scope_of LIKE
/// 'doorway:X|%'`) matched nothing and `/lamad` 404'd even with projections
/// present. Single-word fields (action/provider/receiver/note) were unaffected,
/// which masked the bug. The seed's value shapes (string `inScopeOf`, string
/// `metadataJson`) already match this struct's `Option<String>` fields; only the
/// key casing was wrong.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateReaCommitmentInput {
    #[serde(default)]
    pub id: Option<String>,
    pub action: String,
    pub provider: String,
    pub receiver: String,
    #[serde(default)]
    pub resource_conforms_to: Option<String>,
    #[serde(default)]
    pub resource_classified_as: Option<String>,
    #[serde(default)]
    pub resource_quantity_value: Option<f32>,
    #[serde(default)]
    pub resource_quantity_unit: Option<String>,
    #[serde(default)]
    pub effort_quantity_value: Option<f32>,
    #[serde(default)]
    pub effort_quantity_unit: Option<String>,
    #[serde(default)]
    pub has_beginning: Option<String>,
    #[serde(default)]
    pub has_end: Option<String>,
    #[serde(default)]
    pub due: Option<String>,
    #[serde(default)]
    pub clause_of: Option<String>,
    #[serde(default)]
    pub in_scope_of: Option<String>,
    #[serde(default)]
    pub medium_of_exchange_id: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    /// Predecessor commitment id this create supersedes (spec §3.2/§3.3 re-grant
    /// path). When present on a project-epr create, the create runs the
    /// supersession ceremony: in one transaction the named predecessor is marked
    /// `state='superseded'` and the successor is inserted. Both rows remain
    /// notarized DHT entries; the `superseded` state is projection-layer
    /// current-view materialization per the stewardship-chain precedent
    /// (genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md §9 —
    /// "at most one non-superseded successor per predecessor; first-write-wins").
    /// This field is NOT persisted as a column; the steward-authored `supersedes`
    /// pointer rides the successor's `metadata_json` (spec §3.2 "grant is
    /// steward-authored commitment metadata"), keeping the chain walkable via
    /// `GET /api/v1/commitments/{id}` with no view change.
    #[serde(default)]
    pub supersedes: Option<String>,
}

/// Query parameters for listing REA commitments - camelCase for URL params
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaCommitmentQuery {
    /// Filter by action type
    pub action: Option<String>,
    /// Filter by provider agent
    pub provider: Option<String>,
    /// Filter by receiver agent
    pub receiver: Option<String>,
    /// Filter by commitment state
    pub state: Option<String>,
    /// Filter by agreement/clause
    pub clause_of: Option<String>,
    /// Filter by medium of exchange
    pub medium_of_exchange_id: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_limit() -> i64 {
    100
}

/// Input for updating commitment state
#[derive(Debug, Clone, Deserialize)]
pub struct UpdateReaCommitmentState {
    pub state: String,
    #[serde(default)]
    pub finished: Option<bool>,
    /// Optional metadata reconcile (already-serialized JSON string). When
    /// present, replaces metadata_json alongside the state write — heals rows
    /// whose create predated correct metadata persistence.
    #[serde(default)]
    pub metadata_json: Option<String>,
}

// ============================================================================
// Read Operations
// ============================================================================

/// Get commitment by ID - scoped by app
pub fn get_commitment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<ReaCommitment>, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// List commitments with filtering - scoped by app
pub fn list_commitments(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &ReaCommitmentQuery,
) -> Result<Vec<ReaCommitment>, StorageError> {
    let mut base_query = rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .into_boxed();

    if let Some(ref action) = query.action {
        base_query = base_query.filter(rea_commitments::action.eq(action));
    }

    if let Some(ref provider) = query.provider {
        base_query = base_query.filter(rea_commitments::provider.eq(provider));
    }

    if let Some(ref receiver) = query.receiver {
        base_query = base_query.filter(rea_commitments::receiver.eq(receiver));
    }

    if let Some(ref state) = query.state {
        base_query = base_query.filter(rea_commitments::state.eq(state));
    }

    if let Some(ref clause) = query.clause_of {
        base_query = base_query.filter(rea_commitments::clause_of.eq(clause));
    }

    if let Some(ref medium) = query.medium_of_exchange_id {
        base_query = base_query.filter(rea_commitments::medium_of_exchange_id.eq(medium));
    }

    base_query
        .order(rea_commitments::created_at.desc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get commitments for an agent (as provider or receiver)
pub fn get_commitments_for_agent(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    agent_id: &str,
    limit: i64,
) -> Result<Vec<ReaCommitment>, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(
            rea_commitments::provider
                .eq(agent_id)
                .or(rea_commitments::receiver.eq(agent_id)),
        )
        .order(rea_commitments::created_at.desc())
        .limit(limit)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Commitment states that count as an *active provide commitment* for the
/// resilience precondition. The seeder + reconcile path persist custody-blob
/// commitments as `"active"`; `"accepted"` / `"in-progress"` cover the
/// ValueFlows lifecycle states between proposal and fulfilment. Excludes
/// `"proposed"` (not yet committed), `"cancelled"`, `"terminated"`, `"finished"`.
const ACTIVE_PROVIDE_STATES: [&str; 3] = ["active", "accepted", "in-progress"];

/// Reach tag used for a provide commitment that carries no explicit reach scope.
///
/// Custody-blob commitments (the resilience mesh's bread and butter) store no
/// reach column — only `action="custody-blob"` and `resource_classified_as`
/// (a `sha256-…` blob hash). The resilience scenarios ingest *commons-reach*
/// content and the custody mesh hosts it, so hosting a blob == a `"commons"`
/// provide commitment. See `DeliveryPeer::commitments` for the full rationale.
const DEFAULT_PROVIDE_REACH: &str = "commons";

/// Return the distinct reach tags a peer/agent actively PROVIDES.
///
/// Used by `/api/v1/peers/delivery` to enrich each `DeliveryPeer.commitments`,
/// which the a2o resilience precondition tests as `commitments.includes(reach)`
/// (e.g. `"commons"`).
///
/// Selection: rows where `provider = provider_id`, `h_app_id` matches the app,
/// and `state ∈ {active, accepted, in-progress}`.
///
/// Reach derivation per row: if `in_scope_of` carries an explicit `reach:<class>`
/// scope, that `<class>` is used verbatim; otherwise the row contributes the
/// default `"commons"` reach (custody-of-a-commons-blob). The returned vector is
/// deduplicated and order-stable (first occurrence wins). Empty when the
/// provider has no active provide commitments.
pub fn active_provide_reaches(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    provider_id: &str,
) -> Result<Vec<String>, StorageError> {
    let rows: Vec<Option<String>> = rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::provider.eq(provider_id))
        .filter(rea_commitments::state.eq_any(ACTIVE_PROVIDE_STATES))
        .select(rea_commitments::in_scope_of)
        .load::<Option<String>>(conn)
        .map_err(|e| StorageError::Internal(format!("Provide-reach query failed: {}", e)))?;

    let mut seen = std::collections::HashSet::new();
    let reaches = rows
        .into_iter()
        .map(|scope| reach_from_scope(scope.as_deref()))
        .filter(|r| seen.insert(r.clone()))
        .collect();

    Ok(reaches)
}

/// Derive a reach tag from a commitment's `in_scope_of` value.
///
/// Recognises an explicit `reach:<class>` scope (single value or one element of
/// a `|`-separated scope string); otherwise falls back to [`DEFAULT_PROVIDE_REACH`].
fn reach_from_scope(scope: Option<&str>) -> String {
    let Some(scope) = scope else {
        return DEFAULT_PROVIDE_REACH.to_string();
    };
    scope
        .split('|')
        .find_map(|part| part.trim().strip_prefix("reach:"))
        .map(|class| class.to_string())
        .unwrap_or_else(|| DEFAULT_PROVIDE_REACH.to_string())
}

// ============================================================================
// Write Operations
// ============================================================================

/// Create an REA commitment - scoped by app
pub fn create_commitment(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateReaCommitmentInput,
) -> Result<ReaCommitment, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let new = NewReaCommitment {
        id: &id,
        h_app_id: &ctx.h_app_id,
        action: &input.action,
        provider: &input.provider,
        receiver: &input.receiver,
        resource_conforms_to: input.resource_conforms_to.as_deref(),
        resource_classified_as: input.resource_classified_as.as_deref(),
        resource_quantity_value: input.resource_quantity_value,
        resource_quantity_unit: input.resource_quantity_unit.as_deref(),
        effort_quantity_value: input.effort_quantity_value,
        effort_quantity_unit: input.effort_quantity_unit.as_deref(),
        has_beginning: input.has_beginning.as_deref(),
        has_end: input.has_end.as_deref(),
        due: input.due.as_deref(),
        clause_of: input.clause_of.as_deref(),
        in_scope_of: input.in_scope_of.as_deref(),
        medium_of_exchange_id: input.medium_of_exchange_id.as_deref(),
        state: "proposed",
        finished: 0,
        note: input.note.as_deref(),
        metadata_json: input.metadata_json.as_deref(),
        dht_anchor_hash: None,
    };

    diesel::insert_into(rea_commitments::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    get_commitment(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created commitment".into()))
}

/// Projection state assigned to a predecessor commitment when a successor
/// supersedes it. Excluded from the active-projection set by
/// `find_active_projections` so the doorway's `replace_all` table only ever
/// carries the current (non-superseded) grant.
///
/// This is projection-layer current-view materialization, NOT a DHT mutation:
/// both predecessor and successor remain notarized DHT entries. See the
/// stewardship-chain precedent
/// (genesis/docs/plans/2026-05-19-doorway-stewardship-chain-design.md §9).
pub const SUPERSEDED_STATE: &str = "superseded";

/// Validate that `input` may supersede the predecessor it names, given the
/// current projection state. Returns `Ok(())` when the supersede is admissible.
///
/// Rules (shared by the diesel-direct transactional path and the conductor
/// pre-flight check):
/// - `input.supersedes` must be set (else this is not a supersede; `Internal`).
/// - the successor id (`input.id`) must differ from the predecessor id.
/// - the predecessor must exist (`Validation` — phantom re-grant).
/// - the predecessor must NOT already be superseded — first-write-wins; the
///   second supersede is `AlreadyExists` (→ HTTP 409).
/// - the predecessor must share the successor's `(action, provider, in_scope_of)`
///   binding — a cross-scope/provider/action supersede is `Validation` (a
///   re-grant rewrites the SAME projection's grant, never hijacks another).
///
/// Read-only: performs no writes. The transactional path calls this then writes;
/// the conductor path calls this as a fail-fast pre-flight then again (inside
/// [`mark_superseded`]) to defend against a concurrent supersede.
pub fn validate_supersession(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateReaCommitmentInput,
) -> Result<(), StorageError> {
    let predecessor_id = input.supersedes.as_deref().ok_or_else(|| {
        StorageError::Internal("validate_supersession called without input.supersedes".into())
    })?;

    if let Some(successor_id) = input.id.as_deref() {
        if successor_id == predecessor_id {
            return Err(StorageError::Validation(format!(
                "successor id must differ from the predecessor it supersedes: {predecessor_id}"
            )));
        }
    }

    let predecessor = get_commitment(conn, ctx, predecessor_id)?.ok_or_else(|| {
        StorageError::Validation(format!(
            "cannot supersede unknown commitment: {predecessor_id}"
        ))
    })?;

    if predecessor.state == SUPERSEDED_STATE {
        return Err(StorageError::AlreadyExists(format!(
            "commitment {predecessor_id} is already superseded \
             (at most one non-superseded successor per predecessor)"
        )));
    }
    if predecessor.action != input.action {
        return Err(StorageError::Validation(format!(
            "supersede action mismatch: predecessor is '{}', successor is '{}'",
            predecessor.action, input.action
        )));
    }
    if predecessor.provider != input.provider {
        return Err(StorageError::Validation(format!(
            "supersede provider mismatch: predecessor provider '{}' != successor provider '{}'",
            predecessor.provider, input.provider
        )));
    }
    if predecessor.in_scope_of.as_deref() != input.in_scope_of.as_deref() {
        return Err(StorageError::Validation(format!(
            "supersede scope mismatch: predecessor in_scope_of {:?} != successor in_scope_of {:?}",
            predecessor.in_scope_of, input.in_scope_of
        )));
    }
    Ok(())
}

/// Flip a predecessor commitment's `state` to [`SUPERSEDED_STATE`], inside a
/// transaction that re-checks the not-already-superseded invariant.
///
/// Used by the conductor supersession path AFTER the successor has been
/// projected: the successor entry is already notarized, so the predecessor mark
/// is the final, idempotent step. The re-check guards against a concurrent
/// supersede landing between the pre-flight [`validate_supersession`] and here.
/// Marking an already-superseded predecessor is a no-op success (the mark is
/// idempotent — the desired end state is identical).
pub fn mark_superseded(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    predecessor_id: &str,
) -> Result<(), StorageError> {
    conn.transaction(|conn| {
        let predecessor = get_commitment(conn, ctx, predecessor_id)?.ok_or_else(|| {
            StorageError::Validation(format!(
                "cannot supersede unknown commitment: {predecessor_id}"
            ))
        })?;
        if predecessor.state == SUPERSEDED_STATE {
            // Idempotent: already in the desired end state.
            return Ok(());
        }
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                .filter(rea_commitments::id.eq(predecessor_id)),
        )
        .set(rea_commitments::state.eq(SUPERSEDED_STATE))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("mark-superseded failed: {e}")))?;
        Ok(())
    })
}

/// Create a successor project-epr commitment that supersedes a predecessor,
/// transactionally (spec §3.2/§3.3 re-grant ceremony).
///
/// In ONE transaction:
/// 1. The predecessor (`input.supersedes`) is validated:
///    - it must exist (else `Validation` → the grant references a phantom row),
///    - it must NOT itself already be superseded — the second supersede of the
///      same predecessor is rejected with `AlreadyExists` (→ HTTP 409),
///      satisfying first-write-wins ("at most one non-superseded successor per
///      predecessor"),
///    - it must share the successor's `(provider, in_scope_of)` binding — a
///      cross-scope or cross-provider supersede is a `Validation` rejection
///      (a re-grant rewrites the SAME projection's grant, never hijacks another).
/// 2. The predecessor's `state` is flipped to [`SUPERSEDED_STATE`].
/// 3. The successor row is inserted (state `"proposed"`, same as a plain create).
///
/// The successor's `metadata_json` SHOULD carry a steward-authored `supersedes`
/// pointer (spec §3.2) so the chain is walkable via `GET /api/v1/commitments/{id}`;
/// this function does not synthesize that pointer (the caller — seeder or
/// service — owns metadata authorship), but it does enforce the projection-state
/// invariants above.
///
/// Returns the freshly-inserted successor row. Both rows survive the transaction;
/// a rollback (any step failing) leaves the predecessor un-superseded and no
/// successor inserted — the projection cannot land in a half-superseded state.
pub fn create_with_supersession(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateReaCommitmentInput,
) -> Result<ReaCommitment, StorageError> {
    let predecessor_id = input.supersedes.clone().ok_or_else(|| {
        StorageError::Internal(
            "create_with_supersession called without input.supersedes set".into(),
        )
    })?;

    let successor_id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    conn.transaction(|conn| {
        // Single source of supersession-admissibility truth (exists,
        // not-already-superseded → 409, scope/provider/action match, distinct id).
        validate_supersession(conn, ctx, &input)?;

        // Mark the predecessor superseded.
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                .filter(rea_commitments::id.eq(&predecessor_id)),
        )
        .set(rea_commitments::state.eq(SUPERSEDED_STATE))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("mark-superseded failed: {e}")))?;

        // Insert the successor (state="proposed", like a plain create).
        let new = NewReaCommitment {
            id: &successor_id,
            h_app_id: &ctx.h_app_id,
            action: &input.action,
            provider: &input.provider,
            receiver: &input.receiver,
            resource_conforms_to: input.resource_conforms_to.as_deref(),
            resource_classified_as: input.resource_classified_as.as_deref(),
            resource_quantity_value: input.resource_quantity_value,
            resource_quantity_unit: input.resource_quantity_unit.as_deref(),
            effort_quantity_value: input.effort_quantity_value,
            effort_quantity_unit: input.effort_quantity_unit.as_deref(),
            has_beginning: input.has_beginning.as_deref(),
            has_end: input.has_end.as_deref(),
            due: input.due.as_deref(),
            clause_of: input.clause_of.as_deref(),
            in_scope_of: input.in_scope_of.as_deref(),
            medium_of_exchange_id: input.medium_of_exchange_id.as_deref(),
            state: "proposed",
            finished: 0,
            note: input.note.as_deref(),
            metadata_json: input.metadata_json.as_deref(),
            dht_anchor_hash: None,
        };
        diesel::insert_into(rea_commitments::table)
            .values(&new)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("successor insert failed: {e}")))?;

        get_commitment(conn, ctx, &successor_id)?
            .ok_or_else(|| StorageError::Internal("failed to retrieve inserted successor".into()))
    })
}

/// Update commitment state
pub fn update_commitment_state(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    update: &UpdateReaCommitmentState,
) -> Result<ReaCommitment, StorageError> {
    let finished_val = update.finished.map(|b| if b { 1 } else { 0 });

    if let Some(f) = finished_val {
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                .filter(rea_commitments::id.eq(id)),
        )
        .set((
            rea_commitments::state.eq(&update.state),
            rea_commitments::finished.eq(f),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else {
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                .filter(rea_commitments::id.eq(id)),
        )
        .set(rea_commitments::state.eq(&update.state))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    }

    // Optional metadata reconcile — separate statement keeps the state-write
    // combinations simple (SQLite local writes; two statements are fine).
    if let Some(ref mj) = update.metadata_json {
        diesel::update(
            rea_commitments::table
                .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                .filter(rea_commitments::id.eq(id)),
        )
        .set(rea_commitments::metadata_json.eq(mj))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Metadata update failed: {}", e)))?;
    }

    get_commitment(conn, ctx, id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve updated commitment".into()))
}

/// Upsert a commitment with DHT anchor hash — insert or update anchor if it already exists.
/// Used by REA projection signal handler to anchor commitments onto the DHT.
pub fn upsert_with_anchor(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: CreateReaCommitmentInput,
    dht_anchor_hash: Option<&str>,
) -> Result<ReaCommitment, StorageError> {
    let id = input.id.unwrap_or_else(|| Uuid::new_v4().to_string());

    let existing = get_commitment(conn, ctx, &id)?;

    if existing.is_some() {
        // On reseed of an existing id, refresh the projected columns the
        // EprRouter reads (in_scope_of / note / metadata_json) IN ADDITION to
        // dht_anchor_hash — but ONLY when the input carries them
        // (in_scope_of.is_some() distinguishes a full create input from a
        // state-only anchor update). This backfills rows authored before the
        // camelCase-deserialization fix (57bf7d672) landed in the deployed
        // storage binary, which persisted in_scope_of as NULL. Because
        // find_active_projections filters `in_scope_of LIKE '%doorway:..|%'`
        // and `NULL LIKE _` is never true, those rows were invisible → the
        // doorway EprRouter stayed empty → '/' fell to /threshold and '/lamad'
        // 404'd. A plain reseed could not self-heal because this branch
        // previously touched only dht_anchor_hash. The guard ensures the
        // state-only `update_state_via_conductor` path (in_scope_of=None) never
        // clobbers an existing scope.
        if input.in_scope_of.is_some() {
            diesel::update(
                rea_commitments::table
                    .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                    .filter(rea_commitments::id.eq(&id)),
            )
            .set((
                rea_commitments::dht_anchor_hash.eq(dht_anchor_hash),
                rea_commitments::in_scope_of.eq(input.in_scope_of.as_deref()),
                rea_commitments::note.eq(input.note.as_deref()),
                rea_commitments::metadata_json.eq(input.metadata_json.as_deref()),
            ))
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
        } else {
            // State-only anchor update: advance dht_anchor_hash, preserve the rest.
            diesel::update(
                rea_commitments::table
                    .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
                    .filter(rea_commitments::id.eq(&id)),
            )
            .set(rea_commitments::dht_anchor_hash.eq(dht_anchor_hash))
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Update anchor failed: {}", e)))?;
        }
    } else {
        let new = NewReaCommitment {
            id: &id,
            h_app_id: &ctx.h_app_id,
            action: &input.action,
            provider: &input.provider,
            receiver: &input.receiver,
            resource_conforms_to: input.resource_conforms_to.as_deref(),
            resource_classified_as: input.resource_classified_as.as_deref(),
            resource_quantity_value: input.resource_quantity_value,
            resource_quantity_unit: input.resource_quantity_unit.as_deref(),
            effort_quantity_value: input.effort_quantity_value,
            effort_quantity_unit: input.effort_quantity_unit.as_deref(),
            has_beginning: input.has_beginning.as_deref(),
            has_end: input.has_end.as_deref(),
            due: input.due.as_deref(),
            clause_of: input.clause_of.as_deref(),
            in_scope_of: input.in_scope_of.as_deref(),
            medium_of_exchange_id: input.medium_of_exchange_id.as_deref(),
            state: "proposed",
            finished: 0,
            note: input.note.as_deref(),
            metadata_json: input.metadata_json.as_deref(),
            dht_anchor_hash,
        };

        diesel::insert_into(rea_commitments::table)
            .values(&new)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    get_commitment(conn, ctx, &id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve upserted commitment".into()))
}

// ============================================================================
// Stats
// ============================================================================

/// Get commitment count for an app
pub fn commitment_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

// ============================================================================
// Doorway-Operator Authority Selectors
// ============================================================================
//
// Operator authority for a doorway is a Commitment with action='operate-doorway'
// and in_scope_of='doorway:<id>'. The capabilities array lives in
// resource_classified_as (JSON-encoded list of strings); succession-role and
// reach-scope live in metadata_json. See:
//   - elohim/sdk/schemas/v1/objects/operator-classification.schema.json
//   - elohim/sdk/schemas/v1/views/doorway-operator-binding-view.schema.json
//
// Source of truth: DHT Commitment entry (Notarized, Category A). These
// selectors read the projection only — re-derivable by replaying the signal
// stream filtered to action='operate-doorway'.
//
// Fast-path queries use the (action, in_scope_of, state) composite index from
// migration 2026-05-19-000000_doorway_operator_action_indexes.

/// REA action discriminator for doorway-operator commitments. Single source of
/// truth for this string in the storage crate; the same constant in the DNA's
/// REA_ACTIONS array is the wire-contract value (kept in sync by convention,
/// pending schema-first codegen of the action vocabulary).
pub const OPERATE_DOORWAY_ACTION: &str = "operate-doorway";

/// REA action discriminator for pillar-EPR projection commitments. Single
/// source of truth for this string in the storage crate; mirrors the
/// OPERATE_DOORWAY_ACTION pattern — a new action value on the existing
/// Commitment infrastructure, not a new entry type.
///
/// Notarizes which doorway projects which EPR at what URL path. The action
/// string is content-addressed into commitment ids; changing it breaks
/// idempotency of every existing seed.
pub const PROJECT_EPR_ACTION: &str = "project-epr";

/// Actions that round-trip the conductor when one is connected (anchored,
/// gossiped) but degrade gracefully to diesel-direct when not. custody-blob
/// joined 2026-06-04 (household formation spec §5.3) — it is in the elohim
/// DNA's REA_ACTIONS vocabulary, so content_store::create_rea_commitment
/// accepts it. Mishpat-discriminated actions (delegates-compute,
/// replicates-dwelling) are NOT here: they need a mishpat-role client
/// (stage-2 work).
pub const CONDUCTOR_SOFT_ACTIONS: &[&str] = &["custody-blob"];

/// Build the canonical in_scope_of value for an operate-doorway commitment.
///
/// Stored encoding is a JSON-array string (ValueFlows in_scope_of convention),
/// matching what the DNA projects into rea_commitments.in_scope_of verbatim
/// from the Commitment entry's in_scope_of_json field. Operator commitments are
/// always single-scope, so the resulting string is a one-element JSON array
/// that can be matched exactly via the composite index from migration
/// 2026-05-19-000000_doorway_operator_action_indexes.
pub fn doorway_scope(doorway_id: &str) -> String {
    serde_json::to_string(&[format!("doorway:{}", doorway_id)])
        .expect("serializing a single-element string array cannot fail")
}

// ============================================================================
// Project-EPR Commitment Validator
// ============================================================================
//
// Enforces opinionated substrate constraints on project-epr commitments per
// spec §2.4 ("no dead ends" guarantee). Called at HTTP request-time, BEFORE
// the commitment row is built and stored, so invalid projections are rejected
// at the boundary rather than silently persisted.
//
// The validator operates on ProjectEprValidationInput — a focused subset of
// EprProjectionView — so callers do not need to construct a full view
// (which requires additional fields from the DB) before validating.

/// Policy-relevant subset of EprProjectionView fields for pre-storage validation.
///
/// Deliberately minimal: only the fields that the validator rules (§2.4) need.
/// This allows validation of incoming HTTP requests before the full commitment
/// row is constructed.
#[derive(Debug, Clone)]
pub struct ProjectEprValidationInput {
    /// URL path this projection is served from (e.g. "/lamad").
    pub url_path: String,
    /// How the doorway serves this projection.
    pub mode: ProjectionMode,
    /// Reach class of the projected EPR atom.
    pub reach: String,
    /// EPR CID of a preview / draft build (optional).
    pub preview_epr_ref: Option<String>,
    /// Gate hints the access layer should surface when reach is restricted.
    pub gate_hints: Vec<GateHintRef>,
    /// True when this projection has no onward link — a terminal destination.
    pub dead_end: bool,
    /// Steward-direct endpoint, required when mode is StewardDirect.
    pub steward_direct_endpoint: Option<StewardDirectEndpoint>,
    /// Mount-level bare aliases (legacy urlPaths that 302 to url_path).
    pub redirects_from: Vec<String>,
    /// Route-level alias templates (spec §4).
    pub redirect_templates: Vec<RedirectTemplate>,
    /// Granted claims (spec §3.2) — present for claim-vocabulary validation.
    pub route_claims: Option<RouteClaimGrant>,
}

/// Validate a project-epr commitment per the substrate rules (spec §2.4).
///
/// Returns `Ok(())` when the commitment is safe to persist. Returns
/// `Err(StorageError::Validation(_))` with a human-readable message when any
/// rule is violated. The rules are:
///
/// 1. Non-commons reach requires `previewEprRef` OR `gateHints` (non-empty)
///    OR `deadEnd=true` — at least one onward path for the person.
/// 2. `stewardDirect` mode requires `stewardDirectEndpoint`.
/// 3. `urlPath` must start with `/`.
/// 4. `urlPath` must not have trailing slash unless it IS `/`.
/// 5. (spec §4) aliases (`redirects_from` + `redirect_templates[].from`) must
///    start with `/` and may never collide with a reserved service prefix.
/// 6. (spec §4) a `redirect_templates[].to` target must be canonical: the
///    universal `/epr/…` floor, or this projection's mount.
///
/// Cross-commitment claim uniqueness (spec §3.3, rule 7) is NOT enforced here:
/// it needs a per-doorway DB query of existing grants, which the pure validator
/// has no connection for. It is deferred to the doorway's `replace_all` index
/// build (Task 8), which warns deterministically on a claim conflict.
pub fn validate_project_epr_commitment(
    input: &ProjectEprValidationInput,
) -> Result<(), StorageError> {
    // Rule 1: non-commons reach must declare at least one path forward
    if input.reach != "commons"
        && input.preview_epr_ref.is_none()
        && input.gate_hints.is_empty()
        && !input.dead_end
    {
        return Err(StorageError::Validation(
            "Gated projection must declare at least one of: \
             previewEprRef, gateHints (non-empty), or deadEnd=true"
                .into(),
        ));
    }

    // Rule 2: steward-direct mode requires an endpoint
    if input.mode == ProjectionMode::StewardDirect && input.steward_direct_endpoint.is_none() {
        return Err(StorageError::Validation(
            "steward-direct mode requires stewardDirectEndpoint".into(),
        ));
    }

    // Rule 3: url_path must start with /
    if !input.url_path.starts_with('/') {
        return Err(StorageError::Validation(format!(
            "urlPath must start with '/', got: {}",
            input.url_path
        )));
    }

    // Rule 4: url_path must not have trailing slash unless it IS "/"
    if input.url_path.len() > 1 && input.url_path.ends_with('/') {
        return Err(StorageError::Validation(format!(
            "urlPath must not have trailing slash (except '/'): {}",
            input.url_path
        )));
    }

    // Rule 5 (spec §4): aliases may never collide with a reserved service
    // prefix. The reserved list is pinned by the shared route-claims fixture
    // (two-layer guard with doorway's is_service_path).
    const RESERVED_URL_PREFIXES: &[&str] = &[
        "/epr",
        "/epr-head",
        "/db",
        "/api",
        "/blob",
        "/apps",
        "/status",
        "/health",
        "/admin",
        "/identity",
        "/threshold",
        "/sitemap.xml",
    ];
    let alias_paths = input
        .redirects_from
        .iter()
        .cloned()
        .chain(input.redirect_templates.iter().map(|t| t.from.clone()));
    for alias in alias_paths {
        if !alias.starts_with('/') {
            return Err(StorageError::Validation(format!(
                "alias must start with '/', got: {alias}"
            )));
        }
        if RESERVED_URL_PREFIXES
            .iter()
            .any(|p| alias == *p || alias.starts_with(&format!("{p}/")))
        {
            return Err(StorageError::Validation(format!(
                "alias collides with a reserved service prefix: {alias}"
            )));
        }
    }

    // Rule 6 (spec §4): one hop — a redirect template's target must be a
    // canonical address: the universal /epr floor or this projection's mount.
    for t in &input.redirect_templates {
        let canonical = t.to.starts_with("/epr/")
            || t.to == input.url_path
            || t.to.starts_with(&format!("{}/", input.url_path));
        if !canonical {
            return Err(StorageError::Validation(format!(
                "redirect template target must be canonical (/epr/… or the mount), got: {}",
                t.to
            )));
        }
    }

    Ok(())
}

/// List active operator commitments for a doorway.
///
/// Returns every Commitment with action='operate-doorway', in_scope_of matching
/// the doorway scope, and state='active'. Each row is a distinct operator
/// binding (one row per operator-agent × doorway). The doorway's auth layer
/// uses this at JWT refresh time to rebuild the capabilities snapshot.
pub fn list_active_doorway_operators(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    doorway_id: &str,
) -> Result<Vec<ReaCommitment>, StorageError> {
    let scope = doorway_scope(doorway_id);
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::action.eq(OPERATE_DOORWAY_ACTION))
        .filter(rea_commitments::in_scope_of.eq(&scope))
        .filter(rea_commitments::state.eq("active"))
        .order(rea_commitments::created_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Operator query failed: {}", e)))
}

/// Look up the active operator-binding for one agent on one doorway.
///
/// Returns at most one Commitment (the most-recent active binding). Callers
/// MUST still verify the capability is present in the binding's
/// resource_classified_as list before authorizing the operation. This selector
/// is the substrate lookup; the capability membership check belongs to the
/// auth layer (so capability vocabulary changes do not require a DB schema
/// change).
pub fn find_active_operator_binding(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    operator_agent: &str,
    doorway_id: &str,
) -> Result<Option<ReaCommitment>, StorageError> {
    let scope = doorway_scope(doorway_id);
    rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::action.eq(OPERATE_DOORWAY_ACTION))
        .filter(rea_commitments::in_scope_of.eq(&scope))
        .filter(rea_commitments::state.eq("active"))
        .filter(rea_commitments::provider.eq(operator_agent))
        .order(rea_commitments::created_at.desc())
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Operator lookup failed: {}", e)))
}

// ============================================================================
// Project-EPR Projection Resolvers
// ============================================================================
//
// These selectors operate on project-epr commitments, projecting stored
// commitment rows into EprProjectionView values. The scope format used by
// project-epr commitments is `doorway:{doorway_id}|epr:{epr_id}` — a
// pipe-separated pair that encodes both the target doorway and the EPR being
// projected. This differs from operate-doorway commitments, which use a
// JSON-array scope (`["doorway:{id}"]`).
//
// "Active" for projection commitments means state is not "cancelled" or
// "terminated" — projections are seeded as "proposed" by the bootstrap flow
// and graduate to "active" once confirmed. The resolver accepts any
// non-cancelled state so projections are queryable during the window between
// seeding and DHT notarization.
//
// Source of truth: DHT Commitment entry (action="project-epr"). These
// selectors read the SQLite projection only.

/// Find all active project-epr commitments scoped to a specific doorway.
///
/// "Active" means state is not "cancelled", "terminated", or "superseded".
/// Returns all projections for the given doorway across the remaining states,
/// so the access layer can resolve URL paths from the freshly-seeded set before
/// DHT notarization completes. A superseded predecessor (re-grant ceremony,
/// spec §3.2/§3.3) is excluded so only the current grant serves.
///
/// The `doorway_id` parameter is the SHORT form (e.g. "alpha-elohim-host");
/// the scope column stores `doorway:alpha-elohim-host|epr:{epr_id}`. The
/// returned `EprProjectionView.doorway_id` is the LONG form
/// (`doorway:alpha-elohim-host`).
///
/// NOTE: the `in_scope_of LIKE` filter excludes rows with `in_scope_of IS NULL`
/// (`NULL LIKE _` is never true). Rows seeded before the camelCase-deserialization
/// fix landed in the deployed binary persisted NULL scope and were invisible here,
/// emptying the doorway EprRouter. `upsert_with_anchor` now backfills `in_scope_of`
/// on reseed (see its existing-row branch), so re-running the genesis projection
/// seed against a fixed binary repairs those legacy rows.
pub fn find_active_projections(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    doorway_id: &str,
) -> Result<Vec<EprProjectionView>, StorageError> {
    let scope_filter = format!("%doorway:{}|%", doorway_id);

    let commitments: Vec<ReaCommitment> = rea_commitments::table
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        .filter(rea_commitments::action.eq(PROJECT_EPR_ACTION))
        .filter(rea_commitments::in_scope_of.like(&scope_filter))
        .filter(rea_commitments::state.ne("cancelled"))
        .filter(rea_commitments::state.ne("terminated"))
        // Re-grant supersession (spec §3.2/§3.3): a superseded predecessor is no
        // longer the operative routing law — only the current (non-superseded)
        // successor serves. See create_with_supersession + SUPERSEDED_STATE.
        .filter(rea_commitments::state.ne(SUPERSEDED_STATE))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("DB error: {}", e)))?;

    // Per-row degradation, never whole-set failure (incident #2, 2026-06-07):
    // a single poisoned scope previously turned this collect() into Err, the
    // doorway's projection fetch yielded nothing, and the EprRouter served an
    // EMPTY table — '/' fell to /threshold and every pillar mount 404'd. One
    // malformed row must cost exactly one projection, not the router.
    Ok(commitments
        .into_iter()
        .filter_map(|c| {
            let row_id = c.id.clone();
            match commitment_to_projection_view(c) {
                Ok(view) => Some(view),
                Err(e) => {
                    tracing::warn!(
                        commitment_id = %row_id,
                        error = %e,
                        "Skipping unparseable project-epr row; serving the remaining projections"
                    );
                    None
                }
            }
        })
        .collect())
}

/// Find the project-epr commitment whose urlPath is the longest prefix
/// of the requested path on the given doorway.
///
/// Implements longest-prefix semantics: if both "/" and "/lamad" are
/// registered, a request for "/lamad/concept/foo" resolves to "/lamad".
/// Returns `None` when no registered projection matches the path.
pub fn find_projection_by_url_path(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    doorway_id: &str,
    request_path: &str,
) -> Result<Option<EprProjectionView>, StorageError> {
    let all = find_active_projections(conn, ctx, doorway_id)?;
    let best = all
        .into_iter()
        .filter(|p| path_matches_prefix(request_path, &p.url_path))
        .max_by_key(|p| p.url_path.len());
    Ok(best)
}

/// Returns true when `request_path` is at or under `projection_path`.
///
/// Rules:
/// - projection_path "/" matches everything.
/// - projection_path "/lamad" matches "/lamad", "/lamad/", "/lamad/foo"
///   but NOT "/lamadx" or "/lam".
fn path_matches_prefix(request_path: &str, projection_path: &str) -> bool {
    if projection_path == "/" {
        return true;
    }
    request_path == projection_path || request_path.starts_with(&format!("{}/", projection_path))
}

/// Convert a stored `ReaCommitment` row into an `EprProjectionView`.
///
/// The commitment's `metadata_json` field holds the projection details
/// (urlPath, mode, reach, etc.). Missing fields fall back to safe defaults
/// so the access layer degrades gracefully on partially-migrated seeds.
fn commitment_to_projection_view(c: ReaCommitment) -> Result<EprProjectionView, StorageError> {
    let metadata: serde_json::Value = c
        .metadata_json
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(|e| StorageError::Internal(format!("metadata parse: {}", e)))?
        .unwrap_or(serde_json::Value::Null);

    let scope = c.in_scope_of.unwrap_or_default();
    let (doorway_id, epr_id) = parse_projection_scope(&scope)?;

    Ok(EprProjectionView {
        commitment_id: c.id,
        epr_id,
        doorway_id,
        url_path: metadata
            .get("urlPath")
            .and_then(|v| v.as_str())
            .unwrap_or("/")
            .to_string(),
        mode: metadata
            .get("mode")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or(ProjectionMode::Cached),
        reach: metadata
            .get("reach")
            .and_then(|v| v.as_str())
            .unwrap_or("commons")
            .to_string(),
        base_href: metadata
            .get("baseHref")
            .and_then(|v| v.as_str())
            .unwrap_or("/")
            .to_string(),
        entry_file: metadata
            .get("entryFile")
            .and_then(|v| v.as_str())
            .unwrap_or("index.html")
            .to_string(),
        // Defaults true (bundle-EPR SPA deep-link fallback) when the seed omits
        // the field — matches the serde default on EprProjectionView and the
        // schema default. See spec §12.2.
        spa_fallback: metadata
            .get("spaFallback")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        redirects_from: metadata
            .get("redirectsFrom")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        redirect_templates: metadata
            .get("redirectTemplates")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        route_claims: metadata.get("routeClaims").and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v.clone()).ok()
            }
        }),
        preview_epr_ref: metadata
            .get("previewEprRef")
            .and_then(|v| v.as_str())
            .map(String::from),
        gate_hints: metadata
            .get("gateHints")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default(),
        dead_end: metadata
            .get("deadEnd")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        steward_direct_endpoint: metadata.get("stewardDirectEndpoint").and_then(|v| {
            if v.is_null() {
                None
            } else {
                serde_json::from_value(v.clone()).ok()
            }
        }),
        seeded_at: c.created_at.to_string(),
        seeded_by: c.provider,
    })
}

/// Parse a `doorway:{id}|epr:{id}` scope string into its two components.
///
/// Returns `(doorway_id_long_form, epr_id)` where the doorway_id includes
/// the "doorway:" prefix (e.g. `"doorway:alpha-elohim-host"`).
///
/// Heals two known LEGACY shapes before parsing (incident #2, 2026-06-07 —
/// the alpha EprRouter wipe-out; incident #1 was the NULL-scope era, see the
/// `find_active_projections` NOTE):
///
/// - `["doorway:X|epr:Y"]` — a stale storage binary (pre-43951281f) replayed
///   the zome's `in_scope_of_json` (always a JSON array) verbatim into the
///   column instead of unwrapping via `first_or_none`.
/// - `["doorway:X","epr:Y"]` — the pre-66f16ab5e seeder sent the two scope
///   refs as separate array elements.
///
/// New writes are canonical bare pipe-strings; this tolerance exists so a
/// legacy row degrades to a served projection instead of a parse error.
fn parse_projection_scope(scope: &str) -> Result<(String, String), StorageError> {
    let trimmed = scope.trim();
    if trimmed.starts_with('[') {
        let items: Vec<String> = serde_json::from_str(trimmed).map_err(|e| {
            StorageError::Internal(format!(
                "Malformed projection scope (bad JSON array): {} ({})",
                scope, e
            ))
        })?;
        let healed = match items.as_slice() {
            [single] => single.clone(),
            [doorway, epr] => format!("{}|{}", doorway, epr),
            _ => {
                return Err(StorageError::Internal(format!(
                    "Malformed projection scope (array of {}): {}",
                    items.len(),
                    scope
                )))
            }
        };
        return parse_projection_scope(&healed);
    }

    let parts: Vec<&str> = scope.split('|').collect();
    if parts.len() != 2 {
        return Err(StorageError::Internal(format!(
            "Malformed projection scope: {}",
            scope
        )));
    }
    let doorway_raw = parts[0]
        .strip_prefix("doorway:")
        .ok_or_else(|| {
            StorageError::Internal(format!("Scope missing 'doorway:' prefix: {}", scope))
        })?
        .to_string();
    let epr_id = parts[1]
        .strip_prefix("epr:")
        .ok_or_else(|| StorageError::Internal(format!("Scope missing 'epr:' prefix: {}", scope)))?
        .to_string();
    Ok((format!("doorway:{}", doorway_raw), epr_id))
}

#[cfg(test)]
mod provide_reach_tests {
    use super::*;
    use crate::db::models::NewReaCommitment;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    /// Insert a commitment row with explicit control over state + in_scope_of —
    /// `create_commitment` always forces state="proposed", which these tests
    /// need to bypass to exercise the active-state filter.
    #[allow(clippy::too_many_arguments)]
    fn insert(
        conn: &mut SqliteConnection,
        ctx: &AppContext,
        id: &str,
        provider: &str,
        state: &str,
        in_scope_of: Option<&str>,
    ) {
        let row = NewReaCommitment {
            id,
            h_app_id: &ctx.h_app_id,
            action: "custody-blob",
            provider,
            receiver: "receiver-peer",
            resource_conforms_to: Some("blob"),
            resource_classified_as: Some("sha256-deadbeef"),
            resource_quantity_value: Some(1024.0),
            resource_quantity_unit: Some("B"),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of,
            medium_of_exchange_id: None,
            state,
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
        };
        diesel::insert_into(rea_commitments::table)
            .values(&row)
            .execute(conn)
            .expect("insert commitment");
    }

    /// Seeded active custody-blob (no scope) → "commons" reach exposed.
    #[test]
    fn active_custody_blob_yields_commons() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        insert(&mut conn, &ctx, "c1", "peer-A", "active", None);

        let reaches = active_provide_reaches(&mut conn, &ctx, "peer-A").unwrap();
        assert_eq!(reaches, vec!["commons"]);
    }

    /// Unknown / non-providing peer → empty.
    #[test]
    fn unknown_provider_yields_empty() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        insert(&mut conn, &ctx, "c1", "peer-A", "active", None);

        let reaches = active_provide_reaches(&mut conn, &ctx, "peer-NOBODY").unwrap();
        assert!(reaches.is_empty());
    }

    /// `proposed` (and other inactive) states are excluded from the active set.
    #[test]
    fn proposed_state_excluded() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        insert(&mut conn, &ctx, "c1", "peer-A", "proposed", None);
        insert(&mut conn, &ctx, "c2", "peer-A", "cancelled", None);

        let reaches = active_provide_reaches(&mut conn, &ctx, "peer-A").unwrap();
        assert!(reaches.is_empty(), "non-active states must not contribute");
    }

    /// `accepted` and `in-progress` count as active provide states.
    #[test]
    fn accepted_and_in_progress_count() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        insert(&mut conn, &ctx, "c1", "peer-A", "accepted", None);
        insert(&mut conn, &ctx, "c2", "peer-B", "in-progress", None);

        assert_eq!(
            active_provide_reaches(&mut conn, &ctx, "peer-A").unwrap(),
            vec!["commons"]
        );
        assert_eq!(
            active_provide_reaches(&mut conn, &ctx, "peer-B").unwrap(),
            vec!["commons"]
        );
    }

    /// Explicit `reach:<class>` scope overrides the default and is deduplicated.
    #[test]
    fn explicit_reach_scope_used_and_deduped() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        // Two rows with the SAME explicit reach → one distinct tag.
        insert(
            &mut conn,
            &ctx,
            "c1",
            "peer-A",
            "active",
            Some("reach:household"),
        );
        insert(
            &mut conn,
            &ctx,
            "c2",
            "peer-A",
            "active",
            Some("doorway:alpha|reach:household"),
        );
        // A third row with default (no scope) → adds "commons".
        insert(&mut conn, &ctx, "c3", "peer-A", "active", None);

        let mut reaches = active_provide_reaches(&mut conn, &ctx, "peer-A").unwrap();
        reaches.sort();
        assert_eq!(reaches, vec!["commons", "household"]);
    }

    /// App scoping: a commitment under a different h_app_id is not visible.
    #[test]
    fn app_scoped() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        let other = AppContext::new("other-app");
        insert(&mut conn, &other, "c1", "peer-A", "active", None);

        let reaches = active_provide_reaches(&mut conn, &ctx, "peer-A").unwrap();
        assert!(reaches.is_empty(), "other-app commitment must be invisible");
    }

    #[test]
    fn reach_from_scope_variants() {
        assert_eq!(reach_from_scope(None), "commons");
        assert_eq!(reach_from_scope(Some("reach:qahal")), "qahal");
        assert_eq!(reach_from_scope(Some("doorway:x|reach:family")), "family");
        // No reach: marker → default.
        assert_eq!(reach_from_scope(Some("doorway:x|epr:y")), "commons");
        assert_eq!(reach_from_scope(Some("")), "commons");
    }
}

#[cfg(test)]
mod projection_resolver_tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    /// Seed 4 test projections: two on alpha-elohim-host ("/", "/lamad")
    /// and two on elohim-host ("/", "/lamad").
    fn seed_test_projections(conn: &mut SqliteConnection, ctx: &AppContext) {
        let test_seed = |conn: &mut SqliteConnection, epr: &str, doorway: &str, url_path: &str| {
            let scope = format!("doorway:{}|epr:{}", doorway, epr);
            let id_safe = format!("{}-{}-{}", epr, doorway, url_path).replace('/', "_");

            let metadata = serde_json::json!({
                "urlPath": url_path,
                "mode": "cached",
                "reach": "commons",
                "baseHref": if url_path == "/" { "/".to_string() } else { format!("{}/", url_path) },
                "entryFile": "index.html",
                "redirectsFrom": [],
                "gateHints": [],
                "deadEnd": false
            });

            create_commitment(
                conn,
                ctx,
                CreateReaCommitmentInput {
                    id: Some(id_safe),
                    action: PROJECT_EPR_ACTION.into(),
                    provider: "test-steward".into(),
                    receiver: "test-operator".into(),
                    in_scope_of: Some(scope),
                    note: Some(epr.into()),
                    metadata_json: Some(metadata.to_string()),
                    ..Default::default()
                },
            )
            .unwrap();
        };

        test_seed(conn, "elohim-host-landing", "alpha-elohim-host", "/");
        test_seed(conn, "elohim-host-landing", "elohim-host", "/");
        test_seed(conn, "lamad-spa", "alpha-elohim-host", "/lamad");
        test_seed(conn, "lamad-spa", "elohim-host", "/lamad");
    }

    #[test]
    fn find_active_projections_filters_by_doorway_id() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        seed_test_projections(&mut conn, &ctx);

        let alpha = find_active_projections(&mut conn, &ctx, "alpha-elohim-host").unwrap();
        assert_eq!(alpha.len(), 2);
        assert!(alpha
            .iter()
            .all(|p| p.doorway_id == "doorway:alpha-elohim-host"));

        let beta = find_active_projections(&mut conn, &ctx, "elohim-host").unwrap();
        assert_eq!(beta.len(), 2);
    }

    #[test]
    fn find_projection_by_url_path_longest_prefix_wins() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        seed_test_projections(&mut conn, &ctx);

        let landing =
            find_projection_by_url_path(&mut conn, &ctx, "alpha-elohim-host", "/").unwrap();
        assert_eq!(landing.unwrap().epr_id, "elohim-host-landing");

        let lamad =
            find_projection_by_url_path(&mut conn, &ctx, "alpha-elohim-host", "/lamad/concept/foo")
                .unwrap();
        assert_eq!(lamad.unwrap().epr_id, "lamad-spa");

        let lamad_root =
            find_projection_by_url_path(&mut conn, &ctx, "alpha-elohim-host", "/lamad").unwrap();
        assert_eq!(lamad_root.unwrap().epr_id, "lamad-spa");
    }

    #[test]
    fn find_projection_by_url_path_returns_none_when_no_match() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        seed_test_projections(&mut conn, &ctx);

        let nothing = find_projection_by_url_path(&mut conn, &ctx, "unknown", "/anything").unwrap();
        assert!(nothing.is_none());
    }

    #[test]
    fn path_matches_prefix_handles_root_and_segments() {
        assert!(path_matches_prefix("/", "/"));
        assert!(path_matches_prefix("/anything", "/"));
        assert!(path_matches_prefix("/lamad", "/lamad"));
        assert!(path_matches_prefix("/lamad/", "/lamad"));
        assert!(path_matches_prefix("/lamad/concept/x", "/lamad"));
        assert!(!path_matches_prefix("/lamadx", "/lamad"));
        assert!(!path_matches_prefix("/lam", "/lamad"));
        assert!(!path_matches_prefix("/other", "/lamad"));
    }

    /// Build a full `ReaCommitment` row literal for the mapping tests. No
    /// `make_test_commitment_row` helper pre-existed; this mirrors the
    /// `ReaCommitment` model fields exactly (see db::models). `created_at` is a
    /// `String` on the model, not a chrono type.
    fn make_test_commitment_row() -> ReaCommitment {
        ReaCommitment {
            id: "test-row".into(),
            h_app_id: "lamad".into(),
            action: PROJECT_EPR_ACTION.into(),
            provider: "p".into(),
            receiver: "p".into(),
            resource_conforms_to: None,
            resource_classified_as: None,
            resource_quantity_value: None,
            resource_quantity_unit: None,
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: Some("doorway:alpha-elohim-host|epr:lamad-spa".into()),
            medium_of_exchange_id: None,
            state: "active".into(),
            finished: 0,
            note: None,
            metadata_json: None,
            dht_anchor_hash: None,
            created_at: "2026-06-06T00:00:00Z".into(),
        }
    }

    #[test]
    fn projection_view_maps_route_claims_and_redirect_templates_from_metadata() {
        let metadata = serde_json::json!({
            "urlPath": "/lamad",
            "routeClaims": {
                "schemaVersion": 1,
                "claimsManifestCid": null,
                "claims": [{ "contentType": "path", "template": "path/{id}",
                             "fragments": { "step": "path/{id}/step/{n}" } }]
            },
            "redirectTemplates": [{ "from": "/lamad/resource/{id}", "to": "/epr/{id}" }]
        });
        let row = ReaCommitment {
            id: "test-claims".into(),
            action: "project-epr".into(),
            provider: "p".into(),
            receiver: "p".into(),
            in_scope_of: Some("doorway:alpha-elohim-host|epr:lamad-spa".into()),
            note: None,
            metadata_json: Some(metadata.to_string()),
            ..make_test_commitment_row()
        };
        let view = commitment_to_projection_view(row).unwrap();
        let grant = view.route_claims.expect("granted claims must map");
        assert_eq!(grant.claims[0].content_type, "path");
        assert_eq!(
            grant.claims[0].fragments.get("step").unwrap(),
            "path/{id}/step/{n}"
        );
        assert_eq!(view.redirect_templates[0].from, "/lamad/resource/{id}");
        assert_eq!(view.redirect_templates[0].to, "/epr/{id}");
    }

    // =========================================================================
    // Legacy scope-shape tolerance (incident #2: 2026-06-07 alpha EprRouter
    // wipe-out; incident #1 was the NULL-scope era, see find_active_projections
    // NOTE). A stale storage binary (pre-43951281f) replayed the DHT entry's
    // in_scope_of_json verbatim into the column, persisting
    // `["doorway:alpha-elohim-host|epr:elohim-host-landing"]` — array-wrapped.
    // The LIKE filter still matches the wrapped string, so the parse failure
    // previously failed the WHOLE projection set via collect() and emptied the
    // doorway EprRouter ('/' fell to /threshold, every /lamad mount 404'd).
    // Two rules now hold:
    //   1. parse_projection_scope heals known legacy shapes;
    //   2. find_active_projections degrades per-row, never whole-set.
    // =========================================================================

    #[test]
    fn parse_projection_scope_accepts_bare_pipe_string() {
        let (doorway, epr) =
            parse_projection_scope("doorway:alpha-elohim-host|epr:lamad-spa").unwrap();
        assert_eq!(doorway, "doorway:alpha-elohim-host");
        assert_eq!(epr, "lamad-spa");
    }

    #[test]
    fn parse_projection_scope_heals_array_wrapped_pipe_string() {
        // The live alpha poisoned shape, verbatim: a stale binary's DHT replay
        // wrote serde_json::to_string(&vec![scope]) into the column.
        let (doorway, epr) =
            parse_projection_scope(r#"["doorway:alpha-elohim-host|epr:elohim-host-landing"]"#)
                .unwrap();
        assert_eq!(doorway, "doorway:alpha-elohim-host");
        assert_eq!(epr, "elohim-host-landing");
    }

    #[test]
    fn parse_projection_scope_heals_two_element_legacy_array() {
        // The pre-66f16ab5e seeder shape: two separate scope strings.
        let (doorway, epr) =
            parse_projection_scope(r#"["doorway:alpha-elohim-host","epr:elohim-host-landing"]"#)
                .unwrap();
        assert_eq!(doorway, "doorway:alpha-elohim-host");
        assert_eq!(epr, "elohim-host-landing");
    }

    #[test]
    fn parse_projection_scope_rejects_garbage() {
        assert!(parse_projection_scope("no-pipes-here").is_err());
        assert!(parse_projection_scope("doorway:a|epr:b|extra").is_err());
        assert!(parse_projection_scope(r#"["unrelated"]"#).is_err());
        assert!(parse_projection_scope("[]").is_err());
        assert!(parse_projection_scope("[not-json").is_err());
    }

    #[test]
    fn find_active_projections_skips_unparseable_rows_instead_of_failing_set() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();
        seed_test_projections(&mut conn, &ctx);

        // A poisoned row the LIKE filter matches but no parser heals (three
        // pipe segments). One bad row must not empty the router.
        create_commitment(
            &mut conn,
            &ctx,
            CreateReaCommitmentInput {
                id: Some("poisoned-scope-row".into()),
                action: PROJECT_EPR_ACTION.into(),
                provider: "test-steward".into(),
                receiver: "test-operator".into(),
                in_scope_of: Some("doorway:alpha-elohim-host|epr:x|extra".into()),
                note: Some("poisoned".into()),
                ..Default::default()
            },
        )
        .unwrap();

        let alpha = find_active_projections(&mut conn, &ctx, "alpha-elohim-host").unwrap();
        assert_eq!(alpha.len(), 2, "good rows must survive a poisoned sibling");
        assert!(alpha.iter().all(|p| p.epr_id != "x"));
    }

    #[test]
    fn find_active_projections_serves_array_wrapped_legacy_row() {
        // End-to-end through the resolver: the array-wrapped legacy row both
        // matches the LIKE filter and parses (healed), so it serves.
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        let metadata = serde_json::json!({ "urlPath": "/" });
        create_commitment(
            &mut conn,
            &ctx,
            CreateReaCommitmentInput {
                id: Some("legacy-array-row".into()),
                action: PROJECT_EPR_ACTION.into(),
                provider: "test-steward".into(),
                receiver: "test-operator".into(),
                in_scope_of: Some(
                    r#"["doorway:alpha-elohim-host|epr:elohim-host-landing"]"#.into(),
                ),
                metadata_json: Some(metadata.to_string()),
                ..Default::default()
            },
        )
        .unwrap();

        let alpha = find_active_projections(&mut conn, &ctx, "alpha-elohim-host").unwrap();
        assert_eq!(alpha.len(), 1);
        assert_eq!(alpha[0].epr_id, "elohim-host-landing");
        assert_eq!(alpha[0].doorway_id, "doorway:alpha-elohim-host");
        assert_eq!(alpha[0].url_path, "/");
    }
}

// =============================================================================
// Re-grant supersession tests (spec §3.2/§3.3)
// =============================================================================
#[cfg(test)]
mod supersession_tests {
    use super::*;
    use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

    const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

    fn setup() -> SqliteConnection {
        let mut conn = SqliteConnection::establish(":memory:").expect("in-memory DB");
        conn.run_pending_migrations(MIGRATIONS).expect("migrations");
        conn
    }

    const STEWARD: &str = "12D3KooWSteward";
    const SCOPE: &str = "doorway:alpha-elohim-host|epr:lamad-spa";

    /// Build a project-epr create input mirroring the seeder's body shape.
    /// `route_claims_json` controls whether the grant carries routeClaims:
    /// `None` reproduces the live-alpha grant-less predecessor row.
    fn project_epr_input(
        id: &str,
        route_claims: Option<serde_json::Value>,
        supersedes: Option<&str>,
    ) -> CreateReaCommitmentInput {
        let metadata = serde_json::json!({
            "urlPath": "/lamad",
            "mode": "cached",
            "reach": "commons",
            "baseHref": "/lamad/",
            "entryFile": "index.html",
            "redirectsFrom": [],
            "previewEprRef": null,
            "gateHints": [],
            "deadEnd": false,
            "stewardDirectEndpoint": null,
            "routeClaims": route_claims,
            "redirectTemplates": [],
            // Walkable supersession pointer (spec §3.2 steward-authored metadata).
            "supersedes": supersedes,
        });
        CreateReaCommitmentInput {
            id: Some(id.to_string()),
            action: PROJECT_EPR_ACTION.into(),
            provider: STEWARD.into(),
            receiver: STEWARD.into(),
            in_scope_of: Some(SCOPE.into()),
            note: Some("Project lamad-spa at /lamad".into()),
            metadata_json: Some(metadata.to_string()),
            supersedes: supersedes.map(String::from),
            ..Default::default()
        }
    }

    fn grant_value() -> serde_json::Value {
        serde_json::json!({
            "schemaVersion": 1,
            "claimsManifestCid": null,
            "claims": [
                { "contentType": "path", "template": "path/{id}",
                  "fragments": { "step": "path/{id}/step/{n}" } }
            ]
        })
    }

    /// END-TO-END re-grant: a grant-less predecessor (the live-alpha 2026-05-29
    /// shape) is superseded by a successor carrying routeClaims. After the
    /// supersede, find_active_projections returns ONLY the successor, and it
    /// carries the grant.
    #[test]
    fn regrant_supersedes_grantless_predecessor_with_grant() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        // 1. Seed the grant-less predecessor (routeClaims: null).
        let pred = project_epr_input("project-epr-aaaaaaaaaaaaaaaa", None, None);
        create_commitment(&mut conn, &ctx, pred).unwrap();

        // Sanity: it is active and has no granted claims.
        let before = find_active_projections(&mut conn, &ctx, "alpha-elohim-host").unwrap();
        assert_eq!(before.len(), 1);
        assert!(
            before[0].route_claims.is_none(),
            "predecessor must be grant-less"
        );

        // 2. Supersede with a grant-bearing successor (fingerprint-suffixed id).
        let succ = project_epr_input(
            "project-epr-aaaaaaaaaaaaaaaa-rbbbbbbbb",
            Some(grant_value()),
            Some("project-epr-aaaaaaaaaaaaaaaa"),
        );
        let successor = create_with_supersession(&mut conn, &ctx, succ).unwrap();
        assert_eq!(successor.id, "project-epr-aaaaaaaaaaaaaaaa-rbbbbbbbb");

        // 3. ONLY the successor is active, and it carries the grant.
        let after = find_active_projections(&mut conn, &ctx, "alpha-elohim-host").unwrap();
        assert_eq!(
            after.len(),
            1,
            "exactly one active projection after re-grant"
        );
        assert_eq!(
            after[0].commitment_id,
            "project-epr-aaaaaaaaaaaaaaaa-rbbbbbbbb"
        );
        let grant = after[0]
            .route_claims
            .as_ref()
            .expect("successor must carry the granted claims");
        assert_eq!(grant.claims[0].content_type, "path");

        // 4. The predecessor row still exists but is marked superseded
        //    (notarized history preserved; walkable on the chain).
        let pred_row = get_commitment(&mut conn, &ctx, "project-epr-aaaaaaaaaaaaaaaa")
            .unwrap()
            .expect("predecessor row must survive");
        assert_eq!(pred_row.state, SUPERSEDED_STATE);

        // 5. The successor's metadata carries the walkable supersedes pointer.
        let succ_meta: serde_json::Value =
            serde_json::from_str(successor.metadata_json.as_deref().unwrap()).unwrap();
        assert_eq!(
            succ_meta.get("supersedes").and_then(|v| v.as_str()),
            Some("project-epr-aaaaaaaaaaaaaaaa")
        );
    }

    /// Double-supersede of the SAME predecessor is rejected (first-write-wins):
    /// the second supersede returns AlreadyExists (→ HTTP 409).
    #[test]
    fn double_supersede_rejected_first_write_wins() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        let pred = project_epr_input("pred-1", None, None);
        create_commitment(&mut conn, &ctx, pred).unwrap();

        // First supersede succeeds.
        let s1 = project_epr_input("succ-1", Some(grant_value()), Some("pred-1"));
        create_with_supersession(&mut conn, &ctx, s1).unwrap();

        // Second supersede of the now-superseded predecessor must reject.
        let s2 = project_epr_input("succ-2", Some(grant_value()), Some("pred-1"));
        let err = create_with_supersession(&mut conn, &ctx, s2)
            .expect_err("double-supersede must reject");
        assert!(
            matches!(err, StorageError::AlreadyExists(_)),
            "expected AlreadyExists (409), got: {err:?}"
        );

        // The second successor was NOT inserted (transaction rolled back).
        assert!(get_commitment(&mut conn, &ctx, "succ-2").unwrap().is_none());
        // Exactly one active projection (succ-1) remains.
        let active = find_active_projections(&mut conn, &ctx, "alpha-elohim-host").unwrap();
        assert_eq!(active.len(), 1);
        assert_eq!(active[0].commitment_id, "succ-1");
    }

    /// A cross-scope supersede is rejected: a re-grant rewrites the SAME
    /// projection's grant, never another's.
    #[test]
    fn cross_scope_supersede_rejected() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        // Predecessor on the lamad-spa scope.
        let pred = project_epr_input("pred-scope", None, None);
        create_commitment(&mut conn, &ctx, pred).unwrap();

        // Successor names pred-scope but declares a DIFFERENT scope.
        let mut succ = project_epr_input("succ-scope", Some(grant_value()), Some("pred-scope"));
        succ.in_scope_of = Some("doorway:alpha-elohim-host|epr:other-epr".into());
        let err = create_with_supersession(&mut conn, &ctx, succ)
            .expect_err("cross-scope supersede must reject");
        assert!(
            matches!(err, StorageError::Validation(_)),
            "expected Validation, got: {err:?}"
        );
        assert!(err.to_string().contains("scope mismatch"), "msg: {err}");

        // Predecessor stays active (no partial mutation).
        let pred_row = get_commitment(&mut conn, &ctx, "pred-scope")
            .unwrap()
            .unwrap();
        assert_ne!(pred_row.state, SUPERSEDED_STATE);
    }

    /// A cross-provider supersede is rejected for the same reason.
    #[test]
    fn cross_provider_supersede_rejected() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        let pred = project_epr_input("pred-prov", None, None);
        create_commitment(&mut conn, &ctx, pred).unwrap();

        let mut succ = project_epr_input("succ-prov", Some(grant_value()), Some("pred-prov"));
        succ.provider = "12D3KooWImposter".into();
        let err = create_with_supersession(&mut conn, &ctx, succ)
            .expect_err("cross-provider supersede must reject");
        assert!(matches!(err, StorageError::Validation(_)));
        assert!(err.to_string().contains("provider mismatch"), "msg: {err}");
    }

    /// Superseding an unknown predecessor is a Validation rejection (phantom).
    #[test]
    fn supersede_unknown_predecessor_rejected() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        let succ = project_epr_input("succ-x", Some(grant_value()), Some("does-not-exist"));
        let err = create_with_supersession(&mut conn, &ctx, succ)
            .expect_err("phantom predecessor must reject");
        assert!(matches!(err, StorageError::Validation(_)));
        assert!(err.to_string().contains("unknown commitment"), "msg: {err}");
    }

    /// Successor id equal to predecessor id is rejected before any write.
    #[test]
    fn successor_id_equals_predecessor_rejected() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        let pred = project_epr_input("same-id", None, None);
        create_commitment(&mut conn, &ctx, pred).unwrap();

        let succ = project_epr_input("same-id", Some(grant_value()), Some("same-id"));
        let err =
            create_with_supersession(&mut conn, &ctx, succ).expect_err("equal ids must reject");
        assert!(matches!(err, StorageError::Validation(_)));
        assert!(err.to_string().contains("must differ"), "msg: {err}");
    }

    /// mark_superseded is idempotent: marking an already-superseded predecessor
    /// is a no-op success (the conductor path may re-enter after a signal).
    #[test]
    fn mark_superseded_is_idempotent() {
        let mut conn = setup();
        let ctx = AppContext::default_lamad();

        let pred = project_epr_input("idem-pred", None, None);
        create_commitment(&mut conn, &ctx, pred).unwrap();

        mark_superseded(&mut conn, &ctx, "idem-pred").unwrap();
        // Second call must succeed (idempotent).
        mark_superseded(&mut conn, &ctx, "idem-pred").unwrap();

        let row = get_commitment(&mut conn, &ctx, "idem-pred")
            .unwrap()
            .unwrap();
        assert_eq!(row.state, SUPERSEDED_STATE);
    }
}

#[cfg(test)]
mod operator_helper_tests {
    use super::*;
    use elohim_views::projection::GateHintRelation;

    #[test]
    fn doorway_scope_produces_canonical_json_array() {
        // Single-element JSON array per operator-classification.schema.json
        // scopes pattern. The composite index relies on this exact encoding.
        assert_eq!(
            doorway_scope("alpha-elohim-host"),
            r#"["doorway:alpha-elohim-host"]"#
        );
    }

    #[test]
    fn doorway_scope_escapes_embedded_quotes() {
        // Defensive — doorway ids should never contain quotes (the schema
        // pattern forbids them), but the JSON encoder must still escape them
        // safely if a malformed id slips through.
        let scope = doorway_scope("evil\"-injection");
        let parsed: Vec<String> = serde_json::from_str(&scope).expect("valid JSON");
        assert_eq!(parsed, vec!["doorway:evil\"-injection".to_string()]);
    }

    #[test]
    fn operate_doorway_action_matches_dna_vocabulary() {
        // The constant here must match the entry appended to REA_ACTIONS in
        // elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs.
        // Schema-first codegen of this vocabulary is a future refactor; until
        // then this test is the drift detector.
        assert_eq!(OPERATE_DOORWAY_ACTION, "operate-doorway");
    }

    #[test]
    fn project_epr_action_matches_dna_vocabulary() {
        // The constant here must match the entry appended to REA_ACTIONS in
        // elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs.
        // The action string is also content-addressed into commitment ids;
        // changing it breaks idempotency of every existing seed.
        // Schema-first codegen of this vocabulary is a future refactor; until
        // then this test is the drift detector.
        assert_eq!(PROJECT_EPR_ACTION, "project-epr");
    }

    // -------------------------------------------------------------------------
    // Validator tests (§2.4 rules)
    // -------------------------------------------------------------------------

    fn make_project_epr_input_for_test(
        reach: &str,
        preview: Option<String>,
        hints: Vec<GateHintRef>,
        dead_end: bool,
        endpoint: Option<StewardDirectEndpoint>,
    ) -> ProjectEprValidationInput {
        ProjectEprValidationInput {
            url_path: "/test".into(),
            mode: ProjectionMode::Cached,
            reach: reach.into(),
            preview_epr_ref: preview,
            gate_hints: hints,
            dead_end,
            steward_direct_endpoint: endpoint,
            redirects_from: vec![],
            redirect_templates: vec![],
            route_claims: None,
        }
    }

    fn reserved_prefixes_from_fixture() -> Vec<String> {
        #[derive(serde::Deserialize)]
        struct F {
            #[serde(rename = "reservedPrefixes")]
            reserved_prefixes: Vec<String>,
        }
        let raw = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../sdk/fixtures/route-claims.vectors.json"
        ));
        serde_json::from_str::<F>(raw)
            .expect("route-claims fixture must parse")
            .reserved_prefixes
    }

    #[test]
    fn validator_rejects_alias_on_reserved_prefix() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.redirects_from = vec!["/epr".into()];
        let err = validate_project_epr_commitment(&input).expect_err("reserved alias must reject");
        assert!(
            err.to_string().contains("reserved"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validator_rejects_redirect_template_chain_target() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.redirect_templates = vec![RedirectTemplate {
            from: "/old/{id}".into(),
            to: "/older/{id}".into(), // not /epr/... and not the mount → not canonical
        }];
        let err =
            validate_project_epr_commitment(&input).expect_err("non-canonical target must reject");
        assert!(
            err.to_string().contains("canonical"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validator_accepts_lamad_legacy_template() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.url_path = "/lamad".into();
        input.redirect_templates = vec![RedirectTemplate {
            from: "/lamad/resource/{id}".into(),
            to: "/epr/{id}".into(),
        }];
        assert!(validate_project_epr_commitment(&input).is_ok());
    }

    #[test]
    fn validator_fixture_reserved_list_is_nonempty() {
        // Two-layer guard: storage's validator and doorway's is_service_path both
        // assert against the SAME fixture list (doorway side in Task 8).
        assert!(reserved_prefixes_from_fixture().iter().any(|p| p == "/epr"));
    }

    #[test]
    fn validator_accepts_commons_reach_without_preview_or_hints() {
        let input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        assert!(validate_project_epr_commitment(&input).is_ok());
    }

    #[test]
    fn validator_rejects_non_commons_reach_with_nothing_set() {
        let input =
            make_project_epr_input_for_test("qahal:aleph-members", None, vec![], false, None);
        let err = validate_project_epr_commitment(&input).expect_err("should reject");
        assert!(
            err.to_string().contains("must declare at least one of"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validator_accepts_non_commons_with_dead_end() {
        let input = make_project_epr_input_for_test("qahal:xyz", None, vec![], true, None);
        assert!(validate_project_epr_commitment(&input).is_ok());
    }

    #[test]
    fn validator_accepts_non_commons_with_preview() {
        let input = make_project_epr_input_for_test(
            "qahal:xyz",
            Some("epr:preview-xyz".into()),
            vec![],
            false,
            None,
        );
        assert!(validate_project_epr_commitment(&input).is_ok());
    }

    #[test]
    fn validator_accepts_non_commons_with_hints() {
        let hint = GateHintRef {
            epr_ref: "epr:susan".into(),
            label: Some("Talk to Susan".into()),
            relation: GateHintRelation::PersonWhoCanGrant,
        };
        let input = make_project_epr_input_for_test("qahal:xyz", None, vec![hint], false, None);
        assert!(validate_project_epr_commitment(&input).is_ok());
    }

    #[test]
    fn validator_rejects_steward_direct_without_endpoint() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.mode = ProjectionMode::StewardDirect;
        let err = validate_project_epr_commitment(&input).expect_err("should reject");
        assert!(
            err.to_string().contains("steward-direct mode requires"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validator_rejects_url_path_without_leading_slash() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.url_path = "lamad".into();
        let err = validate_project_epr_commitment(&input).expect_err("should reject");
        assert!(
            err.to_string().contains("urlPath must start with"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn validator_accepts_root_url_path() {
        // "/" is the legal landing-EPR url path — rule 4 special-cases it.
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.url_path = "/".into();
        assert!(validate_project_epr_commitment(&input).is_ok());
    }

    #[test]
    fn validator_rejects_url_path_with_trailing_slash() {
        let mut input = make_project_epr_input_for_test("commons", None, vec![], false, None);
        input.url_path = "/lamad/".into();
        let err = validate_project_epr_commitment(&input).expect_err("should reject");
        assert!(
            err.to_string().contains("trailing slash"),
            "unexpected error: {err}"
        );
    }
}
