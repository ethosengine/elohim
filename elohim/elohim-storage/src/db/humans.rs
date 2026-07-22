//! Human identity CRUD operations using Diesel
//!
//! Manages mutable profile data for the imagodei pillar.
//! Cryptographic provenance (attestations, agent keys) lives in the Holochain DNA;
//! this layer owns the fast, queryable, offline-capable profile record.

use diesel::prelude::*;
use uuid::Uuid;

use super::diesel_schema::humans;
use super::models::{current_timestamp, Human, NewHuman};
use crate::error::StorageError;

// ============================================================================
// Input Types
// ============================================================================

/// Input for creating a human identity record
#[derive(Debug, Clone)]
pub struct CreateHumanInput {
    pub id: String,
    pub agent_pub_key: Option<String>,
    pub display_name: String,
    pub bio: Option<String>,
    /// JSON-serialised Vec<String>
    pub affinities: String,
    pub profile_reach: String,
    pub location: Option<String>,
    pub profile_photo_url: Option<String>,
    pub h_app_id: String,
    /// Household collective id (projection of collectives DHT entry with
    /// kind:household). None for humans outside a household grouping or
    /// where the source does not carry household signal.
    pub household_id: Option<String>,
}

/// Input for updating mutable profile fields (all optional)
#[derive(Debug, Clone, Default)]
pub struct UpdateHumanInput {
    pub display_name: Option<String>,
    pub bio: Option<String>,
    /// JSON-serialised Vec<String> — replaces the stored value when present
    pub affinities: Option<String>,
    pub profile_reach: Option<String>,
    pub location: Option<String>,
    pub profile_photo_url: Option<String>,
}

// ============================================================================
// CRUD Operations
// ============================================================================

/// Insert a new human identity record.
///
/// Returns the created `Human` row. Errors if the `id` already exists.
pub fn create_human(
    conn: &mut SqliteConnection,
    input: CreateHumanInput,
) -> Result<Human, StorageError> {
    let id = if input.id.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        input.id
    };

    let new_human = NewHuman {
        id: id.clone(),
        agent_pub_key: input.agent_pub_key,
        display_name: input.display_name,
        bio: input.bio,
        affinities: input.affinities,
        profile_reach: input.profile_reach,
        location: input.location,
        profile_photo_url: input.profile_photo_url,
        h_app_id: input.h_app_id,
        household_id: input.household_id,
    };

    diesel::insert_into(humans::table)
        .values(&new_human)
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to insert human: {}", e)))?;

    get_human_by_id(conn, &id)?
        .ok_or_else(|| StorageError::Internal("Human not found after insert".to_string()))
}

/// STOPGAP (resolver spec §3.4 / §5 step 1,
/// `genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md`):
/// heal a slug-keyed `humans` row whose `agent_pub_key` (and/or `household_id`)
/// is still NULL, filling only the currently-NULL columns from the supplied
/// values. NEVER overwrites a value that is already set.
///
/// Why this exists: production `humans` rows are created by `seed-humans.ts`
/// (`POST /api/v1/identity/register`) with `agentPubKey: null` — there was no
/// truthful agent-key source at seed time. The truthful `uhCAk…` key DOES exist
/// at runtime: each pod's conductor mints one, exposed via `GET /auth/me`. The
/// seeder now fetches it (`seed-provide-rows.ts`) and calls this heal path so
/// `humans.agent_pub_key` equals the `rea_commitments.provider` the snapshot
/// join compares against (`household_resilience.rs:172-174`). With the key
/// populated, the existing direct-equality join lights `commitment_backed`.
///
/// This is a strict subset of the durable resolver design (step 3 resolves
/// transport ids → agent_cid through `peer_identity_bindings`); it must not
/// overwrite a set key, so a later key rotation / supersede is never blocked.
///
/// Returns the (possibly-unchanged) row. `NotFound` if the id does not exist.
pub fn heal_human_identity(
    conn: &mut SqliteConnection,
    human_id: &str,
    agent_pub_key: Option<&str>,
    household_id: Option<&str>,
) -> Result<Human, StorageError> {
    let existing = get_human_by_id(conn, human_id)?
        .ok_or_else(|| StorageError::NotFound(format!("Human not found: {}", human_id)))?;

    // Fill only currently-NULL columns from the input. A set value is sacred —
    // never clobber it (a wrong overwrite breaks the snapshot join worse than
    // a NULL). An input of None for a column is also a no-op.
    let new_agent_pub_key = match (&existing.agent_pub_key, agent_pub_key) {
        (None, Some(k)) if !k.is_empty() => Some(k.to_string()),
        _ => existing.agent_pub_key.clone(),
    };
    let new_household_id = match (&existing.household_id, household_id) {
        (None, Some(h)) if !h.is_empty() => Some(h.to_string()),
        _ => existing.household_id.clone(),
    };

    // Skip the write entirely when nothing changed (idempotent re-runs).
    if new_agent_pub_key == existing.agent_pub_key && new_household_id == existing.household_id {
        return Ok(existing);
    }

    let now = current_timestamp();
    diesel::update(humans::table.filter(humans::id.eq(human_id)))
        .set((
            humans::agent_pub_key.eq(new_agent_pub_key),
            humans::household_id.eq(new_household_id),
            humans::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to heal human identity: {}", e)))?;

    get_human_by_id(conn, human_id)?
        .ok_or_else(|| StorageError::Internal("Human not found after heal".to_string()))
}

/// GENESIS BOOTSTRAP STOPGAP (rekey-drift self-heal): force the SELF human row's
/// `agent_pub_key` to the pod's CURRENT conductor cell key when a re-key has left
/// the stored key stale.
///
/// Unlike [`heal_human_identity`] (NULL-only, "never clobber a set value"), this
/// deliberately OVERWRITES a set value — but ONLY toward the pod's OWN truthful
/// cell key, and ONLY for the caller-supplied `human_id` (SELF; the caller passes
/// `SELF_HUMAN_ID`). This exists because a non-prod DNA reinstall mints a new
/// `AgentPubKey` while the projected `humans.agent_pub_key` still holds the
/// fossil key — freezing the resilience join on a degraded peer_statuses row.
/// All other columns (household_id, display_name, …) are preserved.
///
/// Idempotent: a no-op (no write) when the stored key already equals `new_key`.
/// Returns the (possibly-unchanged) row; `NotFound` if the id does not exist.
pub fn rekey_human_agent_key(
    conn: &mut SqliteConnection,
    human_id: &str,
    new_key: &str,
) -> Result<Human, StorageError> {
    let existing = get_human_by_id(conn, human_id)?
        .ok_or_else(|| StorageError::NotFound(format!("Human not found: {}", human_id)))?;

    // Already current — skip the write (idempotent re-runs).
    if existing.agent_pub_key.as_deref() == Some(new_key) {
        return Ok(existing);
    }

    let now = current_timestamp();
    diesel::update(humans::table.filter(humans::id.eq(human_id)))
        .set((
            humans::agent_pub_key.eq(Some(new_key)),
            humans::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to rekey human agent key: {}", e)))?;

    get_human_by_id(conn, human_id)?
        .ok_or_else(|| StorageError::Internal("Human not found after rekey".to_string()))
}

/// MEMBERSHIP-TRUTH SUPERSEDE (rekey-drift for NON-self humans): overwrite a
/// human row's SET-but-stale `agent_pub_key` from `old_key` to `new_key` when DHT
/// membership truth carries a newer key than the local projection holds.
///
/// This is the NON-self analogue of [`rekey_human_agent_key`] (which is SELF-only,
/// matches only by `human_id`, and blindly overwrites toward the pod's own cell
/// key). The two are deliberately distinct functions:
///
/// - `rekey_human_agent_key` — genesis_self_heal's SELF row, from the pod's OWN
///   cell key. LEFT UNTOUCHED by this change.
/// - `supersede_human_agent_key` — any membership-backed row, from DHT membership
///   truth, guarded by a COMPARE-AND-SWAP on `old_key`.
///
/// The CAS (`agent_pub_key == old_key` in the WHERE) is load-bearing: the caller
/// resolves `(human_id, old_key)` from a household-scoped read that may be stale
/// by the time the write lands. Matching on `old_key` means a concurrent write
/// that already moved the key aborts this supersede (0 rows) instead of clobbering
/// a fresher value — it never regresses a key it did not itself observe.
///
/// The caller MUST have already established `new_key` is a valid `agent_cid`
/// (`identity_namespace::is_agent_cid`) — this fn additionally refuses an empty
/// `new_key` or a no-op (`old_key == new_key`) as defense in depth.
///
/// Returns the number of rows updated (0 = the CAS precondition no longer holds;
/// a benign, idempotent outcome — never an error).
pub fn supersede_human_agent_key(
    conn: &mut SqliteConnection,
    human_id: &str,
    old_key: &str,
    new_key: &str,
) -> Result<usize, StorageError> {
    if new_key.is_empty() || old_key == new_key {
        return Ok(0);
    }

    let now = current_timestamp();
    diesel::update(
        humans::table
            .filter(humans::id.eq(human_id))
            // COMPARE-AND-SWAP: only move the key we actually observed.
            .filter(humans::agent_pub_key.eq(old_key)),
    )
    .set((
        humans::agent_pub_key.eq(Some(new_key)),
        humans::updated_at.eq(&now),
    ))
    .execute(conn)
    .map_err(|e| StorageError::Internal(format!("Failed to supersede human agent key: {}", e)))
}

/// Retrieve a human by its stable ID.
pub fn get_human_by_id(
    conn: &mut SqliteConnection,
    id: &str,
) -> Result<Option<Human>, StorageError> {
    humans::table
        .filter(humans::id.eq(id))
        .first::<Human>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch human by id: {}", e)))
}

/// Retrieve a human by Holochain agent public key.
pub fn get_human_by_agent_key(
    conn: &mut SqliteConnection,
    agent_pub_key: &str,
) -> Result<Option<Human>, StorageError> {
    humans::table
        .filter(humans::agent_pub_key.eq(agent_pub_key))
        .first::<Human>(conn)
        .optional()
        .map_err(|e| StorageError::Internal(format!("Failed to fetch human by agent key: {}", e)))
}

/// List all humans for a given app, ordered by display name.
pub fn list_humans(
    conn: &mut SqliteConnection,
    h_app_id: &str,
) -> Result<Vec<Human>, diesel::result::Error> {
    use crate::db::diesel_schema::humans::dsl;
    dsl::humans
        .filter(dsl::h_app_id.eq(h_app_id))
        .order(dsl::display_name.asc())
        .load::<Human>(conn)
}

/// Update mutable profile fields for an existing human.
///
/// Only fields present in `input` (i.e., `Some(...)`) are written.
/// Returns the updated row, or `NotFound` if the ID does not exist.
pub fn update_human(
    conn: &mut SqliteConnection,
    id: &str,
    input: UpdateHumanInput,
) -> Result<Human, StorageError> {
    let now = current_timestamp();

    // Fetch existing row first so we can fill in fields not supplied by the caller.
    let existing = get_human_by_id(conn, id)?
        .ok_or_else(|| StorageError::NotFound(format!("Human not found: {}", id)))?;

    let display_name = input.display_name.unwrap_or(existing.display_name);
    let bio = input.bio.or(existing.bio);
    let affinities = input.affinities.unwrap_or(existing.affinities);
    let profile_reach = input.profile_reach.unwrap_or(existing.profile_reach);
    let location = input.location.or(existing.location);
    let profile_photo_url = input.profile_photo_url.or(existing.profile_photo_url);

    let rows_affected = diesel::update(humans::table.filter(humans::id.eq(id)))
        .set((
            humans::display_name.eq(display_name),
            humans::bio.eq(bio),
            humans::affinities.eq(affinities),
            humans::profile_reach.eq(profile_reach),
            humans::location.eq(location),
            humans::profile_photo_url.eq(profile_photo_url),
            humans::updated_at.eq(&now),
        ))
        .execute(conn)
        .map_err(|e| StorageError::Internal(format!("Failed to update human: {}", e)))?;

    if rows_affected == 0 {
        return Err(StorageError::NotFound(format!("Human not found: {}", id)));
    }

    get_human_by_id(conn, id)?
        .ok_or_else(|| StorageError::Internal("Human not found after update".to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:humans_db_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// Insert a slug-keyed human with NULL agent_pub_key + NULL household_id —
    /// the exact production shape after `seed-humans.ts` registers with
    /// `agentPubKey: null` (it sets household_id; we leave both NULL here to
    /// exercise both heal arms).
    fn insert_null_human(conn: &mut SqliteConnection, id: &str) {
        create_human(
            conn,
            CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: None,
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: "imagodei".to_string(),
                household_id: None,
            },
        )
        .expect("insert null human");
    }

    /// STOPGAP heal (resolver spec §3.4): a NULL agent_pub_key heals to the
    /// supplied uhCAk key, and NULL household_id heals too — both currently-NULL
    /// columns fill from the input.
    #[test]
    fn heal_fills_null_agent_key_and_household() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_null_human(&mut conn, "human-matthew-manager");

        let healed = heal_human_identity(
            &mut conn,
            "human-matthew-manager",
            Some("uhCAkMATTHEW"),
            Some("household-dowell"),
        )
        .expect("heal");

        assert_eq!(healed.agent_pub_key.as_deref(), Some("uhCAkMATTHEW"));
        assert_eq!(healed.household_id.as_deref(), Some("household-dowell"));
    }

    /// `list_humans` under the canonical scope returns the imagodei rows the
    /// production writers create. The `/db/humans` handler delegates to this with
    /// `HUMANS_HAPP_ID` (NOT the operating ctx) — pinning the contract that the
    /// 2026-06-19 dark-card probe's empty `/db/humans` was a read-scope artifact.
    #[test]
    fn list_humans_under_canonical_scope_finds_imagodei_rows() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        create_human(
            &mut conn,
            CreateHumanInput {
                id: "h-1".to_string(),
                agent_pub_key: Some("uhCAk-a".to_string()),
                display_name: "A".to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: crate::db::context::HUMANS_HAPP_ID.to_string(),
                household_id: Some("hh-1".to_string()),
            },
        )
        .unwrap();
        let rows = list_humans(&mut conn, crate::db::context::HUMANS_HAPP_ID).unwrap();
        assert_eq!(
            rows.len(),
            1,
            "list_humans under the canonical scope returns the imagodei row"
        );
    }

    /// STOPGAP invariant (resolver spec §3.4, §3.5): heal NEVER overwrites a
    /// value that is already set. A second heal with a *different* key is a
    /// no-op on the set column — this is load-bearing so a later key
    /// rotation/supersede (resolver step 3) is not silently clobbered.
    #[test]
    fn heal_never_overwrites_a_set_value() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_null_human(&mut conn, "human-adam-firstman");

        // First heal seeds the key.
        let first = heal_human_identity(
            &mut conn,
            "human-adam-firstman",
            Some("uhCAkADAM_ORIGINAL"),
            Some("household-eden"),
        )
        .expect("first heal");
        assert_eq!(first.agent_pub_key.as_deref(), Some("uhCAkADAM_ORIGINAL"));
        assert_eq!(first.household_id.as_deref(), Some("household-eden"));

        // Second heal with a DIFFERENT key + household must NOT overwrite.
        let second = heal_human_identity(
            &mut conn,
            "human-adam-firstman",
            Some("uhCAkADAM_DIFFERENT"),
            Some("household-OTHER"),
        )
        .expect("second heal");
        assert_eq!(
            second.agent_pub_key.as_deref(),
            Some("uhCAkADAM_ORIGINAL"),
            "set agent_pub_key must never be overwritten"
        );
        assert_eq!(
            second.household_id.as_deref(),
            Some("household-eden"),
            "set household_id must never be overwritten"
        );
    }

    /// Idempotent re-run: healing with the SAME values changes nothing and does
    /// not error.
    #[test]
    fn heal_is_idempotent() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_null_human(&mut conn, "human-jessica-spouse");

        heal_human_identity(
            &mut conn,
            "human-jessica-spouse",
            Some("uhCAkJESSICA"),
            None,
        )
        .expect("heal once");
        let again = heal_human_identity(
            &mut conn,
            "human-jessica-spouse",
            Some("uhCAkJESSICA"),
            None,
        )
        .expect("heal twice");
        assert_eq!(again.agent_pub_key.as_deref(), Some("uhCAkJESSICA"));
    }

    /// A heal against an absent id is NotFound (never a silent insert).
    #[test]
    fn heal_missing_human_is_not_found() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let err = heal_human_identity(&mut conn, "human-ghost", Some("uhCAkX"), None);
        assert!(matches!(err, Err(StorageError::NotFound(_))));
    }

    /// Insert a slug-keyed human with a SET agent_pub_key — the post-rekey fossil
    /// shape for a NON-self human (a membership stamp landed the then-live key,
    /// then the peer re-keyed and nothing overwrote it).
    fn insert_keyed_human(conn: &mut SqliteConnection, id: &str, agent_key: &str, household: &str) {
        create_human(
            conn,
            CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: Some(agent_key.to_string()),
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: "imagodei".to_string(),
                household_id: Some(household.to_string()),
            },
        )
        .expect("insert keyed human");
    }

    /// SUPERSEDE moves a SET-but-stale key to the membership key when the CAS
    /// precondition (current key == old_key) holds. Other fields are preserved.
    #[test]
    fn supersede_moves_set_key_via_cas() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_keyed_human(
            &mut conn,
            "human-adam-firstman",
            "uhCAkOLD",
            "collective:eden",
        );

        let n = supersede_human_agent_key(&mut conn, "human-adam-firstman", "uhCAkOLD", "uhCAkNEW")
            .expect("supersede");
        assert_eq!(n, 1, "the CAS matched and moved exactly one row");

        let row = get_human_by_id(&mut conn, "human-adam-firstman")
            .unwrap()
            .unwrap();
        assert_eq!(row.agent_pub_key.as_deref(), Some("uhCAkNEW"));
        assert_eq!(
            row.household_id.as_deref(),
            Some("collective:eden"),
            "household_id preserved through supersede"
        );
    }

    /// SUPERSEDE is a no-op when the row's CURRENT key differs from the observed
    /// `old_key` — a concurrent write already moved it, so this stale supersede
    /// must NOT clobber the fresher value.
    #[test]
    fn supersede_cas_aborts_when_current_key_differs() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // The row already holds a THIRD key (someone moved it after we read).
        insert_keyed_human(
            &mut conn,
            "human-adam-firstman",
            "uhCAkFRESH",
            "collective:eden",
        );

        let n = supersede_human_agent_key(&mut conn, "human-adam-firstman", "uhCAkOLD", "uhCAkNEW")
            .expect("supersede");
        assert_eq!(n, 0, "CAS precondition failed → no write");

        let row = get_human_by_id(&mut conn, "human-adam-firstman")
            .unwrap()
            .unwrap();
        assert_eq!(
            row.agent_pub_key.as_deref(),
            Some("uhCAkFRESH"),
            "a fresher key must never be regressed by a stale supersede"
        );
    }

    /// SUPERSEDE refuses a no-op (`old == new`) and an empty `new_key` — defense
    /// in depth so a degenerate call never touches the row.
    #[test]
    fn supersede_rejects_noop_and_empty() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_keyed_human(
            &mut conn,
            "human-adam-firstman",
            "uhCAkOLD",
            "collective:eden",
        );

        assert_eq!(
            supersede_human_agent_key(&mut conn, "human-adam-firstman", "uhCAkOLD", "uhCAkOLD")
                .expect("noop"),
            0
        );
        assert_eq!(
            supersede_human_agent_key(&mut conn, "human-adam-firstman", "uhCAkOLD", "")
                .expect("empty"),
            0
        );
        let row = get_human_by_id(&mut conn, "human-adam-firstman")
            .unwrap()
            .unwrap();
        assert_eq!(row.agent_pub_key.as_deref(), Some("uhCAkOLD"));
    }
}
