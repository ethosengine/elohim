//! Genesis bootstrap self-heal identity (OPERATOR-AUTHORIZED STOPGAP).
//!
//! # Why this exists
//!
//! On a headless edge pod, the `humans` row for the pod's configured human is
//! created by `seed-humans.ts` (`POST /auth/register`) with `agentPubKey: null`
//! — there was no truthful agent-key source at seed time. That NULL is the live
//! cause of the dark resilience card: the snapshot's collective join in
//! `services/household_resilience.rs` equates `humans.agent_pub_key` with
//! `rea_commitments.provider` (the `uhCAk…` `agent_cid`) AND filters out rows
//! whose `humans.household_id` is NULL (lines 194-195). A NULL on either column
//! means the join counts zero `commitmentBackedCollectives` and the card stays
//! dark.
//!
//! The truthful `uhCAk…` key DOES exist at runtime: the pod's OWN conductor
//! cell minted it. This module fills the pod's own configured human row from its
//! own cell key — a bootstrap-trust act, not a TOFU bypass:
//!
//! - **Own cell key only.** The supplied `uhcak` is always the pod's own
//!   conductor cell key (`HcClient::agent_key_uhcak()`), never a claim about
//!   another agent.
//! - **NULL-fill.** The base heal goes through [`heal_human_identity`], which
//!   fills only currently-NULL columns and never clobbers a set value.
//! - **Stale-for-self rekey.** A non-prod DNA reinstall mints a NEW `AgentPubKey`
//!   (`ALLOW_DNA_REINSTALL=true` on alpha) while the projected
//!   `humans.agent_pub_key` (and the local session) still hold the FOSSIL key —
//!   whose `peer_statuses` row is frozen degraded, so `peer_selection` returns
//!   "peers-unavailable". So when the pod's OWN cell key differs from the SELF
//!   row's stored key, this OVERWRITES it toward the current cell key (and heals
//!   the local session the same way — `resolve_provider` prefers the session key,
//!   and a stale session key is still `uhCAk…`-shaped, so an unhealed session
//!   would keep authoring provides under the dead key). Scoped to
//!   `SELF_HUMAN_ID` and gated on `is_agent_cid`; this recurs on every
//!   DNA-content deploy, so it is a standing boot-time reconcile, not a one-shot.
//! - **Provenance-recorded.** Every heal emits an `info!` with
//!   `source="genesis-self-heal-from-cell-key"`.
//! - **Gated.** The call site is gated behind `genesis_self_heal_identity`
//!   (env `GENESIS_SELF_HEAL_IDENTITY`, default false) and only ever touches
//!   the one configured `self_human_id`.
//!
//! # Superseded by
//!
//! This is the genesis stopgap. The durable replacement is the cross-signed
//! `coherent-transport-identity-resolver`
//! (`genesis/docs/superpowers/specs/2026-06-15-coherent-transport-identity-resolver-design.md`),
//! which resolves transport ids → `agent_cid` through cross-signed
//! `AgentPeerBinding` control proofs. Until that lands and a real signed binding
//! is emitted by edge nodes, this self-heal is the only truthful path to a
//! populated `humans.agent_pub_key` on a headless pod.

use diesel::sqlite::SqliteConnection;

use crate::db;
use crate::db::local_sessions::CreateLocalSessionInput;
use crate::error::StorageError;

/// Heal the configured human's identity from the pod's own conductor cell key.
///
/// Idempotent and NULL-only:
/// 1. [`heal_human_identity`](crate::db::humans::heal_human_identity) fills the
///    NULL `agent_pub_key` (and NULL `household_id`) from the inputs; a set
///    value is never overwritten. A missing human row is **self-INSERTED** from
///    the cell key + configured household (the pod's own `SELF_HUMAN_ID`), so a
///    non-matthew pod becomes sessionable for the provide-rows seeder — the
///    `register` stage single-targets the card pod, so this is the only writer
///    of a non-card pod's own human row.
/// 2. If no local session exists yet, create one so the provide-rows seeder's
///    `/auth/me` clears its no-session 401 gate. `/auth/me` reads identity
///    (human_id, agent_pub_key, identifier) straight off the session row, so the
///    `uhcak` here is the load-bearing field; `identifier` mirrors the human id
///    and `doorway_url` is an empty sentinel (the storage `/auth/me` projection
///    returns `doorwayUrl` only when present).
///
/// Returns `Ok(())` on success or a logged skip; only a genuine DB failure
/// surfaces as `Err`.
pub fn genesis_self_heal_identity(
    conn: &mut SqliteConnection,
    human_id: &str,
    uhcak: &str,
    household_id: Option<&str>,
) -> Result<(), StorageError> {
    if uhcak.is_empty() {
        tracing::warn!(
            human_id,
            "genesis self-heal skipped: empty cell agent key (uhcak)"
        );
        return Ok(());
    }

    // --- Heal arm: fill NULL agent_pub_key (+ household_id) from own cell key.
    match db::humans::heal_human_identity(conn, human_id, Some(uhcak), household_id) {
        Ok(_) => {}
        Err(StorageError::NotFound(_)) => {
            // The configured human row is absent on this pod. Genesis bootstrap
            // self-INSERT: create the pod's OWN human from its OWN cell key +
            // configured household so the session arm (below) can mint a session.
            // WHY beyond matthew: the provide-rows seeder fetches THIS pod's key
            // via `/auth/me`, then writes the healed key + active provide-row to
            // the card pod (matthew) through the doorway — so a non-matthew pod
            // (e.g. adam) must be *sessionable* for its household to count, and it
            // can only be sessionable if its own human row exists here. The
            // `register` stage single-targets matthew, so this self-insert is the
            // only thing that lays adam's row on adam's pod. Same bootstrap-trust
            // basis as the NULL-fill heal above: own cell key, own configured
            // `SELF_HUMAN_ID`, never a claim about another agent. The card join is
            // agent_cid=agent_cid (no AgentPeerBinding consumed) → not
            // transport-binding-gated.
            db::humans::create_human(
                conn,
                db::humans::CreateHumanInput {
                    id: human_id.to_string(),
                    agent_pub_key: Some(uhcak.to_string()),
                    display_name: human_id.to_string(),
                    bio: None,
                    affinities: "[]".to_string(),
                    profile_reach: "commons".to_string(),
                    location: None,
                    profile_photo_url: None,
                    h_app_id: crate::db::context::HUMANS_HAPP_ID.to_string(),
                    household_id: household_id.map(|h| h.to_string()),
                },
            )?;
            tracing::info!(
                human_id,
                uhcak,
                source = "genesis-self-heal-from-cell-key",
                "genesis self-heal: inserted own human row (was absent) — pod now sessionable"
            );
        }
        Err(e) => return Err(e),
    }

    // --- Rekey arm (STALE-FOR-SELF): a non-prod DNA reinstall mints a NEW
    // AgentPubKey (`ALLOW_DNA_REINSTALL=true` on alpha), but the projected
    // `humans.agent_pub_key` for SELF still holds the FOSSIL key — and that
    // fossil's peer_statuses row is frozen degraded, so peer_selection returns
    // "peers-unavailable". The NULL-only heal above is a no-op on a SET-but-stale
    // key, so track the pod's own CURRENT cell key here. This recurs on every
    // DNA-content deploy, so it is a standing boot-time reconcile, not a one-shot.
    //
    // Guarded on `is_agent_cid`: the rekey OVERWRITES a set value, so an
    // empty/invalid cell key must never clobber a good key (empty is already
    // short-circuited at the top; this also rejects a transport-id `uhcak`).
    if crate::identity_namespace::is_agent_cid(uhcak) {
        if let Some(human) = db::humans::get_human_by_id(conn, human_id)? {
            if let Some(old) = human.agent_pub_key.as_deref() {
                if old != uhcak {
                    db::humans::rekey_human_agent_key(conn, human_id, uhcak)?;
                    tracing::info!(
                        human_id,
                        old_key_prefix = %key_prefix(old),
                        new_key_prefix = %key_prefix(uhcak),
                        source = "genesis-self-heal-rekey",
                        "genesis self-heal: rekeyed SELF human agent_pub_key (pod re-key drift)"
                    );
                }
            }
        }
    }

    // --- Session arm: clear the no-session 401 so seeder `/auth/me` succeeds.
    if !db::local_sessions::has_any_session(conn)? {
        db::local_sessions::create_session(
            conn,
            CreateLocalSessionInput {
                id: None,
                human_id: human_id.to_string(),
                agent_pub_key: uhcak.to_string(),
                // Genesis sentinel: this session is minted from the local cell
                // key, not a doorway OAuth flow. `/auth/me` returns doorwayUrl
                // only when present, so an empty value is benign.
                doorway_url: String::new(),
                doorway_id: None,
                identifier: human_id.to_string(),
                display_name: None,
                profile_image_hash: None,
                bootstrap_url: None,
            },
        )?;
    }

    // --- Session rekey arm (STALE-FOR-SELF, CRITICAL): the freshly-created
    // session above already carries `uhcak`, so this is a no-op there. But a
    // session that survived the re-key still holds the fossil key — and
    // `conductor_commitment_author::resolve_provider` PREFERS the session key.
    // A stale session key is itself `uhCAk…`-shaped (passes `is_agent_cid`), so
    // without this every future provide commitment keeps being authored under the
    // dead key. Track the current cell key so provider resolution flips to it.
    if crate::identity_namespace::is_agent_cid(uhcak) {
        if let Some(active) = db::local_sessions::get_active_session(conn)? {
            if active.human_id == human_id && active.agent_pub_key != uhcak {
                db::local_sessions::rekey_active_session_agent_key(conn, human_id, uhcak)?;
                tracing::info!(
                    human_id,
                    session_id = %active.id,
                    old_key_prefix = %key_prefix(&active.agent_pub_key),
                    new_key_prefix = %key_prefix(uhcak),
                    source = "genesis-self-heal-rekey",
                    "genesis self-heal: rekeyed SELF local session agent_pub_key (pod re-key drift)"
                );
            }
        }
    }

    tracing::info!(
        human_id,
        uhcak,
        source = "genesis-self-heal-from-cell-key",
        "genesis bootstrap identity heal"
    );
    Ok(())
}

/// Short, log-safe prefix of an agent key for the transition record (avoids
/// dumping the full key into logs while keeping old→new distinguishable).
fn key_prefix(key: &str) -> String {
    key.chars().take(12).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::humans::{create_human, get_human_by_id, CreateHumanInput};
    use crate::db::{run_migrations, DbPool};
    use diesel::r2d2::{ConnectionManager, Pool};

    /// Shared-cache in-memory pool with the real migrations applied — gives us
    /// BOTH the `humans` and `local_sessions` tables (this fn spans both).
    /// Mirrors `db::humans::tests::test_pool`.
    fn test_pool() -> DbPool {
        let url = format!(
            "file:genesis_self_heal_test_{}?mode=memory&cache=shared",
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
    /// the production shape after `seed-humans.ts` registers with
    /// `agentPubKey: null`.
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

    #[test]
    fn heals_null_key_and_household_and_creates_session() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_null_human(&mut conn, "human-matthew-manager");

        genesis_self_heal_identity(
            &mut conn,
            "human-matthew-manager",
            "uhCAkMATTHEW",
            Some("household-dowell"),
        )
        .expect("self-heal");

        let healed = get_human_by_id(&mut conn, "human-matthew-manager")
            .unwrap()
            .expect("human present");
        assert_eq!(healed.agent_pub_key.as_deref(), Some("uhCAkMATTHEW"));
        assert_eq!(healed.household_id.as_deref(), Some("household-dowell"));

        let session = db::local_sessions::get_active_session(&mut conn)
            .unwrap()
            .expect("session created");
        assert_eq!(session.human_id, "human-matthew-manager");
        assert_eq!(session.agent_pub_key, "uhCAkMATTHEW");
    }

    #[test]
    fn re_run_with_equal_key_is_a_noop_does_not_dup_session() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_null_human(&mut conn, "human-adam-firstman");

        genesis_self_heal_identity(
            &mut conn,
            "human-adam-firstman",
            "uhCAkADAM",
            Some("household-eden"),
        )
        .expect("first heal");

        // Second run with the SAME cell key + household is a pure no-op: it must
        // not re-write the row, must not clobber household, and must not create a
        // second session. (A *different* cell key is the re-key case and is
        // handled by `rekeys_stale_self_human_and_session` — NOT here.)
        genesis_self_heal_identity(
            &mut conn,
            "human-adam-firstman",
            "uhCAkADAM",
            Some("household-eden"),
        )
        .expect("second heal (no-op)");

        let healed = get_human_by_id(&mut conn, "human-adam-firstman")
            .unwrap()
            .expect("human present");
        assert_eq!(healed.agent_pub_key.as_deref(), Some("uhCAkADAM"));
        assert_eq!(healed.household_id.as_deref(), Some("household-eden"));

        let sessions = db::local_sessions::list_all_sessions(&mut conn).unwrap();
        assert_eq!(sessions.len(), 1, "no duplicate session on re-run");
    }

    /// Insert a human whose `agent_pub_key` is ALREADY SET to a (now-stale) key,
    /// with a household — the production shape AFTER a pod re-key: the humans row
    /// still holds the fossil `uhCAk…` from before the reinstall.
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

    /// Mint an active session for `human_id` carrying `agent_key` — the fossil
    /// session a re-keyed pod keeps (its key is still `uhCAk…`-shaped, so it
    /// passes `is_agent_cid` and would otherwise author every future provide under
    /// the dead key).
    fn insert_session(conn: &mut SqliteConnection, human_id: &str, agent_key: &str) {
        db::local_sessions::create_session(
            conn,
            CreateLocalSessionInput {
                id: None,
                human_id: human_id.to_string(),
                agent_pub_key: agent_key.to_string(),
                doorway_url: String::new(),
                doorway_id: None,
                identifier: human_id.to_string(),
                display_name: None,
                profile_image_hash: None,
                bootstrap_url: None,
            },
        )
        .expect("insert session");
    }

    /// (a) STALE-FOR-SELF: a pod re-key leaves the SELF human row AND the local
    /// session holding the old `uhCAk…`. Self-heal must move BOTH to the pod's
    /// current cell key, preserving household_id and all other fields.
    #[test]
    fn rekeys_stale_self_human_and_session() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // Post-rekey fossil state: humans row + session both on the OLD key.
        insert_keyed_human(
            &mut conn,
            "human-matthew-manager",
            "uhCAkOLDfossil",
            "household-dowell",
        );
        insert_session(&mut conn, "human-matthew-manager", "uhCAkOLDfossil");

        genesis_self_heal_identity(
            &mut conn,
            "human-matthew-manager",
            "uhCAkNEWlive",
            Some("household-dowell"),
        )
        .expect("rekey heal");

        // Humans row now tracks the live cell key; household preserved.
        let healed = get_human_by_id(&mut conn, "human-matthew-manager")
            .unwrap()
            .expect("human present");
        assert_eq!(
            healed.agent_pub_key.as_deref(),
            Some("uhCAkNEWlive"),
            "stale SELF human key must be rekeyed to the current cell key"
        );
        assert_eq!(
            healed.household_id.as_deref(),
            Some("household-dowell"),
            "household_id must be preserved through the rekey"
        );

        // Session (which resolve_provider prefers) now tracks the live key.
        let session = db::local_sessions::get_active_session(&mut conn)
            .unwrap()
            .expect("active session");
        assert_eq!(
            session.agent_pub_key, "uhCAkNEWlive",
            "stale SELF session key must be rekeyed so provider resolution flips to the live key"
        );
        // No duplicate session minted.
        assert_eq!(
            db::local_sessions::list_all_sessions(&mut conn)
                .unwrap()
                .len(),
            1
        );
    }

    /// (c) SELF-ONLY: another human's row (a different key) is NEVER touched by a
    /// self-heal scoped to SELF_HUMAN_ID — no cross-agent writes.
    #[test]
    fn never_touches_another_humans_row() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_keyed_human(
            &mut conn,
            "human-matthew-manager",
            "uhCAkOLDfossil",
            "household-dowell",
        );
        insert_session(&mut conn, "human-matthew-manager", "uhCAkOLDfossil");
        // A DIFFERENT human with its own key + household.
        insert_keyed_human(
            &mut conn,
            "human-adam-firstman",
            "uhCAkADAMkey",
            "household-eden",
        );

        genesis_self_heal_identity(
            &mut conn,
            "human-matthew-manager",
            "uhCAkNEWlive",
            Some("household-dowell"),
        )
        .expect("self rekey heal");

        // SELF healed…
        let matthew = get_human_by_id(&mut conn, "human-matthew-manager")
            .unwrap()
            .unwrap();
        assert_eq!(matthew.agent_pub_key.as_deref(), Some("uhCAkNEWlive"));

        // …but the OTHER human is completely untouched.
        let adam = get_human_by_id(&mut conn, "human-adam-firstman")
            .unwrap()
            .expect("other human present");
        assert_eq!(
            adam.agent_pub_key.as_deref(),
            Some("uhCAkADAMkey"),
            "another agent's key must never be rekeyed by a SELF heal"
        );
        assert_eq!(adam.household_id.as_deref(), Some("household-eden"));
    }

    /// (d) SAFETY: an empty cell key never writes — a stale SELF key/session are
    /// left exactly as they are rather than being clobbered with an empty string.
    #[test]
    fn empty_uhcak_does_not_rekey_stale_self() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_keyed_human(
            &mut conn,
            "human-matthew-manager",
            "uhCAkOLDfossil",
            "household-dowell",
        );
        insert_session(&mut conn, "human-matthew-manager", "uhCAkOLDfossil");

        genesis_self_heal_identity(
            &mut conn,
            "human-matthew-manager",
            "",
            Some("household-dowell"),
        )
        .expect("empty uhcak skip");

        let human = get_human_by_id(&mut conn, "human-matthew-manager")
            .unwrap()
            .unwrap();
        assert_eq!(
            human.agent_pub_key.as_deref(),
            Some("uhCAkOLDfossil"),
            "empty cell key must never clobber a set key"
        );
        let session = db::local_sessions::get_active_session(&mut conn)
            .unwrap()
            .unwrap();
        assert_eq!(session.agent_pub_key, "uhCAkOLDfossil");
    }

    /// (e) DESIGN-QUESTION INTERPLAY: the fossil session key is itself a VALID
    /// `agent_cid` (`is_agent_cid` == true), so a naive "only heal invalid keys"
    /// check would skip it — and `conductor_commitment_author::resolve_provider`
    /// (which prefers the session key) would keep authoring provides under the
    /// dead key forever. This test proves a *valid-but-stale* session key is still
    /// rekeyed to the live cell key, which is what flips provider resolution — the
    /// precondition that makes `ProvideReconciler::reconcile_provides` author a
    /// FRESH commitment under the new key (see the design-question verdict).
    #[test]
    fn valid_but_stale_session_key_is_still_rekeyed() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_keyed_human(
            &mut conn,
            "human-matthew-manager",
            "uhCAkOLDfossil",
            "household-dowell",
        );
        insert_session(&mut conn, "human-matthew-manager", "uhCAkOLDfossil");

        // Precondition: the stale key looks perfectly valid.
        assert!(
            crate::identity_namespace::is_agent_cid("uhCAkOLDfossil"),
            "the fossil session key IS a valid agent_cid — the trap this test guards"
        );

        genesis_self_heal_identity(
            &mut conn,
            "human-matthew-manager",
            "uhCAkNEWlive",
            Some("household-dowell"),
        )
        .expect("rekey heal");

        let session = db::local_sessions::get_active_session(&mut conn)
            .unwrap()
            .unwrap();
        assert_eq!(
            session.agent_pub_key, "uhCAkNEWlive",
            "a valid-but-stale session key must still be rekeyed to the live cell key"
        );
    }

    #[test]
    fn missing_human_is_self_inserted_and_sessioned() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // No human inserted — the configured SELF_HUMAN_ID is absent on this pod
        // (the `register` stage single-targets the card pod). Self-heal must
        // self-INSERT it from the cell key so the pod is sessionable for the
        // provide-rows seeder (adam's count=2 path).

        genesis_self_heal_identity(
            &mut conn,
            "human-adam-firstman",
            "uhCAkADAM",
            Some("household-eden"),
        )
        .expect("self-insert heal");

        // The configured human is now present, keyed + householded from the cell.
        let healed = get_human_by_id(&mut conn, "human-adam-firstman")
            .unwrap()
            .expect("human self-inserted (was absent)");
        assert_eq!(healed.agent_pub_key.as_deref(), Some("uhCAkADAM"));
        assert_eq!(healed.household_id.as_deref(), Some("household-eden"));

        // And a session IS minted — so /auth/me returns the key for the seeder.
        let session = db::local_sessions::get_active_session(&mut conn)
            .unwrap()
            .expect("session minted after self-insert");
        assert_eq!(session.human_id, "human-adam-firstman");
        assert_eq!(session.agent_pub_key, "uhCAkADAM");
    }

    #[test]
    fn empty_uhcak_is_skipped() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        insert_null_human(&mut conn, "human-matthew-manager");

        genesis_self_heal_identity(
            &mut conn,
            "human-matthew-manager",
            "",
            Some("household-dowell"),
        )
        .expect("empty uhcak skip");

        let healed = get_human_by_id(&mut conn, "human-matthew-manager")
            .unwrap()
            .expect("human present");
        assert_eq!(healed.agent_pub_key, None, "empty key must not be written");
        assert!(!db::local_sessions::has_any_session(&mut conn).unwrap());
    }
}
