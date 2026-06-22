//! The REA / economic facing's folds — the commitment-ledger lens. Pure
//! (DB-free); the diesel rows live in elohim-storage and are mirrored onto
//! `CommitmentRow` before they reach these folds (the §11 add-a-lens recipe,
//! elohim/elohim-facings/CLAUDE.md).
//!
//! Charter: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
//! (§"Folds"). The resiliency facing's `commitment_backed_collectives`
//! (`elohim-storage/src/services/household_resilience.rs` 155–208) is the proven
//! reference whose intent `commitment_backed` lifts here, DB-free.

use std::collections::{BTreeMap, BTreeSet, HashSet};

/// DB-free mirror of one commitment row, loaded **unified across both ledgers**
/// (`rea_commitments` + `mishpat_commitments`) per the charter's "Materialized
/// relation(s)". The diesel rows are mapped onto this by the storage loader so
/// the folds stay DB-free.
///
/// Field provenance — the real diesel shape is `NewMishpatCommitment`
/// (`elohim-storage/src/mishpat_projection.rs`) for the compute-agreements ledger,
/// plus the `rea_commitments` provide-side fields the charter unifies in:
/// - `cid` — the commitment `entry_hash` (`mishpat_commitments.cid`); the natural
///   join key (`economic_events.bounded_by`).
/// - `action` — discriminator string: `provide` | `replicates-content` |
///   `replicates-commons` | `replicates-dwelling` | `delegates-compute` |
///   `custody-blob` | `project-epr` … (mirrors `NewMishpatCommitment.action`).
/// - `scope` — the commitment's own scope/`in_scope_of` (`NewMishpatCommitment.scope`).
///   NOT the classification list — `commitment_backed` matches against
///   `resource_classified_as`, never this field (the easy-to-conflate trap).
/// - `provider` / `recipient` — `agent_cid` (`uhCAk…`) on both ends
///   (`NewMishpatCommitment.provider` / `.recipient`); never a transport id.
/// - `resource_classified_as` — the parsed classification list from the
///   `rea_commitments` provide side. The impure loader runs `classifications_of`
///   (the JSON parse) so the fold receives an already-parsed `Vec<String>`,
///   keeping it DB-free. Empty for compute-ledger rows that carry no classification.
/// - `household_id` — the provider's household (filled by the loader from the
///   `humans` join on `agent_pub_key = provider`, `household_id IS NOT NULL`).
///   `commitment_backed` counts distinct households, NOT providers — a
///   multi-tenant hub running >1 provider agent is one collective (mirrors the
///   reference's distinct-`household_id` HashSet).
/// - `valid_from` / `valid_until` — the validity window
///   (`NewMishpatCommitment.valid_from` / `.valid_until`); public-API fields the
///   later `realized_value_flow` / display folds read.
/// - `bounds_json` — the action-specific policy envelope as a raw JSON string
///   (`NewMishpatCommitment.bounds_json`), kept unparsed (DB-free; no serde dep).
/// - `state` — `proposed` | `active` | … (`NewMishpatCommitment.state`).
///   `commitment_backed` requires `active`.
#[derive(Debug, Clone)]
pub struct CommitmentRow {
    pub cid: String,
    pub action: String,
    pub scope: String,
    pub provider: String,
    pub recipient: String,
    pub resource_classified_as: Vec<String>,
    pub household_id: Option<String>,
    pub valid_from: String,
    pub valid_until: String,
    pub bounds_json: String,
    pub state: String,
}

/// Provide-class actions: the REA `provide` literal the seeder writes, plus the
/// DHT action discriminators the runtime mishpat projection writes for the same
/// economic act (a household committing to hold content). Mirrors the reference's
/// `eq_any` filter (`household_resilience.rs` 185–189) — filtering only `provide`
/// silently drops every runtime-authored commitment.
const PROVIDE_CLASS_ACTIONS: [&str; 3] = ["provide", "replicates-content", "replicates-commons"];

/// **Intent.** Distinct households with an `active` provide-class commitment whose
/// classifications contain `scope` (e.g. `content:commons`). Lifts the proven
/// `commitment_backed_collectives` logic DB-free.
///
/// Counts distinct `household_id` (dropping `None` — the `household_id IS NOT NULL`
/// filter), NOT distinct providers: a multi-tenant hub running several provider
/// agents under one household is ONE collective. Membership is tested against
/// `resource_classified_as` (the classification list), never the row's `scope`
/// field.
pub fn commitment_backed(rows: &[CommitmentRow], scope: &str) -> usize {
    rows.iter()
        .filter(|r| r.state == "active")
        .filter(|r| PROVIDE_CLASS_ACTIONS.contains(&r.action.as_str()))
        .filter(|r| r.resource_classified_as.iter().any(|c| c == scope))
        .filter_map(|r| r.household_id.clone())
        .collect::<HashSet<String>>()
        .len()
}

/// Count of commitments per `action` discriminator. `bucket_by` → deterministic
/// `BTreeMap` key order; no hardcoded action whitelist (it counts whatever is
/// present: `provide`, `operate-doorway`, `project-epr`, `custody-blob`,
/// `delegates-compute`, `replicates-dwelling`, …).
pub fn by_action(rows: &[CommitmentRow]) -> BTreeMap<String, usize> {
    crate::fold::bucket_by(rows, |r| Some(r.action.clone()))
        .into_iter()
        .map(|(action, bucket)| (action, bucket.len()))
        .collect()
}

/// Reciprocal `delegates-compute` pairs (intent-only until a compute-event bridge
/// exists). Scoped to `delegates-compute` only; flags `(a, b)` where A delegates
/// to B AND B delegates to A, emitted once in canonical `(min, max)` order.
///
/// Returns a `BTreeSet<(String, String)>` — auto-sorted, auto-deduped, so a
/// reciprocal relationship reports exactly once on the wire regardless of which
/// directed edge was seen first. Self-pairs (`provider == recipient`) are excluded.
pub fn mutual_compute(rows: &[CommitmentRow]) -> BTreeSet<(String, String)> {
    // All directed delegates-compute edges (provider → recipient), excluding
    // self-edges (a node delegating to itself is not a mutual relationship).
    let edges: HashSet<(String, String)> = rows
        .iter()
        .filter(|r| r.action == "delegates-compute")
        .filter(|r| r.provider != r.recipient)
        .map(|r| (r.provider.clone(), r.recipient.clone()))
        .collect();

    let mut pairs: BTreeSet<(String, String)> = BTreeSet::new();
    for (a, b) in &edges {
        if edges.contains(&(b.clone(), a.clone())) {
            // Canonical (min, max) order so {A,B} dedups to one entry.
            let pair = if a <= b {
                (a.clone(), b.clone())
            } else {
                (b.clone(), a.clone())
            };
            pairs.insert(pair);
        }
    }
    pairs
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Hand-built fixture row. Defaults to an empty classification list and no
    /// household; per-test builders override the fields the fold under test reads.
    fn row(action: &str, provider: &str, recipient: &str, state: &str) -> CommitmentRow {
        CommitmentRow {
            cid: format!("cid-{provider}-{recipient}-{action}"),
            action: action.to_string(),
            scope: "republish-epr".to_string(),
            provider: provider.to_string(),
            recipient: recipient.to_string(),
            resource_classified_as: vec![],
            household_id: None,
            valid_from: "2026-05-28T00:00:00Z".to_string(),
            valid_until: "2026-08-26T00:00:00Z".to_string(),
            bounds_json: "{}".to_string(),
            state: state.to_string(),
        }
    }

    /// A provide-class commitment with a household + classification list.
    fn provide(action: &str, household: &str, classified: &[&str], state: &str) -> CommitmentRow {
        let mut r = row(action, "agent:p", "epr:head", state);
        r.household_id = Some(household.to_string());
        r.resource_classified_as = classified.iter().map(|s| s.to_string()).collect();
        r
    }

    // ── commitment_backed ─────────────────────────────────────────────────────

    #[test]
    fn commitment_backed_counts_distinct_active_provide_households() {
        let rows = vec![
            provide("provide", "house-a", &["content:commons"], "active"),
            // same household, second provider agent — multi-tenant hub → one collective
            provide(
                "replicates-content",
                "house-a",
                &["content:commons"],
                "active",
            ),
            provide(
                "replicates-commons",
                "house-b",
                &["content:commons"],
                "active",
            ),
        ];
        assert_eq!(
            commitment_backed(&rows, "content:commons"),
            2,
            "two distinct households (house-a counted once despite two provider agents)"
        );
    }

    #[test]
    fn commitment_backed_excludes_non_active_non_provide_unmatched_and_null_household() {
        let rows = vec![
            // proposed state → excluded
            provide("provide", "house-a", &["content:commons"], "proposed"),
            // delegates-compute is not provision → excluded even if active
            provide(
                "delegates-compute",
                "house-b",
                &["content:commons"],
                "active",
            ),
            // classification does not contain the scope → excluded
            provide("provide", "house-c", &["content:household"], "active"),
            // active + provide + scope match BUT household is None → dropped (IS NOT NULL)
            {
                let mut r = row("provide", "agent:p", "epr:x", "active");
                r.resource_classified_as = vec!["content:commons".to_string()];
                r // household_id stays None
            },
        ];
        assert_eq!(
            commitment_backed(&rows, "content:commons"),
            0,
            "none qualify: proposed / wrong-action / wrong-scope / null-household all excluded"
        );
    }

    #[test]
    fn commitment_backed_empty_is_zero() {
        assert_eq!(commitment_backed(&[], "content:commons"), 0);
    }

    // ── by_action ─────────────────────────────────────────────────────────────

    #[test]
    fn by_action_buckets_correctly_and_deterministically() {
        let rows = vec![
            row("project-epr", "a", "b", "active"),
            row("custody-blob", "a", "c", "active"),
            row("project-epr", "d", "e", "active"), // repeat action
            row("delegates-compute", "x", "y", "active"),
        ];
        let counts = by_action(&rows);
        assert_eq!(counts.get("project-epr"), Some(&2));
        assert_eq!(counts.get("custody-blob"), Some(&1));
        assert_eq!(counts.get("delegates-compute"), Some(&1));

        // BTreeMap → sorted key order, deterministic on the wire.
        let keys: Vec<&String> = counts.keys().collect();
        assert_eq!(
            keys,
            vec!["custody-blob", "delegates-compute", "project-epr"]
        );
    }

    #[test]
    fn by_action_empty_is_empty() {
        assert!(by_action(&[]).is_empty());
    }

    // ── mutual_compute ────────────────────────────────────────────────────────

    #[test]
    fn mutual_compute_flags_reciprocal_pair_once() {
        let rows = vec![
            row("delegates-compute", "A", "B", "active"),
            row("delegates-compute", "B", "A", "active"),
        ];
        let pairs = mutual_compute(&rows);
        assert_eq!(pairs.len(), 1, "A↔B is one reciprocal relationship");
        assert!(
            pairs.contains(&("A".to_string(), "B".to_string())),
            "canonical (min, max) order: (A, B)"
        );
    }

    #[test]
    fn mutual_compute_does_not_flag_one_way() {
        let rows = vec![
            row("delegates-compute", "A", "B", "active"),
            // C→D one-way, no reciprocal
            row("delegates-compute", "C", "D", "active"),
        ];
        assert!(
            mutual_compute(&rows).is_empty(),
            "no reciprocal edge → no pair"
        );
    }

    #[test]
    fn mutual_compute_dedups_duplicate_directed_edges() {
        let rows = vec![
            row("delegates-compute", "A", "B", "active"),
            row("delegates-compute", "A", "B", "active"), // duplicate directed edge
            row("delegates-compute", "B", "A", "active"),
        ];
        let pairs = mutual_compute(&rows);
        assert_eq!(pairs.len(), 1, "duplicate A→B still yields one A↔B pair");
        assert!(pairs.contains(&("A".to_string(), "B".to_string())));
    }

    #[test]
    fn mutual_compute_ignores_non_delegates_compute_and_self_pairs() {
        let rows = vec![
            // reciprocal but NOT delegates-compute → ignored
            row("replicates-dwelling", "A", "B", "active"),
            row("replicates-dwelling", "B", "A", "active"),
            // self-delegation → excluded
            row("delegates-compute", "S", "S", "active"),
        ];
        assert!(
            mutual_compute(&rows).is_empty(),
            "non-compute reciprocity and self-pairs are not mutual_compute"
        );
    }

    #[test]
    fn mutual_compute_canonical_order_independent_of_input_order() {
        // Edges seen in B→A then A→B order must still canonicalize to (A, B).
        let rows = vec![
            row("delegates-compute", "B", "A", "active"),
            row("delegates-compute", "A", "B", "active"),
        ];
        let pairs = mutual_compute(&rows);
        assert_eq!(
            pairs.into_iter().collect::<Vec<_>>(),
            vec![("A".to_string(), "B".to_string())]
        );
    }
}
