# Doorway Silent Failure Stabilization Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate every silent failure pattern in doorway-service so that broken features return errors instead of degraded data that looks correct.

**Architecture:** Replace warn-and-fallback patterns with fail-loud errors. Stubs return 501 Not Implemented, not 200 with mock data. Discovery blocks readiness instead of racing with HTTP server startup. TypedAdminClient and ZomeCaller are the only conductor communication paths — no hand-rolled MessagePack.

**Tech Stack:** Rust, tokio, hyper, holochain_client (AdminWebsocket), dashmap, tokio::sync::watch

---

## File Map

| File | Changes |
|------|---------|
| `doorway/doorway-service/src/services/discovery.rs` | Remove phantom fallback, remove TODO stubs, add `DiscoveryState` watch channel |
| `doorway/doorway-service/src/main.rs` | Await discovery completion before serving, wire readiness gate |
| `doorway/doorway-service/src/server/http.rs` | Add `discovery_ready` field to AppState, gate conductor-dependent routes |
| `doorway/doorway-service/src/routes/zome_helpers.rs` | Remove wrong-role fallback from `get_agent_pub_key` |
| `doorway/doorway-service/src/routes/auth_routes.rs` | Replace recovery mock stubs with 501 |
| `doorway/doorway-service/src/routes/health.rs` | Add discovery status to readiness check |
| `doorway/doorway-service/src/conductor/chaperone.rs` | Propagate app port error instead of defaulting to 4445 |
| `doorway/doorway-service/src/services/storage_registration.rs` | Replace no-op with error log + return Err |
| `doorway/doorway-service/src/cache/delivery_relay.rs` | Already returns empty — acceptable since `enable_geo_routing` gates it. No change needed. |

---

### Task 1: Add discovery readiness gate to AppState

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (AppState struct)
- Modify: `doorway/doorway-service/src/services/discovery.rs` (spawn functions)
- Modify: `doorway/doorway-service/src/main.rs` (startup sequence)

This task adds a `tokio::sync::watch` channel that discovery writes to when it completes. The HTTP server starts immediately (health probes need it) but conductor-dependent routes check the watch value before proceeding.

- [ ] **Step 1: Add `discovery_ready` to AppState**

In `doorway/doorway-service/src/server/http.rs`, add to the `AppState` struct after `zome_configs`:

```rust
/// Discovery completion signal. Routes that need zome_configs wait on this.
/// `false` = discovery not yet complete, `true` = discovery succeeded and zome_configs populated.
pub discovery_ready: tokio::sync::watch::Receiver<bool>,
```

And in every `AppState` constructor (`new`, `new_with_mongo`, etc.), initialize it:

```rust
// In struct init — create a channel that starts as false
// The sender is passed to spawn_discovery_task; receiver stays in AppState
discovery_ready: tokio::sync::watch::channel(false).1,
```

Note: this is a placeholder — the real wiring happens in step 3 where main.rs creates the channel and passes the sender to discovery.

- [ ] **Step 2: Run `cargo check` to verify the struct change compiles**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: compilation errors in constructors (we'll fix in step 3)

- [ ] **Step 3: Wire the watch channel through main.rs**

In `doorway/doorway-service/src/main.rs`, at the discovery spawn point (~line 448-466), create the channel and pass the sender:

```rust
// Before building Arc<AppState>, create the watch channel
let (discovery_tx, discovery_rx) = tokio::sync::watch::channel(false);
state.discovery_ready = discovery_rx;

// ... later, after Arc::new(state):
let _discovery_handle = spawn_discovery_task_with_signal(
    discovery_config,
    Arc::clone(&state.zome_configs),
    Arc::clone(import_config_store),
    discovery_tx,
);
```

- [ ] **Step 4: Add `spawn_discovery_task_with_signal` to discovery.rs**

In `doorway/doorway-service/src/services/discovery.rs`, add a new spawn function that signals on completion:

```rust
/// Spawn discovery as a background task with completion signal.
///
/// The `ready_tx` channel is set to `true` when discovery completes successfully
/// (all cells discovered and stored in zome_configs). Routes can wait on the
/// corresponding receiver before attempting conductor operations.
pub fn spawn_discovery_task_with_signal(
    config: DiscoveryConfig,
    zome_configs: Arc<DashMap<String, ZomeCallConfig>>,
    import_config_store: Arc<ImportConfigStore>,
    ready_tx: tokio::sync::watch::Sender<bool>,
) -> tokio::task::JoinHandle<DiscoveryResult> {
    tokio::spawn(async move {
        // Wait a bit for conductor to be ready
        tokio::time::sleep(Duration::from_secs(2)).await;

        let service = DiscoveryService::new(config, zome_configs, import_config_store);
        let result = service.discover().await;

        if result.cells_discovered > 0 && result.errors.is_empty() {
            let _ = ready_tx.send(true);
            info!("Discovery ready signal sent ({} cells)", result.cells_discovered);
        } else {
            warn!(
                "Discovery completed with issues: {} cells, {} errors — readiness NOT signaled",
                result.cells_discovered,
                result.errors.len()
            );
        }

        result
    })
}
```

- [ ] **Step 5: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs doorway/doorway-service/src/services/discovery.rs doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): add discovery readiness gate via watch channel"
```

---

### Task 2: Remove phantom fallback from discovery

**Files:**
- Modify: `doorway/doorway-service/src/services/discovery.rs`

The fallback block (lines 129-177) inserts placeholder base64-zero configs with role_name="lamad" when admin connection fails. This masks the real error and makes only lamad visible. Remove it — let the error propagate so the readiness gate stays `false`.

- [ ] **Step 1: Replace the fallback block with error propagation**

In `doorway/doorway-service/src/services/discovery.rs`, replace the `Err(e)` arm of `get_cells()` match (lines 129-177) with:

```rust
            Err(e) => {
                warn!("Failed to get cells from admin interface: {}", e);
                result.errors.push(format!("Admin connection failed: {e}"));
                return result;
            }
```

This removes:
- The placeholder `AAAAAAAAAAAAAAAA=` dna_hash insertion
- The hardcoded `role_name: "lamad"` fallback
- The fake `import_configs_found = 1`
- The misleading "Fallback import config registered" log

Discovery now returns with `cells_discovered: 0` and the error in `result.errors`, which means the readiness gate (from Task 1) stays `false` and routes get a proper 503.

- [ ] **Step 2: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

- [ ] **Step 3: Run tests**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/services/discovery.rs
git commit -m "fix(doorway): remove phantom lamad-only fallback from discovery

Discovery no longer inserts placeholder zome_configs with base64 zeros
when admin connection fails. Instead, it returns the error and the
readiness gate stays false, producing a clear 503 for conductor-dependent
routes."
```

---

### Task 3: Mark discovery stubs as explicit non-implementations

**Files:**
- Modify: `doorway/doorway-service/src/services/discovery.rs`

`discover_import_config` and `discover_routes` return hardcoded Ok(Some(...)) pretending discovery succeeded. Change them to return `Ok(None)` with an info log, so they don't populate configs with hardcoded values but also don't fail discovery.

The import config they return is identical to what the `register_standard_import_config` function provides during steward self-registration in main.rs. Routes registered from `build_manifest()` are the actual route source. These stubs are vestigial — they produce duplicate config.

- [ ] **Step 1: Replace discover_import_config stub**

```rust
    /// Discover import config from a cell.
    ///
    /// Not yet implemented — returns None. Import config comes from
    /// steward self-registration via build_manifest(), not DNA introspection.
    async fn discover_import_config(
        &self,
        _cell: &CellInfo,
        _zome_config: &ZomeCallConfig,
    ) -> Result<Option<ImportConfig>, String> {
        Ok(None)
    }
```

- [ ] **Step 2: Replace discover_routes stub**

```rust
    /// Discover routes from a cell via __doorway_routes.
    ///
    /// Not yet implemented — returns None. Routes come from steward
    /// self-registration via build_manifest(), not DNA introspection.
    async fn discover_routes(
        &self,
        _cell: &CellInfo,
        _zome_config: &ZomeCallConfig,
    ) -> Result<Option<DoorwayRoutes>, String> {
        Ok(None)
    }
```

- [ ] **Step 3: Remove unused `doorway_client` imports**

With the hardcoded route builder gone, check if `DoorwayRoutes` is still needed in the imports. If `discover_routes` return type uses it, keep it. Remove `DoorwayRoutesBuilder` and `Route` if they were only used in the stub.

- [ ] **Step 4: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/services/discovery.rs
git commit -m "fix(doorway): remove hardcoded stub configs from discovery

discover_import_config and discover_routes now return None instead of
hardcoded Ok(Some(config)). Import config and routes come from steward
self-registration via build_manifest(), not DNA introspection stubs."
```

---

### Task 4: Fix get_agent_pub_key wrong-role fallback

**Files:**
- Modify: `doorway/doorway-service/src/routes/zome_helpers.rs`

`get_agent_pub_key` falls back to ANY available zome config when imagodei isn't found. This could return a different agent's key. With discovery now working (Task 1-2), all roles should be populated. Remove the dangerous fallback.

- [ ] **Step 1: Remove the any-role fallback**

Replace the `get_agent_pub_key` function:

```rust
/// Get agent public key from the imagodei zome config
///
/// Returns the agent public key that the conductor uses for this app.
/// This is needed for auth responses.
///
/// Requires discovery to have completed successfully (imagodei role
/// must be in zome_configs). Returns error if not found — callers
/// should check discovery_ready before calling.
pub fn get_agent_pub_key(state: &AppState) -> Result<String> {
    get_zome_config_by_role(state, "imagodei").map(|config| config.agent_pub_key)
}
```

- [ ] **Step 2: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/routes/zome_helpers.rs
git commit -m "fix(doorway): remove wrong-role fallback from get_agent_pub_key

No longer falls back to ANY available zome config when imagodei isn't
found. Returns error if imagodei role is missing from zome_configs."
```

---

### Task 5: Replace recovery mock endpoints with 501

**Files:**
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs`

`recover_custody` and `check_recovery_status` return 200 OK with fake data. `activate_recovery` parses a mock request_id format. All three should return 501 Not Implemented until the imagodei zome integration is built.

- [ ] **Step 1: Replace handle_recover_custody**

Find the function `handle_recover_custody` (~line 2230). Replace everything after the `is_steward` check (line 2289) through the end of the function with:

```rust
    // Recovery requires imagodei zome integration (RecoveryRequest DHT entry).
    // Not yet implemented — return 501 so callers know the feature doesn't exist.
    json_response(
        StatusCode::NOT_IMPLEMENTED,
        &ErrorResponse {
            error: "Social recovery is not yet implemented. Requires imagodei zome integration.".into(),
            code: Some("NOT_IMPLEMENTED".into()),
        },
    )
```

- [ ] **Step 2: Replace handle_check_recovery_status**

Replace the body of `handle_check_recovery_status` (after request parsing) with:

```rust
    json_response(
        StatusCode::NOT_IMPLEMENTED,
        &ErrorResponse {
            error: "Recovery status checking is not yet implemented.".into(),
            code: Some("NOT_IMPLEMENTED".into()),
        },
    )
```

- [ ] **Step 3: Replace handle_activate_recovery**

Replace the body of `handle_activate_recovery` (after request parsing and password validation) with:

```rust
    // Recovery activation requires DHT-verified approval.
    // Not yet implemented — return 501.
    json_response(
        StatusCode::NOT_IMPLEMENTED,
        &ErrorResponse {
            error: "Recovery activation is not yet implemented.".into(),
            code: Some("NOT_IMPLEMENTED".into()),
        },
    )
```

- [ ] **Step 4: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass (existing tests don't test recovery mocks)

- [ ] **Step 5: Commit**

```bash
git add doorway/doorway-service/src/routes/auth_routes.rs
git commit -m "fix(doorway): replace recovery mock endpoints with 501 Not Implemented

recover_custody, check_recovery_status, and activate_recovery were
returning 200 OK with fake data, making recovery appear functional.
Now return 501 until imagodei zome integration is built."
```

---

### Task 6: Propagate app port error in chaperone

**Files:**
- Modify: `doorway/doorway-service/src/conductor/chaperone.rs`

When `list_app_interfaces` fails or returns empty, chaperone defaults to port 4445. This produces a wrong connection later. Instead, return an error.

- [ ] **Step 1: Replace the port fallback**

In `doorway/doorway-service/src/conductor/chaperone.rs` (~line 438), replace:

```rust
    let app_port = match admin.list_app_interfaces().await {
        Ok(ports) if !ports.is_empty() => ports[0],
        Ok(_) => {
            warn!("Chaperone: no app interfaces found, using default 4445");
            4445
        }
        Err(e) => {
            warn!("Chaperone: list_app_interfaces failed: {}, using 4445", e);
            4445
        }
    };
```

With:

```rust
    let app_port = match admin.list_app_interfaces().await {
        Ok(ports) if !ports.is_empty() => ports[0],
        Ok(_) => {
            error!("Chaperone: no app interfaces registered on conductor");
            return sanitize_client_error(StatusCode::BAD_GATEWAY, "No app interfaces");
        }
        Err(e) => {
            error!("Chaperone: list_app_interfaces failed: {}", e);
            return sanitize_client_error(StatusCode::BAD_GATEWAY, "List app interfaces");
        }
    };
```

- [ ] **Step 2: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/conductor/chaperone.rs
git commit -m "fix(doorway): propagate app port error in chaperone instead of defaulting to 4445

list_app_interfaces failure or empty result now returns 502 to the
client instead of silently connecting to the wrong port."
```

---

### Task 7: Make agent provisioning failure explicit

**Files:**
- Modify: `doorway/doorway-service/src/routes/auth_routes.rs`

Agent provisioning failure (~line 770) silently falls back to local keys and continues registration. The user gets a JWT but their agent doesn't exist on the conductor. Change to fail the registration.

- [ ] **Step 1: Replace the provisioning fallback**

In `doorway/doorway-service/src/routes/auth_routes.rs`, find the provisioning match block (~line 756-783). Replace:

```rust
                Err(e) => {
                    warn!(
                        "Agent provisioning failed, falling back to local keys: {}",
                        e
                    );
                    None
                }
```

With:

```rust
                Err(e) => {
                    error!(
                        "Agent provisioning failed — cannot complete registration: {}",
                        e
                    );
                    return json_response(
                        StatusCode::SERVICE_UNAVAILABLE,
                        &ErrorResponse {
                            error: format!("Agent provisioning failed: {e}"),
                            code: Some("PROVISIONING_FAILED".into()),
                        },
                    );
                }
```

- [ ] **Step 2: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/routes/auth_routes.rs
git commit -m "fix(doorway): fail registration when agent provisioning fails

Previously fell back to local keys, producing a JWT for an agent that
doesn't exist on the conductor. Now returns 503 so the client knows
registration didn't complete."
```

---

### Task 8: Replace storage registration no-op with honest error

**Files:**
- Modify: `doorway/doorway-service/src/services/storage_registration.rs`

`register_with_conductor` returns Ok(()) without doing anything. Change to return Err so callers know it's not implemented.

- [ ] **Step 1: Replace the no-op**

Replace `register_with_conductor` function body:

```rust
async fn register_with_conductor(
    conductor_url: &str,
    _installed_app_id: &str,
    zome_name: &str,
    input: &RegisterContentServerInput,
) -> Result<(), String> {
    // Storage registration via infrastructure zome is not yet implemented.
    // Log the intent for debugging, return error so callers don't assume success.
    info!(
        conductor_url = conductor_url,
        zome_name = zome_name,
        content_hash = %input.content_hash,
        capability = %input.capability,
        "Storage registration skipped (zome call not yet implemented)"
    );

    Err("register_with_conductor not yet implemented".to_string())
}
```

- [ ] **Step 2: Check callers handle the error**

Search for all callers of `register_with_conductor` and verify they handle `Err` without panicking. The caller in `register_storage_capabilities` already logs errors and adds them to `StorageRegistrationResult.errors`, so this change will surface the error in the result rather than silently succeeding.

Run: `grep -n 'register_with_conductor' doorway/doorway-service/src/services/storage_registration.rs`

- [ ] **Step 3: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/services/storage_registration.rs
git commit -m "fix(doorway): replace storage registration no-op with explicit error

register_with_conductor was returning Ok(()) without doing anything.
Now returns Err so callers know registration didn't happen."
```

---

### Task 9: Add discovery status to readiness probe

**Files:**
- Modify: `doorway/doorway-service/src/routes/health.rs`

The `/health` readiness check should report whether discovery has completed. This lets k8s know when the pod is ready to serve conductor-dependent requests.

- [ ] **Step 1: Add discovery status to readiness_check**

In the `readiness_check` function (or `health_check`), add a check for `state.discovery_ready`:

```rust
// Discovery status
let discovery_complete = *state.discovery_ready.borrow();

// If discovery hasn't completed and we're not in dev mode, report not ready
if !discovery_complete && !args.dev_mode {
    // Still return 200 for k8s (liveness != readiness), but include in status
    // The error field already exists — add discovery status
}
```

Add `"discovery_complete": discovery_complete` to the health JSON response body.

The exact integration depends on the existing health response structure — read `health_check` to determine where to insert this field.

- [ ] **Step 2: Build and test**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo check 2>&1 | tail -5`
Expected: `Finished`

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all pass

- [ ] **Step 3: Commit**

```bash
git add doorway/doorway-service/src/routes/health.rs
git commit -m "feat(doorway): add discovery_complete to health endpoint

Health response now includes whether zome discovery has completed,
making it visible when the doorway is degraded due to conductor
unavailability."
```

---

### Task 10: Final integration test — clippy, fmt, full test suite

**Files:** None (verification only)

- [ ] **Step 1: Run cargo fmt**

Run: `cd doorway/doorway-service && cargo fmt --check`
Expected: no formatting issues. If any, run `cargo fmt` and commit.

- [ ] **Step 2: Run clippy**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo clippy -- -D warnings 2>&1 | tail -10`
Expected: `Finished` with no errors

- [ ] **Step 3: Run full test suite**

Run: `cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -10`
Expected: all 397+ tests pass

- [ ] **Step 4: Verify no remaining silent fallbacks**

Run: `grep -rn 'warn!.*[Ff]allback\|warn!.*[Dd]efault.*4445\|Ok(())\s*//.*TODO\|role_name: "lamad".*Default' doorway/doorway-service/src/`
Expected: no matches (all silent fallbacks removed)

- [ ] **Step 5: Final commit (if fmt needed fixes)**

```bash
git add -A doorway/doorway-service/
git commit -m "style(doorway): cargo fmt after stabilization"
```

---

## Summary of Changes

| Finding | Task | Fix |
|---------|------|-----|
| Phantom lamad-only fallback | Task 2 | Remove — let error propagate, readiness gate stays false |
| Discovery/HTTP race condition | Task 1 | watch channel + readiness gate |
| Recovery mock endpoints | Task 5 | Return 501 Not Implemented |
| discover_import_config stub | Task 3 | Return Ok(None) — config comes from build_manifest |
| discover_routes stub | Task 3 | Return Ok(None) — routes come from build_manifest |
| get_agent_pub_key wrong-role fallback | Task 4 | Return error if imagodei not found |
| Chaperone port 4445 default | Task 6 | Return 502 error |
| Agent provisioning fallback | Task 7 | Return 503 error |
| Storage registration no-op | Task 8 | Return Err |
| Geo routing empty vec | N/A | Acceptable — gated by `enable_geo_routing` config |
| Health probe missing discovery | Task 9 | Add `discovery_complete` field |

## What This Does NOT Change

- **ZomeCaller** — already works correctly, no changes needed
- **TypedAdminClient** — already wired into discovery (done earlier today), no changes needed
- **call_create_human** — already uses ZomeCaller (done earlier today), no changes needed
- **Delivery relay geo routing** — returns empty vec but is gated by `enable_geo_routing: false` config, not a silent failure
- **Dev mode fallbacks** — the `if args.dev_mode` blocks in auth_routes.rs are intentional and clearly logged; they stay
