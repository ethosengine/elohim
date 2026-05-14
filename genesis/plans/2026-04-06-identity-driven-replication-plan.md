# Identity-Driven Replication Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Every empty elohim-storage node discovers its identity, queries peers for available content, and pulls commons content via EPR → shard protocols. Genesis bootstrap, device onboarding, and account recovery use the same code path.

**Architecture:** Extend the shard protocol with `GetContent` (full record fetch) and `ListContent` (inventory query). Add a replication loop to the P2P event loop that discovers, filters, fetches, and stores content from peers. Adam is the genesis peer who gets a direct SQLite write; the other four peers pull via P2P. Jenkins pipeline changes to seed only Adam and wait for replication.

**Tech Stack:** Rust (libp2p, Diesel, tokio), TypeScript (seed-sqlite.ts), Groovy (Jenkinsfile), K8s YAML

**Design spec:** `genesis/plans/2026-04-06-identity-driven-replication-design.md`

---

### Task 1: Add `GetContent` and `ListContent` to Shard Protocol

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/shard_protocol.rs:23-46`

The shard protocol currently has `Get` (blob by hash), `Have`, `Push`. We need two new variants for content-level replication: one to list available content (inventory) and one to fetch a full content record by ID.

- [ ] **Step 1: Add `ListContent` and `GetContent` variants to `ShardRequest`**

In `shard_protocol.rs`, add two variants to the `ShardRequest` enum:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardRequest {
    /// Get a shard by hash
    Get { hash: String },
    /// Check if peer has a shard
    Have { hash: String },
    /// Push a shard to peer (replication)
    Push { hash: String, data: Vec<u8> },
    /// List content inventory (EPR Head summaries for replication discovery)
    ListContent {
        /// Filter by reach level (e.g., "commons"). None = all reachable content.
        reach_filter: Option<String>,
        /// Pagination offset
        offset: u32,
        /// Pagination limit (max items per response)
        limit: u32,
    },
    /// Get a full content record by ID (metadata + body, not just blob bytes)
    GetContent { id: String },
}
```

- [ ] **Step 2: Add `ContentRecord` and `ContentInventoryItem` to `ShardResponse`**

Add response variants and supporting structs:

```rust
/// Lightweight content summary for inventory listing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentInventoryItem {
    pub id: String,
    pub title: String,
    pub content_type: String,
    pub content_format: String,
    pub reach: String,
    pub blob_cid: Option<String>,
    pub updated_at: String,
}

/// Full content record for replication (everything needed to recreate locally)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentRecord {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub content_type: String,
    pub content_format: String,
    pub blob_hash: Option<String>,
    pub blob_cid: Option<String>,
    pub content_size_bytes: Option<i32>,
    pub metadata_json: Option<String>,
    pub reach: String,
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    pub content_body: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ShardResponse {
    /// Shard data
    Data(Vec<u8>),
    /// Whether peer has the shard
    Have(bool),
    /// Push acknowledgment
    PushAck,
    /// Shard not found
    NotFound,
    /// Error
    Error(String),
    /// Content inventory listing
    ContentList {
        items: Vec<ContentInventoryItem>,
        total: u64,
        has_more: bool,
    },
    /// Full content record
    Content(ContentRecord),
    /// Content not found
    ContentNotFound,
}
```

- [ ] **Step 3: Verify codec compatibility**

The existing `ShardCodec` uses msgpack serialization via `rmp_serde`. Since `ShardRequest`/`ShardResponse` derive `Serialize`/`Deserialize`, new enum variants are automatically handled by the codec. No codec changes needed.

Run:

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --lib 2>&1 | grep -E "^error" | head -20
```

Expected: No errors (only ts-rs warnings).

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/p2p/shard_protocol.rs
git commit -m "feat(p2p): add ListContent and GetContent to shard protocol

Content-level replication primitives. ListContent returns inventory
summaries for discovery. GetContent returns full records for ingestion.
Same msgpack wire format as existing shard requests."
```

---

### Task 2: Handle `ListContent` and `GetContent` in Shard Request Handler

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:1417-1456` (`handle_shard_request`)

- [ ] **Step 1: Add `ListContent` handler**

In `handle_shard_request`, add a match arm after the `Push` arm:

```rust
ShardRequest::ListContent { reach_filter, offset, limit } => {
    let pool = match self.db_pool.as_ref() {
        Some(p) => p,
        None => return ShardResponse::Error("No database pool".to_string()),
    };
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
    };
    let app_ctx = crate::db::AppContext::default_lamad();
    let query = crate::db::content_diesel::ContentQuery {
        reach: reach_filter,
        limit: limit as i64,
        offset: Some(offset as i64),
        ..Default::default()
    };
    match crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query) {
        Ok(items) => {
            let total = crate::db::content_diesel::count_content(&mut conn, &app_ctx, &query)
                .unwrap_or(items.len() as i64) as u64;
            let inventory: Vec<shard_protocol::ContentInventoryItem> = items.iter().map(|cwt| {
                shard_protocol::ContentInventoryItem {
                    id: cwt.content.id.clone(),
                    title: cwt.content.title.clone(),
                    content_type: cwt.content.content_type.clone(),
                    content_format: cwt.content.content_format.clone(),
                    reach: cwt.content.reach.clone(),
                    blob_cid: cwt.content.blob_cid.clone(),
                    updated_at: cwt.content.updated_at.clone(),
                }
            }).collect();
            let has_more = (offset as u64 + inventory.len() as u64) < total;
            info!(count = inventory.len(), total = total, "Serving content inventory");
            ShardResponse::ContentList { items: inventory, total, has_more }
        }
        Err(e) => ShardResponse::Error(format!("Content query failed: {}", e)),
    }
}
```

- [ ] **Step 2: Add `GetContent` handler**

Add the match arm:

```rust
ShardRequest::GetContent { id } => {
    let pool = match self.db_pool.as_ref() {
        Some(p) => p,
        None => return ShardResponse::Error("No database pool".to_string()),
    };
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => return ShardResponse::Error(format!("DB connection failed: {}", e)),
    };
    let app_ctx = crate::db::AppContext::default_lamad();
    match crate::db::content_diesel::get_content_with_tags(&mut conn, &app_ctx, &id) {
        Ok(Some(cwt)) => {
            debug!(id = %id, "Serving content record to peer");
            ShardResponse::Content(shard_protocol::ContentRecord {
                id: cwt.content.id,
                title: cwt.content.title,
                description: cwt.content.description,
                content_type: cwt.content.content_type,
                content_format: cwt.content.content_format,
                blob_hash: cwt.content.blob_hash,
                blob_cid: cwt.content.blob_cid,
                content_size_bytes: cwt.content.content_size_bytes,
                metadata_json: cwt.content.metadata_json,
                reach: cwt.content.reach,
                created_by: cwt.content.created_by,
                tags: cwt.tags,
                content_body: cwt.content.content_body,
            })
        }
        Ok(None) => ShardResponse::ContentNotFound,
        Err(e) => ShardResponse::Error(format!("Content fetch failed: {}", e)),
    }
}
```

- [ ] **Step 3: Verify ContentQuery has `reach` and `offset` fields**

Check that `ContentQuery` supports `reach` filter and `offset` pagination. If not, add them.

Run:

```bash
cd elohim/elohim-storage && grep -n "pub struct ContentQuery" src/db/content_diesel.rs
```

Read the struct definition and verify it has `reach: Option<String>` and `offset: Option<i64>`. Also verify a `count_content` function exists. If either is missing, add them in this step.

- [ ] **Step 4: Compile check**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --lib 2>&1 | grep -E "^error" | head -20
```

Expected: No errors.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/db/content_diesel.rs
git commit -m "feat(p2p): handle ListContent and GetContent shard requests

Peers can now query each other's content inventory and fetch full
content records for replication. ListContent supports reach filtering
and pagination. GetContent returns the complete record with tags."
```

---

### Task 3: Add ReplicationState and Status Reporting

**Files:**
- Create: `elohim/elohim-storage/src/p2p/replication.rs`
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:211-226` (`P2PStatusInfo`)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (module declaration)

- [ ] **Step 1: Create the replication module**

Create `elohim/elohim-storage/src/p2p/replication.rs`:

```rust
//! Identity-driven replication state tracking.
//!
//! Tracks what content this node should have vs what it has, and manages
//! the fetch queue for pulling missing content from peers.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Serialize;

/// Replication progress exposed via /p2p/status
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplicationStatus {
    /// Content IDs discovered but not yet fetched
    pub pending: usize,
    /// Content IDs successfully replicated
    pub completed: usize,
    /// Content IDs that failed fetch (will retry)
    pub failed: usize,
    /// True when all discovered content has been fetched or failed with max retries
    pub caught_up: bool,
}

/// Internal replication state (not serialized directly)
#[derive(Debug, Default)]
struct ReplicationInner {
    /// Content IDs discovered from peers, not yet in local DB
    pending: HashSet<String>,
    /// Content IDs successfully written to local DB
    completed: HashSet<String>,
    /// Content IDs that failed with retry count
    failed: HashMap<String, u32>,
    /// Set after first successful discovery + fetch cycle with no remaining gaps
    caught_up: bool,
    /// Content IDs already known to be in local DB (skip during discovery)
    local_ids: HashSet<String>,
}

const MAX_RETRIES: u32 = 3;

/// Thread-safe replication state manager
#[derive(Debug, Clone)]
pub struct ReplicationState {
    inner: Arc<RwLock<ReplicationInner>>,
}

impl ReplicationState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(ReplicationInner::default())),
        }
    }

    /// Snapshot current status for API reporting
    pub async fn status(&self) -> ReplicationStatus {
        let inner = self.inner.read().await;
        ReplicationStatus {
            pending: inner.pending.len(),
            completed: inner.completed.len(),
            failed: inner.failed.len(),
            caught_up: inner.caught_up,
        }
    }

    /// Register content IDs already in local DB (call on startup)
    pub async fn set_local_ids(&self, ids: HashSet<String>) {
        let mut inner = self.inner.write().await;
        inner.local_ids = ids;
    }

    /// Discover content from a peer inventory. Returns IDs that are new gaps.
    pub async fn discover(&self, remote_ids: Vec<String>) -> Vec<String> {
        let mut inner = self.inner.write().await;
        let mut new_gaps = Vec::new();
        for id in remote_ids {
            if inner.local_ids.contains(&id)
                || inner.completed.contains(&id)
                || inner.pending.contains(&id)
            {
                continue;
            }
            // Skip if already failed max times
            if inner.failed.get(&id).copied().unwrap_or(0) >= MAX_RETRIES {
                continue;
            }
            inner.pending.insert(id.clone());
            new_gaps.push(id);
        }
        new_gaps
    }

    /// Mark a content ID as successfully replicated
    pub async fn mark_completed(&self, id: &str) {
        let mut inner = self.inner.write().await;
        inner.pending.remove(id);
        inner.failed.remove(id);
        inner.completed.insert(id.to_string());
        inner.local_ids.insert(id.to_string());
    }

    /// Mark a content ID as failed (will retry up to MAX_RETRIES)
    pub async fn mark_failed(&self, id: &str) {
        let mut inner = self.inner.write().await;
        inner.pending.remove(id);
        let count = inner.failed.entry(id.to_string()).or_insert(0);
        *count += 1;
        // Re-queue if under retry limit
        if *count < MAX_RETRIES {
            inner.pending.insert(id.to_string());
        }
    }

    /// Check if all discovered content is fetched or exhausted retries
    pub async fn update_caught_up(&self) {
        let mut inner = self.inner.write().await;
        inner.caught_up = inner.pending.is_empty();
    }
}
```

- [ ] **Step 2: Add module declaration**

In `elohim/elohim-storage/src/p2p/mod.rs`, add near the top with other module declarations:

```rust
pub mod replication;
```

- [ ] **Step 3: Add `replication` field to `P2PStatusInfo`**

In `P2PStatusInfo` (line ~211), add the field:

```rust
#[derive(Debug, Clone, Serialize)]
pub struct P2PStatusInfo {
    pub peer_id: String,
    pub listen_addresses: Vec<String>,
    pub connected_peers: usize,
    pub bootstrap_nodes: Vec<String>,
    pub sync_documents: usize,
    pub nat_status: String,
    pub relay_reservations: usize,
    pub announce_addresses: Vec<String>,
    pub relay_mode: String,
    pub replication: replication::ReplicationStatus,
}
```

- [ ] **Step 4: Update `refresh_status` to include replication**

In `refresh_status()` (line ~2490), add replication status to the `P2PStatusInfo` construction. The `P2PNode` will need a `replication_state: ReplicationState` field (added in Task 4).

For now, add a placeholder `Default::default()` for the `replication` field so the code compiles:

```rust
let status = P2PStatusInfo {
    // ... existing fields ...
    replication: replication::ReplicationStatus::default(),
};
```

- [ ] **Step 5: Compile check**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --lib 2>&1 | grep -E "^error" | head -20
```

Expected: No errors.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/replication.rs elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): add ReplicationState tracking and status reporting

Thread-safe replication state with discover/fetch/complete lifecycle.
Exposes pending/completed/failed/caughtUp via /p2p/status endpoint.
Same status endpoint Jenkins polls for verification."
```

---

### Task 4: Wire Replication State into P2PNode

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (P2PNode struct, `new()`, `refresh_status()`)

- [ ] **Step 1: Add `replication_state` field to `P2PNode`**

In the `P2PNode` struct (around line 160), add:

```rust
/// Identity-driven replication state
replication_state: replication::ReplicationState,
```

- [ ] **Step 2: Initialize in `new()`**

In `P2PNode::new()` (around line 469), initialize the field:

```rust
let replication_state = replication::ReplicationState::new();
```

And add it to the struct construction:

```rust
Ok(Self {
    // ... existing fields ...
    replication_state,
})
```

- [ ] **Step 3: Populate local IDs on startup**

After the existing `publish_all_epr_heads()` call in the startup section, add local ID population. Find the `status_interval` first tick handler where `publish_all_epr_heads` is called and add after it:

```rust
// Populate replication state with local content IDs
if let Some(pool) = self.db_pool.as_ref() {
    if let Ok(mut conn) = pool.get() {
        let app_ctx = crate::db::AppContext::default_lamad();
        let query = crate::db::content_diesel::ContentQuery {
            limit: 100_000,
            ..Default::default()
        };
        if let Ok(items) = crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query) {
            let ids: std::collections::HashSet<String> = items.iter().map(|c| c.content.id.clone()).collect();
            info!(count = ids.len(), "Loaded local content IDs for replication state");
            self.replication_state.set_local_ids(ids).await;
        }
    }
}
```

- [ ] **Step 4: Wire `refresh_status` to real replication state**

Replace the `Default::default()` placeholder in `refresh_status()` with:

```rust
let replication = self.replication_state.status().await;
```

- [ ] **Step 5: Compile check**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --lib 2>&1 | grep -E "^error" | head -20
```

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): wire ReplicationState into P2PNode lifecycle

Initialized on startup with local content IDs. Replication status
exposed via refresh_status -> /p2p/status endpoint."
```

---

### Task 5: Implement the Replication Loop

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (event loop + new method)

This is the core new capability. A periodic task that: discovers content from peers via `ListContent`, compares against local state, fetches missing content via `GetContent`, writes to SQLite, and republishes EPR Heads.

- [ ] **Step 1: Add replication interval to the event loop**

In the `run()` method (line ~665), add a new interval timer:

```rust
let mut replication_interval = tokio::time::interval(Duration::from_secs(60));
replication_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
```

Add a match arm in the `tokio::select!` block:

```rust
_ = replication_interval.tick() => {
    drop(swarm);
    self.run_replication_cycle().await;
}
```

- [ ] **Step 2: Implement `run_replication_cycle`**

Add this method to the `impl P2PNode` block:

```rust
/// Run one cycle of identity-driven replication.
///
/// Discovers content from peers, filters by reach (commons this sprint),
/// fetches missing records, stores locally, and republishes EPR Heads.
async fn run_replication_cycle(&self) {
    let swarm = self.swarm.read().await;
    let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
    drop(swarm);

    if peers.is_empty() {
        debug!("Replication cycle: no connected peers, skipping");
        return;
    }

    // Phase 1: Discover — query first connected peer for commons content inventory
    let peer = peers[0];
    let request = ShardRequest::ListContent {
        reach_filter: Some("commons".to_string()),
        offset: 0,
        limit: 5000,
    };

    let mut swarm = self.swarm.write().await;
    let request_id = swarm
        .behaviour_mut()
        .shard_protocol
        .send_request(&peer, request);
    drop(swarm);

    debug!(peer = %peer, request_id = ?request_id, "Sent ListContent for replication discovery");
    // Response handled in handle_shard_response (Step 3)
}
```

- [ ] **Step 3: Handle `ContentList` response for replication**

Find the existing shard response handler (search for `ShardProtocol` in the swarm event handler). Add handling for `ShardResponse::ContentList`:

```rust
ShardResponse::ContentList { items, total, has_more } => {
    info!(
        count = items.len(), total = total, has_more = has_more,
        "Received content inventory from peer"
    );
    let remote_ids: Vec<String> = items.into_iter().map(|i| i.id).collect();
    let new_gaps = self.replication_state.discover(remote_ids).await;

    if new_gaps.is_empty() {
        debug!("No new content to replicate");
        self.replication_state.update_caught_up().await;
        return;
    }

    info!(gaps = new_gaps.len(), "Discovered content gaps, starting fetch");
    self.fetch_missing_content(peer, new_gaps).await;
}
```

Note: the shard protocol response handler currently handles `ShardResponse::Data`, `ShardResponse::Have`, etc. in the swarm event match. Locate the `request_response::Event::Message` handler for the shard protocol and add the `ContentList` match arm there.

- [ ] **Step 4: Implement `fetch_missing_content`**

```rust
/// Fetch missing content records from a peer and store locally.
async fn fetch_missing_content(&self, peer: PeerId, content_ids: Vec<String>) {
    let batch_size = 50;
    let batch_delay = Duration::from_millis(100);

    for chunk in content_ids.chunks(batch_size) {
        for id in chunk {
            let request = ShardRequest::GetContent { id: id.clone() };
            let mut swarm = self.swarm.write().await;
            let _request_id = swarm
                .behaviour_mut()
                .shard_protocol
                .send_request(&peer, request);
            drop(swarm);
        }
        // Self-pace: let SQLite breathe between batches
        tokio::time::sleep(batch_delay).await;
    }
}
```

- [ ] **Step 5: Handle `Content` response — store and republish**

Add to the shard response handler:

```rust
ShardResponse::Content(record) => {
    let content_id = record.id.clone();
    debug!(id = %content_id, "Received content record from peer");

    // Store to local SQLite
    let pool = match self.db_pool.as_ref() {
        Some(p) => p,
        None => {
            self.replication_state.mark_failed(&content_id).await;
            return;
        }
    };
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => {
            self.replication_state.mark_failed(&content_id).await;
            return;
        }
    };

    let input = crate::db::content_diesel::CreateContentInput {
        id: record.id,
        title: record.title,
        description: record.description,
        content_type: record.content_type,
        content_format: record.content_format,
        blob_hash: record.blob_hash,
        blob_cid: record.blob_cid,
        content_size_bytes: record.content_size_bytes,
        metadata_json: record.metadata_json,
        reach: record.reach,
        created_by: record.created_by,
        tags: record.tags,
        content_body: record.content_body,
    };

    let app_ctx = crate::db::AppContext::default_lamad();
    match crate::db::content_diesel::bulk_create_content(&mut conn, &app_ctx, vec![input]) {
        Ok(result) => {
            if result.inserted > 0 || result.skipped > 0 {
                self.replication_state.mark_completed(&content_id).await;

                // Republish EPR Head so other peers can discover from us too
                if let Some(head_bytes) = self.resolve_epr_head_locally(&content_id) {
                    let key = RecordKey::new(&format!("epr:{}", content_id));
                    let record = Record {
                        key,
                        value: head_bytes,
                        publisher: Some(*self.identity.peer_id()),
                        expires: None,
                    };
                    let mut swarm = self.swarm.write().await;
                    let _ = swarm.behaviour_mut().kademlia.put_record(record, libp2p::kad::Quorum::One);
                }
            } else {
                self.replication_state.mark_failed(&content_id).await;
            }
        }
        Err(e) => {
            warn!(id = %content_id, error = %e, "Failed to store replicated content");
            self.replication_state.mark_failed(&content_id).await;
        }
    }

    // Update caught-up status
    self.replication_state.update_caught_up().await;
}
ShardResponse::ContentNotFound => {
    debug!("Content not found on peer");
}
```

- [ ] **Step 6: Compile check**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --lib 2>&1 | grep -E "^error" | head -20
```

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): implement identity-driven replication loop

Periodic cycle: discover commons content from peers via ListContent,
identify gaps against local DB, fetch via GetContent, store locally,
republish EPR Heads for cascade replication. Self-pacing with 50-item
batches and 100ms inter-batch delay."
```

---

### Task 6: Create Adam K8s Manifest with Elevated Resources

**Files:**
- Create: `genesis/manifests/humans/adam-firstman.yaml`
- Create: `genesis/orchestrator/manifests/humans/adam-firstman.yaml` (mirror)
- Modify: `genesis/seeder/src/seed-sqlite.ts` (remove `--conductor-for` filtering)

**Note:** Adam already exists as a persona — `human-adam-firstman` is in `genesis/data/account-packages/adam-firstman.json`, `genesis/data/lamad/presences/adam-firstman.json`, and `genesis/docs/humans/humans.json`. He's in the "Eden Household" group in `conductor-groups.json`. What's missing is the K8s deployment manifest. We're adding the deployment, not the persona.

The current K8s topology deploys 5 humans (matthew, jessica, pete, terrance, frank). Adam will be the 6th, with elevated CPU/memory resources to handle the initial seed write before other peers replicate from him.

- [ ] **Step 1: Create Adam's K8s manifest**

Read `genesis/manifests/humans/matthew-manager.yaml` to understand the structure. Create `genesis/manifests/humans/adam-firstman.yaml` following the same pattern with these key changes:
- All `matthew` / `matthew-manager` references → `adam` / `adam-firstman`
- Container name: `elohim-adam-alpha`
- HUMAN_ID env var: `human-adam-firstman`
- Comment header: `# Adam — Genesis Peer / First-write target for content seed; replicates outward to other peers`
- **Elevated resources**: bump memory and CPU limits/requests above what matthew has. If matthew has e.g. `memory: 2Gi` request, give adam `memory: 4Gi`. If matthew has `cpu: 1000m` limit, give adam `cpu: 2000m`. Match the existing field names exactly — only the values change.
- StatefulSet PVC size: bump above matthew's (if matthew is `20Gi`, give adam `40Gi` since he holds all content initially)
- Bootstrap peers: include all five other conductors so adam connects to everyone

Also create `genesis/orchestrator/manifests/humans/adam-firstman.yaml` as the orchestrator mirror (this directory is the source for jenkins; both must stay in sync).

- [ ] **Step 2: Verify Adam is in conductor-groups.json**

```bash
grep -A 3 "human-adam-firstman" genesis/data/account-packages/conductor-groups.json
```

Should show Adam in "Eden Household" group already. No changes needed if present.

- [ ] **Step 3: Simplify `seed-sqlite.ts` — remove `--conductor-for` filtering**

The seeder no longer needs to filter by stewardship since only Adam gets the direct write. Remove the `CONDUCTOR_FOR` arg parsing and `filterBySteward()` function. The seeder writes all content to whichever `STORAGE_URL` it's pointed at.

Search for and remove:
- `const CONDUCTOR_FOR = args.find(...)` (around line 39)
- The `filterBySteward()` function
- The filtering logic in `main()` that calls `filterBySteward`

- [ ] **Step 4: Commit**

```bash
git add genesis/manifests/humans/adam-firstman.yaml genesis/orchestrator/manifests/humans/adam-firstman.yaml genesis/seeder/src/seed-sqlite.ts
git commit -m "feat(genesis): deploy Adam as elevated-resource genesis peer

Adam already exists as a persona (human-adam-firstman in account
packages and presences). This adds his K8s deployment manifest with
elevated CPU/memory/storage so he can handle the full content seed
write before other peers replicate from him.

seed-sqlite.ts no longer needs --conductor-for filtering since only
Adam receives the direct write."
```

---

### Task 7: Update Jenkins Pipeline for Pull-Based Seeding

**Files:**
- Modify: `genesis/Jenkinsfile` (seeding stage)

- [ ] **Step 1: Identify the seeding stage**

The seeding stage is around line 580 in `genesis/Jenkinsfile`. It currently loops over all humans and runs `seed-sqlite.ts` for each.

```bash
grep -n "seed-sqlite\|seedContent\|SEED_CMD\|Seed Content" genesis/Jenkinsfile | head -10
```

- [ ] **Step 2: Rewrite seeding to target only Adam**

Replace the per-human seeding loop with:

1. Seed Adam's conductor only (direct SQLite write)
2. Register other humans on their conductors (identity only)
3. Wait for Adam's EPR Heads to publish
4. Wait for P2P replication to complete on all peers

The exact Groovy depends on the current stage structure (which uses helper methods per CLAUDE.md). Add a `waitForReplication` helper that polls `/p2p/status` for `replication.caughtUp == true`:

```groovy
def waitForReplication(String storageUrl, int timeoutSeconds = 300) {
    def deadline = System.currentTimeMillis() + (timeoutSeconds * 1000)
    while (System.currentTimeMillis() < deadline) {
        try {
            def response = sh(script: "curl -sf ${storageUrl}/p2p/status", returnStdout: true)
            def status = readJSON(text: response)
            if (status.replication?.caughtUp == true) {
                echo "Replication complete: ${status.replication.completed} items"
                return
            }
            echo "Replication in progress: ${status.replication?.pending ?: '?'} pending, ${status.replication?.completed ?: '?'} completed"
        } catch (e) {
            echo "Status check failed: ${e.message}"
        }
        sleep(time: 10, unit: 'SECONDS')
    }
    error("Replication timed out after ${timeoutSeconds}s")
}
```

- [ ] **Step 3: Update human registration calls**

For the four non-Adam peers, the pipeline registers identity via `POST /auth/register` (or via the existing `seed-humans.ts` script which already does registration). Verify that `seed-humans.ts` can register humans without also seeding content.

```bash
grep -n "register\|auth\|identity" genesis/seeder/src/seed-humans.ts | head -10
```

- [ ] **Step 4: Commit**

```bash
git add genesis/Jenkinsfile
git commit -m "feat(genesis): pull-based seeding pipeline

Only Adam receives direct SQLite write. Other peers registered with
identity only, then replicate commons content via P2P. Pipeline
polls /p2p/status for replication.caughtUp before proceeding to
verification."
```

---

### Task 8: Integration Test — Replication End-to-End

**Files:**
- Create: `elohim/elohim-storage/tests/replication_integration.rs`

This test verifies the full pipeline: one node has content, another discovers and pulls it.

- [ ] **Step 1: Write the integration test**

```rust
//! Integration test for identity-driven replication.
//!
//! Verifies that a peer with no content discovers and pulls
//! commons content from a peer that has it.

#[cfg(test)]
mod tests {
    use elohim_storage::db::{self, content_diesel::{self, CreateContentInput}};
    use elohim_storage::p2p::replication::ReplicationState;
    use std::collections::HashSet;

    #[tokio::test]
    async fn replication_state_discovers_gaps() {
        let state = ReplicationState::new();

        // Simulate local node has items 1-3
        let local: HashSet<String> = ["a", "b", "c"].iter().map(|s| s.to_string()).collect();
        state.set_local_ids(local).await;

        // Peer advertises items 1-5
        let remote = vec!["a", "b", "c", "d", "e"].iter().map(|s| s.to_string()).collect();
        let gaps = state.discover(remote).await;

        assert_eq!(gaps.len(), 2);
        assert!(gaps.contains(&"d".to_string()));
        assert!(gaps.contains(&"e".to_string()));

        let status = state.status().await;
        assert_eq!(status.pending, 2);
        assert_eq!(status.completed, 0);
        assert!(!status.caught_up);
    }

    #[tokio::test]
    async fn replication_state_marks_completed() {
        let state = ReplicationState::new();
        state.set_local_ids(HashSet::new()).await;

        state.discover(vec!["x".to_string()]).await;
        assert_eq!(state.status().await.pending, 1);

        state.mark_completed("x").await;
        assert_eq!(state.status().await.pending, 0);
        assert_eq!(state.status().await.completed, 1);

        state.update_caught_up().await;
        assert!(state.status().await.caught_up);
    }

    #[tokio::test]
    async fn replication_state_retries_failures() {
        let state = ReplicationState::new();
        state.set_local_ids(HashSet::new()).await;

        state.discover(vec!["y".to_string()]).await;

        // Fail twice — should re-queue
        state.mark_failed("y").await;
        assert_eq!(state.status().await.pending, 1); // re-queued

        state.mark_failed("y").await;
        assert_eq!(state.status().await.pending, 1); // re-queued

        // Third failure — exhausted retries (MAX_RETRIES = 3)
        state.mark_failed("y").await;
        assert_eq!(state.status().await.pending, 0); // dropped
        assert_eq!(state.status().await.failed, 1);
    }

    #[tokio::test]
    async fn content_record_converts_to_create_input() {
        // Verify the ContentRecord -> CreateContentInput mapping compiles
        let record = elohim_storage::p2p::shard_protocol::ContentRecord {
            id: "test-1".to_string(),
            title: "Test".to_string(),
            description: None,
            content_type: "concept".to_string(),
            content_format: "markdown".to_string(),
            blob_hash: None,
            blob_cid: None,
            content_size_bytes: None,
            metadata_json: None,
            reach: "commons".to_string(),
            created_by: None,
            tags: vec!["test".to_string()],
            content_body: Some("# Hello".to_string()),
        };

        let input = CreateContentInput {
            id: record.id,
            title: record.title,
            description: record.description,
            content_type: record.content_type,
            content_format: record.content_format,
            blob_hash: record.blob_hash,
            blob_cid: record.blob_cid,
            content_size_bytes: record.content_size_bytes,
            metadata_json: record.metadata_json,
            reach: record.reach,
            created_by: record.created_by,
            tags: record.tags,
            content_body: record.content_body,
        };

        assert_eq!(input.id, "test-1");
        assert_eq!(input.tags, vec!["test".to_string()]);
    }
}
```

- [ ] **Step 2: Run tests**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test replication_integration --lib 2>&1 | tail -20
```

Expected: All 4 tests pass.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/replication_integration.rs
git commit -m "test(p2p): integration tests for replication state machine

Tests discovery of gaps, completion tracking, retry/failure exhaustion,
and ContentRecord to CreateContentInput mapping."
```

---

### Task 9: Add `busy_timeout` PRAGMA (Already Written — Verify)

**Files:**
- Verify: `elohim/elohim-storage/src/db/mod.rs`

The `busy_timeout` PRAGMA was already added earlier in this conversation. Verify it compiles and is correct.

- [ ] **Step 1: Verify the SqlitePragmas customizer exists**

```bash
grep -n "busy_timeout\|SqlitePragmas\|connection_customizer" elohim/elohim-storage/src/db/mod.rs
```

Expected: Shows `PRAGMA busy_timeout = 5000` and `connection_customizer(Box::new(SqlitePragmas))`.

- [ ] **Step 2: Compile check**

```bash
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check --lib 2>&1 | grep -E "^error" | head -5
```

Expected: No errors.

- [ ] **Step 3: Commit if not already committed**

```bash
git add elohim/elohim-storage/src/db/mod.rs
git commit -m "fix(storage): add PRAGMA busy_timeout = 5000 to SQLite pool

Each new connection from the pool now waits up to 5 seconds for a
locked database instead of failing immediately. Prevents 'database
is locked' errors during concurrent operations."
```

