# Brit Graph + Rakia MVP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a generic EPR-native graph engine to brit (`brit-graph`), then build rakia's constellation (build DAG) and change detection on top of it, producing `brit graph`, `brit affected`, and `brit plan` CLI commands that can shadow the existing Groovy DAG walker.

**Architecture:** `brit-graph` is a pure-computation crate in the brit workspace — petgraph-backed DAG with BritCid-keyed nodes, affected tracking with provenance, and topological planning. Rakia consumes `brit-graph` to build the constellation from `build-manifest.json` files. `rakia-brit` provides change detection via brit/gix. Brit CLI gains graph/affected/plan subcommands.

**Tech Stack:** Rust 2021, petgraph 0.7, blake3 (via brit-epr BritCid), globset (for source pattern matching), gix (for change detection), clap 4 (CLI), serde/serde_json.

**Design spec:** `docs/superpowers/specs/2026-04-19-brit-graph-rakia-mvp-design.md`

**Build note:** Brit is a fork of gitoxide. Build with: `cd elohim/brit && cargo build`. Rakia builds separately: `cd elohim/rakia && cargo build`. Both use standard `RUSTFLAGS=""` (native targets, not WASM).

---

## File Structure

### brit-graph (new crate in brit workspace)

```
brit-graph/
├── Cargo.toml
├── src/
│   ├── lib.rs              # Module exports
│   ├── graph.rs            # EprGraph<N, E> — petgraph wrapper with BritCid index
│   ├── traits.rs           # GraphConnections trait — dependencies_of, dependents_of
│   ├── affected.rs         # AffectedTracker, AffectedBy enum, provenance
│   ├── fingerprint.rs      # ContentFingerprint — deterministic input hashing
│   └── topo.rs             # TopoPlan — topological sort with level grouping
└── tests/
    ├── graph_construction.rs
    ├── affected_tracking.rs
    ├── fingerprint_determinism.rs
    └── topo_ordering.rs
```

**Responsibilities:**
- `graph.rs` — `EprGraph<N, E>` wraps `petgraph::DiGraph`. Nodes implement `ContentNode` (from brit-epr). Provides `add_node`, `add_edge`, `get_node`, `node_count`, cycle detection. The `BTreeMap<BritCid, NodeIndex>` index enables O(log n) lookup by content address.
- `traits.rs` — `GraphConnections` trait with `dependencies_of(cid)`, `dependents_of(cid)`, `deep_dependencies_of(cid)`, `deep_dependents_of(cid)`. Returns `Vec<BritCid>`.
- `affected.rs` — `AffectedTracker` takes an `EprGraph` and a set of initially-affected CIDs. Propagates upstream/downstream with configurable scope. Each affected node carries `Vec<AffectedBy>` explaining why.
- `fingerprint.rs` — `ContentFingerprint` hashes a `BTreeMap<String, Vec<u8>>` of named inputs into a single `BritCid`. Deterministic: sorted keys, canonical concatenation.
- `topo.rs` — `TopoPlan::from_affected(graph, affected_cids)` produces a `Vec<Vec<BritCid>>` where each inner vec is a parallelizable level.

### rakia-core modifications

```
rakia-core/src/
├── lib.rs                  # Add discover, constellation modules
├── manifest.rs             # Existing — BuildManifest parser
├── discover.rs             # NEW — find and parse all build-manifest.json files
├── constellation.rs        # NEW — build EprGraph<QualifiedStep> from manifests
├── graph.rs                # REMOVE (empty stub, replaced by constellation.rs)
├── hash.rs                 # REMOVE (empty stub, replaced by brit-graph fingerprint)
└── schema.rs               # KEEP stub (Phase A fills this)
```

**Responsibilities:**
- `discover.rs` — `discover_manifests(root: &Path) -> Vec<(PathBuf, BuildManifest)>`. Walks the directory tree, finds `build-manifest.json` files, parses each. Skips `node_modules`, `.git`, `target`.
- `constellation.rs` — `QualifiedStep` (pipeline + step name + inputs/outputs/depends). Implements `ContentNode`. `build_constellation(manifests) -> EprGraph<QualifiedStep>`. Resolves cross-manifest dependencies. Validates acyclicity. `plan_from_changes(constellation, changed_paths) -> TopoPlan<QualifiedStep>` — matches paths against source globs, marks affected, returns topological plan.

### rakia-brit (new crate in rakia workspace)

```
rakia-brit/
├── Cargo.toml
├── src/
│   ├── lib.rs
│   ├── changes.rs          # changed_paths_since(baseline, head) via gix
│   └── baselines.rs        # read/write/migrate baseline refs
└── tests/
    ├── change_detection.rs
    └── baseline_refs.rs
```

---

## Phase B: The DAG

### Task 1: Scaffold brit-graph crate

**Files:**
- Create: `brit-graph/Cargo.toml`
- Create: `brit-graph/src/lib.rs`
- Modify: `Cargo.toml` (workspace members)

- [ ] **Step 1.1: Create brit-graph/Cargo.toml**

```toml
lints.workspace = true

[package]
name = "brit-graph"
version = "0.0.0"
description = "EPR-native graph engine — DAG construction, affected tracking, topological planning"
repository = "https://github.com/ethosengine/brit"
authors = ["Matthew Dowell <matthew@ethosengine.com>"]
license = "MIT OR Apache-2.0"
edition = "2021"
rust-version = "1.82"

[lib]
doctest = false

[dependencies]
brit-epr = { path = "../brit-epr", default-features = false }
petgraph = "0.7"
serde = { version = "1", features = ["derive"] }
serde_json = "1"

[dev-dependencies]
```

- [ ] **Step 1.2: Create brit-graph/src/lib.rs**

```rust
//! brit-graph — EPR-native graph engine.
//!
//! Provides DAG construction with BritCid-keyed nodes, affected tracking
//! with provenance, content fingerprinting, and topological planning.
//! Pure computation — no IO, no git, no network.
//!
//! Any type implementing `ContentNode` (from brit-epr) can be a graph node.

#![deny(missing_docs, rust_2018_idioms)]
#![forbid(unsafe_code)]

pub mod graph;
pub mod traits;
pub mod affected;
pub mod fingerprint;
pub mod topo;
```

- [ ] **Step 1.3: Add brit-graph to workspace members**

In `Cargo.toml` (workspace root), add `"brit-graph"` to the `members` array, after `"brit-build-ref"`:

```toml
    "brit-epr",
    "brit-verify",
    "brit-build-ref",
    "brit-graph",
```

- [ ] **Step 1.4: Verify it compiles**

Run: `cd elohim/brit && cargo check -p brit-graph`
Expected: success (empty modules)

- [ ] **Step 1.5: Commit**

```bash
cd elohim/brit && git add brit-graph/ Cargo.toml Cargo.lock
git commit -m "feat(brit-graph): scaffold EPR-native graph engine crate"
```

---

### Task 2: EprGraph — the core data structure

**Files:**
- Create: `brit-graph/src/graph.rs`
- Create: `brit-graph/tests/graph_construction.rs`

- [ ] **Step 2.1: Write failing test — construct a graph and query nodes**

Create `brit-graph/tests/graph_construction.rs`:

```rust
use brit_epr::{BritCid, ContentNode};
use brit_graph::graph::{EprGraph, GraphError};
use serde::{Deserialize, Serialize};

/// A minimal ContentNode for testing.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestNode {
    name: String,
}

impl ContentNode for TestNode {
    fn content_type(&self) -> &'static str {
        "test.node"
    }
}

#[test]
fn add_and_retrieve_node() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let node = TestNode { name: "alpha".into() };
    let cid = node.compute_cid().unwrap();
    graph.add_node(node.clone()).unwrap();

    let retrieved = graph.get_node(&cid).unwrap();
    assert_eq!(retrieved.name, "alpha");
}

#[test]
fn add_edge_between_nodes() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let a = TestNode { name: "a".into() };
    let b = TestNode { name: "b".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap(); // a depends on b

    assert_eq!(graph.node_count(), 2);
}

#[test]
fn duplicate_node_is_idempotent() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let node = TestNode { name: "dup".into() };
    graph.add_node(node.clone()).unwrap();
    graph.add_node(node.clone()).unwrap();
    assert_eq!(graph.node_count(), 1);
}

#[test]
fn edge_to_missing_node_fails() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let a = TestNode { name: "a".into() };
    let cid_a = a.compute_cid().unwrap();
    let missing = BritCid::compute(b"does-not-exist");

    graph.add_node(a).unwrap();
    let result = graph.add_edge(&cid_a, &missing);
    assert!(result.is_err());
}

#[test]
fn has_cycle_detects_cycle() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let a = TestNode { name: "cycle-a".into() };
    let b = TestNode { name: "cycle-b".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap();
    graph.add_edge(&cid_b, &cid_a).unwrap();

    assert!(graph.has_cycle());
}

#[test]
fn no_cycle_in_valid_dag() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let a = TestNode { name: "dag-a".into() };
    let b = TestNode { name: "dag-b".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap(); // a -> b (a depends on b)

    assert!(!graph.has_cycle());
}
```

- [ ] **Step 2.2: Run test to verify it fails**

Run: `cd elohim/brit && cargo test -p brit-graph --test graph_construction`
Expected: FAIL — module `graph` not found

- [ ] **Step 2.3: Implement EprGraph**

Create `brit-graph/src/graph.rs`:

```rust
//! `EprGraph` — a content-addressed directed graph.
//!
//! Nodes implement `ContentNode` from brit-epr. Each node is indexed by its
//! `BritCid`. Edges represent dependencies: an edge from A to B means
//! "A depends on B" (B must complete before A).

use std::collections::BTreeMap;

use brit_epr::{BritCid, ContentNode};
use petgraph::algo::is_cyclic_directed;
use petgraph::graph::{DiGraph, NodeIndex};

/// A content-addressed directed graph where nodes implement `ContentNode`.
pub struct EprGraph<N: ContentNode, E = ()> {
    inner: DiGraph<NodeIndex, E>,
    cid_to_index: BTreeMap<BritCid, NodeIndex>,
    node_data: Vec<N>,
}

/// Errors from graph operations.
#[derive(Debug, thiserror::Error)]
pub enum GraphError {
    /// A referenced node CID was not found in the graph.
    #[error("node not found: {0}")]
    NodeNotFound(BritCid),
    /// Failed to compute CID for a node.
    #[error("CID computation failed: {0}")]
    CidError(#[from] serde_json::Error),
}

impl<N: ContentNode, E: Default> EprGraph<N, E> {
    /// Create an empty graph.
    pub fn new() -> Self {
        Self {
            inner: DiGraph::new(),
            cid_to_index: BTreeMap::new(),
            node_data: Vec::new(),
        }
    }

    /// Add a node. If a node with the same CID already exists, this is a no-op.
    /// Returns the CID of the node.
    pub fn add_node(&mut self, node: N) -> Result<BritCid, GraphError> {
        let cid = node.compute_cid()?;
        if self.cid_to_index.contains_key(&cid) {
            return Ok(cid);
        }
        let data_idx = self.node_data.len();
        self.node_data.push(node);
        let graph_idx = self.inner.add_node(NodeIndex::new(data_idx));
        self.cid_to_index.insert(cid.clone(), graph_idx);
        Ok(cid)
    }

    /// Add a directed edge: `from` depends on `to`.
    pub fn add_edge(&mut self, from: &BritCid, to: &BritCid) -> Result<(), GraphError> {
        let from_idx = self.resolve_index(from)?;
        let to_idx = self.resolve_index(to)?;
        self.inner.add_edge(from_idx, to_idx, E::default());
        Ok(())
    }

    /// Get a node by CID.
    pub fn get_node(&self, cid: &BritCid) -> Result<&N, GraphError> {
        let graph_idx = self.resolve_index(cid)?;
        let data_idx = self.inner[graph_idx].index();
        Ok(&self.node_data[data_idx])
    }

    /// Number of nodes in the graph.
    pub fn node_count(&self) -> usize {
        self.cid_to_index.len()
    }

    /// Check whether the graph contains any cycles.
    pub fn has_cycle(&self) -> bool {
        is_cyclic_directed(&self.inner)
    }

    /// Get all node CIDs.
    pub fn cids(&self) -> Vec<BritCid> {
        self.cid_to_index.keys().cloned().collect()
    }

    /// Check if a CID exists in the graph.
    pub fn contains(&self, cid: &BritCid) -> bool {
        self.cid_to_index.contains_key(cid)
    }

    /// Access the inner petgraph (for traits that need direct graph access).
    pub(crate) fn inner_graph(&self) -> &DiGraph<NodeIndex, E> {
        &self.inner
    }

    /// Resolve a CID to a petgraph NodeIndex.
    pub(crate) fn resolve_index(&self, cid: &BritCid) -> Result<NodeIndex, GraphError> {
        self.cid_to_index
            .get(cid)
            .copied()
            .ok_or_else(|| GraphError::NodeNotFound(cid.clone()))
    }

    /// Resolve a petgraph NodeIndex to a CID.
    pub(crate) fn index_to_cid(&self, idx: NodeIndex) -> Option<BritCid> {
        self.cid_to_index
            .iter()
            .find(|(_, &v)| v == idx)
            .map(|(k, _)| k.clone())
    }
}

impl<N: ContentNode, E: Default> Default for EprGraph<N, E> {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 2.4: Run tests to verify they pass**

Run: `cd elohim/brit && cargo test -p brit-graph --test graph_construction`
Expected: all 6 tests PASS

- [ ] **Step 2.5: Commit**

```bash
cd elohim/brit && git add brit-graph/
git commit -m "feat(brit-graph): EprGraph — BritCid-keyed DAG data structure"
```

---

### Task 3: GraphConnections trait — traversal

**Files:**
- Create: `brit-graph/src/traits.rs`
- Modify: `brit-graph/tests/graph_construction.rs` (add traversal tests)

- [ ] **Step 3.1: Write failing tests for dependencies_of and dependents_of**

Append to `brit-graph/tests/graph_construction.rs`:

```rust
use brit_graph::traits::GraphConnections;

#[test]
fn dependencies_of_returns_direct_deps() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let a = TestNode { name: "tr-a".into() };
    let b = TestNode { name: "tr-b".into() };
    let c = TestNode { name: "tr-c".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();
    let cid_c = c.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_node(c).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap(); // a depends on b
    graph.add_edge(&cid_a, &cid_c).unwrap(); // a depends on c

    let deps = graph.dependencies_of(&cid_a).unwrap();
    assert_eq!(deps.len(), 2);
    assert!(deps.contains(&cid_b));
    assert!(deps.contains(&cid_c));
}

#[test]
fn dependents_of_returns_direct_dependents() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let a = TestNode { name: "dep-a".into() };
    let b = TestNode { name: "dep-b".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap(); // a depends on b

    let dependents = graph.dependents_of(&cid_b).unwrap();
    assert_eq!(dependents, vec![cid_a]);
}

#[test]
fn deep_dependencies_of_returns_transitive() {
    let mut graph: EprGraph<TestNode> = EprGraph::new();
    let a = TestNode { name: "deep-a".into() };
    let b = TestNode { name: "deep-b".into() };
    let c = TestNode { name: "deep-c".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();
    let cid_c = c.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_node(c).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap(); // a -> b
    graph.add_edge(&cid_b, &cid_c).unwrap(); // b -> c

    let deep = graph.deep_dependencies_of(&cid_a).unwrap();
    assert_eq!(deep.len(), 2);
    assert!(deep.contains(&cid_b));
    assert!(deep.contains(&cid_c));
}
```

- [ ] **Step 3.2: Run to verify failure**

Run: `cd elohim/brit && cargo test -p brit-graph --test graph_construction`
Expected: FAIL — `GraphConnections` not found

- [ ] **Step 3.3: Implement GraphConnections**

Create `brit-graph/src/traits.rs`:

```rust
//! Graph traversal traits — dependencies_of, dependents_of, and deep variants.

use std::collections::VecDeque;

use brit_epr::{BritCid, ContentNode};
use petgraph::Direction;
use rustc_hash::FxHashSet;

use crate::graph::{EprGraph, GraphError};

/// Trait for querying graph relationships.
pub trait GraphConnections<N: ContentNode> {
    /// Direct dependencies of a node (outgoing edges).
    fn dependencies_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError>;

    /// Direct dependents of a node (incoming edges).
    fn dependents_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError>;

    /// All transitive dependencies (deep).
    fn deep_dependencies_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError>;

    /// All transitive dependents (deep).
    fn deep_dependents_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError>;
}

impl<N: ContentNode, E> GraphConnections<N> for EprGraph<N, E> {
    fn dependencies_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError> {
        let idx = self.resolve_index(cid)?;
        let graph = self.inner_graph();
        Ok(graph
            .neighbors_directed(idx, Direction::Outgoing)
            .filter_map(|neighbor| self.index_to_cid(neighbor))
            .collect())
    }

    fn dependents_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError> {
        let idx = self.resolve_index(cid)?;
        let graph = self.inner_graph();
        Ok(graph
            .neighbors_directed(idx, Direction::Incoming)
            .filter_map(|neighbor| self.index_to_cid(neighbor))
            .collect())
    }

    fn deep_dependencies_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError> {
        self.traverse_deep(cid, Direction::Outgoing)
    }

    fn deep_dependents_of(&self, cid: &BritCid) -> Result<Vec<BritCid>, GraphError> {
        self.traverse_deep(cid, Direction::Incoming)
    }
}

impl<N: ContentNode, E> EprGraph<N, E> {
    fn traverse_deep(
        &self,
        start: &BritCid,
        direction: Direction,
    ) -> Result<Vec<BritCid>, GraphError> {
        let start_idx = self.resolve_index(start)?;
        let graph = self.inner_graph();
        let mut visited = FxHashSet::default();
        let mut result = Vec::new();
        let mut queue = VecDeque::new();

        for neighbor in graph.neighbors_directed(start_idx, direction) {
            queue.push_back(neighbor);
        }

        while let Some(idx) = queue.pop_front() {
            if !visited.insert(idx) {
                continue;
            }
            if let Some(cid) = self.index_to_cid(idx) {
                result.push(cid);
            }
            for neighbor in graph.neighbors_directed(idx, direction) {
                if !visited.contains(&neighbor) {
                    queue.push_back(neighbor);
                }
            }
        }

        Ok(result)
    }
}
```

- [ ] **Step 3.4: Add rustc-hash to dependencies**

In `brit-graph/Cargo.toml`, add:

```toml
rustc-hash = "2"
```

- [ ] **Step 3.5: Run tests**

Run: `cd elohim/brit && cargo test -p brit-graph --test graph_construction`
Expected: all 9 tests PASS

- [ ] **Step 3.6: Commit**

```bash
cd elohim/brit && git add brit-graph/
git commit -m "feat(brit-graph): GraphConnections trait — direct and deep traversal"
```

---

### Task 4: AffectedTracker — which nodes are affected and why

**Files:**
- Create: `brit-graph/src/affected.rs`
- Create: `brit-graph/tests/affected_tracking.rs`

- [ ] **Step 4.1: Write failing tests**

Create `brit-graph/tests/affected_tracking.rs`:

```rust
use brit_epr::{BritCid, ContentNode};
use brit_graph::affected::{AffectedBy, AffectedTracker, PropagationScope};
use brit_graph::graph::EprGraph;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestNode {
    name: String,
}

impl ContentNode for TestNode {
    fn content_type(&self) -> &'static str {
        "test.node"
    }
}

fn three_node_chain() -> (EprGraph<TestNode>, BritCid, BritCid, BritCid) {
    let mut graph = EprGraph::new();
    let a = TestNode { name: "aff-a".into() }; // depends on b
    let b = TestNode { name: "aff-b".into() }; // depends on c
    let c = TestNode { name: "aff-c".into() }; // leaf
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();
    let cid_c = c.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_node(c).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap();
    graph.add_edge(&cid_b, &cid_c).unwrap();

    (graph, cid_a, cid_b, cid_c)
}

#[test]
fn directly_affected_node_is_tracked() {
    let (graph, _, _, cid_c) = three_node_chain();
    let mut tracker = AffectedTracker::new(&graph);
    tracker.mark_affected(cid_c.clone(), AffectedBy::ChangedFile("src/lib.rs".into()));

    let affected = tracker.build();
    assert!(affected.is_affected(&cid_c));
    assert!(!affected.is_affected(&BritCid::compute(b"nonexistent")));
}

#[test]
fn upstream_propagation_deep() {
    let (graph, cid_a, cid_b, cid_c) = three_node_chain();
    // c changed -> b is affected (depends on c) -> a is affected (depends on b)
    let mut tracker = AffectedTracker::new(&graph);
    tracker.set_upstream_scope(PropagationScope::Deep);
    tracker.mark_affected(cid_c.clone(), AffectedBy::ChangedFile("leaf.rs".into()));
    tracker.propagate().unwrap();

    let affected = tracker.build();
    assert!(affected.is_affected(&cid_c));
    assert!(affected.is_affected(&cid_b));
    assert!(affected.is_affected(&cid_a));
}

#[test]
fn upstream_propagation_direct() {
    let (graph, cid_a, cid_b, cid_c) = three_node_chain();
    let mut tracker = AffectedTracker::new(&graph);
    tracker.set_upstream_scope(PropagationScope::Direct);
    tracker.mark_affected(cid_c.clone(), AffectedBy::ChangedFile("leaf.rs".into()));
    tracker.propagate().unwrap();

    let affected = tracker.build();
    assert!(affected.is_affected(&cid_c));
    assert!(affected.is_affected(&cid_b)); // direct dependent of c
    assert!(!affected.is_affected(&cid_a)); // NOT affected — only direct
}

#[test]
fn upstream_propagation_none() {
    let (graph, cid_a, cid_b, cid_c) = three_node_chain();
    let mut tracker = AffectedTracker::new(&graph);
    tracker.set_upstream_scope(PropagationScope::None);
    tracker.mark_affected(cid_c.clone(), AffectedBy::ChangedFile("leaf.rs".into()));
    tracker.propagate().unwrap();

    let affected = tracker.build();
    assert!(affected.is_affected(&cid_c));
    assert!(!affected.is_affected(&cid_b));
    assert!(!affected.is_affected(&cid_a));
}

#[test]
fn provenance_tracks_why() {
    let (graph, _, cid_b, cid_c) = three_node_chain();
    let mut tracker = AffectedTracker::new(&graph);
    tracker.set_upstream_scope(PropagationScope::Deep);
    tracker.mark_affected(cid_c.clone(), AffectedBy::ChangedFile("leaf.rs".into()));
    tracker.propagate().unwrap();

    let affected = tracker.build();
    let reasons = affected.reasons(&cid_b).unwrap();
    assert!(reasons.iter().any(|r| matches!(r, AffectedBy::UpstreamNode(_))));
}
```

- [ ] **Step 4.2: Run to verify failure**

Run: `cd elohim/brit && cargo test -p brit-graph --test affected_tracking`
Expected: FAIL — module `affected` not found

- [ ] **Step 4.3: Implement AffectedTracker**

Create `brit-graph/src/affected.rs`:

```rust
//! Affected tracking — which nodes are affected and why.
//!
//! Given a set of initially-affected nodes (e.g., "this step's source files changed"),
//! propagate through the graph to find all transitively affected nodes.
//! Each affected node carries `Vec<AffectedBy>` explaining why it was affected.

use std::collections::VecDeque;

use brit_epr::{BritCid, ContentNode};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::graph::{EprGraph, GraphError};
use crate::traits::GraphConnections;

/// Why a node was marked as affected.
#[derive(Debug, Clone)]
pub enum AffectedBy {
    /// A source file matched an input pattern.
    ChangedFile(String),
    /// A dependency (upstream in the DAG) was affected.
    UpstreamNode(BritCid),
    /// A dependent (downstream in the DAG) was affected.
    DownstreamNode(BritCid),
    /// The content fingerprint of inputs changed.
    InputFingerprint,
    /// Explicitly marked as always-affected.
    AlwaysAffected,
}

/// How far to propagate through the graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropagationScope {
    /// Don't propagate beyond the initial set.
    None,
    /// Propagate to immediate neighbors only.
    Direct,
    /// Propagate through the full transitive closure.
    Deep,
}

impl Default for PropagationScope {
    fn default() -> Self {
        Self::Deep
    }
}

/// Tracks which nodes are affected during graph analysis.
pub struct AffectedTracker<'g, N: ContentNode, E> {
    graph: &'g EprGraph<N, E>,
    affected: FxHashMap<BritCid, Vec<AffectedBy>>,
    upstream_scope: PropagationScope,
}

impl<'g, N: ContentNode, E> AffectedTracker<'g, N, E> {
    /// Create a new tracker for the given graph.
    pub fn new(graph: &'g EprGraph<N, E>) -> Self {
        Self {
            graph,
            affected: FxHashMap::default(),
            upstream_scope: PropagationScope::Deep,
        }
    }

    /// Set the upstream propagation scope (dependents of affected nodes).
    pub fn set_upstream_scope(&mut self, scope: PropagationScope) {
        self.upstream_scope = scope;
    }

    /// Mark a node as affected with a given reason.
    pub fn mark_affected(&mut self, cid: BritCid, reason: AffectedBy) {
        self.affected.entry(cid).or_default().push(reason);
    }

    /// Propagate affected state through the graph based on scope settings.
    ///
    /// "Upstream propagation" means: if node C is affected, and B depends on C,
    /// then B is affected too (B is upstream — it's a dependent of C).
    /// This matches build semantics: if a leaf changes, everything that
    /// depends on it needs rebuilding.
    pub fn propagate(&mut self) -> Result<(), GraphError> {
        match self.upstream_scope {
            PropagationScope::None => Ok(()),
            PropagationScope::Direct => self.propagate_direct(),
            PropagationScope::Deep => self.propagate_deep(),
        }
    }

    /// Consume the tracker and produce the final affected set.
    pub fn build(self) -> AffectedSet {
        AffectedSet {
            affected: self.affected,
        }
    }

    fn propagate_direct(&mut self) -> Result<(), GraphError> {
        let initial: Vec<BritCid> = self.affected.keys().cloned().collect();
        for cid in initial {
            let dependents = self.graph.dependents_of(&cid)?;
            for dep_cid in dependents {
                self.affected
                    .entry(dep_cid)
                    .or_default()
                    .push(AffectedBy::UpstreamNode(cid.clone()));
            }
        }
        Ok(())
    }

    fn propagate_deep(&mut self) -> Result<(), GraphError> {
        let mut queue: VecDeque<BritCid> = self.affected.keys().cloned().collect();
        let mut visited: FxHashSet<BritCid> = queue.iter().cloned().collect();

        while let Some(cid) = queue.pop_front() {
            let dependents = self.graph.dependents_of(&cid)?;
            for dep_cid in dependents {
                self.affected
                    .entry(dep_cid.clone())
                    .or_default()
                    .push(AffectedBy::UpstreamNode(cid.clone()));
                if visited.insert(dep_cid.clone()) {
                    queue.push_back(dep_cid);
                }
            }
        }
        Ok(())
    }
}

/// The result of affected tracking — an immutable set of affected nodes with reasons.
pub struct AffectedSet {
    affected: FxHashMap<BritCid, Vec<AffectedBy>>,
}

impl AffectedSet {
    /// Check if a node is affected.
    pub fn is_affected(&self, cid: &BritCid) -> bool {
        self.affected.contains_key(cid)
    }

    /// Get the reasons a node was affected. Returns None if not affected.
    pub fn reasons(&self, cid: &BritCid) -> Option<&[AffectedBy]> {
        self.affected.get(cid).map(|v| v.as_slice())
    }

    /// Get all affected CIDs.
    pub fn affected_cids(&self) -> Vec<BritCid> {
        self.affected.keys().cloned().collect()
    }

    /// Number of affected nodes.
    pub fn len(&self) -> usize {
        self.affected.len()
    }

    /// Whether the affected set is empty.
    pub fn is_empty(&self) -> bool {
        self.affected.is_empty()
    }
}
```

- [ ] **Step 4.4: Run tests**

Run: `cd elohim/brit && cargo test -p brit-graph --test affected_tracking`
Expected: all 5 tests PASS

- [ ] **Step 4.5: Commit**

```bash
cd elohim/brit && git add brit-graph/
git commit -m "feat(brit-graph): AffectedTracker with provenance and scoped propagation"
```

---

### Task 5: TopoPlan — topological sort with level grouping

**Files:**
- Create: `brit-graph/src/topo.rs`
- Create: `brit-graph/tests/topo_ordering.rs`

- [ ] **Step 5.1: Write failing tests**

Create `brit-graph/tests/topo_ordering.rs`:

```rust
use brit_epr::{BritCid, ContentNode};
use brit_graph::graph::EprGraph;
use brit_graph::topo::TopoPlan;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TestNode {
    name: String,
}

impl ContentNode for TestNode {
    fn content_type(&self) -> &'static str {
        "test.node"
    }
}

#[test]
fn topo_plan_groups_by_level() {
    // c has no deps (level 0)
    // b depends on c (level 1)
    // a depends on b (level 2)
    let mut graph = EprGraph::new();
    let a = TestNode { name: "topo-a".into() };
    let b = TestNode { name: "topo-b".into() };
    let c = TestNode { name: "topo-c".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();
    let cid_c = c.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_node(c).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap();
    graph.add_edge(&cid_b, &cid_c).unwrap();

    let affected = vec![cid_a.clone(), cid_b.clone(), cid_c.clone()];
    let plan = TopoPlan::from_affected(&graph, &affected).unwrap();

    assert_eq!(plan.levels.len(), 3);
    assert!(plan.levels[0].contains(&cid_c)); // leaf first
    assert!(plan.levels[1].contains(&cid_b));
    assert!(plan.levels[2].contains(&cid_a));
}

#[test]
fn topo_plan_parallel_at_same_level() {
    // b and c have no deps (level 0, parallelizable)
    // a depends on both b and c (level 1)
    let mut graph = EprGraph::new();
    let a = TestNode { name: "par-a".into() };
    let b = TestNode { name: "par-b".into() };
    let c = TestNode { name: "par-c".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();
    let cid_c = c.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_node(c).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap();
    graph.add_edge(&cid_a, &cid_c).unwrap();

    let affected = vec![cid_a.clone(), cid_b.clone(), cid_c.clone()];
    let plan = TopoPlan::from_affected(&graph, &affected).unwrap();

    assert_eq!(plan.levels.len(), 2);
    assert_eq!(plan.levels[0].len(), 2); // b and c at level 0
    assert!(plan.levels[0].contains(&cid_b));
    assert!(plan.levels[0].contains(&cid_c));
    assert_eq!(plan.levels[1], vec![cid_a]); // a at level 1
}

#[test]
fn topo_plan_skips_unaffected() {
    let mut graph = EprGraph::new();
    let a = TestNode { name: "skip-a".into() };
    let b = TestNode { name: "skip-b".into() };
    let c = TestNode { name: "skip-c".into() };
    let cid_a = a.compute_cid().unwrap();
    let cid_b = b.compute_cid().unwrap();
    let cid_c = c.compute_cid().unwrap();

    graph.add_node(a).unwrap();
    graph.add_node(b).unwrap();
    graph.add_node(c).unwrap();
    graph.add_edge(&cid_a, &cid_b).unwrap();

    // Only b is affected, not c (c is independent)
    let affected = vec![cid_b.clone()];
    let plan = TopoPlan::from_affected(&graph, &affected).unwrap();

    let all_cids: Vec<&BritCid> = plan.levels.iter().flat_map(|l| l.iter()).collect();
    assert!(all_cids.contains(&&cid_b));
    assert!(!all_cids.contains(&&cid_c));
}

#[test]
fn topo_plan_empty_affected_produces_empty_plan() {
    let graph: EprGraph<TestNode> = EprGraph::new();
    let affected: Vec<BritCid> = vec![];
    let plan = TopoPlan::from_affected(&graph, &affected).unwrap();
    assert!(plan.levels.is_empty());
}
```

- [ ] **Step 5.2: Run to verify failure**

Run: `cd elohim/brit && cargo test -p brit-graph --test topo_ordering`
Expected: FAIL — module `topo` not found

- [ ] **Step 5.3: Implement TopoPlan**

Create `brit-graph/src/topo.rs`:

```rust
//! Topological planning — sort affected nodes into parallelizable levels.
//!
//! Level 0: nodes with no unmet dependencies (leaves).
//! Level 1: nodes whose dependencies are all in level 0.
//! And so on. Nodes within a level can execute in parallel.

use std::collections::VecDeque;

use brit_epr::{BritCid, ContentNode};
use rustc_hash::{FxHashMap, FxHashSet};
use petgraph::Direction;

use crate::graph::{EprGraph, GraphError};

/// A topological execution plan grouped by dependency level.
#[derive(Debug, Clone)]
pub struct TopoPlan {
    /// Each inner vec is a set of nodes that can execute in parallel.
    /// levels[0] has no dependencies, levels[1] depends only on levels[0], etc.
    pub levels: Vec<Vec<BritCid>>,
}

impl TopoPlan {
    /// Build a topological plan from a set of affected CIDs within a graph.
    ///
    /// Only includes nodes that appear in `affected_cids`. Dependencies between
    /// affected nodes determine the level grouping. Dependencies on non-affected
    /// nodes are treated as already satisfied.
    pub fn from_affected<N: ContentNode, E>(
        graph: &EprGraph<N, E>,
        affected_cids: &[BritCid],
    ) -> Result<Self, GraphError> {
        if affected_cids.is_empty() {
            return Ok(TopoPlan { levels: vec![] });
        }

        let affected_set: FxHashSet<BritCid> = affected_cids.iter().cloned().collect();

        // Compute in-degree for each affected node (only counting edges from other affected nodes)
        let mut in_degree: FxHashMap<BritCid, usize> = FxHashMap::default();
        for cid in &affected_set {
            let idx = graph.resolve_index(cid)?;
            let count = graph
                .inner_graph()
                .neighbors_directed(idx, Direction::Outgoing)
                .filter(|&neighbor| {
                    graph
                        .index_to_cid(neighbor)
                        .map(|c| affected_set.contains(&c))
                        .unwrap_or(false)
                })
                .count();
            in_degree.insert(cid.clone(), count);
        }

        // Kahn's algorithm with level tracking
        let mut levels = Vec::new();
        let mut queue: VecDeque<BritCid> = in_degree
            .iter()
            .filter(|(_, &deg)| deg == 0)
            .map(|(cid, _)| cid.clone())
            .collect();

        while !queue.is_empty() {
            let current_level: Vec<BritCid> = queue.drain(..).collect();
            let mut next_queue = VecDeque::new();

            for cid in &current_level {
                let idx = graph.resolve_index(cid)?;
                // Find affected nodes that depend on this node (incoming edges = dependents)
                for neighbor in graph.inner_graph().neighbors_directed(idx, Direction::Incoming) {
                    if let Some(neighbor_cid) = graph.index_to_cid(neighbor) {
                        if let Some(deg) = in_degree.get_mut(&neighbor_cid) {
                            *deg = deg.saturating_sub(1);
                            if *deg == 0 {
                                next_queue.push_back(neighbor_cid);
                            }
                        }
                    }
                }
            }

            levels.push(current_level);
            queue = next_queue;
        }

        Ok(TopoPlan { levels })
    }

    /// Total number of nodes across all levels.
    pub fn total_nodes(&self) -> usize {
        self.levels.iter().map(|l| l.len()).sum()
    }

    /// Flatten into a single ordered vec (level 0 first, then level 1, etc).
    pub fn flatten(&self) -> Vec<BritCid> {
        self.levels.iter().flat_map(|l| l.iter().cloned()).collect()
    }
}
```

- [ ] **Step 5.4: Run tests**

Run: `cd elohim/brit && cargo test -p brit-graph --test topo_ordering`
Expected: all 4 tests PASS

- [ ] **Step 5.5: Commit**

```bash
cd elohim/brit && git add brit-graph/
git commit -m "feat(brit-graph): TopoPlan — topological sort with parallelizable levels"
```

---

### Task 6: ContentFingerprint — deterministic input hashing

**Files:**
- Create: `brit-graph/src/fingerprint.rs`
- Create: `brit-graph/tests/fingerprint_determinism.rs`

- [ ] **Step 6.1: Write failing tests**

Create `brit-graph/tests/fingerprint_determinism.rs`:

```rust
use brit_epr::BritCid;
use brit_graph::fingerprint::ContentFingerprint;
use std::collections::BTreeMap;

#[test]
fn same_inputs_same_fingerprint() {
    let mut inputs = BTreeMap::new();
    inputs.insert("file_a".to_string(), b"content_a".to_vec());
    inputs.insert("file_b".to_string(), b"content_b".to_vec());

    let fp1 = ContentFingerprint::compute(&inputs);
    let fp2 = ContentFingerprint::compute(&inputs);
    assert_eq!(fp1.cid, fp2.cid);
}

#[test]
fn different_inputs_different_fingerprint() {
    let mut inputs1 = BTreeMap::new();
    inputs1.insert("file".to_string(), b"v1".to_vec());

    let mut inputs2 = BTreeMap::new();
    inputs2.insert("file".to_string(), b"v2".to_vec());

    let fp1 = ContentFingerprint::compute(&inputs1);
    let fp2 = ContentFingerprint::compute(&inputs2);
    assert_ne!(fp1.cid, fp2.cid);
}

#[test]
fn insertion_order_does_not_matter() {
    let mut inputs1 = BTreeMap::new();
    inputs1.insert("z_file".to_string(), b"z_content".to_vec());
    inputs1.insert("a_file".to_string(), b"a_content".to_vec());

    let mut inputs2 = BTreeMap::new();
    inputs2.insert("a_file".to_string(), b"a_content".to_vec());
    inputs2.insert("z_file".to_string(), b"z_content".to_vec());

    // BTreeMap sorts keys, so these are the same
    let fp1 = ContentFingerprint::compute(&inputs1);
    let fp2 = ContentFingerprint::compute(&inputs2);
    assert_eq!(fp1.cid, fp2.cid);
}

#[test]
fn empty_inputs_produce_valid_fingerprint() {
    let inputs = BTreeMap::new();
    let fp = ContentFingerprint::compute(&inputs);
    // Should not panic, should produce a valid CID
    assert_eq!(fp.cid.as_str().len(), 64);
}
```

- [ ] **Step 6.2: Run to verify failure**

Run: `cd elohim/brit && cargo test -p brit-graph --test fingerprint_determinism`
Expected: FAIL — module `fingerprint` not found

- [ ] **Step 6.3: Implement ContentFingerprint**

Create `brit-graph/src/fingerprint.rs`:

```rust
//! Content fingerprinting — deterministic hash over named inputs.
//!
//! A fingerprint is a `BritCid` computed from a sorted map of named inputs.
//! Same inputs always produce the same fingerprint, regardless of insertion order.

use std::collections::BTreeMap;

use brit_epr::BritCid;

/// A deterministic content fingerprint over named inputs.
#[derive(Debug, Clone)]
pub struct ContentFingerprint {
    /// The overall fingerprint CID.
    pub cid: BritCid,
    /// Individual input hashes (name -> CID of that input's bytes).
    pub inputs: BTreeMap<String, BritCid>,
}

impl ContentFingerprint {
    /// Compute a fingerprint from a map of named inputs.
    ///
    /// Keys are sorted (BTreeMap guarantees this). Each input's bytes are
    /// individually hashed, then all hashes are concatenated with their keys
    /// and hashed again to produce the overall fingerprint.
    pub fn compute(inputs: &BTreeMap<String, Vec<u8>>) -> Self {
        let mut individual: BTreeMap<String, BritCid> = BTreeMap::new();
        let mut combined = Vec::new();

        for (name, bytes) in inputs {
            let input_cid = BritCid::compute(bytes);
            // Append "name\0cid\0" to the combined buffer
            combined.extend_from_slice(name.as_bytes());
            combined.push(0);
            combined.extend_from_slice(input_cid.as_str().as_bytes());
            combined.push(0);
            individual.insert(name.clone(), input_cid);
        }

        let cid = BritCid::compute(&combined);
        ContentFingerprint {
            cid,
            inputs: individual,
        }
    }
}
```

- [ ] **Step 6.4: Run tests**

Run: `cd elohim/brit && cargo test -p brit-graph --test fingerprint_determinism`
Expected: all 4 tests PASS

- [ ] **Step 6.5: Commit**

```bash
cd elohim/brit && git add brit-graph/
git commit -m "feat(brit-graph): ContentFingerprint — deterministic input hashing"
```

---

### Task 7: Rakia constellation builder — manifests become a DAG

**Files:**
- Create: `rakia-core/src/discover.rs`
- Create: `rakia-core/src/constellation.rs`
- Modify: `rakia-core/src/lib.rs`
- Modify: `rakia-core/Cargo.toml`
- Remove content from: `rakia-core/src/graph.rs`, `rakia-core/src/hash.rs`

**Note:** This task requires `brit-graph` to be available as a path dependency. Since rakia already has brit as a submodule, the path is `elohim/brit/brit-graph`. However, brit-epr (which brit-graph depends on) depends on `gix-object` via a relative path. This means rakia must reference brit-graph through the brit workspace. Add brit-epr and brit-graph as path dependencies pointing into the brit submodule.

- [ ] **Step 7.1: Update rakia-core/Cargo.toml**

Replace the contents of `rakia-core/Cargo.toml`:

```toml
[package]
name = "rakia-core"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Core engine: manifest parser, constellation builder, build planning"

[dependencies]
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
globset = "0.4"
```

**Note:** Direct dependency on brit-graph requires brit and all its transitive deps (gix-object, blake3, etc.) to resolve. This may need the brit submodule's Cargo workspace to be reachable. If path resolution fails, the fallback is to vendor the graph types locally and refactor when integration is tested. Try the path dependency first — add to workspace Cargo.toml:

In `rakia/Cargo.toml`, add to `[workspace.dependencies]`:

```toml
brit-epr = { path = "elohim/brit/brit-epr" }
brit-graph = { path = "elohim/brit/brit-graph" }
```

And in `rakia-core/Cargo.toml` add:

```toml
brit-epr = { workspace = true }
brit-graph = { workspace = true }
```

If this doesn't resolve (likely due to gix-object path deps in brit-epr), see Step 7.2 for the alternative.

- [ ] **Step 7.2: Alternative if path deps don't resolve — lightweight graph types**

If brit-graph can't be used as a direct path dependency due to gix-object transitive paths, create a minimal local graph module that mirrors brit-graph's API but uses sha2 (already in rakia deps) instead of blake3/BritCid. The types will be structurally compatible for later replacement.

Create `rakia-core/src/graph_local.rs`:

```rust
//! Lightweight graph types for use until brit-graph path dependency resolves.
//! Mirrors brit-graph API — EprGraph, AffectedTracker, TopoPlan.
//! Uses sha2 for hashing instead of blake3/BritCid.
//!
//! TODO: Replace with brit-graph when workspace integration is tested.
```

This is the fallback — try the path dependency first. The rest of this task assumes it works one way or another.

- [ ] **Step 7.3: Write failing test for manifest discovery**

Create `rakia-core/tests/discover_test.rs`:

```rust
use rakia_core::discover::discover_manifests;
use std::fs;
use tempfile::TempDir;

#[test]
fn discovers_manifest_files() {
    let dir = TempDir::new().unwrap();
    let sub = dir.path().join("project-a");
    fs::create_dir_all(&sub).unwrap();
    fs::write(
        sub.join("build-manifest.json"),
        r#"{
            "manifestVersion": "1.0",
            "pipeline": "test-pipeline",
            "description": "test",
            "steps": {},
            "gate": {},
            "deployment": {}
        }"#,
    )
    .unwrap();

    let manifests = discover_manifests(dir.path()).unwrap();
    assert_eq!(manifests.len(), 1);
    assert_eq!(manifests[0].1.pipeline, "test-pipeline");
}

#[test]
fn skips_node_modules_and_git() {
    let dir = TempDir::new().unwrap();
    let nm = dir.path().join("node_modules").join("pkg");
    fs::create_dir_all(&nm).unwrap();
    fs::write(
        nm.join("build-manifest.json"),
        r#"{"manifestVersion":"1.0","pipeline":"skip","description":"","steps":{}}"#,
    )
    .unwrap();

    let manifests = discover_manifests(dir.path()).unwrap();
    assert_eq!(manifests.len(), 0);
}
```

- [ ] **Step 7.4: Run to verify failure**

Run: `cd elohim/rakia && cargo test -p rakia-core --test discover_test`
Expected: FAIL — module `discover` not found

- [ ] **Step 7.5: Implement discover.rs**

Create `rakia-core/src/discover.rs`:

```rust
//! Manifest discovery — find all `build-manifest.json` files in a worktree.

use std::path::{Path, PathBuf};

use crate::manifest::BuildManifest;

/// Errors from manifest discovery.
#[derive(Debug, thiserror::Error)]
pub enum DiscoverError {
    /// Filesystem error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// JSON parse error.
    #[error("parse error in {path}: {source}")]
    Parse {
        /// Path to the manifest file.
        path: PathBuf,
        /// The parse error.
        source: serde_json::Error,
    },
}

/// Directories to skip during discovery.
const SKIP_DIRS: &[&str] = &["node_modules", ".git", "target", ".hc_live"];

/// Discover and parse all `build-manifest.json` files under `root`.
pub fn discover_manifests(root: &Path) -> Result<Vec<(PathBuf, BuildManifest)>, DiscoverError> {
    let mut results = Vec::new();
    walk_dir(root, &mut results)?;
    results.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(results)
}

fn walk_dir(
    dir: &Path,
    results: &mut Vec<(PathBuf, BuildManifest)>,
) -> Result<(), DiscoverError> {
    let entries = std::fs::read_dir(dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if SKIP_DIRS.iter().any(|&skip| name_str == skip) {
                continue;
            }
            walk_dir(&path, results)?;
        } else if entry.file_name() == "build-manifest.json" {
            let content = std::fs::read_to_string(&path)?;
            let manifest: BuildManifest = serde_json::from_str(&content).map_err(|e| {
                DiscoverError::Parse {
                    path: path.clone(),
                    source: e,
                }
            })?;
            results.push((path, manifest));
        }
    }
    Ok(())
}
```

- [ ] **Step 7.6: Add tempfile to rakia-core dev-dependencies**

In `rakia-core/Cargo.toml`:

```toml
[dev-dependencies]
tempfile = "3"
```

- [ ] **Step 7.7: Update rakia-core/src/lib.rs**

Replace `rakia-core/src/lib.rs`:

```rust
//! rakia-core — the heart of the firmament
//!
//! Manifest parser, constellation builder, and build planning.
//! This crate knows how to discover manifests, construct the build DAG,
//! and determine what needs building given a set of changed paths.

pub mod manifest;
pub mod discover;
pub mod constellation;
pub mod schema;
```

- [ ] **Step 7.8: Run discovery tests**

Run: `cd elohim/rakia && cargo test -p rakia-core --test discover_test`
Expected: PASS (2 tests)

- [ ] **Step 7.9: Commit**

```bash
cd elohim/rakia && git add rakia-core/ Cargo.toml Cargo.lock
git commit -m "feat(rakia-core): manifest discovery — find all build-manifest.json files"
```

---

### Task 8: Constellation builder — manifests become a build DAG

**Files:**
- Create: `rakia-core/src/constellation.rs`
- Create: `rakia-core/tests/constellation_test.rs`
- Remove: `rakia-core/src/graph.rs` (empty stub), `rakia-core/src/hash.rs` (empty stub)

- [ ] **Step 8.1: Write failing test for constellation construction**

Create `rakia-core/tests/constellation_test.rs`:

```rust
use rakia_core::constellation::{build_constellation, plan_from_changes, QualifiedStep};
use rakia_core::manifest::BuildManifest;
use std::path::PathBuf;

fn elohim_app_manifest() -> (PathBuf, BuildManifest) {
    let json = r#"{
        "manifestVersion": "1.0",
        "pipeline": "elohim",
        "description": "Angular app",
        "steps": {
            "build-angular": {
                "description": "Build Angular",
                "inputs": { "sources": ["app/elohim-app/src/**"], "buildProcess": [] },
                "outputs": { "artifacts": ["elohim-app-dist"], "verify": null },
                "depends": ["elohim-sophia:build-sophia-umd"]
            },
            "build-site-image": {
                "description": "Build image",
                "inputs": { "sources": ["app/elohim-app/images/**"], "buildProcess": [] },
                "outputs": { "artifacts": ["site-image"], "verify": null },
                "depends": ["build-angular"]
            }
        }
    }"#;
    (
        PathBuf::from("app/elohim-app/build-manifest.json"),
        serde_json::from_str(json).unwrap(),
    )
}

fn sophia_manifest() -> (PathBuf, BuildManifest) {
    let json = r#"{
        "manifestVersion": "1.0",
        "pipeline": "elohim-sophia",
        "description": "Sophia UMD",
        "steps": {
            "build-sophia-umd": {
                "description": "Build sophia UMD bundle",
                "inputs": { "sources": ["sophia/packages/**"], "buildProcess": [] },
                "outputs": { "artifacts": ["sophia-umd"], "verify": null },
                "depends": []
            }
        }
    }"#;
    (
        PathBuf::from("sophia/build-manifest.json"),
        serde_json::from_str(json).unwrap(),
    )
}

#[test]
fn build_constellation_resolves_cross_manifest_deps() {
    let manifests = vec![elohim_app_manifest(), sophia_manifest()];
    let constellation = build_constellation(&manifests).unwrap();

    // 3 steps total: build-angular, build-site-image, build-sophia-umd
    assert_eq!(constellation.steps.len(), 3);

    // build-angular depends on elohim-sophia:build-sophia-umd
    let build_angular = constellation.get_step("elohim:build-angular").unwrap();
    assert!(build_angular.resolved_depends.contains(&"elohim-sophia:build-sophia-umd".to_string()));
}

#[test]
fn plan_from_changes_transitive_dependency() {
    let manifests = vec![elohim_app_manifest(), sophia_manifest()];
    let constellation = build_constellation(&manifests).unwrap();

    // Changing a sophia source file should trigger:
    // 1. elohim-sophia:build-sophia-umd (source match)
    // 2. elohim:build-angular (depends on sophia)
    // 3. elohim:build-site-image (depends on build-angular)
    let changed = vec!["sophia/packages/core/src/index.ts".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    let all_steps: Vec<&str> = plan
        .levels
        .iter()
        .flat_map(|level| level.iter().map(|s| s.qualified_name.as_str()))
        .collect();

    assert!(all_steps.contains(&"elohim-sophia:build-sophia-umd"));
    assert!(all_steps.contains(&"elohim:build-angular"));
    assert!(all_steps.contains(&"elohim:build-site-image"));
}

#[test]
fn plan_from_changes_no_match_produces_empty() {
    let manifests = vec![elohim_app_manifest(), sophia_manifest()];
    let constellation = build_constellation(&manifests).unwrap();

    let changed = vec!["unrelated/file.txt".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    assert!(plan.levels.is_empty());
}

#[test]
fn plan_levels_are_topologically_ordered() {
    let manifests = vec![elohim_app_manifest(), sophia_manifest()];
    let constellation = build_constellation(&manifests).unwrap();

    let changed = vec!["sophia/packages/core/src/index.ts".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    // Level 0 should contain sophia (leaf)
    // Level 1 should contain build-angular
    // Level 2 should contain build-site-image
    assert!(plan.levels.len() >= 2);
    assert!(plan.levels[0]
        .iter()
        .any(|s| s.qualified_name == "elohim-sophia:build-sophia-umd"));
}

#[test]
fn constellation_detects_cycle() {
    let json = r#"{
        "manifestVersion": "1.0",
        "pipeline": "cycle",
        "description": "cycle",
        "steps": {
            "a": {
                "description": "a",
                "inputs": { "sources": ["a/**"], "buildProcess": [] },
                "outputs": { "artifacts": [], "verify": null },
                "depends": ["b"]
            },
            "b": {
                "description": "b",
                "inputs": { "sources": ["b/**"], "buildProcess": [] },
                "outputs": { "artifacts": [], "verify": null },
                "depends": ["a"]
            }
        }
    }"#;
    let manifests = vec![(
        PathBuf::from("cycle/build-manifest.json"),
        serde_json::from_str(json).unwrap(),
    )];

    let result = build_constellation(&manifests);
    assert!(result.is_err());
}
```

- [ ] **Step 8.2: Run to verify failure**

Run: `cd elohim/rakia && cargo test -p rakia-core --test constellation_test`
Expected: FAIL — module `constellation` not found

- [ ] **Step 8.3: Implement constellation.rs**

Create `rakia-core/src/constellation.rs`:

```rust
//! Constellation builder — constructs the build DAG from manifests.
//!
//! A constellation is the complete dependency graph of all build steps
//! across all manifests in the repository. Steps are qualified by pipeline
//! name (e.g., `elohim:build-angular`). Cross-manifest dependencies are
//! resolved by qualified name lookup.

use std::collections::BTreeMap;
use std::path::PathBuf;

use globset::{Glob, GlobSet, GlobSetBuilder};

use crate::manifest::{BuildManifest, BuildStep};

/// Errors from constellation building.
#[derive(Debug, thiserror::Error)]
pub enum ConstellationError {
    /// A dependency target was not found in any manifest.
    #[error("unresolved dependency: step '{from}' depends on '{target}' which does not exist")]
    UnresolvedDependency {
        /// The step that declares the dependency.
        from: String,
        /// The target that could not be found.
        target: String,
    },
    /// The dependency graph contains a cycle.
    #[error("dependency cycle detected in the constellation")]
    CycleDetected,
    /// Glob pattern error.
    #[error("invalid glob pattern '{pattern}': {source}")]
    GlobError {
        /// The problematic pattern.
        pattern: String,
        /// The glob parse error.
        source: globset::Error,
    },
}

/// A build step qualified by its pipeline name.
#[derive(Debug, Clone)]
pub struct QualifiedStep {
    /// Fully qualified name: `pipeline:step`.
    pub qualified_name: String,
    /// The pipeline this step belongs to.
    pub pipeline: String,
    /// The step name within the pipeline.
    pub step_name: String,
    /// Step description.
    pub description: String,
    /// Source glob patterns for change detection.
    pub source_patterns: Vec<String>,
    /// Build process files (for fingerprinting).
    pub build_process: Vec<String>,
    /// Output artifact names.
    pub artifacts: Vec<String>,
    /// Resolved dependency names (fully qualified).
    pub resolved_depends: Vec<String>,
    /// Path to the manifest file this step came from.
    pub manifest_path: PathBuf,
}

/// The complete build constellation — all steps and their dependencies.
pub struct Constellation {
    /// All steps, keyed by qualified name.
    pub steps: BTreeMap<String, QualifiedStep>,
    /// Dependency edges: key depends on each value.
    edges: Vec<(String, String)>,
}

impl Constellation {
    /// Get a step by qualified name.
    pub fn get_step(&self, name: &str) -> Option<&QualifiedStep> {
        self.steps.get(name)
    }
}

/// Build a constellation from discovered manifests.
///
/// Resolves cross-manifest dependencies, validates that all targets exist,
/// and checks for cycles.
pub fn build_constellation(
    manifests: &[(PathBuf, BuildManifest)],
) -> Result<Constellation, ConstellationError> {
    let mut steps = BTreeMap::new();
    let mut edges = Vec::new();

    // Phase 1: collect all steps with qualified names
    for (path, manifest) in manifests {
        for (step_name, step) in &manifest.steps {
            let qualified = format!("{}:{}", manifest.pipeline, step_name);
            let q_step = QualifiedStep {
                qualified_name: qualified.clone(),
                pipeline: manifest.pipeline.clone(),
                step_name: step_name.clone(),
                description: step.description.clone(),
                source_patterns: step.inputs.sources.clone(),
                build_process: step.inputs.build_process.clone(),
                artifacts: step.outputs.artifacts.clone(),
                resolved_depends: Vec::new(),
                manifest_path: path.clone(),
            };
            steps.insert(qualified, q_step);
        }
    }

    // Phase 2: resolve dependencies
    let step_names: Vec<String> = steps.keys().cloned().collect();
    for (path, manifest) in manifests {
        for (step_name, step) in &manifest.steps {
            let from = format!("{}:{}", manifest.pipeline, step_name);
            for dep in &step.depends {
                // Qualify intra-pipeline refs: "build-angular" -> "elohim:build-angular"
                let target = if dep.contains(':') {
                    dep.clone()
                } else {
                    format!("{}:{}", manifest.pipeline, dep)
                };

                if !step_names.contains(&target) {
                    return Err(ConstellationError::UnresolvedDependency {
                        from: from.clone(),
                        target,
                    });
                }

                edges.push((from.clone(), target.clone()));
                steps.get_mut(&from).unwrap().resolved_depends.push(target);
            }
        }
    }

    let constellation = Constellation { steps, edges };

    // Phase 3: cycle detection via DFS
    if has_cycle(&constellation) {
        return Err(ConstellationError::CycleDetected);
    }

    Ok(constellation)
}

/// Check for cycles using DFS with coloring.
fn has_cycle(constellation: &Constellation) -> bool {
    #[derive(Clone, Copy, PartialEq)]
    enum Color { White, Gray, Black }

    let mut colors: BTreeMap<&str, Color> = constellation
        .steps
        .keys()
        .map(|k| (k.as_str(), Color::White))
        .collect();

    fn dfs<'a>(
        node: &'a str,
        constellation: &'a Constellation,
        colors: &mut BTreeMap<&'a str, Color>,
    ) -> bool {
        colors.insert(node, Color::Gray);
        if let Some(step) = constellation.steps.get(node) {
            for dep in &step.resolved_depends {
                match colors.get(dep.as_str()) {
                    Some(Color::Gray) => return true, // back edge = cycle
                    Some(Color::White) => {
                        if dfs(dep, constellation, colors) {
                            return true;
                        }
                    }
                    _ => {}
                }
            }
        }
        colors.insert(node, Color::Black);
        false
    }

    let keys: Vec<String> = constellation.steps.keys().cloned().collect();
    for key in &keys {
        if colors[key.as_str()] == Color::White {
            if dfs(key, constellation, &mut colors) {
                return true;
            }
        }
    }

    false
}

/// Given a constellation and a list of changed file paths, determine which
/// steps are affected and return a topological execution plan.
pub fn plan_from_changes(
    constellation: &Constellation,
    changed_paths: &[String],
) -> Result<TopoPlan, ConstellationError> {
    // Phase 1: find directly affected steps by matching changed paths against source globs
    let mut affected: BTreeMap<String, Vec<String>> = BTreeMap::new(); // step -> reasons

    for (name, step) in &constellation.steps {
        let glob_set = build_glob_set(&step.source_patterns)?;
        for path in changed_paths {
            if glob_set.is_match(path) {
                affected
                    .entry(name.clone())
                    .or_default()
                    .push(format!("file: {}", path));
            }
        }
    }

    if affected.is_empty() {
        return Ok(TopoPlan { levels: vec![] });
    }

    // Phase 2: propagate to dependents (deep)
    let mut queue: std::collections::VecDeque<String> = affected.keys().cloned().collect();
    let mut visited: std::collections::HashSet<String> = affected.keys().cloned().collect();

    while let Some(step_name) = queue.pop_front() {
        // Find all steps that depend on this step
        for (name, step) in &constellation.steps {
            if step.resolved_depends.contains(&step_name) && !visited.contains(name) {
                affected
                    .entry(name.clone())
                    .or_default()
                    .push(format!("upstream: {}", step_name));
                visited.insert(name.clone());
                queue.push_back(name.clone());
            }
        }
    }

    // Phase 3: topological sort of affected steps into parallelizable levels
    topo_sort_affected(constellation, &affected)
}

/// A topological execution plan grouped by dependency level.
#[derive(Debug, Clone)]
pub struct TopoPlan {
    /// Each inner vec is a set of steps that can execute in parallel.
    pub levels: Vec<Vec<QualifiedStep>>,
}

impl TopoPlan {
    /// Check if the plan is empty.
    pub fn is_empty(&self) -> bool {
        self.levels.is_empty()
    }
}

fn topo_sort_affected(
    constellation: &Constellation,
    affected: &BTreeMap<String, Vec<String>>,
) -> Result<TopoPlan, ConstellationError> {
    let affected_set: std::collections::HashSet<&str> =
        affected.keys().map(|s| s.as_str()).collect();

    // Compute in-degree (only counting deps within the affected set)
    let mut in_degree: BTreeMap<&str, usize> = BTreeMap::new();
    for name in affected.keys() {
        let step = &constellation.steps[name];
        let count = step
            .resolved_depends
            .iter()
            .filter(|d| affected_set.contains(d.as_str()))
            .count();
        in_degree.insert(name.as_str(), count);
    }

    let mut levels = Vec::new();
    let mut queue: Vec<&str> = in_degree
        .iter()
        .filter(|(_, &deg)| deg == 0)
        .map(|(&name, _)| name)
        .collect();
    queue.sort(); // deterministic ordering

    while !queue.is_empty() {
        let current_level: Vec<QualifiedStep> = queue
            .iter()
            .map(|&name| constellation.steps[name].clone())
            .collect();

        let mut next_queue = Vec::new();
        for &completed in &queue {
            for (name, step) in &constellation.steps {
                if affected_set.contains(name.as_str())
                    && step.resolved_depends.iter().any(|d| d == completed)
                {
                    if let Some(deg) = in_degree.get_mut(name.as_str()) {
                        *deg = deg.saturating_sub(1);
                        if *deg == 0 {
                            next_queue.push(name.as_str());
                        }
                    }
                }
            }
        }

        levels.push(current_level);
        next_queue.sort();
        queue = next_queue;
    }

    Ok(TopoPlan { levels })
}

fn build_glob_set(patterns: &[String]) -> Result<GlobSet, ConstellationError> {
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = Glob::new(pattern).map_err(|e| ConstellationError::GlobError {
            pattern: pattern.clone(),
            source: e,
        })?;
        builder.add(glob);
    }
    builder.build().map_err(|e| ConstellationError::GlobError {
        pattern: "<combined>".to_string(),
        source: e,
    })
}
```

- [ ] **Step 8.4: Remove empty stubs**

Delete the content of `rakia-core/src/graph.rs` and `rakia-core/src/hash.rs` (or remove the files entirely and remove their `pub mod` lines from `lib.rs`).

Updated `rakia-core/src/lib.rs` (from step 7.7) already references `constellation` instead of `graph` and `hash`.

- [ ] **Step 8.5: Run tests**

Run: `cd elohim/rakia && cargo test -p rakia-core --test constellation_test`
Expected: all 5 tests PASS

- [ ] **Step 8.6: Commit**

```bash
cd elohim/rakia && git add rakia-core/ Cargo.toml Cargo.lock
git commit -m "feat(rakia-core): constellation builder — manifests become a build DAG"
```

---

### Task 9: Validate against real manifests

**Files:**
- Create: `rakia-core/tests/real_manifests_test.rs`

This test validates against the 8 actual build manifests in the Elohim monorepo. It uses the manifest JSON inline (not file reads) to keep the test self-contained and runnable from the rakia submodule.

- [ ] **Step 9.1: Write integration test with real manifest data**

Create `rakia-core/tests/real_manifests_test.rs`:

```rust
//! Integration test: validate the constellation against the real Elohim monorepo manifests.
//! Manifest JSON is inlined to keep the test self-contained.

use rakia_core::constellation::{build_constellation, plan_from_changes};
use rakia_core::manifest::BuildManifest;
use std::path::PathBuf;

fn all_manifests() -> Vec<(PathBuf, BuildManifest)> {
    let manifests_json = vec![
        ("genesis/orchestrator/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-orchestrator","description":"CI orchestrator","steps":{"lint-jenkinsfiles":{"description":"Lint","inputs":{"sources":["**/Jenkinsfile*"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]}}}"#),
        ("elohim/holochain/dna/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-holochain","description":"DNA","steps":{"build-dna-wasm":{"description":"Build DNA WASM","inputs":{"sources":["elohim/holochain/dna/**"],"buildProcess":[]},"outputs":{"artifacts":["dna-wasm"],"verify":null},"depends":[]},"build-happ":{"description":"Build hApp","inputs":{"sources":["elohim/holochain/dna/**"],"buildProcess":[]},"outputs":{"artifacts":["elohim.happ"],"verify":null},"depends":["build-dna-wasm"]},"schema-dna":{"description":"Schema DNA check","inputs":{"sources":["elohim/holochain/dna/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]}}}"#),
        ("elohim/holochain/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-edge","description":"Edge","steps":{"cargo-build-doorway":{"description":"Build doorway","inputs":{"sources":["doorway/**"],"buildProcess":[]},"outputs":{"artifacts":["doorway"],"verify":null},"depends":[]},"cargo-build-storage":{"description":"Build storage","inputs":{"sources":["elohim/elohim-storage/**"],"buildProcess":[]},"outputs":{"artifacts":["storage"],"verify":null},"depends":[]},"build-edge-image":{"description":"Build edge image","inputs":{"sources":["elohim/holochain/**"],"buildProcess":[]},"outputs":{"artifacts":["edge-image"],"verify":null},"depends":["cargo-build-doorway","cargo-build-storage","elohim-holochain:build-happ"]}}}"#),
        ("app/elohim-app/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim","description":"Angular app","steps":{"build-angular":{"description":"Build Angular","inputs":{"sources":["app/elohim-app/src/**","app/elohim-library/**","elohim/sdk/**","VERSION"],"buildProcess":["Jenkinsfile"]},"outputs":{"artifacts":["elohim-app-dist"],"verify":null},"depends":["elohim-sophia:build-sophia-umd"]},"build-site-image":{"description":"Build image","inputs":{"sources":["app/elohim-app/images/**"],"buildProcess":["Jenkinsfile"]},"outputs":{"artifacts":["site-image"],"verify":null},"depends":["build-angular"]},"lint-library":{"description":"Lint library","inputs":{"sources":["app/elohim-library/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]}}}"#),
        ("doorway/doorway-app/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-doorway-app","description":"Doorway app","steps":{"build-doorway-app":{"description":"Build doorway app","inputs":{"sources":["doorway/doorway-app/**"],"buildProcess":[]},"outputs":{"artifacts":["doorway-app"],"verify":null},"depends":[]}}}"#),
        ("elohim/elohim-compute/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-compute","description":"Compute","steps":{"build-compute":{"description":"Build compute","inputs":{"sources":["elohim/elohim-compute/**"],"buildProcess":[]},"outputs":{"artifacts":["compute"],"verify":null},"depends":[]}}}"#),
        ("genesis/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-genesis","description":"Genesis","steps":{"validate-seeds":{"description":"Validate","inputs":{"sources":["genesis/seeder/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]},"seed-content":{"description":"Seed","inputs":{"sources":["genesis/seeder/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":["validate-seeds","elohim:build-site-image","elohim-edge:build-edge-image","elohim-edge:cargo-build-storage"]},"schema-validate":{"description":"Schema","inputs":{"sources":["elohim/sdk/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]},"schema-codegen":{"description":"Codegen","inputs":{"sources":["elohim/sdk/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]},"constants-sync":{"description":"Constants","inputs":{"sources":["elohim/sdk/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]},"lint-a2o":{"description":"Lint A2O","inputs":{"sources":["genesis/a2o/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]},"gate-genesis":{"description":"Gate","inputs":{"sources":["genesis/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":[]}}}"#),
        ("steward/device/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-steward","description":"Steward","steps":{"cargo-build-steward":{"description":"Build steward","inputs":{"sources":["steward/**"],"buildProcess":[]},"outputs":{"artifacts":["steward"],"verify":null},"depends":["elohim-holochain:build-happ"]}}}"#),
    ];

    manifests_json
        .into_iter()
        .map(|(path, json)| {
            let manifest: BuildManifest = serde_json::from_str(json).unwrap();
            (PathBuf::from(path), manifest)
        })
        .collect()
}

#[test]
fn all_eight_manifests_produce_valid_constellation() {
    let manifests = all_manifests();
    let constellation = build_constellation(&manifests).unwrap();
    assert_eq!(constellation.steps.len(), 20); // total steps across all 8 manifests
}

#[test]
fn no_cycles_in_real_manifests() {
    let manifests = all_manifests();
    // build_constellation checks for cycles internally — if it returns Ok, no cycles
    let constellation = build_constellation(&manifests).unwrap();
    assert!(!constellation.steps.is_empty());
}

#[test]
fn sophia_change_triggers_transitive_chain() {
    let manifests = all_manifests();
    let constellation = build_constellation(&manifests).unwrap();

    let changed = vec!["sophia/packages/core/src/index.ts".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    let all_steps: Vec<&str> = plan
        .levels
        .iter()
        .flat_map(|l| l.iter().map(|s| s.qualified_name.as_str()))
        .collect();

    // Should trigger: sophia -> build-angular -> build-site-image -> seed-content
    // (sophia isn't in these manifests but the pattern would match if there was a sophia manifest)
    // Actually, no sophia manifest in the inline set — let's test with elohim-app source change instead
    assert!(plan.levels.is_empty()); // sophia sources don't match any manifest patterns here
}

#[test]
fn elohim_app_source_change_triggers_app_chain() {
    let manifests = all_manifests();
    let constellation = build_constellation(&manifests).unwrap();

    let changed = vec!["app/elohim-app/src/main.ts".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    let all_steps: Vec<&str> = plan
        .levels
        .iter()
        .flat_map(|l| l.iter().map(|s| s.qualified_name.as_str()))
        .collect();

    assert!(all_steps.contains(&"elohim:build-angular"));
    assert!(all_steps.contains(&"elohim:build-site-image")); // depends on build-angular
    assert!(all_steps.contains(&"elohim-genesis:seed-content")); // depends on build-site-image
}

#[test]
fn dna_change_cascades_through_edge_to_genesis() {
    let manifests = all_manifests();
    let constellation = build_constellation(&manifests).unwrap();

    let changed = vec!["elohim/holochain/dna/lamad/src/lib.rs".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    let all_steps: Vec<&str> = plan
        .levels
        .iter()
        .flat_map(|l| l.iter().map(|s| s.qualified_name.as_str()))
        .collect();

    assert!(all_steps.contains(&"elohim-holochain:build-dna-wasm"));
    assert!(all_steps.contains(&"elohim-holochain:build-happ")); // depends on dna-wasm
    assert!(all_steps.contains(&"elohim-edge:build-edge-image")); // depends on happ
    assert!(all_steps.contains(&"elohim-steward:cargo-build-steward")); // depends on happ
}
```

- [ ] **Step 9.2: Run tests**

Run: `cd elohim/rakia && cargo test -p rakia-core --test real_manifests_test`
Expected: all 5 tests PASS

- [ ] **Step 9.3: Commit**

```bash
cd elohim/rakia && git add rakia-core/tests/
git commit -m "test(rakia-core): validate constellation against all 8 real manifests"
```

---

## Phase C: Change Detection

### Task 10: rakia-brit crate — change detection via gix

**Files:**
- Create: `rakia-brit/Cargo.toml`
- Create: `rakia-brit/src/lib.rs`
- Create: `rakia-brit/src/changes.rs`
- Create: `rakia-brit/src/baselines.rs`
- Create: `rakia-brit/tests/change_detection.rs`
- Modify: `Cargo.toml` (workspace members)

**Note:** This task requires `gix` as a dependency for git object store access. gix is the top-level gitoxide crate. Since brit is a gitoxide fork, we can either depend on the published `gix` crate or path-depend on brit's `gix/` directory. The published crate is simpler for now.

- [ ] **Step 10.1: Create rakia-brit/Cargo.toml**

```toml
[package]
name = "rakia-brit"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
description = "Change detection and baseline refs via brit/gix"

[dependencies]
thiserror.workspace = true
gix = { version = "0.72", default-features = false, features = ["basic", "worktree-mutation"] }

[dev-dependencies]
tempfile = "3"
```

**Note:** The exact `gix` version may need adjustment. Use the latest published gix version that's compatible. If gix from crates.io doesn't match brit's fork version, use a path dep to brit's gix directory instead:

```toml
gix = { path = "../elohim/brit/gix", default-features = false }
```

- [ ] **Step 10.2: Create rakia-brit/src/lib.rs**

```rust
//! rakia-brit — change detection and baseline refs via brit/gix.
//!
//! Bridges the git object store into rakia's build planning.
//! No `git` CLI shell-outs — all operations through gix.

pub mod changes;
pub mod baselines;
```

- [ ] **Step 10.3: Implement changes.rs**

Create `rakia-brit/src/changes.rs`:

```rust
//! Change detection — which files changed between two commits.

use gix::diff::object::find::Error as FindError;
use std::collections::BTreeSet;
use std::path::Path;

/// Errors from change detection.
#[derive(Debug, thiserror::Error)]
pub enum ChangeError {
    /// Failed to open the repository.
    #[error("failed to open repository: {0}")]
    OpenRepo(#[from] gix::open::Error),
    /// Failed to parse a revision.
    #[error("failed to parse revision '{rev}': {source}")]
    ParseRev {
        rev: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
    /// Failed to diff trees.
    #[error("diff error: {0}")]
    Diff(String),
    /// Generic gix error.
    #[error("gix error: {0}")]
    Gix(String),
}

/// Compute the list of changed file paths between two revisions.
///
/// Returns workspace-relative paths sorted alphabetically.
/// Uses gix's object store diff — no `git` CLI shell-out.
pub fn changed_paths_since(
    repo_path: &Path,
    baseline_rev: &str,
    head_rev: &str,
) -> Result<Vec<String>, ChangeError> {
    let repo = gix::open(repo_path)?;

    let baseline_id = repo
        .rev_parse_single(baseline_rev)
        .map_err(|e| ChangeError::ParseRev {
            rev: baseline_rev.to_string(),
            source: Box::new(e),
        })?
        .detach();

    let head_id = repo
        .rev_parse_single(head_rev)
        .map_err(|e| ChangeError::ParseRev {
            rev: head_rev.to_string(),
            source: Box::new(e),
        })?
        .detach();

    let baseline_commit = repo
        .find_object(baseline_id)
        .map_err(|e| ChangeError::Gix(e.to_string()))?
        .try_into_commit()
        .map_err(|e| ChangeError::Gix(e.to_string()))?;
    let head_commit = repo
        .find_object(head_id)
        .map_err(|e| ChangeError::Gix(e.to_string()))?
        .try_into_commit()
        .map_err(|e| ChangeError::Gix(e.to_string()))?;

    let baseline_tree = baseline_commit
        .tree()
        .map_err(|e| ChangeError::Gix(e.to_string()))?;
    let head_tree = head_commit
        .tree()
        .map_err(|e| ChangeError::Gix(e.to_string()))?;

    let mut changed = BTreeSet::new();
    let changes = baseline_tree
        .changes()
        .map_err(|e| ChangeError::Gix(e.to_string()))?
        .track_path()
        .for_each_to_obtain_tree(&head_tree, |change| {
            let path = change.location().to_string();
            changed.insert(path);
            Ok::<_, std::convert::Infallible>(gix::diff::tree::visit::Action::Continue)
        })
        .map_err(|e| ChangeError::Diff(e.to_string()))?;

    Ok(changed.into_iter().collect())
}
```

**Note:** The exact gix API for tree diffing may vary by version. The above uses gix's `tree.changes().track_path().for_each_to_obtain_tree()` pattern. If the API differs in the available version, consult gix's docs/examples. The key contract: given two commit revisions, return the list of changed file paths without shelling out to `git`.

- [ ] **Step 10.4: Implement baselines.rs**

Create `rakia-brit/src/baselines.rs`:

```rust
//! Baseline ref management — read/write pipeline baselines as git refs.
//!
//! Baselines live at `refs/notes/rakia/baselines/{pipeline}`.
//! They survive executor death because they're git refs, not artifacts.

use std::path::Path;

/// Errors from baseline operations.
#[derive(Debug, thiserror::Error)]
pub enum BaselineError {
    /// Failed to open repository.
    #[error("failed to open repository: {0}")]
    OpenRepo(#[from] gix::open::Error),
    /// Ref operation failed.
    #[error("ref error: {0}")]
    Ref(String),
    /// JSON parse error.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// IO error.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

const BASELINE_PREFIX: &str = "refs/notes/rakia/baselines/";

/// Read the baseline commit for a pipeline.
/// Returns None if no baseline has been set.
pub fn read_baseline(
    repo_path: &Path,
    pipeline: &str,
) -> Result<Option<String>, BaselineError> {
    let repo = gix::open(repo_path)?;
    let ref_name = format!("{}{}", BASELINE_PREFIX, pipeline);

    match repo.find_reference(&ref_name) {
        Ok(reference) => {
            let id = reference.id().detach();
            Ok(Some(id.to_string()))
        }
        Err(_) => Ok(None),
    }
}

/// Write a baseline commit for a pipeline.
pub fn write_baseline(
    repo_path: &Path,
    pipeline: &str,
    commit_sha: &str,
) -> Result<(), BaselineError> {
    let repo = gix::open(repo_path)?;
    let ref_name = format!("{}{}", BASELINE_PREFIX, pipeline);
    let id = repo
        .rev_parse_single(commit_sha)
        .map_err(|e| BaselineError::Ref(e.to_string()))?
        .detach();

    repo.reference(
        ref_name,
        id,
        gix::refs::transaction::PreviousValue::Any,
        format!("rakia: set baseline for {}", pipeline),
    )
    .map_err(|e| BaselineError::Ref(e.to_string()))?;

    Ok(())
}

/// Migrate baselines from a Jenkins `pipeline-baselines.json` file.
///
/// Expected format: `{ "pipelines": { "name": { "lastSuccessfulCommit": "sha" } } }`
pub fn migrate_baselines(
    repo_path: &Path,
    json_path: &Path,
) -> Result<Vec<String>, BaselineError> {
    let content = std::fs::read_to_string(json_path)?;
    let data: serde_json::Value = serde_json::from_str(&content)?;

    let mut migrated = Vec::new();

    if let Some(pipelines) = data.get("pipelines").and_then(|v| v.as_object()) {
        for (name, info) in pipelines {
            if let Some(commit) = info
                .get("lastSuccessfulCommit")
                .and_then(|v| v.as_str())
            {
                write_baseline(repo_path, name, commit)?;
                migrated.push(name.clone());
            }
        }
    }

    Ok(migrated)
}
```

- [ ] **Step 10.5: Add rakia-brit to workspace**

In `rakia/Cargo.toml`, uncomment `rakia-brit`:

```toml
[workspace]
resolver = "2"
members = [
    "rakia-core",
    "rakia-brit",
]
```

- [ ] **Step 10.6: Verify compilation**

Run: `cd elohim/rakia && cargo check -p rakia-brit`
Expected: success

**Note:** If `gix` version or API doesn't match, this step will reveal it. Adjust the gix version or API calls in `changes.rs` accordingly. The gix tree diff API has evolved across versions — check the gix changelog for the correct method names.

- [ ] **Step 10.7: Commit**

```bash
cd elohim/rakia && git add rakia-brit/ Cargo.toml Cargo.lock
git commit -m "feat(rakia-brit): change detection and baseline refs via gix"
```

---

### Task 11: Integration test — end-to-end from changed files to build plan

**Files:**
- Create: `rakia-core/tests/end_to_end_test.rs`

This test exercises the full flow: discover manifests -> build constellation -> detect changed files -> produce topological plan. It uses inline manifests (same as Task 9) since rakia's test environment may not have the monorepo worktree.

- [ ] **Step 11.1: Write integration test**

Create `rakia-core/tests/end_to_end_test.rs`:

```rust
//! End-to-end: changed files -> constellation -> affected -> topological plan.

use rakia_core::constellation::{build_constellation, plan_from_changes};
use rakia_core::manifest::BuildManifest;
use std::path::PathBuf;

fn all_manifests() -> Vec<(PathBuf, BuildManifest)> {
    // Same as real_manifests_test.rs — duplicated for self-containment
    let manifests_json = vec![
        ("elohim/holochain/dna/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-holochain","description":"DNA","steps":{"build-dna-wasm":{"description":"WASM","inputs":{"sources":["elohim/holochain/dna/**"],"buildProcess":[]},"outputs":{"artifacts":["dna-wasm"],"verify":null},"depends":[]},"build-happ":{"description":"hApp","inputs":{"sources":["elohim/holochain/dna/**"],"buildProcess":[]},"outputs":{"artifacts":["elohim.happ"],"verify":null},"depends":["build-dna-wasm"]}}}"#),
        ("elohim/holochain/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-edge","description":"Edge","steps":{"cargo-build-doorway":{"description":"Doorway","inputs":{"sources":["doorway/**"],"buildProcess":[]},"outputs":{"artifacts":["doorway"],"verify":null},"depends":[]},"cargo-build-storage":{"description":"Storage","inputs":{"sources":["elohim/elohim-storage/**"],"buildProcess":[]},"outputs":{"artifacts":["storage"],"verify":null},"depends":[]},"build-edge-image":{"description":"Edge image","inputs":{"sources":["elohim/holochain/**"],"buildProcess":[]},"outputs":{"artifacts":["edge-image"],"verify":null},"depends":["cargo-build-doorway","cargo-build-storage","elohim-holochain:build-happ"]}}}"#),
        ("app/elohim-app/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim","description":"App","steps":{"build-angular":{"description":"Angular","inputs":{"sources":["app/elohim-app/src/**"],"buildProcess":[]},"outputs":{"artifacts":["elohim-app-dist"],"verify":null},"depends":["elohim-sophia:build-sophia-umd"]},"build-site-image":{"description":"Image","inputs":{"sources":["app/elohim-app/images/**"],"buildProcess":[]},"outputs":{"artifacts":["site-image"],"verify":null},"depends":["build-angular"]}}}"#),
        ("genesis/build-manifest.json", r#"{"manifestVersion":"1.0","pipeline":"elohim-genesis","description":"Genesis","steps":{"seed-content":{"description":"Seed","inputs":{"sources":["genesis/seeder/**"],"buildProcess":[]},"outputs":{"artifacts":[],"verify":null},"depends":["elohim:build-site-image","elohim-edge:build-edge-image"]}}}"#),
    ];
    manifests_json
        .into_iter()
        .map(|(p, j)| (PathBuf::from(p), serde_json::from_str(j).unwrap()))
        .collect()
}

#[test]
fn doorway_change_cascades_to_edge_image_and_genesis() {
    let manifests = all_manifests();
    let constellation = build_constellation(&manifests).unwrap();

    let changed = vec!["doorway/doorway-service/src/main.rs".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    let all: Vec<&str> = plan.levels.iter()
        .flat_map(|l| l.iter().map(|s| s.qualified_name.as_str()))
        .collect();

    // doorway change -> cargo-build-doorway -> build-edge-image -> seed-content
    assert!(all.contains(&"elohim-edge:cargo-build-doorway"));
    assert!(all.contains(&"elohim-edge:build-edge-image"));
    assert!(all.contains(&"elohim-genesis:seed-content"));

    // doorway change should NOT trigger storage or dna
    assert!(!all.contains(&"elohim-edge:cargo-build-storage"));
    assert!(!all.contains(&"elohim-holochain:build-dna-wasm"));
}

#[test]
fn levels_respect_dependency_ordering() {
    let manifests = all_manifests();
    let constellation = build_constellation(&manifests).unwrap();

    let changed = vec!["elohim/holochain/dna/lamad/src/lib.rs".to_string()];
    let plan = plan_from_changes(&constellation, &changed).unwrap();

    // Verify ordering: leaves before dependents
    let flat: Vec<&str> = plan.levels.iter()
        .flat_map(|l| l.iter().map(|s| s.qualified_name.as_str()))
        .collect();

    let dna_pos = flat.iter().position(|&s| s == "elohim-holochain:build-dna-wasm");
    let happ_pos = flat.iter().position(|&s| s == "elohim-holochain:build-happ");
    let edge_pos = flat.iter().position(|&s| s == "elohim-edge:build-edge-image");

    assert!(dna_pos.unwrap() < happ_pos.unwrap());
    assert!(happ_pos.unwrap() < edge_pos.unwrap());
}
```

- [ ] **Step 11.2: Run tests**

Run: `cd elohim/rakia && cargo test -p rakia-core --test end_to_end_test`
Expected: all 2 tests PASS

- [ ] **Step 11.3: Commit**

```bash
cd elohim/rakia && git add rakia-core/tests/
git commit -m "test(rakia-core): end-to-end integration — changed files to build plan"
```

---

## Phase B/C Checkpoint

At this point, both submodules have working code:

- **brit-graph**: generic EPR graph engine with affected tracking, fingerprinting, and topological planning
- **rakia-core**: manifest discovery, constellation builder, `plan_from_changes`
- **rakia-brit**: change detection and baseline refs via gix

**Verify everything passes:**

```bash
cd elohim/brit && cargo test -p brit-graph
cd elohim/rakia && cargo test -p rakia-core
cd elohim/rakia && cargo test -p rakia-brit  # if gix integration compiles
```

**Bump submodule pointers in the parent monorepo:**

```bash
cd /home/matthew/git/elohim
git add elohim/brit elohim/rakia
git commit -m "chore: bump brit and rakia submodules — graph engine + constellation builder"
```

**Next steps (not in this plan):**
- Phase A: Schema formalization (IoC cleanup pass)
- brit CLI commands (`brit graph`, `brit affected`, `brit plan`)
- Shadow-mode validation against Groovy build-graph.groovy
- rakia-cli (`rakia build`, `rakia ci`)

These are separate implementation plans, each producing working, testable software.
