//! Periodic identity FILL from DHT membership truth (fills-never-moves).
//!
//! # The gap this closes
//!
//! The resilience card's stewarding join equates `humans.agent_pub_key` with the
//! `agent_cid` on the holder side (`shard_locations.peer_id`) and the commitment
//! side (`rea_commitments.provider`), and filters out rows with a NULL
//! `household_id`. On a pod whose `humans` rows are MISSING (a peer's row was
//! never laid here) or carry a NULL `agent_pub_key` (seeded with
//! `agentPubKey: null`), that join is dark — adam renders `commitmentBacked 0`
//! while matthew, whose rows carry keys, renders 4.
//!
//! Two existing writers touch `humans.agent_pub_key` but neither closes this gap:
//!
//! - `reconcile::controller::on_membership_projected` — the live SIGNAL path.
//!   Fills a NULL key, but ONLY on a signal AND ONLY for an EXISTING row. It
//!   never creates a row and never fires on a pod that missed the signal window.
//! - the boot pass (`household_backfill::run_once_by_membership` +
//!   `membership_identity_reconcile::reconcile_membership_keys`) — fills NULL
//!   `household_id` and SUPERSEDES a set-but-stale key, but NEVER fills a NULL
//!   `agent_pub_key` and NEVER creates a `humans` row. And it is boot-only.
//!
//! So a pod that boots with a member's row absent (or key-NULL) stays dark until
//! a fresh live signal happens to arrive — which, for a peer's row, it may never.
//! This module is the missing periodic, create-capable, NULL-fill pass.
//!
//! # Where the truth comes from (the load-bearing read)
//!
//! Membership is DHT-shared: the pod reads its OWN imagodei cell. The exact read
//! the boot pass already uses — [`snapshot_household_ids`] — returns
//! `(member_cid, household_cid)` pairs where `member_cid` is the qahal
//! `encode_agent_cid` form `agent:{pubkey}`. The agent key IS carried in the
//! membership entry, cross-member, so no new zome function is needed: we reuse
//! that snapshot verbatim as the fill source.
//!
//! # Write semantics — identity fills-never-moves
//!
//! For each membership member (`member_cid` = `agent:{pubkey}`, `household_cid`):
//!
//! 1. **Namespace guard.** `member_cid` must be `agent_cid`-shaped
//!    (`is_agent_cid`, tolerant of the `agent:` prefix). A transport id
//!    masquerading as a member is observed + skipped — never written into an
//!    `agent_cid` join-key column (mirrors `membership_identity_reconcile`).
//! 2. **Already-present short-circuit.** If ANY `humans` row already carries the
//!    bare key, this member is accounted for — skip. This keeps the pass
//!    idempotent across the other writers (a slug-keyed row healed by
//!    `on_membership_projected`, the self row created by `genesis_self_heal`).
//! 3. **FILL.** If a row keyed `id == member_cid` exists with a NULL
//!    `agent_pub_key`, fill it (NULL-only, via `heal_human_identity`). NEVER
//!    overwrite a SET key — supersession of a stale key stays
//!    `membership_identity_reconcile`'s job.
//! 4. **CREATE.** Otherwise create a minimal `humans` row keyed `id = member_cid`
//!    carrying the bare `agent_pub_key` + `household_id` — the two columns the
//!    resilience join reads. The `id` value is irrelevant to the join; the row
//!    lights it the moment the key + household are present.
//!
//! The pass NEVER moves a set key and NEVER touches a slug-keyed NULL row (it has
//! no stable `human_id` to match a `member_cid` against — that pairing is the
//! forced-bijection problem `membership_identity_reconcile` owns). A slug-keyed
//! NULL row simply stays dark, exactly as before, while the created `agent:`-keyed
//! row lights the join. Duplicate presence for one human in a household is benign:
//! the card counts DISTINCT households.
//!
//! # Testable seam
//!
//! [`apply_membership_fill`] is the pure DB core — exercised directly with a
//! hand-supplied pair list, no conductor. The conductor dependency is isolated
//! behind [`MembershipKeySource`]; the real [`ConductorMembershipKeySource`]
//! wraps [`snapshot_household_ids`], and tests drive [`run_once`] with a fake.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use diesel::sqlite::SqliteConnection;
use tokio::sync::broadcast;
use tokio::time::{interval, MissedTickBehavior};

use crate::db::{AppContext, DbPool};
use crate::hc_client::HcClient;
use crate::identity_namespace::{observe_agent_cid_write, resolve_agent_cid_write};
use crate::services::holochain_humans_replayer::{
    snapshot_household_ids, ConductorMembershipReader,
};
use crate::StorageError;

/// Default fill cadence (seconds). Overridable via `IDENTITY_FILL_SECS`; `0`
/// disables the loop entirely (the task is simply not spawned).
pub const DEFAULT_FILL_SECS: u64 = 300;

/// Settle delay before the first sweep (seconds) — mirrors the peer_status
/// fan-in convention so boot projections + the boot backfill land first.
pub const DEFAULT_SETTLE_SECS: u64 = 30;

/// Outcome tallies for one fill sweep.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FillStats {
    /// New `humans` rows created for a member with no prior row.
    pub created: usize,
    /// Existing `id == member_cid` rows whose NULL `agent_pub_key` was filled.
    pub filled: usize,
    /// Members already accounted for (a row already carries the bare key, or the
    /// `id`-keyed row already holds a SET key — supersede's job, not ours).
    pub skipped_present: usize,
    /// Members skipped because `member_cid` was not `agent_cid`-shaped (a
    /// transport id must never land in an `agent_cid` join-key column).
    pub skipped_non_agent_cid: usize,
}

/// Seam for reading the current `(member_cid, household_cid)` membership pairs
/// from the pod's own imagodei cell. The real impl wraps `snapshot_household_ids`;
/// tests supply a fake so [`run_once`] is exercised without a conductor.
#[async_trait]
pub trait MembershipKeySource: Send + Sync {
    /// Return the current membership pairs. `member_cid` is the `agent:{pubkey}`
    /// form; `household_cid` is the collective the member belongs to. Errors are
    /// surfaced so the loop can log + retry next tick (never fatal).
    async fn fetch_pairs(&self) -> Result<Vec<(String, String)>, StorageError>;
}

/// Pure DB core: apply one fill pass over `pairs`. No conductor, no I/O beyond
/// the connection — unit-tested directly. Best-effort per member: a create/fill
/// failure on one member is logged and skipped, never aborting the sweep (so one
/// bad row cannot dark the rest of the pool).
pub fn apply_membership_fill(
    conn: &mut SqliteConnection,
    pairs: &[(String, String)],
) -> Result<FillStats, StorageError> {
    use crate::db::context::HUMANS_HAPP_ID;
    use crate::db::humans::{
        create_human, get_human_by_agent_key, get_human_by_id, heal_human_identity,
        CreateHumanInput,
    };

    let mut stats = FillStats::default();

    for (member_cid, household_cid) in pairs {
        // (1) Namespace guard. `resolve_agent_cid_write` validates the value is
        // `agent_cid`-shaped AND normalises to the bare `uhCAk…` form the
        // join-key columns hold (stripping any `agent:` prefix). A non-agent_cid
        // member (a transport id) yields None — observe + skip, never write.
        let Some(bare_key) = resolve_agent_cid_write(&[Some(member_cid.as_str())]) else {
            observe_agent_cid_write("humans.agent_pub_key", Some(member_cid.as_str()));
            stats.skipped_non_agent_cid += 1;
            continue;
        };

        // (2) Already-present short-circuit: some row already carries this key
        // (h_app_id-agnostic, mirroring every other `humans` join). Accounted for.
        match get_human_by_agent_key(conn, &bare_key) {
            Ok(Some(_)) => {
                stats.skipped_present += 1;
                continue;
            }
            Ok(None) => {}
            Err(e) => {
                tracing::warn!(member_cid = %member_cid, error = %e, "identity_fill: agent-key lookup failed (skipping member)");
                continue;
            }
        }

        // (3)/(4) FILL an existing id-keyed NULL row, or CREATE a new one.
        match get_human_by_id(conn, member_cid) {
            Ok(Some(row)) => {
                if row.agent_pub_key.is_none() {
                    // FILL — NULL-only heal. `heal_human_identity` fills only
                    // currently-NULL columns (agent_pub_key + household_id) and
                    // never clobbers a set value, so a later supersede is safe.
                    match heal_human_identity(
                        conn,
                        member_cid,
                        Some(bare_key.as_str()),
                        Some(household_cid.as_str()),
                    ) {
                        Ok(_) => {
                            stats.filled += 1;
                            tracing::info!(
                                member_cid = %member_cid,
                                household_cid = %household_cid,
                                source = "identity-fill-membership-truth",
                                "identity_fill: filled NULL agent_pub_key from membership truth"
                            );
                        }
                        Err(e) => {
                            tracing::warn!(member_cid = %member_cid, error = %e, "identity_fill: heal failed (skipping member)");
                        }
                    }
                } else {
                    // The id-keyed row already holds a SET key (possibly stale) —
                    // NOT our job; moving it is `membership_identity_reconcile`'s
                    // guarded supersede. fills-never-moves.
                    stats.skipped_present += 1;
                }
            }
            Ok(None) => {
                // CREATE — a minimal row carrying the two columns the resilience
                // join reads. `id = member_cid` matches what the reconcile
                // controller's dual-match (id OR stripped key) would recognise.
                match create_human(
                    conn,
                    CreateHumanInput {
                        id: member_cid.clone(),
                        agent_pub_key: Some(bare_key.clone()),
                        display_name: member_cid.clone(),
                        bio: None,
                        affinities: "[]".to_string(),
                        profile_reach: "commons".to_string(),
                        location: None,
                        profile_photo_url: None,
                        h_app_id: HUMANS_HAPP_ID.to_string(),
                        household_id: Some(household_cid.clone()),
                    },
                ) {
                    Ok(_) => {
                        stats.created += 1;
                        tracing::info!(
                            member_cid = %member_cid,
                            household_cid = %household_cid,
                            source = "identity-fill-membership-truth",
                            "identity_fill: created humans row for absent member from membership truth"
                        );
                    }
                    Err(e) => {
                        // e.g. a concurrent create landed the same id — benign,
                        // idempotent on the next sweep (it will short-circuit).
                        tracing::warn!(member_cid = %member_cid, error = %e, "identity_fill: create failed (skipping member)");
                    }
                }
            }
            Err(e) => {
                tracing::warn!(member_cid = %member_cid, error = %e, "identity_fill: id lookup failed (skipping member)");
            }
        }
    }

    Ok(stats)
}

/// Run one fill sweep: fetch the current membership pairs from `source`, then
/// apply the pure fill core. The conductor dependency is entirely inside
/// `source`; a fetch error is surfaced so the loop logs + retries next tick.
pub async fn run_once<S: MembershipKeySource + ?Sized>(
    pool: &DbPool,
    source: &S,
) -> Result<FillStats, StorageError> {
    let pairs = source.fetch_pairs().await?;
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("identity_fill: pool: {e}")))?;
    apply_membership_fill(&mut conn, &pairs)
}

/// The periodic fill loop. Mirrors `peer_status_fanout::run_fanout_loop`:
/// `interval` + `MissedTickBehavior::Skip` + a shutdown branch, with a one-time
/// settle delay before the first sweep. `fill_secs == 0` is handled by the caller
/// (the task is simply not spawned).
pub async fn run_fill_loop<S: MembershipKeySource + ?Sized>(
    pool: DbPool,
    source: Arc<S>,
    fill_secs: u64,
    settle_secs: u64,
    mut shutdown: broadcast::Receiver<()>,
) {
    // One-time settle so boot projections + the boot backfill land first.
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_secs(settle_secs)) => {}
        _ = shutdown.recv() => {
            tracing::debug!("identity_fill: shutdown during settle, exiting");
            return;
        }
    }

    let mut ticker = interval(Duration::from_secs(fill_secs.max(1)));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            _ = ticker.tick() => {
                match run_once(&pool, source.as_ref()).await {
                    Ok(stats) => {
                        crate::metrics::inc_identity_fill("created", stats.created as u64);
                        crate::metrics::inc_identity_fill("filled", stats.filled as u64);
                        crate::metrics::inc_identity_fill("skipped_present", stats.skipped_present as u64);
                        crate::metrics::inc_identity_fill(
                            "skipped_non_agent_cid",
                            stats.skipped_non_agent_cid as u64,
                        );
                        crate::metrics::set_identity_fill_last_writes(
                            (stats.created + stats.filled) as u64,
                        );
                        if stats.created > 0 || stats.filled > 0 {
                            tracing::info!(
                                created = stats.created,
                                filled = stats.filled,
                                skipped_present = stats.skipped_present,
                                skipped_non_agent_cid = stats.skipped_non_agent_cid,
                                "identity_fill swept (agent keys projected from membership truth)"
                            );
                        } else {
                            tracing::debug!(
                                skipped_present = stats.skipped_present,
                                skipped_non_agent_cid = stats.skipped_non_agent_cid,
                                "identity_fill swept (no new keys)"
                            );
                        }
                    }
                    Err(e) => tracing::warn!(error = %e, "identity_fill sweep failed (retry next tick)"),
                }
            }
            _ = shutdown.recv() => {
                tracing::debug!("identity_fill: shutdown signal received, exiting");
                break;
            }
        }
    }
}

// ===========================================================================
// Real source — the conductor-dependent half (reuses the boot snapshot read).
// ===========================================================================

/// Production [`MembershipKeySource`]: reads the household-membership snapshot
/// from the pod's own imagodei cell via the SAME two-step traversal the boot
/// backfill uses (local `collectives` projection → conductor
/// `list_memberships_for_collective_cid` per household). Only `pairs` is
/// consumed — the incomplete/withdrawn abstention sets matter to the
/// key-supersede reconcile, not to a NULL-only fill (a missing member just leaves
/// a member unfilled this tick, never a wrong write).
pub struct ConductorMembershipKeySource {
    pool: DbPool,
    ctx: AppContext,
    hc: Arc<HcClient>,
}

impl ConductorMembershipKeySource {
    pub fn new(pool: DbPool, hc: Arc<HcClient>) -> Self {
        Self {
            pool,
            // humans/collectives projections are lamad-app-scoped (the reconcile
            // controller's `default_lamad` ctx uses the same junction).
            ctx: AppContext::default_lamad(),
            hc,
        }
    }
}

#[async_trait]
impl MembershipKeySource for ConductorMembershipKeySource {
    async fn fetch_pairs(&self) -> Result<Vec<(String, String)>, StorageError> {
        let reader = ConductorMembershipReader {
            hc_client: self.hc.clone(),
        };
        let snapshot = snapshot_household_ids(&self.pool, &self.ctx, &reader).await?;
        Ok(snapshot.pairs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::humans::{
        create_human, get_human_by_agent_key, get_human_by_id, CreateHumanInput,
    };
    use crate::db::DbPool;
    use std::sync::Mutex;

    fn pool() -> DbPool {
        crate::test_util::test_pool()
    }

    /// Seed a slug-keyed human (imagodei scope — the production shape). `None`
    /// key = the NULL-fill shape; `None` household = no household stamp.
    fn seed_human(
        conn: &mut SqliteConnection,
        id: &str,
        key: Option<&str>,
        household: Option<&str>,
    ) {
        create_human(
            conn,
            CreateHumanInput {
                id: id.to_string(),
                agent_pub_key: key.map(|k| k.to_string()),
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: crate::db::context::HUMANS_HAPP_ID.to_string(),
                household_id: household.map(|h| h.to_string()),
            },
        )
        .expect("seed human");
    }

    fn pair(member: &str, household: &str) -> (String, String) {
        (member.to_string(), household.to_string())
    }

    // ---- pure core: CREATE ----

    /// The core case: an absent member (adam on adam's own pod) → a new row is
    /// created carrying the bare key + household, the two columns the resilience
    /// join reads. `id` is the `agent:` form; the key is the bare `uhCAk` form.
    #[test]
    fn creates_row_for_absent_member() {
        let p = pool();
        let mut conn = p.get().unwrap();

        let pairs = vec![pair("agent:uhCAkADAM", "collective:eden")];
        let stats = apply_membership_fill(&mut conn, &pairs).expect("fill");
        assert_eq!(stats.created, 1);
        assert_eq!(stats.filled, 0);
        assert_eq!(stats.skipped_present, 0);

        // The created row lights the join: bare key + household present.
        let row = get_human_by_agent_key(&mut conn, "uhCAkADAM")
            .unwrap()
            .expect("row created under the bare key");
        assert_eq!(row.id, "agent:uhCAkADAM");
        assert_eq!(row.agent_pub_key.as_deref(), Some("uhCAkADAM"));
        assert_eq!(row.household_id.as_deref(), Some("collective:eden"));
    }

    // ---- pure core: FILL a NULL id-keyed row ----

    /// An `id == member_cid` row with a NULL key is filled (NULL-only), not
    /// duplicated.
    #[test]
    fn fills_null_key_on_id_keyed_row() {
        let p = pool();
        let mut conn = p.get().unwrap();
        // A row already keyed by the agent: id but with a NULL agent_pub_key.
        seed_human(&mut conn, "agent:uhCAkEVE", None, None);

        let pairs = vec![pair("agent:uhCAkEVE", "collective:eden")];
        let stats = apply_membership_fill(&mut conn, &pairs).expect("fill");
        assert_eq!(stats.filled, 1);
        assert_eq!(stats.created, 0);

        let row = get_human_by_id(&mut conn, "agent:uhCAkEVE")
            .unwrap()
            .unwrap();
        assert_eq!(row.agent_pub_key.as_deref(), Some("uhCAkEVE"));
        assert_eq!(row.household_id.as_deref(), Some("collective:eden"));
    }

    // ---- pure core: NEVER overwrite a non-NULL key ----

    /// A SET key is never moved — that is `membership_identity_reconcile`'s
    /// guarded supersede, not a fill. The member is counted present, untouched.
    #[test]
    fn never_overwrites_a_set_key() {
        let p = pool();
        let mut conn = p.get().unwrap();
        // The id-keyed row holds a DIFFERENT (fossil) key.
        seed_human(
            &mut conn,
            "agent:uhCAkNEWjohn",
            Some("uhCAkOLDjohn"),
            Some("collective:h"),
        );

        // Membership truth carries the new key as the member_cid.
        let pairs = vec![pair("agent:uhCAkNEWjohn", "collective:h")];
        let stats = apply_membership_fill(&mut conn, &pairs).expect("fill");
        assert_eq!(stats.filled, 0, "a set key is never filled");
        assert_eq!(stats.created, 0);
        assert_eq!(stats.skipped_present, 1);

        let row = get_human_by_id(&mut conn, "agent:uhCAkNEWjohn")
            .unwrap()
            .unwrap();
        assert_eq!(
            row.agent_pub_key.as_deref(),
            Some("uhCAkOLDjohn"),
            "the fossil key is preserved — supersede is not this pass's job"
        );
    }

    /// Present short-circuit: if ANY row already carries the bare key (e.g. a
    /// slug-keyed row healed by another writer), the member is accounted for and
    /// no duplicate is created.
    #[test]
    fn short_circuits_when_key_present_on_a_slug_row() {
        let p = pool();
        let mut conn = p.get().unwrap();
        // A slug-keyed row already carries the key (healed by on_membership_projected).
        seed_human(
            &mut conn,
            "human-adam-firstman",
            Some("uhCAkADAM"),
            Some("collective:eden"),
        );

        let pairs = vec![pair("agent:uhCAkADAM", "collective:eden")];
        let stats = apply_membership_fill(&mut conn, &pairs).expect("fill");
        assert_eq!(
            stats.created, 0,
            "the key is already present — no duplicate"
        );
        assert_eq!(stats.filled, 0);
        assert_eq!(stats.skipped_present, 1);

        // No agent:-keyed duplicate was created.
        assert!(get_human_by_id(&mut conn, "agent:uhCAkADAM")
            .unwrap()
            .is_none());
    }

    // ---- pure core: namespace guard ----

    /// A `member_cid` that is NOT `agent_cid`-shaped (a transport id) is never
    /// written — observed + skipped.
    #[test]
    fn skips_non_agent_cid_member() {
        let p = pool();
        let mut conn = p.get().unwrap();

        let pairs = vec![pair("12D3KooWtransportNotAnAgent", "collective:h")];
        let stats = apply_membership_fill(&mut conn, &pairs).expect("fill");
        assert_eq!(stats.skipped_non_agent_cid, 1);
        assert_eq!(stats.created, 0);
        assert_eq!(stats.filled, 0);

        assert!(get_human_by_id(&mut conn, "12D3KooWtransportNotAnAgent")
            .unwrap()
            .is_none());
    }

    // ---- pure core: idempotent re-run ----

    /// A second pass over the same pairs is a no-op (the row now carries the key,
    /// so the present short-circuit fires).
    #[test]
    fn second_pass_is_idempotent() {
        let p = pool();
        let mut conn = p.get().unwrap();

        let pairs = vec![
            pair("agent:uhCAkADAM", "collective:eden"),
            pair("agent:uhCAkEVE", "collective:eden"),
        ];
        let first = apply_membership_fill(&mut conn, &pairs).expect("first");
        assert_eq!(first.created, 2);

        let second = apply_membership_fill(&mut conn, &pairs).expect("second");
        assert_eq!(second.created, 0, "re-run creates nothing");
        assert_eq!(second.filled, 0);
        assert_eq!(second.skipped_present, 2, "both members already present");
    }

    // ---- run_once wiring over a faked membership source ----

    struct FakeSource {
        pairs: Vec<(String, String)>,
        calls: Mutex<usize>,
    }

    #[async_trait]
    impl MembershipKeySource for FakeSource {
        async fn fetch_pairs(&self) -> Result<Vec<(String, String)>, StorageError> {
            *self.calls.lock().unwrap() += 1;
            Ok(self.pairs.clone())
        }
    }

    #[tokio::test]
    async fn run_once_applies_the_faked_source() {
        let p = pool();
        let source = FakeSource {
            pairs: vec![
                pair("agent:uhCAkADAM", "collective:eden"),
                pair("12D3KooWnotAgent", "collective:eden"),
            ],
            calls: Mutex::new(0),
        };

        let stats = run_once(&p, &source).await.expect("run_once");
        assert_eq!(stats.created, 1);
        assert_eq!(stats.skipped_non_agent_cid, 1);
        assert_eq!(*source.calls.lock().unwrap(), 1);

        let mut conn = p.get().unwrap();
        assert!(get_human_by_agent_key(&mut conn, "uhCAkADAM")
            .unwrap()
            .is_some());
    }

    /// A source fetch error surfaces from `run_once` (the loop logs + retries).
    #[tokio::test]
    async fn run_once_propagates_fetch_error() {
        struct FailingSource;
        #[async_trait]
        impl MembershipKeySource for FailingSource {
            async fn fetch_pairs(&self) -> Result<Vec<(String, String)>, StorageError> {
                Err(StorageError::Conductor("membership read down".into()))
            }
        }
        let p = pool();
        let err = run_once(&p, &FailingSource).await;
        assert!(matches!(err, Err(StorageError::Conductor(_))));
    }
}
