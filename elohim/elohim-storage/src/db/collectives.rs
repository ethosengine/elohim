//! Collectives CRUD operations using Diesel with app scoping
//!
//! Collectives are governance contexts with graduated participation.
//! Unifies communities and organizations under a single model.

use diesel::prelude::*;
use serde::Deserialize;
use uuid::Uuid;

use super::context::AppContext;
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
// Participation Read Operations
// ============================================================================

/// Get all active participations for a human
pub fn get_participations_for_human(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    human_id: &str,
) -> Result<Vec<CollectiveParticipation>, StorageError> {
    collective_participations::table
        .filter(collective_participations::h_app_id.eq(&ctx.h_app_id))
        .filter(collective_participations::human_id.eq(human_id))
        .filter(collective_participations::departed_at.is_null())
        .order(collective_participations::joined_at.desc())
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("Query failed: {}", e)))
}

/// Get all active participants of a collective
pub fn get_participants_of_collective(
    conn: &mut SqliteConnection,
    ctx: &AppContext,
    collective_id: &str,
) -> Result<Vec<CollectiveParticipation>, StorageError> {
    collective_participations::table
        .filter(collective_participations::h_app_id.eq(&ctx.h_app_id))
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
    ctx: &AppContext,
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
        .filter(collective_participations::h_app_id.eq(&ctx.h_app_id))
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
        h_app_id: &ctx.h_app_id,
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
    ctx: &AppContext,
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
            .filter(collective_participations::h_app_id.eq(&ctx.h_app_id))
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
    ctx: &AppContext,
    collective_id: &str,
    human_id: &str,
) -> Result<bool, StorageError> {
    let updated = diesel::update(
        collective_participations::table
            .filter(collective_participations::h_app_id.eq(&ctx.h_app_id))
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
        let qahal_ctx = AppContext::new("qahal");

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
        let err = create_participation(&mut conn, &qahal_ctx, &part_input)
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
        create_participation(&mut conn, &qahal_ctx, &part_input)
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
        let part =
            create_participation(&mut conn, &ctx, &part_input).expect("create participation");

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
}
