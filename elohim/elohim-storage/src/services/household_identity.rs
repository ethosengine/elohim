//! Household identity resolution — the CONVERGENT grouping key for the
//! household-resilience fold.
//!
//! # The divergence this closes (2026-09-12 doorway-footprint-convergence)
//!
//! `a2o/features/resilience/doorway-footprint-convergence.feature` measured two
//! doorways answering DIFFERENT holder footprints for one commons EPR from
//! identical DHT facts. Measured on the household mesh:
//!
//! | peer    | row holding `collective:uhCkkD5Z9…` | `household-dowell` row |
//! |---------|-------------------------------------|------------------------|
//! | matthew | `id=family-dowell`                  | cid = NULL             |
//! | jessica | `id=collective:uhCkkD5Z9…`          | cid = NULL             |
//! | james   | `id=collective:uhCkkD5Z9…`          | cid = NULL             |
//!
//! matthew folded `{household-dowell}` = 1 stewarding collective; jessica and
//! james folded `{household-dowell, collective:uhCkkD5Z9…}` = 2 — from the same
//! custody facts.
//!
//! The root cause is a CONTRACT GAP between two layers that key on different
//! columns:
//!
//! - `p2p::projection_reconcile`'s collectives arm converges the DHT
//!   `collective_cid` and says so explicitly — `classify_collective_gap` returns
//!   `InSync` when `local_cids.contains(cid)`, "regardless of which routing alias
//!   it sits under". Every peer therefore reports
//!   `elohim_projection_reconcile_divergent{stream="collectives"} 0`.
//! - the resilience fold grouped by `collectives.id` — the very per-host routing
//!   alias the reconciler declares out of scope.
//!
//! So the reconciler converged the cid while the fold keyed on the alias. This
//! module moves the fold onto the column the substrate actually converges.
//!
//! # The rules (operator-set, 2026-09-12)
//!
//! 1. **Grouping key is the DHT `collective_cid`.** A slug anchored to a cid
//!    lifts to that cid; a cid-form value is already the key; an UN-ANCHORED slug
//!    stays its own bucket (two un-anchored households never merge into one).
//! 2. **The PRESENTED id stays the human-facing slug** whenever a local row
//!    anchors a slug to that cid, falling back to the cid form only when no slug
//!    is anchored anywhere. Convergence of the presented id is what the a2o judge
//!    compares (`kind:id`), and a raw `collective:uhCkk…` in a "names, not nines"
//!    surface would be a felt-surface regression.
//! 3. **Label resolves by cid first, slug fallback.**
//! 4. **Alias expansion is bidirectional.** Sibling joins key on the LOCAL
//!    household id (`stewarded_nodes.household_id`,
//!    `peer_statuses ⋈ stewarded_nodes`). Feeding them a cid while they store a
//!    slug would collapse `online_peers.live`/`.known` to 0 and flip every verdict
//!    to `at-risk`. [`HouseholdIdentity::aliases_of`] expands a group key back
//!    into EVERY local id that folds onto it, so those joins keep matching.
//!
//! # Why this is substrate-floor and not policy
//!
//! Every input is a local projection of DHT truth (`collectives.collective_cid`
//! is stamped only from a conductor-verified `Collective` entry). The resolver
//! adds no judgment — it is a deterministic relabelling that makes two peers
//! holding the same DHT facts produce the same fold, which is exactly the
//! substrate's promise.

use std::collections::{BTreeSet, HashMap};

use diesel::prelude::*;

use crate::error::StorageError;

/// The DHT-canonical collective-cid prefix. A local routing alias (a seed slug
/// like `household-dowell`, `family-eden`) is never cid-shaped.
pub const COLLECTIVE_CID_PREFIX: &str = "collective:";

/// True when `id` is the DHT-canonical `collective:{action_hash}` form rather
/// than a local routing alias.
pub fn is_cid_form(id: &str) -> bool {
    id.starts_with(COLLECTIVE_CID_PREFIX)
}

/// Local household-identity resolver: the cid↔slug vocabulary bridge the fold
/// and its sibling joins share.
///
/// Built from LOCAL tables only — the peer still serves its own verified truth,
/// now keyed on the column the substrate converges rather than on a per-host
/// routing alias.
#[derive(Debug, Default, Clone)]
pub struct HouseholdIdentity {
    /// `collectives.id` → `collective_cid`, anchored rows only.
    cid_by_id: HashMap<String, String>,
    /// `collective_cid` → every local `collectives.id` anchored to it. Ordered
    /// so every derived choice is deterministic across peers and across runs.
    ids_by_cid: HashMap<String, BTreeSet<String>>,
    /// `collectives.id` → region (the fault-domain axis the regional fold reads).
    region_by_id: HashMap<String, Option<String>>,
    /// `collectives.id` → display name.
    name_by_id: HashMap<String, String>,
    /// Distinct non-NULL `humans.household_id` values present locally — the
    /// household vocabulary the node's OWN member rows actually speak. This is
    /// what makes the presented id converge: the slug is seed data, identical on
    /// every peer, whereas which `collectives` row got anchored is not.
    member_household_ids: BTreeSet<String>,
}

impl HouseholdIdentity {
    /// Materialize the resolver from the local projections.
    ///
    /// The `collectives` read is deliberately `h_app_id`-AGNOSTIC, mirroring the
    /// holder-relation's left join: a scope filter here would ship a silent
    /// no-op under the qahal/lamad ctx drift class (the same reasoning the prior
    /// `load_collective_cid_alias_map` recorded).
    pub fn load(conn: &mut SqliteConnection) -> Result<Self, StorageError> {
        use crate::db::diesel_schema::{collectives, humans};

        let rows: Vec<(String, Option<String>, Option<String>, String)> = collectives::table
            .select((
                collectives::id,
                collectives::collective_cid,
                collectives::region,
                collectives::name,
            ))
            .load(conn)
            .map_err(|e| StorageError::Internal(format!("household identity: collectives: {e}")))?;

        let mut this = Self::default();
        for (id, cid, region, name) in rows {
            this.region_by_id.insert(id.clone(), region);
            this.name_by_id.insert(id.clone(), name);
            if let Some(cid) = cid.filter(|c| !c.trim().is_empty()) {
                this.cid_by_id.insert(id.clone(), cid.clone());
                this.ids_by_cid.entry(cid).or_default().insert(id);
            }
        }

        let member_ids: Vec<Option<String>> = humans::table
            .filter(humans::household_id.is_not_null())
            .select(humans::household_id)
            .distinct()
            .load(conn)
            .map_err(|e| StorageError::Internal(format!("household identity: humans: {e}")))?;
        this.member_household_ids = member_ids
            .into_iter()
            .flatten()
            .filter(|h| !h.trim().is_empty())
            .collect();

        Ok(this)
    }

    /// The CONVERGENT grouping key for one `humans.household_id` value.
    ///
    /// An anchored slug lifts to its cid; a cid-form value (with or without a
    /// local row) is already the key; an un-anchored slug stays itself, so two
    /// un-anchored households never merge.
    pub fn group_key(&self, household_id: &str) -> String {
        self.cid_by_id
            .get(household_id)
            .cloned()
            .unwrap_or_else(|| household_id.to_string())
    }

    /// As [`Self::group_key`], threaded through an `Option` for the nullable
    /// `humans.household_id` column.
    pub fn group_key_opt(&self, household_id: Option<String>) -> Option<String> {
        household_id.map(|h| self.group_key(&h))
    }

    /// The id PRESENTED on the wire for a group key (rule 2).
    ///
    /// Preference order, each step deterministic:
    /// 1. the lexicographically-least non-cid `humans.household_id` that folds
    ///    onto this key — the household vocabulary this node's own members speak,
    ///    which is seed data and therefore identical across peers;
    /// 2. the lexicographically-least non-cid local `collectives.id` anchored to
    ///    this cid;
    /// 3. the key itself (a bare cid, or an un-anchored slug).
    pub fn display_id(&self, key: &str) -> String {
        if let Some(slug) = self
            .member_household_ids
            .iter()
            .find(|h| !is_cid_form(h) && self.group_key(h) == key)
        {
            return slug.clone();
        }
        if let Some(ids) = self.ids_by_cid.get(key) {
            if let Some(slug) = ids.iter().find(|id| !is_cid_form(id)) {
                return slug.clone();
            }
        }
        key.to_string()
    }

    /// Display label for a group key — resolved by cid first, slug fallback
    /// (rule 3). Returns `None` when no local row names this household.
    pub fn label(&self, key: &str) -> Option<String> {
        if let Some(ids) = self.ids_by_cid.get(key) {
            for id in ids {
                if let Some(name) = self.name_by_id.get(id) {
                    return Some(name.clone());
                }
            }
        }
        self.name_by_id.get(key).cloned()
    }

    /// Region for a group key — the first non-NULL region among the rows that
    /// fold onto it, in the same deterministic order [`Self::display_id`] uses.
    pub fn region_for(&self, key: &str) -> Option<String> {
        let display = self.display_id(key);
        if let Some(region) = self.region_by_id.get(&display).cloned().flatten() {
            return Some(region);
        }
        if let Some(ids) = self.ids_by_cid.get(key) {
            for id in ids {
                if let Some(region) = self.region_by_id.get(id).cloned().flatten() {
                    return Some(region);
                }
            }
        }
        self.region_by_id.get(key).cloned().flatten()
    }

    /// EVERY local household id that folds onto `key` (rule 4).
    ///
    /// Sibling joins (`stewarded_nodes.household_id`, `peer_statuses ⋈
    /// stewarded_nodes`) store whichever local id their writer saw. Querying them
    /// with a bare group key would silently return zero rows on a peer that
    /// stores the other vocabulary — `online_peers.live`/`.known` collapse to 0
    /// and every verdict flips to `at-risk`. Expanding both directions keeps the
    /// same physical set addressable.
    pub fn aliases_of(&self, key: &str) -> Vec<String> {
        let mut out: BTreeSet<String> = BTreeSet::new();
        out.insert(key.to_string());
        if let Some(ids) = self.ids_by_cid.get(key) {
            out.extend(ids.iter().cloned());
        }
        for h in &self.member_household_ids {
            if self.group_key(h) == key {
                out.insert(h.clone());
            }
        }
        out.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a resolver directly, bypassing the DB — the pure core is what the
    /// rules live in.
    fn fixture(
        anchored: &[(&str, &str)],
        unanchored: &[&str],
        members: &[&str],
        regions: &[(&str, &str)],
        names: &[(&str, &str)],
    ) -> HouseholdIdentity {
        let mut id = HouseholdIdentity::default();
        for (local_id, cid) in anchored {
            id.cid_by_id.insert((*local_id).into(), (*cid).into());
            id.ids_by_cid
                .entry((*cid).into())
                .or_default()
                .insert((*local_id).into());
            id.region_by_id.insert((*local_id).into(), None);
            id.name_by_id.insert((*local_id).into(), (*local_id).into());
        }
        for local_id in unanchored {
            id.region_by_id.insert((*local_id).into(), None);
            id.name_by_id.insert((*local_id).into(), (*local_id).into());
        }
        for (local_id, region) in regions {
            id.region_by_id
                .insert((*local_id).into(), Some((*region).into()));
        }
        for (local_id, name) in names {
            id.name_by_id.insert((*local_id).into(), (*name).into());
        }
        id.member_household_ids = members.iter().map(|m| (*m).to_string()).collect();
        id
    }

    const CID: &str = "collective:uhCkkDowell";

    #[test]
    fn anchored_slug_lifts_to_the_cid() {
        let id = fixture(
            &[("household-dowell", CID)],
            &[],
            &["household-dowell"],
            &[],
            &[],
        );
        assert_eq!(id.group_key("household-dowell"), CID);
    }

    #[test]
    fn cid_form_household_is_already_the_key() {
        let id = fixture(&[], &[], &[], &[], &[]);
        assert_eq!(id.group_key(CID), CID);
    }

    #[test]
    fn unanchored_slugs_stay_distinct_buckets() {
        let id = fixture(&[], &["a", "b"], &["a", "b"], &[], &[]);
        assert_eq!(id.group_key("a"), "a");
        assert_eq!(id.group_key("b"), "b");
        assert_ne!(id.group_key("a"), id.group_key("b"));
    }

    /// The live-mesh shape: matthew anchored the cid onto `family-dowell` while
    /// jessica minted a cid-keyed placeholder. Both must PRESENT the slug their
    /// member rows speak.
    #[test]
    fn presented_id_converges_across_divergent_anchor_inventories() {
        let matthew = fixture(
            &[("family-dowell", CID), ("household-dowell", CID)],
            &[],
            &["household-dowell"],
            &[],
            &[("family-dowell", "Dowell Family")],
        );
        let jessica = fixture(
            &[(CID, CID), ("household-dowell", CID)],
            &[],
            &["household-dowell", CID],
            &[],
            &[(CID, "Dowell Family")],
        );
        assert_eq!(matthew.display_id(CID), "household-dowell");
        assert_eq!(jessica.display_id(CID), "household-dowell");
        assert_eq!(matthew.display_id(CID), jessica.display_id(CID));
    }

    #[test]
    fn presented_id_falls_back_to_the_cid_when_no_slug_is_anchored() {
        let id = fixture(&[(CID, CID)], &[], &[CID], &[], &[]);
        assert_eq!(id.display_id(CID), CID);
    }

    #[test]
    fn label_resolves_by_cid_first_then_slug() {
        let by_cid = fixture(
            &[("family-dowell", CID)],
            &[],
            &[],
            &[],
            &[("family-dowell", "Dowell Family")],
        );
        assert_eq!(by_cid.label(CID).as_deref(), Some("Dowell Family"));

        let by_slug = fixture(
            &[],
            &["household-dowell"],
            &["household-dowell"],
            &[],
            &[("household-dowell", "Dowell Household")],
        );
        assert_eq!(
            by_slug.label("household-dowell").as_deref(),
            Some("Dowell Household")
        );
        assert_eq!(by_slug.label(CID), None);
    }

    /// Rule 4 — the cascade guard. A group key must expand back into every local
    /// id the sibling joins might be storing, in BOTH directions.
    #[test]
    fn aliases_expand_cid_to_every_local_vocabulary() {
        let id = fixture(
            &[(CID, CID), ("household-dowell", CID)],
            &[],
            &["household-dowell", CID],
            &[],
            &[],
        );
        let aliases = id.aliases_of(CID);
        assert!(aliases.contains(&CID.to_string()), "{aliases:?}");
        assert!(
            aliases.contains(&"household-dowell".to_string()),
            "{aliases:?}"
        );
    }

    #[test]
    fn aliases_of_an_unanchored_slug_is_just_itself() {
        let id = fixture(&[], &["solo"], &["solo"], &[], &[]);
        assert_eq!(id.aliases_of("solo"), vec!["solo".to_string()]);
    }

    #[test]
    fn region_backfills_from_the_anchored_row() {
        let id = fixture(
            &[(CID, CID), ("household-dowell", CID)],
            &[],
            &["household-dowell"],
            &[("household-dowell", "tech-valley")],
            &[],
        );
        assert_eq!(id.region_for(CID).as_deref(), Some("tech-valley"));
    }

    #[test]
    fn cid_form_detection() {
        assert!(is_cid_form("collective:uhCkkAbc"));
        assert!(!is_cid_form("household-dowell"));
        assert!(!is_cid_form("family-eden"));
    }
}
