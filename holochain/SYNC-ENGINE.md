# Sync Engine Design

## Overview

The sync engine provides **local-first** experience with CRDT-based conflict resolution using [Automerge 3.0](https://automerge.org/).

Key principles:
- **Offline-first**: Full functionality without network
- **Eventual consistency**: All nodes converge to same state
- **No conflicts**: CRDTs make concurrent edits merge cleanly
- **Stream-based sync**: Delta updates via position tracking

---

## Why Automerge

| Criterion | Automerge | Yjs | Custom |
|-----------|-----------|-----|--------|
| Data model | JSON CRDT | Text/binary | Varies |
| Conflict resolution | Automatic merge | Automatic merge | Manual |
| Language | Rust + WASM | JavaScript | - |
| Performance | Good (3.0) | Excellent | Depends |
| Fit for content | Excellent | Good for text | - |

**Decision**: Automerge 3.0 because:
- JSON data model matches our ContentNode structure
- Rust core integrates with elohim-storage
- WASM bindings work in browsers
- 3.0 addresses historical performance issues

---

## Architecture

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SYNC ENGINE                                         │
│                                                                              │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │                    LOCAL DOCUMENT STORE                             │    │
│   │                                                                     │    │
│   │   ┌─────────────┐    ┌─────────────┐    ┌─────────────┐           │    │
│   │   │  Content A  │    │  Content B  │    │  Content C  │           │    │
│   │   │ (Automerge  │    │ (Automerge  │    │ (Automerge  │           │    │
│   │   │    Doc)     │    │    Doc)     │    │    Doc)     │           │    │
│   │   └─────────────┘    └─────────────┘    └─────────────┘           │    │
│   │                                                                     │    │
│   │   Storage: SQLite (native) or IndexedDB (browser)                  │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                │                                             │
│                                │ Changes                                     │
│                                ▼                                             │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │                    EVENT LOG                                        │    │
│   │                                                                     │    │
│   │   Position: 42                                                      │    │
│   │   ┌────┐  ┌────┐  ┌────┐  ┌────┐  ┌────┐  ┌────┐                  │    │
│   │   │ 37 │→ │ 38 │→ │ 39 │→ │ 40 │→ │ 41 │→ │ 42 │                  │    │
│   │   └────┘  └────┘  └────┘  └────┘  └────┘  └────┘                  │    │
│   │                                                                     │    │
│   │   Each event: { position, doc_id, change_hash, kind }              │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                │                                             │
│                                │ Sync Protocol                               │
│                                ▼                                             │
│   ┌────────────────────────────────────────────────────────────────────┐    │
│   │                    P2P TRANSPORT (libp2p)                           │    │
│   │                                                                     │    │
│   │   Protocol: /elohim/sync/1.0.0                                     │    │
│   │                                                                     │    │
│   │   Messages:                                                         │    │
│   │   • SyncRequest { since: 37 }                                      │    │
│   │   • SyncResponse { events: [38, 39, 40, 41, 42] }                  │    │
│   │   • DocRequest { doc_id, heads }                                   │    │
│   │   • DocResponse { doc_id, changes }                                │    │
│   │                                                                     │    │
│   └────────────────────────────────────────────────────────────────────┘    │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Stream Positions

Inspired by Matrix's sync API, each agent maintains a monotonic stream position:

```rust
pub struct AgentStream {
    /// Current position (monotonically increasing)
    pub position: u64,

    /// Recent events for quick replay
    pub recent_events: VecDeque<SyncEvent>,

    /// Maximum events to keep in memory
    pub max_recent: usize,
}

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

Different event kinds help handle various sync scenarios:

```rust
pub enum EventKind {
    /// I created this change locally
    Local,

    /// Just received from a peer
    New,

    /// Historical sync (catching up)
    Backfill,

    /// Received reference before content (DAG gap)
    Outlier,
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
        /// Heads we have (for incremental sync)
        heads: Vec<String>,
    },

    /// Response with document changes
    DocResponse {
        doc_id: String,
        /// Automerge changes we don't have
        changes: Vec<Vec<u8>>,
    },

    /// Announce new local event
    Announce {
        event: SyncEvent,
    },
}
```

### Sync Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                          SYNC FLOW                                           │
│                                                                              │
│   Node A (position: 100)              Node B (position: 85)                 │
│   ┌─────────────────────┐            ┌─────────────────────┐                │
│   │                     │            │                     │                │
│   │  1. Connect         │◄──────────►│  1. Connect         │                │
│   │                     │            │                     │                │
│   │  2. Exchange        │───────────►│  2. Exchange        │                │
│   │     positions       │     100    │     positions       │                │
│   │                     │◄───────────│                     │                │
│   │                     │      85    │                     │                │
│   │                     │            │                     │                │
│   │  3. B behind, send  │───────────►│  3. Receive events  │                │
│   │     events 86-100   │   events   │     86-100          │                │
│   │                     │            │                     │                │
│   │                     │            │  4. For each event: │                │
│   │                     │            │     DocRequest      │                │
│   │                     │◄───────────│                     │                │
│   │  5. DocResponse     │───────────►│  5. Merge changes   │                │
│   │                     │   changes  │                     │                │
│   │                     │            │                     │                │
│   │  6. Both at 100     │            │  6. Now at 100      │                │
│   │                     │            │                     │                │
│   └─────────────────────┘            └─────────────────────┘                │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Conflict Resolution

Automerge handles conflicts automatically using CRDTs:

### Text Conflicts

```
Initial: "Hello World"

Alice (offline): "Hello Beautiful World"
Bob (offline):   "Hello World!"

After sync: "Hello Beautiful World!"
(Both changes preserved)
```

### Object Conflicts

```rust
// Initial
{ "title": "Original", "version": 1 }

// Alice changes title
{ "title": "Alice's Title", "version": 1 }

// Bob changes version
{ "title": "Original", "version": 2 }

// After merge
{ "title": "Alice's Title", "version": 2 }
// (Different fields, both kept)
```

### Same-Field Conflicts

When the same field is edited concurrently:

```rust
// Initial
{ "status": "draft" }

// Alice: "published"
// Bob: "archived"

// Automerge resolution: deterministic winner based on actor ID
// e.g., { "status": "published" } if Alice's ID < Bob's ID

// But! Original value available via:
doc.get_conflicts("status") // ["published", "archived"]
```

### Governance Override

For cases where automatic resolution isn't appropriate, conflicts can be escalated to Qahal governance:

```rust
pub enum ConflictResolution {
    /// Automerge handled it
    Automatic(AutomergeResolution),

    /// Escalate to governance
    Governance {
        conflict_id: String,
        options: Vec<ConflictOption>,
    },
}
```

---

## Document Lifecycle

### Creating Content

```rust
// 1. Create Automerge document
let mut doc = AutomergeDoc::new();
doc.put_object(ROOT, "id", content_id)?;
doc.put_object(ROOT, "title", title)?;
doc.put_object(ROOT, "content", content)?;

// 2. Store locally
doc_store.save(content_id, &doc)?;

// 3. Add to event log
stream.push(SyncEvent {
    position: stream.next_position(),
    doc_id: content_id.clone(),
    change_hash: doc.get_heads()[0].clone(),
    kind: EventKind::Local,
    timestamp: now(),
});

// 4. Announce to peers
transport.broadcast(SyncMessage::Announce { event })?;
```

### Receiving Content

```rust
// 1. Receive announcement
let event = receive_announce();

// 2. Check if we have this document
if !doc_store.has(&event.doc_id) {
    // Request full document
    let changes = request_doc(&event.doc_id, vec![])?;

    // Create and store
    let doc = AutomergeDoc::load(&changes)?;
    doc_store.save(&event.doc_id, &doc)?;
} else {
    // Request only missing changes
    let current = doc_store.get(&event.doc_id)?;
    let heads = current.get_heads();
    let changes = request_doc(&event.doc_id, heads)?;

    // Merge
    current.merge(&changes)?;
    doc_store.save(&event.doc_id, &current)?;
}

// 3. Add to event log
stream.push(SyncEvent {
    position: stream.next_position(),
    doc_id: event.doc_id,
    change_hash: event.change_hash,
    kind: EventKind::New,
    timestamp: now(),
});
```

---

## Storage Backend

### Native (elohim-storage)

```rust
// SQLite for document metadata and event log
// Automerge binary stored as blobs

CREATE TABLE documents (
    id TEXT PRIMARY KEY,
    automerge_data BLOB NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE events (
    position INTEGER PRIMARY KEY,
    doc_id TEXT NOT NULL,
    change_hash TEXT NOT NULL,
    kind TEXT NOT NULL,
    timestamp INTEGER NOT NULL
);

CREATE INDEX idx_events_doc ON events(doc_id);
```

### Browser (holochain-cache-core)

```typescript
// IndexedDB for persistence
interface DocStore {
  documents: {
    id: string;
    automergeData: Uint8Array;
    updatedAt: number;
  };
  events: {
    position: number;
    docId: string;
    changeHash: string;
    kind: EventKind;
    timestamp: number;
  };
}
```

---

## Integration with elohim-storage

The sync engine integrates with the existing storage layer:

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                     elohim-storage + Sync Engine                             │
│                                                                              │
│   ┌─────────────────────────┐      ┌─────────────────────────┐              │
│   │     Blob Storage        │      │     Sync Engine         │              │
│   │                         │      │                         │              │
│   │  • Large files (media)  │      │  • Content metadata     │              │
│   │  • RS shards            │      │  • Path definitions     │              │
│   │  • Direct P2P transfer  │      │  • User progress        │              │
│   │                         │      │  • Automerge docs       │              │
│   └────────────┬────────────┘      └────────────┬────────────┘              │
│                │                                 │                           │
│                │    Content references blobs     │                           │
│                │◄────────────────────────────────┤                           │
│                │                                 │                           │
│                │    Sync updates content         │                           │
│                ├────────────────────────────────►│                           │
│                │                                 │                           │
│   ┌────────────┴─────────────────────────────────┴────────────┐              │
│   │                    P2P Network (libp2p)                    │              │
│   │                                                            │              │
│   │   /elohim/shard/1.0.0  - Blob transfer                    │              │
│   │   /elohim/sync/1.0.0   - Sync protocol                    │              │
│   │                                                            │              │
│   └────────────────────────────────────────────────────────────┘              │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### What Goes Where

| Data Type | Storage | Sync |
|-----------|---------|------|
| Media files | Blob (shards) | No (hash reference) |
| Content metadata | Automerge doc | Yes |
| Path definitions | Automerge doc | Yes |
| User progress | Automerge doc | Yes |
| Attestations | DHT | No (via Holochain) |
| Trust links | DHT | No (via Holochain) |

---

## Implementation Plan

### Phase 1: Core Automerge Integration

1. Add automerge to Cargo.toml
2. Create document store abstraction
3. Implement basic CRUD with Automerge

### Phase 2: Event Log

1. Add SQLite schema for events
2. Implement stream position tracking
3. Create event kinds handling

### Phase 3: Sync Protocol

1. Define libp2p protocol messages
2. Implement sync request/response
3. Add document change exchange

### Phase 4: Conflict Handling

1. Expose Automerge conflict detection
2. Add governance escalation path
3. Implement resolution UI hooks

---

## Related Documentation

- [P2P-DATAPLANE.md](./P2P-DATAPLANE.md) - Overall P2P architecture
- [elohim-storage/P2P-ARCHITECTURE.md](./elohim-storage/P2P-ARCHITECTURE.md) - Storage P2P details
- [COMMUNITY-COMPUTE.md](./COMMUNITY-COMPUTE.md) - Vision for community-scaled sync
