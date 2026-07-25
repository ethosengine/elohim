//! The **commitment-backed replication** folds — the "who has actually promised
//! to hold this?" lens over the notarized `replicates-*` commitment relation.
//!
//! Pure (DB-free): the diesel rows live in elohim-storage and are mirrored onto
//! [`ReplicationCommitmentRow`] by the loader
//! (`elohim-storage::db::rea_commitments::load_replication_commitment_relation`)
//! before they reach these folds (the §11 add-a-lens recipe,
//! `elohim/elohim-facings/CLAUDE.md`).
//!
//! ## Why this fold exists
//!
//! The resilience card's commitment fields
//! ([`CommitmentBackedReplication`]) and the distribution views'
//! `replication_commitments` list were hard-coded stubs
//! (`CommitmentBackedReplication::default()` / `Vec::new()`). The data to compute
//! them honestly lands in `rea_commitments` once the mishpat→REA replication
//! bridge mirrors each notarized `replicates-*` commitment there; this module is
//! the pure half that turns those rows into the two wire shapes.
//!
//! ## Tier classification — the ONE place the vocabulary lives
//!
//! | commitment `action`                          | payload discriminator          | [`CommitmentTier`] |
//! |----------------------------------------------|--------------------------------|--------------------|
//! | `replicates-dwelling`                        | `provider_role: steward_mutual`| `Dwelling`         |
//! | `replicates-dwelling`                        | `provider_role: collective_steward` | `Collective`  |
//! | `replicates-content` / `replicates-commons`  | (either variant)               | `Commons`          |
//!
//! `provide` rows are deliberately NOT tier-classified: the `provide` row is the
//! `(provider, reach)`-collapsed *aggregate* of the very `replicates-content`
//! commitments this fold already counts per-commitment
//! (`db::rea_commitments::record_provide_from_content_commitment`). Counting both
//! would double-count one economic act. They stay in the loaded relation because
//! the relation is the honest commitment relation; the fold decides what a *tier*
//! is.
//!
//! ## Pledged bytes
//!
//! Read from the commitment's own payload envelope, mirroring
//! `elohim-storage::services::peer_capacity_service::aggregate_pledges_by_tier`:
//! `capacity_bytes` for a dwelling pledge, `commons_bytes` for a commons
//! capacity pledge. A `replicates-content` **content** commitment names an EPR,
//! not a byte budget — it pledges 0 bytes and is counted, not summed.
//!
//! ## Malformed payloads skip, never poison
//!
//! A row whose `metadata_json` is absent / not JSON / missing the discriminator
//! the tier depends on is **skipped** (measured absence), never defaulted into a
//! tier. One bad row must never zero — or inflate — the whole fold.

use std::collections::{HashMap, HashSet};

use elohim_views::infrastructure::{
    CommitmentBackedReplication, CommitmentTier, ReplicationCommitmentRef,
};
use elohim_views::replicates_dwelling::ProviderRole;

/// DB-free mirror of one commitment row for the replication lens.
///
/// Field provenance — the diesel source is `rea_commitments`:
/// - `commitment_cid` — `rea_commitments.id`, which for a bridged row IS the
///   mishpat commitment `entry_hash` (memory `project_mishpat_commitment_cid_is_entry_hash`:
///   the `action_hash` is only ever `dht_anchor_hash`).
/// - `action` — `provide` | `replicates-content` | `replicates-commons` |
///   `replicates-dwelling` (the loader's `eq_any` set).
/// - `provider` / `recipient` — `rea_commitments.provider` / `.receiver`. For a
///   commons **content** commitment the recipient is the EPR `head_ref` (the
///   per-content link); for a dwelling commitment it is the recipient hub id;
///   for a capacity pledge it is empty (no counterparty).
/// - `state` — `active` | `proposed` | `cancelled` | … The folds require `active`.
/// - `metadata_json` — the action-specific policy envelope as a raw JSON string,
///   kept unparsed by the loader (this fold owns the parse so the classification
///   vocabulary lives in ONE place).
/// - `household_id` — the provider's household (filled by the loader from the
///   `humans` join on `agent_pub_key = provider`). `None` when the provider is
///   not a known human — e.g. a dwelling commitment whose provider is a *hub* id
///   rather than an `agent_cid`.
#[derive(Debug, Clone)]
pub struct ReplicationCommitmentRow {
    pub commitment_cid: String,
    pub action: String,
    pub provider: String,
    pub recipient: String,
    pub state: String,
    pub metadata_json: Option<String>,
    pub household_id: Option<String>,
}

/// The one lifecycle state a commitment must be in to back a replication claim.
/// A `proposed`, `cancelled`, or `terminated` promise is not live coverage.
const ACTIVE_STATE: &str = "active";

/// The dwelling/collective replication action (tier decided by `provider_role`).
const DWELLING_ACTION: &str = "replicates-dwelling";

/// Commons-tier replication actions. `replicates-content` is the reach-general
/// action; `replicates-commons` is the migration-window alias the conductor
/// author still emits (see `mishpat_projection::parse_commitment_payload`).
const COMMONS_ACTIONS: [&str; 2] = ["replicates-content", "replicates-commons"];

/// A successfully classified commitment: its tier, the role that decided the
/// tier (dwelling only), and the bytes it pledges.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassifiedCommitment {
    pub tier: CommitmentTier,
    pub provider_role: Option<ProviderRole>,
    pub pledged_bytes: u64,
}

/// Read a `u64` from a payload envelope under either the snake_case key (the
/// DHT payload / bridged `bounds_json` form) or the camelCase key (the typed
/// `ReplicatesDwellingPayload` form other writers store). Neither present, or
/// present-but-not-a-number → `None`.
fn payload_u64(payload: &serde_json::Value, snake: &str, camel: &str) -> Option<u64> {
    payload
        .get(snake)
        .or_else(|| payload.get(camel))
        .and_then(|v| v.as_u64())
}

/// Read a `&str` from a payload envelope under either casing (see [`payload_u64`]).
fn payload_str<'a>(payload: &'a serde_json::Value, snake: &str, camel: &str) -> Option<&'a str> {
    payload
        .get(snake)
        .or_else(|| payload.get(camel))
        .and_then(|v| v.as_str())
}

/// Map a `provider_role` wire string onto the typed enum.
///
/// Accepts BOTH the DHT/serde snake_case form (`steward_mutual` — what the
/// mishpat integrity zome validates and what `ProviderRole`'s
/// `rename_all = "snake_case"` emits) and the bare Rust variant spelling
/// (`StewardMutual`) that a hand-written payload might carry. Anything else is
/// an unknown role: `None`, so the row is SKIPPED rather than defaulted into a
/// tier it never claimed.
fn parse_provider_role(raw: &str) -> Option<ProviderRole> {
    match raw {
        "steward_mutual" | "StewardMutual" => Some(ProviderRole::StewardMutual),
        "collective_steward" | "CollectiveSteward" => Some(ProviderRole::CollectiveSteward),
        _ => None,
    }
}

/// Classify ONE row into its tier + pledged bytes, or `None` when the row is not
/// a live, classifiable replication commitment.
///
/// Returns `None` for: a non-`active` state, a non-`replicates-*` action
/// (including the `provide` aggregate — see the module docs), a
/// `replicates-dwelling` row whose payload is missing/malformed/carries an
/// unknown `provider_role`.
///
/// A commons row with a missing or malformed payload still classifies (the
/// action alone decides the tier) — it simply pledges 0 bytes. That asymmetry is
/// deliberate: the dwelling tier is *decided by* the payload, the commons tier is
/// decided by the action.
pub fn classify(row: &ReplicationCommitmentRow) -> Option<ClassifiedCommitment> {
    if row.state != ACTIVE_STATE {
        return None;
    }

    // Skip-not-poison: an unparseable envelope reads as "no envelope", never as
    // an error that drops the whole fold.
    let payload: Option<serde_json::Value> = row
        .metadata_json
        .as_deref()
        .and_then(|m| serde_json::from_str::<serde_json::Value>(m).ok());

    if row.action == DWELLING_ACTION {
        let payload = payload?;
        let role = parse_provider_role(payload_str(&payload, "provider_role", "providerRole")?)?;
        let tier = match role {
            ProviderRole::StewardMutual => CommitmentTier::Dwelling,
            ProviderRole::CollectiveSteward => CommitmentTier::Collective,
        };
        return Some(ClassifiedCommitment {
            tier,
            provider_role: Some(role),
            pledged_bytes: payload_u64(&payload, "capacity_bytes", "capacityBytes").unwrap_or(0),
        });
    }

    if COMMONS_ACTIONS.contains(&row.action.as_str()) {
        // Only the `capacity` variant carries a byte budget; the `content`
        // variant names an EPR head_ref and pledges no bytes.
        let pledged_bytes = payload
            .as_ref()
            .and_then(|p| payload_u64(p, "commons_bytes", "commonsBytes"))
            .unwrap_or(0);
        return Some(ClassifiedCommitment {
            tier: CommitmentTier::Commons,
            provider_role: None,
            pledged_bytes,
        });
    }

    None
}

/// **Intent.** The per-tier counts + total pledged bytes for a materialized
/// commitment relation — the [`CommitmentBackedReplication`] wire shape the
/// resilience card renders.
///
/// Counts COMMITMENTS (one per notarized `replicates-*` entry), not distinct
/// providers or households: two dwelling pledges from the same hub to two
/// different recipients are two commitments. (Distinct-household counting is a
/// different question, answered by `folds::rea::commitment_backed`.)
///
/// Byte sums use `saturating_add` — a corrupt oversized pledge saturates rather
/// than wrapping to a small number.
pub fn commitment_backed_replication(
    rows: &[ReplicationCommitmentRow],
) -> CommitmentBackedReplication {
    let mut out = CommitmentBackedReplication::default();
    for row in rows {
        let Some(c) = classify(row) else { continue };
        match c.tier {
            CommitmentTier::Dwelling => out.dwelling_commitments += 1,
            CommitmentTier::Collective => out.collective_commitments += 1,
            CommitmentTier::Commons => out.commons_commitments += 1,
        }
        out.total_pledged_bytes = out.total_pledged_bytes.saturating_add(c.pledged_bytes);
    }
    out
}

/// As [`commitment_backed_replication`], but scoped to the commitments PROVIDED
/// by agents in `households`.
///
/// This is the scoping the resilience card wants: `CommitmentBackedReplication`
/// is documented (elohim-views) as "commitment-backed replication counts for a
/// CID's **authoring household**" — so the relevant relation is "the commitments
/// the households stewarding this content have made", not the whole node's
/// ledger.
///
/// A row whose `household_id` is `None` (provider is not a known human — e.g. a
/// hub-id provider on a dwelling commitment) can NOT be attributed to a
/// household and is excluded. Measured absence, never an attributed guess.
pub fn commitment_backed_replication_for_households(
    rows: &[ReplicationCommitmentRow],
    households: &HashSet<String>,
) -> CommitmentBackedReplication {
    if households.is_empty() {
        return CommitmentBackedReplication::default();
    }
    let scoped: Vec<ReplicationCommitmentRow> = rows
        .iter()
        .filter(|r| {
            r.household_id
                .as_deref()
                .is_some_and(|h| households.contains(h))
        })
        .cloned()
        .collect();
    commitment_backed_replication(&scoped)
}

/// Project the classifiable rows into the [`ReplicationCommitmentRef`] wire
/// shape, sorted by `commitment_cid` so the list is deterministic on the wire
/// regardless of row-load order.
pub fn commitment_refs(rows: &[ReplicationCommitmentRow]) -> Vec<ReplicationCommitmentRef> {
    let mut refs: Vec<ReplicationCommitmentRef> = rows
        .iter()
        .filter_map(|row| {
            let c = classify(row)?;
            Some(ReplicationCommitmentRef {
                commitment_cid: row.commitment_cid.clone(),
                tier: c.tier,
                provider_cid: row.provider.clone(),
                recipient_cid: row.recipient.clone(),
                provider_role: c.provider_role,
            })
        })
        .collect();
    refs.sort_by(|a, b| a.commitment_cid.cmp(&b.commitment_cid));
    refs
}

/// The per-content slice of [`commitment_refs`]: only commitments whose
/// `recipient` IS this CID.
///
/// `ReplicationCommitmentRef` is documented (elohim-views) as "one active
/// `replicates-*` commitment whose recipient + scope cover this CID", so the
/// per-content views MUST scope. The exact link exists for the commons
/// **content** variant, whose recipient is the EPR `head_ref`
/// (`mishpat_projection::parse_replicates_commons` → `recipient: head_ref`).
///
/// A dwelling pledge or a commons *capacity* pledge names a hub / no
/// counterparty — it covers a scope, not a named CID, and this fold does NOT
/// evaluate `scope_filter` per-CID. Such a commitment is therefore never claimed
/// to cover a specific CID here. An empty result is a **measured absence** (no
/// commitment names this CID), never a fabricated global list — a per-content
/// view that listed every commitment on the node would be actively misleading.
pub fn commitment_refs_covering(
    rows: &[ReplicationCommitmentRow],
    cid: &str,
) -> Vec<ReplicationCommitmentRef> {
    let one: HashSet<String> = [cid.to_string()].into_iter().collect();
    commitment_refs_covering_any(rows, &one)
}

/// As [`commitment_refs_covering`], but over a SET of equivalent addresses for
/// the same content.
///
/// A distribution view is keyed by a blob address while a content commitment
/// names an EPR `head_ref` — two namespaces for the same artifact. The caller
/// resolves the equivalence it can prove (e.g. blob hash → the `content.id`s
/// stored under that blob) and passes every address; a commitment naming ANY of
/// them covers this content. Addresses the caller cannot prove equivalent are
/// simply absent from the set — the fold never widens the claim on its own.
pub fn commitment_refs_covering_any(
    rows: &[ReplicationCommitmentRow],
    cids: &HashSet<String>,
) -> Vec<ReplicationCommitmentRef> {
    if cids.is_empty() {
        return Vec::new();
    }
    let covering: Vec<ReplicationCommitmentRow> = rows
        .iter()
        .filter(|r| cids.contains(&r.recipient))
        .cloned()
        .collect();
    commitment_refs(&covering)
}

/// Count of classifiable commitments per tier — the deterministic map form, for
/// callers that want the histogram rather than the four-field wire struct.
/// (Kept `BTreeMap`-free: [`CommitmentTier`] is not `Ord`; a small `HashMap`
/// keyed by the tier's wire spelling is the deterministic-enough shape.)
pub fn counts_by_tier(rows: &[ReplicationCommitmentRow]) -> HashMap<&'static str, usize> {
    let mut out: HashMap<&'static str, usize> = HashMap::new();
    for row in rows {
        let Some(c) = classify(row) else { continue };
        let key = match c.tier {
            CommitmentTier::Dwelling => "dwelling",
            CommitmentTier::Collective => "collective",
            CommitmentTier::Commons => "commons",
        };
        *out.entry(key).or_insert(0) += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(
        cid: &str,
        action: &str,
        state: &str,
        metadata_json: Option<&str>,
    ) -> ReplicationCommitmentRow {
        ReplicationCommitmentRow {
            commitment_cid: cid.to_string(),
            action: action.to_string(),
            provider: "uhCAkprovider".to_string(),
            recipient: "uhCAkrecipient".to_string(),
            state: state.to_string(),
            metadata_json: metadata_json.map(|m| m.to_string()),
            household_id: None,
        }
    }

    /// A `replicates-dwelling` envelope in the DHT/bridge (snake_case) form.
    fn dwelling_meta(role: &str, capacity: u64) -> String {
        format!(
            r#"{{"action":"replicates-dwelling","provider_role":"{role}","capacity_bytes":{capacity}}}"#
        )
    }

    /// A `replicates-commons` **capacity** envelope (snake_case).
    fn commons_capacity_meta(bytes: u64) -> String {
        format!(r#"{{"variant":"capacity","commons_bytes":{bytes}}}"#)
    }

    /// A `replicates-commons` **content** envelope (snake_case) — names an EPR,
    /// pledges no bytes.
    fn commons_content_meta() -> String {
        r#"{"variant":"content","epr_scope":["epr:head"],"reach":"commons"}"#.to_string()
    }

    // ── classify: provider_role → tier ────────────────────────────────────────

    #[test]
    fn classify_steward_mutual_is_dwelling_tier() {
        let r = row(
            "c1",
            "replicates-dwelling",
            "active",
            Some(&dwelling_meta("steward_mutual", 1_000)),
        );
        let c = classify(&r).expect("steward_mutual classifies");
        assert_eq!(c.tier, CommitmentTier::Dwelling);
        assert_eq!(c.provider_role, Some(ProviderRole::StewardMutual));
        assert_eq!(c.pledged_bytes, 1_000);
    }

    #[test]
    fn classify_collective_steward_is_collective_tier() {
        let r = row(
            "c1",
            "replicates-dwelling",
            "active",
            Some(&dwelling_meta("collective_steward", 2_500)),
        );
        let c = classify(&r).expect("collective_steward classifies");
        assert_eq!(c.tier, CommitmentTier::Collective);
        assert_eq!(c.provider_role, Some(ProviderRole::CollectiveSteward));
        assert_eq!(c.pledged_bytes, 2_500);
    }

    #[test]
    fn classify_accepts_camel_case_payload_keys() {
        // The typed `ReplicatesDwellingPayload` writer stores camelCase.
        let meta = r#"{"providerRole":"steward_mutual","capacityBytes":77}"#;
        let c = classify(&row("c1", "replicates-dwelling", "active", Some(meta)))
            .expect("camelCase envelope classifies");
        assert_eq!(c.tier, CommitmentTier::Dwelling);
        assert_eq!(c.pledged_bytes, 77);
    }

    #[test]
    fn classify_commons_actions_are_commons_tier() {
        for action in ["replicates-content", "replicates-commons"] {
            let c = classify(&row("c1", action, "active", Some(&commons_content_meta())))
                .unwrap_or_else(|| panic!("{action} classifies"));
            assert_eq!(c.tier, CommitmentTier::Commons, "action {action}");
            assert_eq!(c.provider_role, None, "commons carries no provider_role");
            assert_eq!(c.pledged_bytes, 0, "content variant pledges no bytes");
        }
    }

    #[test]
    fn classify_commons_capacity_reads_commons_bytes() {
        let c = classify(&row(
            "c1",
            "replicates-commons",
            "active",
            Some(&commons_capacity_meta(50_000_000_000)),
        ))
        .expect("capacity variant classifies");
        assert_eq!(c.tier, CommitmentTier::Commons);
        assert_eq!(c.pledged_bytes, 50_000_000_000);
    }

    // ── classify: the skip cases ──────────────────────────────────────────────

    #[test]
    fn classify_skips_non_active_state() {
        for state in ["proposed", "cancelled", "terminated"] {
            assert!(
                classify(&row(
                    "c1",
                    "replicates-dwelling",
                    state,
                    Some(&dwelling_meta("steward_mutual", 10))
                ))
                .is_none(),
                "state {state} must not back a replication claim"
            );
        }
    }

    #[test]
    fn classify_skips_provide_and_unrelated_actions() {
        // `provide` is the (provider, reach)-collapsed aggregate of the very
        // replicates-content commitments already counted — counting it would
        // double-count one economic act.
        for action in [
            "provide",
            "custody-blob",
            "delegates-compute",
            "project-epr",
        ] {
            assert!(
                classify(&row("c1", action, "active", Some(&commons_content_meta()))).is_none(),
                "action {action} is not a replication tier"
            );
        }
    }

    #[test]
    fn classify_skips_malformed_metadata_without_poisoning() {
        // Not JSON at all / missing provider_role / unknown provider_role.
        for meta in [
            Some("{not json at all"),
            Some(r#"{"capacity_bytes":10}"#),
            Some(r#"{"provider_role":"totally-bogus","capacity_bytes":10}"#),
            None,
        ] {
            assert!(
                classify(&row("c1", "replicates-dwelling", "active", meta)).is_none(),
                "dwelling row with envelope {meta:?} must skip, not default into a tier"
            );
        }
    }

    #[test]
    fn classify_commons_survives_malformed_metadata_with_zero_bytes() {
        // The commons tier is decided by the ACTION, so a bad envelope still
        // classifies — it just pledges nothing.
        let c = classify(&row(
            "c1",
            "replicates-content",
            "active",
            Some("{not json"),
        ))
        .expect("commons tier comes from the action, not the envelope");
        assert_eq!(c.tier, CommitmentTier::Commons);
        assert_eq!(c.pledged_bytes, 0);
    }

    // ── commitment_backed_replication ─────────────────────────────────────────

    #[test]
    fn commitment_backed_replication_counts_per_tier_and_sums_pledged_bytes() {
        let rows = vec![
            row(
                "c1",
                "replicates-dwelling",
                "active",
                Some(&dwelling_meta("steward_mutual", 100)),
            ),
            row(
                "c2",
                "replicates-dwelling",
                "active",
                Some(&dwelling_meta("steward_mutual", 200)),
            ),
            row(
                "c3",
                "replicates-dwelling",
                "active",
                Some(&dwelling_meta("collective_steward", 300)),
            ),
            row(
                "c4",
                "replicates-commons",
                "active",
                Some(&commons_capacity_meta(400)),
            ),
            row(
                "c5",
                "replicates-content",
                "active",
                Some(&commons_content_meta()),
            ),
            // skipped: not active
            row(
                "c6",
                "replicates-dwelling",
                "proposed",
                Some(&dwelling_meta("steward_mutual", 999_999)),
            ),
            // skipped: malformed dwelling envelope
            row("c7", "replicates-dwelling", "active", Some("{bad")),
            // skipped: provide aggregate
            row("c8", "provide", "active", None),
        ];
        let out = commitment_backed_replication(&rows);
        assert_eq!(out.dwelling_commitments, 2);
        assert_eq!(out.collective_commitments, 1);
        assert_eq!(out.commons_commitments, 2);
        assert_eq!(
            out.total_pledged_bytes, 1_000,
            "100 + 200 + 300 + 400 + 0 (content pledges no bytes); skipped rows contribute nothing"
        );
    }

    #[test]
    fn commitment_backed_replication_empty_is_all_zero() {
        let out = commitment_backed_replication(&[]);
        assert_eq!(out.dwelling_commitments, 0);
        assert_eq!(out.collective_commitments, 0);
        assert_eq!(out.commons_commitments, 0);
        assert_eq!(out.total_pledged_bytes, 0);
    }

    #[test]
    fn commitment_backed_replication_saturates_rather_than_wrapping() {
        let rows = vec![
            row(
                "c1",
                "replicates-commons",
                "active",
                Some(&commons_capacity_meta(u64::MAX)),
            ),
            row(
                "c2",
                "replicates-commons",
                "active",
                Some(&commons_capacity_meta(10)),
            ),
        ];
        assert_eq!(
            commitment_backed_replication(&rows).total_pledged_bytes,
            u64::MAX,
            "a corrupt oversized pledge saturates, never wraps to a small number"
        );
    }

    // ── household scoping ─────────────────────────────────────────────────────

    #[test]
    fn commitment_backed_replication_for_households_scopes_to_provider_households() {
        let mut in_house = row(
            "c1",
            "replicates-dwelling",
            "active",
            Some(&dwelling_meta("steward_mutual", 100)),
        );
        in_house.household_id = Some("house-a".to_string());
        let mut other_house = row(
            "c2",
            "replicates-dwelling",
            "active",
            Some(&dwelling_meta("steward_mutual", 500)),
        );
        other_house.household_id = Some("house-z".to_string());
        // No household attribution possible (hub-id provider) → excluded.
        let unattributed = row(
            "c3",
            "replicates-commons",
            "active",
            Some(&commons_capacity_meta(700)),
        );

        let households: HashSet<String> = ["house-a".to_string()].into_iter().collect();
        let out = commitment_backed_replication_for_households(
            &[in_house, other_house, unattributed],
            &households,
        );
        assert_eq!(out.dwelling_commitments, 1);
        assert_eq!(out.commons_commitments, 0);
        assert_eq!(out.total_pledged_bytes, 100);
    }

    #[test]
    fn commitment_backed_replication_for_households_empty_scope_is_zero() {
        let mut r = row(
            "c1",
            "replicates-dwelling",
            "active",
            Some(&dwelling_meta("steward_mutual", 100)),
        );
        r.household_id = Some("house-a".to_string());
        let out = commitment_backed_replication_for_households(&[r], &HashSet::new());
        assert_eq!(out.dwelling_commitments, 0);
        assert_eq!(out.total_pledged_bytes, 0);
    }

    // ── commitment_refs ───────────────────────────────────────────────────────

    #[test]
    fn commitment_refs_projects_classifiable_rows_sorted_by_cid() {
        let rows = vec![
            row(
                "c-zeta",
                "replicates-dwelling",
                "active",
                Some(&dwelling_meta("collective_steward", 1)),
            ),
            row(
                "c-alpha",
                "replicates-content",
                "active",
                Some(&commons_content_meta()),
            ),
            // skipped — no tier
            row("c-mid", "provide", "active", None),
        ];
        let refs = commitment_refs(&rows);
        let cids: Vec<&str> = refs.iter().map(|r| r.commitment_cid.as_str()).collect();
        assert_eq!(cids, vec!["c-alpha", "c-zeta"], "deterministic cid order");
        assert_eq!(refs[0].tier, CommitmentTier::Commons);
        assert_eq!(refs[0].provider_role, None);
        assert_eq!(refs[1].tier, CommitmentTier::Collective);
        assert_eq!(
            refs[1].provider_role,
            Some(ProviderRole::CollectiveSteward),
            "the role that decided the tier rides along on the ref"
        );
    }

    #[test]
    fn commitment_refs_covering_scopes_to_the_named_cid() {
        let mut names_cid = row(
            "c1",
            "replicates-content",
            "active",
            Some(&commons_content_meta()),
        );
        names_cid.recipient = "epr:target".to_string();
        let mut names_other = row(
            "c2",
            "replicates-content",
            "active",
            Some(&commons_content_meta()),
        );
        names_other.recipient = "epr:elsewhere".to_string();
        // A scope-general pledge (no counterparty) never claims to cover a CID.
        let mut capacity_pledge = row(
            "c3",
            "replicates-commons",
            "active",
            Some(&commons_capacity_meta(9)),
        );
        capacity_pledge.recipient = String::new();

        let refs =
            commitment_refs_covering(&[names_cid, names_other, capacity_pledge], "epr:target");
        assert_eq!(
            refs.len(),
            1,
            "only the commitment naming this CID covers it"
        );
        assert_eq!(refs[0].commitment_cid, "c1");
    }

    #[test]
    fn commitment_refs_covering_any_matches_either_equivalent_address() {
        let mut by_head = row(
            "c1",
            "replicates-content",
            "active",
            Some(&commons_content_meta()),
        );
        by_head.recipient = "epr:head".to_string();
        let mut by_blob = row(
            "c2",
            "replicates-content",
            "active",
            Some(&commons_content_meta()),
        );
        by_blob.recipient = "sha256-abc".to_string();
        let mut unrelated = row(
            "c3",
            "replicates-content",
            "active",
            Some(&commons_content_meta()),
        );
        unrelated.recipient = "epr:other".to_string();

        let addresses: HashSet<String> = ["epr:head".to_string(), "sha256-abc".to_string()]
            .into_iter()
            .collect();
        let refs = commitment_refs_covering_any(&[by_head, by_blob, unrelated], &addresses);
        let cids: Vec<&str> = refs.iter().map(|r| r.commitment_cid.as_str()).collect();
        assert_eq!(cids, vec!["c1", "c2"], "either proven address covers");
    }

    #[test]
    fn commitment_refs_covering_any_empty_address_set_is_empty() {
        let rows = vec![row(
            "c1",
            "replicates-content",
            "active",
            Some(&commons_content_meta()),
        )];
        assert!(
            commitment_refs_covering_any(&rows, &HashSet::new()).is_empty(),
            "no proven address → no claim"
        );
    }

    #[test]
    fn commitment_refs_covering_unknown_cid_is_measured_absence() {
        let rows = vec![row(
            "c1",
            "replicates-content",
            "active",
            Some(&commons_content_meta()),
        )];
        assert!(
            commitment_refs_covering(&rows, "epr:nobody-named-this").is_empty(),
            "no commitment names this CID → empty, never a fabricated global list"
        );
    }

    // ── counts_by_tier ────────────────────────────────────────────────────────

    #[test]
    fn counts_by_tier_buckets_each_tier() {
        let rows = vec![
            row(
                "c1",
                "replicates-dwelling",
                "active",
                Some(&dwelling_meta("steward_mutual", 1)),
            ),
            row(
                "c2",
                "replicates-dwelling",
                "active",
                Some(&dwelling_meta("collective_steward", 1)),
            ),
            row(
                "c3",
                "replicates-content",
                "active",
                Some(&commons_content_meta()),
            ),
            row(
                "c4",
                "replicates-commons",
                "active",
                Some(&commons_capacity_meta(1)),
            ),
        ];
        let counts = counts_by_tier(&rows);
        assert_eq!(counts.get("dwelling"), Some(&1));
        assert_eq!(counts.get("collective"), Some(&1));
        assert_eq!(counts.get("commons"), Some(&2));
    }

    #[test]
    fn counts_by_tier_empty_is_empty() {
        assert!(counts_by_tier(&[]).is_empty());
    }
}
