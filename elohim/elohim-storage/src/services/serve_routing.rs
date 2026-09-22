//! Capability-aware serve-routing: selects the best peer(s) to fetch content bytes from.
//!
//! Implements the D1 byte-axis selection (storage chooses, doorway never) from the
//! Wave-3 doorway-membrane-prosocial-routing spec.
//!
//! ## Architecture (elohim-facings pattern)
//!
//! - **Pure**: `fold_candidates` maps `ServeRow`s → `Candidate`s with neutral defaults
//!   for absent signals. No diesel import. Unit-testable in isolation.
//! - **Impure**: `load_serve_rows` runs the agent_cid-native multi-table join against
//!   SQLite. Tested via DbPool fixture in `tests/serve_routing.rs`.
//! - **Adapter**: `select_serve_peers` = load → fold → select_diverse. Returns
//!   chosen agent_cids ordered by score.
//!
//! ## Gaps (documented, not blocking T1–T6)
//!
//! - `current_load` and `delivery_score` are neutral-defaulted (0.0, 1.0) — no source
//!   column exists yet; follow-on projection work.
//! - `attested_rtt_ms` is fed from `p2p::transport_paths` — the node's own EWMA of
//!   round-trips it has actually measured to each peer (Entity class Ephemeral/C,
//!   locally observed, in-memory, no DHT entry, no cross-peer sync). This is
//!   deliberately **not** a signed `HealthAttestation`-style claim (no peer can
//!   produce one yet) — the field name is inherited from `elohim_peer_fabric`'s
//!   scoring vocabulary, but the value underneath it is an honest local
//!   observation, not an attested one. A peer never sampled yet resolves to
//!   `None`, which `score::rank` treats as neutral (0.5 factor) rather than a
//!   penalty — so lacking data never demotes a peer below a known-slow one.
//! - Live cross-WAN RTT ordering (T7) requires `@requires:shem` for a genuine
//!   latency *spread* between candidates; vacuous on household topology where
//!   peers are co-located. Household coverage lives in this module's unit tests
//!   (below) and in `elohim_peer_fabric::score`'s ordering tests.

use diesel::prelude::*;

use crate::db::diesel_schema::{
    humans, node_stewardship, peer_transport_manifest, rea_commitments, shard_locations,
    shard_manifests, stewarded_nodes,
};
use crate::p2p::transport_paths;
use crate::StorageError;
use elohim_peer_fabric::score::{self, Candidate};

/// Minimum capability floor — peers below this are excluded by `score::select_diverse`.
/// Zero means: any peer is a candidate regardless of capability column value.
/// Tune upward once capability-level attestation is reliably populated.
pub const MIN_CAP: u8 = 0;

/// One row from the serve-routing join — the projection of a peer that holds a shard
/// of the requested content, enriched with signals for capability-aware ranking.
///
/// Absent columns (current wave) are held as `Option`; `fold_candidates` applies
/// neutral defaults so the score module can rank without panicking.
#[derive(Debug, Clone)]
pub struct ServeRow {
    /// Holochain agent_cid (`uhCAk…`) — the canonical join key.
    pub agent_cid: String,
    /// Fault-domain key from `humans.household_id`. `None` → `""` in fold
    /// (no false fault-domain grouping across unhoused peers).
    pub household_id: Option<String>,
    /// Capability level from `stewarded_nodes.capability_level`. `None` → `MIN_CAP`.
    pub capability_level: Option<i32>,
    /// Whether the peer has an active provide/replicates-* REA commitment.
    pub bonded: bool,
    /// Current load fraction (0.0..=1.0). Not yet projected — always `None` this wave.
    pub current_load: Option<f64>,
    /// This node's own locally-observed EWMA round-trip to the peer, in whole
    /// milliseconds — read from `p2p::transport_paths` (Entity class Ephemeral/C,
    /// in-memory, no DHT entry). `None` when this node has never sampled the
    /// peer yet; `fold_candidates` maps that to `score::rank`'s neutral factor
    /// (never a penalty), so an unattested peer is never demoted below a
    /// known-slow one.
    pub attested_rtt_ms: Option<u32>,
    /// Delivery success score (0.0..=1.0). Not yet projected — always `None` this wave.
    pub delivery_score: Option<f64>,
}

/// Pure fold: map `ServeRow`s → `Candidate`s, applying neutral defaults for absent signals.
///
/// No I/O; no diesel. Unit-testable without a running database.
pub fn fold_candidates(rows: &[ServeRow]) -> Vec<Candidate> {
    rows.iter()
        .map(|r| {
            let raw_cap = r.capability_level.unwrap_or(MIN_CAP as i32);
            // Clamp: a negative or out-of-range integer in the DB becomes 0 (floor).
            let capability_level = raw_cap.clamp(0, u8::MAX as i32) as u8;
            Candidate {
                agent_cid: r.agent_cid.clone(),
                capability_level,
                current_load: r.current_load.unwrap_or(0.0), // full headroom
                attested_rtt_ms: r.attested_rtt_ms,          // None → neutral 0.5 in score
                household_id: r.household_id.clone().unwrap_or_default(), // None → "" (no false grouping)
                bonded: r.bonded,
                delivery_score: r.delivery_score.unwrap_or(1.0), // optimistic default
            }
        })
        .collect()
}

/// Load serve candidates for `blob_hash` from the agent_cid-native shard tables.
///
/// Join path (agent_cid-keyed throughout — no libp2p namespace crossing in the
/// SQL joins themselves):
/// ```text
/// shard_manifests (blob_hash → shard_hashes_json)
///   → shard_locations.shard_hash (peer_id = agent_cid)
///   → humans.agent_pub_key (household_id)
///   → humans.id → node_stewardship.human_id → stewarded_nodes (capability_level)
///   → rea_commitments.provider (bonded = active provide/replicates-*)
/// ```
///
/// `current_load` and `delivery_score` are always `None` this wave (source
/// columns not yet projected). `attested_rtt_ms` is resolved per-peer from the
/// in-memory `p2p::transport_paths` store: `peer_transport_manifest` is
/// consulted read-only for the agent_cid's optional resolved libp2p/iroh
/// aliases (never SQL-joined — only offered as candidate lookup labels, per
/// `transport_paths`'s own cross-plane LABEL convention), since a sample may
/// have landed under whichever label was known when it was recorded.
pub fn load_serve_rows(
    conn: &mut SqliteConnection,
    blob_hash: &str,
) -> Result<Vec<ServeRow>, StorageError> {
    // Step 1: find the shard_manifests rows for this blob_hash, collect shard hashes.
    // Load shard_hashes_json as Vec<String> directly (single-column select — no struct wrapper).
    let manifest_json_rows: Vec<String> = shard_manifests::table
        .filter(shard_manifests::blob_hash.eq(blob_hash))
        .select(shard_manifests::shard_hashes_json)
        .load::<String>(conn)
        .map_err(|e| StorageError::Database(format!("load_serve_rows manifest: {e}")))?;

    if manifest_json_rows.is_empty() {
        return Ok(vec![]);
    }

    // Collect all shard hashes across matching manifests (may span h_app_id).
    let mut shard_hashes: Vec<String> = Vec::new();
    for json in &manifest_json_rows {
        let parsed: Vec<String> = serde_json::from_str(json).unwrap_or_default();
        shard_hashes.extend(parsed);
    }
    shard_hashes.dedup();

    if shard_hashes.is_empty() {
        return Ok(vec![]);
    }

    // Step 2: find distinct agent_cids holding any of these shards.
    // shard_locations.peer_id stores agent_cid (uhCAk…) — verified in CLAUDE.md.
    let location_rows: Vec<String> = shard_locations::table
        .filter(shard_locations::shard_hash.eq_any(&shard_hashes))
        .select(shard_locations::peer_id)
        .distinct()
        .load::<String>(conn)
        .map_err(|e| StorageError::Database(format!("load_serve_rows locations: {e}")))?;

    if location_rows.is_empty() {
        return Ok(vec![]);
    }

    // Step 3: enrich with household_id from humans.agent_pub_key.
    // Load as tuples: (agent_pub_key, household_id, id).
    let human_rows: Vec<(Option<String>, Option<String>, String)> = humans::table
        .filter(humans::agent_pub_key.eq_any(&location_rows))
        .select((humans::agent_pub_key, humans::household_id, humans::id))
        .load::<(Option<String>, Option<String>, String)>(conn)
        .map_err(|e| StorageError::Database(format!("load_serve_rows humans: {e}")))?;

    // Build maps: agent_cid → household_id, agent_cid → human.id (for node join).
    let mut household_by_agent: std::collections::HashMap<String, Option<String>> =
        std::collections::HashMap::new();
    let mut human_id_by_agent: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    for (agent_pub_key, household_id, id) in &human_rows {
        if let Some(ref key) = agent_pub_key {
            household_by_agent.insert(key.clone(), household_id.clone());
            human_id_by_agent.insert(key.clone(), id.clone());
        }
    }

    // Step 4: resolve capability_level via node_stewardship → stewarded_nodes.
    // Join path: human.id → node_stewardship.human_id → stewarded_nodes.capability_level.
    // Load as tuples: (human_id, capability_level).
    let human_ids: Vec<&String> = human_id_by_agent.values().collect();
    let node_cap_rows: Vec<(String, Option<i32>)> = node_stewardship::table
        .inner_join(stewarded_nodes::table.on(stewarded_nodes::id.eq(node_stewardship::node_id)))
        .filter(node_stewardship::human_id.eq_any(&human_ids))
        .select((
            node_stewardship::human_id,
            stewarded_nodes::capability_level,
        ))
        .load::<(String, Option<i32>)>(conn)
        .map_err(|e| StorageError::Database(format!("load_serve_rows node_cap: {e}")))?;

    // Build map: human_id → capability_level (first match wins).
    let mut cap_by_human: std::collections::HashMap<String, Option<i32>> =
        std::collections::HashMap::new();
    for (human_id, capability_level) in node_cap_rows {
        cap_by_human.entry(human_id).or_insert(capability_level);
    }

    // Step 5: resolve bonded — active provide/replicates-* REA commitment.
    // Bonded actions from the score spec: provide, replicates-content, replicates-commons, custody-blob.
    let bonded_actions = vec![
        "provide",
        "replicates-content",
        "replicates-commons",
        "custody-blob",
    ];

    let bonded_providers: std::collections::HashSet<String> = rea_commitments::table
        .filter(rea_commitments::provider.eq_any(&location_rows))
        .filter(rea_commitments::action.eq_any(&bonded_actions))
        .filter(rea_commitments::state.eq("active"))
        .filter(rea_commitments::finished.eq(0))
        .select(rea_commitments::provider)
        .load::<String>(conn)
        .map_err(|e| StorageError::Database(format!("load_serve_rows bonded: {e}")))?
        .into_iter()
        .collect();

    // Step 5.5: resolve each agent_cid's optional cross-plane transport
    // aliases (libp2p PeerId / iroh NodeId) — read-only, never SQL-joined.
    // These are only offered as candidate lookup LABELS into the in-memory
    // transport_paths RTT store (Step 6): a sample for a given peer may have
    // landed under whichever label was known when it was recorded.
    let transport_alias_rows: Vec<(String, Option<String>, Option<String>)> =
        peer_transport_manifest::table
            .filter(peer_transport_manifest::agent_cid.eq_any(&location_rows))
            .select((
                peer_transport_manifest::agent_cid,
                peer_transport_manifest::libp2p_peer_id,
                peer_transport_manifest::iroh_node_id,
            ))
            .load::<(String, Option<String>, Option<String>)>(conn)
            .map_err(|e| StorageError::Database(format!("load_serve_rows transport: {e}")))?;
    let transport_aliases_by_agent: std::collections::HashMap<
        String,
        (Option<String>, Option<String>),
    > = transport_alias_rows
        .into_iter()
        .map(|(cid, libp2p_id, iroh_id)| (cid, (libp2p_id, iroh_id)))
        .collect();

    // Step 6: assemble ServeRows.
    let rows = location_rows
        .into_iter()
        .map(|agent_cid| {
            let household_id = household_by_agent.get(&agent_cid).cloned().flatten();
            let human_id = human_id_by_agent.get(&agent_cid);
            let capability_level = human_id
                .and_then(|hid| cap_by_human.get(hid))
                .copied()
                .flatten();
            let bonded = bonded_providers.contains(&agent_cid);

            // attested_rtt_ms: try every known label this peer may have been
            // recorded under locally — agent_cid always, plus any resolved
            // libp2p/iroh alias.
            let mut labels: Vec<&str> = vec![agent_cid.as_str()];
            if let Some((libp2p_id, iroh_id)) = transport_aliases_by_agent.get(&agent_cid) {
                if let Some(id) = libp2p_id {
                    labels.push(id.as_str());
                }
                if let Some(id) = iroh_id {
                    labels.push(id.as_str());
                }
            }
            let attested_rtt_ms = transport_paths::global().best_known_rtt_ms(&labels);

            ServeRow {
                agent_cid,
                household_id,
                capability_level,
                bonded,
                current_load: None, // not yet projected
                attested_rtt_ms,
                delivery_score: None, // not yet projected
            }
        })
        .collect();

    Ok(rows)
}

/// Select up to `n` peers for serving `blob_hash`, ordered by capability/headroom/RTT/bond/delivery/diversity.
///
/// Returns chosen `agent_cid`s. An empty result means no eligible peer was found —
/// the caller should shed (503), never fan out to all peers.
///
/// **Candidate source:** `shard_locations` (agent_cid-native). The `peer_blob_inventory`
/// path (libp2p-keyed) is a named gap — staged behind the `peer_transport_manifest`
/// population gap (the libp2p_peer_id column has only `#[cfg(test)]` writers in prod).
pub fn select_serve_peers(
    conn: &mut SqliteConnection,
    blob_hash: &str,
    n: usize,
) -> Result<Vec<String>, StorageError> {
    let rows = load_serve_rows(conn, blob_hash)?;
    let candidates = fold_candidates(&rows);
    let chosen = score::select_diverse(&candidates, MIN_CAP, n);
    Ok(chosen.into_iter().map(|s| s.agent_cid).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(cid: &str, hh: Option<&str>, cap: Option<i32>, bonded: bool) -> ServeRow {
        ServeRow {
            agent_cid: cid.into(),
            household_id: hh.map(Into::into),
            capability_level: cap,
            bonded,
            current_load: None,
            attested_rtt_ms: None,
            delivery_score: None,
        }
    }

    #[test]
    fn fold_maps_columns_and_neutralizes_absent_signals() {
        let rows = vec![
            row("uhCAk-a", Some("h1"), Some(5), true),
            row("uhCAk-b", None, None, false),
        ];
        let cands = fold_candidates(&rows);
        assert_eq!(cands.len(), 2);

        // Row 0 — all fields provided.
        assert_eq!(cands[0].agent_cid, "uhCAk-a");
        assert_eq!(cands[0].capability_level, 5);
        assert!(cands[0].bonded);
        assert_eq!(cands[0].household_id, "h1");
        assert_eq!(cands[0].current_load, 0.0); // None → 0.0 (full headroom)
        assert_eq!(cands[0].delivery_score, 1.0); // None → 1.0 (optimistic)
        assert_eq!(cands[0].attested_rtt_ms, None); // None → None (neutral in score)

        // Row 1 — all absent signals.
        assert_eq!(cands[1].agent_cid, "uhCAk-b");
        assert_eq!(cands[1].capability_level, MIN_CAP); // None → MIN_CAP floor
        assert_eq!(cands[1].current_load, 0.0);
        assert_eq!(cands[1].delivery_score, 1.0);
        assert_eq!(cands[1].attested_rtt_ms, None);
        assert_eq!(cands[1].household_id, ""); // None → "" (no false fault-domain grouping)
        assert!(!cands[1].bonded);
    }

    #[test]
    fn fold_empty_rows_returns_empty_candidates() {
        let cands = fold_candidates(&[]);
        assert!(cands.is_empty());
    }

    #[test]
    fn fold_clamps_negative_capability_to_zero() {
        let rows = vec![row("uhCAk-c", None, Some(-5), false)];
        let cands = fold_candidates(&rows);
        assert_eq!(cands[0].capability_level, 0);
    }

    #[test]
    fn select_diverse_on_neutral_rows_returns_up_to_n_ordered() {
        // With all-neutral signals, select_diverse still returns up to n results.
        let rows = vec![
            row("uhCAk-x", Some("hh1"), Some(3), true),
            row("uhCAk-y", Some("hh2"), Some(3), false),
            row("uhCAk-z", Some("hh1"), Some(1), true), // same household as x
        ];
        let cands = fold_candidates(&rows);
        let chosen = score::select_diverse(&cands, MIN_CAP, 2);
        assert_eq!(chosen.len(), 2);
        // Diversity: two households present (hh1, hh2) → must include one from each.
        let hh: std::collections::HashSet<&str> = chosen
            .iter()
            .map(|s| {
                rows.iter()
                    .find(|r| r.agent_cid == s.agent_cid)
                    .and_then(|r| r.household_id.as_deref())
                    .unwrap_or("")
            })
            .collect();
        assert_eq!(hh.len(), 2, "picks should span two distinct households");
    }

    #[test]
    fn no_rows_produces_empty_selection() {
        // Empty candidate set → select_serve_peers returns empty → caller sheds.
        let cands = fold_candidates(&[]);
        let chosen = score::select_diverse(&cands, MIN_CAP, 3);
        assert!(chosen.is_empty(), "no rows → caller sheds (no fanout)");
    }

    fn row_with_rtt(cid: &str, hh: &str, rtt: Option<u32>) -> ServeRow {
        ServeRow {
            agent_cid: cid.into(),
            household_id: Some(hh.into()),
            capability_level: Some(5),
            bonded: true,
            current_load: None,
            attested_rtt_ms: rtt,
            delivery_score: None,
        }
    }

    // --- 3.2: attested_rtt_ms feeds ordering (red-first coverage) ---------

    #[test]
    fn fold_and_rank_orders_lower_rtt_first_when_otherwise_equal() {
        let rows = vec![
            row_with_rtt("uhCAk-far", "h1", Some(300)),
            row_with_rtt("uhCAk-near", "h2", Some(10)),
        ];
        let cands = fold_candidates(&rows);
        let ranked = score::rank(&cands, MIN_CAP);
        assert_eq!(
            ranked[0].agent_cid, "uhCAk-near",
            "lower attested_rtt_ms should rank first when capability/load/bond/delivery are equal"
        );
    }

    #[test]
    fn fold_and_rank_stable_ordering_when_rtt_absent_for_all() {
        // No peer has been locally sampled yet (attested_rtt_ms = None for
        // both) — every other signal is equal, so ordering must be a stable
        // (input-order-preserving) sort, never an arbitrary flip driven by
        // absent data.
        let rows = vec![
            row_with_rtt("uhCAk-first", "h1", None),
            row_with_rtt("uhCAk-second", "h2", None),
        ];
        let cands = fold_candidates(&rows);
        let ranked = score::rank(&cands, MIN_CAP);
        assert_eq!(ranked[0].agent_cid, "uhCAk-first");
        assert_eq!(ranked[1].agent_cid, "uhCAk-second");
    }

    #[test]
    fn fold_and_rank_unknown_rtt_never_demoted_below_a_known_slow_peer() {
        // A not-yet-sampled peer (None → neutral 0.5 rtt_factor) must not
        // rank below a peer with a measured SLOW rtt (~900ms → ~0.1 factor).
        // Absence of data is neutral, never a penalty.
        let rows = vec![
            row_with_rtt("uhCAk-slow", "h1", Some(900)),
            row_with_rtt("uhCAk-unsampled", "h2", None),
        ];
        let cands = fold_candidates(&rows);
        let ranked = score::rank(&cands, MIN_CAP);
        assert_eq!(
            ranked[0].agent_cid, "uhCAk-unsampled",
            "unknown RTT (neutral 0.5) must not be demoted below a known-slow peer"
        );
    }
}
