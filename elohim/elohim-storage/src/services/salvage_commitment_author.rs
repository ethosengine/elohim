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

use std::sync::atomic::{AtomicU64, Ordering};
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

    /// Resolve the node's own canonical `agent_cid` (`uhCAk…`) to WRITE as the
    /// salvage `provider`, mirroring the provide-loop's candidate order
    /// (`conductor_commitment_author::resolve_provider`): (a) the active local
    /// session's `agent_pub_key`, then (b) this pod's own conductor cell key
    /// (`HcClient::agent_key_uhcak`, the truthful `uhCAk…` source
    /// `genesis_self_heal` fills the humans row from). Returns `None` when
    /// NEITHER is a valid `agent_cid` — the caller then SKIPS authoring this tick
    /// rather than writing a transport id (`12D3Koo…` / iroh NodeId) that could
    /// never join `humans.agent_pub_key = rea_commitments.provider`.
    pub fn resolve_self_agent_cid(&self, conn: &mut diesel::SqliteConnection) -> Option<String> {
        resolve_self_agent_cid(conn, &self.hc)
    }
}

/// Free-function form of [`SalvageCommitmentAuthor::resolve_self_agent_cid`] so
/// callers holding an [`HcClient`] + connection (boot-time P2P wiring) resolve
/// the same way the per-tick salvage author does. See that method for the
/// candidate order + the never-write-a-transport-id contract.
pub fn resolve_self_agent_cid(
    conn: &mut diesel::SqliteConnection,
    hc: &HcClient,
) -> Option<String> {
    let session_cid = crate::db::local_sessions::get_active_session(conn)
        .ok()
        .flatten()
        .map(|s| s.agent_pub_key);
    let cell_key = hc.agent_key_uhcak();
    crate::identity_namespace::resolve_agent_cid_write(&[
        session_cid.as_deref(),
        Some(cell_key.as_str()),
    ])
}

/// Process-wide count of salvage authoring ticks skipped because no `agent_cid`
/// self-provider was resolvable. Monotonic for process lifetime (mirrors
/// `conductor_commitment_author::PROVIDER_UNRESOLVED_SKIPS`). Read by
/// [`self_unresolved_skip_count`] (unit tests + introspection).
static SALVAGE_SELF_UNRESOLVED_SKIPS: AtomicU64 = AtomicU64::new(0);

/// Read the running total of salvage ticks skipped for an unresolvable
/// `agent_cid` self-provider. Monotonic; reset only by restart.
pub fn self_unresolved_skip_count() -> u64 {
    SALVAGE_SELF_UNRESOLVED_SKIPS.load(Ordering::Relaxed)
}

/// Record one skipped salvage tick (no `agent_cid` self-provider resolvable) and
/// emit a rate-sane signal: the process-wide counter + the scrapeable metric bump
/// on every skip, but the WARN fires only on the FIRST occurrence per process
/// (the salvage tick retries on a timer, so an unresolved-identity node would
/// otherwise WARN-storm). Subsequent skips log at DEBUG. Mirrors
/// `conductor_commitment_author::record_provider_unresolved_skip`. Returns the new
/// running total (for tests).
fn record_self_unresolved_skip(self_cid: &str) -> u64 {
    let prior = SALVAGE_SELF_UNRESOLVED_SKIPS.fetch_add(1, Ordering::Relaxed);
    crate::metrics::inc_salvage_provider_unresolved();
    if prior == 0 {
        tracing::warn!(
            target: "identity_coherence",
            counter = "elohim_salvage_provider_unresolved_total",
            self_cid = %self_cid,
            "salvage author: no agent_cid self-provider resolvable (session + own cell key \
             both non-agent_cid) — SKIPPING author this tick rather than writing a transport-id \
             provider that could never join the resilience card (rate-sane: first occurrence)"
        );
    } else {
        tracing::debug!(
            target: "identity_coherence",
            counter = "elohim_salvage_provider_unresolved_total",
            self_cid = %self_cid,
            skips = prior + 1,
            "salvage author: agent_cid self-provider still unresolvable — skip (throttled)"
        );
    }
    prior + 1
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
/// `salvage_capacity.agent_cid == humans.agent_pub_key` scoped to the canonical humans scope, and
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
/// **Scope (RESOLVED).** The join filters humans by the canonical
/// [`crate::db::context::HUMANS_HAPP_ID`] (`"imagodei"`, the identity pillar) — the
/// same scope production writers use (`api/identity.rs`, `services/genesis_self_heal.rs`).
/// The earlier `imagodei`-write / `lamad`-read scope split that emptied this join is
/// reconciled (humans-projection scope reconciliation; see the const doc).
///
/// **Remaining dormancy (NOT fixed here).** A candidate can still stay `None` for two
/// reasons: (1) NULL `agent_pub_key` humans (the dormant-human gate — per-pod
/// registration / the humans-replayer arc); (2) a transport-id vs `agent_cid`
/// namespace mismatch on `self_cid` / `salvage_capacity.agent_cid` unless `SELF_CID`
/// pins the agent key (the blocked transport-identity resolver). Both shared with
/// ingest + the resilience card, tracked in
/// `genesis/data/timeline/backlog/resilience-card-membership-humans-projection-gap-2026-06-19.md`.
fn build_salvage_candidates(
    conn: &mut diesel::SqliteConnection,
    self_write_cid: &str,
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
    // round-tripped back through gossip (first-tick convergence). Self is
    // identified by its resolved `agent_cid` (NOT a transport `self_cid`) so the
    // candidate pool, the humans household-join (`agent_pub_key = agent_cid`), the
    // placement rank-match, and the authored `provider` all key on ONE namespace.
    if !candidates.iter().any(|c| c.agent_cid == self_write_cid) {
        candidates.push(PlacementCandidate::from_agent_cid(
            self_write_cid.to_string(),
        ));
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
        .filter(humans::h_app_id.eq(crate::db::context::HUMANS_HAPP_ID))
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
/// The humans join is scoped to the canonical [`crate::db::context::HUMANS_HAPP_ID`]
/// (`"imagodei"`); the writer/reader scope split that previously emptied it is
/// reconciled (it is no longer a parameter). **Honest ceiling:** that resolves only
/// ONE of the dormancy gates. The join can still return no households until (a)
/// humans rows have a populated `agent_pub_key` (per-pod registration / the
/// humans-replayer) AND (b) `self_cid` / `salvage_capacity.agent_cid` are in the
/// `agent_cid` namespace (`SELF_CID` / the blocked transport-identity resolver).
/// Until both clear, the strategy degrades to XOR. Shared with the ingest selector
/// and the resilience card; tracked in
/// `resilience-card-membership-humans-projection-gap-2026-06-19.md`.
///
/// Lives here (not in `P2PNode`) because the production [`SalvageCommitmentAuthor`]
/// needs the conductor handle, which is threaded in the reconcile task — the same
/// place [`crate::services::conductor_commitment_author::ConductorCommitmentAuthor`]
/// is wired for the provide loop. Returns the
/// [`crate::reconcile::custody::SalvageOutcome`] for logging/metrics.
/// ## Self-identity (namespace-coherent authoring)
///
/// `self_cid` is the node's TRANSPORT id (libp2p `12D3Koo…` / iroh NodeId, from
/// `Config::self_cid`) and is used ONLY for read-side matching against existing
/// rows (legacy rows this node authored before this resolver landed carry the
/// transport id as `provider`). `self_agent_cid` is the resolved holochain
/// `agent_cid` (`uhCAk…`) and is the ONLY identity ever WRITTEN — into the
/// candidate pool, the placement rank-match, and the authored `provider`. When
/// `self_agent_cid` is `None` (or somehow not an `agent_cid`) the pass SKIPS all
/// authoring this tick (records a rate-sane skip signal) rather than writing a
/// transport id that could never join `humans.agent_pub_key =
/// rea_commitments.provider`. This is the salvage half of the CID-hardening the
/// provide loop (`conductor_commitment_author`) already applies.
#[allow(clippy::too_many_arguments)]
pub fn run_salvage_pass(
    conn: &mut diesel::SqliteConnection,
    self_cid: &str,
    self_agent_cid: Option<&str>,
    author: &dyn CommitmentAuthor,
    enabled: bool,
    target_replicas: usize,
    inventory_freshness_seconds: u64,
    diversity_placement: bool,
    now: chrono::DateTime<chrono::Utc>,
) -> Result<crate::reconcile::custody::SalvageOutcome, StorageError> {
    use crate::reconcile::custody::{salvage_pass, SalvageConfig};

    // Resolve the WRITE identity. Never mint a transport-id `provider`: if no
    // truthful agent_cid is available, skip authoring this tick (rate-sane
    // signal) and let a later tick retry once the conductor cell key / a session
    // is present. Reconcile still moves bytes for existing rows meanwhile.
    let self_write_cid = match self_agent_cid {
        Some(cid) if crate::identity_namespace::is_agent_cid(cid) => cid,
        _ => {
            if enabled {
                record_self_unresolved_skip(self_cid);
            }
            return Ok(crate::reconcile::custody::SalvageOutcome::default());
        }
    };

    // Regression tripwire (never rejects): `self_write_cid` is guaranteed an
    // agent_cid here, so this stays silent on the correct path — but it would flag
    // any future edit that reintroduced a non-agent_cid provider write.
    crate::identity_namespace::observe_agent_cid_write(
        "rea_commitments.provider",
        Some(self_write_cid),
    );

    let fresh_after = (now - chrono::Duration::seconds(inventory_freshness_seconds as i64))
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string();

    let candidates = build_salvage_candidates(conn, self_write_cid, &fresh_after)?;

    let strategy = select_placement_strategy(diversity_placement);
    let cfg = SalvageConfig {
        enabled,
        target_replicas,
        inventory_freshness_seconds,
    };
    salvage_pass(
        conn,
        self_cid,
        self_write_cid,
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

    // The canonical humans-projection scope (imagodei). `seed_human(.., APP)` seeds
    // rows the join (now hardcoded to this scope) will match. Content commitments
    // stay operating-scoped ("lamad"); only humans live under imagodei.
    const APP: &str = crate::db::context::HUMANS_HAPP_ID;

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

        let cands = build_salvage_candidates(&mut conn, "uhCAk-self", &fresh_cutoff()).unwrap();

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

        let cands = build_salvage_candidates(&mut conn, "uhCAk-self", &fresh_cutoff()).unwrap();

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

        let cands = build_salvage_candidates(&mut conn, "uhCAk-self", &fresh_cutoff()).unwrap();

        assert_eq!(
            hh_of(&cands, "uhCAk-self"),
            Some("hh-self"),
            "the appended self candidate is enriched from humans too"
        );
    }

    /// The humans-join scope is the canonical [`crate::db::context::HUMANS_HAPP_ID`]
    /// (`"imagodei"`), NOT the operating content scope, and is no longer a parameter.
    /// A humans row under the wrong (content) scope is invisible to the join; only a
    /// row under the canonical scope matches. This pins the reconciliation so the
    /// scope cannot silently drift (replaces the old param-driven scope test).
    #[test]
    fn humans_join_uses_canonical_imagodei_scope_only() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        seed_capacity(&mut conn, "uhCAk-a");
        // A human under the WRONG (operating/content) scope must NOT be seen.
        seed_human(
            &mut conn,
            "h-lamad",
            Some("uhCAk-a"),
            Some("hh-wrong"),
            "lamad",
        );
        let cands = build_salvage_candidates(&mut conn, "uhCAk-self", &fresh_cutoff()).unwrap();
        assert!(
            hh_of(&cands, "uhCAk-a").is_none(),
            "lamad-scoped human must not match the canonical imagodei join"
        );

        // Only the imagodei (canonical) row is matched.
        seed_human(
            &mut conn,
            "h-imagodei",
            Some("uhCAk-a"),
            Some("hh-1"),
            crate::db::context::HUMANS_HAPP_ID,
        );
        let cands = build_salvage_candidates(&mut conn, "uhCAk-self", &fresh_cutoff()).unwrap();
        assert_eq!(
            hh_of(&cands, "uhCAk-a"),
            Some("hh-1"),
            "imagodei-scoped human matches"
        );
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
        let cands = build_salvage_candidates(&mut conn, "uhCAk-a", &fresh_cutoff()).unwrap();

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
        let cands = build_salvage_candidates(&mut conn, "uhCAk-a", &fresh_cutoff()).unwrap();

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
        let cands = build_salvage_candidates(&mut conn, "uhCAk-a", &fresh_cutoff()).unwrap();
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

        // enabled=false → the gate short-circuits; nothing authored. Self resolves
        // to its agent_cid (Some) so the gate — not the identity guard — is what
        // suppresses authoring.
        let gated = super::run_salvage_pass(
            &mut conn,
            "uhCAk-self",
            Some("uhCAk-self"),
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
            Some("uhCAk-self"),
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

    /// Identity guard: when self's agent_cid is UNRESOLVABLE (None), an opted-in
    /// salvage tick authors NOTHING and records a skip signal — it must NEVER
    /// write a transport-id provider. This is the core hole this task closes.
    #[test]
    fn run_salvage_pass_skips_authoring_when_agent_cid_unresolvable() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "a".repeat(64);
        // A transport-id self_cid (the production hazard): self_cid is a libp2p id
        // but no agent_cid is resolvable.
        seed_under_replicated_blob(&mut conn, "uhCAk-prov", "uhCAk-steward", &blob, now);
        seed_capacity(&mut conn, "uhCAk-prov");

        let author = RecordingAuthor {
            authored: std::sync::Mutex::new(Vec::new()),
        };

        let before = super::self_unresolved_skip_count();
        let outcome = super::run_salvage_pass(
            &mut conn,
            "12D3KooTransportSelfId", // transport id — must NEVER be written as provider
            None,                     // agent_cid unresolvable → SKIP authoring
            &author,
            true, // opted in
            2,
            600,
            true,
            now,
        )
        .unwrap();

        assert_eq!(
            outcome.commitments_authored, 0,
            "unresolvable agent_cid must author nothing"
        );
        assert!(
            author.authored.lock().unwrap().is_empty(),
            "no transport-id provider row may be written"
        );
        assert_eq!(
            super::self_unresolved_skip_count(),
            before + 1,
            "the skip must be recorded (rate-sane visibility signal)"
        );
    }

    /// A non-agent_cid `self_agent_cid` (defensive: a caller passing a stray
    /// transport id where an agent_cid was expected) is treated as unresolvable —
    /// the is_agent_cid gate refuses it, nothing is authored.
    #[test]
    fn run_salvage_pass_rejects_non_agent_cid_write_identity() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = chrono::Utc::now();
        let blob = "a".repeat(64);
        seed_under_replicated_blob(&mut conn, "uhCAk-prov", "uhCAk-steward", &blob, now);
        seed_capacity(&mut conn, "uhCAk-prov");

        let author = RecordingAuthor {
            authored: std::sync::Mutex::new(Vec::new()),
        };

        let outcome = super::run_salvage_pass(
            &mut conn,
            "12D3KooTransportSelfId",
            Some("12D3KooNotAnAgentCid"), // not uhCAk… → refused by the gate
            &author,
            true,
            2,
            600,
            true,
            now,
        )
        .unwrap();

        assert_eq!(outcome.commitments_authored, 0);
        assert!(author.authored.lock().unwrap().is_empty());
    }
}
