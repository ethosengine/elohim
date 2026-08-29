# Document Lifecycle & Storage Backends

## Creating Content

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

## Receiving Content

```rust
// 1. Receive announcement
let event = receive_announce();

// 2. Check if we have this document
if !doc_store.has(&event.doc_id) {
    // Full document request (empty heads = give me everything)
    let changes = request_doc(&event.doc_id, vec![])?;
    let doc = AutomergeDoc::load(&changes)?;
    doc_store.save(&event.doc_id, &doc)?;
} else {
    // Incremental sync (send our heads, get only missing changes)
    let current = doc_store.get(&event.doc_id)?;
    let heads = current.get_heads();
    let changes = request_doc(&event.doc_id, heads)?;
    current.merge(&changes)?;
    doc_store.save(&event.doc_id, &current)?;
}

// 3. Add to event log
stream.push(SyncEvent {
    kind: EventKind::New,
    ..
});
```

---

## Storage Backend: Native (SQLite)

```sql
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

## Storage Backend: Browser (IndexedDB)

```typescript
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

## Browser-Side Integration

### OfflineOperationQueueService

Queues operations while offline, replays when connected:

```typescript
// Queue write while offline
offlineQueue.enqueue({
  type: 'update-mastery',
  docId: 'mastery:user123/concept-x',
  changes: automergeChanges,
});

// On reconnect: replay queue
offlineQueue.flush(async (op) => {
  await client.applyChanges(op.docId, op.changes);
});
```

### IndexedDB for Persistence

Browser stores Automerge docs in IndexedDB for offline access:

```typescript
// Save doc locally
await idb.put('documents', { id: docId, data: doc.save(), updatedAt: Date.now() });

// Load doc on startup
const stored = await idb.get('documents', docId);
const doc = Automerge.load(stored.data);
```
