//! Content-addressed derivation graph — how one blob was composed from another.
//!
//! This module is the domain-neutral primitive behind "how was every artifact
//! composed": a set of content-addressed [`CompositionNode`]s (each a [`BlobCid`]
//! with a domain-neutral [`ProjectionSource`] identity) joined by
//! [`DerivationEdge`]s that read "`derived` was composed/derived FROM `source`,
//! attributed to some model/agent".
//!
//! It is deliberately kind-agnostic and domain-agnostic — `eprfs-core` knows
//! nothing about skills, agents, hooks, or any specific runtime. A domain
//! adapter (e.g. `elohim-agent-adapter`) decides what each node *means* and which
//! blobs derive from which; the core only models and validates the shape.
//!
//! Every CID in a graph is a real [`BlobCid`] over real bytes
//! ([`BlobCid::compute`]) — never a re-implemented hash. A [`DerivationEdge`]
//! composes conceptually with `attestation.rs`: an edge is a CID-witnessed claim
//! ("`derived` came from `source`, per `attributedTo`"). The graph stays a clean
//! standalone structure rather than coupling to the attestation drafting flow.

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::address::BlobCid;
use crate::error::{EprfsError, Result};
use crate::projection::ProjectionSource;

/// The relation a [`DerivationEdge`] asserts between two content-addressed blobs.
/// Extensible: a domain adapter chooses the kind that fits its derivation; core
/// never enumerates every possible relation exhaustively.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DerivationKind {
    /// `derived` is a projection/materialization of `source` (e.g. a runtime
    /// file grown from a native package).
    Projection,
    /// `derived` was imported from `source` (an ingest of external bytes).
    Import,
    /// `derived` is an attestation about `source`.
    Attestation,
    /// Any other adapter-defined relation.
    Other,
}

/// A content-addressed artifact in a composition graph: the blob's fingerprint,
/// a domain-neutral source identity, and an opaque metadata bag the adapter owns.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionNode {
    pub cid: BlobCid,
    pub source: ProjectionSource,
    pub metadata: Value,
}

impl CompositionNode {
    pub fn new(cid: BlobCid, source: ProjectionSource, metadata: Value) -> Self {
        Self {
            cid,
            source,
            metadata,
        }
    }
}

/// A directed, content-addressed claim: "`derived` was composed/derived FROM
/// `source`", attributed to the model/agent that composed it (the `composedBy`
/// role). The metadata bag is adapter-owned.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DerivationEdge {
    /// The origin blob — what `derived` was composed from.
    pub source: BlobCid,
    /// The resulting blob — composed/derived from `source`.
    pub derived: BlobCid,
    pub relation: DerivationKind,
    /// The model/agent credited with the composition (the `composedBy` role).
    pub attributed_to: String,
    pub metadata: Value,
}

impl DerivationEdge {
    pub fn new(
        source: BlobCid,
        derived: BlobCid,
        relation: DerivationKind,
        attributed_to: impl Into<String>,
        metadata: Value,
    ) -> Self {
        Self {
            source,
            derived,
            relation,
            attributed_to: attributed_to.into(),
            metadata,
        }
    }

    /// Convenience constructor for the common `Projection` edge (a projected
    /// artifact grown from an origin blob).
    pub fn projection(
        source: BlobCid,
        derived: BlobCid,
        attributed_to: impl Into<String>,
        metadata: Value,
    ) -> Self {
        Self::new(
            source,
            derived,
            DerivationKind::Projection,
            attributed_to,
            metadata,
        )
    }
}

/// A content-addressed derivation graph: nodes (blobs) joined by derivation
/// edges. Domain-neutral — the meaning of each node lives in its adapter-owned
/// metadata, not in the core.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompositionGraph {
    pub nodes: Vec<CompositionNode>,
    pub edges: Vec<DerivationEdge>,
    pub metadata: Value,
}

impl CompositionGraph {
    /// A fresh, empty graph with `Null` metadata.
    pub fn new() -> Self {
        Self::default()
    }

    /// Replace the graph-level metadata bag (builder style).
    pub fn with_metadata(mut self, metadata: Value) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn add_node(&mut self, node: CompositionNode) -> &mut Self {
        self.nodes.push(node);
        self
    }

    pub fn add_edge(&mut self, edge: DerivationEdge) -> &mut Self {
        self.edges.push(edge);
        self
    }

    /// Validate the invariants that must hold for any composition graph:
    ///
    /// - every edge's `source` and `derived` CID appears as a node,
    /// - every node's source identity carries a non-empty namespace + id.
    ///
    /// Duplicate node CIDs are intentionally NOT an error: identical bytes yield
    /// the same CID legitimately (the same blob reached by two derivations).
    pub fn validate(&self) -> Result<()> {
        let cids: HashSet<&BlobCid> = self.nodes.iter().map(|node| &node.cid).collect();

        for node in &self.nodes {
            if node.source.namespace.trim().is_empty() || node.source.id.trim().is_empty() {
                return Err(EprfsError::InvalidCompositionGraph(format!(
                    "composition node {} is missing namespace or id",
                    node.cid
                )));
            }
        }

        for edge in &self.edges {
            if !cids.contains(&edge.source) {
                return Err(EprfsError::InvalidCompositionGraph(format!(
                    "edge source cid {} is not a node in the graph",
                    edge.source
                )));
            }
            if !cids.contains(&edge.derived) {
                return Err(EprfsError::InvalidCompositionGraph(format!(
                    "edge derived cid {} is not a node in the graph",
                    edge.derived
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::projection::ProjectionSourceKind;

    fn node(bytes: &[u8], id: &str) -> CompositionNode {
        CompositionNode::new(
            BlobCid::compute(bytes),
            ProjectionSource::new("test", ProjectionSourceKind::Content, id),
            Value::Null,
        )
    }

    #[test]
    fn valid_graph_passes() {
        let pkg = node(b"package-bytes", "Skill:alpha");
        let proj = node(b"projection-bytes", "Skill:alpha@claude");
        let mut graph = CompositionGraph::new();
        graph.add_node(pkg.clone()).add_node(proj.clone());
        graph.add_edge(DerivationEdge::projection(
            pkg.cid.clone(),
            proj.cid.clone(),
            "claude-opus-4-8",
            Value::Null,
        ));

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn edge_referencing_absent_node_is_rejected() {
        let pkg = node(b"package-bytes", "Skill:alpha");
        // the derived blob's node is never added to the graph
        let orphan_derived = BlobCid::compute(b"orphan-projection");
        let mut graph = CompositionGraph::new();
        graph.add_node(pkg.clone());
        graph.add_edge(DerivationEdge::projection(
            pkg.cid.clone(),
            orphan_derived,
            "claude-opus-4-8",
            Value::Null,
        ));

        assert!(matches!(
            graph.validate(),
            Err(EprfsError::InvalidCompositionGraph(_))
        ));
    }

    #[test]
    fn duplicate_node_cids_are_allowed() {
        // Same bytes => same CID legitimately (one blob, two derivations into it).
        let a = node(b"shared-bytes", "Skill:a");
        let b = node(b"shared-bytes", "Skill:b");
        assert_eq!(a.cid, b.cid);

        let src = node(b"origin", "Skill:origin");
        let mut graph = CompositionGraph::new();
        graph.add_node(src.clone()).add_node(a.clone()).add_node(b);
        graph.add_edge(DerivationEdge::projection(
            src.cid.clone(),
            a.cid.clone(),
            "claude-opus-4-8",
            Value::Null,
        ));

        assert!(graph.validate().is_ok());
    }

    #[test]
    fn empty_node_identity_is_rejected() {
        let bad = CompositionNode::new(
            BlobCid::compute(b"x"),
            ProjectionSource::new("", ProjectionSourceKind::Content, "id"),
            Value::Null,
        );
        let mut graph = CompositionGraph::new();
        graph.add_node(bad);
        assert!(matches!(
            graph.validate(),
            Err(EprfsError::InvalidCompositionGraph(_))
        ));
    }

    #[test]
    fn derivation_kind_serializes_kebab_case() {
        assert_eq!(
            serde_json::to_string(&DerivationKind::Projection).unwrap(),
            "\"projection\""
        );
        assert_eq!(
            serde_json::to_string(&DerivationKind::Attestation).unwrap(),
            "\"attestation\""
        );
    }

    #[test]
    fn edge_wire_shape_is_camel_case() {
        let edge = DerivationEdge::projection(
            BlobCid::compute(b"a"),
            BlobCid::compute(b"b"),
            "claude-opus-4-8",
            Value::Null,
        );
        let json = serde_json::to_value(&edge).unwrap();
        assert!(json.get("attributedTo").is_some());
        assert_eq!(json["relation"], "projection");
        assert_eq!(json["attributedTo"], "claude-opus-4-8");
    }

    #[test]
    fn graph_round_trips_through_json() {
        let pkg = node(b"package-bytes", "Skill:alpha");
        let proj = node(b"projection-bytes", "Skill:alpha@claude");
        let mut graph =
            CompositionGraph::new().with_metadata(serde_json::json!({"composedBy": "x"}));
        graph.add_node(pkg.clone()).add_node(proj.clone());
        graph.add_edge(DerivationEdge::projection(
            pkg.cid.clone(),
            proj.cid.clone(),
            "x",
            Value::Null,
        ));

        let json = serde_json::to_string(&graph).unwrap();
        let back: CompositionGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(graph, back);
        assert!(back.validate().is_ok());
    }
}
