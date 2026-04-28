# Doorway Blob Registry Routing — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Eliminate the recurring `/blob/<hash>` thumbnail regression by making the `RouteRegistry` the universal fallback for unmatched paths, replacing the hard-coded `/api/v1/ || /account/` prefix guard at `doorway/doorway-service/src/server/http.rs:1504` that silently misses every new manifest path family (`/blob/`, `/stream/`, …).

**Architecture:** Extract the dispatch tail (registry consultation + SPA fallback + 404 default) into a small, async, unit-testable function `classify_dispatch` that returns a `Disposition` enum. Replace the prefix-gated registry arm + GET-root_app catch-all with a single call to this helper. The helper's behavior becomes contract-tested: any registry-registered path of any prefix routes through the registry; otherwise GET-with-slug-configured serves the SPA, anything else returns 404. New manifest path families (`blob_proxy`, `stream_proxy`, future) will work automatically — fulfilling the contract written in `doorway/CLAUDE.md`: *"Adding a new endpoint to elohim-storage automatically makes it routable through doorway — no doorway code changes needed."*

**Out of scope (deliberately):**
- Removing the legacy `(GET|HEAD) /store/*` and `(GET|HEAD) /api/blob/*` arms at `http.rs:1257-1320`. Initial diagnosis suggested these were dead, but `genesis/seeder/src/doorway-client.ts:420`, `app/elohim-library/projects/elohim-service/src/connection/direct-connection-strategy.ts:123`, and `doorway-connection-strategy.ts:223` still call them. Migrating those callers is a separate sprint; until then the legacy arms must keep working in parallel.
- The other regression the user reported (the specific blob `sha256-1f3ed518…` returning 404 directly from storage on both `elohim-matthew-alpha` and `elohim-frank-alpha`). That is a seeder/replication issue, not a routing issue. This plan does not address it.

**Tech Stack:** Rust (edition 2021), hyper, tokio, `doorway_client` (RouteRegistry/CompiledRoute/HttpMethod types from `doorway/doorway-client/`).

**File map:**
- Modify: `doorway/doorway-service/src/server/http.rs`
  - Add: `Disposition` enum + `classify_dispatch` async fn (near top of module body)
  - Add: `#[cfg(test)] mod dispatch_classification_tests` (sibling of existing `gate_layer_tests`)
  - Modify: dispatch tail (lines 1500-1538) — replace prefix guard + GET catch-all + default 404 with one call to `classify_dispatch`
- Modify: `doorway/doorway-service/src/server/CLAUDE.md` — document the new contract
- (Read-only reference) `doorway/doorway-service/src/services/route_registry.rs` — `RouteRegistry::with_defaults()`, `compiled_routes` field, `CompiledRoute`, `RouteTarget::StorageProxy`, `HttpMethod` enum
- (Read-only reference) `doorway/CLAUDE.md` — the contract this plan restores

---

### Task 1: Add `Disposition` enum, stub `classify_dispatch`, and write the failing tests

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs`

**Why this task is structured this way:** Rust requires the function to compile before tests can run, so we add a stub returning `Disposition::NotFound` first. Three of the four tests will then fail with the stub — that's the "red" phase. Task 2 makes them green.

- [ ] **Step 1: Add the `Disposition` enum and stub `classify_dispatch` function**

Find a place near the top of the module body in `doorway/doorway-service/src/server/http.rs`, just below the imports and before `pub struct AppState`. Add:

```rust
/// Outcome of dispatching an unmatched request through the route registry.
///
/// Computed by `classify_dispatch` — separates the routing decision from the
/// handler invocation so the decision logic can be unit-tested without spinning
/// up an HTTP server or a real storage proxy.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Disposition {
    /// Registry matched a `StorageProxy` route — caller forwards to `endpoint`.
    StorageProxy { endpoint: String },
    /// Registry matched but the target type is not yet handled by dispatch
    /// (BlobProxy, StreamProxy, ZomeCall, AgentProxy). Caller returns 404.
    RegistryUnhandled,
    /// No registry match, GET method, root_app_slug is configured —
    /// caller falls through to the root SPA bootstrap handler.
    RootApp,
    /// No registry match and no SPA fallback applies — caller returns 404.
    NotFound,
}

/// Classify how an unmatched request should be dispatched.
///
/// Called from `handle_request` after all explicit match arms have been tried.
/// Replaces the previous hard-coded `/api/v1/ || /account/` prefix guard, which
/// failed every time elohim-storage's manifest added a new top-level path
/// family (`blob_proxy`, `stream_proxy`, …) — `/blob/<hash>` requests skipped
/// the registry entirely and fell into the SPA bootstrap, breaking thumbnails.
///
/// The contract: if the registry has any compiled route matching `(method, path)`,
/// the registry decides. Otherwise, GET requests with a configured root SPA
/// slug fall through to the SPA; anything else is 404.
async fn classify_dispatch(
    registry: &crate::services::RouteRegistry,
    root_app_slug: Option<&str>,
    method: &Method,
    path: &str,
) -> Disposition {
    let _ = (registry, root_app_slug, method, path);
    Disposition::NotFound
}
```

Note: the `#[allow(unused)]` ergonomics — the underscore-prefixed binding pattern (`let _ = (...);`) silences "unused parameter" warnings until Task 2 fills in the body.

- [ ] **Step 2: Add the test module with four contract tests**

Append at the very end of `doorway/doorway-service/src/server/http.rs` (after the existing `mod gate_layer_tests`):

```rust
#[cfg(test)]
mod dispatch_classification_tests {
    //! Contract tests for `classify_dispatch`.
    //!
    //! These tests pin the dispatch contract that fixes the recurring
    //! /blob/<hash> regression: any path the registry has compiled is routed
    //! by the registry, regardless of prefix. Adding `blob_proxy` /
    //! `stream_proxy` / future manifest path families to elohim-storage's
    //! manifest must NOT require a doorway code change to make them routable.

    use super::{classify_dispatch, Disposition};
    use crate::services::RouteRegistry;
    use doorway_client::HttpMethod;
    use hyper::Method;

    /// Inject a `StorageProxy` route at `path` into a fresh registry.
    /// Mimics what `register_steward_peer` does internally after fetching
    /// the manifest — see `route_registry.rs:671-687`.
    async fn registry_with_storage_route(method: HttpMethod, path: &str) -> RouteRegistry {
        use crate::services::route_registry::{CompiledRoute, RouteSource, RouteTarget};
        let registry = RouteRegistry::with_defaults();
        let route = CompiledRoute {
            method,
            path: path.to_string(),
            source: RouteSource::StewardPeer {
                storage_url: "http://storage:8090".to_string(),
            },
            target: RouteTarget::StorageProxy {
                endpoint: "http://storage:8090".to_string(),
            },
            auth_required: false,
            cache_ttl_secs: 0,
            rate_limit_rpm: 0,
        };
        let mut compiled = registry.compiled_routes.write().await;
        compiled.push(route);
        drop(compiled);
        registry
    }

    #[tokio::test]
    async fn blob_path_dispatches_to_storage_proxy() {
        // Regression: the recurring thumbnail bug. /blob/<hash> must reach
        // the registry and be classified as StorageProxy, not fall through
        // to the SPA bootstrap.
        let registry = registry_with_storage_route(HttpMethod::Get, "/blob/:hash").await;
        let dispo = classify_dispatch(
            &registry,
            Some("lamad"),
            &Method::GET,
            "/blob/sha256-abcdef123456",
        )
        .await;
        assert!(
            matches!(&dispo, Disposition::StorageProxy { endpoint } if endpoint == "http://storage:8090"),
            "GET /blob/<hash> must classify as StorageProxy, got {dispo:?}"
        );
    }

    #[tokio::test]
    async fn arbitrary_new_prefix_reaches_registry() {
        // The durable contract: a hypothetical future manifest path family
        // (e.g. /thumbnails/, /shards/, anything outside /api/v1/+/account/)
        // must route through the registry without a doorway code change.
        let registry = registry_with_storage_route(HttpMethod::Get, "/future/:id").await;
        let dispo = classify_dispatch(
            &registry,
            Some("lamad"),
            &Method::GET,
            "/future/some-id",
        )
        .await;
        assert!(
            matches!(dispo, Disposition::StorageProxy { .. }),
            "Any registry-compiled path must route through the registry"
        );
    }

    #[tokio::test]
    async fn unregistered_get_with_slug_falls_through_to_root_app() {
        // SPA client-side routing: paths the registry doesn't know
        // (e.g. /learn/<id>) must serve the SPA bootstrap on GET when a
        // root_app_slug is configured.
        let registry = RouteRegistry::with_defaults();
        let dispo = classify_dispatch(
            &registry,
            Some("lamad"),
            &Method::GET,
            "/learn/some-path-id",
        )
        .await;
        assert_eq!(
            dispo,
            Disposition::RootApp,
            "Unregistered GET with slug configured must fall through to SPA"
        );
    }

    #[tokio::test]
    async fn unregistered_post_returns_not_found() {
        // API misses must 404 (not serve HTML). Without this, an unknown
        // POST /api/v1/foo would render the SPA bootstrap to a JSON client.
        let registry = RouteRegistry::with_defaults();
        let dispo = classify_dispatch(
            &registry,
            Some("lamad"),
            &Method::POST,
            "/api/v1/no-such-route",
        )
        .await;
        assert_eq!(
            dispo,
            Disposition::NotFound,
            "Unregistered non-GET must 404, never SPA"
        );
    }
}
```

- [ ] **Step 3: Verify the stub compiles and confirm the red phase**

Run:

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib dispatch_classification_tests 2>&1 | tail -40
```

Expected: compiles cleanly. Test results should show **1 passing** (`unregistered_post_returns_not_found` — the stub returns `NotFound`, which matches), **3 failing**:
- `blob_path_dispatches_to_storage_proxy` — got NotFound, expected StorageProxy
- `arbitrary_new_prefix_reaches_registry` — got NotFound, expected StorageProxy
- `unregistered_get_with_slug_falls_through_to_root_app` — got NotFound, expected RootApp

If the stub doesn't compile, fix the import path for `RouteRegistry` (it lives in `crate::services::RouteRegistry`; the test imports `CompiledRoute`/`RouteSource`/`RouteTarget` from `crate::services::route_registry`). Confirm by reading `doorway/doorway-service/src/services/mod.rs`.

- [ ] **Step 4: Commit the red phase**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/server/http.rs
git commit -m "$(cat <<'EOF'
test(doorway): add classify_dispatch contract tests for registry-as-fallback

Pins the dispatch contract that fixes the recurring /blob/<hash> regression:
any path the RouteRegistry has compiled is dispatched by the registry,
regardless of prefix. The hard-coded "/api/v1/||/account/" guard at
http.rs:1504 has misclassified every new manifest path family (blob_proxy,
stream_proxy) as a SPA route since they were added — this commit adds the
red-phase tests; Task 2 implements classify_dispatch to make them pass.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 2: Implement `classify_dispatch` to make the failing tests pass

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (the `classify_dispatch` body added in Task 1)

- [ ] **Step 1: Replace the stub body with the real implementation**

In `doorway/doorway-service/src/server/http.rs`, find the stub `classify_dispatch` from Task 1 and replace its body:

```rust
async fn classify_dispatch(
    registry: &crate::services::RouteRegistry,
    root_app_slug: Option<&str>,
    method: &Method,
    path: &str,
) -> Disposition {
    let http_method = match *method {
        Method::GET => doorway_client::HttpMethod::Get,
        Method::POST => doorway_client::HttpMethod::Post,
        Method::PUT => doorway_client::HttpMethod::Put,
        Method::DELETE => doorway_client::HttpMethod::Delete,
        Method::PATCH => doorway_client::HttpMethod::Patch,
        Method::HEAD => doorway_client::HttpMethod::Head,
        _ => doorway_client::HttpMethod::Get,
    };

    let matches = registry.match_request(http_method, path).await;
    if let Some(route) = matches.first() {
        if let Some(endpoint) = route.storage_endpoint() {
            return Disposition::StorageProxy {
                endpoint: endpoint.to_string(),
            };
        }
        // Future: handle ZomeCall, AgentProxy, BlobProxy, StreamProxy targets.
        // For now any non-StorageProxy registry hit returns 404.
        return Disposition::RegistryUnhandled;
    }

    if *method == Method::GET && root_app_slug.is_some() {
        return Disposition::RootApp;
    }

    Disposition::NotFound
}
```

- [ ] **Step 2: Run the tests to verify all four pass**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib dispatch_classification_tests 2>&1 | tail -20
```

Expected: **4 passed; 0 failed**. If any fail, re-read the failing assertion against the implementation — most likely culprit is a missing `match_request` await or the wrong `HttpMethod` mapping.

- [ ] **Step 3: Run the full doorway-service test suite to confirm no regressions**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -15
```

Expected: all existing tests still pass (this commit only adds — doesn't change behavior of `handle_request` yet).

- [ ] **Step 4: Commit the green phase**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/server/http.rs
git commit -m "$(cat <<'EOF'
feat(doorway): implement classify_dispatch — registry as universal fallback

Resolves the failing contract tests added in the previous commit. The helper
asks the RouteRegistry for any (method, path) match; if the matched route
targets a StorageProxy, the caller forwards to that endpoint. Any other
registry hit falls through as RegistryUnhandled (handler-side 404 for now;
slot reserved for BlobProxy/StreamProxy/ZomeCall/AgentProxy dispatchers).

Registry miss with GET + root_app_slug → SPA bootstrap. Anything else → 404.

The function is not yet wired into handle_request — Task 3 does that.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 3: Wire `classify_dispatch` into `handle_request`, removing the prefix guard

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` lines 1500-1538

- [ ] **Step 1: Replace the dispatch tail**

Open `doorway/doorway-service/src/server/http.rs`. Find the block that begins at line 1500 (currently):

```rust
        // ====================================================================
        // Dynamic Route Registry — all remaining /api/v1/* and /account/* routes
        // ====================================================================
        (_, p) if p.starts_with("/api/v1/") || p.starts_with("/account/") => {
            let http_method = match *req.method() {
                Method::GET => doorway_client::HttpMethod::Get,
                Method::POST => doorway_client::HttpMethod::Post,
                Method::PUT => doorway_client::HttpMethod::Put,
                Method::DELETE => doorway_client::HttpMethod::Delete,
                Method::PATCH => doorway_client::HttpMethod::Patch,
                Method::HEAD => doorway_client::HttpMethod::Head,
                _ => doorway_client::HttpMethod::Get,
            };

            let matches = state.route_registry.match_request(http_method, p).await;

            if let Some(route) = matches.first() {
                if let Some(endpoint) = route.storage_endpoint() {
                    debug!(path = %p, endpoint = %endpoint, "Registry-routed to storage proxy");
                    return Ok(to_boxed(routes::forward_to_storage(req, endpoint, p).await));
                }
                // Future: handle ZomeCall, AgentProxy, BlobProxy, StreamProxy targets
                debug!(path = %p, "Registry matched but target type not yet handled");
                to_boxed(not_found_response(p))
            } else {
                debug!(path = %p, "No registry match");
                to_boxed(not_found_response(p))
            }
        }

        // Root app catch-all: unmatched GET paths serve the SPA (if ROOT_APP_SLUG configured).
        // Handles client-side routing — Angular paths like /learn/123 that aren't API routes.
        (Method::GET, p) if state.args.root_app_slug.is_some() => {
            to_boxed(routes::handle_root_app_request(Arc::clone(&state), p).await)
        }

        // Not found
        _ => to_boxed(not_found_response(&path)),
    };
```

Replace the entire region (the registry match arm + the root_app catch-all + the default 404) with a single wildcard arm that delegates to `classify_dispatch`:

```rust
        // ====================================================================
        // Dynamic Route Registry + SPA fallback — all remaining requests.
        //
        // The registry is consulted on every otherwise-unmatched request.
        // Any path elohim-storage declared in its manifest (routes,
        // blob_proxy, stream_proxy, …) is dispatched without doorway-side
        // path-prefix changes — this is the contract written in
        // doorway/CLAUDE.md ("Adding a new endpoint to elohim-storage
        // automatically makes it routable through doorway").
        //
        // Registry miss + GET + slug configured → SPA bootstrap (Angular
        // client-side routing). Anything else → 404.
        // ====================================================================
        (_, p) => {
            let dispo = classify_dispatch(
                &state.route_registry,
                state.args.root_app_slug.as_deref(),
                req.method(),
                p,
            )
            .await;

            match dispo {
                Disposition::StorageProxy { endpoint } => {
                    debug!(path = %p, %endpoint, "Registry-routed to storage proxy");
                    return Ok(to_boxed(
                        routes::forward_to_storage(req, &endpoint, p).await,
                    ));
                }
                Disposition::RegistryUnhandled => {
                    debug!(path = %p, "Registry matched but target type not yet handled");
                    to_boxed(not_found_response(p))
                }
                Disposition::RootApp => {
                    to_boxed(routes::handle_root_app_request(Arc::clone(&state), p).await)
                }
                Disposition::NotFound => to_boxed(not_found_response(p)),
            }
        }
    };
```

Notes for the implementer:
- The wildcard `(_, p) =>` MUST be the last arm. If there are any explicit match arms below it, they become unreachable and `cargo build` will warn. Make sure the prior `_ => to_boxed(not_found_response(&path))` line is removed (it's replaced by `Disposition::NotFound`).
- `req.method()` is borrowed (returns `&Method`); the existing dispatch already uses it that way.
- `state.args.root_app_slug` is an `Option<String>`; `.as_deref()` converts to `Option<&str>` to match the helper signature.

- [ ] **Step 2: Build and run all tests**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build 2>&1 | tail -10
RUSTFLAGS="" cargo test --lib --bins 2>&1 | tail -20
```

Expected: clean build, all tests pass (including the four `dispatch_classification_tests` from Tasks 1-2 and all pre-existing tests).

If you get an "unreachable pattern" warning, you didn't fully replace the old arms — remove the duplicate `_ =>` default at the bottom.

If you get a borrow-checker error around `req.method()` being used after `req` is consumed by `forward_to_storage`, capture the method before the helper call:

```rust
let req_method_owned = req.method().clone();
let dispo = classify_dispatch(
    &state.route_registry,
    state.args.root_app_slug.as_deref(),
    &req_method_owned,
    p,
)
.await;
```

(The current code already clones `method` at the top of `handle_request` line 789 — `let method = req.method().clone();` — so you may be able to just pass `&method`.)

- [ ] **Step 3: Run clippy and fmt**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo clippy --lib --bins -- -D warnings 2>&1 | tail -15
cargo fmt --check 2>&1 | tail -5
```

Expected: no warnings, no formatting drift. If clippy flags `Disposition::RegistryUnhandled` as an unused enum variant (it's only constructed inside `classify_dispatch`), that's expected and fine — the variant exists for future BlobProxy/StreamProxy/etc. handlers.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/server/http.rs
git commit -m "$(cat <<'EOF'
fix(doorway): make RouteRegistry the universal dispatch fallback

Replaces the hard-coded "(p) if p.starts_with('/api/v1/') ||
p.starts_with('/account/')" guard at http.rs:1500-1528 with a single
wildcard arm that delegates to classify_dispatch. The registry is now
consulted on every otherwise-unmatched request; SPA fallback only fires
on registry miss for GET-with-slug.

This restores the contract written in doorway/CLAUDE.md ("Adding a new
endpoint to elohim-storage automatically makes it routable through
doorway — no doorway code changes needed"), which has been silently
violated since blob_proxy was added to the manifest:

  - storage's build_manifest() declared blob_proxy.base_path = "/blob"
  - route_registry compiled /blob/:hash as a StorageProxy
  - dispatch never consulted the registry for /blob/* paths
  - /blob/<hash> fell into the SPA bootstrap, breaking thumbnails

Stacked-regression history (the recurring whipsaw the user named):
  Jan 16 (#772a3f59) — added /api/blob/* alias rewriting to /store/*
  Mar 12 (#d2657c7e) — 13-proxy purge introduced the prefix guard
  Apr 21 (#80361987) — frontend switched to /blob/{hash} per registry contract
  Apr 28 (this commit) — dispatch finally honors that contract

Future BlobProxy/StreamProxy/ZomeCall/AgentProxy handlers will plug in via
the Disposition::RegistryUnhandled slot without touching the dispatch tail.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 4: Update `server/CLAUDE.md` to document the new dispatch contract

**Files:**
- Modify: `doorway/doorway-service/src/server/CLAUDE.md`

The current text says the registry handles 90% of routes but lists the prefix gate without explaining its (now-removed) restriction. Bring it in line with reality.

- [ ] **Step 1: Replace the "How Routes Work" section**

Open `doorway/doorway-service/src/server/CLAUDE.md`. Find the section that begins with `## How Routes Work` and ends before the next `##` heading. Replace it with:

```markdown
## How Routes Work

```
Request → http.rs match block
  ├─ Built-in routes (health, auth, admin, conductor, bootstrap, signal, cache)
  ├─ Special routes with doorway-specific logic (collectives, elohim-agent, identity)
  ├─ Wildcard arm → classify_dispatch(...)
  │   ├─ Registry match + StorageProxy target  → forward_to_storage()
  │   ├─ Registry match + other target type    → 404 (until that target's
  │   │                                            handler is wired —
  │   │                                            BlobProxy, StreamProxy,
  │   │                                            ZomeCall, AgentProxy)
  │   ├─ No registry match + GET + slug set    → SPA bootstrap
  │   └─ No registry match otherwise           → 404
```

The wildcard arm is unconditional — it consults the RouteRegistry on every
request that didn't match an explicit arm above. Any path elohim-storage
declares in its manifest (routes, blob_proxy, stream_proxy, …) becomes
routable without a doorway code change. Adding a new path-family-prefix
to the dispatch is no longer required and is no longer a regression vector.

The dispatch tail used to be a hand-maintained list of prefixes
(`/api/v1/`, `/account/`). Every new manifest path family (blob_proxy →
`/blob/`, stream_proxy → `/stream/`) silently fell through to the SPA
bootstrap until someone noticed thumbnails breaking. The classify_dispatch
helper exists specifically so that pattern cannot recur.
```

- [ ] **Step 2: Update the "Before adding ANY route" decision tree**

In the same file, the top decision tree currently says "Add a match arm ABOVE the registry fallback." That's still true, but tighten the language. Replace the existing top section with:

```markdown
# Doorway Server — Route Registry Anti-Pattern Gate

**Before adding ANY route to `http.rs`**, answer this question:

> Does this route need doorway-specific logic (auth gating, path rewriting, WebSocket upgrade, non-storage target)?

- **NO** → Do NOT add it here. Add the endpoint to elohim-storage and register it in `build_manifest()`. The RouteRegistry auto-discovers it via the wildcard arm at the bottom of the dispatch — no doorway code change required.
- **YES** → Add a match arm ABOVE the wildcard arm. Document why the registry can't handle it.

We deleted 13 identical proxy files that violated this rule. See `doorway/CLAUDE.md` for the full anti-pattern catalog.

> **Path-prefix guards are forbidden in the wildcard arm.** Earlier versions of this dispatch gated the registry by `p.starts_with("/api/v1/") || p.starts_with("/account/")`, which silently broke every new manifest path family added since (`/blob/`, `/stream/`). The wildcard arm now delegates unconditionally to `classify_dispatch`. If you find yourself wanting to add a prefix check there, stop — the registry already knows its own prefixes.
```

- [ ] **Step 3: Verify the file renders sensibly**

```bash
cd /projects/elohim
cat doorway/doorway-service/src/server/CLAUDE.md | head -40
```

Read the output; confirm the decision tree and the route-flow ASCII diagram are coherent.

- [ ] **Step 4: Commit**

```bash
cd /projects/elohim
git add doorway/doorway-service/src/server/CLAUDE.md
git commit -m "$(cat <<'EOF'
docs(doorway): document registry-as-universal-fallback dispatch contract

Updates CLAUDE.md to match the dispatch tail's new shape (wildcard arm
delegating to classify_dispatch). Adds an explicit prohibition on
path-prefix guards in the wildcard arm — the regression vector that
caused the recurring /blob/<hash> thumbnail bug.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

---

### Task 5: Verify on alpha after orchestrator deploy

**Why this is a task and not just a footnote:** The Eclipse Che environment can't run docker/k8s/holochain locally, so end-to-end verification of the deployed surface happens via the Jenkins pipeline → alpha cluster path. This task captures the verification commands so the executor doesn't ship and walk away without checking.

**Files:** none (verification only)

- [ ] **Step 1: Push to dev and let the orchestrator pick it up**

```bash
cd /projects/elohim
git push origin dev
```

The orchestrator (`genesis/orchestrator/Jenkinsfile`) will detect the changeset under `doorway/doorway-service/` and trigger the doorway pipeline. You will receive a notification when it completes — do not poll.

- [ ] **Step 2: Once doorway pipeline is green, verify alpha is serving thumbnails**

Pick a blob hash that's known to exist on alpha. The user's failing URL was `sha256-1f3ed518a975f0eb55ae72c7cca8ef396c8f73c61ecf730ad54920ea0a24a955`, but the user noted that specific blob is genuinely missing from storage (separate seeder issue). Find a blob that IS on alpha:

```bash
# Pick any path with a thumbnail; the metadata.thumbnailUrl is /blob/<hash>
curl -s https://doorway-alpha.elohim.host/api/v1/lamad/paths 2>&1 | python3 -c "
import json, sys
data = json.load(sys.stdin)
for p in data.get('paths', [])[:5]:
    meta = p.get('metadata', {}) or {}
    if 'thumbnailUrl' in meta:
        print(p.get('id', ''), meta['thumbnailUrl'])
" | head -5
```

Then for each thumbnailUrl, fetch it and inspect:

```bash
URL='https://doorway-alpha.elohim.host/blob/sha256-<hash-from-above>'
curl -sS -D - -o /tmp/blob.bin "$URL" 2>&1 | head -10
file /tmp/blob.bin
ls -l /tmp/blob.bin
```

Expected:
- HTTP 200
- Content-Type: image/png (or image/jpeg, etc.) — NOT `text/html`
- `file` reports a real image format, not "HTML document"
- File size matches reality (a thumbnail is typically 5–500 KB)

If you get HTML back: the dispatch fix didn't deploy. Check the alpha pod's image tag matches the new build.

If you get HTTP 404: that specific blob isn't seeded on alpha. The dispatch fix is working — it just forwarded the request to storage and storage said 404. Try a different blob hash, or accept that the seeder bug is in scope of a separate fix.

- [ ] **Step 3: Spot-check the SPA fallback still works**

```bash
curl -sS -D - https://doorway-alpha.elohim.host/learn/some-path-id 2>&1 | head -5
```

Expected: HTTP 200 with HTML body (the SPA bootstrap or extracted index). The wildcard arm fell through `Disposition::RootApp` correctly.

- [ ] **Step 4: Spot-check that an unknown API path returns 404 (not HTML)**

```bash
curl -sS -D - https://doorway-alpha.elohim.host/api/v1/no-such-route 2>&1 | head -5
```

Expected: HTTP 404. Body is JSON or plain text — NOT the SPA HTML. (This guards against the "API misses serve HTML" failure mode that classify_dispatch was designed to prevent.)

- [ ] **Step 5: If all three checks pass, no further action**

The fix is in. The recurring pattern is broken at the dispatch level. The user's reported `/blob/<hash>` rendering bug is resolved (modulo the separate seeder issue for the specific hash they reported).

If you discover any of the spot-checks failing in unexpected ways, **do not patch over it** — return to the systematic-debugging skill, file the new symptom, and treat it as its own root-cause investigation.

---

## Self-review notes

- **Spec coverage:** the three deliverables the user asked for —
  1. *Hit the regression* — Tasks 1-3 wire `/blob/<hash>` through the registry; verified by `blob_path_dispatches_to_storage_proxy` test and Task 5 alpha curl.
  2. *Durable fix* — Tasks 1-3 eliminate the prefix-list pattern entirely; verified by `arbitrary_new_prefix_reaches_registry` test (proves any future manifest path family routes correctly).
  3. *Cleaned up pattern* — Task 4 documents the new contract in CLAUDE.md and explicitly forbids the regression-vector pattern. Note: the cleanup of legacy `/store/*` and `/api/blob/*` arms is **deliberately deferred** because those arms still have active in-tree callers (genesis/seeder, direct-connection-strategy.ts, doorway-connection-strategy.ts). That's a follow-up sprint, not this plan.
- **Type consistency:** `Disposition` enum used identically in helper signature, helper body, and dispatch caller. `classify_dispatch` signature stable across Tasks 1, 2, 3.
- **No placeholders:** every code block contains the literal code to write or replace; every command contains the literal command to run with the working directory specified.

## Out-of-scope follow-ups (do not attempt in this plan)

These were surfaced during analysis but each warrants its own scoped change:

1. **Migrate `/store/<hash>` callers to `/blob/<hash>`** — `genesis/seeder/src/doorway-client.ts:420`, `app/elohim-library/projects/elohim-service/src/connection/direct-connection-strategy.ts:123`, `app/elohim-library/projects/elohim-service/src/cache/content-resolver.ts:478`. Once all callers are migrated, delete the `(GET|HEAD) /store/*` arms at `http.rs:1257-1280`.

2. **Migrate `/api/blob/<hash>` callers to `/blob/<hash>`** — `app/elohim-library/projects/elohim-service/src/connection/doorway-connection-strategy.ts:223`, `app/elohim-app/src/app/elohim/services/doorway-client.service.ts:179`. The user's commit `80361987` only fixed `storage-client.service.ts`. Once those two are migrated, delete the `(GET|HEAD) /api/blob/*` arms at `http.rs:1282-1320`.

3. **Seeder/replication for missing blobs** — the specific blob `sha256-1f3ed518…` returned 404 directly from storage on multiple peers despite a path metadata `thumbnailUrl` referencing it. The seeder fix `c32adc5f` (reapply: link uploaded thumbnail blobs via path row blobHash) wrote the metadata but the upload itself or peer replication didn't land for at least one path. Tracker: alpha seeder report.

4. **Wire `BlobProxy`/`StreamProxy` target dispatch** — the `Disposition::RegistryUnhandled` branch returns 404 today. When DNA-discovered blob_proxy or stream_proxy starts being used (currently they're only used via steward-self-registration which compiles them as `StorageProxy`), the dispatcher will need handlers for those `RouteTarget` variants. The existing `// Future:` comment at `http.rs:1522` (now moved into `classify_dispatch`) marks the spot.
