//! Contract-aware diverse-peer selector for shard placement.
//!
//! Reads REA commitments (policy) × PeerStatus (liveness) × humans.household_id
//! (diversity) × stewarded_nodes (archetype tiebreak), and produces a ranked list
//! of peers for placing a shard. Respects contract bounds strictly — a peer
//! without an active commitment for the content's reach scope is never picked.
//!
//! ## Column name notes (actual schema vs plan)
//!
//! The diesel schema uses different column names than the plan's pseudocode:
//! - `rea_commitments.provider`            (plan used `provider_agent`)
//! - `rea_commitments.resource_classified_as` (plan used `resource_classification`)
//! - `rea_commitments.state`               (plan used `status`)
//! - `peer_statuses.status` + `general_pool_member` (plan used `lifecycle_state`)
//! - `peer_statuses` has NO `h_app_id` column
//! - `stewarded_nodes.device_archetype_id` (plan used `archetype`)
//! - `stewarded_nodes` has NO `agent_pub_key` column; archetype is joined via
//!   `node_stewardship` (node_id → human_id → agent_pub_key on humans)
//!
//! "Accepting" peer definition: `status IN ('online','degraded') AND general_pool_member=1`
//! (matches the liveness model in heartbeat.rs and network_posture.rs).

use diesel::prelude::*;
use std::collections::HashMap;

use crate::db::diesel_schema::{
    humans, node_stewardship, peer_statuses, rea_commitments, stewarded_nodes,
};
use crate::db::DbPool;
use crate::db::HUMANS_HAPP_ID;
use crate::StorageError;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// Input for a peer selection request.
#[derive(Debug, Clone)]
pub struct SelectionInput<'a> {
    /// App instance scope — matches `h_app_id` in REA commitments and humans.
    pub h_app_id: &'a str,
    /// Content being placed (informational; used in future gap-recording).
    pub content_id: &'a str,
    /// Reach scope of the content (e.g. "commons", "household"). Must match
    /// `resource_classified_as` in REA commitments as `"content:{content_reach}"`.
    pub content_reach: &'a str,
    /// Desired number of distinct steward peers.
    pub desired_count: usize,
}

/// A peer selected for shard placement.
#[derive(Debug, Clone)]
pub struct SelectedPeer {
    /// Same as `agent_pub_key` — the peer's network identity.
    pub peer_id: String,
    pub agent_pub_key: String,
    /// Primary household collective id (NULL if the human is not in a household).
    pub household_id: Option<String>,
    /// Device archetype from stewarded_nodes.device_archetype_id, if known.
    pub archetype: Option<String>,
    /// stewarded_nodes.id for the matching node, if a node record was found.
    pub node_id: Option<String>,
}

/// Outcome of a selection request.
#[derive(Debug)]
pub enum SelectionOutcome {
    /// Desired count met — all peers in the vec were selected.
    Ok(Vec<SelectedPeer>),
    /// Fewer peers than desired were available.
    Short {
        /// Peers that were selected (may be empty).
        peers: Vec<SelectedPeer>,
        /// Why we came up short:
        /// - `"contracts-short"`:   no active REA commitments match the content scope
        /// - `"peers-unavailable"`: commitments exist but none of those peers is accepting
        /// - `"under-committed"`:   some peers found, but fewer than `requested`
        gap_kind: &'static str,
        achieved: i32,
        requested: i32,
    },
}

// ---------------------------------------------------------------------------
// Service
// ---------------------------------------------------------------------------

pub struct PeerSelection {
    pool: DbPool,
    /// Liveness staleness window (seconds). A PeerStatus row whose `timestamp` is
    /// older than `now - staleness_secs` is treated as NOT accepting — a fanned-in
    /// snapshot from a peer that has since gone dark must not count as live
    /// forever. `0` disables the window (legacy behaviour): every row counts
    /// regardless of age.
    ///
    /// Defaults to `0` in [`new`](Self::new) so existing callers and tests keep
    /// legacy semantics; the production wiring (`p2p/mod.rs`) reads
    /// `PEER_STATUS_STALENESS_SECS` (default 900) and injects it via
    /// [`with_staleness_secs`](Self::with_staleness_secs). Self rows heartbeat
    /// every 60s, so a 900s window never dims a live pod.
    staleness_secs: i64,
}

impl PeerSelection {
    pub fn new(pool: DbPool) -> Self {
        Self {
            pool,
            staleness_secs: 0,
        }
    }

    /// Set the liveness staleness window (seconds); `0` disables it. Injected by
    /// the production wiring from `PEER_STATUS_STALENESS_SECS`.
    pub fn with_staleness_secs(mut self, staleness_secs: i64) -> Self {
        self.staleness_secs = staleness_secs;
        self
    }

    /// Select up to `input.desired_count` peers for shard placement.
    ///
    /// Multi-pass greedy diversity:
    ///   Pass 1 — one peer per unseen household
    ///   Pass 2 — fill from distinct archetypes
    ///   Pass 3 — fill from distinct nodes
    ///   Pass 4 — fill from anything remaining
    pub fn select(&self, input: &SelectionInput) -> Result<SelectionOutcome, StorageError> {
        let mut conn = self
            .pool
            .get()
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // ----------------------------------------------------------------
        // Step 1: Peers with active commitments matching the content reach.
        // ----------------------------------------------------------------
        //
        // resource_classified_as stores the scope as "content:{reach}".
        // action="provide" + state="active" + not finished.
        let scope = format!("content:{}", input.content_reach);

        // `resource_classified_as` is a JSON list by contract (non-commons spec
        // §11.2, Option A); the scope match is membership over the parsed list,
        // not a scalar `.eq()` (which silently misses array-wrapped rows). Load
        // candidates, filter membership in Rust, then dedup providers.
        let committed_agents: Vec<String> = {
            let candidates: Vec<(String, Option<String>)> = rea_commitments::table
                .filter(rea_commitments::h_app_id.eq(input.h_app_id))
                .filter(rea_commitments::action.eq("provide"))
                .filter(rea_commitments::state.eq("active"))
                .filter(rea_commitments::finished.eq(0))
                .select((
                    rea_commitments::provider,
                    rea_commitments::resource_classified_as,
                ))
                .load::<(String, Option<String>)>(&mut conn)
                .map_err(|e| StorageError::Internal(e.to_string()))?;

            let mut seen = std::collections::HashSet::new();
            candidates
                .into_iter()
                .filter(|(_, classified)| {
                    crate::db::rea_commitments::classifications_of(classified.as_deref())
                        .iter()
                        .any(|c| c == &scope)
                })
                .filter_map(|(provider, _)| seen.insert(provider.clone()).then_some(provider))
                .collect()
        };

        if committed_agents.is_empty() {
            return Ok(SelectionOutcome::Short {
                peers: vec![],
                gap_kind: "contracts-short",
                achieved: 0,
                requested: input.desired_count as i32,
            });
        }

        // ----------------------------------------------------------------
        // Step 2: Of those, which are alive and accepting?
        //
        // "Accepting" = status IN ('online','degraded') AND general_pool_member=1,
        // which is the definition used by heartbeat.rs and network_posture.rs.
        // peer_statuses has NO h_app_id column, so we don't filter on it here.
        //
        // Staleness (honesty guard): a fanned-in snapshot from a peer that has
        // since gone dark must not count as live forever. When a window is
        // configured, additionally require `timestamp >= now - window`. The row
        // timestamp is microseconds (see diesel_schema); `i64::MIN` as the cutoff
        // makes the filter a no-op when the window is disabled (legacy).
        // ----------------------------------------------------------------
        let cutoff_micros: i64 = if self.staleness_secs > 0 {
            chrono::Utc::now()
                .timestamp_micros()
                .saturating_sub(self.staleness_secs.saturating_mul(1_000_000))
        } else {
            i64::MIN
        };
        let accepting: Vec<String> = peer_statuses::table
            .filter(
                peer_statuses::status
                    .eq("online")
                    .or(peer_statuses::status.eq("degraded")),
            )
            .filter(peer_statuses::general_pool_member.eq(1))
            .filter(peer_statuses::peer_id.eq_any(&committed_agents))
            .filter(peer_statuses::timestamp.ge(cutoff_micros))
            .select(peer_statuses::peer_id)
            .load::<String>(&mut conn)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        if accepting.is_empty() {
            return Ok(SelectionOutcome::Short {
                peers: vec![],
                gap_kind: "peers-unavailable",
                achieved: 0,
                requested: input.desired_count as i32,
            });
        }

        // ----------------------------------------------------------------
        // Step 3: Enrich with household_id from humans.
        // ----------------------------------------------------------------
        #[derive(Queryable)]
        struct HumanRow {
            agent_pub_key: Option<String>,
            household_id: Option<String>,
        }

        let human_rows: Vec<HumanRow> = humans::table
            .filter(humans::h_app_id.eq(HUMANS_HAPP_ID)) // imagodei scope (was input.h_app_id)
            .filter(humans::agent_pub_key.eq_any(&accepting))
            .select((humans::agent_pub_key, humans::household_id))
            .load::<HumanRow>(&mut conn)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        let household_by_agent: HashMap<String, Option<String>> = human_rows
            .into_iter()
            .filter_map(|r| r.agent_pub_key.map(|k| (k, r.household_id)))
            .collect();

        // ----------------------------------------------------------------
        // Step 4: Enrich with archetype + node_id.
        //
        // stewarded_nodes has no agent_pub_key column. The join path is:
        //   humans.agent_pub_key → node_stewardship.human_id (via humans.id)
        //   node_stewardship.node_id → stewarded_nodes.id
        //
        // To avoid a heavy 3-way join, we do two lighter queries:
        //   a) humans: agent_pub_key → id
        //   b) node_stewardship + stewarded_nodes: human_id → (node_id, device_archetype_id)
        // ----------------------------------------------------------------
        #[derive(Queryable)]
        struct HumanIdRow {
            id: String,
            agent_pub_key: Option<String>,
        }

        let human_id_rows: Vec<HumanIdRow> = humans::table
            .filter(humans::h_app_id.eq(HUMANS_HAPP_ID)) // imagodei scope (was input.h_app_id)
            .filter(humans::agent_pub_key.eq_any(&accepting))
            .select((humans::id, humans::agent_pub_key))
            .load::<HumanIdRow>(&mut conn)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // agent_pub_key → human_id (internal id)
        let human_id_by_agent: HashMap<String, String> = human_id_rows
            .into_iter()
            .filter_map(|r| r.agent_pub_key.map(|k| (k, r.id)))
            .collect();

        let human_ids: Vec<&String> = human_id_by_agent.values().collect();

        #[derive(Queryable)]
        struct NodeRow {
            human_id: String,
            node_id: String,
            archetype: Option<String>,
        }

        let node_rows: Vec<NodeRow> = node_stewardship::table
            .inner_join(
                stewarded_nodes::table.on(stewarded_nodes::id.eq(node_stewardship::node_id)),
            )
            .filter(node_stewardship::human_id.eq_any(&human_ids))
            .select((
                node_stewardship::human_id,
                stewarded_nodes::id,
                stewarded_nodes::device_archetype_id,
            ))
            .load::<NodeRow>(&mut conn)
            .map_err(|e| StorageError::Internal(e.to_string()))?;

        // human_id → (node_id, archetype) — take the first match per human
        let mut node_by_human: HashMap<String, NodeRow> = HashMap::new();
        for row in node_rows {
            node_by_human.entry(row.human_id.clone()).or_insert(row);
        }

        // ----------------------------------------------------------------
        // Step 5: Build candidate SelectedPeer list.
        // ----------------------------------------------------------------
        let mut candidates: Vec<SelectedPeer> = accepting
            .iter()
            .map(|peer_id| {
                let household_id = household_by_agent.get(peer_id).cloned().flatten();
                let human_id = human_id_by_agent.get(peer_id);
                let node = human_id.and_then(|hid| node_by_human.get(hid));
                SelectedPeer {
                    peer_id: peer_id.clone(),
                    agent_pub_key: peer_id.clone(),
                    household_id,
                    archetype: node.and_then(|n| n.archetype.clone()),
                    node_id: node.map(|n| n.node_id.clone()),
                }
            })
            .collect();

        // Deterministic ordering before greedy passes.
        candidates.sort_by(|a, b| a.peer_id.cmp(&b.peer_id));

        // ----------------------------------------------------------------
        // Step 6: Greedy diversity ranking.
        // ----------------------------------------------------------------
        let mut picked: Vec<SelectedPeer> = Vec::with_capacity(input.desired_count);
        let mut seen_hh: std::collections::HashSet<String> = Default::default();
        let mut seen_arch: std::collections::HashSet<String> = Default::default();
        let mut seen_node: std::collections::HashSet<String> = Default::default();

        // Pass 1: one peer per unseen household.
        for c in &candidates {
            if picked.len() >= input.desired_count {
                break;
            }
            let hh = c
                .household_id
                .clone()
                .unwrap_or_else(|| format!("__unknown:{}", c.peer_id));
            if seen_hh.insert(hh) {
                if let Some(arch) = &c.archetype {
                    seen_arch.insert(arch.clone());
                }
                if let Some(n) = &c.node_id {
                    seen_node.insert(n.clone());
                }
                picked.push(c.clone());
            }
        }

        // Pass 2: fill with distinct archetypes.
        for c in &candidates {
            if picked.len() >= input.desired_count {
                break;
            }
            if picked.iter().any(|p| p.peer_id == c.peer_id) {
                continue;
            }
            if let Some(arch) = &c.archetype {
                if seen_arch.insert(arch.clone()) {
                    if let Some(n) = &c.node_id {
                        seen_node.insert(n.clone());
                    }
                    picked.push(c.clone());
                }
            }
        }

        // Pass 3: fill with distinct nodes.
        for c in &candidates {
            if picked.len() >= input.desired_count {
                break;
            }
            if picked.iter().any(|p| p.peer_id == c.peer_id) {
                continue;
            }
            if let Some(n) = &c.node_id {
                if seen_node.insert(n.clone()) {
                    picked.push(c.clone());
                }
            }
        }

        // Pass 4: fill from anything remaining.
        for c in &candidates {
            if picked.len() >= input.desired_count {
                break;
            }
            if picked.iter().any(|p| p.peer_id == c.peer_id) {
                continue;
            }
            picked.push(c.clone());
        }

        // ----------------------------------------------------------------
        // Step 7: Outcome.
        // ----------------------------------------------------------------
        if picked.len() >= input.desired_count {
            std::result::Result::Ok(SelectionOutcome::Ok(picked))
        } else {
            let achieved = picked.len() as i32;
            let requested = input.desired_count as i32;
            std::result::Result::Ok(SelectionOutcome::Short {
                peers: picked,
                gap_kind: "under-committed",
                achieved,
                requested,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::run_migrations;
    use diesel::r2d2::{ConnectionManager, Pool};
    use diesel::SqliteConnection;

    fn test_pool() -> DbPool {
        let url = format!(
            "file:peer_selection_staleness_test_{}?mode=memory&cache=shared",
            uuid::Uuid::new_v4().as_simple()
        );
        let pool = Pool::builder()
            .max_size(1)
            .build(ConnectionManager::<SqliteConnection>::new(&url))
            .expect("pool");
        run_migrations(&pool).expect("migrations");
        pool
    }

    /// Seed one accepting peer: an imagodei human, a `content:commons` provide
    /// commitment (lamad), and an online peer_status at the given timestamp.
    fn seed_peer(pool: &DbPool, agent: &str, ts_micros: i64) {
        let mut conn = pool.get().unwrap();
        crate::db::humans::create_human(
            &mut conn,
            crate::db::humans::CreateHumanInput {
                id: format!("human-{agent}"),
                agent_pub_key: Some(agent.to_string()),
                display_name: agent.to_string(),
                bio: None,
                affinities: "[]".to_string(),
                profile_reach: "commons".to_string(),
                location: None,
                profile_photo_url: None,
                h_app_id: crate::db::HUMANS_HAPP_ID.to_string(),
                household_id: None,
            },
        )
        .expect("seed human");

        diesel::insert_into(rea_commitments::table)
            .values((
                rea_commitments::id.eq(format!("cmt-{agent}-commons")),
                rea_commitments::h_app_id.eq("lamad"),
                rea_commitments::action.eq("provide"),
                rea_commitments::provider.eq(agent),
                rea_commitments::receiver.eq(""),
                rea_commitments::resource_classified_as.eq(Some("content:commons")),
                rea_commitments::state.eq("active"),
                rea_commitments::finished.eq(0),
                rea_commitments::created_at.eq("2026-04-19T00:00:00Z"),
            ))
            .execute(&mut conn)
            .expect("seed rea_commitment");

        diesel::insert_into(peer_statuses::table)
            .values((
                peer_statuses::peer_id.eq(agent),
                peer_statuses::status.eq("online"),
                peer_statuses::general_pool_member.eq(1),
                peer_statuses::accepting_stewardship_reserves.eq(1),
                peer_statuses::timestamp.eq(ts_micros),
                peer_statuses::dht_anchor_hash.eq("anchor-placeholder"),
                peer_statuses::updated_at.eq(ts_micros),
            ))
            .execute(&mut conn)
            .expect("seed peer_status");
    }

    fn commons_input() -> SelectionInput<'static> {
        SelectionInput {
            h_app_id: "lamad",
            content_id: "content-x",
            content_reach: "commons",
            desired_count: 2,
        }
    }

    fn picked_ids(outcome: &SelectionOutcome) -> Vec<String> {
        let peers = match outcome {
            SelectionOutcome::Ok(p) => p,
            SelectionOutcome::Short { peers, .. } => peers,
        };
        let mut ids: Vec<String> = peers.iter().map(|p| p.peer_id.clone()).collect();
        ids.sort();
        ids
    }

    /// A 900s window counts a peer whose heartbeat landed 60s ago and EXCLUDES one
    /// whose last snapshot is 901s old — the honesty guard against a fanned-in
    /// "online" from a peer that has since gone dark. Self rows heartbeat every
    /// 60s, so a 900s window never dims a live pod (the fresh row here stands in
    /// for that live cadence).
    #[test]
    fn staleness_window_excludes_a_stale_peer_but_keeps_a_fresh_one() {
        let pool = test_pool();
        let now = chrono::Utc::now().timestamp_micros();
        seed_peer(&pool, "uhCAkFRESH", now - 60 * 1_000_000); // 60s old — live
        seed_peer(&pool, "uhCAkSTALE", now - 901 * 1_000_000); // 901s old — dark

        let sel = PeerSelection::new(pool).with_staleness_secs(900);
        let outcome = sel.select(&commons_input()).unwrap();
        assert_eq!(
            picked_ids(&outcome),
            vec!["uhCAkFRESH".to_string()],
            "only the fresh peer is accepting under a 900s window"
        );
    }

    /// Window `0` (the `new()` default) keeps legacy behaviour: age is ignored, so
    /// both peers — including the 901s-old one — are selected.
    #[test]
    fn staleness_window_zero_keeps_legacy_age_agnostic_behaviour() {
        let pool = test_pool();
        let now = chrono::Utc::now().timestamp_micros();
        seed_peer(&pool, "uhCAkFRESH", now - 60 * 1_000_000);
        seed_peer(&pool, "uhCAkSTALE", now - 901 * 1_000_000);

        let sel = PeerSelection::new(pool); // window 0
        let outcome = sel.select(&commons_input()).unwrap();
        assert_eq!(
            picked_ids(&outcome),
            vec!["uhCAkFRESH".to_string(), "uhCAkSTALE".to_string()],
            "both peers count when the window is disabled"
        );
    }
}
