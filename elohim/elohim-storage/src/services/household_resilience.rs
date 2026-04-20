//! Household-first resilience computation.
//!
//! For a given content id, aggregates shard_locations + peer_statuses
//! into a `HouseholdResilienceView` that answers the protection claim
//! household-to-household rather than peer-to-peer. The view is computed
//! per-request; no persistence, no new DHT entry types. Source of truth:
//! the upstream DHT entries (Agreement + PeerStatus + NodeRegistration).

use std::collections::HashSet;

use diesel::prelude::*;

use crate::db::{peer_statuses, placement_gaps, AppContext, DbPool};
use crate::error::StorageError;
use crate::views::{
    HouseholdResilienceDetails, HouseholdResilienceView, PlacementGapView,
    RegionalDistributionView, ResilienceSnapshotDetailsView, ResilienceSnapshotView,
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

    // Stage 1: household reducer — uses humans.household_id projection joined
    // from shard_locations to count distinct households stewarding this content.
    //
    // Two-step approach: fetch the manifest's shard_hashes_json, parse JSON,
    // then filter shard_locations by eq_any(&shard_hashes). This is required
    // because diesel cannot filter on a JSON-encoded column directly.
    let manifest = crate::db::shard_manifests::get_manifest(&mut conn, &ctx.h_app_id, content_id)?;

    // When no manifest exists for this content_id, return a degenerate
    // at-risk view immediately — do NOT fall back to aggregating across all
    // shard_locations for the h_app_id, which would inflate household counts
    // for orphaned content in any multi-content production database.
    let shard_hashes: Vec<String> = match manifest {
        None => {
            return Ok(HouseholdResilienceView {
                content_id: content_id.to_string(),
                households_stewarding: 0,
                households_reciprocated: 0,
                protection_status: "at-risk".to_string(),
                details: HouseholdResilienceDetails {
                    steward_households: vec![],
                    online_peer_count: 0,
                    health_score: 0.0,
                },
            });
        }
        Some(m) => serde_json::from_str(&m.shard_hashes_json).map_err(|e| {
            StorageError::Internal(format!(
                "shard manifest for content_id={content_id} has malformed shard_hashes_json: {e}"
            ))
        })?,
    };

    use crate::db::diesel_schema::{humans, shard_locations};

    let steward_households: HashSet<String> = {
        let raw_households: Vec<Option<String>> = shard_locations::table
            .inner_join(
                humans::table.on(
                    humans::agent_pub_key
                        .eq(shard_locations::peer_id.nullable()),
                ),
            )
            .filter(shard_locations::h_app_id.eq(&ctx.h_app_id))
            .filter(humans::household_id.is_not_null())
            .filter(shard_locations::shard_hash.eq_any(&shard_hashes))
            .select(humans::household_id)
            .load::<Option<String>>(&mut conn)
            .map_err(|e| StorageError::Internal(format!("household query: {e}")))?;

        raw_households.into_iter().flatten().collect()
    };

    let households_stewarding = steward_households.len() as i32;

    // Stage 2: reciprocation — recorded as zero; reverse allocation traversal
    // is a follow-up concern.
    let _ = viewer_household_id;
    let households_reciprocated: i32 = 0;

    // Stage 3: online peer count — how many nodes across stewarding
    // households currently have an active PeerStatus.
    let online_peer_count = count_online_peers_in_households(&mut conn, &steward_households)?;

    // Stage 4: status classification. Thresholds mirror the a2o spec:
    //   protected ← ≥3 households stewarding AND ≥2 online peers
    //   partial   ← ≥2 households OR ≥1 online peer
    //   at-risk   ← otherwise
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
    let base = compute(pool, ctx, content_id, viewer_household_id)?;

    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;

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

    let commitment_backed_collectives: i32 = {
        rea_commitments::table
            .inner_join(
                humans::table.on(
                    humans::agent_pub_key
                        .nullable()
                        .eq(rea_commitments::provider.nullable()),
                ),
            )
            .filter(rea_commitments::h_app_id.eq(&ctx.h_app_id))
            .filter(rea_commitments::action.eq("provide"))
            .filter(rea_commitments::state.eq("active"))
            .filter(rea_commitments::resource_classified_as.nullable().eq(&scope))
            .filter(humans::household_id.is_not_null())
            .select(diesel::dsl::count_distinct(humans::household_id))
            .first::<i64>(&mut conn)
            .unwrap_or(0) as i32
    };

    // diversity_score: min(stewarding_collectives, max(commitment_backed,1)) / desired
    // RS 4+3 baseline target = 7. Per-content override deferred to Plan 3.
    let desired = 7_i32;
    let diversity_score = if desired == 0 {
        0.0_f32
    } else {
        (base
            .households_stewarding
            .min(commitment_backed_collectives.max(1)) as f32
            / desired as f32)
            .clamp(0.0, 1.0)
    };

    // regional_distribution: join steward collectives → collectives.region.
    let regional_distribution = compute_regional_distribution(
        &mut conn,
        &ctx.h_app_id,
        content_id,
        viewer_household_id,
    )
    .unwrap_or(RegionalDistributionView {
        local: 0,
        regional: 0,
        global: 0,
        unknown: base.households_stewarding,
    });

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

    // Map steward_households (Vec<String> of ids) → Vec<StewardingCollectiveEntry>
    let steward_collective_entries: Vec<StewardingCollectiveEntry> = base
        .details
        .steward_households
        .iter()
        .map(|id| StewardingCollectiveEntry {
            id: id.clone(),
            kind: "household".to_string(),
            label: None,
        })
        .collect();

    Ok(ResilienceSnapshotView {
        content_id: base.content_id.clone(),
        stewarding_collectives: base.households_stewarding,
        commitment_backed_collectives,
        diversity_score,
        regional_distribution,
        placement_gaps: gaps,
        protection_status: base.protection_status.clone(),
        reciprocating_collectives: Some(base.households_reciprocated),
        details: Some(ResilienceSnapshotDetailsView {
            stewarding_collectives: steward_collective_entries,
            online_peer_count: base.details.online_peer_count,
            health_score: base.details.health_score,
        }),
    })
}

fn compute_regional_distribution(
    conn: &mut diesel::SqliteConnection,
    h_app_id: &str,
    content_id: &str,
    viewer_household_id: Option<&str>,
) -> Result<RegionalDistributionView, StorageError> {
    use crate::db::diesel_schema::{collectives, humans, shard_locations};

    // Find the content's shard hashes via the manifest.
    let manifest = crate::db::shard_manifests::get_manifest(conn, h_app_id, content_id)?;
    let shard_hashes: Vec<String> = match &manifest {
        Some(m) => serde_json::from_str(&m.shard_hashes_json).unwrap_or_default(),
        None => {
            return Ok(RegionalDistributionView {
                local: 0,
                regional: 0,
                global: 0,
                unknown: 0,
            })
        }
    };

    // Join shard_locations → humans → collectives to get each steward's region.
    // humans.household_id → collectives.id (left join; stewards without a
    // collective get NULL region → unknown bucket).
    let rows: Vec<(String, Option<String>, Option<String>)> = shard_locations::table
        .inner_join(
            humans::table.on(humans::agent_pub_key.nullable().eq(shard_locations::peer_id.nullable())),
        )
        .left_join(
            collectives::table.on(collectives::id.nullable().eq(humans::household_id)),
        )
        .filter(shard_locations::h_app_id.eq(h_app_id))
        .filter(shard_locations::shard_hash.eq_any(&shard_hashes))
        .select((
            humans::id,
            humans::household_id,
            collectives::region.nullable(),
        ))
        .load(conn)
        .unwrap_or_default();

    let viewer_region: Option<String> = match viewer_household_id {
        None => None,
        Some(vh) => collectives::table
            .filter(collectives::id.eq(vh))
            .select(collectives::region)
            .first::<Option<String>>(conn)
            .unwrap_or(None),
    };

    // Dedupe by household so two peers in the same household count once.
    let mut seen: HashSet<Option<String>> = Default::default();
    let mut dist = RegionalDistributionView {
        local: 0,
        regional: 0,
        global: 0,
        unknown: 0,
    };
    for (_human_id, household_id, steward_region) in rows {
        if !seen.insert(household_id.clone()) {
            continue;
        }
        match (viewer_region.as_deref(), steward_region.as_deref()) {
            (None, None) => dist.unknown += 1,
            (None, Some(_)) => dist.global += 1,
            (Some(_), None) => dist.unknown += 1,
            (Some(vr), Some(sr)) if vr == sr => dist.local += 1,
            (Some(_), Some(_)) => dist.regional += 1,
        }
    }

    Ok(dist)
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
