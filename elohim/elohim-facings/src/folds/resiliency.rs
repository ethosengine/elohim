//! The resiliency facing's folds — select→fold→aggregate over the holder-relation.
//!
//! Pure (DB-free) by construction: every fn here takes a materialized
//! `&[HolderRow]` (or plain view types) and returns a view/aggregate. The impure
//! loaders (`load_holder_relation`, the manifest/region queries) stay in
//! `elohim-storage::services::household_resilience`, which calls these folds as a
//! thin adapter. A new lens is a new fold over the SAME relation, not a new query.
//!
//! Design: `genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md`
//! §3 (the holder-relation) + §11 (the Lens Framework / migration slice).

use std::collections::{BTreeMap, HashSet};

use elohim_views::{
    FaultDomainDiversity, FeltFloorView, FeltStatusView, PlacementGapView,
    RegionalDistributionView, StewardingCollectiveEntry,
};

use crate::fold::{bucket_by, distinct_count_by};
use crate::relation::HolderRow;

/// Fold: distinct non-null hubs stewarding the content (the inter-hub count).
/// The `compute()` steward collect — `relation.iter().filter_map(hub_id)`;
/// `None` hubs are excluded, preserving prior behavior.
pub fn stewarding_hubs(relation: &[HolderRow]) -> HashSet<String> {
    relation.iter().filter_map(|r| r.hub_id.clone()).collect()
}

/// Fold the holder-relation into distinct intra-hub peer counts: for each hub
/// (collective), how many distinct agents hold the content (the operator's
/// "james ↔ matthew ↔ jessica = 3" intra-hub resiliency). Pure; the composability
/// demonstration — a new lens is a new fold over the SAME relation.
///
/// Composed from the generic combinators (the §11 composability proof): bucket by
/// hub (dropping null hubs), then distinct-count agents per bucket. Identical
/// output to the prior hand loop, no bespoke iteration.
///
/// Returns a `BTreeMap` (deterministic key order) — consumers may iterate it into a
/// serialized `Vec` without leaking hash order onto the wire.
pub fn intra_hub_peers(relation: &[HolderRow]) -> BTreeMap<String, i32> {
    bucket_by(relation, |r| r.hub_id.clone())
        .into_iter()
        .map(|(hub, rows)| (hub, distinct_count_by(&rows, |r| r.agent_id.clone()) as i32))
        .collect()
}

/// Fold: regional distribution of distinct hubs relative to the viewer's region.
/// Dedupe by hub so two peers in the same hub count once; a hub with no region
/// (or a viewer with no region) buckets `unknown` — preserving prior behavior.
/// The impure part (manifest → shard_hashes → `load_holder_relation` → viewer
/// region query) stays in storage; this is the pure bucket loop.
pub fn regional_distribution(
    relation: &[HolderRow],
    viewer_region: Option<&str>,
) -> RegionalDistributionView {
    let mut seen: HashSet<Option<String>> = HashSet::new();
    let mut dist = RegionalDistributionView {
        local: 0,
        regional: 0,
        global: 0,
        unknown: 0,
    };
    for r in relation {
        if !seen.insert(r.hub_id.clone()) {
            continue;
        }
        match (viewer_region, r.region.as_deref()) {
            (None, None) => dist.unknown += 1,
            (None, Some(_)) => dist.global += 1,
            (Some(_), None) => dist.unknown += 1,
            (Some(vr), Some(sr)) if vr == sr => dist.local += 1,
            (Some(_), Some(_)) => dist.regional += 1,
        }
    }
    dist
}

/// The RS(4,3) baseline fault-domain target (7 = data + parity shards) — the
/// denominator that normalizes a distinct fault-domain count into a [0,1]
/// diversity score. Owner-declarable per content is deferred (the resilience-tier
/// primitive, genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md);
/// until then this is the baseline. Deliberately NOT derived from reach.
pub const FAULT_DOMAIN_TARGET: u32 = 7;

/// Normalize a distinct fault-domain count into a [0,1] diversity score against
/// the RS baseline target. Pure. Both the relational snapshot (household fault
/// domains) and the graph branch (steward-collective fault domains) key on this
/// single definition, so the denominator cannot drift between the two paths.
///
/// This replaces the prior ad-hoc coverage proxy
/// (`min(stewarding, max(commitment_backed, 1)) / 7`), which falsely capped the
/// score by the count of commitment records — penalizing content that is
/// genuinely spread across many household fault domains but lacks commitment
/// paperwork. The truthful measure is the count of independent fault domains
/// actually holding the content.
pub fn diversity_score(distinct_fault_domains: u32) -> f32 {
    if FAULT_DOMAIN_TARGET == 0 {
        return 0.0;
    }
    (distinct_fault_domains as f32 / FAULT_DOMAIN_TARGET as f32).clamp(0.0, 1.0)
}

/// Core fault-domain diversity fold: distinct household + region fault domains
/// over an iterator of `(household, region)` optionals. Pure. Both the
/// holder-relation path (`fault_domain_diversity`) and the replica-peer path
/// (`elohim_storage::services::distribution_view::compute_fault_domain_diversity`)
/// delegate here, so the fault-domain definition lives in ONE place (no
/// duplicated household/region counting across the two facings).
///
/// `distinct_collective_count` stays 0: neither the holder-relation nor the
/// replica-peer row carries a collective binding distinct from the household.
pub fn fault_domain_diversity_from<'a>(
    domains: impl Iterator<Item = (Option<&'a str>, Option<&'a str>)>,
) -> FaultDomainDiversity {
    let mut households: HashSet<&str> = HashSet::new();
    let mut regions: HashSet<&str> = HashSet::new();
    for (household, region) in domains {
        if let Some(h) = household {
            households.insert(h);
        }
        if let Some(r) = region {
            regions.insert(r);
        }
    }
    let distinct_household_count = households.len() as u32;
    let distinct_region_count = regions.len() as u32;
    FaultDomainDiversity {
        distinct_household_count,
        distinct_collective_count: 0,
        distinct_region_count,
        // Risk when all known holders share a single household fault domain.
        single_fault_domain_risk: distinct_household_count <= 1,
        fault_modes_evaluated: vec!["household".to_string(), "region".to_string()],
    }
}

/// Fold the holder-relation into a `FaultDomainDiversity` — distinct household
/// (`hub_id`) and region fault domains that hold the content. Pure. The scalar
/// diversity score (`diversity_score`) keys on the household fault-domain count
/// (the file's primary failure domain, matching `single_fault_domain_risk`); the
/// region axis is surfaced for the distribution detail view.
pub fn fault_domain_diversity(relation: &[HolderRow]) -> FaultDomainDiversity {
    fault_domain_diversity_from(
        relation
            .iter()
            .map(|r| (r.hub_id.as_deref(), r.region.as_deref())),
    )
}

/// Resilience FLOOR (households wanted) for a tier — the content-relative
/// denominator. Value-driven and owner-declarable; the declared-tier primitive
/// (genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md)
/// will set the tier per content. Until then the tier is "standard" (the legacy
/// `≥3 households` protected bar). Deliberately NOT derived from reach.
pub fn floor_for_tier(tier: &str) -> i32 {
    match tier {
        "vault" => 5,
        "keepsake" => 4,
        "ephemeral" => 1,
        // "standard" and any undeclared / unknown tier
        _ => 3,
    }
}

/// Build the household-addressed felt projection. Pure (no DB) so the honesty
/// rules are exhaustively unit-testable.
///
/// Honest by construction:
/// - **Unmeasured-aware**: `distribution_state == "unmeasured"` (never entered
///   the distribution plane) → `"not-yet-seen"`, never a fake verdict.
/// - **Floor-relative**: "protected" requires meeting the tier's floor
///   (`has >= wants`), so a vault-floor content held only at the standard bar
///   reads `"watching"`, never `"protected"` (no false reassurance); an
///   ephemeral content at its floor of 1 reads `"protected"` (no false alarm).
pub fn build_felt_status(
    distribution_state: &str,
    placement_gaps: &[PlacementGapView],
    held_by: Vec<StewardingCollectiveEntry>,
    tier: &str,
    tier_declared: bool,
) -> FeltStatusView {
    let has_households = held_by.len() as i32;
    let wants_households = floor_for_tier(tier);
    let has_gap = !placement_gaps.is_empty();
    let meets_floor = has_households >= wants_households;

    let reassurance = if distribution_state == "unmeasured" {
        "not-yet-seen"
    } else if meets_floor && !has_gap {
        "protected"
    } else if !meets_floor && has_households <= 1 {
        "needs-help"
    } else {
        // meets the floor but a lapse is active, OR below the floor while still
        // holding (>1 household, gap or not) — keeping watch, not alarming.
        "watching"
    };

    let names: Vec<String> = held_by.iter().filter_map(|h| h.label.clone()).collect();
    let all_named = !held_by.is_empty() && names.len() as i32 == has_households;

    let headline = match reassurance {
        "not-yet-seen" => "We can't confirm these are backed up yet".to_string(),
        "needs-help" => {
            if has_households == 0 {
                "No household is holding these yet — invite one to help".to_string()
            } else {
                "Held by only 1 household — invite another to help hold these".to_string()
            }
        }
        "watching" => {
            if has_gap {
                format!("Still safe — {has_households} households are holding these")
            } else {
                format!(
                    "Held by {has_households} of the {wants_households} households this should live in"
                )
            }
        }
        // "protected"
        _ => {
            if all_named {
                format!("Held by {has_households} households: {}", names.join(", "))
            } else {
                format!("Held by {has_households} households")
            }
        }
    };

    let suggested_action = match reassurance {
        "needs-help" | "not-yet-seen" => Some("Invite a household to help hold these".to_string()),
        _ => None,
    };

    FeltStatusView {
        headline,
        reassurance: reassurance.to_string(),
        held_by,
        floor: FeltFloorView {
            tier: tier.to_string(),
            tier_declared,
            wants_households,
            has_households,
        },
        suggested_action,
    }
}

#[cfg(test)]
mod fold_tests {
    //! The moved/rewritten relation folds preserve prior semantics exactly: the
    //! null-hub drop, distinct-agent counting, hub dedupe, and region bucketing.
    use super::*;

    fn row(hub: Option<&str>, agent: &str, region: Option<&str>) -> HolderRow {
        HolderRow {
            hub_id: hub.map(str::to_string),
            agent_id: agent.to_string(),
            region: region.map(str::to_string),
        }
    }

    #[test]
    fn intra_hub_peers_distinct_agents_per_hub_drops_null() {
        // Mirrors the integration seed (home-multi=2 distinct agents, home-solo=1)
        // plus a duplicate agent (collapses) and a null hub (dropped). The
        // combinator rewrite must reproduce the prior hand loop exactly.
        let rel = vec![
            row(Some("home-multi"), "a", None),
            row(Some("home-multi"), "b", None),
            row(Some("home-multi"), "a", None), // duplicate agent — collapses
            row(Some("home-solo"), "c", None),
            row(None, "d", None), // null hub — dropped
        ];
        let counts = intra_hub_peers(&rel);
        assert_eq!(
            counts.get("home-multi"),
            Some(&2),
            "distinct agents per hub"
        );
        assert_eq!(counts.get("home-solo"), Some(&1));
        assert_eq!(counts.len(), 2, "null hub dropped, no phantom bucket");
    }

    #[test]
    fn stewarding_hubs_excludes_null_and_dedupes() {
        let rel = vec![
            row(Some("h1"), "a", None),
            row(Some("h1"), "b", None), // same hub
            row(None, "c", None),       // null hub excluded
        ];
        let hubs = stewarding_hubs(&rel);
        assert_eq!(hubs.len(), 1, "distinct non-null hubs");
        assert!(hubs.contains("h1"));
    }

    #[test]
    fn fault_domain_diversity_counts_distinct_households_and_regions() {
        // 3 distinct hubs across 2 distinct regions; duplicate agents/hub rows
        // collapse. A null-hub / null-region row contributes to neither set.
        let rel = vec![
            row(Some("h1"), "a", Some("us-east")),
            row(Some("h1"), "b", Some("us-east")), // same hub + region — collapses
            row(Some("h2"), "c", Some("us-west")),
            row(Some("h3"), "d", None), // null region — not counted
            row(None, "e", Some("eu")), // null hub — not counted as household
        ];
        let fdd = fault_domain_diversity(&rel);
        assert_eq!(fdd.distinct_household_count, 3, "h1,h2,h3");
        assert_eq!(fdd.distinct_region_count, 3, "us-east,us-west,eu");
        assert_eq!(fdd.distinct_collective_count, 0);
        assert!(!fdd.single_fault_domain_risk, "3 households > 1");
    }

    #[test]
    fn fault_domain_diversity_empty_relation_is_honest_zero() {
        let fdd = fault_domain_diversity(&[]);
        assert_eq!(fdd.distinct_household_count, 0);
        assert_eq!(fdd.distinct_region_count, 0);
        assert!(
            fdd.single_fault_domain_risk,
            "zero fault domains is a single-fault-domain risk (<=1)"
        );
    }

    #[test]
    fn diversity_score_is_fault_domains_over_target_clamped() {
        assert_eq!(diversity_score(0), 0.0, "no fault domains → 0");
        // 3 household fault domains / 7 baseline.
        assert!((diversity_score(3) - 3.0 / 7.0).abs() < 1e-6);
        // Beyond the target clamps at 1.0 (never > full).
        assert_eq!(diversity_score(FAULT_DOMAIN_TARGET + 1), 1.0);
        assert_eq!(diversity_score(100), 1.0);
    }

    #[test]
    fn regional_distribution_no_viewer_buckets_global_and_unknown() {
        // No viewer region: a hub with a known region → global; a hub with no
        // region → unknown. Dedupe by hub (two peers in h1 count once).
        let rel = vec![
            row(Some("h1"), "a", Some("us-east")),
            row(Some("h1"), "b", Some("us-east")), // same hub — deduped
            row(Some("h2"), "c", None),            // null region → unknown
        ];
        let d = regional_distribution(&rel, None);
        assert_eq!(d.global, 1, "h1 known region, viewer none → global");
        assert_eq!(d.unknown, 1, "h2 null region → unknown");
        assert_eq!(d.local + d.regional, 0);
    }

    #[test]
    fn regional_distribution_viewer_splits_local_vs_regional() {
        let rel = vec![
            row(Some("h1"), "a", Some("us-east")),
            row(Some("h2"), "b", Some("us-west")),
        ];
        let d = regional_distribution(&rel, Some("us-east"));
        assert_eq!(d.local, 1, "h1 matches viewer region → local");
        assert_eq!(d.regional, 1, "h2 differs → regional");
        assert_eq!(d.global + d.unknown, 0);
    }

    /// Slice-0 PROOF-GATE (Wave 1.1): a POPULATED holder-relation lights every
    /// resiliency fold at once — `stewarding_hubs` > 0, `regional_distribution`
    /// buckets non-zero, and `intra_hub_peers` distinct-agent counts. Proves the
    /// dark resilience card (`stewardingCollectives: 0`) is caused by an EMPTY
    /// `shard_locations` relation, NOT a broken fold layer — so the card-lighting
    /// keystone is "populate the relation" (Wave 1), not "fix the folds".
    #[test]
    fn populated_relation_lights_stewarding_regional_and_intra_hub() {
        // 3 distinct collectives across 2 regions; household-dowell holds the
        // content on 3 distinct agents (the matthew/jessica/james mesh).
        let rel = vec![
            row(Some("household-dowell"), "matthew", Some("us-west")),
            row(Some("household-dowell"), "jessica", Some("us-west")),
            row(Some("household-dowell"), "james", Some("us-west")),
            row(Some("household-eden"), "adam", Some("us-east")),
            row(Some("household-gertrude"), "gertrude", None),
        ];

        // stewardingCollectives > 0 — the dark-card metric is reachable.
        let hubs = stewarding_hubs(&rel);
        assert_eq!(hubs.len(), 3, "3 distinct stewarding collectives light");

        // intra-hub resiliency: the dowell mesh shows 3 distinct holders.
        let intra = intra_hub_peers(&rel);
        assert_eq!(
            intra.get("household-dowell"),
            Some(&3),
            "M/J/J = 3 distinct agents"
        );
        assert_eq!(intra.get("household-eden"), Some(&1));

        // regional distribution lights (not all-unknown) for a us-west viewer.
        let dist = regional_distribution(&rel, Some("us-west"));
        assert_eq!(dist.local, 1, "household-dowell in viewer region → local");
        assert_eq!(dist.regional, 1, "household-eden differs → regional");
        assert_eq!(dist.unknown, 1, "household-gertrude null region → unknown");
        // Internal consistency: every distinct hub lands in exactly one bucket.
        assert_eq!(
            dist.local + dist.regional + dist.global + dist.unknown,
            hubs.len() as i32,
        );
    }

    /// Slice-0 PROOF-GATE (Wave 1.1) — the HONEST-DARK companion to the
    /// populated gate above. Over the EMPTY holder-relation the live card
    /// actually sees (`shard_locations` unpopulated — the plan's corrections
    /// #1/#3: the card is dark because the table is empty, NOT because of a
    /// join-namespace bug), the folds return honest zero: no fabricated
    /// stewards, no phantom hub buckets, no theatrical region split. This pins
    /// the sprint plan's central thesis — the dark resilience card is a
    /// data-writer gap, never a lying fold — as an executable assertion, and
    /// proves the folds never invent distribution the relation does not contain
    /// (a declared trust surface owes "no fake verdict", including no fake
    /// non-zero).
    #[test]
    fn empty_relation_folds_honestly_dark_no_theater() {
        let empty: Vec<HolderRow> = vec![];

        // stewardingCollectives is honestly 0 — an honest "no observation",
        // the exact live dark-card condition, not a fabricated verdict.
        assert!(
            stewarding_hubs(&empty).is_empty(),
            "empty relation fabricates no stewards"
        );

        // no intra-hub peers invented.
        assert!(
            intra_hub_peers(&empty).is_empty(),
            "empty relation → no phantom hub buckets"
        );

        // every region bucket is zero, viewer present or absent — the card
        // shows "no region data" honestly, never a fabricated split.
        for viewer in [None, Some("us-west")] {
            let d = regional_distribution(&empty, viewer);
            assert_eq!(
                d.local + d.regional + d.global + d.unknown,
                0,
                "empty relation → all region buckets zero (viewer={viewer:?})"
            );
        }
    }

    /// Slice-0 PROOF-GATE (Wave 1.1) — honest UNDER-report boundary. A
    /// single-holder relation (one distinct hub, even with duplicate agent
    /// rows) lights `stewardingCollectives = 1` EXACTLY — never rounded up
    /// toward a tier floor — so the folds cannot over-claim protection the
    /// relation does not evidence. The complement of the populated (>1) gate:
    /// together they bracket the honesty boundary from both sides (0 stays 0,
    /// 1 stays 1, N counts distinctly).
    #[test]
    fn single_holder_relation_reports_exactly_one_no_overclaim() {
        let rel = vec![
            row(Some("household-dowell"), "matthew", Some("us-west")),
            // duplicate agent row — must collapse, not inflate the count.
            row(Some("household-dowell"), "matthew", Some("us-west")),
        ];

        let hubs = stewarding_hubs(&rel);
        assert_eq!(hubs.len(), 1, "one distinct hub, never inflated");

        assert_eq!(
            intra_hub_peers(&rel).get("household-dowell"),
            Some(&1),
            "one distinct agent, duplicate collapsed"
        );

        let d = regional_distribution(&rel, Some("us-west"));
        assert_eq!(d.local, 1, "the single in-region hub");
        assert_eq!(
            d.regional + d.global + d.unknown,
            0,
            "no bucket beyond the one real holder"
        );
    }
}

#[cfg(test)]
mod felt_status_tests {
    //! Exhaustive honesty rules for the household-addressed felt projection.
    //! Pure (no DB). These ARE the spec for the felt surface's reassurance state
    //! machine — the executable form of the grandma-vertical scenarios + the
    //! operator's content-declared resilience-floor note.
    use super::*;

    fn holder(id: &str, label: Option<&str>) -> StewardingCollectiveEntry {
        StewardingCollectiveEntry {
            id: id.to_string(),
            kind: "household".to_string(),
            label: label.map(str::to_string),
            intra_hub_peers: None,
        }
    }

    fn a_gap() -> PlacementGapView {
        PlacementGapView {
            id: "gap-1".to_string(),
            content_id: "c1".to_string(),
            shard_hash: "shard-1".to_string(),
            requested_steward_count: 4,
            achieved_steward_count: 3,
            contract_coverage: 0.75,
            gap_kind: "peers-unavailable".to_string(),
            first_seen_at: "2026-06-14T00:00:00Z".to_string(),
            last_seen_at: "2026-06-14T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn felt_unmeasured_is_not_yet_seen() {
        // Honesty gate (highest precedence): never seen the distribution plane.
        let felt = build_felt_status("unmeasured", &[], vec![], "standard", false);
        assert_eq!(felt.reassurance, "not-yet-seen");
        assert!(felt.headline.contains("can't confirm"), "{}", felt.headline);
        assert_eq!(
            felt.suggested_action.as_deref(),
            Some("Invite a household to help hold these")
        );
        assert_eq!(felt.floor.tier, "standard");
        assert!(!felt.floor.tier_declared);
        assert_eq!(felt.floor.wants_households, 3);
        assert_eq!(felt.floor.has_households, 0);
    }

    #[test]
    fn felt_protected_names_the_holders() {
        // Scenario 1: 3 households, all named, no gaps, standard tier → protected.
        let held = vec![
            holder("h-dowell", Some("the Dowells")),
            holder("h-ruth", Some("Aunt Ruth")),
            holder("c-church", Some("First Church")),
        ];
        let felt = build_felt_status("measured", &[], held, "standard", false);
        assert_eq!(felt.reassurance, "protected");
        assert_eq!(
            felt.headline,
            "Held by 3 households: the Dowells, Aunt Ruth, First Church"
        );
        assert!(felt.suggested_action.is_none());
        assert_eq!(felt.floor.has_households, 3);
        assert_eq!(felt.held_by.len(), 3);
    }

    #[test]
    fn felt_protected_without_all_labels_falls_back_to_count() {
        let held = vec![
            holder("a", Some("Home A")),
            holder("b", None),
            holder("c", Some("Home C")),
        ];
        let felt = build_felt_status("measured", &[], held, "standard", false);
        assert_eq!(felt.reassurance, "protected");
        assert_eq!(felt.headline, "Held by 3 households");
    }

    #[test]
    fn felt_lapse_but_covered_is_watching() {
        // Scenario 2: a holder lapsed (gap), 2 still hold → watching, reassure.
        let held = vec![
            holder("a", Some("the Dowells")),
            holder("b", Some("First Church")),
        ];
        let felt = build_felt_status("measured", &[a_gap()], held, "standard", false);
        assert_eq!(felt.reassurance, "watching");
        assert_eq!(felt.headline, "Still safe — 2 households are holding these");
        assert!(felt.suggested_action.is_none());
    }

    #[test]
    fn felt_single_household_needs_help() {
        // Scenario 4: held by only 1 household → needs-help + pro-social action.
        let held = vec![holder("a", Some("the Dowells"))];
        let felt = build_felt_status("measured", &[], held, "standard", false);
        assert_eq!(felt.reassurance, "needs-help");
        assert!(
            felt.headline.contains("only 1 household"),
            "{}",
            felt.headline
        );
        assert_eq!(
            felt.suggested_action.as_deref(),
            Some("Invite a household to help hold these")
        );
        assert_eq!(felt.floor.has_households, 1);
        assert_eq!(felt.floor.wants_households, 3);
    }

    #[test]
    fn felt_vault_floor_underclaims_no_false_reassurance() {
        // Operator note: a vault-tier content held only at the standard bar (3)
        // must NOT read "protected" — its floor wants 5. (False-reassurance closed.)
        let held = vec![
            holder("a", Some("the Dowells")),
            holder("b", Some("Aunt Ruth")),
            holder("c", Some("First Church")),
        ];
        let felt = build_felt_status("measured", &[], held, "vault", true);
        assert_eq!(felt.reassurance, "watching");
        assert_eq!(
            felt.headline,
            "Held by 3 of the 5 households this should live in"
        );
        assert_eq!(felt.floor.tier, "vault");
        assert!(felt.floor.tier_declared);
        assert_eq!(felt.floor.wants_households, 5);
        assert_eq!(felt.floor.has_households, 3);
    }

    #[test]
    fn felt_ephemeral_floor_protected_at_one_no_false_alarm() {
        // Operator note: an ephemeral content (e.g. a dropdown-map) at its floor
        // of 1 must NOT read "needs-help" — 1 holder meets the ephemeral floor.
        let held = vec![holder("a", Some("Commons Cache"))];
        let felt = build_felt_status("measured", &[], held, "ephemeral", true);
        assert_eq!(felt.reassurance, "protected");
        assert_eq!(felt.floor.wants_households, 1);
        assert!(felt.suggested_action.is_none());
    }

    #[test]
    fn felt_below_standard_floor_two_held_is_watching() {
        // 2 of 3 (standard), no gap → watching with the floor-relative headline.
        let held = vec![holder("a", Some("Home A")), holder("b", Some("Home B"))];
        let felt = build_felt_status("measured", &[], held, "standard", false);
        assert_eq!(felt.reassurance, "watching");
        assert_eq!(
            felt.headline,
            "Held by 2 of the 3 households this should live in"
        );
    }

    #[test]
    fn felt_zero_holders_needs_help() {
        let felt = build_felt_status("measured", &[], vec![], "standard", false);
        assert_eq!(felt.reassurance, "needs-help");
        assert!(felt.headline.contains("No household"), "{}", felt.headline);
    }

    #[test]
    fn felt_floor_for_tier_mapping() {
        assert_eq!(floor_for_tier("vault"), 5);
        assert_eq!(floor_for_tier("keepsake"), 4);
        assert_eq!(floor_for_tier("standard"), 3);
        assert_eq!(floor_for_tier("ephemeral"), 1);
        assert_eq!(floor_for_tier("anything-undeclared"), 3);
    }
}
