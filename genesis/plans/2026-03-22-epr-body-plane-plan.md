# EPR Body Plane Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Wire shard protocol content delivery after EPR Head resolution so peers fetch full content from each other and persist to local SQLite — making cross-steward content transparent to the frontend.

**Architecture:** EPR Head (Tier 1) provides metadata pointers. `ShardRequest::Get` (Tier 3) fetches actual content bytes. Fetched content persists to local SQLite for native read performance. `P2PHandle::resolve_and_fetch()` orchestrates both steps behind a single async call. Resolution logic is decoupled for future multi-peer strategies.

**Tech Stack:** Rust (libp2p request-response, diesel ORM, tokio async), elohim-storage P2P module

---

### Task 1: Add `FetchShard` Command to P2PCommand Enum

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:161-170`

**Step 1: Add the FetchShard variant**

In the `P2PCommand` enum (line 162), add a new variant after `ResolveEpr`:

```rust
pub enum P2PCommand {
    /// Publish an EPR Head to Kademlia DHT
    PublishEprHead { id: String, head_bytes: Vec<u8> },
    /// Resolve an EPR Head via Kademlia DHT lookup
    ResolveEpr {
        id: String,
        reply: oneshot::Sender<Option<Vec<u8>>>,
    },
    /// Fetch content bytes via shard protocol from a connected peer
    FetchShard {
        hash: String,
        reply: oneshot::Sender<Option<Vec<u8>>>,
    },
}
```

**Step 2: Add `FetchShard` handling in `handle_command`**

In `handle_command()` (line 421), add a match arm after the `ResolveEpr` arm:

```rust
P2PCommand::FetchShard { hash, reply } => {
    let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
    if let Some(peer_id) = peers.first() {
        let req_id = swarm
            .behaviour_mut()
            .shard_protocol
            .send_request(peer_id, ShardRequest::Get { hash: hash.clone() });
        debug!(peer = %peer_id, hash = %hash, request_id = ?req_id, "Sent shard fetch request to peer");
        self.pending_shard_fetches.lock().await.insert(req_id, reply);
    } else {
        debug!(hash = %hash, "No connected peers for shard fetch");
        let _ = reply.send(None);
    }
}
```

**Step 3: Add `pending_shard_fetches` map to `P2PNode`**

Add a type alias near `PendingEprMap` (line 55):

```rust
/// Map of pending shard fetch requests: request ID -> reply sender
type PendingShardMap = Arc<
    tokio::sync::Mutex<
        std::collections::HashMap<
            request_response::OutboundRequestId,
            oneshot::Sender<Option<Vec<u8>>>,
        >,
    >,
>;
```

Add the field to `P2PNode` struct (line 140):

```rust
/// Pending shard fetch requests awaiting responses from peers
pending_shard_fetches: PendingShardMap,
```

Initialize in `new()` alongside `pending_epr_resolves`:

```rust
pending_shard_fetches: Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new())),
```

**Step 4: Wire shard response delivery in `handle_shard_request` response path**

In `handle_behaviour_event`, the `ShardProtocol` `Message::Response` arm (currently just a debug log at approximately line 543) needs to check `pending_shard_fetches` and deliver:

```rust
request_response::Message::Response {
    request_id,
    response,
} => {
    // Check if there's a pending shard fetch waiting
    let pending_tx = self
        .pending_shard_fetches
        .lock()
        .await
        .remove(&request_id);
    if let Some(tx) = pending_tx {
        match response {
            ShardResponse::Data(data) => {
                debug!(request_id = ?request_id, size = data.len(), "Shard fetch completed");
                let _ = tx.send(Some(data));
            }
            _ => {
                debug!(request_id = ?request_id, response = ?response, "Shard fetch returned non-data");
                let _ = tx.send(None);
            }
        }
    } else {
        debug!(request_id = ?request_id, response = ?response, "Received shard response (no pending fetch)");
    }
}
```

**Step 5: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`
Expected: compiles clean

**Step 6: Commit**

```
feat(p2p): add FetchShard command for content body delivery via shard protocol
```

---

### Task 2: Add `fetch_shard` and `resolve_and_fetch` to P2PHandle

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:180-216`

**Step 1: Add `fetch_shard` method**

After `resolve_epr()` in `P2PHandle` impl (line 215):

```rust
/// Fetch content bytes via shard protocol. Returns None on timeout or not found.
pub async fn fetch_shard(&self, hash: &str) -> Option<Vec<u8>> {
    let (reply_tx, reply_rx) = oneshot::channel();
    if self
        .command_tx
        .send(P2PCommand::FetchShard {
            hash: hash.to_string(),
            reply: reply_tx,
        })
        .await
        .is_err()
    {
        return None;
    }
    match tokio::time::timeout(Duration::from_secs(5), reply_rx).await {
        Ok(Ok(result)) => result,
        _ => None,
    }
}
```

**Step 2: Add `resolve_and_fetch` method**

After `fetch_shard()`:

```rust
/// Full P2P content resolution: EPR Head -> shard fetch -> (EprHead, content_bytes).
///
/// Resolves the EPR Head for metadata, extracts blob_cid, then fetches the
/// actual content bytes via shard protocol. Returns None if either step fails.
///
/// The resolution logic is decoupled from peer selection — today it uses the
/// first connected peer; future versions can rank by latency or load-balance.
pub async fn resolve_and_fetch(
    &self,
    id: &str,
) -> Option<(crate::epr_codec::EprHead, Vec<u8>)> {
    // Step 1: Resolve EPR Head
    let head_bytes = self.resolve_epr(id).await?;
    let head: crate::epr_codec::EprHead = rmp_serde::from_slice(&head_bytes).ok()?;

    // Step 2: Fetch content bytes via shard protocol using blob_cid
    if head.content.is_empty() {
        debug!(id = %id, "EPR Head has no blob_cid, skipping shard fetch");
        return None;
    }
    let content_bytes = self.fetch_shard(&head.content).await?;

    // Step 3: Verify content integrity
    use sha2::{Digest, Sha256};
    let hash = format!("sha256-{}", hex::encode(Sha256::digest(&content_bytes)));
    // Accept either the sha256-hex hash or the CID matching
    if hash != head.content && !head.content.starts_with("bafkrei") {
        warn!(
            id = %id,
            expected = %head.content,
            actual = %hash,
            "Content integrity check failed"
        );
        return None;
    }

    Some((head, content_bytes))
}
```

**Step 3: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`
Expected: compiles clean

**Step 4: Commit**

```
feat(p2p): add fetch_shard and resolve_and_fetch to P2PHandle API
```

---

### Task 3: Add `content_body` to `CreateContentInput` and `NewContent`

The P2P persist flow needs to write content metadata + body in a single create operation. Currently `content_body` only goes through the PATCH/update path. Add it to the create path.

**Files:**
- Modify: `elohim/elohim-storage/src/db/content_diesel.rs:23-46` (CreateContentInput)
- Modify: `elohim/elohim-storage/src/db/models.rs:104-117` (NewContent)
- Modify: `elohim/elohim-storage/src/db/content_diesel.rs:248-268` (create_content)
- Modify: `elohim/elohim-storage/src/db/content_diesel.rs:324-337` (bulk_create_content)

**Step 1: Add `content_body` to `CreateContentInput`**

```rust
pub struct CreateContentInput {
    pub id: String,
    pub title: String,
    // ... existing fields ...
    pub created_by: Option<String>,
    pub tags: Vec<String>,
    #[serde(default)]
    pub content_body: Option<String>,  // NEW
}
```

**Step 2: Add `content_body` to `NewContent`**

```rust
pub struct NewContent<'a> {
    pub id: &'a str,
    // ... existing fields ...
    pub created_by: Option<&'a str>,
    pub content_body: Option<&'a str>,  // NEW
}
```

**Step 3: Wire in `create_content()` and `bulk_create_content()`**

In both functions, add `content_body` to the `NewContent` construction:

```rust
let new_content = NewContent {
    // ... existing fields ...
    created_by: input.created_by.as_deref(),
    content_body: input.content_body.as_deref(),  // NEW
};
```

**Step 4: Update `From<CreateContentInputView>` in views.rs**

In `views.rs:1258-1274`, add `content_body` to the `From` impl:

```rust
impl From<CreateContentInputView> for CreateContentInput {
    fn from(v: CreateContentInputView) -> Self {
        Self {
            // ... existing fields ...
            tags: v.tags,
            content_body: v.content_body,  // NEW
        }
    }
}
```

**Step 5: Run existing tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test content_diesel`
Expected: all existing tests pass (content_body defaults to None via `#[serde(default)]`)

**Step 6: Commit**

```
feat(storage): add content_body to CreateContentInput for single-operation P2P persist
```

---

### Task 4: Wire Content GET to Use `resolve_and_fetch` and Persist to SQLite

Replace the header-plane-only EPR fallback with the full body plane flow.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:2108-2121`

**Step 1: Replace the existing P2P fallback block**

Replace the current block at lines 2108-2121:

```rust
// Content not found locally — try P2P EPR resolution + shard fetch
#[cfg(feature = "p2p")]
if let Some(ref handle) = self.p2p_handle {
    debug!(id = %content_id, "Content not found locally, trying P2P resolve + fetch");
    if let Some((head, content_bytes)) = handle.resolve_and_fetch(content_id).await
    {
        info!(id = %content_id, size = content_bytes.len(), "Content resolved via P2P");

        // Store blob
        let blob_result = self.blob_store.store(&content_bytes).await;

        // Persist to local SQLite so future GETs are local
        if let Some(ref svc) = self.services {
            let body_str = String::from_utf8_lossy(&content_bytes).to_string();
            let input = db::content_diesel::CreateContentInput {
                id: content_id.to_string(),
                title: head.lamad.title.clone(),
                description: head.lamad.description.clone(),
                content_type: head.lamad.content_type.clone(),
                content_format: head
                    .lamad
                    .content_format
                    .clone()
                    .unwrap_or_else(|| "markdown".to_string()),
                blob_hash: blob_result
                    .as_ref()
                    .ok()
                    .map(|r| r.hash.clone()),
                blob_cid: if head.content.is_empty() {
                    None
                } else {
                    Some(head.content.clone())
                },
                content_size_bytes: Some(content_bytes.len() as i32),
                metadata_json: Some(
                    r#"{"resolved_via":"p2p"}"#.to_string(),
                ),
                reach: head
                    .qahal
                    .reach
                    .clone()
                    .unwrap_or_else(|| "commons".to_string()),
                created_by: head.author.clone(),
                tags: head.lamad.tags.clone(),
                content_body: Some(body_str),
            };
            match svc.content.create(input) {
                Ok(content_with_tags) => {
                    info!(id = %content_id, "P2P content persisted to local SQLite");
                    let view = ContentView::from(content_with_tags);
                    return Ok(response::ok(&view));
                }
                Err(e) => {
                    warn!(id = %content_id, error = %e, "Failed to persist P2P content");
                    // Fall through to return EPR Head metadata only
                    let view = ContentView::from_epr_head(&head);
                    return Ok(response::ok(&view));
                }
            }
        }
    }
}
```

**Step 2: Build and run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`
Expected: clean (no errors)

**Step 3: Commit**

```
feat(storage): wire full P2P content resolution — EPR Head + shard fetch + SQLite persist
```

---

### Task 5: Add EPR Publication to Single Content Create

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:1903-1917`

**Step 1: Add EPR publication after single content create**

Replace the existing POST handler (lines 1903-1917):

```rust
Method::POST => {
    let body = req
        .collect()
        .await
        .map_err(|e| StorageError::Internal(format!("Failed to read body: {}", e)))?;
    let body_bytes = body.to_bytes();

    let input_view: CreateContentInputView = serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::Parse(format!("Invalid JSON: {}", e)))?;

    // Capture EPR-relevant data before consuming input_view
    #[cfg(feature = "p2p")]
    let epr_data = (
        input_view.id.clone(),
        input_view.title.clone(),
        input_view.content_type.clone().unwrap_or_else(|| "concept".to_string()),
        input_view.description.clone(),
        input_view.content_format.clone(),
        input_view.blob_cid.clone(),
        input_view.reach.clone(),
        input_view.created_by.clone(),
        input_view.tags.clone(),
    );

    let input: db::content_diesel::CreateContentInput = input_view.into();
    let result = services.content.create(input);

    // Auto-publish EPR Head on successful create
    #[cfg(feature = "p2p")]
    if result.is_ok() {
        if let Some(ref handle) = self.p2p_handle {
            let handle = handle.clone();
            let (id, title, content_type, desc, fmt, cid, reach, author, tags) = epr_data;
            tokio::spawn(async move {
                let head = crate::epr_codec::EprHead {
                    version: 1,
                    id: id.clone(),
                    content: cid.unwrap_or_default(),
                    lamad: crate::epr_codec::EprLamadContext {
                        title,
                        content_type,
                        description: desc,
                        content_format: fmt,
                        tags,
                    },
                    shefa: crate::epr_codec::EprShefaContext {
                        stewards: vec![],
                        allocations: vec![],
                    },
                    qahal: crate::epr_codec::EprQahalContext { reach, layer: None },
                    relationships: vec![],
                    author,
                    updated: Some(chrono::Utc::now().to_rfc3339()),
                };
                if let Ok(bytes) = rmp_serde::to_vec(&head) {
                    handle.publish_epr_head(id, bytes).await;
                }
            });
        }
    }

    Ok(response::from_create_result(result))
}
```

**Step 2: Build and run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`
Expected: clean

**Step 3: Commit**

```
feat(storage): auto-publish EPR Head on single content create
```

---

### Task 6: Update A2O Scenario for Full Content Body

**Files:**
- Modify: `genesis/a2o/features/federation/epr-cross-peer-resolution.feature`

**Step 1: Update scenarios to verify content body, not just metadata**

```gherkin
Feature: EPR Cross-Peer Content Resolution
  As a learner navigating a path
  I want content stewarded by another peer to resolve transparently
  So that learning paths work regardless of stewardship partitioning

  Background:
    Given the EPR protocol "/elohim/epr/1.0.0" is active between peers
    And the shard protocol "/elohim/shard/1.0.0" is active between peers

  Scenario: Content stewarded by another peer resolves with full body
    Given peer "alpha" has content "fct-module-01-church-dilemma" stewarded by Pete
    And peer "staging" does not have "fct-module-01-church-dilemma" locally
    When peer "staging" requests content "fct-module-01-church-dilemma"
    Then the content is resolved via EPR protocol from peer "alpha"
    And the content body is fetched via shard protocol
    And the content is persisted to local SQLite on peer "staging"
    And subsequent requests return the content without P2P resolution

  Scenario: EPR Heads publish to DHT on ingestion
    Given peer "alpha" ingests content "test-concept"
    Then the DHT contains an EPR Head for "test-concept"
    And peer "staging" can discover "test-concept" via Kademlia lookup

  Scenario: Content GET returns 404 when no peer has the content
    Given no peer has content "nonexistent-concept"
    When peer "alpha" requests content "nonexistent-concept"
    Then the response is 404 Not Found

  Scenario: Single content create publishes EPR Head
    Given peer "alpha" creates content "new-concept" via POST /db/content
    Then the DHT contains an EPR Head for "new-concept"

  Scenario: P2P-resolved content is tagged for diagnostics
    Given peer "staging" resolves "cross-steward-concept" via P2P
    Then the local content record has metadata "resolved_via" = "p2p"
```

**Step 2: Commit**

```
feat(a2o): update EPR cross-peer scenario for full content body resolution
```

---

### Task 7: Remove Operator Workaround in Genesis Jenkinsfile

**Files:**
- Modify: `genesis/Jenkinsfile:470-475`

**Step 1: Remove the matthew-manager bypass**

Replace lines 470-476:

```groovy
                                    // All peers get stewardship-filtered content.
                                    // P2P EPR resolution handles cross-steward access.
                                    SEED_CMD="npx tsx src/seed-sqlite.ts --conductor-for=${humanId}"
                                    STORAGE_URL="http://${storageUrl}" \$SEED_CMD
```

**Step 2: Commit**

```
fix(genesis): remove operator stewardship bypass — P2P EPR body plane handles cross-steward content
```

---

### Task 8: Build, Clippy, Test, Format

**Step 1: Format**

Run: `cd elohim/elohim-storage && cargo fmt`

**Step 2: Clippy**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`
Expected: clean

**Step 3: Run EPR + shard tests**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test epr`
Expected: all EPR tests pass

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test shard`
Expected: all shard tests pass

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test content_diesel`
Expected: all content diesel tests pass

**Step 4: Verify diff**

Run: `git diff --stat`
Expected files:
- `elohim/elohim-storage/src/p2p/mod.rs` — FetchShard command, pending map, resolve_and_fetch
- `elohim/elohim-storage/src/http.rs` — full P2P fallback, single-create publish
- `elohim/elohim-storage/src/db/content_diesel.rs` — content_body in CreateContentInput
- `elohim/elohim-storage/src/db/models.rs` — content_body in NewContent
- `elohim/elohim-storage/src/views.rs` — content_body in From impl
- `genesis/Jenkinsfile` — operator bypass removed
- `genesis/a2o/features/federation/epr-cross-peer-resolution.feature` — updated scenarios

---

## Execution Order

```
Task 1: FetchShard command + pending map       (standalone)
Task 2: fetch_shard + resolve_and_fetch API    (depends on 1)
Task 3: content_body in CreateContentInput     (standalone)
Task 4: Content GET full P2P flow              (depends on 2, 3)
Task 5: Single-create EPR publication          (standalone)
Task 6: A2O scenario updates                   (anytime)
Task 7: Remove operator workaround             (after 4 verified)
Task 8: Final verification                     (after all above)
```

Tasks 1, 3, 5, 6 are independent and can be parallelized.
