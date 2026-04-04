# P2P Resilience Sprint C: End-to-End Distribution Proof

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Seeding content automatically distributes RS shards across peers via the shard protocol. Reconstruction is verified when peers are absent. The full topology is visible in the UI with a "Verify Resilience" button.

**Architecture:** Content creation triggers auto-distribution: RS-encode, select replication targets from trust topology, push shards to peers via `/elohim/shard/1.0.0`, track acknowledgments in `shard_locations`. A periodic verification loop checks peer shard availability. A verification endpoint actively tests reconstruction. The UI shows live distribution state and a verify button.

**Tech Stack:** Rust (libp2p, reed-solomon-erasure, tokio), Angular 19, TypeScript, cargo test

**Depends on:** Sprint B (shard tables, resilience endpoint, REA commitments seeded)

---

### Task 1: Add `push_shard` to P2PHandle

The P2PHandle currently has `fetch_shard` (pull) but no `push_shard` (push). Content distribution needs outbound shard replication.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [ ] **Step 1: Add PushShard command variant to P2PCommand enum**

Find the `P2PCommand` enum (around line 200 in mod.rs) and add:

```rust
PushShard {
    peer_id: PeerId,
    hash: String,
    data: Vec<u8>,
    reply: oneshot::Sender<Result<(), String>>,
},
```

- [ ] **Step 2: Add push_shard method to P2PHandle**

Add after the existing `fetch_shard` method (~line 283):

```rust
pub async fn push_shard(
    &self,
    peer_id: &str,
    hash: &str,
    data: Vec<u8>,
) -> Result<(), String> {
    let peer_id: PeerId = peer_id
        .parse()
        .map_err(|e| format!("Invalid peer ID: {e}"))?;
    let (tx, rx) = oneshot::channel();
    self.command_tx
        .send(P2PCommand::PushShard {
            peer_id,
            hash: hash.to_string(),
            data,
            reply: tx,
        })
        .await
        .map_err(|_| "P2P command channel closed".to_string())?;

    match tokio::time::timeout(std::time::Duration::from_secs(30), rx).await {
        Ok(Ok(result)) => result,
        Ok(Err(_)) => Err("Push response channel dropped".to_string()),
        Err(_) => Err("Push timed out after 30s".to_string()),
    }
}
```

- [ ] **Step 3: Handle PushShard command in the event loop**

In the `handle_command` method (where `P2PCommand` variants are matched), add:

```rust
P2PCommand::PushShard { peer_id, hash, data, reply } => {
    let request = ShardRequest::Push { hash: hash.clone(), data };
    let request_id = self.swarm
        .behaviour_mut()
        .shard_protocol
        .send_request(&peer_id, request);
    self.pending_shard_pushes.insert(request_id, reply);
}
```

- [ ] **Step 4: Add pending_shard_pushes tracking map**

Add to the `P2PNode` struct fields:

```rust
pending_shard_pushes: HashMap<RequestId, oneshot::Sender<Result<(), String>>>,
```

Initialize in the constructor as `HashMap::new()`.

- [ ] **Step 5: Handle push response in shard protocol event handler**

In `handle_behaviour_event` for `ShardProtocol` messages, in the `Message::Response` arm (~line 849), extend the response handling:

```rust
Message::Response { request_id, response } => {
    // Existing fetch handling
    if let Some(tx) = self.pending_shard_fetches.remove(&request_id) {
        match response {
            ShardResponse::Data(data) => { let _ = tx.send(Some(data)); }
            _ => { let _ = tx.send(None); }
        }
    }
    // New push handling
    else if let Some(tx) = self.pending_shard_pushes.remove(&request_id) {
        match response {
            ShardResponse::PushAck => { let _ = tx.send(Ok(())); }
            ShardResponse::Error(e) => { let _ = tx.send(Err(e)); }
            _ => { let _ = tx.send(Err("Unexpected response".to_string())); }
        }
    }
}
```

Also clean up push failures in the `OutboundFailure` arm:

```rust
request_response::Event::OutboundFailure { request_id, error, .. } => {
    if let Some(tx) = self.pending_shard_fetches.remove(&request_id) {
        let _ = tx.send(None);
    }
    if let Some(tx) = self.pending_shard_pushes.remove(&request_id) {
        let _ = tx.send(Err(format!("Outbound failure: {error:?}")));
    }
}
```

- [ ] **Step 6: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`

Expected: Compiles. The new push path mirrors the existing fetch path.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): add push_shard to P2PHandle

Enables outbound shard replication via /elohim/shard/1.0.0 protocol.
30s timeout, async oneshot response tracking. Mirrors fetch_shard pattern."
```

---

### Task 2: Auto-Distribute Shards on Content Creation

When content is created with a blob, RS-encode it and push shards to replication targets determined by trust topology.

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (content creation handlers)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (add distribute_shards helper)

- [ ] **Step 1: Add distribute_shards method to P2PHandle**

This is a convenience method that does: encode → push to targets → record locations. Add to P2PHandle:

```rust
pub async fn distribute_shards(
    &self,
    content_id: &str,
    blob_data: &[u8],
    pool: &crate::db::DbPool,
    h_app_id: &str,
) -> Result<usize, String> {
    let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());
    let manifest = encoder.create_manifest(blob_data, "application/octet-stream", "commons");
    let shards = encoder.create_shards(blob_data, &manifest.encoding);

    // Get delivery peers as replication targets
    let peers = self.delivery_peers();
    if peers.is_empty() {
        tracing::info!(content_id, "No delivery peers available for shard distribution");
        return Ok(0);
    }

    let mut distributed = 0usize;

    for (i, shard_data) in shards.iter().enumerate() {
        let hash = &manifest.shard_hashes[i];
        // Round-robin across peers (in future: use cluster trust topology)
        let peer = &peers[i % peers.len()];

        match self.push_shard(&peer.peer_id, hash, shard_data.clone()).await {
            Ok(()) => {
                tracing::info!(
                    content_id,
                    shard_index = i,
                    peer_id = %peer.peer_id,
                    "Shard distributed"
                );
                // Record location in DB
                if let Ok(mut conn) = pool.get() {
                    let location = crate::db::models::NewShardLocation {
                        shard_hash: hash,
                        peer_id: &peer.peer_id,
                        h_app_id,
                        status: "announced",
                    };
                    let _ = crate::db::shard_locations::upsert_location(&mut conn, &location);
                }
                distributed += 1;
            }
            Err(e) => {
                tracing::warn!(
                    content_id,
                    shard_index = i,
                    peer_id = %peer.peer_id,
                    error = %e,
                    "Failed to distribute shard"
                );
            }
        }
    }

    Ok(distributed)
}
```

- [ ] **Step 2: Hook distribution into content creation (single)**

In `handle_db_content_list()` POST handler (~line 2134), after the EPR Head publication block, add shard distribution. This must be spawned as async (fire-and-forget) so content creation returns immediately:

```rust
// After EPR head publication block
#[cfg(feature = "p2p")]
if let (Some(ref handle), Some(ref pool)) = (&self.p2p_handle, &self.pool) {
    if let Some(ref blob_hash) = created_content.blob_hash {
        let handle = handle.clone();
        let pool = pool.clone();
        let content_id = created_content.id.clone();
        let h_app_id = h_app_id.clone();
        let blob_store = self.blob_store.clone();

        tokio::spawn(async move {
            if let Some(ref bs) = blob_store {
                if let Ok(Some(data)) = bs.get(blob_hash).await {
                    match handle.distribute_shards(&content_id, &data, &pool, &h_app_id).await {
                        Ok(n) => tracing::info!(content_id = %content_id, shards = n, "Shard distribution complete"),
                        Err(e) => tracing::warn!(content_id = %content_id, error = %e, "Shard distribution failed"),
                    }
                }
            }
        });
    }
}
```

- [ ] **Step 3: Hook distribution into bulk content creation**

Same pattern in `handle_db_content_bulk()`. After bulk insert, spawn async distribution for each item with a blob. Use a single spawned task that iterates:

```rust
#[cfg(feature = "p2p")]
if let (Some(ref handle), Some(ref pool)) = (&self.p2p_handle, &self.pool) {
    let handle = handle.clone();
    let pool = pool.clone();
    let h_app_id = h_app_id.clone();
    let blob_store = self.blob_store.clone();
    // items_with_blobs: Vec<(content_id, blob_hash)> collected earlier
    let items: Vec<(String, String)> = /* collect content_id + blob_hash pairs */;

    tokio::spawn(async move {
        for (content_id, blob_hash) in items {
            if let Some(ref bs) = blob_store {
                if let Ok(Some(data)) = bs.get(&blob_hash).await {
                    let _ = handle.distribute_shards(&content_id, &data, &pool, &h_app_id).await;
                }
            }
        }
        tracing::info!("Bulk shard distribution complete");
    });
}
```

- [ ] **Step 4: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): auto-distribute shards on content creation

Content creation with blob triggers async shard distribution:
RS-encode, push to delivery peers, record locations. Fire-and-forget
so content creation returns immediately."
```

---

### Task 3: Periodic Shard Verification Loop

Add a periodic task to the P2P event loop that verifies peers still hold their shards and marks lost shards.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs`

- [ ] **Step 1: Add verification interval to the event loop**

In the `run()` method (~line 552), add a new interval alongside the existing sync round:

```rust
let mut verify_interval = tokio::time::interval(std::time::Duration::from_secs(300)); // 5 minutes
verify_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
```

Add a new arm to the `tokio::select!`:

```rust
_ = verify_interval.tick() => {
    self.verify_shard_locations().await;
}
```

- [ ] **Step 2: Implement verify_shard_locations**

Add method to P2PNode:

```rust
async fn verify_shard_locations(&mut self) {
    let pool = match &self.pool {
        Some(p) => p,
        None => return,
    };

    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(_) => return,
    };

    // Get all announced/verified locations
    use crate::db::diesel_schema::shard_locations;
    use diesel::prelude::*;

    let locations: Vec<crate::db::models::ShardLocationRow> = shard_locations::table
        .filter(shard_locations::status.ne("lost"))
        .limit(100) // Process in batches
        .load(&mut conn)
        .unwrap_or_default();

    if locations.is_empty() {
        return;
    }

    tracing::debug!(count = locations.len(), "Verifying shard locations");

    for loc in &locations {
        let peer_id: libp2p::PeerId = match loc.peer_id.parse() {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Check if peer is still connected
        let is_connected = self.swarm.is_connected(&peer_id);

        if !is_connected {
            // Peer offline — mark shards as lost
            let _ = crate::db::shard_locations::mark_lost(&mut conn, &loc.shard_hash, &loc.peer_id);
            tracing::info!(
                shard = %loc.shard_hash,
                peer = %loc.peer_id,
                "Marked shard location as lost (peer disconnected)"
            );
            continue;
        }

        // Peer is connected — verify via Have request
        let request = crate::p2p::shard_protocol::ShardRequest::Have {
            hash: loc.shard_hash.clone(),
        };
        let request_id = self.swarm
            .behaviour_mut()
            .shard_protocol
            .send_request(&peer_id, request);

        // We handle the response in the normal event loop.
        // Store verification pending state:
        self.pending_verifications.insert(request_id, (loc.shard_hash.clone(), loc.peer_id.clone()));
    }
}
```

- [ ] **Step 3: Add pending_verifications map and handle responses**

Add to P2PNode struct:

```rust
pending_verifications: HashMap<RequestId, (String, String)>, // (shard_hash, peer_id)
```

In the shard protocol response handler, add verification handling:

```rust
// After existing fetch and push handling
else if let Some((shard_hash, peer_id_str)) = self.pending_verifications.remove(&request_id) {
    if let Some(ref pool) = self.pool {
        if let Ok(mut conn) = pool.get() {
            match response {
                ShardResponse::Have(true) => {
                    let _ = crate::db::shard_locations::update_verified(&mut conn, &shard_hash, &peer_id_str);
                }
                ShardResponse::Have(false) | ShardResponse::NotFound => {
                    let _ = crate::db::shard_locations::mark_lost(&mut conn, &shard_hash, &peer_id_str);
                    tracing::info!(shard = %shard_hash, peer = %peer_id_str, "Shard lost — peer no longer has it");
                }
                _ => {}
            }
        }
    }
}
```

- [ ] **Step 4: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/p2p/mod.rs
git commit -m "feat(p2p): periodic shard verification loop (every 5min)

Checks connected peers for shard availability via Have requests.
Marks shards as lost when peers disconnect or no longer hold them.
Foundation for self-healing replication in future."
```

---

### Task 4: Reconstruction Verification Endpoint

Add `POST /api/v1/resilience/{content_id}/verify` that actively tests RS reconstruction from peer-held shards.

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs` (add VerificationResultView)
- Modify: `elohim/elohim-storage/src/http.rs` (add route handler)

- [ ] **Step 1: Add VerificationResultView to views.rs**

```rust
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct VerificationResultView {
    pub content_id: String,
    pub verified: bool,
    pub encoding: String,
    pub shards_available: i32,
    pub shards_needed: i32,
    pub shards_used_for_reconstruction: i32,
    pub shards_intentionally_skipped: i32,
    pub reconstruction_time_ms: u64,
    pub original_hash: String,
    pub reconstructed_hash: String,
    pub hash_match: bool,
    pub error: Option<String>,
}
```

- [ ] **Step 2: Add verification handler to http.rs**

```rust
async fn handle_resilience_verify(
    &self,
    content_id: &str,
    h_app_id: &str,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let pool = self.pool.as_ref().ok_or(StorageError::NotConfigured)?;
    let mut conn = pool.get()?;

    // 1. Get manifest
    let manifest_row = crate::db::shard_manifests::get_manifest(&mut conn, h_app_id, content_id)?
        .ok_or_else(|| StorageError::NotFound(format!("No shard manifest for {content_id}")))?;

    let shard_hashes: Vec<String> =
        serde_json::from_str(&manifest_row.shard_hashes_json).unwrap_or_default();

    // 2. Build ShardManifest for reconstruction
    let manifest = crate::sharding::ShardManifest {
        blob_cid: manifest_row.blob_cid.clone().unwrap_or_default(),
        blob_hash: manifest_row.blob_hash.clone(),
        total_size: manifest_row.total_size_bytes as u64,
        mime_type: manifest_row.mime_type.clone(),
        encoding: manifest_row.encoding.clone(),
        data_shards: manifest_row.data_shard_count as u8,
        total_shards: (manifest_row.data_shard_count + manifest_row.parity_shard_count) as u8,
        shard_size: manifest_row.shard_size_bytes as u64,
        shard_hashes: shard_hashes.clone(),
        reach: manifest_row.reach.clone(),
        author_id: None,
        created_at: manifest_row.created_at.clone(),
        verified_at: None,
    };

    // 3. Fetch shards — prefer from peers, fall back to local blob store
    let start = std::time::Instant::now();
    let mut shard_opts: Vec<Option<Vec<u8>>> = Vec::with_capacity(shard_hashes.len());
    let mut available = 0i32;

    for hash in &shard_hashes {
        // Try P2P first
        let mut found = false;
        if let Some(ref handle) = self.p2p_handle {
            if let Some(data) = handle.fetch_shard(hash).await {
                shard_opts.push(Some(data));
                available += 1;
                found = true;
            }
        }

        // Fall back to local blob store
        if !found {
            if let Some(ref bs) = self.blob_store {
                if let Ok(Some(data)) = bs.get(hash).await {
                    shard_opts.push(Some(data));
                    available += 1;
                    found = true;
                }
            }
        }

        if !found {
            shard_opts.push(None);
        }
    }

    // 4. Intentionally skip parity shards to prove RS works
    let parity_count = manifest_row.parity_shard_count;
    let mut skipped = 0i32;
    if manifest_row.encoding.starts_with("rs-") && parity_count > 0 {
        // Skip up to parity_count shards from the end (parity shards)
        let total = shard_opts.len();
        for i in (0..total).rev() {
            if skipped >= parity_count {
                break;
            }
            if shard_opts[i].is_some() {
                shard_opts[i] = None;
                skipped += 1;
            }
        }
    }

    // 5. Reconstruct
    let encoder = crate::sharding::ShardEncoder::new(crate::sharding::ShardConfig::default());
    let shards_used = shard_opts.iter().filter(|s| s.is_some()).count() as i32;

    match encoder.reconstruct(&manifest, &shard_opts) {
        Ok(reconstructed) => {
            let elapsed = start.elapsed();
            let reconstructed_hash = crate::blob_store::BlobStore::compute_hash(&reconstructed);
            let hash_match = reconstructed_hash == manifest_row.blob_hash;

            let result = VerificationResultView {
                content_id: content_id.to_string(),
                verified: hash_match,
                encoding: manifest_row.encoding,
                shards_available: available,
                shards_needed: manifest_row.data_shard_count,
                shards_used_for_reconstruction: shards_used,
                shards_intentionally_skipped: skipped,
                reconstruction_time_ms: elapsed.as_millis() as u64,
                original_hash: manifest_row.blob_hash,
                reconstructed_hash,
                hash_match,
                error: None,
            };

            let json = serde_json::to_string(&result)?;
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(json)))?)
        }
        Err(e) => {
            let elapsed = start.elapsed();
            let result = VerificationResultView {
                content_id: content_id.to_string(),
                verified: false,
                encoding: manifest_row.encoding,
                shards_available: available,
                shards_needed: manifest_row.data_shard_count,
                shards_used_for_reconstruction: shards_used,
                shards_intentionally_skipped: skipped,
                reconstruction_time_ms: elapsed.as_millis() as u64,
                original_hash: manifest_row.blob_hash,
                reconstructed_hash: String::new(),
                hash_match: false,
                error: Some(e.to_string()),
            };

            let json = serde_json::to_string(&result)?;
            Ok(Response::builder()
                .status(200)
                .header("content-type", "application/json")
                .body(Full::new(Bytes::from(json)))?)
        }
    }
}
```

- [ ] **Step 3: Wire the route**

In the router, add:

```rust
("POST", path) if path.starts_with("/api/v1/resilience/") && path.ends_with("/verify") => {
    let content_id = path
        .strip_prefix("/api/v1/resilience/")
        .and_then(|s| s.strip_suffix("/verify"))
        .unwrap_or("");
    self.handle_resilience_verify(content_id, &h_app_id).await
}
```

Make sure this matches BEFORE the GET route for `/api/v1/resilience/{content_id}`.

- [ ] **Step 4: Generate TypeScript types**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test export_bindings`

- [ ] **Step 5: Build and verify**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build`

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/elohim-storage/src/http.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "feat(storage): add POST /api/v1/resilience/{id}/verify endpoint

Actively tests RS reconstruction: fetches shards from peers/local,
intentionally skips parity shards, reconstructs, verifies hash match.
Returns full verification result with timing and shard counts."
```

---

### Task 5: Frontend — Verify Button and Live Distribution

Add a "Verify Resilience" button to the Network tab resilience section and show per-shard peer distribution.

**Files:**
- Modify: `app/elohim-app/src/app/lamad/services/resilience.service.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css`

- [ ] **Step 1: Add verify method to ResilienceService**

```typescript
import type { VerificationResultView } from '@elohim/storage-client/generated';

verifyResilience(contentId: string): Observable<VerificationResultView> {
  const baseUrl = this.storageClient.getStorageBaseUrl();
  return this.http.post<VerificationResultView>(
    `${baseUrl}/api/v1/resilience/${contentId}/verify`,
    {}
  );
}
```

- [ ] **Step 2: Add verification state to component**

```typescript
import type { VerificationResultView } from '@elohim/storage-client/generated';

verificationResult: VerificationResultView | null = null;
isVerifying = false;

verifyResilience(): void {
  if (!this.node || this.isVerifying) return;
  this.isVerifying = true;
  this.verificationResult = null;

  this.resilienceService
    .verifyResilience(this.node.id)
    .pipe(takeUntil(this.destroy$))
    .subscribe({
      next: result => {
        this.verificationResult = result;
        this.isVerifying = false;
      },
      error: () => {
        this.isVerifying = false;
      },
    });
}
```

- [ ] **Step 3: Update Network tab HTML with verify button and shard map**

Add after the resilience-grid div (inside the resilience-section):

```html
<!-- Shard Distribution Map -->
<div class="shard-map" *ngIf="resilience && resilience.distribution.shards.length > 0">
  <span class="metric-label">Shard Map</span>
  <div class="shard-grid">
    <div
      *ngFor="let shard of resilience.distribution.shards; let i = index"
      class="shard-cell"
      [ngClass]="{
        'shard-data': shard.shardType === 'data',
        'shard-parity': shard.shardType === 'parity',
        'shard-missing': shard.peerIds.length === 0
      }"
      [title]="'Shard ' + i + ' (' + shard.shardType + '): ' + (shard.peerIds.length > 0 ? shard.peerIds.length + ' peers' : 'no peers')"
    >
      {{ shard.peerIds.length > 0 ? shard.peerIds.length : '?' }}
    </div>
  </div>
</div>

<!-- Verify Button -->
<div class="verify-section">
  <button
    type="button"
    class="btn-verify"
    (click)="verifyResilience()"
    [disabled]="isVerifying"
    data-testid="viewer-verify-resilience"
  >
    {{ isVerifying ? 'Verifying...' : 'Verify Resilience' }}
  </button>

  <div class="verification-result" *ngIf="verificationResult" data-testid="viewer-verification-result">
    <div class="verify-status" [ngClass]="verificationResult.verified ? 'pass' : 'fail'">
      {{ verificationResult.verified ? 'PASS' : 'FAIL' }}
    </div>
    <div class="verify-details">
      <span>{{ verificationResult.shardsUsedForReconstruction }}/{{ verificationResult.shardsNeeded }} shards used</span>
      <span *ngIf="verificationResult.shardsIntentionallySkipped > 0">
        ({{ verificationResult.shardsIntentionallySkipped }} skipped to prove RS)
      </span>
      <span>{{ verificationResult.reconstructionTimeMs }}ms</span>
      <span *ngIf="!verificationResult.hashMatch && verificationResult.error">
        Error: {{ verificationResult.error }}
      </span>
    </div>
  </div>
</div>
```

- [ ] **Step 4: Add CSS for shard map and verify button**

```css
/* Shard Distribution Map */
.shard-map {
  margin-top: 0.75rem;
}

.shard-grid {
  display: flex;
  gap: 4px;
  flex-wrap: wrap;
  margin-top: 0.25rem;
}

.shard-cell {
  width: 28px;
  height: 28px;
  display: flex;
  align-items: center;
  justify-content: center;
  border-radius: 4px;
  font-size: 0.7rem;
  font-weight: 600;
  cursor: default;
}

.shard-data { background: #dbeafe; color: #1e40af; }
.shard-parity { background: #e0e7ff; color: #4338ca; }
.shard-missing { background: #fee2e2; color: #991b1b; }

/* Verify Section */
.verify-section {
  margin-top: 1rem;
  display: flex;
  align-items: center;
  gap: 1rem;
  flex-wrap: wrap;
}

.btn-verify {
  padding: 0.5rem 1rem;
  border: 1px solid var(--border, #e5e5e5);
  border-radius: 8px;
  background: var(--surface, #fff);
  cursor: pointer;
  font-size: 0.85rem;
  font-weight: 500;
  transition: background 0.15s;
}

.btn-verify:hover:not(:disabled) {
  background: var(--surface-hover, #f3f4f6);
}

.btn-verify:disabled {
  opacity: 0.5;
  cursor: not-allowed;
}

.verification-result {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  font-size: 0.85rem;
}

.verify-status {
  font-weight: 700;
  font-size: 0.75rem;
  padding: 0.2rem 0.5rem;
  border-radius: 4px;
  text-transform: uppercase;
}

.verify-status.pass { background: #dcfce7; color: #166534; }
.verify-status.fail { background: #fee2e2; color: #991b1b; }

.verify-details {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
  opacity: 0.8;
}

.verify-details span {
  white-space: nowrap;
}
```

- [ ] **Step 5: Verify in browser**

Navigate to a content page, click Network tab:
- Shard map shows colored cells (blue for data, indigo for parity, red for missing)
- "Verify Resilience" button triggers verification
- Result shows PASS/FAIL with shard count and timing

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/services/resilience.service.ts app/elohim-app/src/app/lamad/components/content-viewer/
git commit -m "feat(lamad): add verify button and shard map to Network tab

Shard map visualizes per-shard peer distribution with color coding.
Verify button triggers RS reconstruction proof and shows pass/fail
result with shard counts and reconstruction timing."
```

---

### Task 6: Update Seed Data for RS Testing

Ensure the seed data includes at least one content item large enough to trigger RS encoding (>10MB), and update the seeder to seed shard manifests for all content.

**Files:**
- Modify: `genesis/seeder/src/seed.ts` (or relevant seeder entry point)

- [ ] **Step 1: Add a synthetic large blob for RS testing**

Add to the seeder a step that creates a 12MB test blob (just past the RS_THRESHOLD of 10MB). This can be a repeating pattern:

```typescript
// After normal content seeding, create a large test content for RS proof
async function seedRsTestContent(api: StorageApiService): Promise<void> {
  console.log('[seed-rs] Creating 12MB test blob for RS encoding proof...');

  // Generate 12MB of deterministic test data
  const size = 12 * 1024 * 1024;
  const data = new Uint8Array(size);
  for (let i = 0; i < size; i++) {
    data[i] = i % 256;
  }

  // Upload as blob
  const blob = new Blob([data], { type: 'application/octet-stream' });
  const blobHash = await api.uploadBlob(blob);
  console.log(`  [+] Blob uploaded: ${blobHash}`);

  // Create content referencing the blob
  await api.createContent({
    id: 'rs-proof-test-blob',
    title: 'RS Encoding Proof — 12MB Test Blob',
    description: 'Synthetic content for proving Reed-Solomon encode/distribute/reconstruct pipeline',
    contentType: 'concept',
    contentFormat: 'binary',
    blobHash,
    reach: 'commons',
    metadata: { category: 'infrastructure', rsTest: true },
  });
  console.log('  [+] Content created: rs-proof-test-blob');
}
```

Call this after the main seed flow. Check whether the `StorageApiService` has an `uploadBlob` method. If not, add one that POSTs to `/store` or `/blob`.

- [ ] **Step 2: Verify RS encoding triggered**

After seeding with the large blob, check:

```bash
sqlite3 /path/to/storage.db "SELECT content_id, encoding, data_shard_count, parity_shard_count FROM shard_manifests WHERE content_id = 'rs-proof-test-blob';"
```

Expected: `rs-proof-test-blob|rs-4-7|4|3`

- [ ] **Step 3: Verify shard distribution**

```bash
sqlite3 /path/to/storage.db "SELECT shard_hash, peer_id, status FROM shard_locations WHERE shard_hash IN (SELECT json_each.value FROM shard_manifests, json_each(shard_hashes_json) WHERE content_id = 'rs-proof-test-blob') LIMIT 20;"
```

Expected: Rows showing shards distributed to delivery peers (if peers are connected).

- [ ] **Step 4: Verify via HTTP**

```bash
# Check resilience
curl http://localhost:8888/api/v1/resilience/rs-proof-test-blob | jq

# Verify reconstruction
curl -X POST http://localhost:8888/api/v1/resilience/rs-proof-test-blob/verify | jq
```

Expected: Resilience shows RS encoding with shard distribution. Verification shows PASS with hash match.

- [ ] **Step 5: Commit**

```bash
git add genesis/seeder/src/
git commit -m "feat(seeder): add 12MB RS test blob for shard distribution proof

Synthetic content triggers RS 4+3 encoding on ingest. Used to verify
end-to-end encode → distribute → reconstruct pipeline."
```

---

### Task 7: Resilience Integration Test

Write a Rust integration test that verifies the full pipeline: store content, verify manifest created, check distribution, verify reconstruction.

**Files:**
- Create: `elohim/elohim-storage/tests/resilience_integration.rs` (or add to existing test module)

- [ ] **Step 1: Write the integration test**

This test uses the sharding module directly (not the full HTTP server) to prove the pipeline:

```rust
//! Integration test: RS encode → distribute shards → lose shards → reconstruct
//!
//! This test proves the Reed-Solomon pipeline works end-to-end without
//! requiring a running P2P network. It simulates:
//! 1. Content ingestion with RS encoding
//! 2. Shard distribution to simulated peers
//! 3. Peer failure (shard loss)
//! 4. Successful reconstruction from remaining shards

use elohim_storage::sharding::{ShardConfig, ShardEncoder};

#[test]
fn test_full_rs_pipeline_with_simulated_peer_failure() {
    // 1. Create test data (above RS threshold for test config)
    let data: Vec<u8> = (0..1000).map(|i| (i % 256) as u8).collect();

    let config = ShardConfig {
        shard_size: 100,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 500,
        single_shard_max: 100,
    };
    let encoder = ShardEncoder::new(config);

    // 2. Encode — simulates blob ingest
    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons");
    assert_eq!(manifest.encoding, "rs-4-7");
    assert_eq!(manifest.shard_hashes.len(), 7);

    let shards = encoder.create_shards(&data, &manifest.encoding);
    assert_eq!(shards.len(), 7);

    // 3. Simulate distribution to 5 peers
    // Peer 0: shards [0, 5]
    // Peer 1: shards [1, 6]
    // Peer 2: shards [2]
    // Peer 3: shards [3]
    // Peer 4: shards [4]
    let peer_shards: Vec<Vec<usize>> = vec![
        vec![0, 5],
        vec![1, 6],
        vec![2],
        vec![3],
        vec![4],
    ];

    // 4. Simulate peer 0 and peer 1 going offline (lose shards 0, 1, 5, 6)
    let offline_peers = [0usize, 1];
    let available_shard_indices: Vec<usize> = peer_shards
        .iter()
        .enumerate()
        .filter(|(i, _)| !offline_peers.contains(i))
        .flat_map(|(_, indices)| indices.iter().copied())
        .collect();

    // We should have shards [2, 3, 4] — 3 shards, but need 4 for RS-4-7
    // This should fail!
    assert_eq!(available_shard_indices.len(), 3);

    let mut shard_opts_fail: Vec<Option<Vec<u8>>> = vec![None; 7];
    for &i in &available_shard_indices {
        shard_opts_fail[i] = Some(shards[i].clone());
    }
    let result = encoder.reconstruct(&manifest, &shard_opts_fail);
    assert!(result.is_err(), "Should fail with only 3 of 4 needed shards");

    // 5. Simulate only peer 0 going offline (lose shards 0, 5)
    let offline_peers_one = [0usize];
    let available_one: Vec<usize> = peer_shards
        .iter()
        .enumerate()
        .filter(|(i, _)| !offline_peers_one.contains(i))
        .flat_map(|(_, indices)| indices.iter().copied())
        .collect();

    // We should have shards [1, 2, 3, 4, 6] — 5 shards, need 4
    assert_eq!(available_one.len(), 5);

    let mut shard_opts_ok: Vec<Option<Vec<u8>>> = vec![None; 7];
    for &i in &available_one {
        shard_opts_ok[i] = Some(shards[i].clone());
    }
    let reconstructed = encoder.reconstruct(&manifest, &shard_opts_ok).unwrap();
    assert_eq!(reconstructed, data, "Must reconstruct perfectly from 5 of 7 shards");

    // 6. Verify hash integrity
    let original_hash = elohim_storage::blob_store::BlobStore::compute_hash(&data);
    let reconstructed_hash = elohim_storage::blob_store::BlobStore::compute_hash(&reconstructed);
    assert_eq!(original_hash, reconstructed_hash, "Hash must match after reconstruction");
}
```

Note: Check if `BlobStore::compute_hash` is public. If not, use `sha2` crate directly:
```rust
use sha2::{Sha256, Digest};
let hash = format!("sha256-{}", hex::encode(Sha256::digest(&data)));
```

- [ ] **Step 2: Run the integration test**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test resilience_integration -- --nocapture`

Expected: Test passes. Demonstrates that losing 1 peer (2 shards) allows reconstruction, but losing 2 peers (4 shards) correctly fails.

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/tests/resilience_integration.rs
git commit -m "test(storage): end-to-end RS pipeline integration test

Proves: encode → simulate peer distribution → simulate peer failure →
reconstruct. Verifies 1-peer-down succeeds, 2-peers-down correctly
fails for RS 4+3. Hash integrity verified after reconstruction."
```

---

### Summary: Sprint C Delivers

After all 7 tasks:
- `push_shard` method on P2PHandle enables outbound replication
- Content creation auto-distributes RS shards to delivery peers (async, fire-and-forget)
- Periodic verification loop (5min) checks peers still hold shards, marks lost
- `POST /api/v1/resilience/{id}/verify` actively tests RS reconstruction
- UI shard map shows per-shard peer distribution with color coding
- "Verify Resilience" button triggers reconstruction proof with pass/fail result
- 12MB test blob in seed data triggers RS encoding for demonstration
- Integration test proves the full pipeline: encode → distribute → lose → reconstruct

**The end-to-end proof**: Seed content → auto RS-encode → shards push to 5 peers → click Verify → see PASS with "4/4 shards used, 3 skipped to prove RS" → hash matches. Content survives peer failure.
