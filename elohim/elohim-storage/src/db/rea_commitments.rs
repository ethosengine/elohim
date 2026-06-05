//! REA Commitment CRUD operations
//!
//! hREA/ValueFlows compatible commitment tracking for the learning economy.
//! Tracks binding promises of future economic activity (compute sharing,
//! content provision, assessment availability, etc.).

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use elohim_views::projection::{
    EprProjectionView, GateHintRef, ProjectionMode, StewardDirectEndpoint,
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
pub const CONDUCTOR_SOFT_ACTIONS: [&str; 1] = ["custody-blob"];

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
}

/// Validate a project-epr commitment per the substrate rules (spec §2.4).
///
/// Returns `Ok(())` when the commitment is safe to persist. Returns
/// `Err(StorageError::Validation(_))` with a human-readable message when any
/// rule is violated. The four rules are:
///
/// 1. Non-commons reach requires `previewEprRef` OR `gateHints` (non-empty)
///    OR `deadEnd=true` — at least one onward path for the person.
/// 2. `stewardDirect` mode requires `stewardDirectEndpoint`.
/// 3. `urlPath` must start with `/`.
/// 4. `urlPath` must not have trailing slash unless it IS `/`.
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
/// "Active" means state is not "cancelled" or "terminated". Returns all
/// projections for the given doorway across all states except cancelled/
/// terminated, so the access layer can resolve URL paths from the freshly-
/// seeded set before DHT notarization completes.
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
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("DB error: {}", e)))?;

    commitments
        .into_iter()
        .map(commitment_to_projection_view)
        .collect()
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
fn parse_projection_scope(scope: &str) -> Result<(String, String), StorageError> {
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
        }
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
