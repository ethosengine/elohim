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
    household_collective_cids, snapshot_household_ids_for_cids, ConductorMembershipReader,
    MembershipReader, MembershipSnapshot,
};
use crate::StorageError;

/// The imagodei zome + coordinator fn that enumerates the caller's OWN
/// household memberships from its source chain (index-free DHT truth).
const IMAGODEI_ZOME: &str = "imagodei";
const SELF_HOUSEHOLD_CIDS_FN: &str = "get_my_household_collective_cids";

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

/// Seam for the INDEX-FREE household discovery read: the set of household
/// Collective cids the pod's OWN agent is a current member of, read from its
/// source chain (the imagodei `get_my_household_collective_cids` coordinator fn).
///
/// This is the half that routes around the empty local `collectives` projection.
/// Membership is DHT truth — every member authors its own `Membership` on its
/// own conductor — but there is no global "all collectives" anchor to walk, so
/// the caller's source chain is the only index-free enumeration. Tests supply a
/// fake so [`discover_household_pairs`] is exercised without a conductor.
#[async_trait]
pub trait SelfHouseholdCidSource: Send + Sync {
    /// Household Collective cids the calling agent is a current member of.
    /// Errors surface so the caller can degrade (an empty result on error, never
    /// a wrong write).
    async fn self_household_cids(&self) -> Result<Vec<String>, StorageError>;
}

/// Compose the household membership snapshot from BOTH cid sources, then read
/// members per cid.
///
/// The cid set is the UNION of:
///   - the local `collectives` projection (family layer) — populated on a pod
///     whose `CollectiveCommitted` signals landed here, EMPTY otherwise; and
///   - the pod's own source chain (`self_cids`) — the index-free DHT-truth read
///     that fills the gap when the projection is empty (the adam case).
///
/// fills-never-moves: this reads the local `collectives` table, it never writes
/// it — a projection heal for that table stays the reconcile controller's job.
/// A failure on EITHER cid source degrades to that side's empty set (never an
/// error): the union still covers whatever the other side found. The per-cid
/// membership read is [`snapshot_household_ids_for_cids`], reused verbatim.
pub async fn discover_household_pairs(
    pool: &DbPool,
    ctx: &AppContext,
    self_cids: &(dyn SelfHouseholdCidSource + '_),
    reader: &(dyn MembershipReader + '_),
) -> Result<MembershipSnapshot, StorageError> {
    // Local projection cids (family layer). Empty is a valid, common outcome.
    let local = household_collective_cids(pool, ctx).unwrap_or_else(|e| {
        tracing::warn!(error = %e, "identity_fill: local collectives read failed; source-chain-only discovery");
        Vec::new()
    });

    // Source-chain (self-authored) household cids — the index-free DHT read.
    let self_side = self_cids.self_household_cids().await.unwrap_or_else(|e| {
        tracing::warn!(error = %e, "identity_fill: self-household-cid read failed; projection-only discovery");
        Vec::new()
    });

    // Union, de-duplicated, deterministic (local first, then novel self cids).
    let mut union = local;
    for cid in self_side {
        if !union.contains(&cid) {
            union.push(cid);
        }
    }

    crate::metrics::set_identity_fill_discovered_cids(union.len() as u64);
    if union.is_empty() {
        // Previously DEBUG-only (see also `holochain_humans_replayer.rs`'s
        // `snapshot_household_ids_for_cids` empty-cids early return), this case
        // has been the invisible steady state for a month: discovery finds
        // nothing, the fill sweep is a correct no-op, and nobody sees it. Rare
        // enough per-sweep (once per `IDENTITY_FILL_SECS`, default 300s) that a
        // WARN here is not a storm.
        tracing::warn!(
            "identity_fill: discovery found zero household cids (no memberships on DHT or \
             projection) — nothing to fill"
        );
    } else {
        tracing::debug!(
            cid_count = union.len(),
            "identity_fill: discovered household collective cids (projection ∪ source-chain)"
        );
    }
    snapshot_household_ids_for_cids(reader, &union).await
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

/// Production [`SelfHouseholdCidSource`]: calls the imagodei
/// `get_my_household_collective_cids` coordinator fn on the pod's own cell. That
/// fn `query()`s the caller's source chain for its own household memberships — no
/// anchor, no local projection — so it surfaces the households a pod belongs to
/// even when the pod's `collectives` table is empty.
pub struct ConductorSelfHouseholdCids {
    hc: Arc<HcClient>,
}

#[async_trait]
impl SelfHouseholdCidSource for ConductorSelfHouseholdCids {
    async fn self_household_cids(&self) -> Result<Vec<String>, StorageError> {
        // Unit-arg zome fn: encode `()` the same way the conductor expects.
        let payload = rmp_serde::to_vec_named(&()).map_err(|e| {
            StorageError::Internal(format!("identity_fill: encode self-household payload: {e}"))
        })?;
        let bytes = self
            .hc
            .call_zome(IMAGODEI_ZOME, SELF_HOUSEHOLD_CIDS_FN, payload)
            .await?;
        let cids: Vec<String> = rmp_serde::from_slice(&bytes).map_err(|e| {
            StorageError::Serialization(format!(
                "identity_fill: decode self-household cids Vec<String>: {e}"
            ))
        })?;
        Ok(cids)
    }
}

/// Production [`MembershipKeySource`]: reads the household-membership snapshot
/// from the pod's own imagodei cell. The household cid set is the UNION of the
/// local `collectives` projection AND the pod's own source chain (via
/// [`ConductorSelfHouseholdCids`]) — the latter is what lights a pod whose
/// projection is empty (the seeder single-targeted a different doorway). Members
/// are then read per cid via `list_memberships_for_collective_cid`. Only `pairs`
/// is consumed — the incomplete/withdrawn abstention sets matter to the
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
        let self_cids = ConductorSelfHouseholdCids {
            hc: self.hc.clone(),
        };
        let snapshot = discover_household_pairs(&self.pool, &self.ctx, &self_cids, &reader).await?;
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

    /// Serializes tests that call `discover_household_pairs` and then assert on
    /// the process-global `IDENTITY_FILL_DISCOVERED_CIDS` gauge it sets — without
    /// this, parallel test threads racing the same gauge would make the
    /// assertions flaky. Poisoning (a prior panic while holding the lock) is
    /// tolerated so one failing test never cascades into spurious failures.
    static DISCOVERED_CIDS_GAUGE_TEST_LOCK: Mutex<()> = Mutex::new(());
    fn lock_discovered_cids_gauge() -> std::sync::MutexGuard<'static, ()> {
        DISCOVERED_CIDS_GAUGE_TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner())
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

    // ---- discovery composition: source-chain cids route around empty projection ----

    use crate::services::holochain_humans_replayer::test_support::membership_record;
    use holochain_types::prelude::Record;
    use std::collections::HashMap;

    /// Fake per-cid membership reader — returns hand-built Person `Record`s.
    struct FakeReader {
        by_cid: HashMap<String, Vec<Record>>,
    }
    #[async_trait]
    impl MembershipReader for FakeReader {
        async fn list_memberships(
            &self,
            collective_cid: &str,
        ) -> Result<Vec<Record>, StorageError> {
            Ok(self.by_cid.get(collective_cid).cloned().unwrap_or_default())
        }
    }

    /// Fake source-chain enumerator — the imagodei `get_my_household_collective_cids`
    /// stand-in.
    struct FakeSelfCids {
        cids: Vec<String>,
    }
    #[async_trait]
    impl SelfHouseholdCidSource for FakeSelfCids {
        async fn self_household_cids(&self) -> Result<Vec<String>, StorageError> {
            Ok(self.cids.clone())
        }
    }

    fn person(member: &str, cid: &str) -> Record {
        membership_record(member, "Person", cid, None)
    }

    /// THE ADAM CASE: the local `collectives` projection is EMPTY, yet the pod's
    /// OWN source chain names a household. Discovery reads its members and the
    /// downstream fill has keyed rows to lay — the join lights.
    #[tokio::test]
    async fn discovery_fills_from_source_chain_when_projection_empty() {
        let _guard = lock_discovered_cids_gauge();
        let p = pool();
        let ctx = AppContext::default_lamad();

        // No collectives seeded — the local projection is empty (adam).
        let self_cids = FakeSelfCids {
            cids: vec!["collective:eden".to_string()],
        };
        let mut by_cid = HashMap::new();
        by_cid.insert(
            "collective:eden".to_string(),
            vec![
                person("agent:uhCAkADAM", "collective:eden"),
                person("agent:uhCAkMATTHEW", "collective:eden"),
            ],
        );
        let reader = FakeReader { by_cid };

        let snap = discover_household_pairs(&p, &ctx, &self_cids, &reader)
            .await
            .expect("discover");
        let mut pairs = snap.pairs;
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                ("agent:uhCAkADAM".to_string(), "collective:eden".to_string()),
                (
                    "agent:uhCAkMATTHEW".to_string(),
                    "collective:eden".to_string()
                ),
            ],
            "source-chain-discovered household members are read even with an empty projection"
        );

        // End-to-end: the discovered pairs flow into the fill, lighting the join.
        let mut conn = p.get().unwrap();
        let stats = apply_membership_fill(&mut conn, &pairs).expect("fill");
        assert_eq!(
            stats.created, 2,
            "both members' rows created from DHT truth"
        );
        assert!(get_human_by_agent_key(&mut conn, "uhCAkMATTHEW")
            .unwrap()
            .is_some());
    }

    /// UNION + DEDUP: a cid present in BOTH the local projection and the source
    /// chain is read ONCE; a cid only in the source chain is added. Both
    /// households' members surface.
    #[tokio::test]
    async fn discovery_unions_projection_and_source_chain_deduped() {
        let _guard = lock_discovered_cids_gauge();
        use crate::db::collectives::{create_collective, CreateCollectiveInput};
        use crate::db::diesel_schema::collectives;
        use crate::db::models::governance_layers;
        use diesel::prelude::*;

        let p = pool();
        let ctx = AppContext::default_lamad();

        // Seed ONE local family collective (stamped with its cid) — the projection
        // side of the union.
        {
            let mut conn = p.get().unwrap();
            let mut input = CreateCollectiveInput::stub("family-local");
            input.governance_layer = governance_layers::FAMILY.to_string();
            let c = create_collective(&mut conn, &ctx, &input).expect("seed collective");
            diesel::update(collectives::table.filter(collectives::id.eq(&c.id)))
                .set(collectives::collective_cid.eq(Some("collective:local")))
                .execute(&mut conn)
                .expect("stamp cid");
        }

        // Source chain reports the SAME local cid (dup) plus a NOVEL one.
        let self_cids = FakeSelfCids {
            cids: vec![
                "collective:local".to_string(),
                "collective:selfnew".to_string(),
            ],
        };

        // Reader counts how many times each cid is asked for (dedup guard).
        struct CountingReader {
            by_cid: HashMap<String, Vec<Record>>,
            calls: std::sync::Mutex<HashMap<String, usize>>,
        }
        #[async_trait]
        impl MembershipReader for CountingReader {
            async fn list_memberships(
                &self,
                collective_cid: &str,
            ) -> Result<Vec<Record>, StorageError> {
                *self
                    .calls
                    .lock()
                    .unwrap()
                    .entry(collective_cid.to_string())
                    .or_insert(0) += 1;
                Ok(self.by_cid.get(collective_cid).cloned().unwrap_or_default())
            }
        }
        let mut by_cid = HashMap::new();
        by_cid.insert(
            "collective:local".to_string(),
            vec![person("agent:uhCAkLOCALMEMBER", "collective:local")],
        );
        by_cid.insert(
            "collective:selfnew".to_string(),
            vec![person("agent:uhCAkSELFMEMBER", "collective:selfnew")],
        );
        let reader = CountingReader {
            by_cid,
            calls: std::sync::Mutex::new(HashMap::new()),
        };

        let snap = discover_household_pairs(&p, &ctx, &self_cids, &reader)
            .await
            .expect("discover");
        let mut pairs = snap.pairs;
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                (
                    "agent:uhCAkLOCALMEMBER".to_string(),
                    "collective:local".to_string()
                ),
                (
                    "agent:uhCAkSELFMEMBER".to_string(),
                    "collective:selfnew".to_string()
                ),
            ],
            "both the projection household and the novel source-chain household surface"
        );
        let local_reads = *reader
            .calls
            .lock()
            .unwrap()
            .get("collective:local")
            .unwrap();
        assert_eq!(
            local_reads, 1,
            "the duplicated cid is read exactly once (union de-duplicated)"
        );
    }

    // ---- discovery darkness observability: the empty-union gauge ----

    /// The invisible-steady-state case: neither the local projection nor the
    /// pod's own source chain names a household. Previously this exited at
    /// DEBUG with no other signal; now the discovered-cid gauge reads 0 (the
    /// warn log itself is not asserted here — `tracing` output is not a return
    /// value — but the gauge is the durable, scrapeable twin of that warn).
    #[tokio::test]
    async fn discovery_sets_zero_gauge_when_union_is_empty() {
        let _guard = lock_discovered_cids_gauge();
        crate::metrics::register_all(); // idempotent (Once-guarded)
        let p = pool();
        let ctx = AppContext::default_lamad();

        let self_cids = FakeSelfCids { cids: vec![] };
        let reader = FakeReader {
            by_cid: HashMap::new(),
        };

        let snap = discover_household_pairs(&p, &ctx, &self_cids, &reader)
            .await
            .expect("discover");
        assert!(snap.pairs.is_empty(), "no household cids discovered");
        assert_eq!(
            crate::metrics::IDENTITY_FILL_DISCOVERED_CIDS.get(),
            0,
            "the discovered-cids gauge reflects the empty union"
        );
    }

    /// The healthy case: the gauge tracks the discovered UNION size (not the
    /// member/pair count), so an operator can distinguish "found households but
    /// zero members" from "found nothing at all."
    #[tokio::test]
    async fn discovery_sets_gauge_to_union_size_when_cids_found() {
        let _guard = lock_discovered_cids_gauge();
        crate::metrics::register_all(); // idempotent (Once-guarded)
        let p = pool();
        let ctx = AppContext::default_lamad();

        let self_cids = FakeSelfCids {
            cids: vec!["collective:eden".to_string(), "collective:eve".to_string()],
        };
        let mut by_cid = HashMap::new();
        by_cid.insert(
            "collective:eden".to_string(),
            vec![person("agent:uhCAkADAM", "collective:eden")],
        );
        by_cid.insert("collective:eve".to_string(), vec![]);
        let reader = FakeReader { by_cid };

        discover_household_pairs(&p, &ctx, &self_cids, &reader)
            .await
            .expect("discover");
        assert_eq!(
            crate::metrics::IDENTITY_FILL_DISCOVERED_CIDS.get(),
            2,
            "the gauge counts the discovered cid union, independent of member counts"
        );
    }
}
