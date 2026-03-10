# elohim-storage-client

Rust client SDK for elohim-storage sync API.

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
elohim-storage-client = { path = "../elohim-storage-client" }
```

### Basic HTTP Client

```rust
use elohim_storage_client::{StorageClient, StorageConfig, ListOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StorageClient::new(StorageConfig {
        base_url: "http://localhost:8080".into(),
        app_id: "lamad".into(),
        api_key: std::env::var("STORAGE_API_KEY").ok(),
        timeout_secs: 30,
    });

    // List documents
    let response = client.list_documents(ListOptions {
        prefix: Some("graph".into()),
        limit: Some(100),
        ..Default::default()
    }).await?;

    // Get document heads
    let heads = client.get_heads("graph:my-doc").await?;

    // Get changes since known heads
    let changes = client.get_changes_since("graph:my-doc", &heads.heads).await?;

    // Apply changes
    let result = client.apply_changes("graph:my-doc", &[changes_bytes]).await?;

    Ok(())
}
```

### Automerge Sync Helper

For higher-level Automerge document operations:

```rust
use elohim_storage_client::{StorageClient, StorageConfig, AutomergeSync};
use automerge::transaction::Transactable;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = StorageClient::new(StorageConfig {
        base_url: "http://localhost:8080".into(),
        app_id: "lamad".into(),
        ..Default::default()
    });
    let mut sync = AutomergeSync::new(client);

    // Load document (creates empty if doesn't exist)
    let mut doc = sync.load("graph:my-doc").await?;

    // Make local changes
    doc.transact::<_, _, automerge::AutomergeError>(|tx| {
        tx.put(automerge::ROOT, "title", "Updated Title")?;
        Ok(())
    })?;

    // Save to server
    sync.save("graph:my-doc", &doc).await?;

    // Bidirectional sync (get server changes + send local changes)
    let result = sync.sync("graph:my-doc", doc).await?;
    println!("Changed: {}, Heads: {:?}", result.changed, result.heads);

    Ok(())
}
```

### Blob Storage

```rust
// Store a blob
let manifest = client.put_blob(&image_data, "image/png").await?;
println!("Stored with hash: {}", manifest.blob_hash);

// Get a blob
let data = client.get_blob(&manifest.blob_hash).await?;

// Check if blob exists
let exists = client.blob_exists(&hash_or_cid).await?;

// Get manifest
let info = client.get_manifest(&hash_or_cid).await?;
```

## API Reference

### StorageClient

| Method | Description |
|--------|-------------|
| `list_documents(options)` | List documents with pagination |
| `get_document(doc_id)` | Get document info |
| `get_heads(doc_id)` | Get current document heads |
| `get_changes_since(doc_id, heads)` | Get changes since given heads |
| `apply_changes(doc_id, changes)` | Apply changes to document |
| `count_documents()` | Get document count |
| `put_blob(data, mime_type)` | Store a blob |
| `get_blob(hash_or_cid)` | Get blob data |
| `blob_exists(hash_or_cid)` | Check if blob exists |
| `get_manifest(hash_or_cid)` | Get blob manifest |

### AutomergeSync

| Method | Description |
|--------|-------------|
| `load(doc_id)` | Load document from server |
| `save(doc_id, doc)` | Save local changes to server |
| `sync(doc_id, doc)` | Bidirectional sync |
| `exists(doc_id)` | Check if document exists |
| `forget(doc_id)` | Clear local head tracking |

## License

AGPL-3.0
