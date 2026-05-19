//! GraphQL resolvers — EprHead graph-backed + topology resolvers.
//!
//! `QueryRoot` is the schema entry point. Resolvers delegate to the injected
//! `Arc<GraphEngine>` via `ctx.data::<Arc<GraphEngine>>()`, or to the
//! `DbPool` + `Arc<Federator>` for topology resolvers.
//!
//! Hyper integration note: `ctx.data` is populated by `build_schema` (not by any
//! axum extractor), so the standard `async-graphql` macro surface works unchanged.

#![cfg(feature = "graph-native")]

use std::sync::Arc;

use async_graphql::{Context, Enum, FieldResult, Object, ID};
use cozo::DataValue;

use crate::db::DbPool;
use crate::graph::{engine::GraphEngine, primitives::scripts::NEIGHBORHOOD};
use crate::services::federator::Federator;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

/// An EPR Head node as seen through the GraphQL surface.
pub struct EprHead {
    pub cid: String,
}

#[Object]
impl EprHead {
    /// Content-address (CID) of this atom.
    async fn cid(&self) -> &str {
        &self.cid
    }

    /// Stable slug of this atom (looked up from epr_node).
    async fn slug(&self, ctx: &Context<'_>) -> FieldResult<String> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let res = engine.run_script(
            r#"?[slug] := *epr_node{cid: $cid, slug}"#,
            &[("cid", DataValue::from(self.cid.as_str()))],
        )?;
        Ok(res
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| match v {
                DataValue::Str(s) => Some(s.to_string()),
                _ => None,
            })
            .unwrap_or_default())
    }

    /// Prerequisite atoms reachable within `maxDepth` hops via any edge type.
    ///
    /// Uses the NEIGHBORHOOD Datalog primitive (any edge, not just PREREQUISITE) so
    /// that the neighbourhood walk reflects the full graph structure. A future sprint
    /// can narrow this to specific edge types via a `relType` argument.
    async fn prerequisites(
        &self,
        ctx: &Context<'_>,
        max_depth: Option<i32>,
    ) -> FieldResult<Vec<EprHead>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let depth = max_depth.unwrap_or(3) as i64;
        let script = format!(
            "{}\n?[to] := neighborhood[to, hops], hops <= $max_hops",
            NEIGHBORHOOD
        );
        let res = engine.run_script(
            &script,
            &[
                ("start", DataValue::from(self.cid.as_str())),
                ("max_hops", DataValue::from(depth)),
            ],
        )?;
        Ok(res
            .rows
            .iter()
            .filter_map(|r| r.first())
            .filter_map(|v| match v {
                DataValue::Str(s) => Some(EprHead { cid: s.to_string() }),
                _ => None,
            })
            .collect())
    }

    /// Atoms this atom explicitly teaches (TEACHES edges).
    async fn teaches(&self, ctx: &Context<'_>) -> FieldResult<Vec<EprHead>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let res = engine.run_script(
            r#"?[to] := *epr_edge{from_cid: $cid, to_cid: to, rel_type: "TEACHES"}"#,
            &[("cid", DataValue::from(self.cid.as_str()))],
        )?;
        Ok(res
            .rows
            .iter()
            .filter_map(|r| r.first())
            .filter_map(|v| match v {
                DataValue::Str(s) => Some(EprHead { cid: s.to_string() }),
                _ => None,
            })
            .collect())
    }
}

// ---------------------------------------------------------------------------
// Contributor (shefa)
// ---------------------------------------------------------------------------

/// A contributor node (identified by DID).
pub struct Contributor {
    pub did: String,
}

/// A household node (identified by CID).
pub struct Household {
    pub cid: String,
}

/// A reciprocity flow from one contributor to another.
pub struct ReciprocityFlow {
    pub from: String,
    pub amount: f64,
}

#[Object]
impl Contributor {
    async fn did(&self) -> &str {
        &self.did
    }

    async fn display_name(&self) -> Option<String> {
        // Display name is not yet stored in the graph projection; returns None
        // until the imagodei node is wired to the graph in a follow-on sprint.
        None
    }

    /// Household this contributor belongs to (via MEMBER_OF edge from contributor DID node).
    async fn household(&self, ctx: &Context<'_>) -> FieldResult<Option<Household>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        // MEMBER_OF edges are projected from shefa REA economic events.
        // The contributor DID is stored as `from_cid` when `rel_type = 'MEMBER_OF'`.
        let res = engine.run_script(
            r#"?[to] := *epr_edge{from_cid: $did, to_cid: to, rel_type: "MEMBER_OF"}"#,
            &[("did", DataValue::from(self.did.as_str()))],
        )?;
        Ok(res
            .rows
            .first()
            .and_then(|r| r.first())
            .and_then(|v| match v {
                DataValue::Str(s) => Some(Household { cid: s.to_string() }),
                _ => None,
            }))
    }

    /// Inbound reciprocity flows (RECIPROCATES_WITH edges pointing to this contributor).
    async fn reciprocity_inbound(&self, ctx: &Context<'_>) -> FieldResult<Vec<ReciprocityFlow>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let res = engine.run_script(
            r#"?[from_cid] := *epr_edge{from_cid: from_cid, to_cid: $did, rel_type: "RECIPROCATES_WITH"}"#,
            &[("did", DataValue::from(self.did.as_str()))],
        )?;
        Ok(res
            .rows
            .iter()
            .filter_map(|r| r.first())
            .filter_map(|v| match v {
                DataValue::Str(s) => Some(ReciprocityFlow {
                    from: s.to_string(),
                    amount: 1.0, // amount not yet stored in graph; placeholder
                }),
                _ => None,
            })
            .collect())
    }
}

#[Object]
impl Household {
    async fn cid(&self) -> &str {
        &self.cid
    }

    /// Members of this household (contributors with MEMBER_OF edges pointing here).
    async fn members(&self, ctx: &Context<'_>) -> FieldResult<Vec<Contributor>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        let res = engine.run_script(
            r#"?[from_cid] := *epr_edge{from_cid: from_cid, to_cid: $cid, rel_type: "MEMBER_OF"}"#,
            &[("cid", DataValue::from(self.cid.as_str()))],
        )?;
        Ok(res
            .rows
            .iter()
            .filter_map(|r| r.first())
            .filter_map(|v| match v {
                DataValue::Str(s) => Some(Contributor { did: s.to_string() }),
                _ => None,
            })
            .collect())
    }

    /// Devices operated by contributors in this household (OPERATES_DEVICE edges).
    async fn devices(&self, ctx: &Context<'_>) -> FieldResult<Vec<Device>> {
        let engine = ctx.data::<Arc<GraphEngine>>()?;
        // Device nodes are reached via: household → member → OPERATES_DEVICE → device
        let res = engine.run_script(
            r#"?[device_id] :=
                *epr_edge{from_cid: member, to_cid: $cid, rel_type: "MEMBER_OF"},
                *epr_edge{from_cid: member, to_cid: device_id, rel_type: "OPERATES_DEVICE"}"#,
            &[("cid", DataValue::from(self.cid.as_str()))],
        )?;
        Ok(res
            .rows
            .iter()
            .filter_map(|r| r.first())
            .filter_map(|v| match v {
                DataValue::Str(s) => Some(Device { id: s.to_string() }),
                _ => None,
            })
            .collect())
    }
}

/// A device operated by a contributor.
pub struct Device {
    pub id: String,
}

#[Object]
impl Device {
    async fn id(&self) -> &str {
        &self.id
    }

    /// Device metrics are not yet projected into the graph; always None.
    async fn metrics(&self) -> Option<String> {
        None
    }
}

#[Object]
impl ReciprocityFlow {
    async fn from(&self) -> &str {
        &self.from
    }

    async fn amount(&self) -> f64 {
        self.amount
    }
}

// ---------------------------------------------------------------------------
// Viewer root — topology resolvers (Epic A, Task A2)
// ---------------------------------------------------------------------------

/// Hardware/deployment archetype mirrored from `elohim_views::infrastructure::DeviceArchetype`.
/// The serde `snake_case` representation is preserved; async-graphql uses PascalCase by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum DeviceArchetypeGql {
    Node,
    Desktop,
    Mobile,
    Steward,
}

impl From<crate::views::DeviceArchetype> for DeviceArchetypeGql {
    fn from(a: crate::views::DeviceArchetype) -> Self {
        use crate::views::DeviceArchetype;
        match a {
            DeviceArchetype::Node => DeviceArchetypeGql::Node,
            DeviceArchetype::Desktop => DeviceArchetypeGql::Desktop,
            DeviceArchetype::Mobile => DeviceArchetypeGql::Mobile,
            DeviceArchetype::Steward => DeviceArchetypeGql::Steward,
        }
    }
}

/// Freshness state mirrored from `elohim_views::shared::FreshnessState`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Enum)]
pub enum FreshnessStateGql {
    Live,
    Stale,
    Offline,
    CachedOfflineUntilReconnect,
    Unverifiable,
    AllOffline,
}

impl From<crate::views::FreshnessState> for FreshnessStateGql {
    fn from(s: crate::views::FreshnessState) -> Self {
        use crate::views::FreshnessState;
        match s {
            FreshnessState::Live => FreshnessStateGql::Live,
            FreshnessState::Stale => FreshnessStateGql::Stale,
            FreshnessState::Offline => FreshnessStateGql::Offline,
            FreshnessState::CachedOfflineUntilReconnect => {
                FreshnessStateGql::CachedOfflineUntilReconnect
            }
            FreshnessState::Unverifiable => FreshnessStateGql::Unverifiable,
            FreshnessState::AllOffline => FreshnessStateGql::AllOffline,
        }
    }
}

/// Liveness / staleness indicator.
pub struct FreshnessGql {
    pub state: FreshnessStateGql,
    /// Non-null when state is Stale; the epoch-ms at which data became stale.
    pub stale_since_ms: Option<String>,
}

impl From<crate::views::Freshness> for FreshnessGql {
    fn from(f: crate::views::Freshness) -> Self {
        FreshnessGql {
            state: f.state.into(),
            stale_since_ms: f.stale_since_ms.map(|ms| ms.to_string()),
        }
    }
}

#[Object]
impl FreshnessGql {
    /// Freshness state bucket.
    async fn state(&self) -> FreshnessStateGql {
        self.state
    }

    /// Epoch-ms when data became stale; null when state is Live.
    async fn stale_since_ms(&self) -> Option<&str> {
        self.stale_since_ms.as_deref()
    }
}

/// Per-device summary in the hub view.
///
/// Byte counters are `String` to avoid JS integer-precision loss at 2^53.
/// Nullable counters indicate metrics were not yet available from the peer.
pub struct DeviceSummaryGql {
    pub peer_id: String,
    pub archetype: DeviceArchetypeGql,
    pub display_name: Option<String>,
    pub online: bool,
    pub freshness: FreshnessGql,
    pub storage_used_bytes: Option<String>,
    pub storage_total_bytes: Option<String>,
    pub memory_used_bytes: Option<String>,
    pub memory_total_bytes: Option<String>,
    pub hosting_count: Option<i32>,
    pub projecting_count: Option<i32>,
    pub beacon_age_ms: Option<String>,
}

impl From<crate::views::DeviceSummary> for DeviceSummaryGql {
    fn from(d: crate::views::DeviceSummary) -> Self {
        DeviceSummaryGql {
            peer_id: d.peer_id,
            archetype: d.archetype.into(),
            display_name: d.display_name,
            online: d.online,
            freshness: d.freshness.into(),
            storage_used_bytes: d.storage_used_bytes.map(|v| v.to_string()),
            storage_total_bytes: d.storage_total_bytes.map(|v| v.to_string()),
            memory_used_bytes: d.memory_used_bytes.map(|v| v.to_string()),
            memory_total_bytes: d.memory_total_bytes.map(|v| v.to_string()),
            hosting_count: d.hosting_count.map(|v| v as i32),
            projecting_count: d.projecting_count.map(|v| v as i32),
            beacon_age_ms: d.beacon_age_ms.map(|v| v.to_string()),
        }
    }
}

#[Object]
impl DeviceSummaryGql {
    async fn peer_id(&self) -> &str {
        &self.peer_id
    }

    async fn archetype(&self) -> DeviceArchetypeGql {
        self.archetype
    }

    async fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    async fn online(&self) -> bool {
        self.online
    }

    async fn freshness(&self) -> &FreshnessGql {
        &self.freshness
    }

    /// Storage used by this device (bytes, as string to avoid JS precision loss).
    async fn storage_used_bytes(&self) -> Option<&str> {
        self.storage_used_bytes.as_deref()
    }

    /// Storage capacity of this device (bytes).
    async fn storage_total_bytes(&self) -> Option<&str> {
        self.storage_total_bytes.as_deref()
    }

    /// Memory in use on this device (bytes).
    async fn memory_used_bytes(&self) -> Option<&str> {
        self.memory_used_bytes.as_deref()
    }

    /// Total memory capacity of this device (bytes).
    async fn memory_total_bytes(&self) -> Option<&str> {
        self.memory_total_bytes.as_deref()
    }

    /// Number of CIDs this device is currently hosting.
    async fn hosting_count(&self) -> Option<i32> {
        self.hosting_count
    }

    /// Number of CIDs this device is currently projecting.
    async fn projecting_count(&self) -> Option<i32> {
        self.projecting_count
    }

    /// Age of the last beacon from this device (ms), null when not available.
    async fn beacon_age_ms(&self) -> Option<&str> {
        self.beacon_age_ms.as_deref()
    }
}

/// Aggregated totals across the agent's devices.
pub struct DeviceTotalsGql {
    pub storage_used_bytes: String,
    pub storage_total_bytes: String,
    pub external_committed_bytes: String,
    pub reciprocity_net_bytes: String,
}

impl From<crate::views::DeviceTotals> for DeviceTotalsGql {
    fn from(t: crate::views::DeviceTotals) -> Self {
        DeviceTotalsGql {
            storage_used_bytes: t.storage_used_bytes.to_string(),
            storage_total_bytes: t.storage_total_bytes.to_string(),
            external_committed_bytes: t.external_committed_bytes.to_string(),
            reciprocity_net_bytes: t.reciprocity_net_bytes.to_string(),
        }
    }
}

#[Object]
impl DeviceTotalsGql {
    /// Total storage used across all devices (bytes).
    async fn storage_used_bytes(&self) -> &str {
        &self.storage_used_bytes
    }

    /// Total storage capacity across all devices (bytes).
    async fn storage_total_bytes(&self) -> &str {
        &self.storage_total_bytes
    }

    /// Bytes committed externally (to other agents' devices) via REA commitments.
    async fn external_committed_bytes(&self) -> &str {
        &self.external_committed_bytes
    }

    /// Signed net byte reciprocity (positive = net recipient, negative = net donor).
    async fn reciprocity_net_bytes(&self) -> &str {
        &self.reciprocity_net_bytes
    }
}

/// Hub view — the agent's federated cluster of devices.
///
/// The GraphQL surface uses "hub" vocabulary (not "cluster") to align with the
/// protocol's hub-archetype abstraction. The underlying Rust service is still
/// `aggregate_my_cluster_view`; renaming the Rust struct is a separate sweep.
pub struct HubView {
    pub agent_cid: String,
    pub devices: Vec<DeviceSummaryGql>,
    pub totals: DeviceTotalsGql,
    pub freshness: FreshnessGql,
}

impl From<crate::views::MyClusterView> for HubView {
    fn from(v: crate::views::MyClusterView) -> Self {
        HubView {
            agent_cid: v.agent_cid,
            devices: v.devices.into_iter().map(DeviceSummaryGql::from).collect(),
            totals: v.totals.into(),
            freshness: v.freshness.into(),
        }
    }
}

#[Object]
impl HubView {
    /// Agent CID for which this hub view was aggregated.
    async fn agent_cid(&self) -> &str {
        &self.agent_cid
    }

    /// Per-device summaries for every peer-identity binding of this agent.
    async fn devices(&self) -> &[DeviceSummaryGql] {
        &self.devices
    }

    /// Aggregate totals across all devices.
    async fn totals(&self) -> &DeviceTotalsGql {
        &self.totals
    }

    /// Roll-up freshness state: `Live` when bindings are empty; `AllOffline`
    /// when all devices are offline (common under `P2PHandle::for_testing()`).
    async fn freshness(&self) -> &FreshnessGql {
        &self.freshness
    }
}

// ---------------------------------------------------------------------------
// Viewer root — peers lens (Epic A, Task A3)
// ---------------------------------------------------------------------------

/// Per-household reciprocation edge in the peer topology view.
///
/// `last_sync_sec` and `net_diff` are `String` to preserve precision across the
/// JS boundary (u64/i64 can exceed 2^53). `u32` CID counts fit `Int!` cleanly.
pub struct PeerHouseholdEdgeGql {
    pub household_id: String,
    pub display_name: Option<String>,
    pub online: bool,
    pub last_sync_sec: Option<String>,
    pub my_cids_hosted_by_them: i32,
    pub their_cids_hosted_by_me: i32,
    pub net_diff: Option<String>,
    pub is_critical_for_me: Option<bool>,
    pub i_am_critical_for_them: Option<bool>,
}

impl From<crate::views::PeerHouseholdEdge> for PeerHouseholdEdgeGql {
    fn from(e: crate::views::PeerHouseholdEdge) -> Self {
        PeerHouseholdEdgeGql {
            household_id: e.household_id,
            display_name: e.display_name,
            online: e.online,
            last_sync_sec: e.last_sync_sec.map(|v| v.to_string()),
            my_cids_hosted_by_them: e.my_cids_hosted_by_them as i32,
            their_cids_hosted_by_me: e.their_cids_hosted_by_me as i32,
            net_diff: e.net_diff.map(|v| v.to_string()),
            is_critical_for_me: e.is_critical_for_me,
            i_am_critical_for_them: e.i_am_critical_for_them,
        }
    }
}

#[Object]
impl PeerHouseholdEdgeGql {
    /// Household identifier for this peer edge.
    async fn household_id(&self) -> &str {
        &self.household_id
    }

    /// Display name of the peer household, if available.
    async fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Whether this peer was online at aggregation time.
    async fn online(&self) -> bool {
        self.online
    }

    /// Seconds since last successful sync with this peer; null when not yet synced.
    async fn last_sync_sec(&self) -> Option<&str> {
        self.last_sync_sec.as_deref()
    }

    /// Number of this agent's CIDs currently hosted by this peer.
    async fn my_cids_hosted_by_them(&self) -> i32 {
        self.my_cids_hosted_by_them
    }

    /// Number of this peer's CIDs currently hosted by this agent.
    async fn their_cids_hosted_by_me(&self) -> i32 {
        self.their_cids_hosted_by_me
    }

    /// Signed net CID diff (positive = net recipient, negative = net donor);
    /// null when either side has not been probed.
    async fn net_diff(&self) -> Option<&str> {
        self.net_diff.as_deref()
    }

    /// Whether this peer is a sole replicator for at least one of this agent's CIDs.
    async fn is_critical_for_me(&self) -> Option<bool> {
        self.is_critical_for_me
    }

    /// Whether this agent is a sole replicator for at least one of this peer's CIDs.
    async fn i_am_critical_for_them(&self) -> Option<bool> {
        self.i_am_critical_for_them
    }
}

/// Sole-replica risk row — a peer household that is the only replicator of
/// one or more CIDs this agent cares about.
pub struct ResilienceCliffGql {
    pub household_id: String,
    pub sole_replica_cid_count: i32,
}

impl From<crate::views::ResilienceCliff> for ResilienceCliffGql {
    fn from(c: crate::views::ResilienceCliff) -> Self {
        ResilienceCliffGql {
            household_id: c.household_id,
            sole_replica_cid_count: c.sole_replica_cid_count as i32,
        }
    }
}

#[Object]
impl ResilienceCliffGql {
    /// Household identifier of the sole replicator.
    async fn household_id(&self) -> &str {
        &self.household_id
    }

    /// Number of CIDs for which this household is the only replicator.
    async fn sole_replica_cid_count(&self) -> i32 {
        self.sole_replica_cid_count
    }
}

/// Peer topology view — reciprocation edges + resilience cliffs for this agent.
///
/// The GraphQL field is `viewer.peers` (the `viewer` wrapper already implies
/// "mine"; "peers" reads naturally as "households I exchange with"). The type
/// name `PeerTopology` preserves the intrinsically topological shape.
pub struct PeerTopology {
    pub agent_cid: String,
    pub edges: Vec<PeerHouseholdEdgeGql>,
    pub reciprocation_count: i32,
    pub resilience_cliffs: Vec<ResilienceCliffGql>,
    pub freshness: FreshnessGql,
}

impl From<crate::views::PeerTopologyView> for PeerTopology {
    fn from(v: crate::views::PeerTopologyView) -> Self {
        PeerTopology {
            agent_cid: v.agent_cid,
            edges: v
                .edges
                .into_iter()
                .map(PeerHouseholdEdgeGql::from)
                .collect(),
            reciprocation_count: v.reciprocation_count as i32,
            resilience_cliffs: v
                .resilience_cliffs
                .into_iter()
                .map(ResilienceCliffGql::from)
                .collect(),
            freshness: v.freshness.into(),
        }
    }
}

#[Object]
impl PeerTopology {
    /// Agent CID for which this peer topology was aggregated.
    async fn agent_cid(&self) -> &str {
        &self.agent_cid
    }

    /// Reciprocation edges — one per distinct peer household that returned data.
    async fn edges(&self) -> &[PeerHouseholdEdgeGql] {
        &self.edges
    }

    /// Number of peer households where `online == true` (live reciprocation count).
    async fn reciprocation_count(&self) -> i32 {
        self.reciprocation_count
    }

    /// Sole-replica risk rows; empty when no CIDs are at cliff risk.
    async fn resilience_cliffs(&self) -> &[ResilienceCliffGql] {
        &self.resilience_cliffs
    }

    /// Roll-up freshness state: `Live` when bindings are empty; `AllOffline`
    /// when all peers are offline (common under `P2PHandle::for_testing()`).
    async fn freshness(&self) -> &FreshnessGql {
        &self.freshness
    }
}

/// Viewer root — agent-scoped lens over hub topology.
///
/// Holds only the agent CID; actual data is resolved lazily per field.
pub struct Viewer {
    pub agent_cid: String,
}

#[Object]
impl Viewer {
    /// The agent CID this viewer is scoped to.
    async fn agent_cid(&self) -> &str {
        &self.agent_cid
    }

    /// Federated hub view: devices, totals, and freshness for this agent.
    async fn hub(&self, ctx: &Context<'_>) -> FieldResult<HubView> {
        let pool = ctx.data::<DbPool>()?;
        let federator = ctx.data::<Arc<Federator>>()?;
        let view = crate::services::cluster_view::aggregate_my_cluster_view(
            pool,
            federator,
            &self.agent_cid,
        )
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(HubView::from(view))
    }

    /// Peer topology view: reciprocation edges, resilience cliffs, and freshness
    /// for households this agent exchanges with.
    async fn peers(&self, ctx: &Context<'_>) -> FieldResult<PeerTopology> {
        let pool = ctx.data::<DbPool>()?;
        let federator = ctx.data::<Arc<Federator>>()?;
        let view = crate::services::peer_topology_view::aggregate_peer_topology_view(
            pool,
            federator,
            &self.agent_cid,
        )
        .await
        .map_err(|e| async_graphql::Error::new(e.to_string()))?;
        Ok(PeerTopology::from(view))
    }
}

// ---------------------------------------------------------------------------
// Query root
// ---------------------------------------------------------------------------

pub struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Look up an EPR Head by CID.
    async fn epr_head(&self, _ctx: &Context<'_>, cid: ID) -> FieldResult<Option<EprHead>> {
        Ok(Some(EprHead {
            cid: cid.to_string(),
        }))
    }

    /// Look up a contributor by DID for shefa graph traversal.
    async fn contributor(&self, _ctx: &Context<'_>, did: ID) -> FieldResult<Option<Contributor>> {
        Ok(Some(Contributor {
            did: did.to_string(),
        }))
    }

    /// Viewer root — agent-scoped hub and topology resolvers.
    ///
    /// `agentCid` identifies the agent whose hub view is returned.
    async fn viewer(&self, _ctx: &Context<'_>, agent_cid: ID) -> FieldResult<Viewer> {
        Ok(Viewer {
            agent_cid: agent_cid.to_string(),
        })
    }
}
