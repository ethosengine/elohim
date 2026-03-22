# EPR Protocol Layer Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add startup EPR Head publication for existing content, server-side reach authorization on EPR resolution, and recognition event logging on P2P content delivery.

**Architecture:** Startup publish scans the content table once and publishes EPR Heads with steward info to Kademlia. Authorization adds `agent_pubkey` to `EprRequest::Resolve` and gates serving based on content reach + human relationships. Recognition logs an REA `EconomicEvent` on the receiving peer after successful P2P content delivery.

**Tech Stack:** Rust (libp2p, diesel ORM, tokio async), elohim-storage P2P module, REA economic events

---

### Task 1: Add CONTENT_DELIVERY Event Type

**Files:**
- Modify: `elohim/elohim-storage/src/db/models.rs:930-962`

**Step 1: Add the constant**

In the `lamad_event_types` module (line 930), add after `MASTERY_ADVANCE`:

```rust
pub const CONTENT_DELIVERY: &str = "content-delivery";
```

**Step 2: Add to ALL array**

Update the `ALL` array (line 945) — change the size from 11 to 12 and add `CONTENT_DELIVERY`:

```rust
pub const ALL: [&str; 12] = [
    CONTENT_VIEW,
    PATH_STEP_COMPLETE,
    AFFINITY_MARK,
    PATH_COMPLETE,
    ATTESTATION_GRANT,
    STEWARDSHIP_BEGIN,
    PRESENCE_CLAIM,
    RECOGNITION_TRANSFER,
    AFFINITY_TRANSFER,
    CITATION,
    MASTERY_ADVANCE,
    CONTENT_DELIVERY,
];
```

**Step 3: Build**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Step 4: Commit**

```
feat(rea): add CONTENT_DELIVERY lamad event type for P2P recognition
```

---

### Task 2: Populate Stewards in EPR Heads

Currently `resolve_epr_head_locally` sets `stewards: vec![], allocations: vec![]`. This needs steward data for authorization and recognition.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:1248-1295`

**Step 1: Update `resolve_epr_head_locally` to query stewardship allocations**

Replace the shefa context construction (lines 1266-1269) with a stewardship lookup:

```rust
// Query stewardship allocations for this content
let shefa = {
    let app_ctx_shefa = crate::db::AppContext::default_lamad();
    match crate::db::stewardship_allocations::get_allocations_for_content(
        &mut conn, &app_ctx_shefa, &content.id,
    ) {
        Ok(allocations) if !allocations.is_empty() => {
            crate::epr_codec::EprShefaContext {
                stewards: allocations
                    .iter()
                    .map(|a| a.steward_presence_id.clone())
                    .collect(),
                allocations: allocations
                    .iter()
                    .map(|a| a.allocation_ratio as f64)
                    .collect(),
            }
        }
        _ => crate::epr_codec::EprShefaContext {
            stewards: vec![],
            allocations: vec![],
        },
    }
};
```

Then use `shefa` in the EprHead construction instead of the inline empty vecs.

**Step 2: Build**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Step 3: Commit**

```
feat(p2p): populate steward presence IDs and ratios in EPR Heads
```

---

### Task 3: Add `agent_pubkey` to EprRequest::Resolve and AccessDenied to EprResponse

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/epr_protocol.rs:34-63`

**Step 1: Add `agent_pubkey` to Resolve**

```rust
pub enum EprRequest {
    /// Resolve an EPR Head by content ID
    Resolve {
        id: String,
        agent_pubkey: Option<String>,
    },
    // ... rest unchanged
}
```

**Step 2: Add `AccessDenied` to EprResponse**

After `NotFound`:

```rust
pub enum EprResponse {
    // ... existing variants ...
    /// Content not found
    NotFound,
    /// Access denied — reach gate failed
    AccessDenied {
        required_reach: String,
        reason: String,
    },
    /// Error
    Error(String),
}
```

**Step 3: Update tests**

Update `test_epr_request_resolve_roundtrip` to include `agent_pubkey`:

```rust
#[test]
fn test_epr_request_resolve_roundtrip() {
    let request = EprRequest::Resolve {
        id: "fct-module-01-church-dilemma".to_string(),
        agent_pubkey: Some("uhCAk_test_agent".to_string()),
    };
    let bytes = rmp_serde::to_vec(&request).unwrap();
    let decoded: EprRequest = rmp_serde::from_slice(&bytes).unwrap();
    match decoded {
        EprRequest::Resolve { id, agent_pubkey } => {
            assert_eq!(id, "fct-module-01-church-dilemma");
            assert_eq!(agent_pubkey.unwrap(), "uhCAk_test_agent");
        }
        _ => panic!("Wrong variant"),
    }
}
```

Add a test for AccessDenied roundtrip:

```rust
#[test]
fn test_epr_response_access_denied_roundtrip() {
    let response = EprResponse::AccessDenied {
        required_reach: "trusted".to_string(),
        reason: "No relationship with content steward".to_string(),
    };
    let bytes = rmp_serde::to_vec(&response).unwrap();
    let decoded: EprResponse = rmp_serde::from_slice(&bytes).unwrap();
    match decoded {
        EprResponse::AccessDenied { required_reach, reason } => {
            assert_eq!(required_reach, "trusted");
            assert_eq!(reason, "No relationship with content steward");
        }
        _ => panic!("Wrong variant"),
    }
}
```

**Step 4: Fix all compile errors from the Resolve variant change**

In `mod.rs`, everywhere `EprRequest::Resolve { id }` is pattern-matched, update to `EprRequest::Resolve { id, agent_pubkey }` (or `EprRequest::Resolve { id, .. }` where agent_pubkey is unused).

Key locations:
- `handle_epr_request` match arm (line ~1300)
- `handle_command` for `ResolveEpr` where the request is constructed (line ~466)

**Step 5: Build and run tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test epr_protocol`

**Step 6: Commit**

```
feat(p2p): add agent_pubkey to EprRequest::Resolve and AccessDenied response
```

---

### Task 4: Store Agent Pubkey on P2PHandle

The requesting peer needs to send its agent pubkey with every resolve request. Store it on `P2PHandle` so `resolve_epr` can include it automatically.

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs:192-195` (P2PHandle struct)
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (handle() method)

**Step 1: Add `agent_pubkey` field to P2PHandle**

```rust
#[derive(Clone)]
pub struct P2PHandle {
    status_rx: tokio::sync::watch::Receiver<P2PStatusInfo>,
    command_tx: mpsc::Sender<P2PCommand>,
    agent_pubkey: String,
}
```

**Step 2: Update `handle()` to pass it**

In `P2PNode::handle()`:

```rust
pub fn handle(&self) -> P2PHandle {
    P2PHandle {
        status_rx: self.status_tx.subscribe(),
        command_tx: self.command_tx.clone(),
        agent_pubkey: self.identity.agent_pubkey().to_string(),
    }
}
```

**Step 3: Update `resolve_epr` to include agent_pubkey in the command**

Add `agent_pubkey` to `P2PCommand::ResolveEpr`:

```rust
ResolveEpr {
    id: String,
    agent_pubkey: String,
    reply: oneshot::Sender<Option<Vec<u8>>>,
},
```

Update `resolve_epr()` to send it:

```rust
pub async fn resolve_epr(&self, id: &str) -> Option<Vec<u8>> {
    let (reply_tx, reply_rx) = oneshot::channel();
    if self
        .command_tx
        .send(P2PCommand::ResolveEpr {
            id: id.to_string(),
            agent_pubkey: self.agent_pubkey.clone(),
            reply: reply_tx,
        })
        .await
        .is_err()
    {
        return None;
    }
    // ... timeout unchanged
}
```

**Step 4: Update `handle_command` to include agent_pubkey in the EprRequest**

In the `ResolveEpr` match arm:

```rust
P2PCommand::ResolveEpr { id, agent_pubkey, reply } => {
    // ... existing local Kademlia check ...
    // When sending to peer:
    let req_id = swarm
        .behaviour_mut()
        .epr_protocol
        .send_request(peer_id, EprRequest::Resolve {
            id: id.clone(),
            agent_pubkey: Some(agent_pubkey.clone()),
        });
    // ...
}
```

**Step 5: Build**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check`

**Step 6: Commit**

```
feat(p2p): thread agent_pubkey through P2PHandle into EprRequest::Resolve
```

---

### Task 5: Implement Server-Side Reach Authorization

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (handle_epr_request)

**Step 1: Add `check_reach_authorization` helper method on P2PNode**

Before `handle_epr_request`:

```rust
/// Check if a requesting agent is authorized to access content at the given reach level.
/// Returns Ok(()) if authorized, Err(reason) if denied.
fn check_reach_authorization(
    &self,
    reach: &str,
    agent_pubkey: Option<&str>,
) -> Result<(), String> {
    match reach {
        "commons" | "public" => Ok(()),
        _ => {
            // Restricted content requires agent identity
            let agent_key = agent_pubkey
                .ok_or_else(|| "Agent identity required for restricted content".to_string())?;

            let pool = self.db_pool.as_ref()
                .ok_or_else(|| "Database not available for authorization".to_string())?;
            let mut conn = pool.get()
                .map_err(|e| format!("DB connection failed: {}", e))?;

            let app_ctx = crate::db::AppContext::default_lamad();

            // Map agent_pubkey -> human
            let human = crate::db::humans::get_human_by_agent_key(&mut conn, agent_key)
                .map_err(|e| format!("Agent lookup failed: {}", e))?
                .ok_or_else(|| "Unknown agent — no human identity found".to_string())?;

            // For "community" and below, check relationship with any steward
            // Future: differentiate by intimacy level per reach tier
            let relationships = crate::db::human_relationships::get_relationship_between(
                &mut conn, &app_ctx, &human.id, "%", None,
            );

            match relationships {
                Ok(rels) if !rels.is_empty() => Ok(()),
                _ => Err(format!(
                    "No qualifying relationship for reach '{}'",
                    reach
                )),
            }
        }
    }
}
```

**Step 2: Wire into `handle_epr_request(Resolve)`**

Update the Resolve handler to check authorization before serving:

```rust
EprRequest::Resolve { id, agent_pubkey } => {
    debug!(id = %id, "Handling EPR Resolve request");

    // First get content to check reach
    if let Some(ref pool) = self.db_pool {
        if let Ok(mut conn) = pool.get() {
            let app_ctx = crate::db::AppContext::default_lamad();
            if let Ok(Some(content_with_tags)) =
                crate::db::content_diesel::get_content_with_tags(&mut conn, &app_ctx, &id)
            {
                // Check reach authorization
                let reach = &content_with_tags.content.reach;
                if let Err(reason) = self.check_reach_authorization(
                    reach,
                    agent_pubkey.as_deref(),
                ) {
                    info!(id = %id, reach = %reach, reason = %reason, "EPR access denied");
                    return EprResponse::AccessDenied {
                        required_reach: reach.clone(),
                        reason,
                    };
                }
            }
        }
    }

    // Authorized — serve the EPR Head
    match self.resolve_epr_head_locally(&id) {
        Some(bytes) => {
            info!(id = %id, size = bytes.len(), "Serving EPR Head");
            EprResponse::Head(bytes)
        }
        None => EprResponse::NotFound,
    }
}
```

**Step 3: Build and run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`

**Step 4: Commit**

```
feat(p2p): server-side reach authorization on EPR Head resolution
```

---

### Task 6: Startup EPR Head Publication

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/mod.rs` (run() and new helper)

**Step 1: Add `initial_publish_done` flag to P2PNode**

Add field to struct:

```rust
/// Whether startup EPR Head publication has run
initial_publish_done: Arc<std::sync::atomic::AtomicBool>,
```

Initialize in `new()`:

```rust
initial_publish_done: Arc::new(std::sync::atomic::AtomicBool::new(false)),
```

**Step 2: Add `publish_all_epr_heads` method**

```rust
/// Publish EPR Heads for all existing content to Kademlia DHT.
/// Runs once on startup with adaptive rate limiting.
async fn publish_all_epr_heads(&self) {
    let pool = match self.db_pool.as_ref() {
        Some(p) => p,
        None => {
            info!("Skipping startup EPR publish — no DB pool");
            return;
        }
    };
    let mut conn = match pool.get() {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "Skipping startup EPR publish — DB connection failed");
            return;
        }
    };

    let app_ctx = crate::db::AppContext::default_lamad();
    let query = crate::db::content_diesel::ContentQuery {
        limit: 10000,
        ..Default::default()
    };
    let content_items = match crate::db::content_diesel::list_content(&mut conn, &app_ctx, &query) {
        Ok(items) => items,
        Err(e) => {
            warn!(error = %e, "Skipping startup EPR publish — content query failed");
            return;
        }
    };
    drop(conn); // Release connection before async work

    let total = content_items.len();
    if total == 0 {
        info!("No content to publish EPR Heads for");
        return;
    }

    info!(total = total, "Starting EPR Head publication for existing content");

    let mut published = 0u64;
    let mut failed = 0u64;
    let mut batch_delay = Duration::from_millis(1); // Start fast

    for item in &content_items {
        if let Some(head_bytes) = self.resolve_epr_head_locally(&item.content.id) {
            let key = RecordKey::new(&format!("epr:{}", item.content.id));
            let record = Record {
                key,
                value: head_bytes,
                publisher: Some(*self.identity.peer_id()),
                expires: None,
            };
            let mut swarm = self.swarm.write().await;
            match swarm
                .behaviour_mut()
                .kademlia
                .put_record(record, libp2p::kad::Quorum::One)
            {
                Ok(_) => {
                    published += 1;
                    // Adaptive: success reduces delay (floor 1ms)
                    batch_delay = Duration::from_millis(
                        (batch_delay.as_millis() as u64 / 2).max(1),
                    );
                }
                Err(e) => {
                    failed += 1;
                    debug!(id = %item.content.id, error = ?e, "Failed to publish EPR Head");
                    // Adaptive: failure increases delay (cap 500ms)
                    batch_delay = Duration::from_millis(
                        (batch_delay.as_millis() as u64 * 2).min(500),
                    );
                }
            }
            drop(swarm);
        }

        // Adaptive pacing — yield between publishes
        if batch_delay.as_millis() > 1 {
            tokio::time::sleep(batch_delay).await;
        } else {
            tokio::task::yield_now().await;
        }
    }

    info!(
        published = published,
        failed = failed,
        total = total,
        "Startup EPR Head publication complete"
    );
}
```

**Step 3: Trigger in `run()` on first status tick**

In the `run()` method, after the `status_interval.tick()` arm, add a check:

```rust
_ = status_interval.tick() => {
    drop(swarm);
    self.refresh_status().await;
    // One-time startup EPR Head publication
    if !self.initial_publish_done.load(std::sync::atomic::Ordering::Relaxed) {
        self.initial_publish_done.store(true, std::sync::atomic::Ordering::Relaxed);
        self.publish_all_epr_heads().await;
    }
}
```

**Step 4: Build and run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`

**Step 5: Commit**

```
feat(p2p): startup EPR Head publication with adaptive rate limiting
```

---

### Task 7: Recognition Event on P2P Content Delivery

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs:2203-2207`

**Step 1: Add recognition event after successful P2P persist**

In `handle_db_content_by_id`, after the "P2P content persisted to local SQLite" info log (line ~2205), add recognition logging:

```rust
Ok(content_with_tags) => {
    info!(id = %content_id, "P2P content persisted to local SQLite");

    // Log recognition event (fire-and-forget)
    if let Ok(mut econ_conn) = self.get_conn() {
        let primary_steward = head.shefa.stewards.first()
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());
        let econ_input = crate::db::economic_events::CreateEconomicEventInput {
            id: None,
            action: crate::db::models::rea_actions::DELIVER_SERVICE.to_string(),
            provider: primary_steward,
            receiver: content_id.to_string(),
            lamad_event_type: Some(
                crate::db::models::lamad_event_types::CONTENT_DELIVERY.to_string(),
            ),
            content_id: Some(content_id.to_string()),
            resource_quantity_value: Some(1.0),
            note: Some("P2P EPR resolution".to_string()),
            ..Default::default()
        };
        let econ_ctx = db::AppContext::default_lamad();
        if let Err(e) = crate::db::economic_events::record_event(
            &mut econ_conn, &econ_ctx, econ_input,
        ) {
            debug!(id = %content_id, error = %e, "Failed to log delivery recognition (non-fatal)");
        } else {
            debug!(id = %content_id, "Delivery recognition event recorded");
        }
    }

    let view = ContentView::from(content_with_tags);
    return Ok(response::ok(&view));
}
```

**Step 2: Verify `CreateEconomicEventInput` implements Default**

Check if it does. If not, construct all fields explicitly. The `record_event` function in `economic_events.rs` (line 264) validates action and lamad_event_type, so those must be correct.

**Step 3: Build and run clippy**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`

**Step 4: Commit**

```
feat(rea): log CONTENT_DELIVERY recognition event on P2P content resolution
```

---

### Task 8: Final Verification

**Step 1: Format**

Run: `cd elohim/elohim-storage && cargo fmt`

**Step 2: Clippy**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings`
Expected: clean

**Step 3: Run all EPR tests**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test epr`
Expected: all pass (including new AccessDenied roundtrip)

**Step 4: Run content diesel tests**

Run: `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test content_diesel`
Expected: all pass

**Step 5: Verify diff**

Run: `git diff --stat`
Expected files:
- `elohim/elohim-storage/src/p2p/epr_protocol.rs` — agent_pubkey, AccessDenied, tests
- `elohim/elohim-storage/src/p2p/mod.rs` — steward population, authorization, startup publish, agent_pubkey threading
- `elohim/elohim-storage/src/http.rs` — recognition event
- `elohim/elohim-storage/src/db/models.rs` — CONTENT_DELIVERY constant

---

## Execution Order

```
Task 1: CONTENT_DELIVERY event type          (standalone)
Task 2: Steward population in EPR Heads      (standalone)
Task 3: Protocol changes (agent_pubkey, AccessDenied)  (standalone)
Task 4: Agent pubkey on P2PHandle            (depends on 3)
Task 5: Server-side reach authorization      (depends on 2, 3, 4)
Task 6: Startup EPR Head publication         (depends on 2)
Task 7: Recognition on delivery              (depends on 1)
Task 8: Final verification                   (after all)
```

Tasks 1, 2, 3 are independent and can be parallelized.
