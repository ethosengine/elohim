//! GraphQL resolvers — EprHead graph-backed query resolvers.
//!
//! `QueryRoot` is the schema entry point. Resolvers delegate to the injected
//! `Arc<GraphEngine>` via `ctx.data::<Arc<GraphEngine>>()`.
//!
//! Hyper integration note: `ctx.data` is populated by `build_schema` (not by any
//! axum extractor), so the standard `async-graphql` macro surface works unchanged.

#![cfg(feature = "graph-native")]

use std::sync::Arc;

use async_graphql::{Context, FieldResult, Object, ID};
use cozo::DataValue;

use crate::graph::{engine::GraphEngine, primitives::scripts::NEIGHBORHOOD};

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
}
