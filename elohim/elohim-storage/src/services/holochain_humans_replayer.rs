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
            .call_zome(IMAGODEI_ZOME, LIST_MEMBERSHIPS_FN, payload)
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
/// `(member_cid, household_cid)` pairs.
///
/// - Only `MemberKind::Person` members are emitted (collectives-of-collectives
///   and elohim-agent members never carry a household_id).
/// - Withdrawn members (`withdrawn_at_block_height` set) are dropped.
/// - `household_cid` is the caller-supplied `collective_cid` — the exact value
///   the controller stamps — NOT re-derived from the entry, so a malformed
///   entry collective_cid cannot drift the stamp.
/// - A record whose entry is absent or fails to decode is skipped with a warn;
///   one bad row never poisons the rest (per-row degradation, fail-open).
fn extract_household_pairs(collective_cid: &str, records: &[Record]) -> Vec<(String, String)> {
    let mut pairs = Vec::new();
    for record in records {
        let Some(Entry::App(eb)) = record.entry.as_option() else {
            // Membership entries are always App entries with a body; an absent
            // entry (e.g. a delete) is not a current membership.
            continue;
        };
        let membership: MembershipWire = match rmp_serde::from_slice(eb.bytes()) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!(
                    collective_cid = %collective_cid,
                    error = %e,
                    "household replayer: skipping membership record that failed to decode"
                );
                continue;
            }
        };
        if membership.member_kind != MEMBER_KIND_PERSON {
            continue;
        }
        if membership.withdrawn_at_block_height.is_some() {
            continue;
        }
        pairs.push((membership.member_cid, collective_cid.to_string()));
    }
    pairs
}

/// Source the set of household collective cids from the local `collectives`
/// projection (governance_layer == family, collective_cid present). The
/// projection is the read-optimised cache of the collectives DHT truth; it is
/// the only per-conductor index of "which households exist on this node".
fn household_collective_cids(pool: &DbPool, ctx: &AppContext) -> Result<Vec<String>, StorageError> {
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

/// Boot-time snapshot of `(member_cid, household_cid)` pairs from the current
/// DHT household memberships, for the `household_backfill` startup pass.
///
/// Tolerant of DHT unavailability: a missing/unreachable conductor or a
/// malformed record degrades to a smaller mapping and a warning, never an error
/// that fails startup. An empty mapping is a valid, idempotent-safe outcome.
///
/// `ctx` scopes the household-collective projection lookup to the right app
/// (`humans`/`collectives` are h_app_id-scoped). `reader` is the conductor seam.
pub async fn snapshot_household_ids(
    pool: &DbPool,
    ctx: &AppContext,
    reader: &dyn MembershipReader,
) -> Result<Vec<(String, String)>, StorageError> {
    let cids = match household_collective_cids(pool, ctx) {
        Ok(cids) => cids,
        Err(e) => {
            // Projection read failed — degrade to empty rather than fail boot.
            tracing::warn!(
                error = %e,
                "household replayer: could not read household collectives from projection; empty snapshot"
            );
            return Ok(vec![]);
        }
    };

    if cids.is_empty() {
        tracing::debug!(
            "household replayer: no household collectives in projection; empty snapshot"
        );
        return Ok(vec![]);
    }

    let mut mapping: Vec<(String, String)> = Vec::new();
    for cid in &cids {
        match reader.list_memberships(cid).await {
            Ok(records) => {
                mapping.extend(extract_household_pairs(cid, &records));
            }
            Err(e) => {
                // Per-collective tolerance: one unreachable/failed read does not
                // abandon the others (a household whose steward is offline must
                // not block backfill for the rest of the node's households).
                tracing::warn!(
                    collective_cid = %cid,
                    error = %e,
                    "household replayer: membership read failed; skipping this household"
                );
            }
        }
    }

    tracing::info!(
        households = cids.len(),
        pairs = mapping.len(),
        "household replayer: snapshot complete"
    );
    Ok(mapping)
}

#[cfg(test)]
mod tests {
    use super::*;
    use holochain_types::prelude::{
        Action, ActionHash, AgentPubKey, AppEntryBytes, AppEntryDef, Create, EntryHash, EntryType,
        EntryVisibility, RecordEntry, SerializedBytes, Signature, SignedActionHashed, Timestamp,
        UnsafeBytes,
    };

    /// Build a `Record` carrying a `MembershipWire`-shaped App entry, so the
    /// pure extractor can be exercised without a live conductor. The entry bytes
    /// are encoded with the SAME `rmp_serde::to_vec_named` the conductor uses.
    fn membership_record(
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
        let action = Action::Create(Create {
            author: AgentPubKey::from_raw_36(vec![0u8; 36]),
            timestamp: Timestamp::now(),
            action_seq: 1,
            prev_action: ActionHash::from_raw_36(vec![3u8; 36]),
            entry_type: EntryType::App(AppEntryDef {
                entry_index: 0.into(),
                zome_index: 0.into(),
                visibility: EntryVisibility::Public,
            }),
            entry_hash: EntryHash::from_raw_36(vec![1u8; 36]),
            weight: Default::default(),
        });
        let signed = SignedActionHashed::new_unchecked(action, Signature([0u8; 64]));
        Record::new(signed, Some(entry))
    }

    #[test]
    fn extract_emits_person_pairs_with_caller_household_cid() {
        let cid = "collective:uhCkkHousehold1";
        let records = vec![
            membership_record("agent:uhCAkAlice", "Person", cid, None),
            membership_record("agent:uhCAkBob", "Person", cid, None),
        ];
        let pairs = extract_household_pairs(cid, &records);
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
        let pairs = extract_household_pairs(cid, &records);
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
        let pairs = extract_household_pairs(cid, &records);
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
        let pairs = extract_household_pairs(queried, &records);
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
        let pairs = extract_household_pairs(cid, &[bad, good]);
        assert_eq!(
            pairs,
            vec![("agent:uhCAkGood".to_string(), cid.to_string())]
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

        let mut pairs = snapshot_household_ids(&pool, &ctx, &reader)
            .await
            .expect("snapshot");
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

        let pairs = snapshot_household_ids(&pool, &ctx, &reader)
            .await
            .expect("snapshot must not fail when one household is unreachable");
        assert_eq!(
            pairs,
            vec![(
                "agent:uhCAkAlive".to_string(),
                "collective:uhCkkUp".to_string()
            )]
        );
    }

    #[tokio::test]
    async fn snapshot_empty_when_no_household_collectives() {
        let pool = crate::test_util::test_pool();
        let ctx = AppContext::default_lamad();
        let reader = MockReader {
            by_cid: std::collections::HashMap::new(),
        };
        let pairs = snapshot_household_ids(&pool, &ctx, &reader)
            .await
            .expect("snapshot");
        assert!(pairs.is_empty());
    }
}
