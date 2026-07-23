//! Membership-truth identity reconciliation: agent-key SUPERSEDE + rekey cascade.
//!
//! # Why this exists
//!
//! `humans` is an OPERATIONAL PROJECTION of DHT membership truth. The projection
//! must converge to the DHT head — INCLUDING supersede (a stale SET key → the
//! live membership key), which the two existing NULL-only writers deliberately
//! never do:
//!
//! - `reconcile::controller::on_membership_projected` (the live SIGNAL path) is
//!   NULL-only by design — an out-of-order signal must never regress a key.
//! - `household_backfill::run_once_by_membership` fills only NULL `household_id`.
//!
//! Non-prod DNA reinstalls (`ALLOW_DNA_REINSTALL`) mint a NEW `AgentPubKey` per
//! pod on every deploy. For a NON-self human (a peer's row projected onto this
//! pod), the row keeps its FOSSIL `agent_pub_key` forever: the `genesis_self_heal`
//! rekey arm is hardcoded SELF-only, and every other writer is NULL-only. The
//! fossil then poisons BOTH sides of the resilience stewarding join
//! (`services::household_resilience`): the holder side
//! (`shard_locations.peer_id == humans.agent_pub_key`) and the commitment side
//! (`rea_commitments.provider == humans.agent_pub_key`).
//!
//! This boot-time pass converges those SET-but-stale keys to the live membership
//! key and CASCADES the holder rows (`shard_locations.peer_id`,
//! `rea_commitments.provider`) in the SAME transaction — because healing the human
//! alone would realign the commitment side while STRANDING the holder side under
//! the dead key (the fossil-to-fossil `shard == humans` match that currently
//! aligns). Attribution follows the human: same pod, same bytes, new key.
//!
//! # Safe pairing (the load-bearing constraint)
//!
//! The qahal `Membership` entry carries only `member_cid` (`agent:{pubkey}`) —
//! NO stable slug `human_id`. Across a rekey a fossil slug row
//! (`human-adam-firstman`, key `uhCAkOLD`) cannot be matched to its live
//! membership (`agent:uhCAkNEW`) by key or by id — only by household. Within a
//! household the pairing of a fossil to a live key is therefore by ELIMINATION,
//! and elimination is unambiguous ONLY when exactly one live key is unaccounted
//! for AND exactly one orphan fossil exists (the bijection is then forced). Any
//! other shape is left untouched:
//!
//! - 0 orphan fossils → nothing stale.
//! - 0 unmatched live keys → every member already projected.
//! - ≥2 on either side → GENUINELY AMBIGUOUS. Superseding the wrong fossil would
//!   mis-attribute one human's identity AND shards to another — an identity
//!   violation strictly worse than a still-stale row. We log + skip; never guess.
//! - ANY WITHDRAWN member in the household's membership history → ABSTAIN. A
//!   cleanly-departed member contributes NO live key to `membership_keys`
//!   (the replayer drops withdrawn members), yet its `humans` row lingers with
//!   `household_id` still set — surfacing as an ORPHAN FOSSIL with no live-key
//!   partner. A withdrawal + one genuinely-new member then collapses the visible
//!   sets to a FALSE `1-unmatched / 1-orphan`, re-keying the DEPARTED member's
//!   shards + commitments onto the NEW member's key. That is the same
//!   mis-attribution the ambiguity guard forbids; the completeness guard alone
//!   does NOT catch it (the read is complete). So a household flagged
//!   `withdrawn` (see [`crate::services::holochain_humans_replayer`]) abstains.
//!
//! This is the conservative realisation of the "supersede when the membership key
//! differs from a set local key" rule: correct-or-abstain, never mis-attribute.
//! (Residual: a household where ≥2 members re-key simultaneously with no already-
//! current member stays stale until enough members are projected to force the
//! bijection. This is safe — it degrades to the pre-existing dark state, never to
//! a wrong attribution.)
//!
//! # h_app_id scoping (silent-no-op guard)
//!
//! `humans` is matched WITHOUT an `h_app_id` filter — mirroring every other
//! consumer (`on_membership_projected`, `run_once_by_membership`, and the
//! resilience humans join are all `h_app_id`-agnostic on `humans`; the rows live
//! under `imagodei` but are always joined by key/id/household only). The CASCADE
//! targets (`shard_locations`, `rea_commitments`) ARE scoped to the dataplane the
//! resilience card reads — `cascade_h_app_id` = `lamad` (the card's
//! `default_lamad` ctx). Getting either scope wrong ships a silent no-op.

use std::collections::HashSet;

use diesel::prelude::*;

use crate::db::DbPool;
use crate::StorageError;

/// Outcome counts for one reconciliation pass (returned for logging + tests).
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReconcileStats {
    /// Human rows whose stale `agent_pub_key` was superseded to the live key.
    pub superseded: usize,
    /// `shard_locations` rows re-attributed by the cascade.
    pub shard_locations_reattributed: usize,
    /// `rea_commitments` rows re-attributed by the cascade.
    pub commitments_reattributed: usize,
    /// Households skipped because the fossil↔membership pairing was not 1:1.
    pub ambiguous_skipped: usize,
    /// Membership `member_cid`s skipped because they were not `agent_cid`-shaped.
    pub non_agent_cid_skipped: usize,
    /// Households skipped because their membership READ was incomplete (a
    /// dropped/undecodable record could hide a member, so the visible sets might
    /// form a FALSE forced 1:1). Distinct from `ambiguous_skipped`, which is a
    /// complete read that is genuinely ≥2 either side.
    pub incomplete_read_skipped: usize,
    /// Households skipped because ≥1 Person member has cleanly WITHDRAWN. A
    /// departed member's `humans` row lingers as an orphan fossil with no live-key
    /// partner, so a withdrawal + a new join would collapse the forced-bijection
    /// guard into a FALSE 1:1 that re-keys the departed member's shards onto the
    /// new member's key. Distinct from `incomplete_read_skipped` (the read here is
    /// COMPLETE — the departure is legitimate, not a dropped record).
    pub withdrawn_abstain_skipped: usize,
}

/// Short, log-safe prefix of an agent key for the transition record (mirrors
/// `genesis_self_heal::key_prefix`).
fn key_prefix(key: &str) -> String {
    key.chars().take(12).collect()
}

/// Reconcile SET-but-stale `humans.agent_pub_key` values to DHT membership truth,
/// cascading the holder rows. `mapping` is the SAME `(member_cid, household_cid)`
/// snapshot the household backfill consumes (`holochain_humans_replayer`).
///
/// `cascade_h_app_id` scopes the `shard_locations` / `rea_commitments` cascade to
/// the dataplane the resilience card reads (`lamad`). Humans are matched
/// `h_app_id`-agnostically (see module docs).
///
/// `incomplete_households` is the set of `household_cid`s whose membership READ
/// was incomplete (from [`crate::services::holochain_humans_replayer::MembershipSnapshot`]).
/// Any household in this set is treated as ambiguous and ABSTAINED — a
/// dropped/undecodable member could otherwise collapse the forced-bijection guard
/// into a FALSE 1:1. Pass an empty set when completeness is guaranteed.
///
/// `withdrawn_households` is the set of `household_cid`s with ≥1 cleanly-withdrawn
/// Person member (same snapshot). Any household in this set ABSTAINS too: a
/// departed member's lingering `humans` row is an orphan fossil with no live-key
/// partner, so a withdrawal + a new join collapses the guard into a FALSE 1:1 that
/// mis-attributes the departed member's shards onto the new member's key. Pass an
/// empty set when no departures are possible.
///
/// Best-effort and idempotent: it never errors on "nothing to do", and a second
/// run over the same mapping is a no-op (the fossils are gone). A DB failure on
/// ONE household is logged and skipped — it never aborts the remaining households
/// in the pass (arbitrary HashMap order must not pick victims).
pub fn reconcile_membership_keys(
    pool: &DbPool,
    cascade_h_app_id: &str,
    mapping: Vec<(String, String)>,
    incomplete_households: &HashSet<String>,
    withdrawn_households: &HashSet<String>,
) -> Result<ReconcileStats, StorageError> {
    let mut stats = ReconcileStats::default();
    if mapping.is_empty() {
        return Ok(stats);
    }

    // Group current membership agent-keys by household. `member_cid` is the
    // imagodei `encode_agent_cid` form `agent:{pubkey}`; strip to the bare
    // `agent_cid` the join-key columns hold. A `member_cid` that is NOT
    // `agent_cid`-shaped (a transport id) must NEVER be written into an
    // `agent_cid` join-key column — observe + skip (mirrors on_membership_projected
    // and salvage_commitment_author).
    let mut by_household: std::collections::HashMap<String, HashSet<String>> =
        std::collections::HashMap::new();
    for (member_cid, household_cid) in &mapping {
        let key = member_cid
            .strip_prefix("agent:")
            .unwrap_or(member_cid)
            .trim();
        if !crate::identity_namespace::is_agent_cid(key) {
            crate::identity_namespace::observe_agent_cid_write("humans.agent_pub_key", Some(key));
            crate::metrics::inc_identity_key_supersede("non_agent_cid_skip", 1);
            stats.non_agent_cid_skipped += 1;
            continue;
        }
        by_household
            .entry(household_cid.clone())
            .or_default()
            .insert(key.to_string());
    }

    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    for (household_cid, membership_keys) in &by_household {
        // Per-household error ISOLATION: a DB failure on one household must not
        // abort the rest of the pass (HashMap order is arbitrary — propagating
        // the first Err would pick arbitrary victims that boot). Warn + continue,
        // consistent with this module's per-item degradation elsewhere.
        if let Err(e) = reconcile_one_household(
            &mut conn,
            cascade_h_app_id,
            household_cid,
            membership_keys,
            incomplete_households,
            withdrawn_households,
            &mut stats,
        ) {
            tracing::warn!(
                household_cid = %household_cid,
                error = %e,
                "membership_identity_reconcile: household reconcile failed (non-fatal, skipping this household)"
            );
        }
    }

    if stats.superseded > 0
        || stats.ambiguous_skipped > 0
        || stats.non_agent_cid_skipped > 0
        || stats.incomplete_read_skipped > 0
        || stats.withdrawn_abstain_skipped > 0
    {
        tracing::info!(
            superseded = stats.superseded,
            shard_locations_reattributed = stats.shard_locations_reattributed,
            commitments_reattributed = stats.commitments_reattributed,
            ambiguous_skipped = stats.ambiguous_skipped,
            non_agent_cid_skipped = stats.non_agent_cid_skipped,
            incomplete_read_skipped = stats.incomplete_read_skipped,
            withdrawn_abstain_skipped = stats.withdrawn_abstain_skipped,
            "membership_identity_reconcile: key-supersede pass complete"
        );
    }
    Ok(stats)
}

/// Reconcile a single household's humans against its current membership keys.
fn reconcile_one_household(
    conn: &mut SqliteConnection,
    cascade_h_app_id: &str,
    household_cid: &str,
    membership_keys: &HashSet<String>,
    incomplete_households: &HashSet<String>,
    withdrawn_households: &HashSet<String>,
    stats: &mut ReconcileStats,
) -> Result<(), StorageError> {
    use crate::db::diesel_schema::humans;

    // INCOMPLETE READ → ABSTAIN. If this household's membership read dropped a
    // record (undecodable) or failed, `membership_keys` may be MISSING a real
    // member — the visible sets could form a FALSE forced 1:1 that supersedes one
    // human's fossil onto another's live key. The forced-bijection guard is only
    // sound over a COMPLETE member set, so an incomplete household is treated
    // exactly like an ambiguous one: observe + skip, never guess.
    if incomplete_households.contains(household_cid) {
        stats.incomplete_read_skipped += 1;
        crate::metrics::inc_identity_key_supersede("incomplete_read_skip", 1);
        tracing::warn!(
            household_cid = %household_cid,
            "membership_identity_reconcile: incomplete membership read — supersede skipped (a dropped member could force a false 1:1)"
        );
        return Ok(());
    }

    // WITHDRAWN MEMBER → ABSTAIN. A cleanly-departed Person contributes NO key to
    // `membership_keys`, but its `humans` row lingers as an orphan fossil with no
    // live-key partner. The read is COMPLETE (so the incomplete guard above does
    // not fire), yet a withdrawal + one genuinely-new member collapses the visible
    // sets into a FALSE `1-unmatched / 1-orphan` that re-keys the DEPARTED member's
    // shards + commitments onto the NEW member's key — a mis-attribution strictly
    // worse than a stale row. Abstain for any household with a departure in its
    // membership history (degrades to the pre-existing dark state, never a wrong
    // attribution). See module docs + `holochain_humans_replayer`.
    if withdrawn_households.contains(household_cid) {
        stats.withdrawn_abstain_skipped += 1;
        crate::metrics::inc_identity_key_supersede("withdrawn_abstain_skip", 1);
        tracing::warn!(
            household_cid = %household_cid,
            "membership_identity_reconcile: household has a withdrawn member — supersede skipped (a departed fossil has no live-key partner and would force a false 1:1)"
        );
        return Ok(());
    }

    // Humans stamped into this household. NO h_app_id filter — mirrors every
    // other consumer of `humans` (see module docs).
    let rows: Vec<(String, Option<String>)> = humans::table
        .filter(humans::household_id.eq(household_cid))
        .select((humans::id, humans::agent_pub_key))
        .load(conn)
        .map_err(|e| {
            StorageError::Internal(format!("membership_identity_reconcile load humans: {e}"))
        })?;

    // Keys currently held by SOME human in this household.
    let held: HashSet<String> = rows.iter().filter_map(|(_, k)| k.clone()).collect();

    // Live membership keys not held by any human here (candidate live keys).
    let unmatched: Vec<&String> = membership_keys
        .iter()
        .filter(|k| !held.contains(*k))
        .collect();

    // Orphan fossils: rows with a SET, valid `agent_cid` key that is NOT a current
    // membership key of this household — a stale key a re-key left behind. (A NULL
    // key is NOT an orphan; NULL-fill is the signal/backfill path's job, and a
    // household-scoped NULL fill would be just as ambiguous as a wrong supersede.)
    let orphan_fossils: Vec<(&String, &String)> = rows
        .iter()
        .filter_map(|(id, k)| {
            let k = k.as_ref()?;
            if crate::identity_namespace::is_agent_cid(k) && !membership_keys.contains(k) {
                Some((id, k))
            } else {
                None
            }
        })
        .collect();

    // SAFE PAIRING: fire only when the bijection is forced (exactly one live key
    // unaccounted-for AND exactly one orphan fossil). See module docs.
    if unmatched.len() == 1 && orphan_fossils.len() == 1 {
        let new_key = unmatched[0];
        let (human_id, old_key) = orphan_fossils[0];

        // Supersede + cascade in ONE transaction: no partial state. The human CAS
        // and both cascades either all land or all roll back.
        let (superseded, shards, commits) = conn
            .transaction::<(usize, usize, usize), StorageError, _>(|conn| {
                let n =
                    crate::db::humans::supersede_human_agent_key(conn, human_id, old_key, new_key)?;
                if n == 0 {
                    // CAS aborted (the key already moved) — do NOT cascade against
                    // a key that no longer describes this human.
                    return Ok((0, 0, 0));
                }
                let shards = crate::db::shard_locations::rekey_peer_id(
                    conn,
                    cascade_h_app_id,
                    old_key,
                    new_key,
                )?;
                let commits = crate::db::rea_commitments::rekey_provider(
                    conn,
                    cascade_h_app_id,
                    old_key,
                    new_key,
                )?;
                Ok((n, shards, commits))
            })?;

        if superseded > 0 {
            stats.superseded += superseded;
            stats.shard_locations_reattributed += shards;
            stats.commitments_reattributed += commits;
            crate::metrics::inc_identity_key_supersede("supersede", superseded as u64);
            crate::metrics::inc_identity_key_supersede("shard_locations", shards as u64);
            crate::metrics::inc_identity_key_supersede("rea_commitments", commits as u64);
            tracing::info!(
                household_cid = %household_cid,
                human_id = %human_id,
                old_key_prefix = %key_prefix(old_key),
                new_key_prefix = %key_prefix(new_key),
                shard_locations_reattributed = shards,
                commitments_reattributed = commits,
                source = "membership-truth-key-supersede",
                "membership_identity_reconcile: superseded stale human agent key + cascaded holder rows"
            );
        }
    } else if !unmatched.is_empty() && !orphan_fossils.is_empty() {
        // Both non-empty but not a forced 1:1 — genuinely ambiguous. Never guess.
        stats.ambiguous_skipped += 1;
        crate::metrics::inc_identity_key_supersede("ambiguous_skip", 1);
        tracing::warn!(
            household_cid = %household_cid,
            unmatched_membership_keys = unmatched.len(),
            orphan_fossils = orphan_fossils.len(),
            "membership_identity_reconcile: ambiguous fossil↔membership pairing — supersede skipped (never mis-attribute)"
        );
    }
    // else: no orphan fossils (nothing stale) or no unmatched keys (all current) —
    // nothing to do.
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::humans::{create_human, get_human_by_id, CreateHumanInput};
    use crate::db::models::NewReaCommitment;
    use crate::db::models::NewShardLocation;
    use crate::db::DbPool;

    fn pool() -> DbPool {
        crate::test_util::test_pool()
    }

    /// Seed a slug-keyed human with an explicit key + household (h_app_id
    /// "imagodei" — the production scope; the reconcile is h_app_id-agnostic on
    /// humans). `None` key = the NULL-fill shape.
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
                h_app_id: "imagodei".to_string(),
                household_id: household.map(|h| h.to_string()),
            },
        )
        .expect("seed human");
    }

    fn seed_shard(conn: &mut SqliteConnection, shard: &str, peer: &str) {
        crate::db::shard_locations::upsert_location(
            conn,
            &NewShardLocation {
                shard_hash: shard,
                peer_id: peer,
                h_app_id: "lamad",
                status: "verified",
            },
        )
        .expect("seed shard");
    }

    /// Seed an ACTIVE `provide` commitment under `provider`, classified for the
    /// commons scope — the shape the resilience commitment-backed join counts.
    fn seed_provide(conn: &mut SqliteConnection, id: &str, provider: &str) {
        diesel::insert_into(crate::db::diesel_schema::rea_commitments::table)
            .values(&NewReaCommitment {
                id,
                h_app_id: "lamad",
                action: "provide",
                provider,
                receiver: "collective:eden",
                resource_conforms_to: Some("content"),
                resource_classified_as: Some("[\"content:commons\"]"),
                resource_quantity_value: Some(1.0),
                resource_quantity_unit: Some("doc"),
                effort_quantity_value: None,
                effort_quantity_unit: None,
                has_beginning: None,
                has_end: None,
                due: None,
                clause_of: None,
                in_scope_of: None,
                medium_of_exchange_id: None,
                state: "active",
                finished: 0,
                note: None,
                metadata_json: None,
                dht_anchor_hash: None,
            })
            .execute(conn)
            .expect("seed provide commitment");
    }

    /// Mirror the resilience commitment-backed join (`household_resilience.rs`
    /// lines 172-208): distinct households with an active provide commitment whose
    /// provider joins a human's key. Returns the distinct household count — the
    /// value the card renders. Used to prove the card lights.
    fn commitment_backed_household_count(conn: &mut SqliteConnection) -> usize {
        use crate::db::diesel_schema::{humans, rea_commitments};
        let rows: Vec<Option<String>> = rea_commitments::table
            .inner_join(
                humans::table.on(humans::agent_pub_key
                    .nullable()
                    .eq(rea_commitments::provider.nullable())),
            )
            .filter(rea_commitments::h_app_id.eq("lamad"))
            .filter(rea_commitments::action.eq("provide"))
            .filter(rea_commitments::state.eq("active"))
            .filter(humans::household_id.is_not_null())
            .select(humans::household_id)
            .load(conn)
            .expect("commitment-backed join");
        rows.into_iter()
            .flatten()
            .collect::<HashSet<String>>()
            .len()
    }

    /// THE POINT: a fossil-shape household → after reconcile the humans row carries
    /// the live key, the shard rows are re-attributed, the commitment side aligns,
    /// and the resilience folds (holder relation + commitment-backed count) light.
    #[test]
    fn fossil_household_supersede_cascade_lights_the_card() {
        let p = pool();
        let mut conn = p.get().unwrap();

        // Post-rekey fossil state on this pod:
        //  - adam's projected human row holds the FOSSIL key.
        //  - adam's shard-holder rows are under the FOSSIL key (aligned fossil↔fossil).
        //  - adam's provide commitment was authored under the LIVE key (his re-keyed
        //    session), so it does NOT join the fossil human → commitment side dark.
        seed_human(
            &mut conn,
            "human-adam-firstman",
            Some("uhCAkOLDadam"),
            Some("collective:eden"),
        );
        seed_shard(&mut conn, "shard-1", "uhCAkOLDadam");
        seed_shard(&mut conn, "shard-2", "uhCAkOLDadam");
        seed_provide(&mut conn, "commit-adam", "uhCAkNEWadam");

        // Precondition: the commitment side is dark (provider NEW ≠ human OLD).
        assert_eq!(
            commitment_backed_household_count(&mut conn),
            0,
            "before: the live-key provide does not join the fossil human"
        );

        // Membership truth: adam's live key, in his household.
        let mapping = vec![(
            "agent:uhCAkNEWadam".to_string(),
            "collective:eden".to_string(),
        )];
        drop(conn);

        let stats =
            reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                .expect("reconcile");
        assert_eq!(stats.superseded, 1);
        assert_eq!(stats.shard_locations_reattributed, 2);
        // The provide was already under the new key → idempotent no-op on rea.
        assert_eq!(stats.commitments_reattributed, 0);

        let mut conn = p.get().unwrap();

        // Human row now carries the live key.
        let adam = get_human_by_id(&mut conn, "human-adam-firstman")
            .unwrap()
            .unwrap();
        assert_eq!(adam.agent_pub_key.as_deref(), Some("uhCAkNEWadam"));

        // Holder rows re-attributed to the live key (cascade kept the holder side
        // aligned — healing the human alone would have stranded these).
        assert!(
            crate::db::shard_locations::get_locations_for_peer(&mut conn, "uhCAkOLDadam")
                .unwrap()
                .is_empty()
        );
        assert_eq!(
            crate::db::shard_locations::get_locations_for_peer(&mut conn, "uhCAkNEWadam")
                .unwrap()
                .len(),
            2
        );

        // Holder relation (the actual resilience fold) now resolves adam's shards
        // to his household under the live key.
        let relation = crate::services::household_resilience::load_holder_relation(
            &mut conn,
            "lamad",
            &["shard-1".to_string(), "shard-2".to_string()],
        )
        .expect("holder relation");
        assert!(
            relation
                .iter()
                .any(|h| h.hub_id.as_deref() == Some("collective:eden")),
            "holder relation counts adam's household after cascade"
        );

        // Commitment side now lights: provider NEW == human NEW.
        assert_eq!(
            commitment_backed_household_count(&mut conn),
            1,
            "after: the card counts the household — supersede aligned the commitment side"
        );
    }

    /// SAFETY: a membership `member_cid` that is NOT agent_cid-shaped (a transport
    /// id) is never written — observed + skipped.
    #[test]
    fn non_agent_cid_membership_is_never_written() {
        let p = pool();
        let mut conn = p.get().unwrap();
        seed_human(
            &mut conn,
            "human-x",
            Some("uhCAkOLDx"),
            Some("collective:h"),
        );
        drop(conn);

        // A libp2p transport id masquerading as a member.
        let mapping = vec![(
            "12D3KooWtransportNotAnAgent".to_string(),
            "collective:h".to_string(),
        )];
        let stats =
            reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                .expect("reconcile");
        assert_eq!(stats.non_agent_cid_skipped, 1);
        assert_eq!(stats.superseded, 0);

        let mut conn = p.get().unwrap();
        let x = get_human_by_id(&mut conn, "human-x").unwrap().unwrap();
        assert_eq!(
            x.agent_pub_key.as_deref(),
            Some("uhCAkOLDx"),
            "a transport id must never supersede an agent_cid"
        );
    }

    /// SAFETY: a NULL-key human is NOT superseded here — NULL-fill is the
    /// signal/backfill path's job; this pass only moves SET-but-stale keys.
    #[test]
    fn null_key_human_is_not_touched() {
        let p = pool();
        let mut conn = p.get().unwrap();
        seed_human(&mut conn, "human-null", None, Some("collective:h"));
        drop(conn);

        let mapping = vec![("agent:uhCAkLIVE".to_string(), "collective:h".to_string())];
        let stats =
            reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                .expect("reconcile");
        assert_eq!(stats.superseded, 0, "NULL is the fill path, not supersede");

        let mut conn = p.get().unwrap();
        let h = get_human_by_id(&mut conn, "human-null").unwrap().unwrap();
        assert_eq!(h.agent_pub_key, None, "a NULL key stays NULL in this pass");
    }

    /// SAFETY: a household with ≥2 orphan fossils (all members re-keyed at once,
    /// none yet projected) is AMBIGUOUS — no supersede fires, no row is touched.
    /// Mis-pairing would attribute one human's identity to another.
    #[test]
    fn ambiguous_multi_fossil_household_is_skipped() {
        let p = pool();
        let mut conn = p.get().unwrap();
        seed_human(
            &mut conn,
            "human-adam",
            Some("uhCAkOLDadam"),
            Some("collective:eden"),
        );
        seed_human(
            &mut conn,
            "human-eve",
            Some("uhCAkOLDeve"),
            Some("collective:eden"),
        );
        drop(conn);

        // Two live membership keys, two fossils, none currently matched → 2×2.
        let mapping = vec![
            (
                "agent:uhCAkNEWadam".to_string(),
                "collective:eden".to_string(),
            ),
            (
                "agent:uhCAkNEWeve".to_string(),
                "collective:eden".to_string(),
            ),
        ];
        let stats =
            reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                .expect("reconcile");
        assert_eq!(stats.superseded, 0);
        assert_eq!(stats.ambiguous_skipped, 1);

        let mut conn = p.get().unwrap();
        assert_eq!(
            get_human_by_id(&mut conn, "human-adam")
                .unwrap()
                .unwrap()
                .agent_pub_key
                .as_deref(),
            Some("uhCAkOLDadam"),
        );
        assert_eq!(
            get_human_by_id(&mut conn, "human-eve")
                .unwrap()
                .unwrap()
                .agent_pub_key
                .as_deref(),
            Some("uhCAkOLDeve"),
        );
    }

    /// The single-fossil-among-already-current case: one member is already
    /// projected on the live key, so the remaining orphan pairs unambiguously to
    /// the one unmatched live key. This is the realistic post-signal-path shape.
    #[test]
    fn single_fossil_with_one_current_member_supersedes() {
        let p = pool();
        let mut conn = p.get().unwrap();
        // eve is already on her live key (signal path stamped her NULL→live);
        // adam is the lone fossil.
        seed_human(
            &mut conn,
            "human-eve",
            Some("uhCAkNEWeve"),
            Some("collective:eden"),
        );
        seed_human(
            &mut conn,
            "human-adam",
            Some("uhCAkOLDadam"),
            Some("collective:eden"),
        );
        drop(conn);

        let mapping = vec![
            (
                "agent:uhCAkNEWeve".to_string(),
                "collective:eden".to_string(),
            ),
            (
                "agent:uhCAkNEWadam".to_string(),
                "collective:eden".to_string(),
            ),
        ];
        let stats =
            reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                .expect("reconcile");
        assert_eq!(
            stats.superseded, 1,
            "the lone fossil pairs to the lone unmatched key"
        );
        assert_eq!(stats.ambiguous_skipped, 0);

        let mut conn = p.get().unwrap();
        assert_eq!(
            get_human_by_id(&mut conn, "human-adam")
                .unwrap()
                .unwrap()
                .agent_pub_key
                .as_deref(),
            Some("uhCAkNEWadam"),
        );
        // eve untouched (already current).
        assert_eq!(
            get_human_by_id(&mut conn, "human-eve")
                .unwrap()
                .unwrap()
                .agent_pub_key
                .as_deref(),
            Some("uhCAkNEWeve"),
        );
    }

    /// IDEMPOTENT: a second pass over the same mapping is a no-op (the fossil is
    /// gone; the membership key now matches the human).
    #[test]
    fn second_pass_is_a_noop() {
        let p = pool();
        let mut conn = p.get().unwrap();
        seed_human(
            &mut conn,
            "human-adam",
            Some("uhCAkOLDadam"),
            Some("collective:eden"),
        );
        seed_shard(&mut conn, "shard-1", "uhCAkOLDadam");
        drop(conn);

        let mapping = vec![(
            "agent:uhCAkNEWadam".to_string(),
            "collective:eden".to_string(),
        )];
        let first = reconcile_membership_keys(
            &p,
            "lamad",
            mapping.clone(),
            &HashSet::new(),
            &HashSet::new(),
        )
        .expect("first");
        assert_eq!(first.superseded, 1);
        let second =
            reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                .expect("second");
        assert_eq!(second.superseded, 0);
        assert_eq!(second.ambiguous_skipped, 0);
    }

    /// REVIEWER TRACE (mis-attribution surface): household `hh` has a real fossil
    /// A (rekeyed, humans row on the OLD key) + a same-boot new-join B (no humans
    /// row). If A's OWN membership record fails to decode this boot, the visible
    /// set collapses to `{B_new}` unmatched / `{A_fossil}` orphan — a FALSE forced
    /// 1:1 that would supersede A's fossil onto B's LIVE key. Marking `hh`
    /// incomplete makes reconcile abstain. This test proves BOTH halves: the bug
    /// fires without the guard, and the guard prevents it.
    #[test]
    fn incomplete_read_abstains_on_decode_dropped_member_plus_new_join() {
        let hh = "collective:eden";

        // (contrast) WITHOUT the completeness guard the false 1:1 FIRES — proving
        // the guard is load-bearing, not decorative. A's fossil is mis-attributed
        // onto B's live key (the exact mis-attribution the fix prevents).
        {
            let p = pool();
            {
                let mut conn = p.get().unwrap();
                seed_human(&mut conn, "human-adam", Some("uhCAkOLDadam"), Some(hh));
            }
            // Only B's membership survived decode (A's was dropped).
            let mapping = vec![("agent:uhCAkNEWbob".to_string(), hh.to_string())];
            let stats =
                reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                    .expect("reconcile without completeness guard");
            assert_eq!(stats.superseded, 1, "without the guard the false 1:1 fires");
            let adam = get_human_by_id(&mut p.get().unwrap(), "human-adam")
                .unwrap()
                .unwrap();
            assert_eq!(
                adam.agent_pub_key.as_deref(),
                Some("uhCAkNEWbob"),
                "the bug: A's fossil mis-attributed onto B's live key"
            );
        }

        // (fix) WITH `hh` marked incomplete, reconcile ABSTAINS — A keeps its
        // fossil, no mis-attribution; the skip is counted + observable.
        {
            let p = pool();
            {
                let mut conn = p.get().unwrap();
                seed_human(&mut conn, "human-adam", Some("uhCAkOLDadam"), Some(hh));
            }
            let mapping = vec![("agent:uhCAkNEWbob".to_string(), hh.to_string())];
            let incomplete: HashSet<String> = std::iter::once(hh.to_string()).collect();
            let stats =
                reconcile_membership_keys(&p, "lamad", mapping, &incomplete, &HashSet::new())
                    .expect("reconcile with completeness guard");
            assert_eq!(stats.superseded, 0);
            assert_eq!(stats.incomplete_read_skipped, 1);
            assert_eq!(stats.ambiguous_skipped, 0);
            let adam = get_human_by_id(&mut p.get().unwrap(), "human-adam")
                .unwrap()
                .unwrap();
            assert_eq!(
                adam.agent_pub_key.as_deref(),
                Some("uhCAkOLDadam"),
                "the fix: an incomplete read abstains — A keeps its fossil"
            );
        }
    }

    /// REGRESSION (a) — withdrawal + new join must NOT force-map.
    ///
    /// Household `hh` has a WITHDRAWN member A (its `humans` row lingers on key
    /// `uhCAkOLDadam`, `household_id` still set) plus a genuinely-NEW member B
    /// (`agent:uhCAkNEWbob`, no `humans` row yet). The replayer drops the withdrawn
    /// member from `membership_keys`, so the visible sets collapse to
    /// `{B_new}` unmatched / `{A_fossil}` orphan — a FALSE forced 1:1. WITHOUT the
    /// withdrawn guard this re-keys A's identity + shards onto B's live key; WITH
    /// the guard the household abstains. Proves BOTH halves (the completeness guard
    /// alone does NOT catch this — the read is complete).
    #[test]
    fn withdrawn_member_plus_new_join_abstains_not_force_maps() {
        let hh = "collective:eden";

        // (contrast) WITHOUT the withdrawn flag the false 1:1 FIRES — the departed
        // member's fossil is mis-attributed onto the new member's live key.
        {
            let p = pool();
            {
                let mut conn = p.get().unwrap();
                seed_human(&mut conn, "human-adam", Some("uhCAkOLDadam"), Some(hh));
            }
            // Only B (the new join) is in current membership; A withdrew (dropped).
            let mapping = vec![("agent:uhCAkNEWbob".to_string(), hh.to_string())];
            let stats =
                reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                    .expect("reconcile without withdrawn guard");
            assert_eq!(
                stats.superseded, 1,
                "without the withdrawn guard the false 1:1 fires"
            );
            let adam = get_human_by_id(&mut p.get().unwrap(), "human-adam")
                .unwrap()
                .unwrap();
            assert_eq!(
                adam.agent_pub_key.as_deref(),
                Some("uhCAkNEWbob"),
                "the bug: the departed member's fossil mis-attributed onto the new member's key"
            );
        }

        // (fix) WITH `hh` flagged withdrawn, reconcile ABSTAINS — A keeps its
        // fossil, no mis-attribution; the skip is counted + observable and is
        // distinct from the incomplete-read skip.
        {
            let p = pool();
            {
                let mut conn = p.get().unwrap();
                seed_human(&mut conn, "human-adam", Some("uhCAkOLDadam"), Some(hh));
            }
            let mapping = vec![("agent:uhCAkNEWbob".to_string(), hh.to_string())];
            let withdrawn: HashSet<String> = std::iter::once(hh.to_string()).collect();
            let stats =
                reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &withdrawn)
                    .expect("reconcile with withdrawn guard");
            assert_eq!(stats.superseded, 0);
            assert_eq!(stats.withdrawn_abstain_skipped, 1);
            assert_eq!(stats.incomplete_read_skipped, 0);
            assert_eq!(stats.ambiguous_skipped, 0);
            let adam = get_human_by_id(&mut p.get().unwrap(), "human-adam")
                .unwrap()
                .unwrap();
            assert_eq!(
                adam.agent_pub_key.as_deref(),
                Some("uhCAkOLDadam"),
                "the fix: a household with a withdrawal abstains — the departed member keeps its key"
            );
        }
    }

    /// REGRESSION (b) — the clean supersede case still maps when NO household is
    /// flagged withdrawn (empty withdrawn set). A lone fossil among an already-
    /// current member pairs unambiguously to the one unmatched live key.
    #[test]
    fn clean_supersede_still_maps_with_empty_withdrawn_set() {
        let p = pool();
        let mut conn = p.get().unwrap();
        seed_human(
            &mut conn,
            "human-eve",
            Some("uhCAkNEWeve"),
            Some("collective:eden"),
        );
        seed_human(
            &mut conn,
            "human-adam",
            Some("uhCAkOLDadam"),
            Some("collective:eden"),
        );
        seed_shard(&mut conn, "shard-1", "uhCAkOLDadam");
        drop(conn);

        let mapping = vec![
            (
                "agent:uhCAkNEWeve".to_string(),
                "collective:eden".to_string(),
            ),
            (
                "agent:uhCAkNEWadam".to_string(),
                "collective:eden".to_string(),
            ),
        ];
        let stats =
            reconcile_membership_keys(&p, "lamad", mapping, &HashSet::new(), &HashSet::new())
                .expect("reconcile");
        assert_eq!(
            stats.superseded, 1,
            "no withdrawal flagged → the clean forced-bijection supersede still fires"
        );
        assert_eq!(stats.withdrawn_abstain_skipped, 0);
        assert_eq!(stats.shard_locations_reattributed, 1);

        let mut conn = p.get().unwrap();
        assert_eq!(
            get_human_by_id(&mut conn, "human-adam")
                .unwrap()
                .unwrap()
                .agent_pub_key
                .as_deref(),
            Some("uhCAkNEWadam"),
        );
    }
}
