//! Provides a snapshot of (memberCid, householdCid) pairs from current DHT
//! household memberships, for the `household_backfill` startup pass.
//!
//! ## Why this is a two-step traversal (storage projection → DHT)
//!
//! Household membership is NOT carried on the `Human` DHT entry — the `Human`
//! integrity struct has no `household_id` field. Membership lives in the qahal
//! collectives/memberships graph: a `Membership` entry references both a
//! `member_cid` (`agent:<pubkey>`) and a `collective_cid`
//! (`collective:<action_hash>`), and only collectives whose `governance_layer`
//! is `family` are households.
//!
//! The imagodei DNA exposes NO per-conductor enumerator of collectives or
//! memberships (no `list_collectives`, no `get_my_memberships`). The only
//! membership read is `list_memberships_for_collective_cid(collective_cid)`,
//! which requires the collective_cid up front. We therefore seed the household
//! set from the LOCAL `collectives` projection (governance_layer == family,
//! collective_cid present) — the storage projection of the collectives DHT
//! truth — and then read the authoritative member list back from the conductor
//! per household. The membership pairs returned are DHT truth; the seed list is
//! only the set of cids to ask about.
//!
//! ## Composition with the live reconcile path
//!
//! The runtime stamp (`reconcile::controller::on_membership_projected`) writes
//! `humans.household_id = collective_cid` for household memberships. This boot
//! pass produces the SAME value for the SAME (member, household) pair, so the
//! one-shot backfill and the live signal path are byte-identical — the backfill
//! is pure catch-up for rows that missed the live signal (legacy / null rows).
//! `household_backfill::run_once_by_membership` applies the NULL-only,
//! dual-match (id OR stripped agent_pub_key) update that mirrors the controller.
//!
//! ## DHT-availability tolerance
//!
//! Every conductor read is best-effort: a missing imagodei client, an
//! unreachable conductor, or a single malformed membership record degrades to a
//! warning and a smaller (possibly empty) mapping — never a startup failure. An
//! empty mapping is a valid outcome (the backfill is idempotent and no-op-safe).

use std::sync::Arc;

use async_trait::async_trait;
use holochain_types::prelude::{Entry, Record};
use serde::Deserialize;

use crate::db::collectives::{list_collectives, CollectiveQuery};
use crate::db::models::governance_layers;
use crate::db::{AppContext, DbPool};
use crate::hc_client::HcClient;
use crate::StorageError;

/// The imagodei zome name / role that hosts the qahal coordinator
/// (`list_memberships_for_collective_cid`).
const IMAGODEI_ZOME: &str = "imagodei";
const LIST_MEMBERSHIPS_FN: &str = "list_memberships_for_collective_cid";

/// Minimal projection of the qahal `Membership` integrity entry — only the
/// fields the backfill needs. Decoded with `rmp_serde::from_slice` from the
/// raw `Entry::App` bytes (the same MessagePack-named encoding the conductor
/// uses); these are all plain strings, so no `holo_hash` byte-array fields are
/// involved (the `serde_json::Value` pre-pass trap does not apply here).
///
/// We intentionally avoid pulling the qahal integrity crate as a storage dep —
/// a structurally-faithful local mirror is sufficient and keeps the wasm
/// version-coupling out of the native build.
#[derive(Debug, Clone, Deserialize)]
struct MembershipWire {
    member_cid: String,
    /// Serialized externally-tagged unit variant: `"Person"` / `"Collective"` /
    /// `"ElohimAgent"`. Only `Person` members map to a household human row.
    member_kind: String,
    /// Present in the wire shape for fidelity, but intentionally NOT consulted:
    /// the emitted household_id is the cid we QUERIED on, matching the controller
    /// (which stamps the collective_cid it dispatched on), so a drifted entry
    /// field can never skew the stamp. See `extract_household_pairs`.
    #[allow(dead_code)]
    collective_cid: String,
    /// Set when the membership was cleanly withdrawn; such members no longer
    /// belong to the household and must not be stamped.
    #[serde(default)]
    withdrawn_at_block_height: Option<u64>,
}

const MEMBER_KIND_PERSON: &str = "Person";

/// Reads the membership records for a single household collective from the
/// conductor. The seam exists so the orchestration can be unit-tested with a
/// typed mock and the pure extraction logic exercised without a live conductor.
#[async_trait]
pub trait MembershipReader: Send + Sync {
    /// Return the raw `Record`s for the given `collective_cid`, mirroring the
    /// imagodei `list_memberships_for_collective_cid` return shape.
    ///
    /// Errors are surfaced so the orchestrator can decide per-collective whether
    /// to skip-and-warn; they never abort the whole snapshot.
    async fn list_memberships(&self, collective_cid: &str) -> Result<Vec<Record>, StorageError>;
}

/// Production reader: calls the imagodei coordinator via the conductor client.
pub struct ConductorMembershipReader {
    pub hc_client: Arc<HcClient>,
}

#[async_trait]
impl MembershipReader for ConductorMembershipReader {
    async fn list_memberships(&self, collective_cid: &str) -> Result<Vec<Record>, StorageError> {
        let payload = rmp_serde::to_vec_named(&collective_cid.to_string()).map_err(|e| {
            StorageError::Internal(format!(
                "holochain_humans_replayer: encode collective_cid: {e}"
            ))
        })?;
        let bytes = self
            .hc_client
            .call_zome_imagodei(IMAGODEI_ZOME, LIST_MEMBERSHIPS_FN, payload)
            .await?;
        // `Vec<Record>` decodes through the typed Record deserializer — the
        // holo_hash fields inside each SignedActionHashed survive because we do
        // NOT go through a serde_json::Value pre-pass.
        let records: Vec<Record> = rmp_serde::from_slice(&bytes).map_err(|e| {
            StorageError::Serialization(format!(
                "holochain_humans_replayer: decode Vec<Record>: {e}"
            ))
        })?;
        Ok(records)
    }
}

/// Pure extraction: turn a household's membership `Record`s into
/// `(member_cid, household_cid)` pairs, plus an INCOMPLETE-read flag and a
/// HAD-WITHDRAWN flag.
///
/// - Only `MemberKind::Person` members are emitted (collectives-of-collectives
///   and elohim-agent members never carry a household_id).
/// - Withdrawn Person members (`withdrawn_at_block_height` set) are dropped from
///   `pairs` AND flag `withdrawn_seen`.
/// - `household_cid` is the caller-supplied `collective_cid` — the exact value
///   the controller stamps — NOT re-derived from the entry, so a malformed
///   entry collective_cid cannot drift the stamp.
/// - A record whose entry is absent (a delete/tombstone — structurally not a
///   current membership) is skipped silently.
///
/// # Returns `(pairs, incomplete, withdrawn_seen)`
///
/// `incomplete` is `true` when ≥1 record was present-but-UNDECODABLE
/// (`rmp_serde` failed). This is load-bearing for the key-supersede reconcile:
/// its forced-bijection guard assumes the membership set for a household is
/// COMPLETE. A dropped-undecodable record can hide a REAL member, collapsing
/// `2-unmatched / 1-orphan` (safe abstain) into a FALSE `1-unmatched / 1-orphan`
/// that mis-attributes one human's fossil onto another's live key. So a household
/// with any decode failure is reported incomplete and the reconcile abstains.
///
/// `withdrawn_seen` is `true` when ≥1 Person member of this household has cleanly
/// WITHDRAWN. This is a DIFFERENT abstention trigger from `incomplete`: the read
/// is complete, but a withdrawn member's `humans` row lingers with `household_id`
/// still set, so it surfaces to the reconcile as an ORPHAN FOSSIL that has NO live
/// membership-key partner (the departed member contributes no key to `pairs`). A
/// withdrawn member plus one genuinely-new member then collapses the forced
/// bijection into a FALSE `1-unmatched / 1-orphan`, re-keying the departed member's
/// shards + commitments onto the new member's key — a mis-attribution strictly
/// worse than a stale row. So a household with any withdrawal is reported and the
/// reconcile abstains for it (correct-or-abstain; degrades to the pre-existing dark
/// state, never a wrong attribution). Non-Person / withdrawn-non-Person drops are
/// CORRECT exclusions and do NOT flag either signal.
fn extract_household_pairs(
    collective_cid: &str,
    records: &[Record],
) -> (Vec<(String, String)>, bool, bool) {
    let mut pairs = Vec::new();
    let mut incomplete = false;
    let mut withdrawn_seen = false;
    for record in records {
        let Some(Entry::App(eb)) = record.entry.as_option() else {
            // Membership entries are always App entries with a body; an absent
            // entry (e.g. a delete) is not a current membership.
            continue;
        };
        let membership: MembershipWire = match rmp_serde::from_slice(eb.bytes()) {
            Ok(m) => m,
            Err(e) => {
                // A record we could NOT read: it may be a Person membership we are
                // now silently missing. Mark the household incomplete so supersede
                // abstains rather than risk a false forced-bijection.
                incomplete = true;
                tracing::warn!(
                    collective_cid = %collective_cid,
                    error = %e,
                    "household replayer: undecodable membership record — marking household read INCOMPLETE (supersede will abstain)"
                );
                continue;
            }
        };
        if membership.member_kind != MEMBER_KIND_PERSON {
            continue;
        }
        if membership.withdrawn_at_block_height.is_some() {
            // A departed Person member: its `humans` row lingers as an orphan
            // fossil with no live-key partner. Flag the household so supersede
            // abstains (a withdrawal + a new join is exactly the false-1:1 shape).
            withdrawn_seen = true;
            tracing::debug!(
                collective_cid = %collective_cid,
                "household replayer: withdrawn Person member — marking household so supersede abstains (departed fossil has no live-key partner)"
            );
            continue;
        }
        pairs.push((membership.member_cid, collective_cid.to_string()));
    }
    (pairs, incomplete, withdrawn_seen)
}

/// Source the set of household collective cids from the local `collectives`
/// projection (governance_layer == family, collective_cid present). The
/// projection is the read-optimised cache of the collectives DHT truth; it is
/// the per-conductor index of "which households exist on this node" — but it is
/// EMPTY on a pod whose `CollectiveCommitted` signals landed on a different
/// conductor (the seeder single-targets one doorway). `identity_fill` unions
/// this with an index-free source-chain read to route around that gap, so this
/// is `pub(crate)` for that composition.
pub(crate) fn household_collective_cids(
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Vec<String>, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    let query = CollectiveQuery {
        governance_layer: Some(governance_layers::FAMILY.to_string()),
        reach: None,
        active_only: true,
        limit: 10_000,
        offset: 0,
    };
    let collectives = list_collectives(&mut conn, ctx, &query)?;
    Ok(collectives
        .into_iter()
        // Pre-coherence rows have a NULL collective_cid (not yet notarized) and
        // cannot be queried on the DHT — skip them; the live signal path will
        // stamp them once they notarize.
        .filter_map(|c| c.collective_cid)
        .collect())
}

/// Result of a boot membership snapshot: the `(member_cid, household_cid)` pairs
/// AND the sets of `household_cid`s the key-supersede reconcile must ABSTAIN on.
///
/// A household is INCOMPLETE when its list read failed entirely OR ≥1 of its
/// membership records failed to decode — in either case a REAL member may be
/// absent from `pairs`. A household is WITHDRAWN-flagged when ≥1 Person member has
/// cleanly departed — a departed member's `humans` row lingers as an orphan fossil
/// with no live-key partner in `pairs`. Both are abstention triggers for the
/// key-supersede reconcile (`membership_identity_reconcile`), because either shape
/// can collapse its forced-bijection guard into a FALSE 1:1 (mis-attributing one
/// human's fossil onto another's live key). They are tracked SEPARATELY so the two
/// abstention reasons stay observable (undecodable read vs legitimate departure).
///
/// The NULL-only `household_backfill` is unaffected by either — it consumes `pairs`
/// directly (a missing/departed member just leaves a `household_id` NULL, never a
/// wrong attribution).
#[derive(Debug, Default, Clone)]
pub struct MembershipSnapshot {
    pub pairs: Vec<(String, String)>,
    pub incomplete_households: std::collections::HashSet<String>,
    /// Households with ≥1 cleanly-withdrawn Person member — the reconcile abstains
    /// (the departed fossil has no live-key partner, so a forced 1:1 would
    /// mis-attribute it onto a new member's key).
    pub withdrawn_households: std::collections::HashSet<String>,
}

/// Boot-time snapshot of `(member_cid, household_cid)` pairs from the current
/// DHT household memberships, plus the set of households whose read was
/// incomplete (see [`MembershipSnapshot`]).
///
/// Tolerant of DHT unavailability: a missing/unreachable conductor or a
/// malformed record degrades to a smaller mapping (and marks that household
/// incomplete), never an error that fails startup. An empty mapping is a valid,
/// idempotent-safe outcome.
///
/// `ctx` scopes the household-collective projection lookup to the right app
/// (`humans`/`collectives` are h_app_id-scoped). `reader` is the conductor seam.
pub async fn snapshot_household_ids(
    pool: &DbPool,
    ctx: &AppContext,
    reader: &dyn MembershipReader,
) -> Result<MembershipSnapshot, StorageError> {
    let cids = match household_collective_cids(pool, ctx) {
        Ok(cids) => cids,
        Err(e) => {
            // Projection read failed — degrade to empty rather than fail boot.
            tracing::warn!(
                error = %e,
                "household replayer: could not read household collectives from projection; empty snapshot"
            );
            return Ok(MembershipSnapshot::default());
        }
    };

    snapshot_household_ids_for_cids(reader, &cids).await
}

/// Read the membership pairs for an EXPLICIT set of household collective cids —
/// the conductor half of [`snapshot_household_ids`], factored out so callers that
/// discover the cid set some other way (e.g. `identity_fill`, unioning the local
/// projection with an index-free source-chain read) can reuse the per-collective
/// read + incomplete/withdrawn accounting without re-deriving the cid set.
///
/// Same DHT-availability tolerance as [`snapshot_household_ids`]: a failed or
/// malformed per-collective read degrades to a smaller mapping (marking that
/// household incomplete), never an error. An empty `cids` yields the default
/// (empty) snapshot.
pub async fn snapshot_household_ids_for_cids(
    reader: &dyn MembershipReader,
    cids: &[String],
) -> Result<MembershipSnapshot, StorageError> {
    if cids.is_empty() {
        tracing::debug!("household replayer: no household collectives to read; empty snapshot");
        return Ok(MembershipSnapshot::default());
    }

    let mut snapshot = MembershipSnapshot::default();
    for cid in cids {
        match reader.list_memberships(cid).await {
            Ok(records) => {
                let (pairs, incomplete, withdrawn_seen) = extract_household_pairs(cid, &records);
                if incomplete {
                    snapshot.incomplete_households.insert(cid.clone());
                }
                if withdrawn_seen {
                    snapshot.withdrawn_households.insert(cid.clone());
                }
                snapshot.pairs.extend(pairs);
            }
            Err(e) => {
                // Per-collective tolerance: one unreachable/failed read does not
                // abandon the others (a household whose steward is offline must
                // not block backfill for the rest of the node's households). But
                // it IS an incomplete read — mark it so key-supersede abstains
                // rather than act on a partial member set. (With zero pairs the
                // household won't even reach the reconcile's per-household loop;
                // marking it is belt-and-suspenders and future-proof.)
                snapshot.incomplete_households.insert(cid.clone());
                tracing::warn!(
                    collective_cid = %cid,
                    error = %e,
                    "household replayer: membership read failed; skipping this household (marked incomplete)"
                );
            }
        }
    }

    tracing::info!(
        households = cids.len(),
        pairs = snapshot.pairs.len(),
        incomplete = snapshot.incomplete_households.len(),
        "household replayer: snapshot complete"
    );
    Ok(snapshot)
}

/// Test-only fixtures shared with sibling services (`identity_fill`) that need
/// to build fake conductor `Record`s without a live conductor.
#[cfg(test)]
pub(crate) mod test_support {
    use holochain_types::prelude::{
        Action, ActionData, ActionHash, ActionHeader, AgentPubKey, AppEntryBytes, AppEntryDef,
        CreateData, Entry, EntryHash, EntryType, EntryVisibility, Record, RecordEntry,
        SerializedBytes, Signature, SignedActionHashed, Timestamp, UnsafeBytes,
    };

    /// Build a `Record` carrying a `MembershipWire`-shaped App entry, so the
    /// pure extractor can be exercised without a live conductor. The entry bytes
    /// are encoded with the SAME `rmp_serde::to_vec_named` the conductor uses.
    pub(crate) fn membership_record(
        member_cid: &str,
        member_kind: &str,
        collective_cid: &str,
        withdrawn: Option<u64>,
    ) -> Record {
        #[derive(serde::Serialize)]
        struct WireOut<'a> {
            member_cid: &'a str,
            member_kind: &'a str,
            collective_cid: &'a str,
            withdrawn_at_block_height: Option<u64>,
        }
        let body = rmp_serde::to_vec_named(&WireOut {
            member_cid,
            member_kind,
            collective_cid,
            withdrawn_at_block_height: withdrawn,
        })
        .expect("encode membership wire");
        let sb = SerializedBytes::from(UnsafeBytes::from(body));
        let entry = Entry::App(AppEntryBytes::try_from(sb).expect("app entry bytes"));

        // A minimal Create action is enough — the extractor only reads the entry.
        // `new_unchecked` hashes the Action itself (no signature check), so the
        // placeholder prev_action/entry_hash/signature never need to be valid.
        //
        // Holochain 0.7 split `Action` into a common `header` plus per-variant
        // `data`: the old flat `Action::Create(Create { author, timestamp,
        // action_seq, prev_action, entry_type, entry_hash, weight })` is now
        // `ActionHeader { author, timestamp, action_seq, prev_action }` +
        // `ActionData::Create(CreateData { entry_type, entry_hash })`.
        // `prev_action` became `Option<ActionHash>` (None only for genesis) and
        // the rate-limiting `weight` field is gone.
        let action = Action {
            header: ActionHeader {
                author: AgentPubKey::from_raw_36(vec![0u8; 36]),
                timestamp: Timestamp::now(),
                action_seq: 1,
                prev_action: Some(ActionHash::from_raw_36(vec![3u8; 36])),
            },
            data: ActionData::Create(CreateData {
                entry_type: EntryType::App(AppEntryDef {
                    entry_index: 0.into(),
                    zome_index: 0.into(),
                    visibility: EntryVisibility::Public,
                }),
                entry_hash: EntryHash::from_raw_36(vec![1u8; 36]),
            }),
        };
        let signed = SignedActionHashed::new_unchecked(action, Signature([0u8; 64]));
        // 0.7: `Record::new` takes a `RecordEntry`, not an `Option<Entry>`.
        Record::new(signed, RecordEntry::Present(entry))
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::membership_record;
    use super::*;
    use holochain_types::prelude::{AppEntryBytes, RecordEntry};

    #[test]
    fn extract_emits_person_pairs_with_caller_household_cid() {
        let cid = "collective:uhCkkHousehold1";
        let records = vec![
            membership_record("agent:uhCAkAlice", "Person", cid, None),
            membership_record("agent:uhCAkBob", "Person", cid, None),
        ];
        let (pairs, incomplete, withdrawn) = extract_household_pairs(cid, &records);
        assert!(!incomplete, "all records decoded → complete read");
        assert!(!withdrawn, "no departures → no withdrawn flag");
        assert_eq!(
            pairs,
            vec![
                ("agent:uhCAkAlice".to_string(), cid.to_string()),
                ("agent:uhCAkBob".to_string(), cid.to_string()),
            ]
        );
    }

    #[test]
    fn extract_skips_non_person_members() {
        let cid = "collective:uhCkkHousehold1";
        let records = vec![
            membership_record("collective:uhCkkSub", "Collective", cid, None),
            membership_record("agent:uhCAkAgent", "ElohimAgent", cid, None),
            membership_record("agent:uhCAkPerson", "Person", cid, None),
        ];
        let (pairs, incomplete, withdrawn) = extract_household_pairs(cid, &records);
        assert!(
            !incomplete,
            "non-Person exclusions are correct, not incompleteness"
        );
        assert!(!withdrawn, "no withdrawn Person → no withdrawn flag");
        assert_eq!(
            pairs,
            vec![("agent:uhCAkPerson".to_string(), cid.to_string())]
        );
    }

    #[test]
    fn extract_skips_withdrawn_members() {
        let cid = "collective:uhCkkHousehold1";
        let records = vec![
            membership_record("agent:uhCAkGone", "Person", cid, Some(42)),
            membership_record("agent:uhCAkStays", "Person", cid, None),
        ];
        let (pairs, incomplete, withdrawn) = extract_household_pairs(cid, &records);
        assert!(
            !incomplete,
            "withdrawn exclusions are correct, not incompleteness"
        );
        // …but a withdrawn PERSON DOES flag the household: the departed member's
        // `humans` row lingers as an orphan fossil with no live-key partner, so
        // the key-supersede reconcile must abstain (else a withdrawal + a new join
        // collapses the forced bijection into a mis-attributing false 1:1).
        assert!(
            withdrawn,
            "a withdrawn Person member must flag the household for supersede abstention"
        );
        assert_eq!(
            pairs,
            vec![("agent:uhCAkStays".to_string(), cid.to_string())]
        );
    }

    #[test]
    fn extract_withdrawn_non_person_does_not_flag() {
        // A withdrawn non-Person (a sub-collective / elohim-agent) has no `humans`
        // row and so never becomes an orphan fossil — it must NOT flag the household.
        let cid = "collective:uhCkkHousehold1";
        let records = vec![
            membership_record("collective:uhCkkSub", "Collective", cid, Some(9)),
            membership_record("agent:uhCAkStays", "Person", cid, None),
        ];
        let (pairs, incomplete, withdrawn) = extract_household_pairs(cid, &records);
        assert!(!incomplete);
        assert!(
            !withdrawn,
            "a withdrawn non-Person member is not an orphan-fossil risk"
        );
        assert_eq!(
            pairs,
            vec![("agent:uhCAkStays".to_string(), cid.to_string())]
        );
    }

    #[test]
    fn extract_uses_caller_cid_not_entry_cid() {
        // Guard: even if the entry's own collective_cid drifts, the emitted
        // household_cid is the caller's (the cid we queried) — matching the
        // controller, which stamps the collective_cid it dispatched on.
        let queried = "collective:uhCkkCorrect";
        let records = vec![membership_record(
            "agent:uhCAkAlice",
            "Person",
            "collective:uhCkkDriftedInEntry",
            None,
        )];
        let (pairs, incomplete, withdrawn) = extract_household_pairs(queried, &records);
        assert!(!incomplete);
        assert!(!withdrawn);
        assert_eq!(
            pairs,
            vec![("agent:uhCAkAlice".to_string(), queried.to_string())]
        );
    }

    #[test]
    fn extract_skips_undecodable_record_keeps_rest() {
        let cid = "collective:uhCkkHousehold1";
        // A record whose entry is NOT a valid MembershipWire (raw garbage bytes).
        let bad_sb = holochain_types::prelude::SerializedBytes::from(
            holochain_types::prelude::UnsafeBytes::from(vec![0xff, 0xff, 0xff]),
        );
        let bad_entry = Entry::App(AppEntryBytes::try_from(bad_sb).expect("bytes"));
        let mut bad = membership_record("agent:ignored", "Person", cid, None);
        bad.entry = RecordEntry::Present(bad_entry);

        let good = membership_record("agent:uhCAkGood", "Person", cid, None);
        let (pairs, incomplete, withdrawn) = extract_household_pairs(cid, &[bad, good]);
        // The good row still comes through (per-row degradation)…
        assert_eq!(
            pairs,
            vec![("agent:uhCAkGood".to_string(), cid.to_string())]
        );
        // …but the household is flagged INCOMPLETE: the undecodable row could have
        // been a real Person member, so key-supersede must abstain for it.
        assert!(
            incomplete,
            "an undecodable membership record must mark the household read incomplete"
        );
        assert!(!withdrawn, "an undecodable record is not a withdrawal");
    }

    /// The cross-boundary msgpack shape, pinned by a test rather than by luck.
    ///
    /// `ConductorMembershipReader::list_memberships` decodes the conductor's
    /// response with `rmp_serde::from_slice::<Vec<Record>>`. That decode is the
    /// single most upgrade-fragile line in this module: `Record` carries a
    /// `SignedActionHashed`, whose `Action` was restructured in Holochain 0.7
    /// (flat variants → `header` + `data`), and whose `holo_hash` fields are raw
    /// bytes on the wire. This test round-trips a `holochain_types` 0.7 `Record`
    /// through the SAME encoder the conductor uses and the SAME decoder the
    /// reader uses, then runs the result through the real extractor — so a
    /// future client-family bump that silently changes the wire shape fails
    /// here instead of on a live mesh.
    ///
    /// It also guards the rule this module's decode comment states: going
    /// through the typed `Record` deserializer (never a `serde_json::Value`
    /// pre-pass) is what keeps the `holo_hash` fields intact.
    #[test]
    fn vec_record_survives_msgpack_roundtrip_through_the_reader_decode_path() {
        let cid = "collective:uhCkkHousehold1";
        let original = vec![
            membership_record("agent:uhCAkAlice", "Person", cid, None),
            membership_record("agent:uhCAkBob", "Person", cid, Some(42)),
        ];

        // Encode exactly as the conductor does…
        let bytes = rmp_serde::to_vec_named(&original).expect("encode Vec<Record>");
        // …and decode exactly as `ConductorMembershipReader` does.
        let decoded: Vec<Record> =
            rmp_serde::from_slice(&bytes).expect("Vec<Record> must decode from conductor msgpack");

        assert_eq!(decoded.len(), 2, "both records survive the round trip");

        // The action header survived — these are the fields 0.7 moved.
        let header = &decoded[0].action().header;
        assert_eq!(header.action_seq, 1);
        assert!(
            header.prev_action.is_some(),
            "prev_action must round-trip as Some for a non-genesis action"
        );
        // The holo_hash-typed fields survived as hashes, not as mangled bytes.
        assert_eq!(
            decoded[0].action().author(),
            original[0].action().author(),
            "AgentPubKey must survive the msgpack boundary intact"
        );
        assert_eq!(
            decoded[0].action_address(),
            original[0].action_address(),
            "the action hash must be stable across the round trip"
        );

        // And the entry still decodes through the real extractor: Alice is a
        // current member, Bob withdrew.
        let (pairs, incomplete, withdrawn) = extract_household_pairs(cid, &decoded);
        assert!(!incomplete, "a clean round trip is a complete read");
        assert!(withdrawn, "Bob's withdrawal must survive the round trip");
        assert_eq!(
            pairs,
            vec![("agent:uhCAkAlice".to_string(), cid.to_string())]
        );
    }

    // -----------------------------------------------------------------------
    // Orchestration over a typed mock reader (no live conductor)
    // -----------------------------------------------------------------------

    struct MockReader {
        by_cid: std::collections::HashMap<String, Result<Vec<Record>, ()>>,
    }

    #[async_trait]
    impl MembershipReader for MockReader {
        async fn list_memberships(
            &self,
            collective_cid: &str,
        ) -> Result<Vec<Record>, StorageError> {
            match self.by_cid.get(collective_cid) {
                Some(Ok(records)) => Ok(records.clone()),
                Some(Err(())) => Err(StorageError::Conductor("mock unreachable".into())),
                None => Ok(vec![]),
            }
        }
    }

    fn seed_household_collective(pool: &DbPool, ctx: &AppContext, id: &str, cid: &str) {
        use crate::db::collectives::{create_collective, CreateCollectiveInput};
        let mut conn = pool.get().expect("conn");
        let mut input = CreateCollectiveInput::stub(id);
        input.governance_layer = governance_layers::FAMILY.to_string();
        let collective = create_collective(&mut conn, ctx, &input).expect("create collective");
        // Stamp the collective_cid the replayer queries on.
        use crate::db::diesel_schema::collectives;
        use diesel::prelude::*;
        diesel::update(collectives::table.filter(collectives::id.eq(&collective.id)))
            .set(collectives::collective_cid.eq(Some(cid)))
            .execute(&mut conn)
            .expect("stamp cid");
    }

    #[tokio::test]
    async fn snapshot_collects_pairs_across_households() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        seed_household_collective(&pool, &ctx, "family-dowell", "collective:uhCkkDowell");
        seed_household_collective(&pool, &ctx, "family-smith", "collective:uhCkkSmith");

        let mut by_cid = std::collections::HashMap::new();
        by_cid.insert(
            "collective:uhCkkDowell".to_string(),
            Ok(vec![
                membership_record("agent:uhCAkAdam", "Person", "collective:uhCkkDowell", None),
                membership_record("agent:uhCAkEve", "Person", "collective:uhCkkDowell", None),
            ]),
        );
        by_cid.insert(
            "collective:uhCkkSmith".to_string(),
            Ok(vec![membership_record(
                "agent:uhCAkJohn",
                "Person",
                "collective:uhCkkSmith",
                None,
            )]),
        );
        let reader = MockReader { by_cid };

        let snap = snapshot_household_ids(&pool, &ctx, &reader)
            .await
            .expect("snapshot");
        assert!(
            snap.incomplete_households.is_empty(),
            "all reads succeeded and decoded → no incomplete households"
        );
        let mut pairs = snap.pairs;
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                (
                    "agent:uhCAkAdam".to_string(),
                    "collective:uhCkkDowell".to_string()
                ),
                (
                    "agent:uhCAkEve".to_string(),
                    "collective:uhCkkDowell".to_string()
                ),
                (
                    "agent:uhCAkJohn".to_string(),
                    "collective:uhCkkSmith".to_string()
                ),
            ]
        );
    }

    #[tokio::test]
    async fn snapshot_tolerates_unreachable_household() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        seed_household_collective(&pool, &ctx, "family-up", "collective:uhCkkUp");
        seed_household_collective(&pool, &ctx, "family-down", "collective:uhCkkDown");

        let mut by_cid = std::collections::HashMap::new();
        by_cid.insert(
            "collective:uhCkkUp".to_string(),
            Ok(vec![membership_record(
                "agent:uhCAkAlive",
                "Person",
                "collective:uhCkkUp",
                None,
            )]),
        );
        by_cid.insert("collective:uhCkkDown".to_string(), Err(()));
        let reader = MockReader { by_cid };

        let snap = snapshot_household_ids(&pool, &ctx, &reader)
            .await
            .expect("snapshot must not fail when one household is unreachable");
        assert_eq!(
            snap.pairs,
            vec![(
                "agent:uhCAkAlive".to_string(),
                "collective:uhCkkUp".to_string()
            )]
        );
        // The unreachable household's read failed → marked incomplete so
        // key-supersede would abstain for it (even though it contributes no pairs).
        assert!(
            snap.incomplete_households.contains("collective:uhCkkDown"),
            "a failed household read must be marked incomplete"
        );
        assert!(!snap.incomplete_households.contains("collective:uhCkkUp"));
    }

    #[tokio::test]
    async fn snapshot_flags_household_with_withdrawn_member() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        seed_household_collective(&pool, &ctx, "family-left", "collective:uhCkkLeft");
        seed_household_collective(&pool, &ctx, "family-stable", "collective:uhCkkStable");

        let mut by_cid = std::collections::HashMap::new();
        // uhCkkLeft has a withdrawn Person + a genuinely-new active member.
        by_cid.insert(
            "collective:uhCkkLeft".to_string(),
            Ok(vec![
                membership_record("agent:uhCAkGone", "Person", "collective:uhCkkLeft", Some(7)),
                membership_record("agent:uhCAkNew", "Person", "collective:uhCkkLeft", None),
            ]),
        );
        by_cid.insert(
            "collective:uhCkkStable".to_string(),
            Ok(vec![membership_record(
                "agent:uhCAkAlice",
                "Person",
                "collective:uhCkkStable",
                None,
            )]),
        );
        let reader = MockReader { by_cid };

        let snap = snapshot_household_ids(&pool, &ctx, &reader)
            .await
            .expect("snapshot");
        // The withdrawn member contributes no pair; the new member does.
        let mut pairs = snap.pairs.clone();
        pairs.sort();
        assert_eq!(
            pairs,
            vec![
                (
                    "agent:uhCAkAlice".to_string(),
                    "collective:uhCkkStable".to_string()
                ),
                (
                    "agent:uhCAkNew".to_string(),
                    "collective:uhCkkLeft".to_string()
                ),
            ]
        );
        // The household with a departure is flagged for supersede abstention; the
        // stable household is not. Neither is an incomplete read.
        assert!(snap.incomplete_households.is_empty());
        assert!(
            snap.withdrawn_households.contains("collective:uhCkkLeft"),
            "a withdrawn Person must flag its household for supersede abstention"
        );
        assert!(!snap.withdrawn_households.contains("collective:uhCkkStable"));
    }

    #[tokio::test]
    async fn snapshot_empty_when_no_household_collectives() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let reader = MockReader {
            by_cid: std::collections::HashMap::new(),
        };
        let snap = snapshot_household_ids(&pool, &ctx, &reader)
            .await
            .expect("snapshot");
        assert!(snap.pairs.is_empty());
        assert!(snap.incomplete_households.is_empty());
    }
}
