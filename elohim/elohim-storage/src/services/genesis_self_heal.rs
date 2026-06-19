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
//! - **NULL-only / re-keyable.** Writes go through [`heal_human_identity`],
//!   which fills only currently-NULL columns and NEVER clobbers a set value, so
//!   a later cross-signed key rotation / supersede is not blocked.
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
                    h_app_id: "imagodei".to_string(),
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

    tracing::info!(
        human_id,
        uhcak,
        source = "genesis-self-heal-from-cell-key",
        "genesis bootstrap identity heal"
    );
    Ok(())
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
    fn re_run_is_a_noop_does_not_clobber_or_dup_session() {
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

        // Second run with a DIFFERENT key + household must NOT overwrite the
        // already-set values, and must NOT create a second session.
        genesis_self_heal_identity(
            &mut conn,
            "human-adam-firstman",
            "uhCAkDIFFERENT",
            Some("household-other"),
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
