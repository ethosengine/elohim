//! Household-first resilience computation.
//!
//! For a given content id, aggregates shard_locations + peer_statuses
//! into a `HouseholdResilienceView` that answers the protection claim
//! household-to-household rather than peer-to-peer. The view is computed
//! per-request; no persistence, no new DHT entry types. Source of truth:
//! the upstream DHT entries (Agreement + PeerStatus + NodeRegistration).

use std::collections::HashSet;

use diesel::prelude::*;
use elohim_facings::folds::resiliency;
use elohim_facings::relation::HolderRow;

use crate::db::{peer_statuses, placement_gaps, AppContext, DbPool};
use crate::error::StorageError;
use crate::views::{
    CommitmentBackedReplication, HouseholdResilienceDetails, HouseholdResilienceView,
    OnlinePeersView, PlacementGapView, ResilienceSnapshotDetailsView, ResilienceSnapshotView,
    StewardingCollectiveEntry,
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
    compute_base(&mut conn, content_id, viewer_household_id, &relation)
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
    content_id: &str,
    viewer_household_id: Option<&str>,
    relation: &[HolderRow],
) -> Result<HouseholdResilienceView, StorageError> {
    // Fold distinct non-null households stewarding this content.
    let steward_households: HashSet<String> = resiliency::stewarding_hubs(relation);
    let households_stewarding = steward_households.len() as i32;

    // Reciprocation recorded as zero; reverse allocation traversal is a follow-up.
    let _ = viewer_household_id;
    let households_reciprocated: i32 = 0;

    // Online peer count across the stewarding households (a separate projection).
    let online_peer_count = count_online_peers_in_households(conn, &steward_households)?;

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
        commitment_backed_replication: CommitmentBackedReplication::default(), // T15: computed
    })
}

/// Enriched collective-general resilience snapshot. Builds on `compute()` and
/// adds commitment-backed count, diversity score, regional distribution, and
/// placement gaps. Handler for `/api/v1/resilience/{id}/household`.
pub fn snapshot(
    pool: &DbPool,
    ctx: &AppContext,
    content_id: &str,
    viewer_household_id: Option<&str>,
) -> Result<ResilienceSnapshotView, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    // Materialize the manifest + holder-relation ONCE — every fold below reads
    // this SAME relation (the design's "materialize once, fold many"). Loading on
    // this single connection also removes the prior cross-connection skew between
    // the stewarding count and the intra fold (they read on separate `pool.get()`s).
    let (measured, relation) = load_manifest_and_relation(&mut conn, &ctx.h_app_id, content_id)?;

    let base = compute_base(&mut conn, content_id, viewer_household_id, &relation)?;

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

    let commitment_backed_collectives: i32 = candidate_rows
        .into_iter()
        .filter(|(_, classified)| {
            crate::db::rea_commitments::classifications_of(classified.as_deref())
                .iter()
                .any(|c| c == &scope)
        })
        .filter_map(|(household_id, _)| household_id)
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

    // Map steward_households (Vec<String> of ids) → Vec<StewardingCollectiveEntry>,
    // enriching with the collective's display name (collectives.name) so the felt
    // surface can render names, not nines. `kind` stays "household": the stewards
    // are sourced from humans.household_id (all households today); multi-kind
    // derivation is a captured follow-on (resilience-tier-content-declared-floor).
    let collective_labels: std::collections::HashMap<String, String> =
        if base.details.steward_households.is_empty() {
            std::collections::HashMap::new()
        } else {
            use crate::db::diesel_schema::collectives;
            collectives::table
                .filter(collectives::h_app_id.eq(&ctx.h_app_id))
                .filter(collectives::id.eq_any(&base.details.steward_households))
                .select((collectives::id, collectives::name))
                .load::<(String, String)>(&mut conn)
                .unwrap_or_default()
                .into_iter()
                .collect()
        };

    let steward_collective_entries: Vec<StewardingCollectiveEntry> = base
        .details
        .steward_households
        .iter()
        .map(|id| StewardingCollectiveEntry {
            id: id.clone(),
            kind: "household".to_string(),
            label: collective_labels.get(id).cloned(),
            intra_hub_peers: Some(intra_by_hub.get(id).copied().unwrap_or(0)),
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
    let known_peer_count: i32 = {
        use crate::db::diesel_schema::stewarded_nodes;
        if base.details.steward_households.is_empty() {
            0
        } else {
            stewarded_nodes::table
                .filter(stewarded_nodes::h_app_id.eq(&ctx.h_app_id))
                .filter(stewarded_nodes::household_id.is_not_null())
                .filter(
                    stewarded_nodes::household_id
                        .assume_not_null()
                        .eq_any(&base.details.steward_households),
                )
                .count()
                .first::<i64>(&mut conn)
                .unwrap_or(0) as i32
        }
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
        // The graph branch computes coverage_shortfall via CoverageRollup;
        // the relational path does not select that lens — missing ≡ not-selected.
        coverage_shortfall: None,
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

fn count_online_peers_in_households(
    conn: &mut diesel::SqliteConnection,
    households: &HashSet<String>,
) -> Result<i32, StorageError> {
    if households.is_empty() {
        return Ok(0);
    }
    let mut count = 0;
    for h in households.iter() {
        let rows = peer_statuses::list_by_household(conn, h)
            .map_err(|e| StorageError::Internal(format!("list_by_household: {e}")))?;
        for row in rows {
            if matches!(row.status.as_str(), "online" | "degraded") {
                count += 1;
            }
        }
    }
    Ok(count)
}
