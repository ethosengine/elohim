//! The holder-relation row — one materialized `(hub, agent, region)` tuple that
//! holds a content shard. This is the SINGLE relation the resiliency facing
//! folds over. Moved verbatim (v1) from
//! `elohim-storage::services::household_resilience`; the storage-side loader
//! (`load_holder_relation`, impure — takes `&mut conn`) STAYS in storage and
//! returns `Vec<HolderRow>`.
//!
//! The grouping dimension is `hub_id` — the HUB abstraction (a household,
//! dwelling, or collective), the unifying edge-key that reconciles the otherwise
//! divergent edge stores. v1 sources `hub_id` from `humans.household_id`;
//! dwelling/collective grouping populates the same field from those sources
//! without touching the folds. `hub_id` is `None` for a holder with no hub
//! (bucketed `unknown` by the regional fold; excluded by the stewarding fold,
//! which counts only non-null hubs — preserving prior behavior). The per-agent
//! dimension (`agent_id`) is RETAINED so intra-hub peer counts are a fold away,
//! not a new query.
//!
//! Design: `genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md` §3.
#[derive(Debug, Clone)]
pub struct HolderRow {
    pub hub_id: Option<String>,
    pub agent_id: String,
    pub region: Option<String>,
}
