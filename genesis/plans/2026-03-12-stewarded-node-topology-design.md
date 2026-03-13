# Stewarded Node Topology Design

**Date:** 2026-03-12
**Status:** Approved
**Scope:** StewardedNode entity end-to-end, two seeded nodes, dashboard availability aggregation

## Motivation

A single peer (person) operates multiple physical nodes with different resource profiles. Today the protocol models people (Human, presences) but not their infrastructure. A family member who slides a new storage blade into their cluster should see it appear in their shefa dashboard with aggregated capacity — no manual configuration beyond claiming the node.

The protocol doesn't need affinity labels or role assignments. Raw capacity fields (CPU, RAM, storage, bandwidth) are sufficient — intelligence in the operator agent layer (future slice) infers what a node is good for. This design builds the data foundation that agent automation will later consume.

## Design Principles

- **Stewardship is a relationship, not ownership.** Multiple humans steward a node with varying affinity. The family member who plugged it in has primary affinity; others in the household can steward it too.
- **No affinity labels.** The protocol stores raw specs. k8s `node-affinity` stays outside the protocol as a dev convenience.
- **Context is content.** Natural language descriptions of nodes and stewardship relationships are EPR-addressed ContentNodes, not database text fields. Same pipeline, same reach model, same agent resolution.
- **No shortcuts.** Full protocol path: DNA entry → elohim-storage projection → generated TypeScript → Angular dashboard.
- **No doorway involvement.** elohim-storage serves its own HTTP API for node operations.

## Layer 1: DNA (node-registry integrity zome)

Extend the existing `NodeRegistration` entry in `elohim/holochain/dna/node-registry/zomes/node_registry_integrity/src/lib.rs`:

### New fields on NodeRegistration

| Field | Type | Purpose |
|-------|------|---------|
| `claim_status` | `String` | `"unclaimed"`, `"claimed"`, `"released"` |
| `context_epr_id` | `Option<String>` | EPR reference to natural language context ContentNode |

Existing capacity fields unchanged: `cpu_cores`, `memory_gb`, `storage_tb`, `bandwidth_mbps`, `steward_tier`, `custodian_opt_in`, `region`, etc.

### Coordinator zome updates

- `register_node()` — accepts new fields, creates entry + index links as before
- `claim_node(node_id, agent_pub_key)` — transitions claim_status from unclaimed → claimed
- `release_node(node_id)` — transitions claim_status to released (hard reset)

## Layer 2: elohim-storage (SQLite projection + HTTP)

### Migration: `stewarded_nodes` table

```sql
CREATE TABLE stewarded_nodes (
    id TEXT PRIMARY KEY NOT NULL,
    display_name TEXT NOT NULL,
    claim_status TEXT NOT NULL DEFAULT 'unclaimed',
    cpu_cores INTEGER NOT NULL DEFAULT 0,
    memory_gb INTEGER NOT NULL DEFAULT 0,
    storage_tb REAL NOT NULL DEFAULT 0.0,
    bandwidth_mbps INTEGER NOT NULL DEFAULT 0,
    steward_tier TEXT NOT NULL DEFAULT 'caretaker',
    custodian_opt_in INTEGER NOT NULL DEFAULT 1,
    region TEXT,
    context_epr_id TEXT,
    dht_anchor_hash TEXT,
    app_id TEXT NOT NULL DEFAULT 'shefa',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_stewarded_nodes_claim_status ON stewarded_nodes(claim_status);
CREATE INDEX idx_stewarded_nodes_app_id ON stewarded_nodes(app_id);
```

### Migration: `node_stewardship` table

```sql
CREATE TABLE node_stewardship (
    node_id TEXT NOT NULL REFERENCES stewarded_nodes(id),
    human_id TEXT NOT NULL REFERENCES humans(id),
    affinity_score REAL NOT NULL DEFAULT 0.0,
    relationship TEXT NOT NULL DEFAULT 'primary',
    context_epr_id TEXT,
    granted_at TEXT NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (node_id, human_id)
);

CREATE INDEX idx_node_stewardship_human ON node_stewardship(human_id);
```

### Rust model (`db/models.rs`)

```rust
#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = stewarded_nodes)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardedNode {
    pub id: String,
    pub display_name: String,
    pub claim_status: String,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    pub bandwidth_mbps: i32,
    pub steward_tier: String,
    pub custodian_opt_in: i32,
    pub region: Option<String>,
    pub context_epr_id: Option<String>,
    pub dht_anchor_hash: Option<String>,
    pub app_id: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Queryable, Selectable, Serialize, Deserialize, TS)]
#[diesel(table_name = node_stewardship)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeStewardship {
    pub node_id: String,
    pub human_id: String,
    pub affinity_score: f64,
    pub relationship: String,
    pub context_epr_id: Option<String>,
    pub granted_at: String,
}
```

### View types (`views.rs`)

```rust
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct StewardedNodeView {
    pub id: String,
    pub display_name: String,
    pub claim_status: String,
    pub cpu_cores: i32,
    pub memory_gb: i32,
    pub storage_tb: f64,
    pub bandwidth_mbps: i32,
    pub steward_tier: String,
    pub custodian_opt_in: bool,        // i32 → bool at boundary
    pub region: Option<String>,
    pub context_epr_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub stewards: Vec<NodeStewardshipView>,  // Joined for convenience
}

#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct NodeStewardshipView {
    pub human_id: String,
    pub display_name: String,          // Joined from humans table
    pub affinity_score: f64,
    pub relationship: String,
    pub context_epr_id: Option<String>,
}
```

### HTTP routes (elohim-storage)

| Method | Path | Purpose |
|--------|------|---------|
| `POST` | `/nodes/register` | Register a new stewarded node |
| `GET` | `/nodes` | List nodes (filterable by claim_status) |
| `GET` | `/nodes/:id` | Get node with stewardship relationships |

## Layer 3: Generated TypeScript

Auto-generated via `cargo test export_bindings` into `elohim/sdk/storage-client-ts/src/generated/`. Produces `StewardedNodeView` and `NodeStewardshipView` types — camelCase, typed, ready for Angular.

## Layer 4: Angular (shefa dashboard)

### Model changes

Replace `OwnedNode` with `StewardedNode` in `shefa-dashboard.model.ts`. Update `NodeTopologyState` to use the new type.

### Availability calculation

Three numbers per resource type (CPU cores, memory GB, storage TB, bandwidth Mbps):

| Metric | Calculation |
|--------|-------------|
| **Total** | Sum of raw specs across all claimed stewarded nodes |
| **Committed** | Sum of pledges (constitutional limits — future slice, zero for now) |
| **Available** | Total minus committed (equals total in this slice) |

Displayed as bars or gauges in the existing compute metrics panel.

### Per-node detail

Each node shows raw specs, claim status, stewards with affinity scores, and resolved context (EPR content fetched and displayed as natural language description).

## Layer 5: Genesis seeder

### Seed data: `genesis/data/shefa/nodes.json`

```json
{
  "nodes": [
    {
      "id": "node-storage-01",
      "displayName": "Storage Node",
      "cpuCores": 2,
      "memoryGb": 4,
      "storageTb": 4.0,
      "bandwidthMbps": 1000,
      "stewardTier": "guardian",
      "region": "home-lab",
      "context": "NAS in the garage. 4TB spinning disk, runs Harbor container registry and family photo archive. Runs hot in summer — monitor thermals."
    },
    {
      "id": "node-operations-01",
      "displayName": "Operations Node",
      "cpuCores": 4,
      "memoryGb": 8,
      "storageTb": 0.25,
      "bandwidthMbps": 1000,
      "stewardTier": "guardian",
      "region": "home-lab",
      "context": "Edge node running Jenkins agents and CI/CD pipelines. Handles batch workloads. Modest storage — build artifacts should be pushed to storage node."
    }
  ],
  "stewardship": [
    { "nodeId": "node-storage-01", "humanId": "human-matthew-manager", "affinityScore": 1.0, "relationship": "primary", "context": "Set up and maintains the hardware. Knows the RAID configuration and network topology." },
    { "nodeId": "node-storage-01", "humanId": "human-jessica-spouse", "affinityScore": 0.6, "relationship": "household", "context": "Uses for photo backups. Knows how to restart if the NAS light goes red." },
    { "nodeId": "node-operations-01", "humanId": "human-matthew-manager", "affinityScore": 1.0, "relationship": "primary", "context": "Primary operator. Manages Jenkins configuration and pipeline definitions." },
    { "nodeId": "node-operations-01", "humanId": "human-jessica-spouse", "affinityScore": 0.4, "relationship": "household" }
  ]
}
```

### Seeder script: `genesis/seeder/src/seed-nodes.ts`

1. Read `genesis/data/shefa/nodes.json`
2. For each node's `context` field: create a ContentNode via existing content seeding API, get back the EPR ID
3. For each stewardship `context`: same — create ContentNode, get EPR ID
4. Register nodes via `POST /nodes/register` with `contextEprId` references
5. Create stewardship relationships with `contextEprId` references

**Execution order:** Runs after `seed-humans` and `seed-content` (needs human IDs and content pipeline available).

## What's NOT in this slice

- Operator agent automation (future: observe specs → propose role → act within constitutional limits)
- mDNS discovery / claim flow (future: unclaimed nodes visible on LAN, claim from dashboard or join key)
- Constitutional commitment configuration (future: pledge capacity to self/community/network tiers)
- Hot/cold data placement (future: agent distributes data based on access patterns and node specs)
- Network sharing (future: surplus capacity shared with community/network per tiered commitments)
- Disk speed (IOPS/throughput), GPU, NPU capability fields (future: richer hardware introspection)

## Future: Plug-and-Play Story

The full arc this enables (not in scope, but this design is the foundation):

1. New blade appears on LAN → mDNS announces it as unclaimed
2. Shefa dashboard shows "New node detected" with raw specs
3. Operator agent proposes: "4TB disk, 2 cores — looks like a storage node. Recommend claiming for blob archive and replicating Harbor data."
4. Human approves from dashboard (or agent auto-claims within constitutional bounds)
5. Agent rebalances: moves cold blobs to new node, updates shard assignments
6. Dashboard updates: total storage increases, availability recalculates, community sharing capacity grows
