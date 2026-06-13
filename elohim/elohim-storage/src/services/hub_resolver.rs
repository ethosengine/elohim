//! Hub identity resolver — Wave 2 T2 (read-side, inline diesel, no DNA).
//!
//! ## Source of truth
//!
//! All helpers here are READ-ONLY projections of data ultimately notarized on the DHT
//! (`Collective` / `Membership` entries in the imagodei DNA). The local SQL state
//! is Category-A2 derived-via-link — a queryable cache; the DHT is canonical.
//!
//! ## Join-key finding (pinned 2026-05-29)
//!
//! `agent_cid` (e.g. `agent:uhCAkXxx`) stored in `peer_identity_bindings.agent_cid` and
//! `content.created_by` joins to **`humans.id`** — NOT to `humans.agent_pub_key`.
//! Evidence: `peer_topology_view.rs:369` (`humans::id.eq(&agent_cid)`).
//! `humans.agent_pub_key` stores the raw pubkey WITHOUT the `agent:` prefix and is used
//! only for libp2p `peer_id` matching (e.g. `household_resilience.rs:271`).
//! No prefix-stripping is required when joining `agent_cid → humans.id`.
//!
//! ## Hub identity (operator decision 2026-05-29)
//!
//! Canonical hub id = `collective:{action_hash}` (`collectives.collective_cid`).
//! Legacy slug (`collectives.id`, e.g. `family-dowell`) is a first-class alias.
//! All resolver functions return `collective_cid` when populated, otherwise fall back
//! to the slug `id`. Callers remain representation-stable once the projector (T5)
//! populates `collective_cid`.

use diesel::prelude::*;

use crate::error::StorageError;

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Return the canonical hub identifier for the collective row pointed to by
/// `collectives.collective_cid` when set, otherwise fall back to `collectives.id`
/// (the slug).
fn hub_id_from_row(collective_cid: Option<String>, slug_id: String) -> String {
    collective_cid.unwrap_or(slug_id)
}

/// Resolve the owning household hub for an agent.
///
/// Chain: `agent_cid → humans.id (= humans row) → humans.household_id →
/// collectives row`. Returns `collective_cid` when the collective row has one
/// populated, otherwise the slug `id`. Returns `None` when no `humans` row
/// exists for the agent or the human has no `household_id`.
pub fn resolve_owning_hub(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> Result<Option<String>, StorageError> {
    use crate::db::diesel_schema::{collectives, humans};

    // Step 1: agent_cid → humans.household_id (agent_cid == humans.id per join-key finding).
    let household_id: Option<String> = humans::table
        .filter(humans::id.eq(agent_cid))
        .select(humans::household_id)
        .first::<Option<String>>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Database(format!("hub_resolver resolve_owning_hub humans: {e}"))
        })?
        .flatten();

    let slug = match household_id {
        None => return Ok(None),
        Some(s) => s,
    };

    // Step 2: household_id (slug) → collectives row → prefer collective_cid.
    // Deliberate degradation: if the FK dangles (collectives row absent — e.g.
    // projection lag before T5 lands the Collective projector), `row` is None and
    // we return Ok(None), so callers fall back to agent_cid exactly as in the
    // "no household" path.
    let row: Option<(Option<String>,)> = collectives::table
        .filter(collectives::id.eq(&slug))
        .select((collectives::collective_cid,))
        .first::<(Option<String>,)>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Database(format!("hub_resolver resolve_owning_hub collectives: {e}"))
        })?;

    Ok(row.map(|(cid,)| hub_id_from_row(cid, slug)))
}

/// Resolve the dwelling hub for a libp2p peer.
///
/// Chain: `peer_id → agent_cid (active peer_identity_bindings row) →
/// dwelling_hub_id` via [`resolve_owning_hub`]. This is the reusable
/// `peer_id → agent_cid → dwelling_hub_id` mapping the tiered-quilt design
/// (§2 "Maturity path") asks for explicitly — the same derivation serves the
/// placement prioritizer's `recipient_hub_id` hint AND device→hub metric
/// aggregation; build it once, not per-feature.
///
/// Returns `None` when the peer has no active identity binding, or the bound
/// agent resolves to no household hub (same degradation contract as
/// [`resolve_owning_hub`]).
pub fn resolve_peer_dwelling_hub(
    conn: &mut SqliteConnection,
    peer_id: &str,
    now_iso: &str,
) -> Result<Option<String>, StorageError> {
    let Some(binding) = crate::db::peer_identity_bindings::lookup_active(conn, peer_id, now_iso)?
    else {
        return Ok(None);
    };
    resolve_owning_hub(conn, &binding.agent_cid)
}

/// Return all collectives the agent actively participates in.
///
/// Uses a two-path query over `collective_participations`:
/// 1. **CID path** (new canonical): rows where `member_cid == agent_cid`.
/// 2. **Human-id fallback** (seed rows): rows where `human_id == humans.id`
///    for the agent (i.e. when `member_cid` is NULL for that row).
///
/// For each matched collective the returned id is `collective_cid` if set,
/// otherwise the slug `id`. Active only (`departed_at IS NULL`).
pub fn resolve_member_collectives(
    conn: &mut SqliteConnection,
    agent_cid: &str,
) -> Result<Vec<String>, StorageError> {
    use crate::db::diesel_schema::{collective_participations, collectives, humans};

    // Resolve agent_cid → humans.id (same column — they are equal).
    let human_id: Option<String> = humans::table
        .filter(humans::id.eq(agent_cid))
        .select(humans::id)
        .first::<String>(conn)
        .optional()
        .map_err(|e| {
            StorageError::Database(format!(
                "hub_resolver resolve_member_collectives humans: {e}"
            ))
        })?;

    // Gather collective_ids from both paths, then deduplicate.
    // Path A: member_cid canonical match.
    let mut collective_ids: Vec<String> = collective_participations::table
        .filter(collective_participations::member_cid.eq(agent_cid))
        .filter(collective_participations::departed_at.is_null())
        .select(collective_participations::collective_id)
        .load::<String>(conn)
        .map_err(|e| {
            StorageError::Database(format!(
                "hub_resolver resolve_member_collectives cid-path: {e}"
            ))
        })?;

    // Path B: human_id fallback (only for rows where member_cid IS NULL, to avoid
    // double-counting rows that were already returned by Path A).
    if let Some(ref hid) = human_id {
        let fallback: Vec<String> = collective_participations::table
            .filter(collective_participations::human_id.eq(hid))
            .filter(collective_participations::member_cid.is_null())
            .filter(collective_participations::departed_at.is_null())
            .select(collective_participations::collective_id)
            .load::<String>(conn)
            .map_err(|e| {
                StorageError::Database(format!(
                    "hub_resolver resolve_member_collectives human-id-path: {e}"
                ))
            })?;
        collective_ids.extend(fallback);
    }

    // Deduplicate (preserve first occurrence order).
    let mut seen = std::collections::HashSet::new();
    let deduped: Vec<String> = collective_ids
        .into_iter()
        .filter(|id| seen.insert(id.clone()))
        .collect();

    // Resolve all collective slugs → canonical ids in ONE query (avoid N+1 —
    // Wave 3 calls this per-blob in the gossip receive hot path).
    let cid_rows: Vec<(String, Option<String>)> = collectives::table
        .filter(collectives::id.eq_any(&deduped))
        .select((collectives::id, collectives::collective_cid))
        .load::<(String, Option<String>)>(conn)
        .map_err(|e| {
            StorageError::Database(format!(
                "hub_resolver resolve_member_collectives collective lookup: {e}"
            ))
        })?;
    let cid_map: std::collections::HashMap<String, Option<String>> = cid_rows.into_iter().collect();

    // Map each slug → canonical id (collective_cid if set, else the slug — also
    // covers a slug missing from collectives), preserving first-occurrence order.
    let result: Vec<String> = deduped
        .into_iter()
        .map(|slug| {
            let cid = cid_map.get(&slug).cloned().flatten();
            hub_id_from_row(cid, slug)
        })
        .collect();

    Ok(result)
}

/// Return the member identities for a hub, given a hub id that may be a
/// `collective:{hash}` CID **or** a slug.
///
/// Resolution: match `collectives.collective_cid == hub_id OR collectives.id == hub_id`.
/// Returns member identities as `member_cid` when set, otherwise `human_id`.
/// Active only (`departed_at IS NULL`).
pub fn members_of_hub(
    conn: &mut SqliteConnection,
    hub_id: &str,
) -> Result<Vec<String>, StorageError> {
    use crate::db::diesel_schema::{collective_participations, collectives};

    // Resolve hub_id → collectives.id (slug) — the slug is the FK in
    // collective_participations. CID-first, then slug-fallback: deterministic even
    // if a slug ever collides with another row's collective_cid (the `.or()` +
    // `.first()` form could non-deterministically return the wrong row).
    let slug: Option<String> = {
        let by_cid: Option<String> = collectives::table
            .filter(collectives::collective_cid.eq(hub_id))
            .select(collectives::id)
            .first::<String>(conn)
            .optional()
            .map_err(|e| {
                StorageError::Database(format!("hub_resolver members_of_hub cid lookup: {e}"))
            })?;
        match by_cid {
            Some(s) => Some(s),
            None => collectives::table
                .filter(collectives::id.eq(hub_id))
                .select(collectives::id)
                .first::<String>(conn)
                .optional()
                .map_err(|e| {
                    StorageError::Database(format!("hub_resolver members_of_hub slug lookup: {e}"))
                })?,
        }
    };

    let slug = match slug {
        None => return Ok(vec![]),
        Some(s) => s,
    };

    // Load participants from collective_participations keyed by slug.
    let rows: Vec<(String, Option<String>)> = collective_participations::table
        .filter(collective_participations::collective_id.eq(&slug))
        .filter(collective_participations::departed_at.is_null())
        .select((
            collective_participations::human_id,
            collective_participations::member_cid,
        ))
        .load(conn)
        .map_err(|e| {
            StorageError::Database(format!(
                "hub_resolver members_of_hub participations query: {e}"
            ))
        })?;

    Ok(rows
        .into_iter()
        .map(|(human_id, member_cid)| member_cid.unwrap_or(human_id))
        .collect())
}

/// Resolve a slug to its canonical `collective:{hash}` CID.
///
/// Returns `None` when the collective row does not exist or `collective_cid` is NULL.
pub fn slug_to_cid(
    conn: &mut SqliteConnection,
    slug: &str,
) -> Result<Option<String>, StorageError> {
    use crate::db::diesel_schema::collectives;

    collectives::table
        .filter(collectives::id.eq(slug))
        .select(collectives::collective_cid)
        .first::<Option<String>>(conn)
        .optional()
        .map(|opt| opt.flatten())
        .map_err(|e| StorageError::Database(format!("hub_resolver slug_to_cid: {e}")))
}

/// Resolve a canonical `collective:{hash}` CID to its slug.
///
/// Returns `None` when no row with that `collective_cid` is found.
pub fn cid_to_slug(conn: &mut SqliteConnection, cid: &str) -> Result<Option<String>, StorageError> {
    use crate::db::diesel_schema::collectives;

    collectives::table
        .filter(collectives::collective_cid.eq(cid))
        .select(collectives::id)
        .first::<String>(conn)
        .optional()
        .map_err(|e| StorageError::Database(format!("hub_resolver cid_to_slug: {e}")))
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::collectives::{
        create_collective, create_participation, CreateCollectiveInput, CreateParticipationInput,
    };
    use crate::db::humans::CreateHumanInput;
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    fn test_pool() -> DbPool {
        let url = format!(
            "file:hub_resolver_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    fn ctx() -> crate::db::context::AppContext {
        crate::db::context::AppContext::new("test-app")
    }

    /// Seed a collective row with a slug id; optionally populate collective_cid.
    fn seed_collective(conn: &mut SqliteConnection, slug: &str, cid: Option<&str>) {
        let app_ctx = ctx();
        let input = CreateCollectiveInput {
            id: slug.to_string(),
            name: format!("{slug} collective"),
            description: None,
            governance_layer: "family".to_string(),
            constitutional_parent_id: None,
            reach: "private".to_string(),
            region: None,
            metadata_json: None,
            created_by: None,
        };
        create_collective(conn, &app_ctx, &input).expect("seed collective");

        if let Some(c) = cid {
            use crate::db::diesel_schema::collectives;
            diesel::update(
                collectives::table
                    .filter(collectives::h_app_id.eq(&app_ctx.h_app_id))
                    .filter(collectives::id.eq(slug)),
            )
            .set(collectives::collective_cid.eq(Some(c)))
            .execute(conn)
            .expect("set collective_cid");
        }
    }

    /// Seed a humans row. `agent_cid` is stored as `humans.id` (canonical join key).
    fn seed_human(conn: &mut SqliteConnection, agent_cid: &str, household_id: Option<&str>) {
        let input = CreateHumanInput {
            id: agent_cid.to_string(),
            agent_pub_key: None,
            display_name: format!("Human {agent_cid}"),
            bio: None,
            affinities: "[]".to_string(),
            profile_reach: "commons".to_string(),
            location: None,
            profile_photo_url: None,
            h_app_id: "test-app".to_string(),
            household_id: household_id.map(|s| s.to_string()),
        };
        crate::db::humans::create_human(conn, input).expect("seed human");
    }

    /// Seed a participation row via human_id (seed-data pattern, member_cid NULL).
    fn seed_participation_by_human_id(
        conn: &mut SqliteConnection,
        collective_slug: &str,
        human_id: &str,
    ) {
        let app_ctx = ctx();
        let input = CreateParticipationInput {
            id: Some(format!("part-{collective_slug}-{human_id}")),
            collective_id: collective_slug.to_string(),
            human_id: human_id.to_string(),
            intimacy_level: "recognition".to_string(),
            role_context: None,
            governance_weight: 1.0,
            consent_state: "consented".to_string(),
            metadata_json: None,
        };
        create_participation(conn, &app_ctx, &input).expect("seed participation");
    }

    /// Seed a participation row with member_cid set (CID-canonical path).
    fn seed_participation_with_cid(
        conn: &mut SqliteConnection,
        collective_slug: &str,
        human_id: &str,
        member_cid: &str,
    ) {
        use crate::db::diesel_schema::collective_participations;
        seed_participation_by_human_id(conn, collective_slug, human_id);
        let part_id = format!("part-{collective_slug}-{human_id}");
        diesel::update(
            collective_participations::table.filter(collective_participations::id.eq(&part_id)),
        )
        .set(collective_participations::member_cid.eq(Some(member_cid)))
        .execute(conn)
        .expect("set member_cid");
    }

    // ─── resolve_owning_hub ────────────────────────────────────────────────────

    /// T2-1: agent with household that has collective_cid set → returns CID.
    #[test]
    fn resolve_owning_hub_returns_cid_when_set() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            Some("collective:uhCkkTestHash001"),
        );
        seed_human(&mut conn, "agent:uhCAkAlice001", Some("family-dowell"));

        let hub = resolve_owning_hub(&mut conn, "agent:uhCAkAlice001")
            .expect("no error")
            .expect("Some");

        assert_eq!(hub, "collective:uhCkkTestHash001", "returns CID when set");
    }

    /// T2-2: agent with household that has NO collective_cid → returns slug.
    #[test]
    fn resolve_owning_hub_returns_slug_when_cid_null() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", None);
        seed_human(&mut conn, "agent:uhCAkAlice002", Some("family-dowell"));

        let hub = resolve_owning_hub(&mut conn, "agent:uhCAkAlice002")
            .expect("no error")
            .expect("Some");

        assert_eq!(hub, "family-dowell", "falls back to slug when CID is NULL");
    }

    /// T2-3: agent with no household → returns None.
    #[test]
    fn resolve_owning_hub_returns_none_when_no_household() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_human(&mut conn, "agent:uhCAkAlice003", None);

        let hub = resolve_owning_hub(&mut conn, "agent:uhCAkAlice003").expect("no error");
        assert!(hub.is_none(), "None when no household_id");
    }

    /// T2-4: agent not in humans table → returns None (graceful).
    #[test]
    fn resolve_owning_hub_returns_none_for_unknown_agent() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let hub = resolve_owning_hub(&mut conn, "agent:unknown").expect("no error");
        assert!(hub.is_none(), "None for unknown agent");
    }

    // ─── resolve_member_collectives ────────────────────────────────────────────

    /// T2-5: member_cid NULL (seed data) → resolved via human_id fallback, returns slug.
    #[test]
    fn resolve_member_collectives_via_human_id_fallback() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", None);
        // agent_cid IS humans.id — no prefix strip needed
        seed_human(&mut conn, "agent:uhCAkBob001", Some("family-dowell"));
        seed_participation_by_human_id(&mut conn, "family-dowell", "agent:uhCAkBob001");

        let collectives =
            resolve_member_collectives(&mut conn, "agent:uhCAkBob001").expect("no error");

        assert_eq!(collectives.len(), 1);
        assert_eq!(
            collectives[0], "family-dowell",
            "returns slug when CID NULL"
        );
    }

    /// T2-6: member_cid set → resolved via CID path, returns CID.
    #[test]
    fn resolve_member_collectives_via_member_cid_canonical() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            Some("collective:uhCkkTestHash002"),
        );
        seed_human(&mut conn, "agent:uhCAkBob002", Some("family-dowell"));
        seed_participation_with_cid(
            &mut conn,
            "family-dowell",
            "agent:uhCAkBob002",
            "agent:uhCAkBob002",
        );

        let collectives =
            resolve_member_collectives(&mut conn, "agent:uhCAkBob002").expect("no error");

        assert_eq!(collectives.len(), 1);
        assert_eq!(
            collectives[0], "collective:uhCkkTestHash002",
            "returns CID when member_cid and collective_cid both set"
        );
    }

    /// T2-7: agent participates in two collectives (human_id path) → both returned.
    #[test]
    fn resolve_member_collectives_returns_all_active() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", None);
        seed_collective(&mut conn, "team-alpha", None);
        seed_human(&mut conn, "agent:uhCAkBob003", None);
        seed_participation_by_human_id(&mut conn, "family-dowell", "agent:uhCAkBob003");
        seed_participation_by_human_id(&mut conn, "team-alpha", "agent:uhCAkBob003");

        let mut collectives =
            resolve_member_collectives(&mut conn, "agent:uhCAkBob003").expect("no error");
        collectives.sort();

        assert_eq!(collectives, vec!["family-dowell", "team-alpha"]);
    }

    /// T2-8: departed participation is excluded.
    #[test]
    fn resolve_member_collectives_excludes_departed() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", None);
        seed_human(&mut conn, "agent:uhCAkBob004", None);
        seed_participation_by_human_id(&mut conn, "family-dowell", "agent:uhCAkBob004");

        // Mark as departed
        use crate::db::diesel_schema::collective_participations;
        diesel::update(
            collective_participations::table
                .filter(collective_participations::id.eq("part-family-dowell-agent:uhCAkBob004")),
        )
        .set(collective_participations::departed_at.eq(Some("2026-01-01T00:00:00Z")))
        .execute(&mut conn)
        .expect("set departed_at");

        let collectives =
            resolve_member_collectives(&mut conn, "agent:uhCAkBob004").expect("no error");
        assert!(collectives.is_empty(), "departed participation excluded");
    }

    // ─── members_of_hub ────────────────────────────────────────────────────────

    /// T2-9: members_of_hub by CID works when collective_cid is set.
    #[test]
    fn members_of_hub_by_cid() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            Some("collective:uhCkkTestHash003"),
        );
        seed_human(&mut conn, "agent:uhCAkCarol001", Some("family-dowell"));
        seed_human(&mut conn, "agent:uhCAkDave001", Some("family-dowell"));
        seed_participation_by_human_id(&mut conn, "family-dowell", "agent:uhCAkCarol001");
        seed_participation_by_human_id(&mut conn, "family-dowell", "agent:uhCAkDave001");

        let mut members =
            members_of_hub(&mut conn, "collective:uhCkkTestHash003").expect("no error");
        members.sort();

        assert_eq!(
            members,
            vec!["agent:uhCAkCarol001", "agent:uhCAkDave001"],
            "members_of_hub by CID returns human_ids as identity (member_cid NULL)"
        );
    }

    /// T2-10: members_of_hub by slug works (legacy path).
    #[test]
    fn members_of_hub_by_slug() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", None);
        seed_participation_by_human_id(&mut conn, "family-dowell", "agent:uhCAkEve001");

        let members = members_of_hub(&mut conn, "family-dowell").expect("no error");
        assert_eq!(members, vec!["agent:uhCAkEve001"]);
    }

    /// T2-11: members_of_hub returns member_cid when set, otherwise human_id.
    #[test]
    fn members_of_hub_prefers_member_cid() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            Some("collective:uhCkkTestHash004"),
        );
        // Part A: no member_cid → human_id returned
        seed_participation_by_human_id(&mut conn, "family-dowell", "agent:uhCAkFrank001");
        // Part B: member_cid set → that is returned
        seed_participation_with_cid(
            &mut conn,
            "family-dowell",
            "agent:uhCAkGrace001",
            "agent:uhCAkGrace001",
        );

        let mut members =
            members_of_hub(&mut conn, "collective:uhCkkTestHash004").expect("no error");
        members.sort();

        assert!(
            members.contains(&"agent:uhCAkFrank001".to_string()),
            "human_id used when member_cid NULL"
        );
        assert!(
            members.contains(&"agent:uhCAkGrace001".to_string()),
            "member_cid used when set"
        );
    }

    /// T2-12: members_of_hub for unknown hub returns empty vec.
    #[test]
    fn members_of_hub_unknown_hub_returns_empty() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let members = members_of_hub(&mut conn, "collective:nobody").expect("no error");
        assert!(members.is_empty());
    }

    // ─── slug_to_cid / cid_to_slug ─────────────────────────────────────────────

    /// T2-13: slug_to_cid and cid_to_slug round-trip.
    #[test]
    fn slug_cid_round_trip() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(
            &mut conn,
            "family-dowell",
            Some("collective:uhCkkRoundTrip001"),
        );

        let cid = slug_to_cid(&mut conn, "family-dowell")
            .expect("no error")
            .expect("Some");
        assert_eq!(cid, "collective:uhCkkRoundTrip001");

        let slug = cid_to_slug(&mut conn, "collective:uhCkkRoundTrip001")
            .expect("no error")
            .expect("Some");
        assert_eq!(slug, "family-dowell");
    }

    /// T2-14: slug_to_cid returns None when collective_cid is NULL.
    #[test]
    fn slug_to_cid_none_when_null() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        seed_collective(&mut conn, "family-dowell", None);

        let cid = slug_to_cid(&mut conn, "family-dowell").expect("no error");
        assert!(cid.is_none(), "None when collective_cid is NULL");
    }

    /// T2-15: cid_to_slug returns None for unknown CID.
    #[test]
    fn cid_to_slug_none_for_unknown() {
        let pool = test_pool();
        let mut conn = pool.get().expect("conn");

        let slug = cid_to_slug(&mut conn, "collective:nobody").expect("no error");
        assert!(slug.is_none());
    }
}
