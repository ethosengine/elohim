//! Collectives CRUD operations using Diesel with app scoping
//!
//! Collectives are governance contexts with graduated participation.
//! Unifies communities and organizations under a single model.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::{AppContext, PARTICIPATIONS_HAPP_ID};
use super::diesel_schema::{collective_participations, collectives};
use super::models::{
    consent_states, current_timestamp, governance_layers, intimacy_levels, Collective,
    CollectiveParticipation, NewCollective, NewCollectiveParticipation,
};
use crate::error::StorageError;

// ============================================================================
// Query Types
// ============================================================================

/// Input for creating a collective
#[derive(Debug, Clone, Deserialize)]
pub struct CreateCollectiveInput {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default = "default_governance_layer")]
    pub governance_layer: String,
    #[serde(default)]
    pub constitutional_parent_id: Option<String>,
    #[serde(default = "default_community_reach")]
    pub reach: String,
    /// Opaque free-text geographic region label. None ⇒ region unknown.
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<String>,
    #[serde(default)]
    pub created_by: Option<String>,
}

fn default_governance_layer() -> String {
    governance_layers::COMMUNITY.to_string()
}

fn default_community_reach() -> String {
    "community".to_string()
}

impl CreateCollectiveInput {
    /// Minimal placeholder used to materialize an FK parent for joins
    /// arriving on a peer that never received the collective definition
    /// (collective definitions are seeded to a single peer; account packages
    /// are imported on each human's own peer). The stub is a projection
    /// placeholder, not truth — `create_collective` is an id-scoped upsert,
    /// so the authoritative `CollectiveProjected` signal (or a later seed
    /// POST) converges on the same row.
    pub fn stub(id: &str) -> Self {
        Self {
            id: id.to_string(),
            name: id.to_string(),
            description: None,
            governance_layer: default_governance_layer(),
            constitutional_parent_id: None,
            reach: default_community_reach(),
            region: None,
            metadata_json: None,
            created_by: None,
        }
    }
}

/// Query parameters for listing collectives
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CollectiveQuery {
    pub governance_layer: Option<String>,
    pub reach: Option<String>,
    /// If true, only return active (non-dissolved) collectives
    #[serde(default = "default_true")]
    pub active_only: bool,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_true() -> bool {
    true
}
fn default_limit() -> i64 {
    100
}

/// Input for creating a participation
#[derive(Debug, Clone, Deserialize)]
pub struct CreateParticipationInput {
    #[serde(default)]
    pub id: Option<String>,
    pub collective_id: String,
    pub human_id: String,
    #[serde(default = "default_intimacy")]
    pub intimacy_level: String,
    #[serde(default)]
    pub role_context: Option<String>,
    #[serde(default = "default_governance_weight")]
    pub governance_weight: f32,
    #[serde(default = "default_consent")]
    pub consent_state: String,
    #[serde(default)]
    pub metadata_json: Option<String>,
}

fn default_intimacy() -> String {
    intimacy_levels::RECOGNITION.to_string()
}
fn default_governance_weight() -> f32 {
    1.0
}
fn default_consent() -> String {
    consent_states::CONSENTED.to_string()
}

// ============================================================================
// Collective Read Operations
// ============================================================================

/// Get collective by ID
pub fn get_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<Collective>, StorageError> {
    collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::id.eq(id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// App-scope-AGNOSTIC existence probe for FK-parent purposes.
///
/// `collective_participations.collective_id` references bare
/// `collectives(id)` — a parent row under ANY `h_app_id` satisfies the FK
/// (seed-collectives rows land under the legacy "lamad" scope while
/// account-import joins run under "qahal"). Use this, not the app-scoped
/// `get_collective`, when deciding whether a participation insert can land.
pub fn collective_id_exists(conn: &mut SqliteConnection, id: &str) -> Result<bool, StorageError> {
    use diesel::dsl::count_star;
    collectives::table
        .filter(collectives::id.eq(id))
        .select(count_star())
        .first::<i64>(conn)
        .map(|n| n > 0)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Scope-agnostic lookup of a collective's content-CID identity (`collective_cid`).
///
/// Like [`collective_id_exists`], this matches on the bare `collectives(id)`
/// slug across ALL `h_app_id` scopes — seed-collectives rows land under the
/// legacy "lamad" scope while REA commitments are authored under other scopes,
/// so an app-scoped read would miss the parent. Returns:
/// - `Ok(Some(cid))` — the slug names a collective that has been reconciled to
///   its DHT content-CID identity;
/// - `Ok(None)` — either the slug is not a collective at all, OR it is a
///   collective not yet DHT-anchored (`collective_cid` NULL, pre-coherence).
///
/// The caller ([`resolve_party_chain_root`]) cannot distinguish those two `None`
/// cases from the return alone — and does not need to: both degrade to the
/// slug-derived root.
pub fn collective_cid_for_slug(
    conn: &mut SqliteConnection,
    slug: &str,
) -> Result<Option<String>, StorageError> {
    collectives::table
        .filter(collectives::id.eq(slug))
        .select(collectives::collective_cid)
        .first::<Option<String>>(conn)
        .optional()
        .map(|opt| opt.flatten().filter(|s| !s.trim().is_empty()))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Resolve an economic-party identifier to the chain-root cid it should be
/// stored as on an REA commitment (Wave A re-point #5, design §4.1).
///
/// The identity head is a lineage DAG whose durable name is the chain-root cid;
/// a commitment must name the root, never a rotation-fragile raw key.
///
/// - When `party` names a **collective** (a group-controlled identity head), its
///   chain-root is the collective's content-CID identity (`collective_cid`) — a
///   `Collective` + its `Membership{Steward}` set already IS a group-controlled
///   identity head (design §4.1). Routed through [`identity_root_cid`] for
///   uniformity. When the collective is not yet DHT-anchored (`collective_cid`
///   NULL, pre-coherence) this **degrades safely** to the slug-derived root
///   rather than fabricating a cid — the same degrade-don't-guess stance the
///   diversity-placement path takes on a NULL household_id.
/// - Otherwise `party` is a human agent key (or an external label); route it
///   through [`identity_root_cid`] so the indirection is uniform. Degenerate
///   today (the value is unchanged), but the seam is installed for Wave B, when
///   a human's key resolves back to its genesis root.
///
/// Empty party in → empty root out (a one-sided provide commitment's empty
/// `receiver`): never invent an identity for an absent party.
pub fn resolve_party_chain_root(
    conn: &mut SqliteConnection,
    party: &str,
) -> Result<String, StorageError> {
    if party.trim().is_empty() {
        return Ok(String::new());
    }
    if let Some(cid) = collective_cid_for_slug(conn, party)? {
        return Ok(crate::identity_root::identity_root_cid(&cid));
    }
    Ok(crate::identity_root::identity_root_cid(party))
}

/// List collectives with filtering
pub fn list_collectives(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    query: &CollectiveQuery,
) -> Result<Vec<Collective>, StorageError> {
    let mut base_query = collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .into_boxed();

    if let Some(ref layer) = query.governance_layer {
        base_query = base_query.filter(collectives::governance_layer.eq(layer));
    }

    if let Some(ref reach) = query.reach {
        base_query = base_query.filter(collectives::reach.eq(reach));
    }

    if query.active_only {
        base_query = base_query.filter(collectives::dissolved_at.is_null());
    }

    base_query
        .order(collectives::name.asc())
        .limit(query.limit)
        .offset(query.offset)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Collective Write Operations
// ============================================================================

/// Create or upsert a collective (for seeding)
pub fn create_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    input: &CreateCollectiveInput,
) -> Result<Collective, StorageError> {
    if !governance_layers::is_valid(&input.governance_layer) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid governance layer: {}. Valid layers: {:?}",
            input.governance_layer,
            governance_layers::ALL
        )));
    }

    // Upsert: if exists, update name/description/layer; if not, insert
    let existing = get_collective(conn, ctx, &input.id)?;

    if existing.is_some() {
        diesel::update(
            collectives::table
                .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                .filter(collectives::id.eq(&input.id)),
        )
        .set((
            collectives::name.eq(&input.name),
            collectives::description.eq(&input.description),
            collectives::governance_layer.eq(&input.governance_layer),
            collectives::reach.eq(&input.reach),
            collectives::region.eq(&input.region),
            collectives::metadata_json.eq(&input.metadata_json),
            collectives::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;
    } else {
        let new = NewCollective {
            id: &input.id,
            h_app_id: &ctx.h_app_id,
            name: &input.name,
            description: input.description.as_deref(),
            governance_layer: &input.governance_layer,
            constitutional_parent_id: input.constitutional_parent_id.as_deref(),
            reach: &input.reach,
            region: input.region.as_deref(),
            metadata_json: input.metadata_json.as_deref(),
            created_by: input.created_by.as_deref(),
            collective_cid: None,
            slug: None,
        };

        diesel::insert_into(collectives::table)
            .values(&new)
            .execute(conn)
            .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;
    }

    get_collective(conn, ctx, &input.id)?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created collective".into()))
}

// ============================================================================
// Shared collective projection mapping (signal arm + reconcile arm)
// ============================================================================

/// Hints a `Collective`'s charter carries for the local projection.
///
/// Household coherence (2026-06-04 formation spec §5.4): a charter that declares
/// `{"kind":"household"}` projects as `governance_layer='family'`, and a declared
/// `slugAlias` merges onto the pre-coherence seed row (one household, one row —
/// the slug is the display alias, the CID is canonical).
///
/// Parsing is total: a malformed / absent charter yields the community defaults
/// rather than an error, because the projection must still land.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
pub struct CharterHints {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default, rename = "slugAlias")]
    pub slug_alias: Option<String>,
}

impl CharterHints {
    /// Parse a charter string; a non-JSON or absent charter yields defaults.
    pub fn parse(charter: Option<&str>) -> Self {
        charter
            .and_then(|c| serde_json::from_str::<Self>(c).ok())
            .unwrap_or_default()
    }

    pub fn is_household(&self) -> bool {
        self.kind.as_deref() == Some("household")
    }

    /// Household-ness is the ONLY DNA-derived governance layer; everything else
    /// projects as `community` (steward/seed set it otherwise).
    pub fn governance_layer(&self) -> String {
        if self.is_household() {
            governance_layers::FAMILY.to_string()
        } else {
            governance_layers::COMMUNITY.to_string()
        }
    }

    // TODO: replace with canonical collective-reach constants once the reach
    // vocabulary is reconciled (roadmap #13 — don't bake new literals).
    pub fn reach(&self) -> String {
        if self.is_household() {
            "trusted".to_string()
        } else {
            "community".to_string()
        }
    }

    /// The declared slug alias, trimmed and normalized to `None` when empty.
    pub fn slug_alias(&self) -> Option<&str> {
        self.slug_alias
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}

/// How a collective-CID projection may interact with an already-anchored row.
///
/// Mirrors `content_diesel::StampMode`, minus a `HealCanonical` tier: the
/// `collectives` table carries NO declaration-ordering column, so a heal can
/// never PROVE a forward move. "When in doubt do not move" therefore collapses
/// the two heal tiers into one fill-only mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectiveStampMode {
    /// Canonical channel (post-commit `CollectiveProjected` signal, seed POST):
    /// always writes, including re-pointing a row that already carries a
    /// DIFFERENT cid. These channels are own-authored or request-borne.
    Declare,
    /// Reconcile heal: FILLS absence only — creates a missing row, or stamps a
    /// cid onto a row that carries none. A row already carrying a DIFFERENT
    /// non-empty cid is left untouched (heal fills, never moves).
    GapFill,
}

/// Outcome of [`project_collective`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollectiveProjectionOutcome {
    /// No local row carried this cid; a new cid-keyed row was inserted.
    Created,
    /// An existing un-anchored row (the charter's `slugAlias`, or a caller-
    /// supplied routing alias) was stamped with this cid. Convergence.
    AliasMerged,
    /// A row already carried EXACTLY this cid; value fields were refreshed but
    /// nothing converged.
    Refreshed,
    /// `GapFill` mode and the candidate alias row already carries a DIFFERENT
    /// cid — left to the canonical channels. A correct refusal, not an error.
    SkippedDeclared,
}

/// Everything the shared mapping needs to project ONE `Collective` DHT entry.
#[derive(Debug, Clone)]
pub struct CollectiveProjection<'a> {
    /// Canonical `collective:{action_hash}` identity. THE reconciliation key.
    pub collective_cid: &'a str,
    pub display_name: &'a str,
    pub founder_agent_cid: Option<&'a str>,
    /// Raw charter string from the entry (drives governance layer + slug alias).
    pub charter: Option<&'a str>,
    /// An ADDITIONAL local-row id to consider merging onto, beyond the charter's
    /// declared `slugAlias` — the reconcile arm passes the peer-advertised
    /// routing alias here. Honored ONLY when a row under that id exists locally
    /// AND carries no cid, so it can fill an un-anchored seed row but can never
    /// re-point an anchored one. Peer bytes never enter the row body; this is a
    /// pointer to a LOCAL row, and every value field still comes from the own
    /// conductor's entry.
    pub merge_onto_id: Option<&'a str>,
    pub mode: CollectiveStampMode,
}

/// The ONE mapping from a conductor-read `Collective` entry to the local
/// `collectives` projection — shared by the post-commit signal arm
/// (`ReconcileController::on_collective_projected`, `Declare` mode) and the
/// cross-peer reconcile arm (`p2p::projection_reconcile`, `GapFill` mode).
///
/// Resolution order (first match wins):
/// 1. A row already carrying this exact cid → refresh value fields.
/// 2. An alias candidate (charter `slugAlias`, then `merge_onto_id`) that exists
///    locally → stamp the cid onto it, subject to the mode's move guard.
/// 3. Otherwise insert a cid-keyed row (`id = collective_cid`), the shape a
///    steward later re-slugs.
pub fn project_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    p: &CollectiveProjection<'_>,
) -> Result<CollectiveProjectionOutcome, StorageError> {
    conn.immediate_transaction(|conn| project_collective_within_txn(conn, ctx, p))
}

/// Body of [`project_collective`], run inside the caller's `BEGIN IMMEDIATE`
/// transaction so the step-1 existence check and the eventual write are
/// atomic against another connection racing the same `collective_cid` — the
/// TOCTOU a non-unique `idx_collectives_cid` let through (two concurrent
/// callers each pass step 1 before either commits, each minting its own
/// cid-bearing row). Migration `2026-07-28-090000_collectives_cid_unique_index`
/// backstops this with a real `UNIQUE` constraint for any write path that
/// still reaches `collectives` outside this function (e.g. a direct
/// `create_collective` call with a pre-set `collective_cid`). If a write
/// below trips that constraint anyway, a concurrent writer won the race
/// between our read and our write — re-read and take the refresh branch
/// instead of bubbling a hard error; that row existing under someone else's
/// write is exactly the condition step 1 checks for.
fn project_collective_within_txn(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    p: &CollectiveProjection<'_>,
) -> Result<CollectiveProjectionOutcome, StorageError> {
    let hints = CharterHints::parse(p.charter);
    let layer = hints.governance_layer();
    let reach = hints.reach();

    // (1) Already anchored to this cid, under whatever id — refresh only.
    if let Some(existing_id) = id_for_collective_cid(conn, ctx, p.collective_cid)? {
        refresh_collective_row(conn, ctx, &existing_id, p.display_name, &layer)?;
        return Ok(CollectiveProjectionOutcome::Refreshed);
    }

    // (2) Alias merge onto an existing local row.
    for alias in [hints.slug_alias(), p.merge_onto_id].into_iter().flatten() {
        let Some(current_cid) = collective_cid_of_id(conn, ctx, alias)? else {
            continue; // no local row under this alias
        };
        if current_cid.is_some() && p.mode == CollectiveStampMode::GapFill {
            // Row carries a DIFFERENT cid (step 1 already excluded "same cid").
            // Heal fills, never moves — the canonical channels own this.
            return Ok(CollectiveProjectionOutcome::SkippedDeclared);
        }
        let stamped = diesel::update(
            collectives::table
                .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                .filter(collectives::id.eq(alias)),
        )
        .set((
            collectives::collective_cid.eq(Some(p.collective_cid)),
            collectives::slug.eq(Some(alias)),
            collectives::governance_layer.eq(&layer),
            collectives::updated_at.eq(current_timestamp()),
        ))
        .execute(conn);
        return match stamped {
            Ok(_) => Ok(CollectiveProjectionOutcome::AliasMerged),
            Err(e) if is_cid_unique_violation(&e) => recover_from_cid_race(conn, ctx, p, &layer),
            Err(e) => Err(StorageError::Internal(format!(
                "Collective alias merge failed: {}",
                e
            ))),
        };
    }

    // (3) Insert a cid-keyed row (idempotent upsert on id), then stamp the cid.
    let input = CreateCollectiveInput {
        id: p.collective_cid.to_string(),
        name: p.display_name.to_string(),
        description: None,
        governance_layer: layer.clone(),
        constitutional_parent_id: None,
        reach,
        region: None,
        metadata_json: None,
        created_by: p.founder_agent_cid.map(str::to_string),
    };
    create_collective(conn, ctx, &input)?;
    let stamped = diesel::update(
        collectives::table
            .filter(collectives::h_app_id.eq(&ctx.h_app_id))
            .filter(collectives::id.eq(p.collective_cid)),
    )
    .set(collectives::collective_cid.eq(Some(p.collective_cid)))
    .execute(conn);
    match stamped {
        Ok(_) => Ok(CollectiveProjectionOutcome::Created),
        Err(e) if is_cid_unique_violation(&e) => recover_from_cid_race(conn, ctx, p, &layer),
        Err(e) => Err(StorageError::Internal(format!(
            "Collective cid stamp failed: {}",
            e
        ))),
    }
}

/// A write above hit the `idx_collectives_cid_unique` constraint: another
/// writer committed a row under this exact `collective_cid` between our
/// step-1 read and our write. Re-read (the violation proves a row now
/// exists) and converge onto it via the same refresh step 1 would have
/// taken had it run a moment later — never a duplicate insert.
fn recover_from_cid_race(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    p: &CollectiveProjection<'_>,
    layer: &str,
) -> Result<CollectiveProjectionOutcome, StorageError> {
    let existing_id = id_for_collective_cid(conn, ctx, p.collective_cid)?.ok_or_else(|| {
        StorageError::Internal(format!(
            "Collective cid {} raced a unique-constraint violation but no row carries it on re-read",
            p.collective_cid
        ))
    })?;
    refresh_collective_row(conn, ctx, &existing_id, p.display_name, layer)?;
    Ok(CollectiveProjectionOutcome::Refreshed)
}

fn is_cid_unique_violation(err: &diesel::result::Error) -> bool {
    matches!(
        err,
        diesel::result::Error::DatabaseError(diesel::result::DatabaseErrorKind::UniqueViolation, _)
    )
}

/// Shared value-field refresh for a row already anchored to the target cid
/// (step 1's normal path, and the race-recovery path's converge branch).
fn refresh_collective_row(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
    display_name: &str,
    layer: &str,
) -> Result<(), StorageError> {
    diesel::update(
        collectives::table
            .filter(collectives::h_app_id.eq(&ctx.h_app_id))
            .filter(collectives::id.eq(id)),
    )
    .set((
        collectives::name.eq(display_name),
        collectives::governance_layer.eq(layer),
        collectives::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Collective refresh failed: {}", e)))?;
    Ok(())
}

/// The `collectives.id` of the row carrying `collective_cid` in this app scope,
/// if any. Empty-string cids are treated as absent (never an identity).
pub fn id_for_collective_cid(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    collective_cid: &str,
) -> Result<Option<String>, StorageError> {
    collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::collective_cid.eq(collective_cid))
        .select(collectives::id)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// `Ok(None)` — no row under `id` in this scope. `Ok(Some(None))` — row present
/// but un-anchored. `Ok(Some(Some(cid)))` — row present and anchored.
pub fn collective_cid_of_id(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Option<Option<String>>, StorageError> {
    collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::id.eq(id))
        .select(collectives::collective_cid)
        .first::<Option<String>>(conn)
        .optional()
        .map(|outer| outer.map(|inner| inner.filter(|s| !s.trim().is_empty())))
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Cross-peer reconcile support (p2p::projection_reconcile collectives arm)
// ============================================================================

/// `(id, collective_cid)` inventory for the `collectives` reconcile stream, plus
/// the honest whole-corpus total (offset-invariant, so a requester can detect a
/// windowed page).
///
/// **NULL-`collective_cid` rows are EXCLUDED, by design.** A pre-coherence seed
/// row has no DHT identity to reconcile ON: advertising it would invite peers to
/// diff against a slug that names nothing notarized, and healing from it would
/// mean adopting a peer's local routing alias as truth — strictly worse than a
/// gap. Such rows remain upgradable in place: the `slugAlias` merge in
/// [`project_collective`] stamps them the moment their real cid arrives, and
/// they enter the inventory on the next sweep.
///
/// Dissolved collectives are excluded for the same reason `list_collectives`
/// defaults to `active_only`: a dissolved row is not something to converge peers
/// onto. Ordered `updated_at DESC, id ASC` — hot set first, mirroring
/// `content_diesel::list_content_anchor_inventory`.
pub fn list_collective_cid_inventory(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    offset: i64,
    cap: i64,
) -> Result<(Vec<(String, String)>, i64), StorageError> {
    let total: i64 = collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::collective_cid.is_not_null())
        .filter(collectives::collective_cid.ne(""))
        .filter(collectives::dissolved_at.is_null())
        .count()
        .get_result(conn)
        .map_err(|e| {
            StorageError::Internal(format!("collective cid inventory count failed: {e}"))
        })?;

    let rows: Vec<(String, Option<String>)> = collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::collective_cid.is_not_null())
        .filter(collectives::collective_cid.ne(""))
        .filter(collectives::dissolved_at.is_null())
        .order((collectives::updated_at.desc(), collectives::id.asc()))
        .offset(offset.max(0))
        .limit(cap)
        .select((collectives::id, collectives::collective_cid))
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("collective cid inventory load failed: {e}"))
        })?;

    // `IS NOT NULL` guarantees `Some`; `filter_map` discards defensively rather
    // than unwrap (the same discipline the content inventory uses).
    let entries = rows
        .into_iter()
        .filter_map(|(id, cid)| cid.map(|c| (id, c)))
        .collect();
    Ok((entries, total))
}

/// Which of `ids` exist in the local `collectives` projection for this scope.
///
/// App-scoped deliberately, so it agrees with
/// [`list_collective_cid_inventory`]: a scope-agnostic presence probe (see
/// [`collective_id_exists`], which exists for FK purposes) paired with a
/// scope-bound inventory would classify a foreign-scope row as "present but
/// un-anchored" and try to stamp a cid onto a row this reconcile never reads.
///
/// NOTE: same `SQLITE_MAX_VARIABLE_NUMBER` caveat as
/// `content_diesel::content_ids_present` — callers must chunk large id sets.
pub fn collective_ids_present(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    ids: &[String],
) -> Result<std::collections::HashSet<String>, StorageError> {
    if ids.is_empty() {
        return Ok(Default::default());
    }
    let found: Vec<String> = collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::id.eq_any(ids))
        .select(collectives::id)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("collective presence query failed: {e}")))?;
    Ok(found.into_iter().collect())
}

/// `(dht_anchor_hash, dht_anchor_hash)` inventory for the
/// `collective_participations` reconcile stream, plus the honest whole-corpus
/// total (offset-invariant, so a requester can detect a windowed page).
///
/// **NULL / empty `dht_anchor_hash` rows are EXCLUDED, by design** — the same
/// ruling the collectives arm makes about NULL-cid rows. A participation row
/// born from a local account-import or a seed POST carries no DHT identity to
/// reconcile ON; advertising it would invite peers to diff against a peer-local
/// UUID that names nothing notarized. Such rows stay upgradable IN PLACE: the
/// shared mapping in [`crate::db::memberships::project_membership`] merges a
/// conductor-read `Membership` onto the existing `(collective_id, human_id)`
/// row and stamps its anchor the moment the Membership entry is resolved.
///
/// **BOTH slots carry the anchor.** The diesel `id` is a peer-local UUID —
/// meaningless across peers and actively dangerous as a reconciliation key
/// (two peers projecting the SAME Membership mint different UUIDs). The
/// `dht_anchor_hash` — the `Membership` entry's ActionHash, which is exactly
/// the idempotency key the local projector already uses — IS the identity, so
/// the `id` slot repeats it rather than advertising something a heal must
/// remember never to trust.
///
/// Departed rows are deliberately INCLUDED (unlike dissolved collectives): a
/// departure is DHT truth carried by the same entry, and a peer that has never
/// seen the Membership at all needs the departure as much as the join.
/// Ordered `updated_at DESC, id ASC` — hot set first, mirroring the other
/// inventories.
pub fn list_participation_anchor_inventory(
    conn: &mut SqliteConnection,
    offset: i64,
    cap: i64,
) -> Result<(Vec<String>, i64), StorageError> {
    let total: i64 = collective_participations::table
        .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
        .filter(collective_participations::dht_anchor_hash.is_not_null())
        .filter(collective_participations::dht_anchor_hash.ne(""))
        .count()
        .get_result(conn)
        .map_err(|e| {
            StorageError::Internal(format!("participation anchor inventory count failed: {e}"))
        })?;

    let rows: Vec<Option<String>> = collective_participations::table
        .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
        .filter(collective_participations::dht_anchor_hash.is_not_null())
        .filter(collective_participations::dht_anchor_hash.ne(""))
        .order((
            collective_participations::updated_at.desc(),
            collective_participations::id.asc(),
        ))
        .offset(offset.max(0))
        .limit(cap)
        .select(collective_participations::dht_anchor_hash)
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("participation anchor inventory load failed: {e}"))
        })?;

    // `IS NOT NULL` guarantees `Some`; `filter_map` discards defensively rather
    // than unwrap (the same discipline the content/collectives inventories use).
    Ok((rows.into_iter().flatten().collect(), total))
}

/// Which of `anchors` this peer already holds in its participation projection.
///
/// The reconcile arm's ONE presence query: an anchor present here is in sync (a
/// row already carries this Membership), an anchor absent is a gap the own
/// conductor can answer for.
///
/// NOTE: same `SQLITE_MAX_VARIABLE_NUMBER` caveat as
/// [`collective_ids_present`] — callers must chunk large id sets.
pub fn participation_anchors_present(
    conn: &mut SqliteConnection,
    anchors: &[String],
) -> Result<std::collections::HashSet<String>, StorageError> {
    if anchors.is_empty() {
        return Ok(Default::default());
    }
    let found: Vec<Option<String>> = collective_participations::table
        .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
        .filter(collective_participations::dht_anchor_hash.eq_any(anchors))
        .select(collective_participations::dht_anchor_hash)
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("participation presence query failed: {e}")))?;
    Ok(found.into_iter().flatten().collect())
}

/// Dissolve a collective (sets dissolved_at timestamp)
pub fn dissolve_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    id: &str,
) -> Result<Collective, StorageError> {
    diesel::update(
        collectives::table
            .filter(collectives::h_app_id.eq(&ctx.h_app_id))
            .filter(collectives::id.eq(id)),
    )
    .set((
        collectives::dissolved_at.eq(current_timestamp()),
        collectives::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    get_collective(conn, ctx, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Collective {} not found", id)))
}

// ============================================================================
// Index-free self-discovery gap-fill (services::identity_fill)
// ============================================================================

/// Outcome of [`gap_fill_household_collective_cid_via_membership`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HouseholdCidGapFillOutcome {
    /// The membership join resolved exactly one distinct slug-form
    /// `household_id`, and that slug's local row was un-anchored — stamped.
    Stamped,
    /// This discovered cid already matches SOME local `collective_cid` —
    /// nothing to do (the common steady-state outcome after the first stamp).
    AlreadyKnown,
    /// No member of this household resolved to a local `humans` row carrying
    /// a SLUG-form `household_id` — either no member matched any row yet, or
    /// every match was cid-form only (a row this same identity_fill pass
    /// itself created/filled, per `controller.rs:1085`'s
    /// `household_id = collective_cid` convention). Nothing to safely stamp
    /// onto yet; a later sweep (once a genesis-self-healed member is found)
    /// may resolve it.
    NoCandidateRow,
    /// The one resolved slug's local row already carries a DIFFERENT non-NULL
    /// cid than this discovery — fills-never-moves; left untouched. Logged at
    /// WARN so a genuine household re-formation is observable rather than
    /// silently swallowed.
    Mismatch,
    /// The membership join resolved MORE THAN ONE distinct slug-form
    /// `household_id` among this household's members — this pass cannot
    /// safely tell which slug names the row for this discovered cid, so it
    /// abstains rather than guess a wrong pairing.
    Ambiguous,
    /// The NULL-guarded stamp UPDATE touched zero rows — another writer
    /// (a live `CollectiveProjected` signal, a concurrent sweep) already
    /// stamped this exact row between the read and the write. Converged,
    /// not an error.
    RacedElsewhere,
}

/// A local `collectives.id`/`humans.household_id` slug is never CID-shaped
/// (the canonical `collective:{action_hash}` form) — genesis/seed slugs read
/// `household-dowell`, `family-eden`, `couple-adam-eve`, never `collective:…`.
/// This is the exclusion the membership join needs: a `humans` row this SAME
/// identity_fill pass (or a prior one) already created/filled carries
/// `household_id = collective_cid` verbatim (`controller.rs:1085`,
/// `identity_fill::apply_membership_fill`) — a cid-form value that must never
/// poison the slug candidate set.
fn is_collective_cid_shaped(value: &str) -> bool {
    value.starts_with("collective:")
}

/// Stamp a local, un-anchored `collectives` row from a household cid
/// discovered via index-free own-conductor DHT enumeration
/// (`identity_fill::discover_household_pairs`'s `get_my_household_collective_cids`
/// read), joined DETERMINISTICALLY through the SAME household's membership
/// truth — the other half of the collectives-arm bootstrap gap
/// (`backlog-collectives-arm-bootstrap-gap-no-stamped-cid-anywhere`).
///
/// # Why this closes the bootstrap circle
///
/// `identity_fill` already discovers a pod's household cid(s) straight from
/// DHT membership truth and fills `humans.agent_pub_key`/`humans.household_id`
/// from it — verified live (a `humans` row created from membership truth,
/// `created=1`). But nothing ever stamped that SAME cid onto the LOCAL
/// `collectives` row, so [`list_collective_cid_inventory`] (which filters
/// `collective_cid IS NOT NULL` BY DESIGN — a pre-coherence row has no DHT
/// identity to reconcile on) advertised nothing, the cross-peer collectives
/// reconcile arm had zero inventory to discover fleet-wide, and the stamp
/// could never propagate to any peer — even one whose own self-discovery is
/// silent. Fleet-wide evidence 2026-07-28: a fill sweep created a member row
/// from membership truth at 08:29Z, yet `collectives_ids_discovered=0` on
/// every doorway ~40 minutes later.
///
/// # Why cardinality alone cannot pick the row
///
/// The genesis seed data plants ~20 `governance_layer=family` rows
/// (`household-dowell`, `household-susan`, `household-daniel`, …) — the same
/// shared reference dataset every doorway seeds locally — so "the lone
/// un-anchored FAMILY row" never holds in production. The row this discovery
/// belongs to must be resolved DETERMINISTICALLY, not by counting candidates.
///
/// # The membership join
///
/// `member_cids` are this household's CURRENT Person members (the SAME
/// `member_cid`s in `MembershipSnapshot.pairs` for `discovered_cid` —
/// `agent:{pubkey}` form). For each, strip the `agent:` prefix and look up any
/// EXISTING `humans` row already carrying that bare key
/// ([`crate::db::humans::get_human_by_agent_key`] — the exact lookup
/// `identity_fill::apply_membership_fill`'s already-present short-circuit
/// already performs). A matched row's `household_id` is a candidate ONLY when
/// it is SLUG-shaped ([`is_collective_cid_shaped`] excludes a cid-form value —
/// a row this same identity_fill pass created/filled carries
/// `household_id = collective_cid`, which would otherwise poison the join with
/// the very value we are trying to independently corroborate). A genesis-
/// self-healed member (`genesis_self_heal_identity`, e.g. matthew healed to
/// `household_id = "household-dowell"`) is exactly the kind of row that
/// resolves the slug.
///
/// # fills-never-moves
///
/// The candidate row is stamped only when the join is UNAMBIGUOUS: exactly one
/// distinct slug-form `household_id` resolves across all matched members, and
/// that slug's local row currently carries `collective_cid IS NULL`. Zero
/// matched members, or matched members carrying only cid-form values, yields
/// [`HouseholdCidGapFillOutcome::NoCandidateRow`] (nothing to stamp onto yet —
/// never a guess). More than one distinct slug yields
/// [`HouseholdCidGapFillOutcome::Ambiguous`]. A resolved slug whose row already
/// carries a DIFFERENT cid yields [`HouseholdCidGapFillOutcome::Mismatch`]
/// (never touched) — the same correct-or-abstain stance
/// `membership_identity_reconcile` takes on a forced-bijection ambiguity,
/// never a guessed pairing.
pub fn gap_fill_household_collective_cid_via_membership(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    discovered_cid: &str,
    member_cids: &[String],
) -> Result<HouseholdCidGapFillOutcome, StorageError> {
    use crate::db::humans::get_human_by_agent_key;

    if discovered_cid.trim().is_empty() {
        return Ok(HouseholdCidGapFillOutcome::NoCandidateRow);
    }

    // Already known locally under ANY row in this scope — nothing to do.
    let already_known: Option<String> = collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::collective_cid.eq(discovered_cid))
        .select(collectives::id)
        .first::<String>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Internal(format!(
                "household collective_cid gap-fill: already-known lookup failed: {e}"
            ))
        })?;
    if already_known.is_some() {
        return Ok(HouseholdCidGapFillOutcome::AlreadyKnown);
    }

    // Resolve the household's members to EXISTING humans rows (the same
    // agent-key lookup `apply_membership_fill`'s short-circuit performs),
    // collecting the DISTINCT slug-form `household_id` values seen. Cid-form
    // values (a row this pass itself already lit) are excluded — they would
    // otherwise trivially "confirm" the very cid we are corroborating.
    let mut slugs: std::collections::HashSet<String> = std::collections::HashSet::new();
    for member_cid in member_cids {
        let bare = member_cid
            .strip_prefix("agent:")
            .unwrap_or(member_cid)
            .trim();
        if bare.is_empty() {
            continue;
        }
        match get_human_by_agent_key(conn, bare) {
            Ok(Some(row)) => {
                if let Some(hid) = row.household_id {
                    if !is_collective_cid_shaped(&hid) {
                        slugs.insert(hid);
                    }
                }
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(
                    member_cid = %member_cid,
                    error = %e,
                    "household collective_cid gap-fill: humans lookup failed (skipping member)"
                );
            }
        }
    }

    if slugs.len() > 1 {
        tracing::warn!(
            discovered_cid = %discovered_cid,
            distinct_slugs = slugs.len(),
            "identity_fill: ambiguous household collective_cid gap-fill via membership join (multiple distinct slugs) — abstaining rather than guessing a pairing"
        );
        return Ok(HouseholdCidGapFillOutcome::Ambiguous);
    }

    let Some(slug) = slugs.into_iter().next() else {
        return Ok(HouseholdCidGapFillOutcome::NoCandidateRow);
    };

    let existing_cid: Option<Option<String>> = collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::id.eq(&slug))
        .select(collectives::collective_cid)
        .first::<Option<String>>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Internal(format!(
                "household collective_cid gap-fill: slug row lookup failed: {e}"
            ))
        })?;

    match existing_cid {
        None => {
            // The resolved slug names no local row at all — nothing to stamp.
            Ok(HouseholdCidGapFillOutcome::NoCandidateRow)
        }
        Some(Some(existing)) if existing == discovered_cid => {
            Ok(HouseholdCidGapFillOutcome::AlreadyKnown)
        }
        Some(Some(existing)) => {
            // fills-never-moves: a DIFFERENT cid is already anchored — never
            // touch it, but this divergence is worth an operator's eyes.
            tracing::warn!(
                collective_id = %slug,
                existing_cid = %existing,
                discovered_cid = %discovered_cid,
                "identity_fill: local household collective already carries a DIFFERENT cid than discovered — fills-never-moves, leaving untouched"
            );
            Ok(HouseholdCidGapFillOutcome::Mismatch)
        }
        Some(None) => {
            let n = diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                    .filter(collectives::id.eq(&slug))
                    .filter(collectives::collective_cid.is_null()),
            )
            .set((
                collectives::collective_cid.eq(Some(discovered_cid)),
                collectives::updated_at.eq(current_timestamp()),
            ))
            .execute(conn)
            .map_err(|e| {
                StorageError::Internal(format!(
                    "household collective_cid gap-fill: stamp failed: {e}"
                ))
            })?;

            Ok(if n > 0 {
                tracing::info!(
                    collective_id = %slug,
                    collective_cid = %discovered_cid,
                    "identity_fill: stamped local household collectives row from discovered DHT truth (membership-join resolved)"
                );
                HouseholdCidGapFillOutcome::Stamped
            } else {
                // Lost a race against another writer (live signal, concurrent
                // sweep) between our SELECT and this NULL-guarded UPDATE.
                HouseholdCidGapFillOutcome::RacedElsewhere
            })
        }
    }
}

// ============================================================================
// Participation Read Operations
//
// EVERY function below scopes by [`PARTICIPATIONS_HAPP_ID`] and takes NO
// `AppContext`. That is the anti-drift shape, not an omission: the write side
// (account-import) and the read side (the legacy participants route) had
// silently disagreed — `"qahal"` vs `"lamad"` — and no signature stopped them.
// With the scope owned here, a caller cannot hand in the operating content
// scope by accident. See `db::context::PARTICIPATIONS_HAPP_ID`.
// ============================================================================

/// Get all active participations for a human
pub fn get_participations_for_human(
    conn: &mut SqliteConnection,
    human_id: &str,
) -> Result<Vec<CollectiveParticipation>, StorageError> {
    collective_participations::table
        .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
        .filter(collective_participations::human_id.eq(human_id))
        .filter(collective_participations::departed_at.is_null())
        .order(collective_participations::joined_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all active participants of a collective
pub fn get_participants_of_collective(
    conn: &mut SqliteConnection,
    collective_id: &str,
) -> Result<Vec<CollectiveParticipation>, StorageError> {
    collective_participations::table
        .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
        .filter(collective_participations::collective_id.eq(collective_id))
        .filter(collective_participations::departed_at.is_null())
        .order(collective_participations::joined_at.asc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

// ============================================================================
// Participation Write Operations
// ============================================================================

/// Create a participation (tolerates UNIQUE constraint violations for re-seeding)
pub fn create_participation(
    conn: &mut SqliteConnection,
    input: &CreateParticipationInput,
) -> Result<CollectiveParticipation, StorageError> {
    if !intimacy_levels::is_valid(&input.intimacy_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid intimacy level: {}. Valid levels: {:?}",
            input.intimacy_level,
            intimacy_levels::ALL
        )));
    }

    let id = input
        .id
        .clone()
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Check if participation already exists (upsert for re-seeding)
    let existing: Option<CollectiveParticipation> = collective_participations::table
        .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
        .filter(collective_participations::collective_id.eq(&input.collective_id))
        .filter(collective_participations::human_id.eq(&input.human_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?;

    if let Some(existing) = existing {
        // Update existing participation
        diesel::update(
            collective_participations::table.filter(collective_participations::id.eq(&existing.id)),
        )
        .set((
            collective_participations::intimacy_level.eq(&input.intimacy_level),
            collective_participations::role_context.eq(&input.role_context),
            collective_participations::governance_weight.eq(input.governance_weight),
            collective_participations::consent_state.eq(&input.consent_state),
            collective_participations::departed_at.eq(None::<String>), // Re-join if departed
            collective_participations::updated_at.eq(current_timestamp()),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

        return collective_participations::table
            .filter(collective_participations::id.eq(&existing.id))
            .first(conn)
            .optional()
            .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
            .ok_or_else(|| {
                StorageError::Internal("Failed to retrieve updated participation".into())
            });
    }

    let new = NewCollectiveParticipation {
        id: &id,
        h_app_id: PARTICIPATIONS_HAPP_ID,
        collective_id: &input.collective_id,
        human_id: &input.human_id,
        intimacy_level: &input.intimacy_level,
        role_context: input.role_context.as_deref(),
        governance_weight: input.governance_weight,
        consent_state: &input.consent_state,
        metadata_json: input.metadata_json.as_deref(),
        member_cid: None,
        member_kind: "person",
        dht_anchor_hash: None,
    };

    diesel::insert_into(collective_participations::table)
        .values(&new)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Insert failed: {}", e)))?;

    collective_participations::table
        .filter(collective_participations::id.eq(&id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| StorageError::Internal("Failed to retrieve created participation".into()))
}

/// Update participation intimacy level
pub fn update_participation_intimacy(
    conn: &mut SqliteConnection,
    participation_id: &str,
    new_level: &str,
) -> Result<CollectiveParticipation, StorageError> {
    if !intimacy_levels::is_valid(new_level) {
        return Err(StorageError::InvalidInput(format!(
            "Invalid intimacy level: {}. Valid levels: {:?}",
            new_level,
            intimacy_levels::ALL
        )));
    }

    diesel::update(
        collective_participations::table
            .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
            .filter(collective_participations::id.eq(participation_id)),
    )
    .set((
        collective_participations::intimacy_level.eq(new_level),
        collective_participations::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    collective_participations::table
        .filter(collective_participations::id.eq(participation_id))
        .first(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))?
        .ok_or_else(|| {
            StorageError::NotFound(format!("Participation {} not found", participation_id))
        })
}

/// Depart from a collective (sets departed_at — soft exit)
pub fn depart_collective(
    conn: &mut SqliteConnection,
    collective_id: &str,
    human_id: &str,
) -> Result<bool, StorageError> {
    let updated = diesel::update(
        collective_participations::table
            .filter(collective_participations::h_app_id.eq(PARTICIPATIONS_HAPP_ID))
            .filter(collective_participations::collective_id.eq(collective_id))
            .filter(collective_participations::human_id.eq(human_id))
            .filter(collective_participations::departed_at.is_null()),
    )
    .set((
        collective_participations::departed_at.eq(current_timestamp()),
        collective_participations::updated_at.eq(current_timestamp()),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Update failed: {}", e)))?;

    Ok(updated > 0)
}

// ============================================================================
// Stats
// ============================================================================

/// Get collective count for an app
pub fn collective_count(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
) -> Result<i64, StorageError> {
    collectives::table
        .filter(collectives::h_app_id.eq(&ctx.h_app_id))
        .filter(collectives::dissolved_at.is_null())
        .count()
        .get_result(conn)
        .map_err(|e| StorageError::Internal(format!("Count query failed: {}", e)))
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:collectives_db_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn make_ctx() -> AppContext {
        AppContext::new("test-app")
    }

    /// Wave 2 T1: new columns collective_cid + slug are persisted and readable on collectives.
    #[test]
    fn hub_identity_cid_slug_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        let input = CreateCollectiveInput {
            id: "family-dowell".to_string(),
            name: "Dowell Family".to_string(),
            description: None,
            governance_layer: "family".to_string(),
            constitutional_parent_id: None,
            reach: "private".to_string(),
            region: None,
            metadata_json: None,
            created_by: None,
        };

        // Insert via the existing upsert path (collective_cid + slug default to None)
        let collective = create_collective(&mut conn, &ctx, &input).expect("create");
        assert_eq!(collective.id, "family-dowell");
        assert!(
            collective.collective_cid.is_none(),
            "collective_cid NULL pre-coherence"
        );
        assert!(collective.slug.is_none(), "slug NULL when not set");

        // Directly write the new columns via diesel to prove they are writable + readable
        diesel::update(
            collectives::table
                .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                .filter(collectives::id.eq("family-dowell")),
        )
        .set((
            collectives::collective_cid.eq(Some("collective:uhCkkTestActionHash0001")),
            collectives::slug.eq(Some("family-dowell")),
        ))
        .execute(&mut conn)
        .expect("update cid+slug");

        let updated = get_collective(&mut conn, &ctx, "family-dowell")
            .expect("get")
            .expect("Some");

        assert_eq!(
            updated.collective_cid.as_deref(),
            Some("collective:uhCkkTestActionHash0001"),
            "collective_cid round-trips"
        );
        assert_eq!(
            updated.slug.as_deref(),
            Some("family-dowell"),
            "slug round-trips"
        );
    }

    /// D5 input: `region` set through the create-collective input path lands
    /// in the `collectives.region` column on insert AND is overwritten on the
    /// upsert update path. This is the column resilience's
    /// `compute_regional_distribution` reads for geographic-distribution
    /// bucketing — without a settable input it was structurally always NULL.
    #[test]
    fn region_threads_through_create_and_upsert() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        // Insert path: region carried straight onto the row.
        let input = CreateCollectiveInput {
            id: "collective-pnw".to_string(),
            name: "PNW Collective".to_string(),
            description: None,
            governance_layer: "community".to_string(),
            constitutional_parent_id: None,
            reach: "commons".to_string(),
            region: Some("us-pnw".to_string()),
            metadata_json: None,
            created_by: None,
        };
        let created = create_collective(&mut conn, &ctx, &input).expect("create");
        assert_eq!(
            created.region.as_deref(),
            Some("us-pnw"),
            "region lands in the column on insert"
        );

        // Upsert update path: same id, new region → column is overwritten.
        let updated_input = CreateCollectiveInput {
            region: Some("us-east".to_string()),
            ..input.clone()
        };
        let updated = create_collective(&mut conn, &ctx, &updated_input).expect("upsert");
        assert_eq!(
            updated.region.as_deref(),
            Some("us-east"),
            "region overwritten on upsert update"
        );

        // A creation with no region yields NULL (the honest unknown bucket).
        let no_region = CreateCollectiveInput {
            id: "collective-unknown".to_string(),
            region: None,
            ..input
        };
        let plain = create_collective(&mut conn, &ctx, &no_region).expect("create no-region");
        assert!(plain.region.is_none(), "absent region stays NULL");
    }

    /// Regression for the genesis #1105 jessica-alpha FK storm: account
    /// packages import on each human's own peer, but collective definitions
    /// are seeded to ONE peer — a participation insert on any other peer has
    /// no FK parent. The import path must materialize a stub (under the
    /// scope later writers use) instead of FK-failing; this pins the
    /// db-layer contract that path relies on.
    #[test]
    fn participation_without_parent_fk_fails_and_stub_materializes() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        // Mirror deployed SQLite semantics — the production storm proves FK
        // enforcement is on there; make the test independent of the
        // libsqlite3 compile-time default.
        diesel::sql_query("PRAGMA foreign_keys = ON")
            .execute(&mut conn)
            .expect("enable FK enforcement");
        let lamad_ctx = AppContext::new("lamad");

        let part_input = CreateParticipationInput {
            id: None,
            collective_id: "household-dowell".to_string(),
            human_id: "human-jessica-spouse".to_string(),
            intimacy_level: "connection".to_string(),
            role_context: None,
            governance_weight: 1.0,
            consent_state: "consented".to_string(),
            metadata_json: None,
        };

        // The storm shape: no parent row anywhere → FK constraint failure.
        let err = create_participation(&mut conn, &part_input)
            .expect_err("participation without FK parent must fail");
        assert!(
            err.to_string().contains("FOREIGN KEY constraint failed"),
            "expected FK failure, got: {err}"
        );

        // The fix shape: app-scope-agnostic probe → stub under lamad scope →
        // the qahal-scoped participation insert lands (FK is on bare id).
        assert!(!collective_id_exists(&mut conn, "household-dowell").expect("probe"));
        let stub = CreateCollectiveInput::stub("household-dowell");
        create_collective(&mut conn, &lamad_ctx, &stub).expect("stub create");
        assert!(collective_id_exists(&mut conn, "household-dowell").expect("probe"));
        create_participation(&mut conn, &part_input)
            .expect("participation lands once the stub parent exists");

        // Convergence: the authoritative projection's later lamad-scoped
        // upsert must hit the SAME row, not PK-collide on a second scope.
        let projected = CreateCollectiveInput {
            id: "household-dowell".to_string(),
            name: "Dowell Household".to_string(),
            description: Some("projected".to_string()),
            governance_layer: "family".to_string(),
            constitutional_parent_id: None,
            reach: "private".to_string(),
            region: None,
            metadata_json: None,
            created_by: None,
        };
        let converged =
            create_collective(&mut conn, &lamad_ctx, &projected).expect("projection upsert");
        assert_eq!(
            converged.name, "Dowell Household",
            "upsert updated the stub"
        );
    }

    // ========================================================================
    // Shared projection mapping + reconcile support
    // ========================================================================

    const CID_A: &str = "collective:uhCkkAlphaCollectiveActionHash00000000000";
    const CID_B: &str = "collective:uhCkkBravoCollectiveActionHash00000000000";

    fn seed_row(conn: &mut SqliteConnection, ctx: &AppContext, id: &str, cid: Option<&str>) {
        create_collective(
            conn,
            ctx,
            &CreateCollectiveInput {
                id: id.to_string(),
                name: format!("seed {id}"),
                description: None,
                governance_layer: "community".to_string(),
                constitutional_parent_id: None,
                reach: "community".to_string(),
                region: None,
                metadata_json: None,
                created_by: None,
            },
        )
        .expect("seed row");
        if let Some(cid) = cid {
            diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                    .filter(collectives::id.eq(id)),
            )
            .set(collectives::collective_cid.eq(Some(cid)))
            .execute(conn)
            .expect("stamp cid");
        }
    }

    /// Household coherence: `{"kind":"household"}` drives `family` + `trusted`;
    /// anything else (including a malformed charter) stays community. Parsing is
    /// total — a projection must land even for an unreadable charter.
    #[test]
    fn charter_hints_derive_governance_totally() {
        let household = CharterHints::parse(Some(r#"{"kind":"household","slugAlias":"fam-x"}"#));
        assert!(household.is_household());
        assert_eq!(household.governance_layer(), "family");
        assert_eq!(household.reach(), "trusted");
        assert_eq!(household.slug_alias(), Some("fam-x"));

        for charter in [
            None,
            Some("not json"),
            Some("{}"),
            Some(r#"{"kind":"guild"}"#),
        ] {
            let h = CharterHints::parse(charter);
            assert!(!h.is_household(), "charter {charter:?} is not a household");
            assert_eq!(h.governance_layer(), "community");
            assert_eq!(h.reach(), "community");
            assert_eq!(h.slug_alias(), None);
        }
        // An empty/whitespace alias is normalized away, never merged onto "".
        assert_eq!(
            CharterHints::parse(Some(r#"{"slugAlias":"   "}"#)).slug_alias(),
            None
        );
    }

    /// GapFill FILL: an absent collective is created cid-keyed from conductor
    /// truth (the AbsentLocal heal — a non-authoring peer acquiring the row).
    #[test]
    fn gapfill_creates_absent_collective_cid_keyed() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        let outcome = project_collective(
            &mut conn,
            &ctx,
            &CollectiveProjection {
                collective_cid: CID_A,
                display_name: "Dowell Household",
                founder_agent_cid: Some("uhCAkFounder0001"),
                charter: Some(r#"{"kind":"household"}"#),
                merge_onto_id: None,
                mode: CollectiveStampMode::GapFill,
            },
        )
        .expect("project");
        assert_eq!(outcome, CollectiveProjectionOutcome::Created);

        let row = get_collective(&mut conn, &ctx, CID_A)
            .expect("get")
            .expect("Some");
        assert_eq!(row.collective_cid.as_deref(), Some(CID_A));
        assert_eq!(row.name, "Dowell Household");
        assert_eq!(
            row.governance_layer, "family",
            "household charter drives family layer through the SHARED mapping"
        );
        assert_eq!(row.created_by.as_deref(), Some("uhCAkFounder0001"));
    }

    /// GapFill FILL onto an alias: a pre-coherence seed row (NULL cid) is
    /// stamped in place rather than duplicated — one household, one row. Both
    /// the charter's declared `slugAlias` and the reconcile's peer-advertised
    /// routing alias reach the same guard.
    #[test]
    fn gapfill_merges_onto_unanchored_alias_row_not_a_duplicate() {
        for use_charter_alias in [true, false] {
            let pool = test_pool();
            let mut conn = pool.get().expect("conn");
            let ctx = make_ctx();
            seed_row(&mut conn, &ctx, "household-dowell", None);

            let charter = if use_charter_alias {
                r#"{"kind":"household","slugAlias":"household-dowell"}"#.to_string()
            } else {
                r#"{"kind":"household"}"#.to_string()
            };
            let merge_onto = if use_charter_alias {
                None
            } else {
                Some("household-dowell")
            };

            let outcome = project_collective(
                &mut conn,
                &ctx,
                &CollectiveProjection {
                    collective_cid: CID_A,
                    display_name: "Dowell Household",
                    founder_agent_cid: None,
                    charter: Some(&charter),
                    merge_onto_id: merge_onto,
                    mode: CollectiveStampMode::GapFill,
                },
            )
            .expect("project");
            assert_eq!(outcome, CollectiveProjectionOutcome::AliasMerged);

            let row = get_collective(&mut conn, &ctx, "household-dowell")
                .expect("get")
                .expect("Some");
            assert_eq!(row.collective_cid.as_deref(), Some(CID_A));
            assert_eq!(row.slug.as_deref(), Some("household-dowell"));
            assert_eq!(row.governance_layer, "family");
            assert!(
                get_collective(&mut conn, &ctx, CID_A)
                    .expect("get")
                    .is_none(),
                "the alias merge must NOT also mint a cid-keyed duplicate"
            );
        }
    }

    /// REFRESH: a row already carrying this exact cid is refreshed, never
    /// duplicated and never counted as convergence.
    #[test]
    fn same_cid_is_refreshed_not_reconverged() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();
        seed_row(&mut conn, &ctx, "household-dowell", Some(CID_A));

        let outcome = project_collective(
            &mut conn,
            &ctx,
            &CollectiveProjection {
                collective_cid: CID_A,
                display_name: "Renamed Household",
                founder_agent_cid: None,
                charter: Some(r#"{"kind":"household"}"#),
                merge_onto_id: Some("household-dowell"),
                mode: CollectiveStampMode::GapFill,
            },
        )
        .expect("project");
        assert_eq!(outcome, CollectiveProjectionOutcome::Refreshed);

        let row = get_collective(&mut conn, &ctx, "household-dowell")
            .expect("get")
            .expect("Some");
        assert_eq!(row.name, "Renamed Household", "value fields refresh");
        assert_eq!(row.collective_cid.as_deref(), Some(CID_A), "cid unmoved");
    }

    /// REFUSE-MOVE: heal fills, never moves. An alias row already carrying a
    /// DIFFERENT cid is left untouched under `GapFill` — `collectives` has no
    /// declaration-ordering column, so no forward move is ever provable.
    /// `Declare` (the canonical post-commit channel) may still re-point it.
    #[test]
    fn gapfill_refuses_to_move_an_anchored_alias_declare_may() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();
        seed_row(&mut conn, &ctx, "household-dowell", Some(CID_A));

        let refused = project_collective(
            &mut conn,
            &ctx,
            &CollectiveProjection {
                collective_cid: CID_B,
                display_name: "Impostor",
                founder_agent_cid: None,
                charter: Some(r#"{"kind":"household","slugAlias":"household-dowell"}"#),
                merge_onto_id: Some("household-dowell"),
                mode: CollectiveStampMode::GapFill,
            },
        )
        .expect("project");
        assert_eq!(refused, CollectiveProjectionOutcome::SkippedDeclared);
        let row = get_collective(&mut conn, &ctx, "household-dowell")
            .expect("get")
            .expect("Some");
        assert_eq!(
            row.collective_cid.as_deref(),
            Some(CID_A),
            "heal must not move an anchored row"
        );
        assert_eq!(row.name, "seed household-dowell", "no value write either");

        // The canonical channel (post-commit signal) MAY re-point it.
        let declared = project_collective(
            &mut conn,
            &ctx,
            &CollectiveProjection {
                collective_cid: CID_B,
                display_name: "Re-pointed",
                founder_agent_cid: None,
                charter: Some(r#"{"kind":"household","slugAlias":"household-dowell"}"#),
                merge_onto_id: None,
                mode: CollectiveStampMode::Declare,
            },
        )
        .expect("project");
        assert_eq!(declared, CollectiveProjectionOutcome::AliasMerged);
        assert_eq!(
            get_collective(&mut conn, &ctx, "household-dowell")
                .expect("get")
                .expect("Some")
                .collective_cid
                .as_deref(),
            Some(CID_B)
        );
    }

    /// TOCTOU regression (review of commit 33225a8ae, LOW finding): two
    /// `project_collective` calls for the SAME `collective_cid`, routed
    /// through DIFFERENT local ids (the first mints the cid-keyed row via
    /// step 3; the second names a distinct, non-existent alias that would
    /// otherwise reach step 3's own insert path too) must converge onto ONE
    /// row, never mint a second cid-bearing row. The second call's outcome
    /// must be in the refresh/alias class (`Refreshed`), not `Created`.
    #[test]
    fn same_cid_via_different_ids_never_duplicates_row() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        // First call: no alias, no existing row → step 3 inserts a cid-keyed
        // row under id == CID_A.
        let first = project_collective(
            &mut conn,
            &ctx,
            &CollectiveProjection {
                collective_cid: CID_A,
                display_name: "Dowell Household",
                founder_agent_cid: None,
                charter: Some(r#"{"kind":"household"}"#),
                merge_onto_id: None,
                mode: CollectiveStampMode::GapFill,
            },
        )
        .expect("first project");
        assert_eq!(first, CollectiveProjectionOutcome::Created);

        // Second call: SAME cid, a DIFFERENT candidate local id via
        // merge_onto_id — one that names no local row, so absent the step-1
        // guard this call would fall through to step 3 and mint its own
        // second cid-keyed row (a duplicate under a different id).
        let second = project_collective(
            &mut conn,
            &ctx,
            &CollectiveProjection {
                collective_cid: CID_A,
                display_name: "Renamed Household",
                founder_agent_cid: None,
                charter: Some(r#"{"kind":"household"}"#),
                merge_onto_id: Some("household-not-yet-local"),
                mode: CollectiveStampMode::GapFill,
            },
        )
        .expect("second project");
        assert_eq!(
            second,
            CollectiveProjectionOutcome::Refreshed,
            "second call must converge (refresh), never mint a duplicate"
        );

        // Exactly one row in this app scope carries CID_A.
        let cid_bearing_count: i64 = collectives::table
            .filter(collectives::h_app_id.eq(&ctx.h_app_id))
            .filter(collectives::collective_cid.eq(CID_A))
            .count()
            .get_result(&mut conn)
            .expect("count");
        assert_eq!(
            cid_bearing_count, 1,
            "exactly one row must carry collective_cid = CID_A"
        );

        // The value fields converged from the second call.
        let row = get_collective(&mut conn, &ctx, CID_A)
            .expect("get")
            .expect("Some");
        assert_eq!(row.name, "Renamed Household");

        // And no row was ever created under the alias id that never existed.
        assert!(
            get_collective(&mut conn, &ctx, "household-not-yet-local")
                .expect("get")
                .is_none(),
            "the never-local alias id must never have been materialized"
        );
    }

    /// Ruling 1: the reconcile inventory advertises ONLY rows with a non-NULL,
    /// non-empty `collective_cid`, and never a dissolved one. A pre-coherence
    /// seed row has no DHT identity to reconcile on.
    #[test]
    fn inventory_excludes_null_empty_and_dissolved_cids() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_row(&mut conn, &ctx, "anchored", Some(CID_A));
        seed_row(&mut conn, &ctx, "null-cid", None);
        seed_row(&mut conn, &ctx, "empty-cid", Some(""));
        seed_row(&mut conn, &ctx, "dissolved", Some(CID_B));
        dissolve_collective(&mut conn, &ctx, "dissolved").expect("dissolve");
        // A row under a DIFFERENT app scope must not leak into this inventory.
        seed_row(
            &mut conn,
            &AppContext::new("other-app"),
            "foreign",
            Some(CID_B),
        );

        let (rows, total) =
            list_collective_cid_inventory(&mut conn, &ctx, 0, 100).expect("inventory");
        assert_eq!(
            rows,
            vec![("anchored".to_string(), CID_A.to_string())],
            "only the anchored, non-dissolved, in-scope row is advertised"
        );
        assert_eq!(total, 1, "honest total matches the filtered set");

        // Presence is scope-bound too, and reach-agnostic over cid state.
        let present = collective_ids_present(
            &mut conn,
            &ctx,
            &[
                "anchored".to_string(),
                "null-cid".to_string(),
                "foreign".to_string(),
                "nope".to_string(),
            ],
        )
        .expect("presence");
        assert!(present.contains("anchored"));
        assert!(
            present.contains("null-cid"),
            "an un-anchored row IS present — that is what makes it a CidGap, not AbsentLocal"
        );
        assert!(!present.contains("foreign"), "other scope is not present");
        assert!(!present.contains("nope"));
    }

    /// The inventory windows like the other reconcile tables: honest
    /// offset-invariant total, page-bounded rows.
    #[test]
    fn inventory_total_is_offset_invariant() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();
        for i in 0..5 {
            seed_row(
                &mut conn,
                &ctx,
                &format!("c{i}"),
                Some(&format!("{CID_A}{i}")),
            );
        }
        let (page, total) = list_collective_cid_inventory(&mut conn, &ctx, 0, 2).expect("page 0");
        assert_eq!(page.len(), 2);
        assert_eq!(total, 5, "total is the whole corpus, not the served page");
        let (page2, total2) = list_collective_cid_inventory(&mut conn, &ctx, 4, 2).expect("page 2");
        assert_eq!(page2.len(), 1, "final page carries the remainder");
        assert_eq!(total2, 5, "total is offset-invariant");
    }

    /// `id_for_collective_cid` / `collective_cid_of_id` are the two lookups the
    /// shared mapping and the reconcile diff are built on.
    #[test]
    fn cid_and_id_lookups_distinguish_absent_from_unanchored() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();
        seed_row(&mut conn, &ctx, "anchored", Some(CID_A));
        seed_row(&mut conn, &ctx, "null-cid", None);

        assert_eq!(
            id_for_collective_cid(&mut conn, &ctx, CID_A).expect("q"),
            Some("anchored".to_string())
        );
        assert_eq!(
            id_for_collective_cid(&mut conn, &ctx, CID_B).expect("q"),
            None
        );

        assert_eq!(
            collective_cid_of_id(&mut conn, &ctx, "anchored").expect("q"),
            Some(Some(CID_A.to_string()))
        );
        assert_eq!(
            collective_cid_of_id(&mut conn, &ctx, "null-cid").expect("q"),
            Some(None),
            "present-but-un-anchored is Some(None), NOT None"
        );
        assert_eq!(
            collective_cid_of_id(&mut conn, &ctx, "missing").expect("q"),
            None,
            "absent row is the outer None"
        );
    }

    /// Wave 2 T1: new columns member_cid + member_kind + dht_anchor_hash are persisted and
    /// readable on collective_participations.
    #[test]
    fn hub_membership_cid_columns_roundtrip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        // Seed the parent collective first (FK constraint)
        let collective_input = CreateCollectiveInput {
            id: "family-dowell".to_string(),
            name: "Dowell Family".to_string(),
            description: None,
            governance_layer: "family".to_string(),
            constitutional_parent_id: None,
            reach: "private".to_string(),
            region: None,
            metadata_json: None,
            created_by: None,
        };
        create_collective(&mut conn, &ctx, &collective_input).expect("create collective");

        // Create participation — new columns arrive as None/"person" defaults
        let part_input = CreateParticipationInput {
            id: Some("part-001".to_string()),
            collective_id: "family-dowell".to_string(),
            human_id: "human-alice".to_string(),
            intimacy_level: "recognition".to_string(),
            role_context: None,
            governance_weight: 1.0,
            consent_state: "consented".to_string(),
            metadata_json: None,
        };
        let part = create_participation(&mut conn, &part_input).expect("create participation");

        assert_eq!(
            part.member_kind, "person",
            "member_kind defaults to 'person'"
        );
        assert!(part.member_cid.is_none(), "member_cid NULL pre-coherence");
        assert!(
            part.dht_anchor_hash.is_none(),
            "dht_anchor_hash NULL pre-coherence"
        );

        // Directly write the new columns to prove they are writable + readable
        diesel::update(
            collective_participations::table.filter(collective_participations::id.eq("part-001")),
        )
        .set((
            collective_participations::member_cid.eq(Some("agent:uhCAkAlicePubKey0001")),
            collective_participations::member_kind.eq("person"),
            collective_participations::dht_anchor_hash.eq(Some("uhCkkMembershipActionHash0001")),
        ))
        .execute(&mut conn)
        .expect("update member cols");

        let updated: CollectiveParticipation = collective_participations::table
            .filter(collective_participations::id.eq("part-001"))
            .first(&mut conn)
            .expect("fetch updated");

        assert_eq!(
            updated.member_cid.as_deref(),
            Some("agent:uhCAkAlicePubKey0001"),
            "member_cid round-trips"
        );
        assert_eq!(updated.member_kind, "person", "member_kind round-trips");
        assert_eq!(
            updated.dht_anchor_hash.as_deref(),
            Some("uhCkkMembershipActionHash0001"),
            "dht_anchor_hash round-trips"
        );
    }

    // ========================================================================
    // gap_fill_household_collective_cid_via_membership
    // (backlog-collectives-arm-bootstrap-gap-no-stamped-cid-anywhere)
    //
    // Cardinality alone cannot pick the row: the genesis seed data plants MANY
    // un-anchored `governance_layer=family` rows locally (one per real+fixture
    // household in the shared reference dataset every doorway seeds), so these
    // scenarios exercise the DETERMINISTIC membership join instead — resolving
    // through EXISTING `humans` rows (the same agent-key lookup
    // `identity_fill::apply_membership_fill`'s short-circuit performs).
    // ========================================================================

    const DISCOVERED_CID: &str = "collective:uhCkkDiscoveredActionHash00000000000000";
    const OTHER_CID: &str = "collective:uhCkkOtherActionHash0000000000000000000";

    fn seed_family_row(conn: &mut SqliteConnection, ctx: &AppContext, id: &str, cid: Option<&str>) {
        create_collective(
            conn,
            ctx,
            &CreateCollectiveInput {
                id: id.to_string(),
                name: format!("seed {id}"),
                description: None,
                governance_layer: governance_layers::FAMILY.to_string(),
                constitutional_parent_id: None,
                reach: "trusted".to_string(),
                region: None,
                metadata_json: None,
                created_by: None,
            },
        )
        .expect("seed family row");
        if let Some(cid) = cid {
            diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                    .filter(collectives::id.eq(id)),
            )
            .set(collectives::collective_cid.eq(Some(cid)))
            .execute(conn)
            .expect("stamp cid");
        }
    }

    /// Seed a `humans` row that ALREADY carries an agent key — the shape a
    /// genesis-self-healed member (`genesis_self_heal_identity`) or an earlier
    /// identity_fill CREATE/FILL leaves behind. `household_id` is whatever the
    /// caller supplies (a SLUG like `household-dowell`, or a cid-form value to
    /// exercise the exclusion). The membership join looks these up via
    /// `get_human_by_agent_key`, h_app_id-agnostically, exactly as production
    /// does.
    fn seed_healed_member(
        conn: &mut SqliteConnection,
        id: &str,
        agent_pub_key: &str,
        household_id: Option<&str>,
    ) {
        use crate::db::humans::CreateHumanInput;
        crate::db::humans::create_human(
            conn,
            CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: Some(agent_pub_key.to_string()),
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: "imagodei".to_string(),
                household_id: household_id.map(|h| h.to_string()),
            },
        )
        .expect("seed healed member");
    }

    /// THE CORE BOOTSTRAP-GAP CASE: a genesis-self-healed member (matthew-
    /// shaped — agent key SET, `household_id` the SLUG `household-dowell`)
    /// resolves the join unambiguously; the pre-coherence seed row
    /// (governance_layer=family, collective_cid NULL) gets stamped. A second,
    /// not-yet-matched member (james) is present in the membership set but
    /// contributes nothing (no local row yet) — the join still resolves.
    #[test]
    fn gap_fill_stamps_the_row_resolved_by_a_single_slug() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", None);
        seed_healed_member(
            &mut conn,
            "human-matthew-manager",
            "uhCAkMATTHEW",
            Some("household-dowell"),
        );

        let member_cids = vec![
            "agent:uhCAkMATTHEW".to_string(),
            "agent:uhCAkJAMES".to_string(),
        ];
        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            DISCOVERED_CID,
            &member_cids,
        )
        .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::Stamped);

        let row = get_collective(&mut conn, &ctx, "household-dowell")
            .expect("query")
            .expect("row exists");
        assert_eq!(
            row.collective_cid.as_deref(),
            Some(DISCOVERED_CID),
            "the seed row's NULL collective_cid is stamped once the membership join resolves a single slug"
        );
        assert_eq!(row.id, "household-dowell", "the slug id is never moved");
    }

    /// Ambiguous shape: the household's members resolve to TWO DISTINCT slugs
    /// (a mis-seeded fixture, or two households whose members happen to be
    /// enumerated together) — this pass cannot safely tell which slug names
    /// the row for this discovered cid, so it abstains. Neither row is
    /// touched.
    #[test]
    fn gap_fill_ambiguous_when_members_resolve_two_distinct_slugs() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", None);
        seed_family_row(&mut conn, &ctx, "household-gertrude", None);
        seed_healed_member(
            &mut conn,
            "human-matthew-manager",
            "uhCAkMATTHEW",
            Some("household-dowell"),
        );
        seed_healed_member(
            &mut conn,
            "human-gertrude-grandma",
            "uhCAkGERTRUDE",
            Some("household-gertrude"),
        );

        let member_cids = vec![
            "agent:uhCAkMATTHEW".to_string(),
            "agent:uhCAkGERTRUDE".to_string(),
        ];
        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            DISCOVERED_CID,
            &member_cids,
        )
        .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::Ambiguous);

        for id in ["household-dowell", "household-gertrude"] {
            let row = get_collective(&mut conn, &ctx, id)
                .expect("query")
                .expect("row exists");
            assert!(
                row.collective_cid.is_none(),
                "ambiguous pairing must never guess-write either candidate"
            );
        }
    }

    /// No member of this household resolves to ANY local `humans` row —
    /// nothing to safely stamp onto yet, a clean no-op (never guessed).
    #[test]
    fn gap_fill_no_member_matches_any_row_is_a_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", None);

        let member_cids = vec!["agent:uhCAkUNKNOWN".to_string()];
        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            DISCOVERED_CID,
            &member_cids,
        )
        .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::NoCandidateRow);

        let row = get_collective(&mut conn, &ctx, "household-dowell")
            .expect("query")
            .expect("row exists");
        assert!(row.collective_cid.is_none());
    }

    /// EXCLUSION: a matched member's `household_id` is CID-FORM (the shape a
    /// row THIS SAME identity_fill pass already created/filled carries,
    /// `household_id = collective_cid` per `controller.rs:1085`) — it must be
    /// excluded from the slug candidate set, never poisoning or blocking the
    /// join. Paired with one genuine slug match, the join still resolves
    /// cleanly to that lone slug.
    #[test]
    fn gap_fill_excludes_cid_form_household_id_from_the_slug_join() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", None);
        // A member row THIS pass already lit — household_id is the raw cid,
        // never a slug.
        seed_healed_member(
            &mut conn,
            "agent:uhCAkJAMES",
            "uhCAkJAMES",
            Some(DISCOVERED_CID),
        );
        // The genuine slug-resolving member.
        seed_healed_member(
            &mut conn,
            "human-matthew-manager",
            "uhCAkMATTHEW",
            Some("household-dowell"),
        );

        let member_cids = vec![
            "agent:uhCAkJAMES".to_string(),
            "agent:uhCAkMATTHEW".to_string(),
        ];
        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            DISCOVERED_CID,
            &member_cids,
        )
        .expect("gap fill");
        assert_eq!(
            outcome,
            HouseholdCidGapFillOutcome::Stamped,
            "the cid-form value must not poison or block the single genuine slug match"
        );

        let row = get_collective(&mut conn, &ctx, "household-dowell")
            .expect("query")
            .expect("row exists");
        assert_eq!(row.collective_cid.as_deref(), Some(DISCOVERED_CID));
    }

    /// If EVERY matched member carries only a cid-form `household_id` (no
    /// slug resolves at all yet), there is nothing safe to stamp onto — a
    /// clean no-op, not a false Ambiguous or a guessed Stamp.
    #[test]
    fn gap_fill_all_cid_form_matches_is_a_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", None);
        seed_healed_member(
            &mut conn,
            "agent:uhCAkJAMES",
            "uhCAkJAMES",
            Some(DISCOVERED_CID),
        );

        let member_cids = vec!["agent:uhCAkJAMES".to_string()];
        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            DISCOVERED_CID,
            &member_cids,
        )
        .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::NoCandidateRow);

        let row = get_collective(&mut conn, &ctx, "household-dowell")
            .expect("query")
            .expect("row exists");
        assert!(row.collective_cid.is_none());
    }

    /// fills-never-moves: the resolved slug's row already carries a DIFFERENT
    /// cid than this discovery — reported as Mismatch and left completely
    /// untouched (never overwritten).
    #[test]
    fn gap_fill_leaves_a_different_cid_untouched_and_reports_mismatch() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", Some(OTHER_CID));
        seed_healed_member(
            &mut conn,
            "human-matthew-manager",
            "uhCAkMATTHEW",
            Some("household-dowell"),
        );

        let member_cids = vec!["agent:uhCAkMATTHEW".to_string()];
        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            DISCOVERED_CID,
            &member_cids,
        )
        .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::Mismatch);

        let row = get_collective(&mut conn, &ctx, "household-dowell")
            .expect("query")
            .expect("row exists");
        assert_eq!(
            row.collective_cid.as_deref(),
            Some(OTHER_CID),
            "the existing cid is never overwritten by a different discovery"
        );
    }

    /// The discovered cid already matches SOME local `collective_cid` (a
    /// re-run after a prior successful stamp, or a live signal already landed
    /// it) — idempotent no-op, resolved before the membership join even runs.
    #[test]
    fn gap_fill_already_known_cid_is_a_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", Some(DISCOVERED_CID));

        let outcome =
            gap_fill_household_collective_cid_via_membership(&mut conn, &ctx, DISCOVERED_CID, &[])
                .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::AlreadyKnown);
    }

    /// The resolved slug names NO local `collectives` row at all (a household
    /// whose member is healed but whose seed row never landed on this pod) —
    /// nothing to stamp onto, a clean no-op.
    #[test]
    fn gap_fill_resolved_slug_names_no_local_row_is_a_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_healed_member(
            &mut conn,
            "human-matthew-manager",
            "uhCAkMATTHEW",
            Some("household-dowell"),
        );

        let member_cids = vec!["agent:uhCAkMATTHEW".to_string()];
        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            DISCOVERED_CID,
            &member_cids,
        )
        .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::NoCandidateRow);
    }

    /// An empty discovered cid is a clean no-op (nothing to gap-fill from).
    #[test]
    fn gap_fill_empty_discovered_cid_is_a_noop() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");
        let ctx = make_ctx();

        seed_family_row(&mut conn, &ctx, "household-dowell", None);

        let outcome = gap_fill_household_collective_cid_via_membership(
            &mut conn,
            &ctx,
            "",
            &["agent:uhCAkMATTHEW".to_string()],
        )
        .expect("gap fill");
        assert_eq!(outcome, HouseholdCidGapFillOutcome::NoCandidateRow);
    }
}
