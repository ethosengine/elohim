//! Household-first resilience computation.
//!
//! For a given content id, aggregates shard_locations + peer_statuses
//! into a `HouseholdResilienceView` that answers the protection claim
//! household-to-household rather than peer-to-peer. The view is computed
//! per-request; no persistence, no new DHT entry types. Source of truth:
//! the upstream DHT entries (Agreement + PeerStatus + NodeRegistration).

use std::collections::HashSet;

use diesel::prelude::*;
use elohim_facings::folds::{replication_commitment, resiliency};
use elohim_facings::relation::HolderRow;

use crate::db::{peer_statuses, placement_gaps, AppContext, DbPool};
use crate::error::StorageError;
use crate::services::household_identity::HouseholdIdentity;
use crate::views::{
    HouseholdResilienceDetails, HouseholdResilienceView, OnlinePeersView, PlacementGapView,
    ResilienceSnapshotDetailsView, ResilienceSnapshotView, StewardingCollectiveEntry,
};

/// Compute per-content household resilience. The viewer's household id is
/// optional — when present, `households_reciprocated` counts mutual
/// stewardship; when absent, it stays zero.
pub fn compute(
    pool: &DbPool,
    ctx: &AppContext,
    content_id: &str,
    viewer_household_id: Option<&str>,
) -> Result<HouseholdResilienceView, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("pool: {e}")))?;
    let (_measured, relation) = load_manifest_and_relation(&mut conn, &ctx.h_app_id, content_id)?;
    // Legacy entry point — age-agnostic (window 0). The staleness honesty guard is
    // applied at the production HTTP entry via `snapshot_with_staleness_secs`.
    compute_base(
        &mut conn,
        &ctx.h_app_id,
        content_id,
        viewer_household_id,
        &relation,
        0,
    )
}

/// Materialize the holder-relation for a content ONCE — the impure step that both
/// `compute` (standalone) and `snapshot` share, so the canonical
/// `shard_locations ⋈ humans ⟕ collectives` join runs a SINGLE time per request
/// (select→fold→aggregate: materialize once, fold many).
///
/// Returns `(measured, relation)` where `measured` is whether a shard manifest
/// exists — the distribution-state tell. No manifest means the content never
/// entered the distribution plane; we do NOT aggregate across all shard_locations
/// (that would inflate counts for orphaned content), so the relation is empty and
/// folds to the degenerate at-risk view. Malformed `shard_hashes_json` is a hard
/// error (mirrors the prior `compute`).
fn load_manifest_and_relation(
    conn: &mut diesel::SqliteConnection,
    h_app_id: &str,
    content_id: &str,
) -> Result<(bool, Vec<HolderRow>), StorageError> {
    let manifest = crate::db::shard_manifests::get_manifest(conn, h_app_id, content_id)?;
    let relation = match &manifest {
        None => vec![],
        Some(m) => {
            let shard_hashes: Vec<String> =
                serde_json::from_str(&m.shard_hashes_json).map_err(|e| {
                    StorageError::Internal(format!(
                        "shard manifest for content_id={content_id} has malformed shard_hashes_json: {e}"
                    ))
                })?;
            load_holder_relation(conn, h_app_id, &shard_hashes)?
        }
    };
    Ok((manifest.is_some(), relation))
}

/// The base resilience reduction over an already-materialized holder-relation.
/// Pure except for the online-peer count (a separate projection read). Shared by
/// `compute` and `snapshot` so the relation is materialized ONCE per request and
/// threaded to every fold. Folding the empty relation yields the degenerate
/// at-risk view, so the no-manifest case needs no special-casing here.
fn compute_base(
    conn: &mut diesel::SqliteConnection,
    h_app_id: &str,
    content_id: &str,
    viewer_household_id: Option<&str>,
    relation: &[HolderRow],
    staleness_secs: i64,
) -> Result<HouseholdResilienceView, StorageError> {
    // Fold distinct non-null households stewarding this content.
    let steward_households: HashSet<String> = resiliency::stewarding_hubs(relation);
    let households_stewarding = steward_households.len() as i32;

    // Reciprocation recorded as zero; reverse allocation traversal is a follow-up.
    let _ = viewer_household_id;
    let households_reciprocated: i32 = 0;

    // Online peer count across the stewarding households (a separate projection).
    // The staleness window (0 = disabled) is injected by the caller; `now` is read
    // once here (per request, not per row).
    let identity = HouseholdIdentity::load(conn)?;
    // The transport's current connected set, read ONCE per request. `None` when
    // no p2p plane armed the registry — every reader then keeps the heartbeat
    // behaviour rather than reading an unpublished set as "nobody is live".
    let connected = crate::services::peer_liveness::connected_snapshot();
    let online_peer_count = count_online_peers_in_households(
        conn,
        &steward_households,
        &identity,
        staleness_secs,
        chrono::Utc::now().timestamp_micros(),
        connected.as_ref(),
    )?;

    // Status classification (a2o spec): protected ← ≥3 households AND ≥2 online;
    // partial ← ≥2 households OR ≥1 online; at-risk otherwise.
    let protection_status = match (households_stewarding, online_peer_count) {
        (n, o) if n >= 3 && o >= 2 => "protected",
        (n, o) if n >= 2 || o >= 1 => "partial",
        _ => "at-risk",
    }
    .to_string();

    let health_score = if households_stewarding == 0 {
        0.0_f32
    } else {
        (online_peer_count as f32 / households_stewarding as f32).clamp(0.0, 1.0)
    };

    // commitment_backed_replication: the per-tier counts + pledged bytes of the
    // notarized `replicates-*` commitments made by the households stewarding this
    // content. `CommitmentBackedReplication` is documented (elohim-views) as the
    // counts "for a CID's authoring household", so the scoping axis is the
    // steward households — NOT the whole node's ledger, and not a per-CID join.
    //
    // The relation is the `rea_commitments` replication relation, which is
    // populated per-commitment by the mishpat→REA mirror
    // (`db::rea_commitments::mirror_replication_commitment`). Before that bridge
    // existed this field was a hard-coded `::default()`, because the live
    // producer wrote only to `mishpat_commitments` (the wrong-table split) —
    // every reader here queries `rea_commitments`.
    //
    // Loader degrades to an empty relation on query failure, so a DB hiccup folds
    // to honest zeros rather than darking the whole card.
    // The commitment relation's `household_id` values come from the same
    // dual-vocabulary `humans` column the holder relation reads, so they get the
    // SAME convergent grouping key — otherwise a fallback-only human's
    // commitments (cid-form household) would silently miss the steward set they
    // back, or vice versa.
    let commitment_relation: Vec<_> =
        crate::db::rea_commitments::load_replication_commitment_relation(conn, h_app_id)
            .into_iter()
            .map(|mut row| {
                row.household_id = identity.group_key_opt(row.household_id);
                row
            })
            .collect();
    let commitment_backed_replication =
        replication_commitment::commitment_backed_replication_for_households(
            &commitment_relation,
            &steward_households,
        );

    let mut steward_households_sorted: Vec<String> = steward_households.into_iter().collect();
    steward_households_sorted.sort();

    Ok(HouseholdResilienceView {
        content_id: content_id.to_string(),
        households_stewarding,
        households_reciprocated,
        protection_status,
        details: HouseholdResilienceDetails {
            steward_households: steward_households_sorted,
            online_peer_count,
            health_score,
        },
        commitment_backed_replication,
    })
}

/// Enriched collective-general resilience snapshot. Builds on `compute()` and
/// adds commitment-backed count, diversity score, regional distribution, and
/// placement gaps. Handler for `/api/v1/resilience/{id}/household`.
///
/// Legacy (age-agnostic) entry point — delegates with a disabled staleness window
/// so existing callers and tests keep their semantics. The production HTTP handler
/// uses [`snapshot_with_staleness_secs`] to apply the liveness honesty guard.
pub fn snapshot(
    pool: &DbPool,
    ctx: &AppContext,
    content_id: &str,
    viewer_household_id: Option<&str>,
) -> Result<ResilienceSnapshotView, StorageError> {
    snapshot_with_staleness_secs(pool, ctx, content_id, viewer_household_id, 0)
}

/// As [`snapshot`], but applies a liveness staleness window (seconds; `0` = the
/// legacy age-agnostic behaviour) to the online-peer count. A fanned-in "online"
/// from a peer whose last DHT snapshot is older than the window does not count
/// toward the protection verdict. The production wiring (`api::resilience`) reads
/// `PEER_STATUS_STALENESS_SECS` (default 900) and injects it here.
pub fn snapshot_with_staleness_secs(
    pool: &DbPool,
    ctx: &AppContext,
    content_id: &str,
    viewer_household_id: Option<&str>,
    staleness_secs: i64,
) -> Result<ResilienceSnapshotView, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    // Materialize the manifest + holder-relation ONCE — every fold below reads
    // this SAME relation (the design's "materialize once, fold many"). Loading on
    // this single connection also removes the prior cross-connection skew between
    // the stewarding count and the intra fold (they read on separate `pool.get()`s).
    let (measured, relation) = load_manifest_and_relation(&mut conn, &ctx.h_app_id, content_id)?;

    let base = compute_base(
        &mut conn,
        &ctx.h_app_id,
        content_id,
        viewer_household_id,
        &relation,
        staleness_secs,
    )?;

    // Distribution-state honesty (2026-06-12 unmeasured≠zero): no manifest means
    // the content never entered the distribution plane — a non-measurement, not a
    // measured zero. Renderers show "not yet distributed", not a fake at-risk verdict.
    let distribution_state = if measured { "measured" } else { "unmeasured" }.to_string();

    // Intra-hub fold over the SAME relation (the operator's "james ↔ matthew ↔
    // jessica = 3"). An empty relation (unmeasured) folds to an empty map.
    let intra_by_hub = resiliency::intra_hub_peers(&relation);

    // commitment_backed_collectives: distinct households with an active provide
    // commitment whose resource_classified_as matches this content's reach.
    use crate::db::diesel_schema::{content, humans, rea_commitments};
    let content_reach: String = content::table
        .filter(content::id.eq(content_id))
        .filter(content::h_app_id.eq(&ctx.h_app_id))
        .select(content::reach)
        .first(&mut conn)
        .unwrap_or_else(|_| "commons".to_string());
    let scope = format!("content:{}", content_reach);

    // The scope membership test is applied in Rust, not SQL: `resource_classified_as`
    // is a JSON list by contract (non-commons spec §11.2, Option A), and a scalar
    // `.eq(&scope)` silently misses an array-wrapped `["content:commons"]` row (U1,
    // 2026-06-19 — the dark card). Load the candidate `(household_id,
    // resource_classified_as)` pairs (small — provide rows per content), keep only
    // those whose classifications contain the scope, and count distinct households.
    let candidate_rows: Vec<(Option<String>, Option<String>)> = rea_commitments::table
        .inner_join(
            humans::table.on(humans::agent_pub_key
                .nullable()
                .eq(rea_commitments::provider.nullable())),
        )
        .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
        // Count provide commitments under BOTH conventions: the seeder writes
        // the REA action literal `provide`; the runtime mishpat projection
        // writes the DHT action discriminator (`replicates-content`, and its
        // migration-window alias `replicates-commons`) for the same economic
        // act (a household committing to hold content). Filtering only
        // `provide` silently dropped every runtime-authored commitment.
        .filter(rea_commitments::action.eq_any([
            "provide",
            "replicates-content",
            "replicates-commons",
        ]))
        .filter(rea_commitments::state.eq("active"))
        .filter(humans::household_id.is_not_null())
        .select((
            humans::household_id,
            rea_commitments::resource_classified_as,
        ))
        .load::<(Option<String>, Option<String>)>(&mut conn)
        .map_err(|e| StorageError::Internal(format!("commitment-backed query: {e}")))?;

    // Distinct-household count under the SAME convergent grouping key as the
    // holder relation (one physical household must not count twice because its
    // members' `humans.household_id` values straddle the slug and cid
    // namespaces).
    let identity = HouseholdIdentity::load(&mut conn)?;
    let commitment_backed_collectives: i32 = candidate_rows
        .into_iter()
        .filter(|(_, classified)| {
            crate::db::rea_commitments::classifications_of(classified.as_deref())
                .iter()
                .any(|c| c == &scope)
        })
        .filter_map(|(household_id, _)| identity.group_key_opt(household_id))
        .collect::<HashSet<String>>()
        .len() as i32;

    // diversity_score: the REAL fault-domain diversity over the SAME holder-relation
    // (replaces the prior ad-hoc coverage proxy that falsely capped the score by the
    // count of commitment records). The fault domain is the household (`hub_id`); the
    // fold also surfaces the region axis for the distribution facing. The scalar score
    // normalizes the distinct household fault-domain count against the RS 4+3 baseline
    // (7). Empty/unmeasured relation → 0 distinct households → 0.0 (honest, never fake).
    let fault_domains = resiliency::fault_domain_diversity(&relation);
    let diversity_score = resiliency::diversity_score(fault_domains.distinct_household_count);

    // regional_distribution: fold the SAME relation by region, relative to the
    // viewer's region (the only extra read is the viewer's own region). Pure +
    // infallible — the relation was already materialized above, so there is no
    // load to fall back from (a load error already returned at materialization).
    let viewer_region: Option<String> = match viewer_household_id {
        None => None,
        Some(vh) => {
            use crate::db::diesel_schema::collectives;
            collectives::table
                .filter(collectives::id.eq(vh))
                .select(collectives::region)
                .first::<Option<String>>(&mut conn)
                .unwrap_or(None)
        }
    };
    let regional_distribution =
        resiliency::regional_distribution(&relation, viewer_region.as_deref());

    // placement_gaps for this content.
    let gap_rows = placement_gaps::list_gaps(
        &mut conn,
        &ctx.h_app_id,
        placement_gaps::GapQuery {
            content_id: Some(content_id.to_string()),
            ..Default::default()
        },
    )?;
    let gaps: Vec<PlacementGapView> = gap_rows.into_iter().map(Into::into).collect();

    // PRESENTATION BOUNDARY (2026-09-12). The fold groups on the convergent DHT
    // `collective_cid`; the wire restores the human-facing slug here so "names,
    // not nines" survives and the a2o footprint judge (which compares
    // `kind:id`) sees the same string from every peer.
    //
    // `id` — the slug any local row anchors to this cid, else the cid form.
    // `label` — resolved by cid first, slug fallback.
    //
    // No wire-shape change: the same two fields, resolved from a convergent key
    // instead of from a per-host routing alias.
    let steward_collective_entries: Vec<StewardingCollectiveEntry> = base
        .details
        .steward_households
        .iter()
        .map(|key| StewardingCollectiveEntry {
            id: identity.display_id(key),
            kind: "household".to_string(),
            label: identity.label(key),
            intra_hub_peers: Some(intra_by_hub.get(key).copied().unwrap_or(0)),
        })
        .collect();

    // Felt projection — the household-addressed re-statement ("names, not nines").
    // Floor-relative + unmeasured-aware. The tier defaults to "standard"/undeclared
    // until the content-declared resilience-tier primitive lands (see
    // genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md);
    // it is deliberately NOT derived from reach (that mapping is the conflation
    // the tier primitive corrects).
    let felt_status = Some(resiliency::build_felt_status(
        &distribution_state,
        &gaps,
        steward_collective_entries.clone(),
        "standard",
        false,
    ));

    // Known denominator: stewarded nodes registered across the stewarding
    // collectives (the D2 join) — "2/3 peers live", never a bare zero.
    // Known peers come from the SAME two-junction, alias-resolved peer set that
    // produced `live` in `compute_base` — one resolution, so the numerator can
    // never exceed or drift from its denominator. `staleness_secs` is irrelevant
    // to `known` (a dark peer is still known), so the window is passed as 0.
    let steward_household_set: HashSet<String> =
        base.details.steward_households.iter().cloned().collect();
    // `known` is liveness-agnostic by definition, so the connected view is not
    // consulted here — a dark peer is still a known peer.
    let known_peer_count: i32 = count_household_peers(
        &mut conn,
        &steward_household_set,
        &identity,
        0,
        chrono::Utc::now().timestamp_micros(),
        None,
    )
    .map(|c| c.known)
    .unwrap_or(0);

    // Floor-relative coverage shortfall, stated from the SAME floor the felt
    // projection above compares against (tier "standard" until the
    // content-declared resilience-tier primitive lands; deliberately NOT derived
    // from reach). Present-vs-absent is the contract, not zero-vs-null: a
    // MEASURED snapshot always states the number (0 ≡ "the floor is met", itself
    // a real measurement), while an UNMEASURED one omits the key — a shortfall
    // computed over a non-measurement would fabricate a measurement. Never a
    // present null. Saturating: extra collectives past the floor read 0, not a
    // negative "surplus".
    let coverage_shortfall: Option<u32> = if distribution_state == "measured" {
        let floor = resiliency::floor_for_tier("standard");
        Some((floor - base.households_stewarding).max(0) as u32)
    } else {
        None
    };

    Ok(ResilienceSnapshotView {
        content_id: base.content_id.clone(),
        distribution_state,
        stewarding_collectives: base.households_stewarding,
        commitment_backed_collectives,
        diversity_score,
        regional_distribution,
        placement_gaps: gaps,
        protection_status: base.protection_status.clone(),
        reciprocating_collectives: Some(base.households_reciprocated),
        details: Some(ResilienceSnapshotDetailsView {
            stewarding_collectives: steward_collective_entries,
            online_peers: OnlinePeersView {
                live: base.details.online_peer_count,
                known: known_peer_count,
            },
            health_score: base.details.health_score,
        }),
        felt_status,
        coverage_shortfall,
        // The commitment-backed replication fold `compute_base` ALREADY ran over
        // the `rea_commitments` replication relation (loaded once per request);
        // surface it on the served snapshot instead of computing-then-dropping it.
        // No second query: this is the same value `HouseholdResilienceView`
        // carries, moved onto the wire shape the HTTP route actually serves.
        commitment_backed_replication: Some(base.commitment_backed_replication),
    })
}

/// Impure loader: one materialized read of the holder-relation for a content —
/// every `(hub, agent, region)` that holds one of the content's shards, joined on
/// the canonical agent identity (`humans.agent_pub_key == shard_locations.peer_id`,
/// both `agent_cid` — never a transport id; see `elohim-storage/CLAUDE.md`
/// Identity & Transport Coherence). This is the SINGLE query the resiliency facing
/// folds over (the pure folds live in `elohim_facings::folds::resiliency`); it
/// replaces the previously-divergent steward join (`compute`) and regional join
/// (`compute_regional_distribution`), and RETAINS the per-agent dimension so
/// intra-hub peer counts are a fold away, not a new query. STAYS in storage
/// because it takes `&mut conn`; returns the crate's `HolderRow`.
///
/// Design: `genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md` §3.
/// Raw join row: (shard_hash, peer_id, household_id, human_id, region).
type HolderJoinRow = (String, String, Option<String>, String, Option<String>);
/// Winning row per (shard_hash, peer_id) after dedup: (human_id, household_id, region).
type BestHolder = (String, Option<String>, Option<String>);

pub(crate) fn load_holder_relation(
    conn: &mut diesel::SqliteConnection,
    h_app_id: &str,
    shard_hashes: &[String],
) -> Result<Vec<HolderRow>, StorageError> {
    use crate::db::diesel_schema::{collectives, humans, shard_locations};

    if shard_hashes.is_empty() {
        return Ok(vec![]);
    }

    // shard_locations → humans (canonical agent join) → collectives (left join;
    // a holder without a collective gets NULL region → unknown bucket).
    let rows: Vec<HolderJoinRow> = shard_locations::table
        .inner_join(
            humans::table.on(humans::agent_pub_key
                .nullable()
                .eq(shard_locations::peer_id.nullable())),
        )
        .left_join(collectives::table.on(collectives::id.nullable().eq(humans::household_id)))
        .filter(shard_locations::h_app_id.eq(h_app_id))
        .filter(shard_locations::shard_hash.eq_any(shard_hashes))
        .select((
            shard_locations::shard_hash,
            shard_locations::peer_id,
            humans::household_id,
            humans::id,
            collectives::region.nullable(),
        ))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("holder relation: {e}")))?;

    // Household-vocabulary normalization onto the CONVERGENT grouping key
    // (2026-09-12 doorway-footprint-convergence; supersedes the 2026-08-22
    // cid→slug canonicalization, which was a no-op on every peer that lacked the
    // slug row carrying the cid).
    //
    // `humans.household_id` is written in TWO namespaces for the SAME physical
    // household: the slug id (`household-dowell`, seeder/membership-signal path)
    // and the DHT-canonical cid (`collective:{action_hash}`, the `identity_fill`
    // CREATE path — it writes the membership `household_cid` verbatim because no
    // slug row exists to pair it with).
    //
    // The prior fix canonicalized cid→slug through the LOCAL `collectives` row,
    // which only works on a peer whose slug row happens to carry the cid. On the
    // household mesh matthew anchored the cid onto `family-dowell` while jessica
    // and james minted a cid-KEYED placeholder (`id == collective_cid`), so the
    // alias map was the identity function there and the two doorways kept telling
    // different truths from identical custody facts.
    //
    // The fold now groups on the DHT `collective_cid` — the column
    // `p2p::projection_reconcile`'s collectives arm actually converges (it returns
    // `InSync` for a cid held under ANY local routing alias). An un-anchored slug
    // stays its own bucket, so two un-anchored households never merge. The
    // human-facing slug is restored at the PRESENTATION boundary
    // (`HouseholdIdentity::display_id`), never here.
    let identity = HouseholdIdentity::load(conn)?;
    let rows: Vec<HolderJoinRow> = rows
        .into_iter()
        .map(|(shard_hash, peer_id, household_id, human_id, region)| {
            let key = identity.group_key_opt(household_id);
            // A cid-form household missed the id-keyed left join, so its region
            // came back NULL; backfill from whichever row anchors the group.
            let region = region.or_else(|| key.as_deref().and_then(|k| identity.region_for(k)));
            (shard_hash, peer_id, key, human_id, region)
        })
        .collect();

    // Dedup against duplicate-`agent_pub_key` inflation. `humans.agent_pub_key` is
    // only non-uniquely indexed (idx_humans_agent_pub_key, NOT UNIQUE), so a
    // CID-keyed fallback human row can coexist with the canonical slug-keyed row
    // (controller.rs tolerates the shape, and membership projection can stamp the
    // SAME key onto both). Left un-deduped, ONE physical shard_location fans out
    // into multiple HolderRows and silently inflates distinct-household /
    // intra-hub / diversity counts for what is physically ONE holder. Collapse to
    // one row per (shard_hash, peer_id) using the SAME last-write-wins-by-humans.id
    // discipline salvage_commitment_author applies (the lexicographically-greatest
    // `id` wins) so the choice is deterministic and consistent across the crate.
    let mut best: std::collections::HashMap<(String, String), BestHolder> =
        std::collections::HashMap::new();
    for (shard_hash, peer_id, household_id, human_id, region) in rows {
        let key = (shard_hash, peer_id);
        match best.get(&key) {
            Some((existing_id, _, _)) if *existing_id >= human_id => {}
            _ => {
                best.insert(key, (human_id, household_id, region));
            }
        }
    }

    let mut out: Vec<HolderRow> = best
        .into_values()
        .map(|(agent_id, hub_id, region)| HolderRow {
            hub_id,
            agent_id,
            region,
        })
        .collect();
    // Deterministic order (HashMap iteration is not). The resiliency folds are all
    // set/sorted-based so order never affects the view, but a stable relation keeps
    // load_holder_relation itself reproducible for any future order-sensitive fold.
    out.sort_by(|a, b| {
        (
            a.hub_id.as_deref(),
            a.agent_id.as_str(),
            a.region.as_deref(),
        )
            .cmp(&(
                b.hub_id.as_deref(),
                b.agent_id.as_str(),
                b.region.as_deref(),
            ))
    });
    Ok(out)
}

/// Count online/degraded peers across the stewarding households, applying an
/// optional liveness staleness window.
///
/// `staleness_secs > 0` excludes any PeerStatus row older than
/// `now_micros - staleness_secs*1e6` (the `timestamp` column is microseconds) —
/// the honesty guard so a fanned-in "online" from a peer that has since gone dark
/// does not count toward the protection verdict forever. `staleness_secs == 0`
/// disables the window (legacy: age-agnostic). `now_micros` is threaded in (not
/// read from the clock here) so the predicate is deterministically unit-testable.
/// `live` / `known` peer counts for a set of stewarding households — computed
/// TOGETHER so the numerator can never drift from its denominator.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct HouseholdPeerCounts {
    /// Peers in these households currently online/degraded within the window.
    pub live: i32,
    /// Peers KNOWN to belong to these households, live or not — the honest
    /// denominator ("2 of 3 peers live", never a bare zero).
    pub known: i32,
}

/// Resolve every peer (agent_cid) locally known to belong to one of
/// `households`, then split it by liveness.
///
/// # Two junctions, not one (2026-09-12)
///
/// Peer↔household membership is recorded in TWO places, and reading only one of
/// them is why the live mesh answered `onlinePeers {live: 0, known: 0}` for every
/// content while reporting `intraHubPeers: 3` from the very same households:
///
/// - `stewarded_nodes.household_id` — the device-registration junction. Empty on
///   the household mesh: nothing had registered devices there.
/// - `humans.agent_pub_key` → `humans.household_id` — the junction the holder
///   relation ALREADY folds over (which is why `intraHubPeers` was truthful).
///
/// Counting only `stewarded_nodes` reported zero live peers for three demonstrably
/// online peers, which is not a measurement — it is a missing join wearing a
/// measurement's clothes. Worse, the chaos drill showed the failure is asymmetric:
/// custody rows outlive a killed peer, so a household kept being told "3 copies"
/// while liveness — the signal that should have degraded the badge — was
/// structurally pinned at 0. Both junctions now feed one deduplicated peer set.
///
/// # Vocabulary
///
/// `households` carries CONVERGENT group keys (a `collective:` cid wherever one is
/// anchored) while both junctions store whichever LOCAL vocabulary their writer
/// saw. `stewarded_nodes` is therefore queried through
/// [`HouseholdIdentity::aliases_of`], and the `humans` side is folded through
/// [`HouseholdIdentity::group_key`] — the same relabelling from both directions.
///
/// Neither junction is `h_app_id`-filtered, matching
/// `peer_statuses::list_by_household` and `load_holder_relation` (a scope filter
/// here would ship a silent no-op under the qahal/lamad ctx drift class — the
/// same reasoning the identity resolver's `collectives` read records).
/// # Two liveness sources, in priority order (2026-09-12)
///
/// `connected` is the local transport's CURRENT connected-peer view
/// ([`crate::services::peer_liveness`]), or `None` when no transport plane armed
/// it. When present it DECIDES `live`: a household peer is live iff this node
/// can see it right now, under either label namespace. When absent the count
/// falls back to the `peer_statuses` heartbeat window, which is what this
/// function has always done.
///
/// The fallback is not a nicety. The heartbeat cannot go down on its own: a
/// SIGKILLed peer writes no offline row, so its last beat stays inside the 900 s
/// window and the card reported three live copies of content whose holders were
/// already dead (blob-durability DELTA 2026-09-12c, cause 2). `known` is
/// unaffected either way — a dark peer is still a known peer, and "1 of 3" is
/// the honest reading.
fn count_household_peers(
    conn: &mut diesel::SqliteConnection,
    households: &HashSet<String>,
    identity: &HouseholdIdentity,
    staleness_secs: i64,
    now_micros: i64,
    connected: Option<&HashSet<String>>,
) -> Result<HouseholdPeerCounts, StorageError> {
    use crate::db::diesel_schema::{humans, stewarded_nodes};

    if households.is_empty() {
        return Ok(HouseholdPeerCounts::default());
    }

    // Junction 1 — registered devices, keyed on whichever local alias the writer used.
    let alias_ids: Vec<String> = households
        .iter()
        .flat_map(|key| identity.aliases_of(key))
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect();
    let mut peers: HashSet<String> = stewarded_nodes::table
        .filter(stewarded_nodes::household_id.is_not_null())
        .filter(
            stewarded_nodes::household_id
                .assume_not_null()
                .eq_any(&alias_ids),
        )
        .select(stewarded_nodes::id)
        .load::<String>(conn)
        .map_err(|e| StorageError::Internal(format!("stewarded_nodes by household: {e}")))?
        .into_iter()
        .collect();

    // Junction 2 — the member rows the holder relation already trusts.
    let member_rows: Vec<(Option<String>, Option<String>)> = humans::table
        .filter(humans::agent_pub_key.is_not_null())
        .filter(humans::household_id.is_not_null())
        .select((humans::agent_pub_key, humans::household_id))
        .load(conn)
        .map_err(|e| StorageError::Internal(format!("humans by household: {e}")))?;
    for (agent_pub_key, household_id) in member_rows {
        let (Some(key), Some(hh)) = (agent_pub_key, household_id) else {
            continue;
        };
        if households.contains(&identity.group_key(&hh)) {
            peers.insert(key);
        }
    }

    if peers.is_empty() {
        return Ok(HouseholdPeerCounts::default());
    }
    let known = peers.len() as i32;

    // Transport-observed liveness wins when it exists. Set intersection, no
    // window: the transport's set IS the measurement, and it is already bounded
    // by the ping budget on the publishing side.
    if let Some(connected) = connected {
        let live = peers.iter().filter(|p| connected.contains(*p)).count() as i32;
        return Ok(HouseholdPeerCounts { live, known });
    }

    // `i64::MIN` cutoff makes the age test a no-op when the window is disabled.
    let cutoff_micros: i64 = if staleness_secs > 0 {
        now_micros.saturating_sub(staleness_secs.saturating_mul(1_000_000))
    } else {
        i64::MIN
    };
    let peer_ids: Vec<String> = peers.into_iter().collect();
    let live = peer_statuses::list_by_peer_ids(conn, &peer_ids)
        .map_err(|e| StorageError::Internal(format!("peer_statuses by peer ids: {e}")))?
        .into_iter()
        .filter(|row| {
            matches!(row.status.as_str(), "online" | "degraded") && row.timestamp >= cutoff_micros
        })
        .count() as i32;

    Ok(HouseholdPeerCounts { live, known })
}

/// Live-peer count alone — the reduction `compute_base` needs.
fn count_online_peers_in_households(
    conn: &mut diesel::SqliteConnection,
    households: &HashSet<String>,
    identity: &HouseholdIdentity,
    staleness_secs: i64,
    now_micros: i64,
    connected: Option<&HashSet<String>>,
) -> Result<i32, StorageError> {
    Ok(count_household_peers(
        conn,
        households,
        identity,
        staleness_secs,
        now_micros,
        connected,
    )?
    .live)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewStewardedNode;
    use crate::db::peer_statuses::PeerStatusRow;
    use crate::db::run_migrations;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:household_staleness_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// Seed a stewarded node (id == peer_id, joined by `list_by_household`) plus an
    /// online peer_status at `ts_micros`, both in `household`.
    fn seed_online_peer(
        conn: &mut SqliteConnection,
        peer_id: &str,
        household: &str,
        ts_micros: i64,
    ) {
        diesel::insert_into(crate::db::diesel_schema::stewarded_nodes::table)
            .values(&NewStewardedNode {
                id: peer_id.into(),
                display_name: peer_id.into(),
                claim_status: "claimed".into(),
                cpu_cores: 1,
                memory_gb: 1,
                storage_tb: 0.1,
                bandwidth_mbps: 10,
                steward_tier: "household".into(),
                custodian_opt_in: 0,
                region: None,
                context_epr_id: None,
                dht_anchor_hash: None,
                h_app_id: "lamad".into(),
                device_archetype_id: None,
                household_id: Some(household.to_string()),
                hostname: None,
                node_role: None,
                capability_level: None,
                can_steward: 1,
                can_infer: 0,
                can_doorway: 0,
                signature: None,
                signed_at: None,
            })
            .execute(conn)
            .expect("seed stewarded node");

        peer_statuses::upsert(
            conn,
            &PeerStatusRow {
                peer_id: peer_id.into(),
                status: "online".into(),
                general_pool_member: 1,
                accepting_stewardship_reserves: 1,
                archetype_class: None,
                timestamp: ts_micros,
                dht_anchor_hash: String::new(),
                updated_at: ts_micros,
            },
        )
        .expect("seed peer_status");
    }

    /// Two humans of the SAME physical household, one recorded under the slug
    /// vocabulary (`household-dowell`, seeder/membership-signal path) and one
    /// under the DHT-cid vocabulary (`collective:{action_hash}`, the
    /// `identity_fill` CREATE path), must fold to ONE steward household when the
    /// local collectives projection declares the alias
    /// (`collectives.collective_cid`). This is the 2026-08-22 card-tells-truth
    /// divergence: doorway A (all-slug humans) said `stewardingCollectives: 1`
    /// while doorway B (mixed-vocabulary humans) said 2 for identical custody
    /// facts. Also asserts the alias target's region backfills the cid-form
    /// holder (it missed the id-keyed left join).
    #[test]
    fn holder_relation_canonicalizes_collective_cid_household_aliases() {
        use crate::db::diesel_schema::{collectives, humans, shard_locations};
        use crate::db::models::{NewHuman, NewShardLocation};

        let pool = test_pool();
        let mut conn = pool.get().unwrap();

        diesel::insert_into(collectives::table)
            .values((
                collectives::id.eq("household-dowell"),
                collectives::h_app_id.eq("lamad"),
                collectives::name.eq("Dowell Household"),
                collectives::governance_layer.eq("family"),
                collectives::reach.eq("trusted"),
                collectives::created_at.eq("2026-08-22 00:00:00"),
                collectives::updated_at.eq("2026-08-22 00:00:00"),
                collectives::region.eq("tech-valley"),
                collectives::collective_cid.eq("collective:uhCkkALIAS"),
            ))
            .execute(&mut *conn)
            .expect("seed collective");

        for human in [
            // Slug-vocabulary row (canonical seeded human).
            NewHuman {
                id: "human-jessica-spouse".into(),
                agent_pub_key: Some("uhCAkJESSICA".into()),
                display_name: "Jessica".into(),
                bio: None,
                affinities: "[]".into(),
                profile_reach: "commons".into(),
                location: None,
                profile_photo_url: None,
                h_app_id: "imagodei".into(),
                household_id: Some("household-dowell".into()),
            },
            // DHT-cid-vocabulary row (identity_fill CREATE shape: id = member_cid,
            // household_id = the membership household_cid, verbatim).
            NewHuman {
                id: "agent:uhCAkMATTHEW".into(),
                agent_pub_key: Some("uhCAkMATTHEW".into()),
                display_name: "agent:uhCAkMATTHEW".into(),
                bio: None,
                affinities: "[]".into(),
                profile_reach: "commons".into(),
                location: None,
                profile_photo_url: None,
                h_app_id: "imagodei".into(),
                household_id: Some("collective:uhCkkALIAS".into()),
            },
        ] {
            diesel::insert_into(humans::table)
                .values(&human)
                .execute(&mut *conn)
                .expect("seed human");
        }

        for peer in ["uhCAkJESSICA", "uhCAkMATTHEW"] {
            diesel::insert_into(shard_locations::table)
                .values(&NewShardLocation {
                    shard_hash: "sha256-shard-1",
                    peer_id: peer,
                    h_app_id: "lamad",
                    status: "confirmed",
                })
                .execute(&mut *conn)
                .expect("seed shard location");
        }

        let relation = load_holder_relation(&mut conn, "lamad", &["sha256-shard-1".to_string()])
            .expect("holder relation");

        let relation_identity = HouseholdIdentity::load(&mut conn).expect("identity");

        assert_eq!(relation.len(), 2, "both holders survive");
        for row in &relation {
            assert_eq!(
                row.hub_id.as_deref(),
                Some("collective:uhCkkALIAS"),
                "both vocabularies group on the CONVERGENT DHT collective_cid \
                 (2026-09-12; the grouping key moved off the per-host routing alias)"
            );
            assert_eq!(
                row.region.as_deref(),
                Some("tech-valley"),
                "the anchored row's region backfills the cid-form holder"
            );
        }
        assert_eq!(
            resiliency::stewarding_hubs(&relation).len(),
            1,
            "one physical household folds to ONE steward household across vocabularies"
        );
        // The presentation boundary restores the human-facing slug, so the felt
        // surface still reads "names, not nines".
        assert_eq!(
            relation_identity.display_id("collective:uhCkkALIAS"),
            "household-dowell",
            "the wire still presents the slug, never the raw cid, when one is anchored"
        );
    }

    /// A 900s window counts a peer heartbeated 60s ago and excludes one 901s stale
    /// — the household-resilience half of the same honesty guard. Self rows
    /// heartbeat every 60s, so a 900s window never dims a live pod.
    #[test]
    fn count_online_applies_staleness_window() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = chrono::Utc::now().timestamp_micros();
        seed_online_peer(&mut conn, "uhCAkFRESH", "hh-a", now - 60 * 1_000_000);
        seed_online_peer(&mut conn, "uhCAkSTALE", "hh-a", now - 901 * 1_000_000);

        let households: HashSet<String> = HashSet::from(["hh-a".to_string()]);
        let identity = HouseholdIdentity::load(&mut conn).expect("identity");

        // Window 900: only the fresh peer counts.
        assert_eq!(
            count_online_peers_in_households(&mut conn, &households, &identity, 900, now, None)
                .unwrap(),
            1,
            "the 901s-stale peer is excluded under a 900s window"
        );

        // Window 0: legacy age-agnostic — both count.
        assert_eq!(
            count_online_peers_in_households(&mut conn, &households, &identity, 0, now, None)
                .unwrap(),
            2,
            "both peers count when the window is disabled"
        );
    }

    /// THE 2026-09-12c cause-2 regression: liveness comes from the local
    /// transport's CONNECTED set, not from a heartbeat window a killed peer can
    /// never move.
    ///
    /// Household `{a, b, c}`, all three heartbeating one minute ago — under the
    /// old reading all three are "live" forever, which is exactly what told a
    /// household it had three copies of "manifesto" after two of its peers had
    /// been SIGKILLed. With the transport seeing only `{a, b}`, `live` is 2 and
    /// `known` stays 3: "2 of 3", never a bare zero and never a comforting lie.
    #[test]
    fn live_follows_the_connected_set_while_known_follows_the_junction() {
        let pool = test_pool();
        let mut conn = pool.get().unwrap();
        let now = chrono::Utc::now().timestamp_micros();
        // All three beat 60s ago — well inside the 900s window.
        for peer in ["uhCAkA", "uhCAkB", "uhCAkC"] {
            seed_online_peer(&mut conn, peer, "hh-a", now - 60 * 1_000_000);
        }

        let households: HashSet<String> = HashSet::from(["hh-a".to_string()]);
        let identity = HouseholdIdentity::load(&mut conn).expect("identity");

        // Heartbeat-only: the old reading. Three live, because a dead peer
        // cannot write its own obituary.
        let heartbeat =
            count_household_peers(&mut conn, &households, &identity, 900, now, None).unwrap();
        assert_eq!(
            (heartbeat.live, heartbeat.known),
            (3, 3),
            "the heartbeat window alone cannot see the kill — this is the red"
        );

        // Transport-observed: c is gone.
        let connected: HashSet<String> =
            HashSet::from(["uhCAkA".to_string(), "uhCAkB".to_string()]);
        let observed = count_household_peers(
            &mut conn,
            &households,
            &identity,
            900,
            now,
            Some(&connected),
        )
        .unwrap();
        assert_eq!(
            observed.live, 2,
            "live must follow the peers this node can actually reach right now"
        );
        assert_eq!(
            observed.known, 3,
            "known is liveness-agnostic — the third peer is still a household member, \
             so the card reads `2 of 3` rather than a bare zero"
        );

        // And an EMPTY connected set is a real measurement, not a fallback: a
        // node that can reach nobody must say so.
        let alone = count_household_peers(
            &mut conn,
            &households,
            &identity,
            900,
            now,
            Some(&HashSet::new()),
        )
        .unwrap();
        assert_eq!((alone.live, alone.known), (0, 3));
    }
}
