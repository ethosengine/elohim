# @elohim/storage-client

TypeScript client SDK for elohim-storage sync API.

## Installation

```bash
npm install @elohim/storage-client
```

## Usage

### Basic HTTP Client

```typescript
import { StorageClient } from '@elohim/storage-client';

const client = new StorageClient({
  baseUrl: 'http://localhost:8080',
  appId: 'lamad',
  apiKey: process.env.STORAGE_API_KEY, // optional
  timeout: 30000, // optional, default 30s
});

// List documents
const { documents, total } = await client.listDocuments({
  prefix: 'graph',
  limit: 100,
});

// Get document heads
const { heads } = await client.getHeads('graph:my-doc');

// Get changes since known heads
const { changes, new_heads } = await client.getChangesSince('graph:my-doc', knownHeads);

// Apply changes (base64-encoded from server, or Uint8Array)
await client.applyChanges('graph:my-doc', [changesBytes]);
```

### Automerge Sync Helper

For higher-level Automerge document operations:

```typescript
import { StorageClient, AutomergeSync } from '@elohim/storage-client';
import * as Automerge from '@automerge/automerge';

interface MyDoc {
  title: string;
  items: string[];
}

const client = new StorageClient({ baseUrl: 'http://localhost:8080', appId: 'lamad' });
const sync = new AutomergeSync(client);

// Load document (creates empty if doesn't exist)
let doc = await sync.load<MyDoc>('graph:my-doc');

// Make local changes
doc = Automerge.change(doc, d => {
  d.title = 'Updated Title';
  d.items.push('new item');
});

// Save to server
await sync.save('graph:my-doc', doc);

// Bidirectional sync (get server changes + send local changes)
const { doc: synced, changed, heads } = await sync.sync('graph:my-doc', doc);
```

### Blob Storage

```typescript
// Store a blob
const manifest = await client.putBlob(imageData, 'image/png');
console.log('Stored with CID:', manifest.blob_cid);

// Get a blob
const data = await client.getBlob(manifest.blob_hash);

// Check if blob exists
const exists = await client.blobExists(hashOrCid);

// Get manifest
const info = await client.getManifest(hashOrCid);
```

## API Reference

### StorageClient

| Method | Description |
|--------|-------------|
| `listDocuments(options)` | List documents with pagination |
| `getDocument(docId)` | Get document info |
| `getHeads(docId)` | Get current document heads |
| `getChangesSince(docId, heads)` | Get changes since given heads |
| `applyChanges(docId, changes)` | Apply changes to document |
| `countDocuments()` | Get document count |
| `putBlob(data, mimeType)` | Store a blob |
| `getBlob(hashOrCid)` | Get blob data |
| `blobExists(hashOrCid)` | Check if blob exists |
| `getManifest(hashOrCid)` | Get blob manifest |

### AutomergeSync

| Method | Description |
|--------|-------------|
| `load<T>(docId)` | Load document from server |
| `save<T>(docId, doc)` | Save local changes to server |
| `sync<T>(docId, doc)` | Bidirectional sync |
| `exists(docId)` | Check if document exists |
| `forget(docId)` | Clear local head tracking |

## License

AGPL-3.0
