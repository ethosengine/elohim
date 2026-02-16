---
name: automerge-sync
description: Reference for Automerge 0.5 CRDT sync engine, stream positions, delta sync protocol, document lifecycle, conflict resolution, and browser-side integration. Use when someone asks "how does sync work", "handle merge conflicts", "manage stream positions", "offline-first content", or implements CRDT document flows.
metadata:
  author: elohim-protocol
  version: 1.0.0
---

# Automerge Sync Engine Reference

The sync engine provides **local-first** experience with CRDT-based conflict resolution using Automerge.

## Design Philosophy

- **Offline-first**: Full functionality without network
- **Eventual consistency**: All nodes converge to same state
- **No conflicts**: CRDTs make concurrent edits merge cleanly
- **Stream-based sync**: Delta updates via position tracking

---

## Document Model

Each content item is one Automerge document, stored in SQLite (native) or IndexedDB (browser).

```
Content item "concept-governance"  ->  Automerge document (binary blob)
Content item "path-onboarding"     ->  Automerge document (binary blob)
User progress "user123/concept-x"  ->  Automerge document (binary blob)
```

**What goes in Automerge docs:**
- Content metadata (title, description, type)
- Path definitions (chapters, steps, ordering)
- User progress (mastery levels, engagement counts)

**What does NOT go in Automerge docs:**
- Media files (use blob/shard storage instead - hash references only)
- Attestations (use Holochain DHT)
- Trust links (use Holochain DHT)

---

## Stream Positions

Inspired by Matrix's sync API. Each agent maintains a monotonic sequence number.

```rust
pub struct AgentStream {
    /// Current position (monotonically increasing)
    pub position: u64,

    /// Recent events for quick replay
    pub recent_events: VecDeque<SyncEvent>,

    /// Maximum events to keep in memory
    pub max_recent: usize,
}
```

### SyncEvent

```rust
pub struct SyncEvent {
    /// Position in this agent's stream
    pub position: u64,

    /// Document that changed
    pub doc_id: String,

    /// Automerge change hash
    pub change_hash: String,

    /// Kind of event
    pub kind: EventKind,

    /// Timestamp
    pub timestamp: u64,
}
```

### Event Kinds

```rust
pub enum EventKind {
    Local,      // I created this change locally
    New,        // Just received from a peer
    Backfill,   // Historical sync (catching up)
    Outlier,    // Received reference before content (DAG gap)
}
```

| Kind | When Used | Priority |
|------|-----------|----------|
| Local | User edits offline | Highest - sync first |
| New | Received from peer | High - recent activity |
| Backfill | Joining network | Normal - historical |
| Outlier | DAG incomplete | Low - resolve later |

---

## Sync Protocol

### Message Types

```rust
pub enum SyncMessage {
    /// Request events since position
    SyncRequest {
        since: u64,
        limit: Option<u32>,
    },

    /// Response with events
    SyncResponse {
        events: Vec<SyncEvent>,
        has_more: bool,
    },

    /// Request document changes
    DocRequest {
        doc_id: String,
        heads: Vec<String>,  // Automerge heads we have
    },

    /// Response with document changes
    DocResponse {
        doc_id: String,
        changes: Vec<Vec<u8>>,  // Automerge change blobs
    },

    /// Announce new local event
    Announce {
        event: SyncEvent,
    },
}
```

### Sync Flow

```
Node A (position: 100)              Node B (position: 85)

1. Connect                          1. Connect

2. Exchange positions               2. Exchange positions
   "I'm at 100"  ───────────────>
                  <───────────────  "I'm at 85"

3. B is behind, A sends             3. Receive events 86-100
   events 86-100  ───────────────>

                                    4. For each new doc_id:
                                       DocRequest { doc_id, heads }
                  <───────────────

5. DocResponse { changes }          5. Merge changes into local doc
               ───────────────>

6. Both at 100                      6. Now at 100
```

### Periodic Sync

The `SyncCoordinator` schedules periodic sync with connected peers:

```
Every 30s:
  For each connected peer:
    If peer.sync_position < my_position:
      Send SyncResponse with delta events
    If peer.sync_position > my_position:
      Send SyncRequest { since: my_position }
```

---

## Conflict Resolution

### Automerge handles conflicts automatically:

**Different fields** - Both changes preserved:
```
Initial: { title: "Original", version: 1 }
Alice:   { title: "Alice's Title", version: 1 }
Bob:     { title: "Original", version: 2 }
Merged:  { title: "Alice's Title", version: 2 }
```

**Same field** - Deterministic winner (by actor ID):
```
Initial: { status: "draft" }
Alice:   { status: "published" }
Bob:     { status: "archived" }
Merged:  { status: "published" }  // if Alice's ID < Bob's
```

Access conflicting values:
```rust
doc.get_conflicts("status")  // ["published", "archived"]
```

### Governance Override

For conflicts requiring human judgment:
```rust
pub enum ConflictResolution {
    Automatic(AutomergeResolution),
    Governance {
        conflict_id: String,
        options: Vec<ConflictOption>,
    },
}
```

---

## Document Lifecycle

Creating: build Automerge doc -> store locally -> add to event log (EventKind::Local) -> announce to peers.
Receiving: check if doc exists -> full request (empty heads) or incremental sync (send our heads) -> merge -> log as EventKind::New.

See `references/document-lifecycle.md` for full Rust code examples, SQLite/IndexedDB schemas, storage-client-ts sync methods, and OfflineOperationQueueService patterns.

### storage-client-ts Sync Methods

Key methods: `listDocuments()`, `getHeads()`, `getChangesSince()`, `applyChanges()`, `countDocuments()`. See `references/document-lifecycle.md` for usage examples.

---

## Gotchas

1. **Automerge 0.5 crate vs 3.0 design** - The Rust crate is `automerge = "0.5"` (in Cargo.toml). The design docs reference "Automerge 3.0" which is the JavaScript version. The Rust API differs.

2. **Per-agent stream positions** - Each agent has its own monotonic position. Don't confuse with document-level heads. Positions track the agent's event log, heads track document state.

3. **DAG gaps (Outlier events)** - If you receive a change reference before having the change itself, mark as `Outlier` and fetch later. Don't block sync.

4. **Blob references, not blob sync** - Automerge docs contain hash references to blobs. The blob data itself transfers via the shard protocol, not the sync protocol.

5. **CRDT merge is deterministic** - Same inputs always produce same output. No coordination needed for merge. But same-field conflicts resolve by actor ID ordering, which may surprise users.

---

## Key Files

| File | Purpose |
|------|---------|
| `holochain/SYNC-ENGINE.md` | Primary design document |
| `elohim-node/src/sync/coordinator.rs` | SyncCoordinator (peer tracking, scheduling) |
| `elohim-node/src/sync/protocol.rs` | SyncMessage types |
| `elohim-node/src/sync/stream.rs` | AgentStream, position tracking |
| `elohim-node/src/sync/merge.rs` | Automerge merge operations |
| `holochain/sdk/storage-client-ts/src/client.ts` | TypeScript sync API |
| `holochain/P2P-DATAPLANE.md` | Overall P2P architecture |

## External References

- Automerge docs: `https://automerge.org/docs/`
- Automerge Rust crate (0.5): `https://docs.rs/automerge/0.5/automerge/`
- Automerge JS: `https://automerge.org/docs/quickstart/`
