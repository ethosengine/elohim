//! Reciprocity view service — SQL-only per-agent ledger.
//!
//! ## Source of Truth
//!
//! Operational (Category C) read-projection over notarized REA Commitment +
//! EconomicEvent (Category A, content_store zome). DHT is authoritative; no
//! federation needed because every peer indexes the global REA flow via the
//! rea_projection signal stream.
//!
//! Surfaces "I committed X bytes to Adam, delivered Y", paired with "Adam
//! committed P bytes to me, delivered Q." This is the per-agent reciprocity
//! ledger that powers the topology UI's reciprocation-count drilldown.

use std::collections::{HashMap, HashSet};

use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::{Double, Float, Nullable};
use thiserror::Error;

use crate::db::models::PeerIdentityBindingRow;
use crate::db::DbPool;
use crate::views::{ReciprocityRow, ReciprocityView};

#[derive(Debug, Error)]
pub enum ReciprocityViewError {
    #[error("db error: {0}")]
    Db(#[from] diesel::result::Error),
    #[error("pool error: {0}")]
    Pool(String),
}

/// Aggregate a `ReciprocityView` for the given agent's peer set.
///
/// Pure SQL — no federation. The ledger is locally complete because both
/// `rea_commitments` AND `economic_events` are projections of DHT-notarized
/// truth that every peer's storage indexes via the existing `rea_projection`
/// signal handler.
///
/// # Arguments
/// * `pool` — shared SQLite connection pool
/// * `agent_cid` — the agent whose ledger to build
/// * `bindings` — the agent's active `PeerIdentityBindingRow`s (provider of the
///   peer_id set)
/// * `connected_peers` — snapshot from `swarm.connected_peers()` at request time.
///   HTTP callers (no live swarm) pass `&HashSet::new()` — online will be
///   `Some(false)` for all rows, which is correct semantics for cold-cache reads.
pub async fn aggregate_reciprocity_view(
    pool: &DbPool,
    agent_cid: &str,
    bindings: &[PeerIdentityBindingRow],
    connected_peers: &HashSet<String>,
) -> Result<ReciprocityView, ReciprocityViewError> {
    let mut conn = pool
        .get()
        .map_err(|e| ReciprocityViewError::Pool(e.to_string()))?;

    let my_peers: Vec<&str> = bindings.iter().map(|b| b.peer_id.as_str()).collect();

    if my_peers.is_empty() {
        return Ok(ReciprocityView {
            agent_cid: agent_cid.to_string(),
            inflow: vec![],
            outflow: vec![],
            net_hosted_bytes: 0,
            capacity_available_bytes: 0,
        });
    }

    let outflow = compute_flow_rows(
        &mut conn,
        &my_peers,
        FlowDirection::Outflow,
        pool,
        connected_peers,
    )
    .await?;
    let inflow = compute_flow_rows(
        &mut conn,
        &my_peers,
        FlowDirection::Inflow,
        pool,
        connected_peers,
    )
    .await?;

    let total_outflow_delivered: i64 = outflow.iter().map(|r| r.delivered_bytes as i64).sum();
    let total_inflow_delivered: i64 = inflow.iter().map(|r| r.delivered_bytes as i64).sum();
    let net_hosted_bytes = total_inflow_delivered - total_outflow_delivered;

    let capacity_available_bytes =
        crate::services::device_capacity::available_bytes_for(pool, agent_cid).await;

    Ok(ReciprocityView {
        agent_cid: agent_cid.to_string(),
        inflow,
        outflow,
        net_hosted_bytes,
        capacity_available_bytes,
    })
}

/// Aggregate stewarded bytes per peer.
///
/// For each peer in `my_peer_ids`, SUMs the `resource_quantity_value` of all
/// `rea_commitments` rows where `action = 'custody-blob'` and `provider = peer`.
/// Returns a map from peer_id → total committed bytes (as `u64`).
///
/// Peers with no matching commitments are absent from the returned map.
///
/// ## Source-of-truth lineage
/// Category-C operational projection over Category-A notarized REA Commitments
/// (`rea_commitments` table). No federation needed — every node indexes the
/// global REA flow via the `rea_projection` signal stream.
///
/// ## Care-class / compute-class isolation
/// This function touches only `custody-blob` commitments, which are
/// compute-class. The `action` filter prevents accidental co-mingling with
/// care-class commitment kinds.
pub async fn aggregate_stewarded_bytes_by_peer(
    pool: &DbPool,
    my_peer_ids: &[String],
) -> Result<HashMap<String, u64>, ReciprocityViewError> {
    use crate::db::diesel_schema::rea_commitments::dsl as rc;

    if my_peer_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let peer_strs: Vec<&str> = my_peer_ids.iter().map(String::as_str).collect();

    let mut conn = pool
        .get()
        .map_err(|e| ReciprocityViewError::Pool(e.to_string()))?;

    // Use Double (f64) for the SUM so multi-TB aggregates retain full precision.
    // The underlying column is f32, but SQLite REAL is 64-bit — widening here is
    // lossless at the DB layer and prevents ~10⁶-byte truncation at multi-TB scale.
    let rows: Vec<(String, Option<f64>)> = rc::rea_commitments
        .filter(rc::action.eq("custody-blob"))
        .filter(rc::provider.eq_any(&peer_strs))
        .group_by(rc::provider)
        .select((
            rc::provider,
            sql::<Nullable<Double>>("SUM(resource_quantity_value)"),
        ))
        .load::<(String, Option<f64>)>(&mut conn)?;

    Ok(rows
        .into_iter()
        .filter_map(|(peer, bytes)| bytes.map(|b| (peer, b.max(0.0) as u64)))
        .collect())
}

/// Direction of byte flow relative to this agent's peer set.
#[derive(Debug, Clone, Copy)]
enum FlowDirection {
    /// I committed to host their bytes (provider IN my_peers, GROUP BY receiver).
    Outflow,
    /// They committed to host my bytes (receiver IN my_peers, GROUP BY provider).
    Inflow,
}

/// Build per-counterparty `ReciprocityRow`s for one flow direction.
///
/// Step 1: SUM committed bytes from `rea_commitments` where `action='custody-blob'`
///         and the agent's peers are on the relevant side, grouped by counterparty.
///
/// Step 2: For each counterparty SUM delivered bytes from `economic_events`
///         where `action='serve-blob'`, matching `(provider, receiver)` pair.
///         Pairing by `(provider, receiver, action_pair)` because there is no
///         `commitment_id` column linking events to commitments.
///
/// Phase 4 additions:
/// - `display_name`: resolved via `imagodei_lookup::resolve_display_name` per counterparty.
/// - `online`: computed via `connectivity::any_online_in` against the connected-peers snapshot.
async fn compute_flow_rows(
    conn: &mut diesel::SqliteConnection,
    my_peers: &[&str],
    direction: FlowDirection,
    pool: &DbPool,
    connected_peers: &HashSet<String>,
) -> Result<Vec<ReciprocityRow>, ReciprocityViewError> {
    use crate::db::diesel_schema::economic_events::dsl as ev;
    use crate::db::diesel_schema::rea_commitments::dsl as rc;

    // Step 1: per-counterparty SUM of committed bytes.
    let committed_rows: Vec<(String, Option<f32>)> = match direction {
        FlowDirection::Outflow => rc::rea_commitments
            .filter(rc::action.eq("custody-blob"))
            .filter(rc::provider.eq_any(my_peers))
            .group_by(rc::receiver)
            .select((
                rc::receiver,
                sql::<Nullable<Float>>("SUM(resource_quantity_value)"),
            ))
            .load::<(String, Option<f32>)>(conn)?,
        FlowDirection::Inflow => rc::rea_commitments
            .filter(rc::action.eq("custody-blob"))
            .filter(rc::receiver.eq_any(my_peers))
            .group_by(rc::provider)
            .select((
                rc::provider,
                sql::<Nullable<Float>>("SUM(resource_quantity_value)"),
            ))
            .load::<(String, Option<f32>)>(conn)?,
    };

    // Step 2: for each counterparty, SUM delivered bytes from economic_events.
    let mut rows: Vec<ReciprocityRow> = Vec::with_capacity(committed_rows.len());
    for (counterparty, committed_opt) in committed_rows {
        let committed_bytes = committed_opt.unwrap_or(0.0).max(0.0) as u64;

        let delivered_opt: Option<f32> = match direction {
            FlowDirection::Outflow => ev::economic_events
                .filter(ev::action.eq("serve-blob"))
                .filter(ev::provider.eq_any(my_peers))
                .filter(ev::receiver.eq(&counterparty))
                .select(sql::<Nullable<Float>>("SUM(resource_quantity_value)"))
                .first::<Option<f32>>(conn)?,
            FlowDirection::Inflow => ev::economic_events
                .filter(ev::action.eq("serve-blob"))
                .filter(ev::receiver.eq_any(my_peers))
                .filter(ev::provider.eq(&counterparty))
                .select(sql::<Nullable<Float>>("SUM(resource_quantity_value)"))
                .first::<Option<f32>>(conn)?,
        };
        let delivered_bytes = delivered_opt.unwrap_or(0.0).max(0.0) as u64;

        let honored_percent = if committed_bytes == 0 {
            0.0
        } else {
            (delivered_bytes as f64 / committed_bytes as f64) * 100.0
        };

        // Phase 4: resolve display_name via imagodei lookup.
        let display_name =
            crate::services::imagodei_lookup::resolve_display_name(pool, &counterparty).await;

        // Phase 4: determine online status via connected-peers snapshot.
        // Look up counterparty's peer_ids from peer_identity_bindings.
        let counterparty_peers: Vec<String> = {
            use crate::db::diesel_schema::peer_identity_bindings::dsl as bind;
            bind::peer_identity_bindings
                .filter(bind::agent_cid.eq(&counterparty))
                .filter(bind::superseded_by.is_null())
                .select(bind::peer_id)
                .load::<String>(conn)
                .unwrap_or_default()
        };
        let counterparty_peer_strs: Vec<&str> =
            counterparty_peers.iter().map(String::as_str).collect();
        let online = Some(crate::services::connectivity::any_online_in(
            &counterparty_peer_strs,
            connected_peers,
        ));

        rows.push(ReciprocityRow {
            counterparty_household_id: counterparty,
            display_name,
            committed_bytes,
            delivered_bytes,
            honored_percent,
            online,
        });
    }

    Ok(rows)
}
