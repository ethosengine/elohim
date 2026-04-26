# Storage Phase 11 — Zome-Forwarding Bridge Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Connect the four `/api/v1/account/*` M5 stubs (`self-revocation`, `recovery/:id/vote`, `portal-hosts` POST, `portal-hosts/:url_b64` DELETE) to the imagodei coordinator zome via storage's existing HcClient, in Tauri-direct deployment mode. Browser-via-doorway calls return a clearer 503 contract for M6.

**Architecture:** Add a small `HcClientRegistry` keyed by zome role (infrastructure, imagodei). Connect the imagodei role at startup alongside the existing infrastructure role. A mode gate compares the resolved caller agent key to the connected cell's owner — match → forward to zome; mismatch → `503 BROWSER_WRITE_PATH_PENDING`. Connect failure → `503 IMAGODEI_BRIDGE_OFFLINE`. A generic `forward_to_imagodei<I, O>` helper handles MessagePack encode/decode and error mapping for all four routes.

**Tech Stack:** Rust 1.x, hyper, tokio, holochain_client 0.9.0-dev.5, rmp-serde 1.3, ts-rs (already in elohim-storage Cargo.toml).

**Spec:** `genesis/docs/superpowers/specs/2026-04-26-storage-phase-11-zome-forwarding-bridge-design.md`

**Build commands (Eclipse Che):**
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
cargo fmt --check
cargo test export_bindings    # only if InputView/OutputView types changed
```

Per `feedback_shift_measure_jenkins`: live a2o verification runs on Jenkins, not locally. Local pre-push gate is fmt + clippy + build + lib tests only.

---

## File Structure

**Files to create:**
- `elohim/elohim-storage/src/hc_client_registry.rs` — new module, ~80 lines
- (no new test files; tests are added to existing modules)

**Files to modify:**
- `elohim/elohim-storage/src/lib.rs` — add `pub mod hc_client_registry;` export
- `elohim/elohim-storage/src/hc_client.rs` — add `trait CellOwner` and impl for HcClient
- `elohim/elohim-storage/src/main.rs:425-560` — refactor heartbeat block to consume registry; add imagodei role connection
- `elohim/elohim-storage/src/api/mod.rs:84-86` — pass registry to `account::handle`
- `elohim/elohim-storage/src/api/account.rs` — full set: handle signature change, four forwarder helpers, mode gate, error mapping, three new InputView/OutputView types in `views.rs`, remove old stub function, replace stub tests with new tests
- `elohim/elohim-storage/src/views.rs` — add three InputView types and four OutputView types
- `genesis/a2o/features/auth/recovery/recovery-m5-self-revoke.feature` — remove 1 `@phase11-pending` tag
- `genesis/a2o/features/auth/recovery/recovery-m5-vote-as-emergency-contact.feature` — remove 2 `@phase11-pending` tags
- `genesis/a2o/features/auth/recovery/recovery-m5-portal-host-discovery.feature` — remove 1 `@phase11-pending` tag
- `genesis/a2o/features/auth/recovery/recovery-m5-defender-role-gate.feature` — remove 1 `@phase11-pending` tag

---

### Task 1: Document the provenance assumption

**Why:** Spec's risk register flags `agent_info()?.agent_initial_pubkey` returning the cell-owner (not the signer's key) as the architectural premise. Already empirically true: the existing heartbeat path commits `record_peer_status` zome calls and the resulting peer statuses are correctly attributed to the cell-owner agent. Confirm by reading heartbeat.rs and writing one comment in account.rs that anchors this dependency for future maintainers.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs:11-16` (the existing module-level doc block)

- [ ] **Step 1: Read the existing PeerStatus attribution path**

Run: `grep -n "agent_pub_key\|caller_pubkey\|agent_info" /projects/elohim/elohim/holochain/dna/infrastructure/zomes/coordinator/infrastructure/src/lib.rs` (or wherever `record_peer_status` lives — search if path differs).

Expected: at least one read of `agent_info()?.agent_initial_pubkey` returning the calling agent. Existing heartbeat data attributes peer statuses to peers correctly, confirming the cell-owner-is-caller behavior.

- [ ] **Step 2: Add provenance comment to `api/account.rs` module doc**

Modify the module doc block (currently lines 11-16) to add:

```rust
//! ## Provenance assumption (Phase 11)
//! `HcClient::call_zome` signs the call with admin-issued credentials, but
//! the conductor presents the call to the zome as the cell's owner agent.
//! Empirically verified by the heartbeat path: `record_peer_status` reads
//! `agent_info()?.agent_initial_pubkey` and the resulting peer statuses are
//! correctly attributed to peers (not to storage's signer). The mode gate
//! `verify_caller_owns_cell` exploits this: if the connected cell's owner
//! matches the resolved human key, the zome will see the human as caller.
```

- [ ] **Step 3: Verify build still compiles after comment-only change**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs
git commit -m "docs(storage): pin Phase 11 provenance assumption in account module doc"
```

---

### Task 2: Add `trait CellOwner` to `hc_client.rs`

**Why:** The mode gate needs only the cell-owner agent key — no other HcClient capability. Trait-ifying it makes the gate trivially unit-testable without mocking `AppWebsocket`.

**Files:**
- Modify: `elohim/elohim-storage/src/hc_client.rs` (add trait + impl, near the existing `pub fn agent_pub_key(&self)` method around line 276)

- [ ] **Step 1: Write the failing test**

Append to `hc_client.rs`:

```rust
#[cfg(test)]
mod cell_owner_tests {
    use super::*;

    /// A stub CellOwner used by `account.rs` mode-gate tests. Verifies the
    /// trait dispatch lands on the stub's `agent_key_hex()` return.
    struct StubOwner(String);
    impl CellOwner for StubOwner {
        fn agent_key_hex(&self) -> String {
            self.0.clone()
        }
    }

    #[test]
    fn stub_cell_owner_returns_configured_hex() {
        let stub = StubOwner("uhCAkSTUB".to_string());
        let dyn_owner: &dyn CellOwner = &stub;
        assert_eq!(dyn_owner.agent_key_hex(), "uhCAkSTUB");
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib hc_client::cell_owner_tests
```

Expected: FAIL with `cannot find trait CellOwner in this scope`.

- [ ] **Step 3: Add the trait and impl**

Add (around line 276, near `pub fn agent_pub_key`):

```rust
/// A `CellOwner` exposes the agent key of the cell a client is connected to.
///
/// The mode gate in `api/account.rs` uses this to assert that the caller's
/// resolved human key matches the connected cell's owner — a Tauri-direct
/// invariant. Trait-ifying it lets unit tests exercise the gate without
/// touching the holochain_client websocket layer.
pub trait CellOwner: Send + Sync {
    /// Returns the connected cell's owner agent key as a hex string,
    /// matching the encoding used by `api/identity::extract_agent_key`
    /// (X-Agent-Id header / local session row).
    fn agent_key_hex(&self) -> String;
}

impl CellOwner for HcClient {
    fn agent_key_hex(&self) -> String {
        hex::encode(self.cell_id.agent_pubkey().get_raw_39())
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib hc_client::cell_owner_tests
```

Expected: PASS (one test).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/hc_client.rs
git commit -m "storage(hc-client): add CellOwner trait for mode-gate testing"
```

---

### Task 3: Create `hc_client_registry.rs` module

**Why:** Phase 11 introduces a second HcClient (imagodei role). Centralizing role-keyed connections in a small registry keeps `main.rs` tidy and makes future roles (lamad, mishpat) drop-in.

**Files:**
- Create: `elohim/elohim-storage/src/hc_client_registry.rs`
- Modify: `elohim/elohim-storage/src/lib.rs` (add `pub mod hc_client_registry;`)

- [ ] **Step 1: Create the registry module**

Write `elohim/elohim-storage/src/hc_client_registry.rs`:

```rust
//! HcClientRegistry — role-keyed cache of conductor connections.
//!
//! Phase 11 introduces a second HcClient connection (role: "imagodei")
//! alongside the existing infrastructure role used by the heartbeat path.
//! Keeping these in one struct keeps `main.rs` startup tidy and avoids
//! having to thread two `Option<Arc<HcClient>>` parameters separately.
//!
//! Connect failure for any role is logged and non-fatal — the node keeps
//! serving HTTP. Downstream code checks `Option<Arc<HcClient>>` and
//! returns a 503 if the role is unconnected.

use std::sync::Arc;
use tracing::{info, warn};

use crate::hc_client::{HcClient, HcClientConfig};

/// Role-keyed registry of HcClient connections. Fields hold `None` when
/// the role failed to connect at startup; downstream code returns 503
/// (`IMAGODEI_BRIDGE_OFFLINE` etc.) if the role is unavailable.
pub struct HcClientRegistry {
    pub infrastructure: Option<Arc<HcClient>>,
    pub imagodei: Option<Arc<HcClient>>,
}

/// Connection inputs. Mirrors the relevant CLI args without depending on
/// the Args struct directly (cleaner test surface).
#[derive(Debug, Clone)]
pub struct HcRegistryInputs {
    pub admin_url: String,
    pub app_url: String,
    pub app_id: String,
}

impl HcClientRegistry {
    /// Connect each role in sequence. Per-role failure is logged and
    /// returns `None` for that role — the registry as a whole always
    /// constructs.
    pub async fn connect(inputs: &HcRegistryInputs) -> Self {
        let infrastructure = Self::connect_role(inputs, "infrastructure").await;
        let imagodei = Self::connect_role(inputs, "imagodei").await;
        Self {
            infrastructure,
            imagodei,
        }
    }

    async fn connect_role(inputs: &HcRegistryInputs, role: &str) -> Option<Arc<HcClient>> {
        match HcClient::connect(HcClientConfig {
            admin_url: inputs.admin_url.clone(),
            app_url: inputs.app_url.clone(),
            app_id: inputs.app_id.clone(),
            role: Some(role.to_string()),
        })
        .await
        {
            Ok(hc) => {
                info!(role, "HcClient connected");
                Some(Arc::new(hc))
            }
            Err(e) => {
                warn!(
                    role,
                    error = %e,
                    "HcClient connect failed — routes for this role will return 503"
                );
                None
            }
        }
    }
}
```

- [ ] **Step 2: Wire the module into `lib.rs`**

Find the section in `elohim/elohim-storage/src/lib.rs` that lists `pub mod hc_client;` and add `pub mod hc_client_registry;` immediately below it (alphabetical-ish; the existing module list is not strictly ordered).

Run: `grep -n "pub mod hc_client" /projects/elohim/elohim/elohim-storage/src/lib.rs` to find the line.

Then add the new line immediately after.

- [ ] **Step 3: Build the crate**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: `Finished` — registry has no consumers yet, so this verifies it compiles standalone.

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/hc_client_registry.rs elohim/elohim-storage/src/lib.rs
git commit -m "storage(hc-registry): add role-keyed HcClient registry module"
```

---

### Task 4: Refactor `main.rs` to construct the registry and connect imagodei

**Why:** The existing `main.rs:468` block constructs an HcClient inline for the infrastructure role. Replace with a single `HcClientRegistry::connect` call; the heartbeat path consumes `registry.infrastructure`. The imagodei role is connected at the same point.

**Files:**
- Modify: `elohim/elohim-storage/src/main.rs:425-560`

- [ ] **Step 1: Read the existing block once more before editing**

Re-read `elohim/elohim-storage/src/main.rs` lines 425-560 to refresh on the heartbeat construction. The refactor preserves the heartbeat behavior; only the source of the HcClient changes.

- [ ] **Step 2: Refactor the block**

Replace the block at `main.rs:468-559` (the `match elohim_storage::hc_client::HcClient::connect(...) { Ok(hc) => { ... } Err(e) => { ... } }` block) with a registry-based version. Keep everything outside the match (peer_policy_path, conductor forwarder spawning) unchanged.

The refactor pattern — leave the `if let Some(admin_url) = &args.admin_url { ... }` outer guard alone, and replace the inner heartbeat-construction `match` with:

```rust
                let registry = elohim_storage::hc_client_registry::HcClientRegistry::connect(
                    &elohim_storage::hc_client_registry::HcRegistryInputs {
                        admin_url: admin_url.clone(),
                        app_url: args.app_url.clone(),
                        app_id: args.app_id.clone(),
                    },
                )
                .await;
                let registry = std::sync::Arc::new(registry);

                // Heartbeat path — consume registry.infrastructure.
                if let Some(hc) = registry.infrastructure.clone() {
                    let agent = hc.cell_id().agent_pubkey().clone();
                    let publisher =
                        elohim_storage::heartbeat::ZomeCallPublisher::new(hc.clone(), agent);
                    let probe = elohim_storage::heartbeat::DefaultProbe::new(
                        blob_store.clone(),
                        hc.clone(),
                    );
                    let mut heartbeat = elohim_storage::heartbeat::HeartbeatTask::new(
                        policy_cfg, publisher, probe,
                    );
                    if let Some(archetype) = config.device_archetype.clone() {
                        heartbeat = heartbeat.with_archetype_class(archetype);
                    }
                    let hb_shutdown = shutdown_tx.subscribe();
                    tokio::spawn(async move {
                        heartbeat.run(hb_shutdown).await;
                    });
                    info!(
                        policy_path = %peer_policy_path.display(),
                        "PeerStatus heartbeat task started (infrastructure role)"
                    );

                    // InfrastructureSignal subscriber (unchanged from M5).
                    if let Some(subscriber_pool) = db_pool.clone() {
                        let hc_sub = hc.clone();
                        tokio::spawn(async move {
                            let pool = subscriber_pool;
                            let handle_id = hc_sub
                                .subscribe_infrastructure_signals(
                                    move |signal: elohim_storage::signals::InfrastructureSignal| {
                                        match pool.get() {
                                            Ok(mut conn) => {
                                                if let Err(e) =
                                                    elohim_storage::signals::handle_signal(
                                                        &mut conn, signal,
                                                    )
                                                {
                                                    warn!(
                                                        error = %e,
                                                        "InfrastructureSignal projection failed"
                                                    );
                                                }
                                            }
                                            Err(e) => warn!(
                                                error = %e,
                                                "Failed to acquire DB connection for signal projection"
                                            ),
                                        }
                                    },
                                )
                                .await;
                            info!(
                                subscription_id = %handle_id,
                                "InfrastructureSignal subscriber registered (projects PeerStatusRecorded → SQLite)"
                            );
                        });
                    } else {
                        warn!(
                            "InfrastructureSignal subscriber disabled: shared DB pool unavailable"
                        );
                    }
                } else {
                    warn!(
                        "PeerStatus heartbeat disabled: infrastructure HcClient unavailable in registry"
                    );
                }

                // Stash the registry in shared state for HTTP handlers.
                hc_registry_for_http = Some(registry);
```

Then before `if let Some(admin_url) = &args.admin_url`, declare `let mut hc_registry_for_http: Option<std::sync::Arc<elohim_storage::hc_client_registry::HcClientRegistry>> = None;`.

After the outer `if let` closes, the `hc_registry_for_http` binding holds either the registry or `None` (when admin_url isn't configured). It is wired into the HTTP handler set in Task 5.

- [ ] **Step 3: Build to verify the refactor compiles**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: `Finished` with no errors. Heartbeat behavior is preserved (same flow, just reading from registry).

- [ ] **Step 4: Commit**

```bash
git add elohim/elohim-storage/src/main.rs
git commit -m "storage(main): route heartbeat HcClient through HcClientRegistry"
```

---

### Task 5: Thread registry through `api/mod.rs` to `account::handle`

**Why:** The HTTP dispatch needs to pass the registry to the account controller so the four forwarder helpers can pick the imagodei HcClient. Pass as a separate parameter (don't pollute `Services`).

**Files:**
- Modify: `elohim/elohim-storage/src/api/mod.rs` (`handle_api_request` signature; `account::handle` call at line 86)
- Modify: `elohim/elohim-storage/src/api/account.rs` (`handle` signature)
- Modify: `elohim/elohim-storage/src/main.rs` and `elohim/elohim-storage/src/http.rs` — both call sites for `handle_api_request` (find with grep)

- [ ] **Step 1: Find the `handle_api_request` call sites**

Run: `grep -rn "handle_api_request" /projects/elohim/elohim/elohim-storage/src/`

Expected: one definition in `api/mod.rs:70`, one or more call sites in `http.rs` (and possibly `main.rs`). Note each line.

- [ ] **Step 2: Update `account::handle` signature**

In `elohim/elohim-storage/src/api/account.rs`, change the function signature at line 39:

```rust
pub async fn handle(
    req: Request<Incoming>,
    method: Method,
    resource_path: &str,
    pool: &DbPool,
    hc_registry: Option<&Arc<crate::hc_client_registry::HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
```

Add at the top of the file: `use std::sync::Arc;` (if not already present).

The four route arms keep their stub call shape for now; Task 13 swaps them. The new `hc_registry` parameter is unused in this task — it's intentionally `_unused` until later tasks consume it. Add `#[allow(unused_variables)]` to the function temporarily, OR rename to `_hc_registry` until Task 6 reads it. Use `_hc_registry`.

- [ ] **Step 3: Update `handle_api_request` signature**

In `elohim/elohim-storage/src/api/mod.rs`, change the function signature at line 70:

```rust
pub async fn handle_api_request(
    req: Request<Incoming>,
    method: Method,
    path: &str,
    pool: DbPool,
    services: Option<Arc<Services>>,
    hc_registry: Option<Arc<crate::hc_client_registry::HcClientRegistry>>,
) -> Result<Response<Full<Bytes>>, StorageError> {
```

Update the `account::handle` call at line 86:

```rust
return account::handle(req, method, resource_path, &pool, hc_registry.as_ref()).await;
```

- [ ] **Step 4: Update each `handle_api_request` call site**

Append `hc_registry.clone()` (or `None` for tests, if tests construct calls inline) to every caller. Most likely one site in `http.rs`. Pass `None` if the caller doesn't have a registry (e.g., test fixtures).

The actual `Some(Arc<HcClientRegistry>)` value flows from `main.rs`'s `hc_registry_for_http` (Task 4) → into the HTTP server state struct → into the per-request dispatch. Find where the HTTP server state is built (likely `http.rs` `HttpServer::new` or similar) and add a field for the registry.

If the engineer hits a structural challenge here (e.g., the HTTP server state is constructed in a way that doesn't accept new fields cleanly), prefer the smallest-blast-radius option: add an `hc_registry: Option<Arc<HcClientRegistry>>` field and a `with_hc_registry(self, registry)` builder method.

- [ ] **Step 5: Build to verify the wiring compiles**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: `Finished` with no errors. The registry is plumbed end-to-end but unused.

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/api/mod.rs elohim/elohim-storage/src/api/account.rs elohim/elohim-storage/src/http.rs elohim/elohim-storage/src/main.rs
git commit -m "storage(api): thread HcClientRegistry to account controller"
```

---

### Task 6: Add `verify_caller_owns_cell` mode gate + the two 503 contracts

**Why:** Distinguishes Tauri-direct (cell-owner matches caller → forward) from browser-via-doorway (mismatch → defer to M6). Plus a `IMAGODEI_BRIDGE_OFFLINE` 503 for the case where startup couldn't connect imagodei.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs`

- [ ] **Step 1: Write the failing tests**

Inside the existing `mod tests` block at the bottom of `account.rs`, replace the body of `zome_bridge_stub_returns_503` and `zome_bridge_all_stub_routes_return_503` (which are getting deleted in Task 13) with new tests immediately. (We could also add the new tests now and delete the old ones in Task 13; that's the cleaner sequence — leave the old tests alone here, just add the new ones.)

Append to `mod tests`:

```rust
    use crate::hc_client::CellOwner;

    struct StubOwner(&'static str);
    impl CellOwner for StubOwner {
        fn agent_key_hex(&self) -> String {
            self.0.to_string()
        }
    }

    /// When the caller's resolved agent key matches the connected cell's
    /// owner (Tauri-direct invariant), the gate returns Ok — caller has
    /// the cell and zome calls proceed.
    #[test]
    fn verify_caller_owns_cell_passes_when_keys_match() {
        let owner = StubOwner("uhCAkMATCH");
        let result = verify_caller_owns_cell(&owner, "uhCAkMATCH");
        assert!(result.is_ok(), "expected Ok when keys match");
    }

    /// When the caller's resolved agent key does not match the connected
    /// cell's owner (browser-via-doorway path), the gate returns Err with
    /// a 503 body containing `code: "BROWSER_WRITE_PATH_PENDING"`.
    #[test]
    fn verify_caller_owns_cell_returns_browser_pending_on_mismatch() {
        let owner = StubOwner("uhCAkOWNER");
        let result = verify_caller_owns_cell(&owner, "uhCAkCALLER");
        let resp = result.expect_err("expected Err with 503 response");
        assert_eq!(resp.status(), hyper::StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = futures::executor::block_on(async {
            use http_body_util::BodyExt;
            resp.into_body().collect().await.unwrap().to_bytes()
        });
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["code"], "BROWSER_WRITE_PATH_PENDING");
    }

    /// The IMAGODEI_BRIDGE_OFFLINE response surfaces when startup couldn't
    /// connect the imagodei role — distinct code from BROWSER_WRITE_PATH_PENDING.
    #[test]
    fn imagodei_bridge_offline_response_has_correct_code() {
        let resp = response_503_imagodei_bridge_offline();
        assert_eq!(resp.status(), hyper::StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = futures::executor::block_on(async {
            use http_body_util::BodyExt;
            resp.into_body().collect().await.unwrap().to_bytes()
        });
        let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
        assert_eq!(body["code"], "IMAGODEI_BRIDGE_OFFLINE");
    }
```

Add `futures` to the dev-dependencies if not already present (`grep -n "^futures" /projects/elohim/elohim/elohim-storage/Cargo.toml`); if missing, add `futures = "0.3"` under `[dev-dependencies]`.

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account::tests::verify_caller_owns_cell
```

Expected: FAIL with `cannot find function verify_caller_owns_cell` and similar for `response_503_imagodei_bridge_offline`.

- [ ] **Step 3: Implement the gate + the two 503 helpers**

Add to `account.rs` (immediately above the existing `zome_bridge_not_yet_wired` function around line 484):

```rust
// ---------------------------------------------------------------------------
// Phase 11: mode gate + 503 contracts
// ---------------------------------------------------------------------------

/// Asserts the caller's resolved agent key matches the connected cell's
/// owner. The Tauri-direct invariant — when matched, the imagodei zome
/// will see the human as caller (cell owner == caller per the provenance
/// note in the module doc).
///
/// `Err(Response<...>)` is returned with `503 BROWSER_WRITE_PATH_PENDING`
/// when the keys do not match — the browser-via-doorway path that lands
/// in M6 once the hosting trust model is settled.
fn verify_caller_owns_cell(
    owner: &dyn crate::hc_client::CellOwner,
    agent_key: &str,
) -> Result<(), Response<Full<Bytes>>> {
    if owner.agent_key_hex() != agent_key {
        return Err(response_503_browser_write_path_pending());
    }
    Ok(())
}

/// 503 response for the browser-via-doorway write path (M6+).
fn response_503_browser_write_path_pending() -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "browser write path not yet implemented",
        "code": "BROWSER_WRITE_PATH_PENDING",
        "message": "Self-sovereign writes require a peer the human controls. \
                    The browser-via-doorway write path is deferred to M6 where \
                    the hosting trust model is settled."
    });
    response::json_response(hyper::StatusCode::SERVICE_UNAVAILABLE, &body)
}

/// 503 response when the imagodei HcClient failed to connect at startup.
/// Recovery: restart storage with the imagodei DNA installed.
fn response_503_imagodei_bridge_offline() -> Response<Full<Bytes>> {
    let body = serde_json::json!({
        "error": "imagodei bridge offline",
        "code": "IMAGODEI_BRIDGE_OFFLINE",
        "message": "The storage process did not connect to the imagodei \
                    coordinator zome at startup. Account write routes are \
                    unavailable until storage restarts with the imagodei DNA \
                    installed."
    });
    response::json_response(hyper::StatusCode::SERVICE_UNAVAILABLE, &body)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account::tests
```

Expected: all 6+ tests pass (3 new + 3 existing M5 tests + 2 stub tests still present).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs elohim/elohim-storage/Cargo.toml
git commit -m "storage(account): add mode-gate + BROWSER_WRITE_PATH_PENDING/IMAGODEI_BRIDGE_OFFLINE 503 contracts"
```

---

### Task 7: Add `map_zome_err_to_http` error mapping function

**Why:** All four forwarder helpers need a consistent error-to-HTTP mapping. String-matching on well-known zome error prefixes is acknowledged-brittle but matches what the zome actually returns today.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs`

- [ ] **Step 1: Write the failing tests**

Append to `mod tests` in `account.rs`:

```rust
    use crate::error::StorageError;

    /// Gate-rejection messages from the imagodei coordinator map to 403.
    #[test]
    fn map_zome_err_to_http_403_for_gate_rejection() {
        let cases = [
            "Conductor(\"create_self_revocation: caller does not control revoked_key (different human_id)\")",
            "Conductor(\"submit_revocation_vote: caller is not an active emergency contact for human-x\")",
            "Conductor(\"submit_specialist_revocation: caller is not a configured defender for this human\")",
            "Conductor(\"submit_specialist_revocation: revoked_pub_key does not belong to target human\")",
        ];
        for msg in cases {
            let err = StorageError::Conductor(msg.to_string());
            let resp = map_zome_err_to_http(&err);
            assert_eq!(
                resp.status(),
                hyper::StatusCode::FORBIDDEN,
                "expected 403 for {msg}"
            );
        }
    }

    /// Input-validation failures map to 400.
    #[test]
    fn map_zome_err_to_http_400_for_invalid_input() {
        let cases = [
            "Conductor(\"create_self_revocation: invalid reason 'bogus'\")",
            "Conductor(\"submit_revocation_vote: revocation rev-x already effective\")",
            "Conductor(\"submit_revocation_vote: revocation rev-x has trigger_type=voluntary, votes not accepted\")",
            "Conductor(\"submit_revocation_vote: attestation cannot be empty\")",
            "Conductor(\"submit_revocation_vote: no KeyRevocation with id rev-missing\")",
        ];
        for msg in cases {
            let err = StorageError::Conductor(msg.to_string());
            let resp = map_zome_err_to_http(&err);
            assert_eq!(
                resp.status(),
                hyper::StatusCode::BAD_REQUEST,
                "expected 400 for {msg}"
            );
        }
    }

    /// Connectivity failures map to 503.
    #[test]
    fn map_zome_err_to_http_503_for_connection_error() {
        let err = StorageError::Connection("Admin connect failed: refused".to_string());
        let resp = map_zome_err_to_http(&err);
        assert_eq!(resp.status(), hyper::StatusCode::SERVICE_UNAVAILABLE);
    }

    /// Anything else falls through to 500.
    #[test]
    fn map_zome_err_to_http_500_for_unknown() {
        let err = StorageError::Conductor("Zome call failed: unexpected internal error".to_string());
        let resp = map_zome_err_to_http(&err);
        assert_eq!(resp.status(), hyper::StatusCode::INTERNAL_SERVER_ERROR);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account::tests::map_zome_err_to_http
```

Expected: FAIL with `cannot find function map_zome_err_to_http`.

- [ ] **Step 3: Implement the mapping function**

Add to `account.rs` (just below `response_503_imagodei_bridge_offline`):

```rust
/// Maps a `StorageError` from a zome call to an HTTP response.
///
/// PHASE-11-DEBT: this string-matches well-known zome error prefixes.
/// Brittle by design — matches what `imagodei` returns today. Typed
/// errors over the conductor wire are an M6+ refactor.
fn map_zome_err_to_http(err: &StorageError) -> Response<Full<Bytes>> {
    let msg = err.to_string();

    // 403 — gate rejections (defender, EC, ownership)
    if msg.contains("not a configured defender")
        || msg.contains("not an active emergency contact")
        || msg.contains("does not control")
        || msg.contains("does not belong to")
    {
        let body = serde_json::json!({
            "error": "forbidden",
            "code": "ZOME_GATE_REJECTED",
            "message": msg,
        });
        return response::json_response(hyper::StatusCode::FORBIDDEN, &body);
    }

    // 400 — input validation
    if msg.contains("invalid reason")
        || msg.contains("already effective")
        || msg.contains("votes not accepted")
        || msg.contains("attestation cannot be empty")
        || msg.contains("no KeyRevocation with id")
    {
        return response::bad_request(&msg);
    }

    // 503 — conductor connectivity
    if msg.contains("Connection") || msg.contains("Admin connect") || msg.contains("App connect") {
        return response::service_unavailable(&msg);
    }

    response::internal_error(&msg)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account::tests::map_zome_err_to_http
```

Expected: all 4 tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs
git commit -m "storage(account): add map_zome_err_to_http with 403/400/503/500 branches"
```

---

### Task 8: Add three missing InputView types and four OutputView types

**Why:** Of the four routes, only `add_portal_host` has a M5 InputView (`AddPortalHostInputView`). The other three need new InputView types. All four also need OutputView types so HTTP responses are camelCase JSON not raw zome MessagePack.

**Files:**
- Modify: `elohim/elohim-storage/src/views.rs`

- [ ] **Step 1: Read where AddPortalHostInputView is defined**

Run: `grep -n "AddPortalHostInputView\|SubmitSpecialistRevocationInputView" /projects/elohim/elohim/elohim-storage/src/views.rs`

Expected: AddPortalHostInputView at ~6993, SubmitSpecialistRevocationInputView near it. New types go in the same neighborhood for cohesion.

- [ ] **Step 2: Add the three input views and four output views**

Add immediately after `AddPortalHostInputView` (or wherever the M5 account-related views cluster):

```rust
/// Input for self-revocation (M4 fast-path: a human voluntarily revokes
/// one of their own keys).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateSelfRevocationInputView {
    /// Base64-encoded AgentPubKey (e.g. "uhCAk...") of the key being revoked.
    /// Must belong to the same human as the caller.
    pub revoked_key: String,
    /// Reason — one of REVOCATION_REASONS recognised by the imagodei zome.
    pub reason: String,
}

/// Output of `create_self_revocation` — projected from the zome's
/// `KeyRevocationOutput` for HTTP camelCase responses.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct CreateSelfRevocationOutputView {
    pub revocation_id: String,
    /// Base64-encoded ActionHash of the committed KeyRevocation entry.
    pub action_hash: String,
}

/// Input for an emergency-contact vote on a pending KeyRevocation.
/// The `revocation_id` arrives via the URL path `/recovery/:id/vote`,
/// so the body holds only the steward's vote payload.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SubmitRevocationVoteInputView {
    /// `true` to approve the revocation, `false` to reject. The M4 zome
    /// only counts approvals towards the threshold; rejections are recorded
    /// for transparency.
    pub approved: bool,
    /// Free-text steward attestation — must be non-empty.
    pub attestation: String,
}

/// Output of `submit_revocation_vote` — projected from the zome's
/// `RevocationVoteOutput` for HTTP camelCase responses.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct SubmitRevocationVoteOutputView {
    pub vote_id: String,
    pub current_votes: u32,
    pub required_votes: u32,
    pub threshold_now_reached: bool,
}

/// Output of `add_portal_host` — projected from the zome's `ActionHash`
/// return for a uniform HTTP shape.
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct AddPortalHostOutputView {
    /// Base64-encoded ActionHash of the committed PortalHost entry.
    pub action_hash: String,
}

/// Output of `remove_portal_host`. Empty body on success — included for
/// uniform contract shape (clients may still expect a JSON body).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct RemovePortalHostOutputView {
    /// Always `true` — the zome returns `()` on success; clients can use
    /// this as a presence check.
    pub deleted: bool,
}
```

- [ ] **Step 3: Build to verify the new types compile**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: `Finished` with no errors.

- [ ] **Step 4: Regenerate TypeScript bindings**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
cargo test export_bindings 2>&1 | tail -20
```

Expected: bindings regenerated under `elohim/sdk/storage-client-ts/src/generated/`. Five new files appear: `CreateSelfRevocationInputView.ts`, `CreateSelfRevocationOutputView.ts`, `SubmitRevocationVoteInputView.ts`, `SubmitRevocationVoteOutputView.ts`, `AddPortalHostOutputView.ts`, `RemovePortalHostOutputView.ts` (six files — five new + AddPortalHostInputView.ts may also re-emit unchanged).

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/views.rs elohim/sdk/storage-client-ts/src/generated/
git commit -m "storage(views): add Phase 11 InputView/OutputView types for account write routes"
```

---

### Task 9: Add the generic `forward_to_imagodei` helper

**Why:** All four route helpers share the same shape: encode payload → call zome → decode response → return JSON. Encapsulating in a generic helper keeps each route's code under 30 lines.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs`

- [ ] **Step 1: Add the helper**

Add to `account.rs` (just below `map_zome_err_to_http`):

```rust
// ---------------------------------------------------------------------------
// Phase 11: generic zome-call forwarder
// ---------------------------------------------------------------------------

/// Forward a zome call to the imagodei coordinator and return the decoded
/// output. MessagePack-encodes `input`, calls `hc.call_zome("imagodei",
/// fn_name, payload)`, and MessagePack-decodes the response into `O`.
///
/// Errors are returned as `StorageError`; route handlers map them to HTTP
/// via `map_zome_err_to_http`.
async fn forward_to_imagodei<I, O>(
    hc: &crate::hc_client::HcClient,
    fn_name: &str,
    input: &I,
) -> Result<O, StorageError>
where
    I: serde::Serialize,
    O: serde::de::DeserializeOwned,
{
    let payload = rmp_serde::to_vec_named(input).map_err(|e| {
        StorageError::Conductor(format!("encode {fn_name} input: {e}"))
    })?;
    let bytes = hc.call_zome("imagodei", fn_name, payload).await?;
    let output: O = rmp_serde::from_slice(&bytes).map_err(|e| {
        StorageError::Conductor(format!("decode {fn_name} output: {e}"))
    })?;
    Ok(output)
}
```

- [ ] **Step 2: Build to verify the helper compiles**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: `Finished` with no errors. The helper is currently unused; clippy may warn (`#[allow(dead_code)]` if needed for this commit, removed in Task 10).

- [ ] **Step 3: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs
git commit -m "storage(account): add generic forward_to_imagodei zome-call helper"
```

---

### Task 10: Implement and wire `forward_self_revocation`

**Why:** First of the four route un-stubs. `POST /api/v1/account/self-revocation` forwards to `imagodei::create_self_revocation`.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs`

- [ ] **Step 1: Read the M4 self-revocation feature file to confirm body shape**

Run:
```bash
grep -A20 "Scenario:" /projects/elohim/genesis/a2o/features/auth/recovery/recovery-m5-self-revoke.feature | head -40
```

Confirms the API body shape sent by Angular: `{ revokedKey: "uhCAk...", reason: "..." }`. Matches `CreateSelfRevocationInputView`.

- [ ] **Step 2: Add a zome-input wrapper struct with serde**

Add to `account.rs` (just below `forward_to_imagodei`):

```rust
// ---------------------------------------------------------------------------
// Phase 11: zome-input wrappers
// ---------------------------------------------------------------------------
//
// These match the imagodei coordinator zome's input structs exactly. We
// keep them in `account.rs` rather than `views.rs` because they are wire-
// internal — they do NOT cross the HTTP boundary.

#[derive(serde::Serialize)]
struct CreateSelfRevocationZomeInput {
    /// Holochain hashes serialize to bytes via the holo_hash crate; here we
    /// use a String form decoded by the conductor's hash deserializer. The
    /// zome's `CreateSelfRevocationInput` carries `AgentPubKey` which serdes
    /// from a base64 string when fed by an off-chain client.
    revoked_key: holo_hash::AgentPubKey,
    reason: String,
}

#[derive(serde::Deserialize)]
struct CreateSelfRevocationZomeOutput {
    revocation_id: String,
    action_hash: holo_hash::ActionHash,
}
```

Add `holo_hash` to Cargo.toml dependencies if not present. Run:

```bash
grep -n "^holo_hash\|^holochain" /projects/elohim/elohim/elohim-storage/Cargo.toml
```

If `holo_hash` is not listed but `holochain_client` is (with `lair_signing` feature), `holo_hash` may be re-exported via `holochain_client::HoloHash` — check the `holochain_client` crate or use `holochain_types::prelude::AgentPubKey` if that's the established pattern. If unclear: try `holo_hash = "0.5"` and verify against the version `holochain_client` uses; mismatch errors will guide.

- [ ] **Step 3: Add `forward_self_revocation` and wire the route**

Replace the stub at `account.rs:55-56`:

```rust
        // ── Self-revocation (zome write — Phase 11 bridge) ────────────────
        (Method::POST, "/self-revocation") => {
            handle_self_revocation(req, _hc_registry, pool).await
        }
```

(Rename `_hc_registry` → `hc_registry` in the `handle` signature now that it has a consumer.)

Add after `forward_to_imagodei`:

```rust
async fn handle_self_revocation(
    req: Request<Incoming>,
    hc_registry: Option<&Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(response::bad_request("missing X-Agent-Id and no active session")),
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei.as_ref() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    let body_bytes = read_request_body(req).await?;
    let input_view: crate::views::CreateSelfRevocationInputView =
        serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::BadRequest(format!("invalid request body: {e}")))?;

    let revoked_key = holo_hash::AgentPubKey::try_from(input_view.revoked_key.as_str())
        .map_err(|e| StorageError::BadRequest(format!("invalid revokedKey: {e}")))?;
    let zome_input = CreateSelfRevocationZomeInput {
        revoked_key,
        reason: input_view.reason,
    };

    match forward_to_imagodei::<_, CreateSelfRevocationZomeOutput>(
        hc,
        "create_self_revocation",
        &zome_input,
    )
    .await
    {
        Ok(out) => {
            let view = crate::views::CreateSelfRevocationOutputView {
                revocation_id: out.revocation_id,
                action_hash: out.action_hash.to_string(),
            };
            Ok(response::created(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}

/// Read the entire request body into a `Bytes`. Hyper streams bodies; we
/// collect to a `Vec<u8>` for serde decode.
async fn read_request_body(req: Request<Incoming>) -> Result<Bytes, StorageError> {
    use http_body_util::BodyExt;
    let body = req.into_body();
    let collected = body.collect().await.map_err(|e| {
        StorageError::BadRequest(format!("read request body: {e}"))
    })?;
    Ok(collected.to_bytes())
}
```

If `StorageError::BadRequest` doesn't exist as a variant, find the existing pattern by running:
```bash
grep -n "enum StorageError" /projects/elohim/elohim/elohim-storage/src/error.rs
```
and use the closest matching variant (e.g. `Validation` or `InvalidInput`). Adjust the calls accordingly.

- [ ] **Step 4: Build to verify compilation**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
```

Expected: `Finished` — `holo_hash::AgentPubKey::try_from(&str)` should compile (the `From<&str>` impl decodes base64). If it doesn't, swap to `holo_hash::AgentPubKey::from_raw_39_panicky(...)` or use the conversion function from a sibling module — check signing.rs for the established pattern:
```bash
grep -n "AgentPubKey::" /projects/elohim/elohim/elohim-storage/src/signing.rs
```

- [ ] **Step 5: Run unit tests to verify nothing regressed**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account
```

Expected: all account tests pass (the existing M5 + new mode-gate + error-mapping). The new forwarder is not unit-tested at this layer (verified by a2o on Jenkins).

- [ ] **Step 6: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs elohim/elohim-storage/Cargo.toml
git commit -m "storage(account): unstub POST /api/v1/account/self-revocation"
```

---

### Task 11: Implement and wire `forward_revocation_vote`

**Why:** `POST /api/v1/account/recovery/:id/vote` forwards to `imagodei::submit_revocation_vote`. The `:id` is extracted from the URL path; body carries `{ approved, attestation }`.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs`

- [ ] **Step 1: Replace the stub at the route arm**

In the dispatcher (around line 62 after Task 10's edits):

```rust
        // ── Recovery vote (zome write — Phase 11 bridge) ──────────────────
        (Method::POST, p) if p.starts_with("/recovery/") && p.ends_with("/vote") => {
            let revocation_id = p
                .trim_start_matches("/recovery/")
                .trim_end_matches("/vote");
            if revocation_id.is_empty() {
                return Ok(response::bad_request("missing revocation id in URL path"));
            }
            handle_revocation_vote(req, hc_registry, pool, revocation_id.to_string()).await
        }
```

- [ ] **Step 2: Add zome-input wrappers**

Add below the `CreateSelfRevocation*` wrappers:

```rust
#[derive(serde::Serialize)]
struct SubmitRevocationVoteZomeInput {
    revocation_id: String,
    approved: bool,
    attestation: String,
}

#[derive(serde::Deserialize)]
struct SubmitRevocationVoteZomeOutput {
    vote_id: String,
    current_votes: u32,
    required_votes: u32,
    threshold_now_reached: bool,
}
```

- [ ] **Step 3: Add the handler**

Add below `handle_self_revocation`:

```rust
async fn handle_revocation_vote(
    req: Request<Incoming>,
    hc_registry: Option<&Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
    revocation_id: String,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(response::bad_request("missing X-Agent-Id and no active session")),
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei.as_ref() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    let body_bytes = read_request_body(req).await?;
    let input_view: crate::views::SubmitRevocationVoteInputView =
        serde_json::from_slice(&body_bytes)
            .map_err(|e| StorageError::BadRequest(format!("invalid request body: {e}")))?;

    let zome_input = SubmitRevocationVoteZomeInput {
        revocation_id,
        approved: input_view.approved,
        attestation: input_view.attestation,
    };

    match forward_to_imagodei::<_, SubmitRevocationVoteZomeOutput>(
        hc,
        "submit_revocation_vote",
        &zome_input,
    )
    .await
    {
        Ok(out) => {
            let view = crate::views::SubmitRevocationVoteOutputView {
                vote_id: out.vote_id,
                current_votes: out.current_votes,
                required_votes: out.required_votes,
                threshold_now_reached: out.threshold_now_reached,
            };
            Ok(response::ok(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}
```

- [ ] **Step 4: Build + test**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account
```

Expected: build OK, all account tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs
git commit -m "storage(account): unstub POST /api/v1/account/recovery/:id/vote"
```

---

### Task 12: Implement and wire `forward_add_portal_host`

**Why:** `POST /api/v1/account/portal-hosts` forwards to `imagodei::add_portal_host`. Already has an `AddPortalHostInputView` from M5.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs`

- [ ] **Step 1: Replace the stub at the route arm**

In the dispatcher:

```rust
        // ── Portal hosts ──────────────────────────────────────────────────
        (Method::GET, "/portal-hosts") => get_portal_hosts(req, pool).await,
        (Method::POST, "/portal-hosts") => handle_add_portal_host(req, hc_registry, pool).await,
        (Method::DELETE, p) if p.starts_with("/portal-hosts/") => {
            let url_b64 = p.trim_start_matches("/portal-hosts/");
            if url_b64.is_empty() {
                return Ok(response::bad_request("missing url_b64 in URL path"));
            }
            handle_remove_portal_host(req, hc_registry, pool, url_b64.to_string()).await
        }
```

(The DELETE arm is wired now but its handler is added in Task 13.)

- [ ] **Step 2: Add zome-input wrappers**

Add below the previous wrappers:

```rust
#[derive(serde::Serialize)]
struct AddPortalHostZomeInput {
    host_url: String,
    label: Option<String>,
    /// One of "Public", "Trusted", "Private" — the zome enum's ts-rs
    /// representation. Defaults to "Trusted" when None at the InputView.
    reach: Option<String>,
}
```

The zome's `PortalHostReach` enum serializes by variant name. The InputView field is already `Option<String>` — pass through.

- [ ] **Step 3: Add the handler**

Add below `handle_revocation_vote`:

```rust
async fn handle_add_portal_host(
    req: Request<Incoming>,
    hc_registry: Option<&Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(response::bad_request("missing X-Agent-Id and no active session")),
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei.as_ref() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    let body_bytes = read_request_body(req).await?;
    let input_view: crate::views::AddPortalHostInputView = serde_json::from_slice(&body_bytes)
        .map_err(|e| StorageError::BadRequest(format!("invalid request body: {e}")))?;

    let zome_input = AddPortalHostZomeInput {
        host_url: input_view.host_url,
        label: input_view.label,
        reach: input_view.reach,
    };

    match forward_to_imagodei::<_, holo_hash::ActionHash>(
        hc,
        "add_portal_host",
        &zome_input,
    )
    .await
    {
        Ok(action_hash) => {
            let view = crate::views::AddPortalHostOutputView {
                action_hash: action_hash.to_string(),
            };
            Ok(response::created(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}
```

- [ ] **Step 4: Build + test**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account
```

Expected: build OK, tests pass.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs
git commit -m "storage(account): unstub POST /api/v1/account/portal-hosts"
```

---

### Task 13: Implement `forward_remove_portal_host`, delete `zome_bridge_not_yet_wired`, delete stub tests

**Why:** Last route un-stubbed (`DELETE /api/v1/account/portal-hosts/:url_b64`). With all four routes live, the `zome_bridge_not_yet_wired` function is dead code; remove it. The two `zome_bridge_*` unit tests test the stub that no longer exists; remove them.

**Files:**
- Modify: `elohim/elohim-storage/src/api/account.rs`

- [ ] **Step 1: Add the remove-portal-host handler**

Add below `handle_add_portal_host`:

```rust
async fn handle_remove_portal_host(
    req: Request<Incoming>,
    hc_registry: Option<&Arc<crate::hc_client_registry::HcClientRegistry>>,
    pool: &DbPool,
    url_b64: String,
) -> Result<Response<Full<Bytes>>, StorageError> {
    let mut conn = get_conn(pool)?;
    let agent_key = match extract_agent_key(&req, &mut conn)? {
        Some(k) => k,
        None => return Ok(response::bad_request("missing X-Agent-Id and no active session")),
    };

    let registry = match hc_registry {
        Some(r) => r,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };
    let hc = match registry.imagodei.as_ref() {
        Some(h) => h,
        None => return Ok(response_503_imagodei_bridge_offline()),
    };

    if let Err(resp) = verify_caller_owns_cell(hc.as_ref(), &agent_key) {
        return Ok(resp);
    }

    // The zome's `remove_portal_host` takes a `String` (the URL itself),
    // so we URL-safe base64 decode the path segment back to the URL.
    use base64::Engine;
    let host_url_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(url_b64.as_bytes())
        .map_err(|e| StorageError::BadRequest(format!("invalid url_b64: {e}")))?;
    let host_url = String::from_utf8(host_url_bytes)
        .map_err(|e| StorageError::BadRequest(format!("url_b64 is not valid UTF-8: {e}")))?;

    // The zome returns `()` — encode an empty serde value as the expected output
    // type, then translate to the OutputView with `deleted: true`.
    match forward_to_imagodei::<_, ()>(hc, "remove_portal_host", &host_url).await {
        Ok(()) => {
            let view = crate::views::RemovePortalHostOutputView { deleted: true };
            Ok(response::ok(&view))
        }
        Err(e) => Ok(map_zome_err_to_http(&e)),
    }
}
```

If `base64` is not in `Cargo.toml`:
```bash
grep -n "^base64" /projects/elohim/elohim/elohim-storage/Cargo.toml
```
If missing, add `base64 = "0.22"`.

- [ ] **Step 2: Delete `zome_bridge_not_yet_wired` and its module-level doc**

In `account.rs`, remove the `zome_bridge_not_yet_wired` function (currently at line ~495) and the surrounding section comment block (lines ~484-494). The whole block:

```rust
// ---------------------------------------------------------------------------
// Phase 11 stub: zome bridge not yet wired
// ---------------------------------------------------------------------------

/// Returns 503 with a machine-readable body indicating ...
fn zome_bridge_not_yet_wired(route_hint: &str) -> Response<Full<Bytes>> {
    ...
}
```

— delete entirely.

- [ ] **Step 3: Delete the two stub unit tests**

In `mod tests`, remove `zome_bridge_stub_returns_503` and `zome_bridge_all_stub_routes_return_503` (currently lines ~552-555 and ~613-628). Keep all other tests intact.

- [ ] **Step 4: Build + test the full suite**

Run:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib api::account
```

Expected: build OK, clippy clean (`map_zome_err_to_http` should no longer warn `dead_code`), all account tests pass — the surviving M5 tests (`portal_host_view_serialises_camel_case`, `revocation_view_threshold_bool_coercion`) plus the new mode-gate, error-mapping, and CellOwner tests.

- [ ] **Step 5: Commit**

```bash
git add elohim/elohim-storage/src/api/account.rs elohim/elohim-storage/Cargo.toml
git commit -m "storage(account): unstub DELETE /portal-hosts/:url_b64; drop zome_bridge_not_yet_wired + stub tests"
```

---

### Task 14: Remove the five `@phase11-pending` tags

**Why:** With the four routes live, the five a2o scenarios pinned to `@phase11-pending` should run on Jenkins. Tag removal is the visible deliverable that turns Phase 11 from "stubbed" to "real."

**Files:**
- Modify: `genesis/a2o/features/auth/recovery/recovery-m5-self-revoke.feature` (1 tag)
- Modify: `genesis/a2o/features/auth/recovery/recovery-m5-vote-as-emergency-contact.feature` (2 tags)
- Modify: `genesis/a2o/features/auth/recovery/recovery-m5-portal-host-discovery.feature` (1 tag)
- Modify: `genesis/a2o/features/auth/recovery/recovery-m5-defender-role-gate.feature` (1 tag)

- [ ] **Step 1: Find every `@phase11-pending` tag**

Run:
```bash
grep -rn "@phase11-pending" /projects/elohim/genesis/a2o/features/auth/recovery/
```

Expected: 5 hits across 4 files. If the count differs, reconcile against the spec's table — investigate any extra/missing.

- [ ] **Step 2: Remove each tag**

For each hit, open the file and delete the `@phase11-pending` tag from the tag line. Tags appear on lines like:

```gherkin
@m5-recovery @phase11-pending @self-revoke
```

becomes:

```gherkin
@m5-recovery @self-revoke
```

Preserve other tags. Do NOT delete the scenarios — only the `@phase11-pending` tag.

- [ ] **Step 3: Inspect the step definitions in case any assert the 503 message**

Run:
```bash
grep -n "PHASE_11_PENDING\|conductor bridge not yet wired\|503" /projects/elohim/genesis/a2o/steps/ui/account-m5.steps.ts
```

If any step expects `503` or `PHASE_11_PENDING` for one of the four routes, that step needs updating. Inspect each result — they may be testing the success path now and only expect 503 on the BROWSER_WRITE_PATH_PENDING / IMAGODEI_BRIDGE_OFFLINE branches.

If a step needs updating to assert the new success contract, update it to match the OutputView shapes from Task 8 (e.g., `revocationId`, `actionHash`, `currentVotes`, etc.).

- [ ] **Step 4: Verify the files parse as Gherkin**

The simplest verification: search for orphan `@` (e.g., trailing spaces from tag deletion):

```bash
grep -nE "^\s*@\s*$|@\s+$" /projects/elohim/genesis/a2o/features/auth/recovery/recovery-m5-*.feature
```

Expected: no matches.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/features/auth/recovery/ genesis/a2o/steps/ui/account-m5.steps.ts
git commit -m "a2o(recovery-m5): remove @phase11-pending tags now that storage bridge is live"
```

---

### Task 15: Final pre-push gate

**Why:** Ensure the full crate is green before merging to dev. Per `feedback_dev_branch_no_pr`, dev is the integration target and a local merge will follow this verification.

**Files:** none — verification only.

- [ ] **Step 1: Run the full pre-push set**

Run:
```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo fmt --check
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

Expected: fmt clean, clippy clean, build green, all lib tests pass.

If `cargo fmt --check` reports drift, run `cargo fmt` and stage the diff in a fixup commit:
```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo fmt
git add elohim/elohim-storage/src/
git commit -m "storage(fmt): rustfmt drift on Phase 11 forwarder helpers"
```

- [ ] **Step 2: Confirm the TypeScript bindings are committed**

Run:
```bash
git status --short elohim/sdk/storage-client-ts/src/generated/
```

Expected: empty (all generated files were committed in Task 8). If anything is dirty, stage and commit:
```bash
git add elohim/sdk/storage-client-ts/src/generated/
git commit -m "storage(generated-ts): refresh bindings after Phase 11 view changes"
```

- [ ] **Step 3: Confirm the branch is clean**

Run:
```bash
git status --short
git log --oneline dev..HEAD
```

Expected: working tree clean (only ambient drift in unrelated paths like Cargo.lock or elohim/brit submodule). Phase 11 commits visible in the log.

- [ ] **Step 4: Sanity-grep for leftover `PHASE_11_PENDING`**

Run:
```bash
grep -rn "PHASE_11_PENDING\|zome_bridge_not_yet_wired" /projects/elohim/elohim/elohim-storage/src/
grep -rn "@phase11-pending" /projects/elohim/genesis/a2o/features/
```

Expected: no matches in either grep — all references removed.

- [ ] **Step 5: Push (after sprint orchestrator confirms)**

```bash
git push origin feature/storage-phase-11-zome-forwarding-bridge 2>&1 | tail -10
```

Expected: pre-push hook runs the same gates and exits 0; push succeeds. The branch is then ready for local merge to `dev` (per `feedback_dev_branch_no_pr`).

---

## Cross-task verification (run before declaring done)

After all 15 tasks complete, run a single sanity sweep:

```bash
# 1. No PHASE_11_PENDING / zome_bridge_not_yet_wired anywhere in storage
grep -rn "PHASE_11_PENDING\|zome_bridge_not_yet_wired" /projects/elohim/elohim/elohim-storage/src/

# 2. No @phase11-pending tags left in features
grep -rn "@phase11-pending" /projects/elohim/genesis/a2o/features/

# 3. Both new 503 codes are exposed
grep -n "BROWSER_WRITE_PATH_PENDING\|IMAGODEI_BRIDGE_OFFLINE" /projects/elohim/elohim/elohim-storage/src/api/account.rs

# 4. All four routes are wired (no stub call remains)
grep -n "handle_self_revocation\|handle_revocation_vote\|handle_add_portal_host\|handle_remove_portal_host" /projects/elohim/elohim/elohim-storage/src/api/account.rs

# 5. Pre-push set still green
cd /projects/elohim/elohim/elohim-storage \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo fmt --check \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings \
  && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib
```

If any of (1)–(4) returns unexpected matches/misses, investigate before pushing. (5) must exit 0.

---

## Risks and recovery hints

- **Task 4 (heartbeat refactor) breaks heartbeat at startup** — symptom: peer-status entries stop appearing. Recovery: revert Task 4's commit and re-apply with the original heartbeat construction wrapped in a one-shot adapter. The behavior is preserved by construction; if it breaks, the failure is in the rebinding of `hc` from registry's `Arc`. Inspect the `tokio::spawn` argument lifetimes.
- **Task 8 ts-rs codegen produces stale TypeScript** — symptom: Angular layer can't import `CreateSelfRevocationOutputView`. Recovery: re-run `cargo test export_bindings` and confirm the generated file paths match `INTERFACE_FILES` in `elohim/sdk/schemas/scripts/codegen-ts.mjs`.
- **Task 10 holo_hash dependency mismatch** — symptom: `holo_hash::AgentPubKey::try_from(&str)` doesn't exist or fails to compile. Recovery: read `signing.rs` for the established AgentPubKey conversion pattern and apply it. The `holochain_client` crate may re-export holo_hash with a specific feature flag.
- **Task 14 a2o step assertions fail** — symptom: scenarios fail because steps assert the old 503 contract. Recovery: update the step in `account-m5.steps.ts` to assert the new OutputView fields. Re-run the feature-file `grep -n "503\|PHASE_11_PENDING"` search to find every site.
- **`agent_info()` does NOT match cell-owner under signing credentials** (the spec's central premise) — symptom: zome rejects with "caller does not control revoked_key" or "caller is not an active emergency contact" even when the M5 Angular UI has the right key. Recovery: BLOCK and report. The fix is M6-scope and outside Phase 11.

---

## DRY / YAGNI / TDD audit

- **DRY:** the four route handlers each have ~30-40 lines and share `verify_caller_owns_cell`, `forward_to_imagodei`, `map_zome_err_to_http`, and `read_request_body`. Further consolidation (e.g., a macro for the `match registry/hc/verify` ladder) would obscure the per-route parameter passing and is rejected.
- **YAGNI:** no reconnect logic, no connection pool, no rate limiting, no typed conductor errors, no projection echo wait. All deferred per the spec's "Out of scope."
- **TDD:** tests-first for `verify_caller_owns_cell`, `map_zome_err_to_http`, and the `CellOwner` trait. Forwarders are not unit-tested (they require a live conductor); their verification is a2o on Jenkins.
- **Frequent commits:** 13 implementation commits + a2o tag-removal + final-gate fixup, one per task. Each commit is independently buildable and revertable.
