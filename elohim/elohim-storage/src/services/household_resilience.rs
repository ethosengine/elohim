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
    CommitmentBackedReplication, FeltFloorView, FeltStatusView, HouseholdResilienceDetails,
    HouseholdResilienceView, OnlinePeersView, PlacementGapView, RegionalDistributionView,
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
                // Sprint-3 stub: per-tier commitment counts from rea_commitments land in follow-up
                commitment_backed_replication: CommitmentBackedReplication::default(),
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
                humans::table.on(humans::agent_pub_key.eq(shard_locations::peer_id.nullable())),
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
    let base = compute(pool, ctx, content_id, viewer_household_id)?;

    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(e.to_string()))?;

    // Distribution-state honesty (2026-06-12 unmeasured≠zero): a content with
    // no shard manifest has never entered the distribution plane — every
    // count below is a non-measurement, not a measured zero. Renderers show a
    // distinct "not yet distributed" state instead of a fake at-risk verdict.
    let distribution_state =
        match crate::db::shard_manifests::get_manifest(&mut conn, &ctx.h_app_id, content_id)? {
            Some(_) => "measured",
            None => "unmeasured",
        }
        .to_string();

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

    #[allow(deprecated)]
    let commitment_backed_collectives: i32 = {
        rea_commitments::table
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
            .filter(
                rea_commitments::action
                    .eq_any(["provide", "replicates-content", "replicates-commons"]),
            )
            .filter(rea_commitments::state.eq("active"))
            .filter(
                rea_commitments::resource_classified_as
                    .nullable()
                    .eq(&scope),
            )
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
    let regional_distribution =
        compute_regional_distribution(&mut conn, &ctx.h_app_id, content_id, viewer_household_id)
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
        })
        .collect();

    // Felt projection — the household-addressed re-statement ("names, not nines").
    // Floor-relative + unmeasured-aware. The tier defaults to "standard"/undeclared
    // until the content-declared resilience-tier primitive lands (see
    // genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md);
    // it is deliberately NOT derived from reach (that mapping is the conflation
    // the tier primitive corrects).
    let felt_status = Some(build_felt_status(
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
    })
}

/// Resilience FLOOR (households wanted) for a tier — the content-relative
/// denominator. Value-driven and owner-declarable; the declared-tier primitive
/// (genesis/data/timeline/backlog/resilience-tier-content-declared-floor.md)
/// will set the tier per content. Until then the tier is "standard" (the legacy
/// `≥3 households` protected bar). Deliberately NOT derived from reach.
fn floor_for_tier(tier: &str) -> i32 {
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
pub(crate) fn build_felt_status(
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
            humans::table.on(humans::agent_pub_key
                .nullable()
                .eq(shard_locations::peer_id.nullable())),
        )
        .left_join(collectives::table.on(collectives::id.nullable().eq(humans::household_id)))
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
