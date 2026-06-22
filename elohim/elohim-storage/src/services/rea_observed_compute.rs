//! REA economic facing — the OBSERVED-compute reciprocity adapter (Wave 4.3).
//!
//! The 4.1 `mutual_compute` fold (`elohim_facings::folds::rea`) flags reciprocal
//! `delegates-compute` pairs over a `&[CommitmentRow]` slice. By itself it is
//! INTENT-only: a pair appears as soon as both agents have *committed* to
//! delegate, whether or not any compute was ever actually performed.
//!
//! This adapter is the OBSERVED counterpart. It does NOT modify the fold — it
//! changes *which rows the fold sees*. Given the set of commitment cids that an
//! OBSERVED `compute-fulfilled` `EconomicEvent` is `bounded_by`, it keeps only
//! the `delegates-compute` commitments that were actually fulfilled, then calls
//! the unmodified `mutual_compute`. The result is "reciprocal compute that was
//! observed in both directions" — the realized, not merely promised, reciprocity.
//!
//! ## Two readings off ONE fold
//! - `mutual_compute(all_rows)`            → INTENT reciprocity (who promised).
//! - `mutual_compute(fulfilled_rows)`      → OBSERVED reciprocity (what happened),
//!   where `fulfilled_rows = retain_fulfilled(all_rows, fulfilled_cids)`.
//!
//! ## Why the filter, not a fold change
//! `mutual_compute` already filters `action == "delegates-compute"` and excludes
//! self-edges. Observed-ness is orthogonal to that logic — it is purely a
//! row-membership question keyed on `cid ∈ fulfilled_cids`. Encoding it as a fold
//! parameter would couple the pure lens crate to the `economic_events` ledger;
//! keeping it here (a thin storage-side adapter) honors the
//! `elohim-facings` DB-boundary discipline and the "consume, don't modify"
//! contract for the 4.1 folds.
//!
//! ## Producer status (correct-but-dormant)
//! As of Wave 4.3 nothing observes real compute fulfillment, so
//! `fulfilled_cids` is empty in production today and `observed_mutual_compute`
//! returns `{}` — honest and correct. It is deliberately NOT wired into a hot
//! path that would be structurally empty (the inventory-≠-replication trap); it
//! lights up the moment a real `compute-fulfilled` observer lands (operator-owned
//! per the charter §"Non-goals"). See
//! `db::economic_events::record_compute_fulfilled_event` /
//! `list_compute_fulfilled_events` for the event side.

use std::collections::{BTreeSet, HashSet};

use elohim_facings::folds::rea::{mutual_compute, CommitmentRow};

use crate::db::models::EconomicEvent;

/// Project a slice of `compute-fulfilled` `EconomicEvent`s down to the SET of
/// commitment cids they are `bounded_by` — the bridge between the event ledger
/// (`db::economic_events::list_compute_fulfilled_events`) and the
/// [`observed_mutual_compute`] fold input.
///
/// `bounded_by` holds the commitment `entry_hash` (= `CommitmentRow.cid`), so the
/// resulting set joins directly against the delegates-compute rows. Events with a
/// NULL `bounded_by` (none should occur on the `compute-fulfilled` path, which
/// always sets it) are dropped. Pure; no DB.
pub fn fulfilled_cids_from_events(events: &[EconomicEvent]) -> HashSet<String> {
    events.iter().filter_map(|e| e.bounded_by.clone()).collect()
}

/// Keep only the commitment rows whose `cid` appears in `fulfilled_cids` — i.e.
/// the `delegates-compute` agreements for which an OBSERVED `compute-fulfilled`
/// event exists. Pure; no DB.
pub fn retain_fulfilled<'a>(
    rows: &'a [CommitmentRow],
    fulfilled_cids: &HashSet<String>,
) -> Vec<&'a CommitmentRow> {
    rows.iter()
        .filter(|r| fulfilled_cids.contains(&r.cid))
        .collect()
}

/// OBSERVED reciprocal `delegates-compute` pairs: reciprocity that was actually
/// fulfilled in BOTH directions, not merely committed.
///
/// `fulfilled_cids` is the set of commitment cids that a `compute-fulfilled`
/// `EconomicEvent` is `bounded_by` (project it from
/// `db::economic_events::list_compute_fulfilled_events`). A reciprocal pair
/// `(A, B)` appears ONLY when BOTH the A→B commitment AND the B→A commitment have
/// a fulfillment event — i.e. observed compute flowed both ways.
///
/// Built by filtering to the fulfilled rows and then calling the UNMODIFIED 4.1
/// `mutual_compute` fold — the canonical `(min, max)`-ordered, deduped
/// `BTreeSet<(String, String)>` it already returns.
///
/// ⚠ HELD security boundary: the observed-ness this reads is only as trustworthy
/// as the events feeding `fulfilled_cids`. Until `record_compute_fulfilled_event`
/// gains counterparty attestation + party-matching (see its doc's "HELD security
/// boundary"), the reciprocity here is unilaterally forgeable — do NOT drive any
/// economic consequence off it.
pub fn observed_mutual_compute(
    rows: &[CommitmentRow],
    fulfilled_cids: &HashSet<String>,
) -> BTreeSet<(String, String)> {
    // Materialize the fulfilled subset into owned rows so we can hand the
    // unmodified fold a `&[CommitmentRow]`.
    let fulfilled: Vec<CommitmentRow> = retain_fulfilled(rows, fulfilled_cids)
        .into_iter()
        .cloned()
        .collect();
    mutual_compute(&fulfilled)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `delegates-compute` commitment row with an explicit `cid` (the join key
    /// to its `compute-fulfilled` event's `bounded_by`).
    fn delegate(cid: &str, provider: &str, recipient: &str) -> CommitmentRow {
        CommitmentRow {
            cid: cid.to_string(),
            action: "delegates-compute".to_string(),
            scope: "republish-epr".to_string(),
            provider: provider.to_string(),
            recipient: recipient.to_string(),
            resource_classified_as: vec![],
            household_id: None,
            valid_from: "2026-05-28T00:00:00Z".to_string(),
            valid_until: "2026-08-26T00:00:00Z".to_string(),
            bounds_json: "{}".to_string(),
            state: "active".to_string(),
        }
    }

    fn cids(items: &[&str]) -> HashSet<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn observed_mutual_compute_flags_pair_when_both_directions_fulfilled() {
        // A→B (cid-1) and B→A (cid-2) — a reciprocal compute agreement.
        let rows = vec![delegate("cid-1", "A", "B"), delegate("cid-2", "B", "A")];
        // BOTH commitments have an observed compute-fulfilled event.
        let fulfilled = cids(&["cid-1", "cid-2"]);

        let observed = observed_mutual_compute(&rows, &fulfilled);
        assert_eq!(
            observed.len(),
            1,
            "both directions fulfilled → one observed reciprocal pair"
        );
        assert!(
            observed.contains(&("A".to_string(), "B".to_string())),
            "canonical (min, max) order: (A, B)"
        );
    }

    #[test]
    fn observed_mutual_compute_drops_pair_when_one_direction_unfulfilled() {
        let rows = vec![delegate("cid-1", "A", "B"), delegate("cid-2", "B", "A")];
        // Only the A→B direction was observed; B→A never fulfilled.
        let fulfilled = cids(&["cid-1"]);

        let observed = observed_mutual_compute(&rows, &fulfilled);
        assert!(
            observed.is_empty(),
            "one-directional fulfillment is not OBSERVED reciprocity"
        );
    }

    #[test]
    fn observed_is_a_subset_of_intent() {
        // Intent: A↔B AND C↔D both committed (two reciprocal pairs).
        let rows = vec![
            delegate("cid-ab", "A", "B"),
            delegate("cid-ba", "B", "A"),
            delegate("cid-cd", "C", "D"),
            delegate("cid-dc", "D", "C"),
        ];
        // Intent reads ALL rows → two pairs.
        let intent = mutual_compute(&rows);
        assert_eq!(
            intent.len(),
            2,
            "two reciprocal pairs are committed (intent)"
        );

        // Observed: only A↔B was fulfilled both ways; C↔D had no events.
        let fulfilled = cids(&["cid-ab", "cid-ba"]);
        let observed = observed_mutual_compute(&rows, &fulfilled);
        assert_eq!(
            observed.into_iter().collect::<Vec<_>>(),
            vec![("A".to_string(), "B".to_string())],
            "observed reciprocity is the fulfilled subset of intent"
        );
    }

    #[test]
    fn observed_mutual_compute_empty_fulfilled_is_empty() {
        // The correct-but-dormant production state: no fulfillment observed yet.
        let rows = vec![delegate("cid-1", "A", "B"), delegate("cid-2", "B", "A")];
        assert!(
            observed_mutual_compute(&rows, &HashSet::new()).is_empty(),
            "no observed events → no observed reciprocity (dormant, not wrong)"
        );
    }

    #[test]
    fn retain_fulfilled_keeps_only_matching_cids() {
        let rows = vec![
            delegate("cid-1", "A", "B"),
            delegate("cid-2", "B", "A"),
            delegate("cid-3", "C", "D"),
        ];
        let kept = retain_fulfilled(&rows, &cids(&["cid-1", "cid-3"]));
        let kept_cids: Vec<&str> = kept.iter().map(|r| r.cid.as_str()).collect();
        assert_eq!(kept_cids, vec!["cid-1", "cid-3"]);
    }

    // ── END-TO-END: compute-fulfilled events → fold reads OBSERVED reciprocity ──
    //
    // The deliverable seam: a `compute-fulfilled` EconomicEvent is `bounded_by`
    // the originating commitment, and `mutual_compute` (via the observed adapter)
    // reads it as observed reciprocity when BOTH directions are fulfilled. This
    // threads REAL DB rows through `record_compute_fulfilled_event` →
    // `list_compute_fulfilled_events` → `fulfilled_cids_from_events` →
    // `observed_mutual_compute`. The commitment `cid` (entry_hash) used to build
    // the `CommitmentRow`s is the SAME value as each event's `bounded_by`, which
    // is what proves the join lands on entry_hash (not action_hash).
    mod end_to_end {
        use super::*;
        use crate::db::context::AppContext;
        use crate::db::economic_events::{
            list_compute_fulfilled_events, record_compute_fulfilled_event,
        };
        use diesel::prelude::*;
        use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

        const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

        fn test_conn() -> SqliteConnection {
            let mut conn = SqliteConnection::establish(":memory:").expect("in-memory SQLite");
            conn.run_pending_migrations(MIGRATIONS).expect("migrations");
            conn
        }

        #[test]
        fn compute_fulfilled_events_drive_observed_mutual_compute_join_on_entry_hash() {
            let mut conn = test_conn();
            let ctx = AppContext::new("test-app");

            // Two reciprocal delegates-compute commitments. Their `cid` IS the
            // entry_hash and IS the value each fulfillment event is bounded_by.
            let cid_ab = "uhCEk-entry-AB";
            let cid_ba = "uhCEk-entry-BA";
            let rows = vec![
                delegate(cid_ab, "agent:A", "agent:B"),
                delegate(cid_ba, "agent:B", "agent:A"),
            ];

            // Record BOTH fulfillment events (observed compute flowed both ways),
            // bounded_by the commitment entry_hash.
            record_compute_fulfilled_event(
                &mut conn,
                &ctx,
                "cf-ab",
                "agent:A",
                "agent:B",
                cid_ab,
                "2026-06-22T00:00:00Z",
            )
            .expect("record A→B fulfillment");
            record_compute_fulfilled_event(
                &mut conn,
                &ctx,
                "cf-ba",
                "agent:B",
                "agent:A",
                cid_ba,
                "2026-06-22T00:01:00Z",
            )
            .expect("record B→A fulfillment");

            // Project the ledger → fulfilled cids → fold input.
            let events = list_compute_fulfilled_events(&mut conn, &ctx).expect("list events");
            let fulfilled = fulfilled_cids_from_events(&events);
            assert_eq!(
                fulfilled,
                cids(&[cid_ab, cid_ba]),
                "fulfilled cids project from the events' bounded_by (entry_hash)"
            );

            let observed = observed_mutual_compute(&rows, &fulfilled);
            assert_eq!(
                observed.into_iter().collect::<Vec<_>>(),
                vec![("agent:A".to_string(), "agent:B".to_string())],
                "both directions fulfilled → mutual_compute reads OBSERVED reciprocity"
            );
        }

        #[test]
        fn dropping_one_fulfillment_event_collapses_observed_reciprocity() {
            let mut conn = test_conn();
            let ctx = AppContext::new("test-app");

            let cid_ab = "uhCEk-entry-AB";
            let cid_ba = "uhCEk-entry-BA";
            let rows = vec![
                delegate(cid_ab, "agent:A", "agent:B"),
                delegate(cid_ba, "agent:B", "agent:A"),
            ];

            // Only the A→B direction is fulfilled; B→A is intent-only.
            record_compute_fulfilled_event(
                &mut conn,
                &ctx,
                "cf-ab",
                "agent:A",
                "agent:B",
                cid_ab,
                "2026-06-22T00:00:00Z",
            )
            .expect("record A→B fulfillment only");

            let events = list_compute_fulfilled_events(&mut conn, &ctx).expect("list events");
            let fulfilled = fulfilled_cids_from_events(&events);
            assert_eq!(fulfilled, cids(&[cid_ab]), "only A→B observed");

            assert!(
                observed_mutual_compute(&rows, &fulfilled).is_empty(),
                "one-directional observed compute is NOT reciprocity — pair collapses"
            );
        }
    }
}
