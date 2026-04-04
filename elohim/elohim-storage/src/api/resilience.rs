//! Resilience API controller
//!
//! Routes: `/api/v1/resilience/{content_id}`
//!         `/api/v1/resilience/{content_id}/verify`  (Sprint C)

use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request, Response};

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use crate::services::response;
use crate::views::*;

use super::get_conn;

pub async fn handle(
    _req: Request<hyper::body::Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let path = resource_path.trim_start_matches('/');

    match (&method, path) {
        // GET /api/v1/resilience/{content_id}
        (&Method::GET, content_id) if !content_id.is_empty() && !content_id.contains('/') => {
            handle_get_resilience(content_id, pool, ctx).await
        }
        _ => Ok(response::not_found(&format!(
            "Unknown resilience route: /api/v1/resilience/{}",
            path
        ))),
    }
}

async fn handle_get_resilience(
    content_id: &str,
    pool: &DbPool,
    ctx: &AppContext,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;

    // 1. Get shard manifest
    let manifest =
        crate::db::shard_manifests::get_manifest(&mut conn, &ctx.h_app_id, content_id)?;

    let encoding = match &manifest {
        Some(m) => EncodingInfoView {
            strategy: m.encoding.clone(),
            data_shards: m.data_shard_count,
            parity_shards: m.parity_shard_count,
            total_size_bytes: m.total_size_bytes as i64,
            shard_size_bytes: m.shard_size_bytes as i64,
        },
        None => EncodingInfoView {
            strategy: "unknown".to_string(),
            data_shards: 0,
            parity_shards: 0,
            total_size_bytes: 0,
            shard_size_bytes: 0,
        },
    };

    // 2. Get shard locations
    let locations = crate::db::shard_locations::get_locations_for_content(
        &mut conn,
        &ctx.h_app_id,
        content_id,
    )?;

    let shard_hashes: Vec<String> = manifest
        .as_ref()
        .map(|m| serde_json::from_str(&m.shard_hashes_json).unwrap_or_default())
        .unwrap_or_default();

    let data_shard_count = manifest.as_ref().map(|m| m.data_shard_count).unwrap_or(0) as usize;

    let shards: Vec<ShardInfoView> = shard_hashes
        .iter()
        .enumerate()
        .map(|(i, hash)| {
            let peers: Vec<String> = locations
                .iter()
                .filter(|l| l.shard_hash == *hash)
                .map(|l| l.peer_id.clone())
                .collect();
            let status = if peers.is_empty() {
                "missing"
            } else {
                "distributed"
            };
            ShardInfoView {
                hash: hash.clone(),
                shard_type: if i < data_shard_count {
                    "data".to_string()
                } else {
                    "parity".to_string()
                },
                peer_ids: peers,
                status: status.to_string(),
            }
        })
        .collect();

    let distinct_peers: std::collections::HashSet<&str> =
        locations.iter().map(|l| l.peer_id.as_str()).collect();
    let shards_with_locations = shards.iter().filter(|s| !s.peer_ids.is_empty()).count() as i32;

    let distribution = DistributionView {
        total_shards: shard_hashes.len() as i32,
        shards_with_locations,
        distinct_peers: distinct_peers.len() as i32,
        shards,
    };

    // 3. Get stewardship allocations
    let allocs = crate::db::stewardship_allocations::get_allocations_for_content(
        &mut conn,
        ctx,
        content_id,
    )?;
    let stewardship = ResilienceStewardshipView {
        steward_count: allocs.len() as i32,
        allocations: allocs
            .into_iter()
            .map(StewardshipAllocationView::from)
            .collect(),
    };

    // 4. Get storage commitments (REA commitments with action="provide")
    let commitment_query = crate::db::rea_commitments::ReaCommitmentQuery {
        action: Some("provide".to_string()),
        ..Default::default()
    };
    let commitments_list =
        crate::db::rea_commitments::list_commitments(&mut conn, ctx, &commitment_query)
            .unwrap_or_default();
    let total_committed: f32 = commitments_list
        .iter()
        .filter_map(|c| c.resource_quantity_value)
        .sum();
    let commitments = CommitmentHealthView {
        active_peers: commitments_list.len() as i32,
        total_committed_bytes: (total_committed * 1_073_741_824.0) as i64, // GB to bytes
        total_used_bytes: 0, // Computed from actual usage in future sprint
    };

    // 5. Compute health score
    let parity = encoding.parity_shards;
    let can_survive = parity.min(distribution.distinct_peers.saturating_sub(1));
    let score = if distribution.total_shards == 0 {
        0.0
    } else {
        (shards_with_locations as f32) / (distribution.total_shards as f32)
    };
    let status = if score >= 0.8 && stewardship.steward_count > 0 {
        "healthy"
    } else if score >= 0.5 {
        "degraded"
    } else {
        "at_risk"
    };

    let resilience = ResilienceView {
        content_id: content_id.to_string(),
        encoding,
        distribution,
        stewardship,
        commitments,
        health: HealthScoreView {
            score,
            can_survive_failures: can_survive,
            status: status.to_string(),
        },
    };

    Ok(response::ok(&resilience))
}
