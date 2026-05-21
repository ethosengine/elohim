//! Resilience hub projection service (C2).
//!
//! Computes `Vec<HubSummary>` for a given content item by querying
//! `peer_blob_inventory` (gossip operational) and cross-referencing
//! peer-identity bindings + household/collective membership tables.
//!
//! ## Hub classification ([[project_hub_archetype_abstraction]])
//!
//! Hub is a *role*, not a notarized entity. Classification happens at this
//! projection layer; the substrate stays kind-agnostic:
//!
//! 1. Query `peer_blob_inventory` for all peers holding this content's blob.
//! 2. For each peer, resolve `peer_identity_bindings.agent_cid` →
//!    `humans.agent_pub_key` → `humans.household_id` / `humans.id`.
//! 3. If the human row has a non-null `household_id` → kind = `Dwelling`.
//! 4. If the human row has a matching `collective_participations` row → kind = `Collective`.
//! 5. Otherwise → kind = `Computed`.
//!
//! **Current limitation:** `peer_identity_bindings.agent_cid` maps to
//! `humans.agent_pub_key` (nullable). When a peer has not yet published an
//! identity binding, it is classified as `Computed` (substrate-honest default).
//! Richer classification becomes possible as binding tables fill via DHT gossip.
//!
//! Hub grouping: for C2's MVP, each peer is its own hub. When a
//! `household_id` is resolved, all peers in the same household are grouped
//! under one `HubSummary` keyed by `household_id`; replica_count is the count
//! of distinct peers in that household. Collective grouping is parallel.

use std::collections::HashMap;

use diesel::prelude::*;

use crate::db::{AppContext, DbPool};
use crate::error::StorageError;
use elohim_views::{HubKind, HubSummary};

/// Compute hub summaries for the given content blob hash.
///
/// Returns an empty vec when no inventory records exist for this content.
/// Errors only on DB pool exhaustion or hard query failures.
pub fn hub_summary(
    pool: &DbPool,
    _ctx: &AppContext,
    content_id: &str,
) -> Result<Vec<HubSummary>, StorageError> {
    let mut conn = pool
        .get()
        .map_err(|e| StorageError::Internal(format!("resilience hub pool: {e}")))?;

    use crate::db::diesel_schema::{
        collective_participations, humans, peer_blob_inventory, peer_identity_bindings,
    };

    // -----------------------------------------------------------------------
    // Step 1: fetch all peer_blob_inventory rows where blob_hash = content_id.
    //
    // The content_id used as the primary lookup key here is the blob_hash
    // column in peer_blob_inventory. Callers pass the content identifier
    // from their route context — EPR-addressed blobs use sha256-{hex} or
    // blake3-{hex} as their stable identity, which is what inventory gossip
    // records. When content_id is a higher-level lamad ID (not a raw hash),
    // there will be zero rows and the endpoint returns empty hubs[] — that is
    // the correct substrate-honest response.
    // -----------------------------------------------------------------------

    let inventory_rows: Vec<(String, String, i64)> = peer_blob_inventory::table
        .filter(peer_blob_inventory::blob_hash.eq(content_id))
        .select((
            peer_blob_inventory::peer_id,
            peer_blob_inventory::last_seen_at,
            peer_blob_inventory::sequence,
        ))
        .load::<(String, String, i64)>(&mut conn)
        .map_err(|e| StorageError::Internal(format!("peer_blob_inventory query: {e}")))?;

    if inventory_rows.is_empty() {
        return Ok(vec![]);
    }

    // -----------------------------------------------------------------------
    // Step 2: resolve peer_id → agent_cid via peer_identity_bindings.
    // Only the most recent (non-superseded) binding per peer_id is used.
    // -----------------------------------------------------------------------

    let peer_ids: Vec<&str> = inventory_rows.iter().map(|(p, _, _)| p.as_str()).collect();

    let identity_rows: Vec<(String, String)> = peer_identity_bindings::table
        .filter(peer_identity_bindings::peer_id.eq_any(&peer_ids))
        .filter(peer_identity_bindings::superseded_by.is_null())
        .select((
            peer_identity_bindings::peer_id,
            peer_identity_bindings::agent_cid,
        ))
        .load::<(String, String)>(&mut conn)
        .map_err(|e| StorageError::Internal(format!("peer_identity_bindings query: {e}")))?;

    // peer_id → agent_cid
    let peer_to_agent: HashMap<String, String> = identity_rows.into_iter().collect();

    // -----------------------------------------------------------------------
    // Step 3: resolve agent_cid → (human_id, household_id) via humans table.
    // humans.agent_pub_key is the canonical field that stores the agent CID.
    // -----------------------------------------------------------------------

    let agent_cids: Vec<&str> = peer_to_agent.values().map(|s| s.as_str()).collect();

    let human_rows: Vec<(String, Option<String>, Option<String>)> = humans::table
        .filter(humans::agent_pub_key.eq_any(&agent_cids))
        .select((humans::id, humans::agent_pub_key, humans::household_id))
        .load::<(String, Option<String>, Option<String>)>(&mut conn)
        .map_err(|e| StorageError::Internal(format!("humans query: {e}")))?;

    // agent_cid → (human_id, household_id)
    let agent_to_human: HashMap<String, (String, Option<String>)> = human_rows
        .into_iter()
        .filter_map(|(human_id, agent_cid_opt, hh_id)| {
            agent_cid_opt.map(|cid| (cid, (human_id, hh_id)))
        })
        .collect();

    // -----------------------------------------------------------------------
    // Step 4: resolve human_id → collective_id via collective_participations.
    // A human with an active (not departed) participation is classified as
    // Collective.
    // -----------------------------------------------------------------------

    let human_ids: Vec<&str> = agent_to_human
        .values()
        .map(|(hid, _)| hid.as_str())
        .collect();

    let collective_rows: Vec<(String, String)> = collective_participations::table
        .filter(collective_participations::human_id.eq_any(&human_ids))
        .filter(collective_participations::departed_at.is_null())
        .select((
            collective_participations::human_id,
            collective_participations::collective_id,
        ))
        .load::<(String, String)>(&mut conn)
        .map_err(|e| StorageError::Internal(format!("collective_participations query: {e}")))?;

    // human_id → collective_id (first active one wins; richer multi-collective
    // grouping is a follow-up when the UI can display it)
    let human_to_collective: HashMap<String, String> = collective_rows.into_iter().collect();

    // -----------------------------------------------------------------------
    // Step 5: compute seconds-since-last-seen for each peer.
    // `last_seen_at` is TEXT ISO-8601; we approximate via a rough parse.
    // On parse failure we set last_verified_seconds = None (not fatal).
    // -----------------------------------------------------------------------

    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);

    // -----------------------------------------------------------------------
    // Step 6: group peers into hubs and classify kind.
    //
    // Grouping key priority:
    //   1. household_id  → Dwelling  (key = household_id)
    //   2. collective_id → Collective (key = collective_id)
    //   3. fallback      → Computed  (key = peer_id)
    //
    // Hub is determined at projection layer; substrate stays kind-agnostic.
    // -----------------------------------------------------------------------

    // hub_key → (kind, replica_count, earliest_last_seen_secs, display_label)
    let mut hub_map: HashMap<String, (HubKind, i32, Option<i64>, Option<String>)> = HashMap::new();

    for (peer_id, last_seen_at, _seq) in &inventory_rows {
        let last_seen_secs = parse_iso8601_to_epoch_secs(last_seen_at);
        let seconds_ago = last_seen_secs.map(|lss| now_secs.saturating_sub(lss));

        // Resolve the classification chain
        let (hub_key, kind, label): (String, HubKind, Option<String>) =
            if let Some(agent_cid) = peer_to_agent.get(peer_id) {
                if let Some((human_id, hh_id_opt)) = agent_to_human.get(agent_cid) {
                    if let Some(hh_id) = hh_id_opt {
                        // Dwelling: household_id known
                        (hh_id.clone(), HubKind::Dwelling, None)
                    } else if let Some(coll_id) = human_to_collective.get(human_id.as_str()) {
                        // Collective: active collective participation
                        (coll_id.clone(), HubKind::Collective, None)
                    } else {
                        // Human found but no household/collective — Computed
                        (peer_id.clone(), HubKind::Computed, None)
                    }
                } else {
                    // agent_cid exists in binding but no human row — Computed
                    (peer_id.clone(), HubKind::Computed, None)
                }
            } else {
                // No identity binding for this peer — Computed (peer-of-one hub)
                (
                    peer_id.clone(),
                    HubKind::Computed,
                    label_from_peer_id(peer_id),
                )
            };

        let entry = hub_map.entry(hub_key).or_insert((kind, 0, None, label));
        entry.1 += 1; // increment replica_count
                      // Track the most recent last_seen (smallest seconds_ago value)
        entry.2 = match (entry.2, seconds_ago) {
            (None, s) => s,
            (Some(existing), Some(new)) => Some(existing.min(new)),
            (Some(existing), None) => Some(existing),
        };
    }

    // -----------------------------------------------------------------------
    // Step 7: materialise into sorted Vec<HubSummary>.
    // Sorted by kind (Dwelling < Collective < Computed), then hub_id for stable output.
    // -----------------------------------------------------------------------

    let mut summaries: Vec<HubSummary> = hub_map
        .into_iter()
        .map(
            |(hub_id, (kind, replica_count, last_verified_seconds, display_label))| HubSummary {
                hub_id,
                kind,
                replica_count,
                last_verified_seconds,
                display_label,
            },
        )
        .collect();

    summaries.sort_by(|a, b| {
        kind_ord(a.kind)
            .cmp(&kind_ord(b.kind))
            .then(a.hub_id.cmp(&b.hub_id))
    });

    Ok(summaries)
}

/// Parse an ISO-8601 TEXT timestamp to Unix epoch seconds. Returns None on
/// parse failure — callers treat None as "last_verified_seconds unavailable".
fn parse_iso8601_to_epoch_secs(ts: &str) -> Option<i64> {
    // Minimal parser: accept "YYYY-MM-DDTHH:MM:SSZ" or "YYYY-MM-DD HH:MM:SS"
    // The full ISO-8601 parse is complex; use a simple manual approach since
    // elohim-storage stores timestamps in a consistent format.
    //
    // We use chrono if available; fall back to None if the format is
    // unrecognised. This is used only to compute a human-friendly
    // "seconds since last seen" and is non-critical.
    let s = ts.trim().trim_end_matches('Z');
    let s = s.replace(' ', "T");
    // Expected: "YYYY-MM-DDTHH:MM:SS"
    let parts: Vec<&str> = s.splitn(2, 'T').collect();
    if parts.len() != 2 {
        return None;
    }
    let date_parts: Vec<u32> = parts[0].split('-').filter_map(|p| p.parse().ok()).collect();
    let time_parts: Vec<u32> = parts[1].split(':').filter_map(|p| p.parse().ok()).collect();
    if date_parts.len() != 3 || time_parts.len() < 2 {
        return None;
    }
    // Simple Julian-day-to-epoch without external crate.
    // Good enough for "seconds since last seen" in the range of 2020–2040.
    let (y, m, d) = (
        date_parts[0] as i64,
        date_parts[1] as i64,
        date_parts[2] as i64,
    );
    let (hh, mm, ss) = (
        time_parts[0] as i64,
        time_parts[1] as i64,
        time_parts.get(2).copied().unwrap_or(0) as i64,
    );
    // Days since Unix epoch (1970-01-01). Simplified: uses proleptic Gregorian.
    let a = (14 - m) / 12;
    let y2 = y + 4800 - a;
    let m2 = m + 12 * a - 3;
    let jdn = d + (153 * m2 + 2) / 5 + 365 * y2 + y2 / 4 - y2 / 100 + y2 / 400 - 32045;
    // Julian Day Number for 1970-01-01 is 2440588.
    let days_since_epoch = jdn - 2_440_588;
    let epoch_secs = days_since_epoch * 86400 + hh * 3600 + mm * 60 + ss;
    Some(epoch_secs)
}

/// Stable sort key for hub kind.
fn kind_ord(k: HubKind) -> u8 {
    match k {
        HubKind::Dwelling => 0,
        HubKind::Collective => 1,
        HubKind::Computed => 2,
    }
}

/// Short display hint for a raw peer_id when no identity binding is found.
/// Returns None (UI falls back to hub_id itself) to avoid misleading labels.
fn label_from_peer_id(_peer_id: &str) -> Option<String> {
    None
}

#[derive(Debug, thiserror::Error)]
pub enum ResilienceError {
    #[error("database pool: {0}")]
    Pool(String),
    #[error("database query: {0}")]
    Query(#[from] diesel::result::Error),
}
