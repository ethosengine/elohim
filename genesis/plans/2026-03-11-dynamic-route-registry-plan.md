# Dynamic Route Registry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Replace 13 hardcoded proxy files in doorway with a single dynamic route registry, where the steward's elohim-storage self-registers as the first peer on boot.

**Architecture:** On startup, doorway queries its elohim-storage for a route manifest, registers those routes into the existing RouteRegistry, and uses the compiled route table to handle all `/api/v1/*` traffic. The 13 identical proxy files are deleted. Built-in routes (health, auth, admin, conductor proxy) remain hardcoded.

**Tech Stack:** Rust (doorway-service, elohim-storage), doorway-client crate (DoorwayRoutes types), hyper HTTP, reqwest for proxying, tokio async

**Design Doc:** `genesis/plans/2026-03-11-dynamic-route-registry-design.md`

---

## Task 1: Add Route Manifest Endpoint to elohim-storage

The steward's elohim-storage needs to declare its route surface so doorway can discover it.

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml` — add doorway-client dependency
- Modify: `elohim/elohim-storage/src/http.rs` — add `/manifest` route match + handler
- Test: Manual curl verification (elohim-storage has no test harness for HTTP routes)

**Step 1: Add doorway-client dependency**

In `elohim/elohim-storage/Cargo.toml`, add:
```toml
doorway-client = { path = "../../crates/doorway-client" }
```

**Step 2: Add manifest handler to http.rs**

In `elohim/elohim-storage/src/http.rs`, find the route matching section (~line 273). Add a match arm for `GET /manifest` that returns a `DoorwayRoutes` JSON response.

The manifest should declare all routes elohim-storage currently serves under `/api/v1/*` and `/db/*`. Build this from the existing route match arms in the same file — every `(Method::GET, p) if p.starts_with("/api/v1/mastery")` etc. becomes a route declaration.

Key routes to include:
- `/api/v1/mastery/*` — GET (read), POST (write, auth required)
- `/api/v1/mastery/engagement` — POST (auth required)
- `/api/v1/mastery/assessment` — POST (auth required)
- `/api/v1/mastery/batch` — POST (auth required)
- `/api/v1/mastery/stats` — GET
- `/api/v1/mastery/path/{pathId}` — GET
- `/db/content/*` — GET (cacheable)
- `/db/paths/*` — GET (cacheable)
- `/db/relationships/*` — GET (cacheable)
- `/db/stats` — GET
- All other `/api/v1/*` routes currently handled by elohim-storage

Use the `DoorwayRoutes` and `Route` types from `doorway_client`:
```rust
use doorway_client::{DoorwayRoutesBuilder, Route as DoorwayRoute, BlobProxyConfig};

fn build_manifest() -> doorway_client::DoorwayRoutes {
    DoorwayRoutesBuilder::new()
        // Mastery (read + write)
        .route(DoorwayRoute::get("/api/v1/mastery/{contentId}")
            .handler("get_mastery")
            .cache_ttl(300)
            .build())
        .route(DoorwayRoute::get("/api/v1/mastery")
            .handler("list_mastery")
            .cache_ttl(60)
            .build())
        .route(DoorwayRoute::post("/api/v1/mastery")
            .handler("initialize_mastery")
            .auth_required()
            .build())
        .route(DoorwayRoute::post("/api/v1/mastery/engagement")
            .handler("record_engagement")
            .auth_required()
            .build())
        // ... remaining routes from http.rs match arms
        .with_blobs_at("/blob")
        .build()
}
```

**Step 3: Test manifest endpoint**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --release
# Start storage, then:
curl http://localhost:8090/manifest | jq
```

Expected: JSON matching `DoorwayRoutes` schema with all routes declared.

**Step 4: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add /manifest endpoint declaring route surface for doorway discovery"
```

---

## Task 2: Add RouteRegistry to AppState and Initialize on Boot

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs:43-106` — add `route_registry` field to `AppState`
- Modify: `doorway/doorway-service/src/server/http.rs` — find `AppState::with_pool` and `AppState::with_services` constructors, add registry initialization
- Modify: `doorway/doorway-service/src/main.rs:211-214` — wire registry into state

**Step 1: Add route_registry to AppState struct**

In `doorway/doorway-service/src/server/http.rs`, add to the `AppState` struct (~line 43):
```rust
use crate::services::{RouteRegistry, RouteRegistryConfig};

// In AppState struct:
/// Dynamic route registry for peer-discovered routes
pub route_registry: Arc<RouteRegistry>,
```

**Step 2: Initialize in AppState constructors**

Find `with_pool` and `with_services` methods. Add:
```rust
route_registry: Arc::new(RouteRegistry::with_defaults()),
```

**Step 3: Verify it compiles**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | head -20
```

**Step 4: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): add RouteRegistry to AppState"
```

---

## Task 3: Self-Register Steward's Storage on Boot

**Files:**
- Modify: `doorway/doorway-service/src/main.rs` — add steward self-registration after AppState construction
- Modify: `doorway/doorway-service/src/services/route_registry.rs` — add `register_steward_peer` method and `RouteSource::StewardPeer`

**Step 1: Add StewardPeer route source**

In `route_registry.rs`, add to `RouteSource` enum (~line 96):
```rust
/// The doorway operator's own elohim-storage (first peer)
StewardPeer { storage_url: String },
```

Add to `RouteTarget` enum (~line 110):
```rust
/// Proxy to a peer's elohim-storage endpoint
StorageProxy { endpoint: String },
```

**Step 2: Add steward registration method**

In `RouteRegistry`, add:
```rust
/// Register the steward's elohim-storage as the first peer.
///
/// Fetches the route manifest from storage and compiles routes.
/// This is the same mechanism any peer uses — the steward is just first.
pub async fn register_steward_peer(
    &self,
    storage_url: &str,
) -> Result<usize, String> {
    // Fetch manifest
    let manifest_url = format!("{}/manifest", storage_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let response = client.get(&manifest_url)
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| format!("Failed to fetch manifest from {manifest_url}: {e}"))?;

    if !response.status().is_success() {
        return Err(format!("Manifest returned {}", response.status()));
    }

    let routes: doorway_client::DoorwayRoutes = response.json()
        .await
        .map_err(|e| format!("Failed to parse manifest: {e}"))?;

    let route_count = routes.routes.len();

    // Register as steward peer using existing DNA route infrastructure
    // (steward is conceptually a "local DNA" — same compilation path)
    self.register_steward_routes(storage_url, routes).await;

    Ok(route_count)
}

/// Compile steward routes into the route table
async fn register_steward_routes(
    &self,
    storage_url: &str,
    routes: doorway_client::DoorwayRoutes,
) {
    let mut compiled = Vec::new();

    for route in &routes.routes {
        compiled.push(CompiledRoute {
            method: route.method,
            path: route.path.clone(),
            source: RouteSource::StewardPeer {
                storage_url: storage_url.to_string(),
            },
            target: RouteTarget::StorageProxy {
                endpoint: storage_url.to_string(),
            },
            auth_required: route.auth_required,
            cache_ttl_secs: route.cache_ttl_secs,
            rate_limit_rpm: route.rate_limit_rpm,
        });
    }

    // Compile blob proxy if declared
    if let Some(ref blob_config) = routes.blob_proxy {
        if blob_config.enabled {
            compiled.push(CompiledRoute {
                method: doorway_client::HttpMethod::Get,
                path: format!("{}/:hash", blob_config.base_path),
                source: RouteSource::StewardPeer {
                    storage_url: storage_url.to_string(),
                },
                target: RouteTarget::BlobProxy {
                    config: blob_config.clone(),
                },
                auth_required: false,
                cache_ttl_secs: blob_config.cache_ttl_secs,
                rate_limit_rpm: 0,
            });
        }
    }

    let count = compiled.len();
    let mut all_routes = self.compiled_routes.write().await;
    all_routes.extend(compiled);

    tracing::info!(
        route_count = count,
        storage_url = %storage_url,
        "Steward peer registered"
    );
}
```

**Step 3: Call self-registration in main.rs**

In `main.rs`, after AppState is constructed (~line 216), add:
```rust
// Self-register steward's elohim-storage as first peer
if let Some(ref storage_url) = state.args.storage_url {
    match state.route_registry.register_steward_peer(storage_url).await {
        Ok(count) => {
            tracing::info!(
                routes = count,
                storage_url = %storage_url,
                "Steward storage self-registered as first peer"
            );
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "Failed to self-register steward storage — dynamic routes unavailable"
            );
        }
    }
}
```

**Step 4: Verify it compiles**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | head -20
```

**Step 5: Commit**

```bash
git add doorway/doorway-service/src/services/route_registry.rs doorway/doorway-service/src/main.rs
git commit -m "feat(doorway): self-register steward storage as first peer on boot"
```

---

## Task 4: Add Generic Storage Proxy Function

The 13 proxy files all do the same thing. Extract a single reusable proxy function.

**Files:**
- Create: `doorway/doorway-service/src/routes/storage_proxy.rs`
- Modify: `doorway/doorway-service/src/routes/mod.rs` — add module + re-export

**Step 1: Write the generic proxy**

Create `doorway/doorway-service/src/routes/storage_proxy.rs`:

```rust
//! Generic storage proxy — forwards requests to a peer's elohim-storage.
//!
//! Used by the route registry to proxy matched routes to their target endpoint.
//! Replaces the 13 identical per-domain proxy files.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Method, Request, Response, StatusCode};
use tracing::{debug, warn};

/// Forward an HTTP request to a peer's elohim-storage endpoint.
///
/// Preserves: method, path, query string, content-type, authorization, body.
/// Adds: Cross-Origin-Resource-Policy header for Angular COEP compatibility.
pub async fn forward_to_storage(
    req: Request<Incoming>,
    storage_url: &str,
    path: &str,
) -> Response<Full<Bytes>> {
    let storage_endpoint = format!("{}{}", storage_url.trim_end_matches('/'), path);

    let query = req.uri().query();
    let full_url = match query {
        Some(q) => format!("{storage_endpoint}?{q}"),
        None => storage_endpoint,
    };

    let method = req.method().clone();
    debug!(method = %method, url = %full_url, "Forwarding to peer storage");

    let client = reqwest::Client::new();
    let mut builder = match method {
        Method::GET => client.get(&full_url),
        Method::POST => client.post(&full_url),
        Method::PUT => client.put(&full_url),
        Method::DELETE => client.delete(&full_url),
        Method::HEAD => client.head(&full_url),
        Method::PATCH => client.patch(&full_url),
        _ => {
            return method_not_allowed();
        }
    };

    // Forward headers
    if let Some(ct) = req.headers().get("content-type") {
        if let Ok(ct_str) = ct.to_str() {
            builder = builder.header("Content-Type", ct_str);
        }
    }
    if let Some(auth) = req.headers().get("authorization") {
        if let Ok(auth_str) = auth.to_str() {
            builder = builder.header("Authorization", auth_str);
        }
    }

    // Forward body for write methods
    if matches!(method, Method::POST | Method::PUT | Method::PATCH) {
        match req.collect().await {
            Ok(collected) => {
                builder = builder.body(collected.to_bytes().to_vec());
            }
            Err(e) => {
                warn!(error = %e, "Failed to read request body");
                return error_response(
                    StatusCode::BAD_REQUEST,
                    &format!("Failed to read request body: {e}"),
                );
            }
        }
    }

    // Send and relay response
    match builder.send().await {
        Ok(response) => {
            let status = response.status();
            let content_type = response
                .headers()
                .get("content-type")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("application/json")
                .to_string();

            match response.bytes().await {
                Ok(body) => Response::builder()
                    .status(StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::OK))
                    .header("Content-Type", content_type)
                    .header("Cross-Origin-Resource-Policy", "cross-origin")
                    .body(Full::new(Bytes::from(body.to_vec())))
                    .unwrap(),
                Err(e) => {
                    warn!(error = %e, "Failed to read storage response body");
                    error_response(
                        StatusCode::BAD_GATEWAY,
                        &format!("Failed to read storage response: {e}"),
                    )
                }
            }
        }
        Err(e) => {
            warn!(error = %e, path = %path, "Failed to forward request to storage");
            error_response(
                StatusCode::BAD_GATEWAY,
                &format!("Failed to connect to storage: {e}"),
            )
        }
    }
}

fn method_not_allowed() -> Response<Full<Bytes>> {
    Response::builder()
        .status(StatusCode::METHOD_NOT_ALLOWED)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(r#"{"error": "Method not allowed"}"#)))
        .unwrap()
}

fn error_response(status: StatusCode, msg: &str) -> Response<Full<Bytes>> {
    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(format!(r#"{{"error": "{msg}"}}"#))))
        .unwrap()
}
```

**Step 2: Register in mod.rs**

In `doorway/doorway-service/src/routes/mod.rs`, add:
```rust
pub mod storage_proxy;
pub use storage_proxy::forward_to_storage;
```

**Step 3: Verify it compiles**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | head -20
```

**Step 4: Commit**

```bash
git add doorway/doorway-service/src/routes/storage_proxy.rs doorway/doorway-service/src/routes/mod.rs
git commit -m "feat(doorway): add generic storage proxy for registry-routed requests"
```

---

## Task 5: Add match_request to RouteRegistry

`find_routes_for_path` only matches on path. We need method-aware matching.

**Files:**
- Modify: `doorway/doorway-service/src/services/route_registry.rs`

**Step 1: Write the test**

```rust
#[tokio::test]
async fn test_match_request_filters_by_method() {
    let registry = RouteRegistry::with_defaults();

    let routes = DoorwayRoutesBuilder::new()
        .route(DoorwayRoute::get("/api/v1/mastery/{id}")
            .handler("get_mastery")
            .cache_ttl(300)
            .build())
        .route(DoorwayRoute::post("/api/v1/mastery")
            .handler("create_mastery")
            .build())
        .build();

    registry.register_dna_routes("dna-1", "lamad", "content_store", routes).await;

    // GET should match the get route
    let get_matches = registry.match_request(HttpMethod::Get, "/api/v1/mastery/abc").await;
    assert_eq!(get_matches.len(), 1);
    assert_eq!(get_matches[0].target_handler(), Some("get_mastery"));

    // POST should match the post route
    let post_matches = registry.match_request(HttpMethod::Post, "/api/v1/mastery").await;
    assert_eq!(post_matches.len(), 1);

    // DELETE should match nothing
    let delete_matches = registry.match_request(HttpMethod::Delete, "/api/v1/mastery").await;
    assert!(delete_matches.is_empty());
}
```

**Step 2: Implement match_request**

```rust
/// Match a request by HTTP method and path.
///
/// Returns matching routes filtered by both method and path pattern.
pub async fn match_request(
    &self,
    method: HttpMethod,
    path: &str,
) -> Vec<CompiledRoute> {
    self.compiled_routes
        .read()
        .await
        .iter()
        .filter(|r| r.method == method && path_matches(&r.path, path))
        .cloned()
        .collect()
}
```

Add helper on `CompiledRoute`:
```rust
impl CompiledRoute {
    /// Get the handler name if target is a ZomeCall
    pub fn target_handler(&self) -> Option<&str> {
        match &self.target {
            RouteTarget::ZomeCall { fn_name, .. } => Some(fn_name),
            _ => None,
        }
    }

    /// Get the storage endpoint if target is a StorageProxy
    pub fn storage_endpoint(&self) -> Option<&str> {
        match &self.target {
            RouteTarget::StorageProxy { endpoint } => Some(endpoint),
            _ => None,
        }
    }
}
```

**Step 3: Run tests**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo test --lib route_registry -- --nocapture
```

**Step 4: Commit**

```bash
git add doorway/doorway-service/src/services/route_registry.rs
git commit -m "feat(doorway): add method-aware match_request to RouteRegistry"
```

---

## Task 6: Wire Registry Lookup into http.rs

Replace the 13 hardcoded `/api/v1/*` match arms with a single registry lookup.

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs:1187-1304` — replace domain API routes section

**Step 1: Replace the hardcoded domain API routes**

Find the section starting with comment `// Domain API Routes (v1)` (~line 1187). Replace everything from there to `// Not found` (~line 1303) with:

```rust
// ====================================================================
// Dynamic Routes — resolved from RouteRegistry
// ====================================================================
// Routes registered by peers (steward's storage, external agents).
// The steward's elohim-storage self-registers on boot.
// Additional peers register via POST /doorway/register.
(_, p) if p.starts_with("/api/v1/") => {
    let method = match req.method() {
        &Method::GET => doorway_client::HttpMethod::Get,
        &Method::POST => doorway_client::HttpMethod::Post,
        &Method::PUT => doorway_client::HttpMethod::Put,
        &Method::DELETE => doorway_client::HttpMethod::Delete,
        &Method::PATCH => doorway_client::HttpMethod::Patch,
        &Method::HEAD => doorway_client::HttpMethod::Head,
        _ => {
            return Ok(to_boxed(
                Response::builder()
                    .status(StatusCode::METHOD_NOT_ALLOWED)
                    .header("Content-Type", "application/json")
                    .body(Full::new(Bytes::from(
                        r#"{"error": "Method not allowed"}"#,
                    )))
                    .unwrap(),
            ));
        }
    };

    let matches = state.route_registry.match_request(method, p).await;

    if let Some(route) = matches.first() {
        match &route.target {
            crate::services::RouteTarget::StorageProxy { endpoint } => {
                return Ok(to_boxed(
                    routes::forward_to_storage(req, endpoint, p).await,
                ));
            }
            crate::services::RouteTarget::AgentProxy {
                endpoint,
                path_suffix,
                ..
            } => {
                let proxy_path = path_suffix
                    .as_ref()
                    .map(|s| format!("{s}{}", &p[p.find('/').unwrap_or(0)..]))
                    .unwrap_or_else(|| p.to_string());
                return Ok(to_boxed(
                    routes::forward_to_storage(req, endpoint, &proxy_path).await,
                ));
            }
            crate::services::RouteTarget::ZomeCall { .. } => {
                // Future: route to conductor via worker pool
                return Ok(to_boxed(
                    Response::builder()
                        .status(StatusCode::NOT_IMPLEMENTED)
                        .header("Content-Type", "application/json")
                        .body(Full::new(Bytes::from(
                            r#"{"error": "Zome call routing not yet implemented"}"#,
                        )))
                        .unwrap(),
                ));
            }
            _ => {}
        }
    }

    // No route registered — 404
    to_boxed(not_found_response(&path))
}
```

**Note:** Keep the collectives special case ABOVE this catch-all if `collectives.rs` has cross-domain path rewriting. Check if `/api/v1/humans/{id}/collectives` needs to stay as a hardcoded match. If so, keep it above the registry catch-all:
```rust
// Cross-domain collectives route (until path rewriting moves to registry)
(Method::GET, p) if p.starts_with("/api/v1/humans/") && p.ends_with("/collectives") => {
    return Ok(to_boxed(
        routes::handle_collectives_request(req, Arc::clone(&state), p).await,
    ));
}
```

**Step 2: Verify it compiles**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | head -30
```

**Step 3: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs
git commit -m "feat(doorway): route /api/v1/* through registry instead of hardcoded match arms"
```

---

## Task 7: Delete the 13 Identical Proxy Files

Now that routing goes through the registry, the copy-paste proxy files are dead code.

**Files:**
- Delete: `doorway/doorway-service/src/routes/governance.rs`
- Delete: `doorway/doorway-service/src/routes/attestations.rs`
- Delete: `doorway/doorway-service/src/routes/contributors.rs`
- Delete: `doorway/doorway-service/src/routes/steward.rs`
- Delete: `doorway/doorway-service/src/routes/presence.rs`
- Delete: `doorway/doorway-service/src/routes/economic_events.rs`
- Delete: `doorway/doorway-service/src/routes/exchange.rs`
- Delete: `doorway/doorway-service/src/routes/custodians.rs`
- Delete: `doorway/doorway-service/src/routes/compute.rs`
- Delete: `doorway/doorway-service/src/routes/flow_planning.rs`
- Delete: `doorway/doorway-service/src/routes/stewarded_resources.rs`
- Delete: `doorway/doorway-service/src/routes/stewardship.rs`
- Delete: `doorway/doorway-service/src/routes/account.rs`
- Modify: `doorway/doorway-service/src/routes/mod.rs` — remove module declarations and re-exports

**Step 1: Delete the files**

```bash
cd doorway/doorway-service/src/routes
rm governance.rs attestations.rs contributors.rs steward.rs \
   presence.rs economic_events.rs exchange.rs custodians.rs \
   compute.rs flow_planning.rs stewarded_resources.rs \
   stewardship.rs account.rs
```

**Step 2: Clean up mod.rs**

Remove the `pub mod` and `pub use` lines for each deleted file. Keep:
- `collectives.rs` (cross-domain path rewriting — temporary, see follow-up)
- `storage_proxy.rs` (new generic proxy)
- All other non-proxy routes (api, apps, auth_routes, blob, db, health, identity, etc.)

**Step 3: Remove unused imports from http.rs**

Any `routes::handle_*_request` calls that were removed in Task 6 may leave unused imports. Fix compiler warnings.

**Step 4: Verify it compiles and tests pass**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | head -20
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20
```

**Step 5: Commit**

```bash
git add -A doorway/doorway-service/src/routes/
git add doorway/doorway-service/src/server/http.rs
git commit -m "refactor(doorway): delete 13 hardcoded proxy files, routing now via registry"
```

---

## Task 8: Integration Smoke Test

Verify the full flow works end-to-end.

**Step 1: Start elohim-storage**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo run --release
```

**Step 2: Verify manifest endpoint**

```bash
curl http://localhost:8090/manifest | jq '.routes | length'
```

Expected: number > 0

**Step 3: Start doorway**

```bash
cd doorway/doorway-service
RUSTFLAGS="" cargo run
```

Expected in logs: `Steward storage self-registered as first peer` with route count.

**Step 4: Test previously broken routes**

```bash
# Mastery (was 405)
curl -X POST http://localhost:8888/api/v1/mastery \
  -H "Content-Type: application/json" \
  -d '{"contentId":"test"}' -w "\n%{http_code}\n"

# Governance (was hardcoded, now via registry)
curl http://localhost:8888/api/v1/governance -w "\n%{http_code}\n"
```

Expected: Not 404 or 405. May return domain errors (no data), but HTTP routing works.

**Step 5: Commit any fixes**

```bash
git commit -m "test(doorway): verify dynamic route registry end-to-end"
```

---

## Follow-Up (Not in This PR)

- **`collectives.rs` cross-domain routing**: Currently kept as a hardcoded special case. The `/api/v1/humans/{id}/collectives` path rewriting should move into the registry's path mapping. Note: there may be a purpose for collective-scoped doorways (e.g., church lobby kiosk driving network discovery) — design this when collective-hosted doorways are explored.
- **DiscoveryService completion**: Replace hardcoded stubs with actual `__doorway_routes` zome calls. This enables peers without elohim-storage (conductor-only) to declare routes.
- **External peer registration**: The `POST /doorway/register` endpoint already exists in the route_registry types. Wire it into http.rs as a built-in route.
- **DNS custom domains**: Steward configures `oakland-church.elohim.host` mapping to a peer's routes via admin dashboard.
- **Projection cache integration**: Registry routes with `cache_ttl > 0` should check DoorwayResolver/projection before proxying to storage.
- **Doorway federation**: Doorways sharing projection caches via MongoDB layer for CDN-like behavior.
