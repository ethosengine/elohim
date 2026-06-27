//! Production [`CommitmentAuthor`] for Phase-3 salvage (P3-5).
//!
//! When the salvage pass self-selects this node as a new holder for an
//! under-replicated blob, it must author a NOTARIZED placement intent — a
//! `custody-blob` REA commitment. That intent is Class A (notarized), so it goes
//! through the conductor (`content_store::create_rea_commitment`), NOT a local
//! SQL insert. Post-commit the DNA emits `ProjectionSignal::ReaCommitmentCommitted`
//! and `rea_projection` projects it into `rea_commitments` with `dht_anchor_hash`.
//! The next custody reconcile pass then sees `provider == self`, the blob missing
//! locally, and fetches the bytes — **salvage authors intent; reconcile moves
//! bytes** (no new fetch path).
//!
//! ## Sync trait over an async conductor call
//!
//! [`crate::reconcile::custody::CommitmentAuthor`] is SYNC (so `salvage_pass`
//! stays sync + unit-testable). [`conductor_writes::call_create_rea_commitment`]
//! is async. We bridge with the codebase-canonical
//! `tokio::task::block_in_place` + `Handle::block_on` pattern (mirrors
//! `services::epr_store`), which moves other tokio tasks off this worker thread
//! before driving the conductor future to completion — avoiding the
//! "cannot start a runtime from within a runtime" panic. The caller
//! (`P2PNode::run_salvage_pass` / the reconcile task) is already on a tokio
//! runtime, so a current `Handle` is always available there.

use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::error::StorageError;
use crate::hc_client::HcClient;
use crate::reconcile::custody::CommitmentAuthor;
use crate::services::conductor_writes;

/// Authors notarized salvage `custody-blob` commitments via the conductor.
pub struct SalvageCommitmentAuthor {
    hc: Arc<HcClient>,
}

impl SalvageCommitmentAuthor {
    /// Construct with a live conductor handle (the `lamad`/`content_store` cell).
    pub fn new(hc: Arc<HcClient>) -> Self {
        Self { hc }
    }
}

/// Deterministic commitment id matching the seeder's custody-blob shape
/// (`seed-commitments.ts::buildCustodyCommitmentBody`):
/// `custody-blob-<sha256(provider|receiver|blob)[:16]>`. Re-authoring the same
/// (provider, receiver, blob) tuple yields the same id → idempotent at the
/// conductor (a duplicate is a no-op / 409, never a corrupt second row).
fn deterministic_custody_id(provider: &str, receiver: &str, blob_marker: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{provider}|{receiver}|{blob_marker}").as_bytes());
    let digest = hex::encode(hasher.finalize());
    format!("custody-blob-{}", &digest[..16])
}

impl CommitmentAuthor for SalvageCommitmentAuthor {
    fn author_custody_blob(
        &self,
        blob_marker: &str,
        provider: &str,
        receiver: &str,
    ) -> Result<(), StorageError> {
        // Build the canonical custody-blob input. Matches the seeder/HTTP shape
        // (action="custody-blob", resource_classified_as=[blob_marker], unit "B").
        let input = shefa_types::CreateReaCommitmentInput {
            id: deterministic_custody_id(provider, receiver, blob_marker),
            action: "custody-blob".to_string(),
            provider: provider.to_string(),
            receiver: receiver.to_string(),
            resource_classified_as: vec![blob_marker.to_string()],
            resource_quantity_value: None,
            resource_quantity_unit: Some("B".to_string()),
            effort_quantity_value: None,
            effort_quantity_unit: None,
            has_beginning: None,
            has_end: None,
            due: None,
            clause_of: None,
            in_scope_of: Vec::new(),
            note: Some(format!(
                "salvage: {provider} self-selected to host {blob_marker} for {receiver}"
            )),
            metadata_json: Some(
                serde_json::json!({
                    "origin": "salvage",
                    "blobMarker": blob_marker,
                })
                .to_string(),
            ),
        };

        // Bridge sync → async with the canonical block_in_place + block_on pattern.
        let handle = tokio::runtime::Handle::try_current().map_err(|_| {
            StorageError::Internal(
                "salvage author: no tokio runtime handle (must be called from an async context)"
                    .to_string(),
            )
        })?;
        let hc = self.hc.clone();
        tokio::task::block_in_place(|| {
            handle.block_on(async move {
                conductor_writes::call_create_rea_commitment(&hc, &input).await
            })
        })
        .map(|_bytes| ())
    }
}

/// P3-8: Build the salvage candidate pool and enrich each candidate's
/// `household_id` from the notarized `humans` projection.
///
/// The pool starts from FRESH, opted-in `salvage_capacity` rows (agent_cid-keyed
/// — the pool the placement metric ranks never crosses namespaces), with self
/// appended so this node can self-select even before its own capacity ad has
/// round-tripped through gossip (first-tick convergence).
///
/// **Household enrichment (option B — joined, not self-reported).** Mirrors the
/// ingest selector (`services/peer_selection.rs:184-203`): join
/// `salvage_capacity.agent_cid == humans.agent_pub_key` scoped to `h_app_id`, and
/// set each candidate's `household_id` from the result. Failure domain comes from
/// the notarized projection, never peer self-report (option A, rejected on trust
/// grounds). A candidate with no matching human (or a human whose `household_id`
/// is NULL) keeps `household_id: None` — and the diversity strategy treats `None`
/// as the candidate's own domain, degrading to XOR for it.
///
/// **Namespace note.** The match is sound only in that it never produces a
/// *wrong* binding (a cid receives one of its OWN humans rows or nothing). It is
/// NOT guaranteed both sides are the `uhCAk…` agent-key namespace on the live
/// mesh: `salvage_capacity.agent_cid`/`self_cid` may be a libp2p/iroh transport id
/// unless `SELF_CID` pins the agent key — one of the reasons this join is latent
/// in production today (see the dormancy note on [`run_salvage_pass`]).
///
/// **Dormancy.** Two reasons a candidate can stay `None` in production: (1) NULL
/// `agent_pub_key` humans (the dormant-human gate); (2) a writer/reader `h_app_id`
/// SCOPE SPLIT — household-bearing humans rows are WRITTEN under
/// `h_app_id="imagodei"` (`api/identity.rs`, `services/genesis_self_heal.rs`)
/// while this join (like the ingest selector) READS under `"lamad"`, so it returns
/// no household rows until a shared substrate scope reconciliation lands. Both are
/// shared with ingest + the resilience card, tracked in
/// `genesis/data/timeline/backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md`
/// — NOT fixed here.
fn build_salvage_candidates(
    conn: &mut diesel::SqliteConnection,
    self_cid: &str,
    h_app_id: &str,
    fresh_after: &str,
) -> Result<Vec<crate::reconcile::placement::PlacementCandidate>, StorageError> {
    use crate::db::diesel_schema::humans;
    use crate::reconcile::placement::PlacementCandidate;
    use diesel::prelude::*;

    let fresh_rows = crate::db::salvage_capacity::list_fresh(conn, fresh_after)?;
    let mut candidates: Vec<PlacementCandidate> = fresh_rows
        .into_iter()
        .map(|r| PlacementCandidate {
            agent_cid: r.agent_cid,
            household_id: None,
            archetype: Some(r.archetype),
            spare_bytes: Some(r.spare_bytes.max(0) as u64),
        })
        .collect();
    // Include self so it can self-select even before its own capacity ad has
    // round-tripped back through gossip (first-tick convergence).
    if !candidates.iter().any(|c| c.agent_cid == self_cid) {
        candidates.push(PlacementCandidate::from_agent_cid(self_cid.to_string()));
    }

    // Enrich household_id from the humans projection (mirror of the ingest
    // selector). One query over every candidate cid (incl. self).
    let cids: Vec<String> = candidates.iter().map(|c| c.agent_cid.clone()).collect();

    #[derive(Queryable)]
    struct HumanRow {
        agent_pub_key: Option<String>,
        household_id: Option<String>,
    }
    let human_rows: Vec<HumanRow> = humans::table
        .filter(humans::h_app_id.eq(h_app_id))
        .filter(humans::agent_pub_key.eq_any(&cids))
        // `agent_pub_key` is only non-uniquely indexed, so two rows could share a
        // key with different households. Order by the PK `id` so the collect below
        // is deterministic (last-write-wins → the lexicographically-greatest `id`
        // wins) rather than SQLite-row-order-dependent. The bound household is
        // always one of the cid's OWN rows, never another candidate's.
        .order_by(humans::id.asc())
        .select((humans::agent_pub_key, humans::household_id))
        .load::<HumanRow>(conn)
        .map_err(|e| StorageError::Internal(e.to_string()))?;
    // agent_cid → household_id. filter_map drops NULL-agent_pub_key (dormant) rows.
    let household_by_agent: std::collections::HashMap<String, Option<String>> = human_rows
        .into_iter()
        .filter_map(|r| r.agent_pub_key.map(|k| (k, r.household_id)))
        .collect();

    for c in &mut candidates {
        if let Some(hh) = household_by_agent.get(&c.agent_cid) {
            // Known human: take its household (which may itself be None).
            c.household_id = hh.clone();
        }
        // No matching human → leave None (already the default).
    }

    Ok(candidates)
}

/// P3-6/P3-8: Run one salvage pass against the local projection.
///
/// Builds the candidate pool ([`build_salvage_candidates`] — FRESH opted-in
/// `salvage_capacity` rows + self, each enriched with `household_id` joined from
/// the `humans` projection scoped to `h_app_id`), selects the placement strategy
/// (P3-8: [`crate::reconcile::placement::DiversityAwarePlacementStrategy`] when
/// `diversity_placement` is on, the MVP
/// [`crate::reconcile::placement::XorDistanceStrategy`] when off — both behind the
/// same `PlacementStrategy` seam, so the diversity choice never reworks
/// `salvage_pass`), and invokes [`crate::reconcile::custody::salvage_pass`] with
/// the injected `author`.
///
/// Because the diversity strategy degrades **exactly** to XOR when no household
/// data is present, turning the knob on is never worse than XOR and strictly
/// better once households populate.
///
/// **Safe no-op** when disabled (`salvage_capacity_enabled = false` → the pass
/// skips all authoring) or when the pool is empty (no under-replicated blob can
/// self-select). Authoring is the only side effect; the next custody reconcile
/// pass moves the bytes — **salvage authors intent; reconcile moves bytes** (no
/// new fetch path).
///
/// `h_app_id` is the projection scope for the humans join. Pass the canonical
/// `"lamad"` to MIRROR the ingest selector (`distribute_shards(.., "lamad")`) —
/// NOT the installed Holochain app id (`args.app_id="elohim"`), which would empty
/// the join harder. **Honest caveat:** `"lamad"` does not light the feature in
/// production *today* either — household-bearing humans rows are WRITTEN under
/// `h_app_id="imagodei"`, so the `"lamad"` read returns no household rows and the
/// strategy degrades to XOR. This dormancy is deliberate and SHARED with the
/// ingest selector + resilience card; making the join populate is a substrate-wide
/// scope-reconciliation step left to the backlog
/// (`resilience-card-membership-humans-projection-gap-2026-06-19.md`), not this
/// slice. 1b ships the decision logic + plumbing (fixture-verified); the
/// production efficacy waits on that shared fix.
///
/// Lives here (not in `P2PNode`) because the production [`SalvageCommitmentAuthor`]
/// needs the conductor handle, which is threaded in the reconcile task — the same
/// place [`crate::services::conductor_commitment_author::ConductorCommitmentAuthor`]
/// is wired for the provide loop. Returns the
/// [`crate::reconcile::custody::SalvageOutcome`] for logging/metrics.
#[allow(clippy::too_many_arguments)]
pub fn run_salvage_pass(
    conn: &mut diesel::SqliteConnection,
    self_cid: &str,
    h_app_id: &str,
    author: &dyn CommitmentAuthor,
    enabled: bool,
    target_replicas: usize,
    inventory_freshness_seconds: u64,
    diversity_placement: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<crate::reconcile::custody::SalvageOutcome, StorageError> {
    use crate::reconcile::custody::{salvage_pass, SalvageConfig};

    let fresh_after = (now - chrono::Duration::seconds(inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let candidates = build_salvage_candidates(conn, self_cid, h_app_id, &fresh_after)?;

    let strategy = select_placement_strategy(diversity_placement);
    let cfg = SalvageConfig {
        enabled,
        target_replicas,
        inventory_freshness_seconds,
    };
    salvage_pass(
        conn,
        self_cid,
        strategy.as_ref(),
        author,
        &candidates,
        cfg,
        now,
    )
}

/// Select the placement strategy for a salvage pass. P3-8 knob: the
/// diversity-aware strategy (household-first, XOR tiebreak) when on, the MVP
/// pure-XOR strategy when off. Both impl the same `PlacementStrategy` seam.
fn select_placement_strategy(
    diversity_placement: bool,
) -> Box<dyn crate::reconcile::placement::PlacementStrategy> {
    use crate::reconcile::placement::{DiversityAwarePlacementStrategy, XorDistanceStrategy};
    if diversity_placement {
        Box::new(DiversityAwarePlacementStrategy)
    } else {
        Box::new(XorDistanceStrategy)
    }
}

#[cfg(test)]
mod tests {
    //! P3-8 slice 1b: the humans household-join enrichment + the diversity
    //! placement knob. The strategy ALGORITHMS themselves are covered in
    //! `reconcile::placement`; here we cover (a) the join that gives salvage real
    //! household data and (b) that the knob routes to the right strategy.

    use super::{build_salvage_candidates, select_placement_strategy};
    use crate::db::diesel_schema::humans;
    use crate::db::models::NewHuman;
    use crate::db::salvage_capacity::apply_capacity;
    use crate::error::StorageError;
    use crate::reconcile::placement::{PlacementCandidate, PlacementStrategy, XorDistanceStrategy};
    use crate::test_util::test_pool;
    use diesel::prelude::*;
    use diesel::SqliteConnection;

    const APP: &str = "lamad";

    fn iso_now() -> String {
        chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
    }

    /// A cutoff an hour in the past → every freshly-applied capacity row is fresh.
    fn fresh_cutoff() -> String {
        (chrono::Utc::now() - chrono::Duration::seconds(3600))
            .format("%Y-%m-%dT%H:%M:%SZ")
            .to_string()
    }

    fn seed_capacity(conn: &mut SqliteConnection, cid: &str) {
        apply_capacity(conn, cid, 4096, "node", &iso_now(), 1).unwrap();
    }

    fn seed_human(
        conn: &mut SqliteConnection,
        id: &str,
        agent_pub_key: Option<&str>,
        household_id: Option<&str>,
        h_app_id: &str,
    ) {
        diesel::insert_into(humans::table)
            .values(&NewHuman {
                id: id.to_string(),
                agent_pub_key: agent_pub_key.map(str::to_string),
                display_name: id.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: h_app_id.to_string(),
                household_id: household_id.map(str::to_string),
            })
            .execute(conn)
            .expect("insert human");
    }

    /// The household a built candidate carries (None if absent / unenriched).
    fn hh_of<'a>(cands: &'a [PlacementCandidate], cid: &str) -> Option<&'a str> {
        cands
            .iter()
            .find(|c| c.agent_cid == cid)
            .expect("cid present in candidate set")
            .household_id
            .as_deref()
    }

    // ---- step 3: the humans join populates household_id ----------------------

    #[test]
    fn join_populates_household_id_from_matching_humans() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        seed_capacity(&mut conn, "uhCAk-a");
        seed_capacity(&mut conn, "uhCAk-b");
        seed_human(&mut conn, "h-a", Some("uhCAk-a"), Some("hh-1"), APP);
        seed_human(&mut conn, "h-b", Some("uhCAk-b"), Some("hh-2"), APP);

        let cands =
            build_salvage_candidates(&mut conn, "uhCAk-self", APP, &fresh_cutoff()).unwrap();

        assert_eq!(hh_of(&cands, "uhCAk-a"), Some("hh-1"));
        assert_eq!(hh_of(&cands, "uhCAk-b"), Some("hh-2"));
    }

    #[test]
    fn unmatched_and_null_household_and_dormant_stay_none() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // cid-c has a salvage row but NO matching human → unmatched.
        seed_capacity(&mut conn, "uhCAk-c");
        // cid-d has a human, but that human's household_id is NULL.
        seed_capacity(&mut conn, "uhCAk-d");
        seed_human(&mut conn, "h-d", Some("uhCAk-d"), None, APP);
        // A dormant human: populated household but NULL agent_pub_key. It is never
        // a candidate (candidates come from salvage_capacity) and must not bind.
        seed_human(&mut conn, "h-dormant", None, Some("hh-X"), APP);

        let cands =
            build_salvage_candidates(&mut conn, "uhCAk-self", APP, &fresh_cutoff()).unwrap();

        assert!(
            hh_of(&cands, "uhCAk-c").is_none(),
            "unmatched cid stays None"
        );
        assert!(
            hh_of(&cands, "uhCAk-d").is_none(),
            "known human with NULL household stays None"
        );
        assert!(
            cands
                .iter()
                .all(|c| c.household_id.as_deref() != Some("hh-X")),
            "dormant (NULL agent_pub_key) household must never bind to a candidate"
        );
    }

    #[test]
    fn self_candidate_is_household_enriched() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        // No salvage_capacity row for self → self is appended, then enriched.
        seed_human(
            &mut conn,
            "h-self",
            Some("uhCAk-self"),
            Some("hh-self"),
            APP,
        );

        let cands =
            build_salvage_candidates(&mut conn, "uhCAk-self", APP, &fresh_cutoff()).unwrap();

        assert_eq!(
            hh_of(&cands, "uhCAk-self"),
            Some("hh-self"),
            "the appended self candidate is enriched from humans too"
        );
    }

    /// The `h_app_id` scope filter is LOAD-BEARING: a humans row under one scope is
    /// invisible to a join under another. This is exactly why the slice ships
    /// inert in production — household-bearing rows are written under `"imagodei"`
    /// while this join (mirroring the ingest selector) reads `"lamad"`, so the
    /// production read sees zero household rows (a shared, backlogged dormancy; see
    /// `build_salvage_candidates` rustdoc). The test pins the mechanism so the
    /// scope can't silently drift to `args.app_id` ("elohim") OR be assumed to
    /// match writers without evidence: a row written under scope X is found under X
    /// and NOT under a different scope Y.
    #[test]
    fn h_app_id_scope_filter_is_load_bearing() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        seed_capacity(&mut conn, "uhCAk-a");
        // Seed under one scope; only a join under THAT scope sees the household.
        seed_human(&mut conn, "h-a", Some("uhCAk-a"), Some("hh-1"), "scope-X");

        // Same scope → match.
        let same =
            build_salvage_candidates(&mut conn, "uhCAk-self", "scope-X", &fresh_cutoff()).unwrap();
        assert_eq!(hh_of(&same, "uhCAk-a"), Some("hh-1"));

        // Different scope (the writer/reader split that makes the join dormant in
        // prod, and the "elohim" mistake) → no household.
        for wrong_scope in ["scope-Y", "elohim", "imagodei"] {
            let wrong =
                build_salvage_candidates(&mut conn, "uhCAk-self", wrong_scope, &fresh_cutoff())
                    .unwrap();
            assert!(
                hh_of(&wrong, "uhCAk-a").is_none(),
                "a humans row under scope-X must be invisible to a join under {wrong_scope}"
            );
        }
    }

    // ---- step 6: the knob selects the strategy; safety property holds --------

    /// Seed a two-household, two-peers-each pool (cids + matching humans).
    fn seed_two_households(conn: &mut SqliteConnection) {
        for (cid, hh) in [
            ("uhCAk-a", "hh-1"),
            ("uhCAk-b", "hh-1"),
            ("uhCAk-c", "hh-2"),
            ("uhCAk-d", "hh-2"),
        ] {
            seed_capacity(conn, cid);
            seed_human(conn, &format!("h-{cid}"), Some(cid), Some(hh), APP);
        }
    }

    /// The DISCRIMINATING knob test: a fixture where the XOR-nearest pair
    /// CO-LOCATES in one household, so the diversity strategy MUST diverge from XOR
    /// to spread. Unlike a fixture where XOR already happens to span households,
    /// this test FAILS if `select_placement_strategy(true)` were broken to return
    /// XOR. The household assignment (b,d→hh-1; a,c→hh-2) is derived from the actual
    /// XOR ordering of `"uhblob-X"` over `{a,b,c,d}` (closest-2 = {b,d}); a loud
    /// precondition assert re-verifies that at runtime so the test can't silently
    /// rot if the hash ordering ever shifts.
    #[test]
    fn knob_on_diverges_from_xor_when_xor_would_colocate() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for (cid, hh) in [
            ("uhCAk-a", "hh-2"),
            ("uhCAk-b", "hh-1"),
            ("uhCAk-c", "hh-2"),
            ("uhCAk-d", "hh-1"),
        ] {
            seed_capacity(&mut conn, cid);
            seed_human(&mut conn, &format!("h-{cid}"), Some(cid), Some(hh), APP);
        }
        let cands = build_salvage_candidates(&mut conn, "uhCAk-a", APP, &fresh_cutoff()).unwrap();

        let blob = "uhblob-X";
        let xor = select_placement_strategy(false).rank(blob, &cands, 2);
        let div = select_placement_strategy(true).rank(blob, &cands, 2);

        let hhs_of = |picks: &[String]| -> std::collections::HashSet<String> {
            picks
                .iter()
                .map(|c| {
                    hh_of(&cands, c)
                        .expect("pick carries a household")
                        .to_string()
                })
                .collect()
        };

        // Precondition: with this fixture XOR co-locates (both picks in ONE
        // household). If this ever fails, the hash ordering shifted — re-derive the
        // fixture; do NOT let the test pass without exercising the knob.
        assert_eq!(
            hhs_of(&xor).len(),
            1,
            "fixture precondition: XOR-closest-2 must co-locate in one household \
             (else the knob is untested) — got {xor:?}"
        );

        // The knob's reason for existing: diversity diverges and spreads.
        assert_eq!(
            hhs_of(&div).len(),
            2,
            "knob ON must spread two replicas across both households"
        );
        assert_ne!(
            div, xor,
            "knob ON must diverge from XOR exactly when XOR would co-locate"
        );
    }

    #[test]
    fn knob_off_matches_pure_xor() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        seed_two_households(&mut conn);
        let cands = build_salvage_candidates(&mut conn, "uhCAk-a", APP, &fresh_cutoff()).unwrap();

        let off = select_placement_strategy(false).rank("uhblob-X", &cands, 3);
        let xor = XorDistanceStrategy.rank("uhblob-X", &cands, 3);
        assert_eq!(off, xor, "knob OFF must rank identically to pure XOR");
    }

    #[test]
    fn all_none_household_equals_xor_either_way() {
        // No humans seeded → every candidate household_id=None → the safety
        // property: BOTH knob settings equal pure XOR (never worse than today).
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        for cid in ["uhCAk-a", "uhCAk-b", "uhCAk-c", "uhCAk-d", "uhCAk-e"] {
            seed_capacity(&mut conn, cid);
        }
        let cands = build_salvage_candidates(&mut conn, "uhCAk-a", APP, &fresh_cutoff()).unwrap();
        assert!(
            cands.iter().all(|c| c.household_id.is_none()),
            "no humans → no household enrichment"
        );

        let expected = XorDistanceStrategy.rank("uhblob-Y", &cands, 3);
        assert_eq!(
            select_placement_strategy(true).rank("uhblob-Y", &cands, 3),
            expected,
            "knob ON with no household data must equal XOR"
        );
        assert_eq!(
            select_placement_strategy(false).rank("uhblob-Y", &cands, 3),
            expected,
            "knob OFF must equal XOR"
        );
    }

    // ---- end-to-end: run_salvage_pass composition + the enabled gate ----------

    /// Records the (blob, provider, receiver) tuples a salvage pass authors.
    struct RecordingAuthor {
        authored: std::sync::Mutex<Vec<(String, String, String)>>,
    }
    impl crate::reconcile::custody::CommitmentAuthor for RecordingAuthor {
        fn author_custody_blob(
            &self,
            blob_marker: &str,
            provider: &str,
            receiver: &str,
        ) -> Result<(), StorageError> {
            self.authored.lock().unwrap().push((
                blob_marker.to_string(),
                provider.to_string(),
                receiver.to_string(),
            ));
            Ok(())
        }
    }

    /// Insert an under-replicated custody-blob commitment (one provider hosting).
    fn seed_under_replicated_blob(
        conn: &mut SqliteConnection,
        provider: &str,
        receiver: &str,
        blob: &str,
        now: chrono::DateTime<chrono::Utc>,
    ) {
        use crate::db::diesel_schema::rea_commitments;
        use crate::db::models::NewReaCommitment;
        diesel::insert_into(rea_commitments::table)
            .values(&NewReaCommitment {
                id: "c1",
                h_app_id: "test",
                action: "custody-blob",
                provider,
                receiver,
                resource_conforms_to: None,
                resource_classified_as: Some(blob),
                resource_quantity_value: Some(1024.0),
                resource_quantity_unit: Some("bytes"),
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
                dht_anchor_hash: Some("hash1"),
            })
            .execute(conn)
            .unwrap();
        // Mark the existing provider as freshly hosting → honored = 1 (< target 2).
        let when = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();
        crate::db::peer_blob_inventory::apply_snapshot(
            conn,
            provider,
            std::slice::from_ref(&blob.to_string()),
            1,
            &when,
        )
        .unwrap();
    }

    /// The full wiring: run_salvage_pass builds candidates (with the join), selects
    /// the strategy, runs salvage_pass, and authors via the injected author. With
    /// one provider + target 2, self is among the closest-2 (only {self, provider}
    /// in the pool) → self self-selects and authors. Covers the composition the
    /// unit tests above don't reach, and the bool `enabled` gate.
    #[test]
    fn run_salvage_pass_authors_when_self_selected_and_gates_on_enabled() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "a".repeat(64);
        seed_under_replicated_blob(&mut conn, "uhCAk-prov", "uhCAk-steward", &blob, now);
        // The provider is in the salvage pool; self is appended by build_candidates.
        seed_capacity(&mut conn, "uhCAk-prov");

        let author = RecordingAuthor {
            authored: std::sync::Mutex::new(Vec::new()),
        };

        // enabled=false → the gate short-circuits; nothing authored.
        let gated = super::run_salvage_pass(
            &mut conn,
            "uhCAk-self",
            APP,
            &author,
            false,
            2,
            600,
            true,
            now,
        )
        .unwrap();
        assert_eq!(
            gated.commitments_authored, 0,
            "disabled pass authors nothing"
        );
        assert!(author.authored.lock().unwrap().is_empty());

        // enabled=true → self is among the closest-2 → authors exactly once.
        let outcome = super::run_salvage_pass(
            &mut conn,
            "uhCAk-self",
            APP,
            &author,
            true,
            2,
            600,
            true,
            now,
        )
        .unwrap();
        assert_eq!(
            outcome.commitments_authored, 1,
            "self self-selects and authors"
        );
        let recorded = author.authored.lock().unwrap();
        assert_eq!(
            recorded.as_slice(),
            [(blob, "uhCAk-self".to_string(), "uhCAk-steward".to_string())],
            "authors provider=self, receiver=the content steward"
        );
    }
}
