# Runtime Request Correlation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every HTTP request through the Elohim runtimes **correlatable end-to-end**. An a2o test scenario hits a doorway, the doorway proxies to a specific peer's storage, the peer writes logs — and the test can ask each runtime, "show me the logs for request X on peer Y". This is the prerequisite for closing the dev-to-acceptance loop against a federated mesh.

**Architecture:** Three layers of IDs flowing through headers and tracing spans:

1. **`X-Request-ID`** — stable per-request UUID, generated on first touch, propagated on every hop. Answers "which request was this?"
2. **`X-Target-Peer`** — slug naming the intended peer (e.g., `terrance-household`, `shem`, `mary-mobile`). Answers "which peer should handle this?" Doorway uses it for federation routing.
3. **`X-Served-By`** — response-only slug naming the peer that actually handled it. Answers "which peer ran the code?" Populated by the runtime that serves the response.

All three become tracing span fields on every log line. A bounded in-process ring buffer (no Loki required) retains the last N log entries indexed by request-ID. `/admin/logs?request_id=X` returns them. The a2o framework captures IDs from response headers, writes them into scenario artifacts, and fetches backend logs per peer on failure.

**Tech Stack:** Rust with `hyper` (raw, not axum), `tracing` + `tracing-subscriber` (JSON subscriber + custom ring-buffer layer), `tokio::task_local!` for per-request context, `uuid` v4, `reqwest` for outbound propagation. a2o side uses existing TypeScript + Playwright. Schemas authored first per project convention.

---

## Scope & Phases

This plan has six phases. Each phase is independently deployable and produces observable value. Execute as separate sprints via `/shift` if preferred — each phase closes with passing tests and a commit that ships.

| Phase | Subsystem | Outcome |
|---|---|---|
| **1** | Wire contract | JSON schema + Rust/TS types frozen before any runtime work |
| **2** | Doorway inbound | Doorway generates/echoes IDs, logs carry them, errors include them |
| **3** | Doorway log retrieval | Ring-buffer layer + `/admin/logs` endpoint — first operational correlation |
| **4** | Doorway outbound + peer routing | `X-Target-Peer` triggers federation proxy; IDs propagate downstream |
| **5** | Elohim-storage mirror | Same middleware + logs endpoint; end-to-end correlation across the hop |
| **6** | a2o integration | Persona→peer mapping, capture hook, log-fetch on failure, sprint-report extension |

---

## Phase 1: Wire Contract (schema-first)

Per project convention: write the schema, then Rust structs and TypeScript types comply with it. No runtime code until the contract is locked.

### Task 1.1: Author correlation wire schema

**Files:**
- Create: `elohim/sdk/schemas/v1/correlation/request-correlation.schema.json`
- Create: `elohim/sdk/schemas/v1/correlation/admin-logs-response.schema.json`

- [ ] **Step 1: Write the headers schema**

```json
// elohim/sdk/schemas/v1/correlation/request-correlation.schema.json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/correlation/request-correlation.schema.json",
  "title": "RequestCorrelationHeaders",
  "description": "Headers attached to every HTTP request/response across Elohim runtimes for end-to-end correlation and peer routing.",
  "type": "object",
  "properties": {
    "xRequestId": {
      "type": "string",
      "pattern": "^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$",
      "description": "UUID v4. Client MAY set it; if absent, first-touch runtime generates it. Echoed on every response and propagated on every hop."
    },
    "xTargetPeer": {
      "type": "string",
      "pattern": "^[a-z0-9][a-z0-9-]{0,62}[a-z0-9]$",
      "description": "Slug of the peer that SHOULD handle this request. Doorway proxies to that peer's doorway if known via federation. Absent = local handling."
    },
    "xServedBy": {
      "type": "string",
      "pattern": "^[a-z0-9][a-z0-9-]{0,62}[a-z0-9]$",
      "description": "Response-only. Slug of the runtime that actually produced this response. Always present on responses from Elohim runtimes."
    }
  },
  "required": ["xRequestId", "xServedBy"]
}
```

- [ ] **Step 2: Write the admin-logs response schema**

```json
// elohim/sdk/schemas/v1/correlation/admin-logs-response.schema.json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "$id": "https://elohim.protocol/schemas/correlation/admin-logs-response.schema.json",
  "title": "AdminLogsResponse",
  "description": "Response of GET /admin/logs — log entries retained in the process ring buffer, filterable by request_id and/or target_peer.",
  "type": "object",
  "required": ["servedBy", "query", "entries"],
  "additionalProperties": false,
  "properties": {
    "servedBy": { "type": "string", "description": "Slug of the runtime that produced this response" },
    "query": {
      "type": "object",
      "additionalProperties": false,
      "properties": {
        "requestId":  { "type": "string" },
        "targetPeer": { "type": "string" },
        "limit":      { "type": "integer", "minimum": 1, "maximum": 10000 }
      }
    },
    "truncated": { "type": "boolean", "description": "True if older entries matching the query were evicted from the ring buffer before capture" },
    "entries": {
      "type": "array",
      "items": { "$ref": "#/definitions/LogEntry" }
    }
  },
  "definitions": {
    "LogEntry": {
      "type": "object",
      "required": ["timestamp", "level", "target", "message"],
      "additionalProperties": true,
      "properties": {
        "timestamp":  { "type": "string", "format": "date-time" },
        "level":      { "enum": ["TRACE", "DEBUG", "INFO", "WARN", "ERROR"] },
        "target":     { "type": "string", "description": "tracing target (e.g., 'doorway::routes::storage_proxy')" },
        "message":    { "type": "string" },
        "requestId":  { "type": "string" },
        "targetPeer": { "type": "string" },
        "servedBy":   { "type": "string" },
        "fields":     { "type": "object", "description": "Additional structured fields from the tracing event" }
      }
    }
  }
}
```

- [ ] **Step 3: Write the error-response addendum schema**

Doorway's existing error body (`{error, message, statusCode}`) must be extended with `requestId` and `servedBy`. Rather than a new schema, document this inside the request-correlation schema:

Edit `request-correlation.schema.json` — append to the top-level `description`:

```
Every 4xx/5xx JSON response body emitted by Elohim runtimes MUST include `requestId` and `servedBy` top-level string fields in addition to any error-specific payload.
```

- [ ] **Step 4: Commit**

```bash
git add elohim/sdk/schemas/v1/correlation/
git commit -m "feat(schemas): request correlation wire contract (headers + admin/logs response)"
```

### Task 1.2: Generate Rust and TypeScript types from schemas

**Files:**
- Create: `elohim/sdk/schemas/v1/correlation/types.rs` (hand-authored, schema-compliant)
- Modify: `elohim/sdk/schemas/scripts/codegen-ts.mjs` (add correlation schemas to `INTERFACE_FILES`)

- [ ] **Step 1: Inspect the existing codegen script**

```bash
cat elohim/sdk/schemas/scripts/codegen-ts.mjs | head -40
```

Locate the `INTERFACE_FILES` array (or equivalent). Note the path convention.

- [ ] **Step 2: Add correlation schemas to codegen**

Edit `codegen-ts.mjs`. Append to `INTERFACE_FILES`:

```javascript
  { schema: 'v1/correlation/request-correlation.schema.json', output: 'generated/RequestCorrelationHeaders.ts' },
  { schema: 'v1/correlation/admin-logs-response.schema.json', output: 'generated/AdminLogsResponse.ts' },
```

- [ ] **Step 3: Run codegen**

```bash
cd /projects/elohim
pnpm run schema:codegen:ts
```

Expected: two new files appear under `elohim/sdk/storage-client-ts/src/generated/` with matching shapes.

- [ ] **Step 4: Hand-author the Rust counterpart types**

```rust
// doorway/doorway-service/src/correlation/types.rs
//! Wire types for request correlation — MUST match
//! elohim/sdk/schemas/v1/correlation/*.schema.json

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Per-request context carried in tokio::task_local and tracing spans.
/// Field names match the wire schema (camelCase on the wire, snake_case in Rust).
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub request_id: String,
    pub target_peer: Option<String>,
    pub served_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_peer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub served_by: Option<String>,
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLogsQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub target_peer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdminLogsResponse {
    pub served_by: String,
    pub query: AdminLogsQuery,
    pub truncated: bool,
    pub entries: Vec<LogEntry>,
}

/// Header names — single source of truth.
pub mod headers {
    pub const REQUEST_ID: &str = "x-request-id";
    pub const TARGET_PEER: &str = "x-target-peer";
    pub const SERVED_BY: &str = "x-served-by";
}
```

- [ ] **Step 5: Mirror the same file into elohim-storage**

```bash
mkdir -p /projects/elohim/elohim/elohim-storage/src/correlation
cp /projects/elohim/doorway/doorway-service/src/correlation/types.rs \
   /projects/elohim/elohim/elohim-storage/src/correlation/types.rs
```

- [ ] **Step 6: Wire module declarations**

Edit `doorway/doorway-service/src/lib.rs` — add `pub mod correlation;` and `correlation/mod.rs`:

```rust
// doorway/doorway-service/src/correlation/mod.rs
pub mod types;
pub use types::*;
```

Edit `elohim/elohim-storage/src/lib.rs` similarly.

- [ ] **Step 7: Verify both crates compile**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build --lib
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --lib
```

Expected: both succeed with no warnings about unused types (types are `pub`, not yet referenced — that's fine).

- [ ] **Step 8: Commit**

```bash
git add doorway/doorway-service/src/correlation/ \
        doorway/doorway-service/src/lib.rs \
        elohim/elohim-storage/src/correlation/ \
        elohim/elohim-storage/src/lib.rs \
        elohim/sdk/schemas/scripts/codegen-ts.mjs \
        elohim/sdk/storage-client-ts/src/generated/RequestCorrelationHeaders.ts \
        elohim/sdk/storage-client-ts/src/generated/AdminLogsResponse.ts
git commit -m "feat(correlation): Rust + TypeScript types for wire contract"
```

---

## Phase 2: Doorway Inbound — Request Context + Span + Error Echo

Goal: every inbound HTTP request gets a `RequestContext` in task-local state; every log line carries `request_id` and `target_peer` via tracing span; every response (success and error) echoes `X-Request-ID` and `X-Served-By`.

### Task 2.1: Define the task-local context + extraction helpers

**Files:**
- Create: `doorway/doorway-service/src/correlation/context.rs`
- Create: `doorway/doorway-service/src/correlation/context_test.rs` (or `#[cfg(test)] mod tests` inside)
- Modify: `doorway/doorway-service/src/correlation/mod.rs`

- [ ] **Step 1: Write the failing test**

```rust
// doorway/doorway-service/src/correlation/context.rs  (bottom, in #[cfg(test)] mod tests)
#[cfg(test)]
mod tests {
    use super::*;
    use hyper::{Request, HeaderMap};

    fn req_with_headers(headers: &[(&str, &str)]) -> Request<()> {
        let mut b = Request::builder();
        for (k, v) in headers {
            b = b.header(*k, *v);
        }
        b.body(()).unwrap()
    }

    #[test]
    fn extracts_existing_request_id() {
        let req = req_with_headers(&[("x-request-id", "11111111-2222-4333-8444-555555555555")]);
        let ctx = RequestContext::extract(req.headers(), "doorway-alpha");
        assert_eq!(ctx.request_id, "11111111-2222-4333-8444-555555555555");
    }

    #[test]
    fn generates_uuid_when_header_absent() {
        let req = req_with_headers(&[]);
        let ctx = RequestContext::extract(req.headers(), "doorway-alpha");
        // UUID v4: 8-4-4-4-12 hex with version 4
        let re = regex_lite::Regex::new(
            r"^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$"
        ).unwrap();
        assert!(re.is_match(&ctx.request_id), "not a v4 UUID: {}", ctx.request_id);
    }

    #[test]
    fn captures_target_peer_when_present() {
        let req = req_with_headers(&[("x-target-peer", "terrance-household")]);
        let ctx = RequestContext::extract(req.headers(), "doorway-alpha");
        assert_eq!(ctx.target_peer.as_deref(), Some("terrance-household"));
    }

    #[test]
    fn target_peer_is_none_when_absent() {
        let req = req_with_headers(&[]);
        let ctx = RequestContext::extract(req.headers(), "doorway-alpha");
        assert_eq!(ctx.target_peer, None);
    }

    #[test]
    fn rejects_malformed_target_peer_slug() {
        // Slug with uppercase should be ignored (treated as absent) rather than propagated.
        let req = req_with_headers(&[("x-target-peer", "Has-Upper")]);
        let ctx = RequestContext::extract(req.headers(), "doorway-alpha");
        assert_eq!(ctx.target_peer, None);
    }

    #[test]
    fn served_by_is_set_from_argument() {
        let req = req_with_headers(&[]);
        let ctx = RequestContext::extract(req.headers(), "doorway-alpha");
        assert_eq!(ctx.served_by, "doorway-alpha");
    }
}
```

- [ ] **Step 2: Add `regex-lite` to dev-dependencies**

Edit `doorway/doorway-service/Cargo.toml`, under `[dev-dependencies]`:

```toml
regex-lite = "0.1"
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib correlation::context
```

Expected: FAIL — `RequestContext::extract` not defined.

- [ ] **Step 4: Write the implementation**

```rust
// doorway/doorway-service/src/correlation/context.rs
//! Per-request context: extracted from headers or generated, then stashed in
//! a tokio task-local so error helpers and ring-buffer layer can read it.

use super::types::{headers, RequestContext};
use hyper::HeaderMap;
use uuid::Uuid;

const SLUG_RE: &str = r"^[a-z0-9][a-z0-9-]{0,62}[a-z0-9]$";

impl RequestContext {
    pub fn extract(headers: &HeaderMap, served_by: &str) -> Self {
        let request_id = headers
            .get(headers::REQUEST_ID)
            .and_then(|v| v.to_str().ok())
            .filter(|s| Uuid::parse_str(s).is_ok())
            .map(String::from)
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        let target_peer = headers
            .get(headers::TARGET_PEER)
            .and_then(|v| v.to_str().ok())
            .filter(|s| is_valid_slug(s))
            .map(String::from);

        RequestContext {
            request_id,
            target_peer,
            served_by: served_by.to_string(),
        }
    }
}

fn is_valid_slug(s: &str) -> bool {
    if s.is_empty() || s.len() > 64 { return false; }
    let bytes = s.as_bytes();
    let ok_char = |b: u8| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-';
    if bytes[0] == b'-' || *bytes.last().unwrap() == b'-' { return false; }
    bytes.iter().all(|&b| ok_char(b))
}

tokio::task_local! {
    /// Per-request context. Only present inside `with_context(ctx, fut)`.
    pub static REQUEST_CTX: RequestContext;
}

/// Current context if inside a scoped async block; None otherwise.
pub fn current() -> Option<RequestContext> {
    REQUEST_CTX.try_with(|c| c.clone()).ok()
}
```

Add to `correlation/mod.rs`:

```rust
pub mod context;
pub mod types;
pub use context::{current, REQUEST_CTX};
pub use types::*;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib correlation::context
```

Expected: all 6 tests pass.

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/correlation/ doorway/doorway-service/Cargo.toml Cargo.lock
git commit -m "feat(correlation): RequestContext extraction + task-local scope"
```

### Task 2.2: Scope every inbound request inside the context + tracing span

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (top of request dispatch)
- Modify: `doorway/doorway-service/src/main.rs` (compute `node_slug` from `args.node_id`)

Wraps the existing `service_fn` handler. Cannot use tower here — doorway is raw hyper.

- [ ] **Step 1: Derive a node slug on startup**

Edit `main.rs`. Near where `args.node_id` is read, add:

```rust
// Node slug used as `served_by` in correlation headers/logs.
// Derived from node_id: lowercase, non-alphanum → '-', trimmed.
let node_slug: String = args.node_id
    .chars()
    .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
    .collect::<String>()
    .trim_matches('-')
    .to_string();
info!("Node slug (served_by): {}", node_slug);
```

Store `node_slug` on `AppState` — add the field:

```rust
// src/server/http.rs  AppState
pub node_slug: String,
```

Pass it from `main.rs` when constructing AppState.

- [ ] **Step 2: Find the `service_fn` call site**

```bash
grep -n "service_fn" /projects/elohim/doorway/doorway-service/src/server/http.rs
```

Note the line where `service_fn(move |req| ...)` wraps the handler.

- [ ] **Step 3: Wrap it with context scoping + tracing span**

At the identified line, change:

```rust
// BEFORE
.serve_connection(io, service_fn(move |req| {
    let state = state.clone();
    async move { handle_request(state, req).await }
}))
```

To:

```rust
// AFTER
.serve_connection(io, service_fn(move |req| {
    let state = state.clone();
    async move {
        use crate::correlation::{RequestContext, REQUEST_CTX};
        use tracing::Instrument;
        let ctx = RequestContext::extract(req.headers(), &state.node_slug);
        let span = tracing::info_span!(
            "http_request",
            request_id = %ctx.request_id,
            target_peer = ctx.target_peer.as_deref().unwrap_or("-"),
            served_by = %ctx.served_by,
            method = %req.method(),
            path = %req.uri().path(),
        );
        REQUEST_CTX
            .scope(ctx.clone(), async move {
                let mut response = handle_request(state, req).await;
                // Echo IDs on EVERY response (success + error).
                let headers_mut = response.headers_mut();
                if let Ok(val) = ctx.request_id.parse() {
                    headers_mut.insert(crate::correlation::headers::REQUEST_ID, val);
                }
                if let Ok(val) = ctx.served_by.parse() {
                    headers_mut.insert(crate::correlation::headers::SERVED_BY, val);
                }
                response
            })
            .instrument(span)
            .await
    }
}))
```

(`handle_request` signature is unchanged — whatever it returns we rewrap with headers.)

- [ ] **Step 4: Verify it compiles and runs**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build --lib --bins
```

Expected: compiles. `AppState` struct initialization may need updates wherever it's constructed — follow the compiler errors.

- [ ] **Step 5: Write an integration test for the echo behavior**

```rust
// doorway/doorway-service/tests/request_correlation_inbound.rs
//! Integration test: every response includes X-Request-ID + X-Served-By.
//! Boots a minimal doorway on an ephemeral port and hits it with reqwest.

use std::net::SocketAddr;

#[tokio::test]
async fn echoes_client_supplied_request_id() {
    // This test requires a minimal-harness boot of doorway.
    // If a test harness doesn't exist yet, use the in-process 'handle_request'
    // directly with a fabricated state — see `src/server/http.rs` test fixtures.
    let port = test_harness::start_doorway_for_test().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("http://127.0.0.1:{port}/health"))
        .header("x-request-id", "11111111-2222-4333-8444-555555555555")
        .send()
        .await
        .unwrap();

    assert_eq!(
        resp.headers().get("x-request-id").unwrap(),
        "11111111-2222-4333-8444-555555555555"
    );
    assert!(resp.headers().get("x-served-by").is_some());
}

#[tokio::test]
async fn generates_request_id_when_absent() {
    let port = test_harness::start_doorway_for_test().await;
    let resp = reqwest::get(format!("http://127.0.0.1:{port}/health"))
        .await
        .unwrap();
    let id = resp.headers().get("x-request-id").unwrap().to_str().unwrap();
    // UUID v4 format
    assert_eq!(id.len(), 36);
    assert_eq!(id.chars().nth(14).unwrap(), '4');
}
```

If `test_harness::start_doorway_for_test` doesn't exist, note as follow-up task — for now, add the same two assertions via an **in-process** test using `handle_request` directly on a fabricated `AppState` (see existing `#[cfg(test)]` patterns in `http.rs` around line 1860 `gate_layer_tests`).

- [ ] **Step 6: Run integration tests**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --test request_correlation_inbound -- --nocapture
```

Expected: both tests pass.

- [ ] **Step 7: Commit**

```bash
git add doorway/doorway-service/src/main.rs \
        doorway/doorway-service/src/server/http.rs \
        doorway/doorway-service/tests/request_correlation_inbound.rs
git commit -m "feat(correlation): scope every inbound request in RequestContext span; echo IDs on responses"
```

### Task 2.3: Update error helpers to include `requestId` + `servedBy` in body

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` — `bad_request_response`, `not_found_response`, inline 500-error builders

Error bodies are JSON `{error, message, statusCode}`. We extend them with `requestId` and `servedBy` pulled from `REQUEST_CTX`.

- [ ] **Step 1: Write the failing test**

```rust
// doorway/doorway-service/src/server/http.rs — new #[cfg(test)] mod tests block
#[cfg(test)]
mod correlation_error_tests {
    use super::*;
    use crate::correlation::{RequestContext, REQUEST_CTX};

    #[tokio::test]
    async fn bad_request_body_includes_request_id() {
        let ctx = RequestContext {
            request_id: "11111111-2222-4333-8444-555555555555".into(),
            target_peer: None,
            served_by: "doorway-test".into(),
        };
        let body = REQUEST_CTX
            .scope(ctx, async {
                let resp = bad_request_response("bad input");
                let b = resp.into_body();
                http_body_util::BodyExt::collect(b).await.unwrap().to_bytes()
            })
            .await;
        let parsed: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed["requestId"], "11111111-2222-4333-8444-555555555555");
        assert_eq!(parsed["servedBy"], "doorway-test");
        assert_eq!(parsed["error"], "Bad Request");
        assert_eq!(parsed["message"], "bad input");
    }

    #[tokio::test]
    async fn bad_request_body_without_context_still_renders() {
        // Outside of REQUEST_CTX scope, requestId/servedBy are omitted.
        let resp = bad_request_response("bad input");
        let b = resp.into_body();
        let bytes = http_body_util::BodyExt::collect(b).await.unwrap().to_bytes();
        let parsed: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(parsed["error"], "Bad Request");
        assert!(parsed.get("requestId").is_none() || parsed["requestId"].is_null());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib correlation_error_tests
```

Expected: FAIL — body does not contain requestId.

- [ ] **Step 3: Update `bad_request_response` and `not_found_response`**

Replace at http.rs:1830 (not_found) and http.rs:1845 (bad_request) with context-aware versions:

```rust
fn merge_correlation(mut body: serde_json::Value) -> serde_json::Value {
    if let Some(ctx) = crate::correlation::current() {
        body["requestId"] = serde_json::Value::String(ctx.request_id);
        body["servedBy"]  = serde_json::Value::String(ctx.served_by);
        if let Some(peer) = ctx.target_peer {
            body["targetPeer"] = serde_json::Value::String(peer);
        }
    }
    body
}

fn not_found_response(path: &str) -> Response<Full<Bytes>> {
    let body = merge_correlation(serde_json::json!({
        "error": "Not Found",
        "path": path,
        "hint": "Use WebSocket connection to /admin or /app/:port"
    }));
    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}

fn bad_request_response(message: &str) -> Response<Full<Bytes>> {
    let body = merge_correlation(serde_json::json!({
        "error": "Bad Request",
        "message": message
    }));
    Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap()
}
```

- [ ] **Step 4: Sweep inline 500 builders**

Find every inline 500 response:

```bash
grep -n "INTERNAL_SERVER_ERROR\|StatusCode::INTERNAL" /projects/elohim/doorway/doorway-service/src
```

For each location, wrap the JSON body through `merge_correlation`. Example — http.rs:1804:

```rust
// BEFORE
.body(Full::new(Bytes::from(
    r#"{"error": "Internal serialization error"}"#,
)))

// AFTER
.body(Full::new(Bytes::from(
    merge_correlation(serde_json::json!({
        "error": "Internal Server Error",
        "message": "Failed to serialize response"
    })).to_string()
)))
```

- [ ] **Step 5: Run the correlation tests**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib correlation_error_tests
```

Expected: both tests pass.

- [ ] **Step 6: Run the full test suite to catch regressions**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib --bins
```

Expected: all tests pass (no body-shape assertions elsewhere break on the new fields).

- [ ] **Step 7: Commit**

```bash
git add doorway/doorway-service/src/server/http.rs
git commit -m "feat(correlation): error response bodies include requestId + servedBy"
```

---

## Phase 3: Doorway — JSON Logs + Ring Buffer + /admin/logs

Without retrievable logs, correlation headers don't close the loop. This phase adds an in-process ring buffer (no external infra) and a query endpoint.

### Task 3.1: Switch tracing subscriber to JSON and add span fields automatically

**Files:**
- Modify: `doorway/doorway-service/src/main.rs` (subscriber init)

`tracing-subscriber` already has the `json` feature enabled in Cargo.toml. Switch the `fmt::layer()` to `json()` output so each line includes span fields (including `request_id`, `target_peer`, `served_by`).

- [ ] **Step 1: Update subscriber init in main.rs**

Find this block around line 40:

```rust
tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| format!("doorway={log_level},info").into()),
    )
    .with(tracing_subscriber::fmt::layer())
    .init();
```

Replace with:

```rust
let json_logs = std::env::var("DOORWAY_LOG_FORMAT").unwrap_or_default() == "json"
    || !args.dev_mode;
let fmt_layer: Box<dyn tracing_subscriber::Layer<_> + Send + Sync> = if json_logs {
    Box::new(
        tracing_subscriber::fmt::layer()
            .json()
            .with_current_span(true)
            .with_span_list(false)
    )
} else {
    Box::new(tracing_subscriber::fmt::layer())
};
tracing_subscriber::registry()
    .with(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| format!("doorway={log_level},info").into()),
    )
    .with(fmt_layer)
    .init();
```

JSON output in production/k8s; plaintext for local `dev-mode` unless `DOORWAY_LOG_FORMAT=json` is set.

- [ ] **Step 2: Verify the binary still boots**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo build --release
RUSTFLAGS="" ./target/release/doorway --help
```

Expected: help output prints without panic.

- [ ] **Step 3: Sanity-test JSON format locally**

```bash
DOORWAY_LOG_FORMAT=json RUSTFLAGS="" cargo run --release -- --help 2>&1 | head -5
```

Expected: lines are valid JSON where present. (Startup banners may still be `info!` macro output — acceptable.)

- [ ] **Step 4: Commit**

```bash
git add doorway/doorway-service/src/main.rs
git commit -m "feat(correlation): JSON tracing subscriber for structured logs"
```

### Task 3.2: Ring buffer layer + LogStore

**Files:**
- Create: `doorway/doorway-service/src/correlation/ring_buffer.rs`
- Modify: `doorway/doorway-service/src/correlation/mod.rs`
- Modify: `doorway/doorway-service/src/main.rs` (register the layer)

A bounded `VecDeque<LogEntry>` shared behind a `Mutex`. Each tracing event becomes an entry with fields extracted from the current span.

- [ ] **Step 1: Write the failing test**

```rust
// doorway/doorway-service/src/correlation/ring_buffer.rs  (bottom, #[cfg(test)])
#[cfg(test)]
mod tests {
    use super::*;
    use tracing_subscriber::{prelude::*, Registry};

    fn setup(capacity: usize) -> LogStore {
        let store = LogStore::new(capacity);
        let layer = RingBufferLayer::new(store.clone());
        let subscriber = Registry::default().with(layer);
        // Test-local default; do NOT set_global_default.
        let _guard = tracing::subscriber::set_default(subscriber);
        // Leak the guard deliberately for test duration.
        std::mem::forget(_guard);
        store
    }

    #[test]
    fn captures_events_into_ring() {
        let store = setup(100);
        tracing::info!("hello");
        let all = store.query(&AdminLogsQuery { limit: None, request_id: None, target_peer: None });
        assert!(all.entries.iter().any(|e| e.message == "hello"));
    }

    #[test]
    fn filters_by_request_id() {
        let store = setup(100);
        let ctx = crate::correlation::RequestContext {
            request_id: "11111111-2222-4333-8444-555555555555".into(),
            target_peer: None,
            served_by: "test".into(),
        };
        crate::correlation::REQUEST_CTX.sync_scope(ctx.clone(), || {
            let span = tracing::info_span!("scoped", request_id = %ctx.request_id);
            let _g = span.enter();
            tracing::info!("inside");
        });
        tracing::info!("outside");
        let r = store.query(&AdminLogsQuery {
            request_id: Some("11111111-2222-4333-8444-555555555555".into()),
            target_peer: None, limit: None,
        });
        assert!(r.entries.iter().any(|e| e.message == "inside"));
        assert!(r.entries.iter().all(|e| e.message != "outside"));
    }

    #[test]
    fn evicts_oldest_when_full() {
        let store = setup(2);
        tracing::info!("one");
        tracing::info!("two");
        tracing::info!("three");
        let all = store.query(&AdminLogsQuery { limit: None, request_id: None, target_peer: None });
        assert_eq!(all.entries.len(), 2);
        assert!(all.entries.iter().any(|e| e.message == "three"));
        assert!(all.entries.iter().all(|e| e.message != "one"));
    }
}
```

Note: `sync_scope` on task_local isn't available directly — use `REQUEST_CTX.scope(ctx, async { ... })` wrapped with `tokio::runtime::Runtime::new().unwrap().block_on(...)` in tests. Adjust test helper accordingly.

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib correlation::ring_buffer
```

Expected: FAIL — module not found.

- [ ] **Step 3: Write the implementation**

```rust
// doorway/doorway-service/src/correlation/ring_buffer.rs
//! In-process bounded log ring buffer, populated by a tracing Layer.
//! Used by /admin/logs for per-request correlation retrieval.

use super::types::{AdminLogsQuery, AdminLogsResponse, LogEntry};
use std::collections::{BTreeMap, VecDeque};
use std::sync::{Arc, Mutex};
use tracing::{Event, Subscriber};
use tracing_subscriber::layer::{Context, Layer};
use tracing_subscriber::registry::LookupSpan;

#[derive(Clone)]
pub struct LogStore {
    inner: Arc<Mutex<Inner>>,
}

struct Inner {
    capacity: usize,
    buffer: VecDeque<LogEntry>,
    /// Total events ever pushed — lets `truncated` flag detect eviction.
    total: u64,
}

impl LogStore {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(Inner {
                capacity,
                buffer: VecDeque::with_capacity(capacity),
                total: 0,
            })),
        }
    }

    pub fn push(&self, entry: LogEntry) {
        let mut g = self.inner.lock().unwrap();
        if g.buffer.len() >= g.capacity {
            g.buffer.pop_front();
        }
        g.buffer.push_back(entry);
        g.total += 1;
    }

    pub fn query(&self, q: &AdminLogsQuery) -> AdminLogsResponse {
        let g = self.inner.lock().unwrap();
        let limit = q.limit.unwrap_or(500).min(10_000);
        let filtered: Vec<LogEntry> = g
            .buffer
            .iter()
            .filter(|e| q.request_id.as_deref().map_or(true, |r| e.request_id.as_deref() == Some(r)))
            .filter(|e| q.target_peer.as_deref().map_or(true, |p| e.target_peer.as_deref() == Some(p)))
            .cloned()
            .take(limit)
            .collect();
        AdminLogsResponse {
            served_by: String::new(), // filled in by handler from AppState.node_slug
            query: q.clone(),
            truncated: g.buffer.len() < g.total as usize,
            entries: filtered,
        }
    }
}

pub struct RingBufferLayer {
    store: LogStore,
}

impl RingBufferLayer {
    pub fn new(store: LogStore) -> Self { Self { store } }
}

impl<S> Layer<S> for RingBufferLayer
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_event(&self, event: &Event<'_>, ctx: Context<'_, S>) {
        let metadata = event.metadata();
        let mut visitor = FieldVisitor::default();
        event.record(&mut visitor);

        let (request_id, target_peer, served_by) = collect_correlation_from_spans(&ctx, event)
            .or_else(|| collect_correlation_from_task_local())
            .unwrap_or_default();

        let entry = LogEntry {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: metadata.level().to_string(),
            target: metadata.target().to_string(),
            message: visitor.message.unwrap_or_default(),
            request_id,
            target_peer,
            served_by,
            fields: visitor.fields,
        };
        self.store.push(entry);
    }
}

type CorrelationTriplet = (Option<String>, Option<String>, Option<String>);

fn collect_correlation_from_spans<S>(ctx: &Context<'_, S>, event: &Event<'_>) -> Option<CorrelationTriplet>
where
    S: Subscriber + for<'a> LookupSpan<'a>,
{
    let scope = ctx.event_scope(event)?;
    for span in scope.from_root() {
        if let Some(ext) = span.extensions().get::<SpanCorrelation>() {
            return Some((ext.request_id.clone(), ext.target_peer.clone(), ext.served_by.clone()));
        }
    }
    None
}

fn collect_correlation_from_task_local() -> Option<CorrelationTriplet> {
    let ctx = super::current()?;
    Some((Some(ctx.request_id), ctx.target_peer, Some(ctx.served_by)))
}

/// Per-span correlation triplet — recorded once when the span is created via
/// a companion Layer that reads span fields. For now the task-local path is
/// sufficient; SpanCorrelation is a placeholder for future optimization.
#[derive(Clone, Default)]
struct SpanCorrelation {
    request_id: Option<String>,
    target_peer: Option<String>,
    served_by: Option<String>,
}

#[derive(Default)]
struct FieldVisitor {
    message: Option<String>,
    fields: BTreeMap<String, serde_json::Value>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.message = Some(format!("{:?}", value).trim_matches('"').to_string());
        } else {
            self.fields.insert(field.name().to_string(), serde_json::Value::String(format!("{:?}", value)));
        }
    }
    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = Some(value.to_string());
        } else {
            self.fields.insert(field.name().to_string(), serde_json::Value::String(value.to_string()));
        }
    }
    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(field.name().to_string(), serde_json::Value::from(value));
    }
    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(field.name().to_string(), serde_json::Value::from(value));
    }
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_string(), serde_json::Value::from(value));
    }
}
```

- [ ] **Step 4: Wire the layer into main.rs**

Extend the subscriber builder:

```rust
// main.rs after the fmt_layer setup
let log_store = crate::correlation::ring_buffer::LogStore::new(10_000);
let ring_layer = crate::correlation::ring_buffer::RingBufferLayer::new(log_store.clone());

tracing_subscriber::registry()
    .with(tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| format!("doorway={log_level},info").into()))
    .with(fmt_layer)
    .with(ring_layer)
    .init();
```

Store `log_store` on `AppState`:

```rust
pub log_store: Arc<crate::correlation::ring_buffer::LogStore>,
```

(Use `Arc::new(log_store)` when building AppState.)

- [ ] **Step 5: Add `chrono` if missing** (already present in doorway Cargo.toml)

- [ ] **Step 6: Run tests to verify they pass**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib correlation::ring_buffer
```

Expected: 3 tests pass.

- [ ] **Step 7: Commit**

```bash
git add doorway/doorway-service/src/correlation/ring_buffer.rs \
        doorway/doorway-service/src/correlation/mod.rs \
        doorway/doorway-service/src/main.rs \
        doorway/doorway-service/src/server/http.rs
git commit -m "feat(correlation): ring-buffer log layer + LogStore on AppState"
```

### Task 3.3: Add `GET /admin/logs` endpoint

**Files:**
- Create: `doorway/doorway-service/src/routes/admin_logs.rs`
- Modify: `doorway/doorway-service/src/routes/mod.rs` (export)
- Modify: `doorway/doorway-service/src/server/http.rs` (dispatch match arm)

- [ ] **Step 1: Write the failing test**

```rust
// doorway/doorway-service/src/routes/admin_logs.rs  (bottom, #[cfg(test)])
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_logs_for_request_id() {
        let store = crate::correlation::ring_buffer::LogStore::new(100);
        store.push(crate::correlation::types::LogEntry {
            timestamp: "2026-04-19T10:00:00Z".into(),
            level: "INFO".into(),
            target: "t".into(),
            message: "m".into(),
            request_id: Some("11111111-2222-4333-8444-555555555555".into()),
            target_peer: None,
            served_by: Some("doorway-test".into()),
            fields: Default::default(),
        });
        let query = "request_id=11111111-2222-4333-8444-555555555555";
        let resp = handle_admin_logs(&store, "doorway-test", query).await;
        assert_eq!(resp.status(), 200);
        let body = http_body_util::BodyExt::collect(resp.into_body())
            .await.unwrap().to_bytes();
        let parsed: crate::correlation::AdminLogsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.entries.len(), 1);
        assert_eq!(parsed.served_by, "doorway-test");
    }

    #[tokio::test]
    async fn returns_empty_for_unknown_request_id() {
        let store = crate::correlation::ring_buffer::LogStore::new(100);
        let resp = handle_admin_logs(&store, "doorway-test", "request_id=00000000-0000-4000-8000-000000000000").await;
        let body = http_body_util::BodyExt::collect(resp.into_body())
            .await.unwrap().to_bytes();
        let parsed: crate::correlation::AdminLogsResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(parsed.entries.len(), 0);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib routes::admin_logs
```

Expected: FAIL — module not found.

- [ ] **Step 3: Write the implementation**

```rust
// doorway/doorway-service/src/routes/admin_logs.rs
//! GET /admin/logs — query ring-buffered tracing events by request_id or target_peer.
//! Auth: requires ADMIN_API_KEY via the X-Admin-Key header (handled by the caller dispatch in http.rs).

use crate::correlation::{
    ring_buffer::LogStore,
    types::{AdminLogsQuery, AdminLogsResponse},
};
use bytes::Bytes;
use http_body_util::Full;
use hyper::{Response, StatusCode};

type BoxBody = http_body_util::combinators::BoxBody<Bytes, hyper::Error>;

pub async fn handle_admin_logs(
    store: &LogStore,
    served_by: &str,
    query_string: &str,
) -> Response<Full<Bytes>> {
    let query = parse_query(query_string);
    let mut resp_body = store.query(&query);
    resp_body.served_by = served_by.to_string();

    let json = match serde_json::to_string(&resp_body) {
        Ok(s) => s,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error":"serialize","message":"{e}"}}"#
                ))))
                .unwrap();
        }
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .header("Cache-Control", "no-store")
        .body(Full::new(Bytes::from(json)))
        .unwrap()
}

fn parse_query(s: &str) -> AdminLogsQuery {
    let mut q = AdminLogsQuery { request_id: None, target_peer: None, limit: None };
    for pair in s.split('&').filter(|p| !p.is_empty()) {
        let mut it = pair.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = urlencoding::decode(it.next().unwrap_or("")).ok().map(|c| c.into_owned());
        match (k, v) {
            ("request_id", Some(v))  => q.request_id = Some(v),
            ("target_peer", Some(v)) => q.target_peer = Some(v),
            ("limit", Some(v))       => q.limit = v.parse().ok(),
            _ => {}
        }
    }
    q
}
```

- [ ] **Step 4: Dispatch from http.rs**

Find the admin dispatch block (grep for `/admin/routes`). Add above it:

```rust
(Method::GET, p) if p == "/admin/logs" => {
    require_admin(&req, &state)?;
    let qs = req.uri().query().unwrap_or("");
    let resp = routes::admin_logs::handle_admin_logs(
        &state.log_store, &state.node_slug, qs,
    ).await;
    return Ok(to_boxed(resp));
}
```

(`require_admin` already exists for the admin routes — reuse it. If its name differs, match the existing pattern in that block.)

- [ ] **Step 5: Export from routes/mod.rs**

```rust
pub mod admin_logs;
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib routes::admin_logs
```

Expected: both tests pass.

- [ ] **Step 7: End-to-end smoke test**

```bash
# In one terminal
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo run --release &

# In another
curl -H "x-request-id: 11111111-2222-4333-8444-555555555555" \
     http://localhost:8888/health
curl -H "X-Admin-Key: $ADMIN_API_KEY" \
     "http://localhost:8888/admin/logs?request_id=11111111-2222-4333-8444-555555555555"
```

Expected: the admin/logs response contains at least one entry for that request ID with message "Processing request" or similar.

- [ ] **Step 8: Commit**

```bash
git add doorway/doorway-service/src/routes/admin_logs.rs \
        doorway/doorway-service/src/routes/mod.rs \
        doorway/doorway-service/src/server/http.rs
git commit -m "feat(correlation): GET /admin/logs endpoint with request_id/target_peer filtering"
```

---

## Phase 4: Doorway Outbound — Propagation + Peer Routing

Now that doorway owns request IDs, make sure they travel DOWN (to elohim-storage) and ACROSS (to federation peers). Add peer-slug routing via `X-Target-Peer`.

### Task 4.1: Propagate correlation headers on storage proxy calls

**Files:**
- Modify: `doorway/doorway-service/src/routes/storage_proxy.rs`

Every call to elohim-storage through `forward_to_storage` must forward `X-Request-ID` and `X-Target-Peer` from the originating request.

- [ ] **Step 1: Read the existing `forward_to_storage` signature**

```bash
grep -n "pub async fn forward_to_storage" /projects/elohim/doorway/doorway-service/src/routes/storage_proxy.rs
```

Note its signature and how it builds the outbound `reqwest` request.

- [ ] **Step 2: Write the failing test**

```rust
// doorway/doorway-service/src/routes/storage_proxy.rs  (bottom, #[cfg(test)])
#[cfg(test)]
mod propagation_tests {
    use super::*;

    #[tokio::test]
    async fn propagates_request_id_to_downstream() {
        // Boot a mock HTTP server capturing inbound headers.
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let (tx, mut rx) = tokio::sync::mpsc::channel::<hyper::HeaderMap>(1);

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let io = hyper_util::rt::TokioIo::new(stream);
            hyper::server::conn::http1::Builder::new()
                .serve_connection(io, hyper::service::service_fn(move |req: hyper::Request<hyper::body::Incoming>| {
                    let tx = tx.clone();
                    async move {
                        tx.send(req.headers().clone()).await.unwrap();
                        Ok::<_, hyper::Error>(hyper::Response::new(http_body_util::Empty::<bytes::Bytes>::new()))
                    }
                }))
                .await
                .unwrap();
        });

        let ctx = crate::correlation::RequestContext {
            request_id: "11111111-2222-4333-8444-555555555555".into(),
            target_peer: Some("terrance-household".into()),
            served_by: "doorway-alpha".into(),
        };
        crate::correlation::REQUEST_CTX
            .scope(ctx, async move {
                // Call the helper that adds correlation headers to the builder
                let client = reqwest::Client::new();
                let rb = client.get(format!("http://{addr}/whatever"));
                let rb = apply_correlation_headers(rb);
                rb.send().await.unwrap();
            })
            .await;

        let captured = rx.recv().await.unwrap();
        assert_eq!(
            captured.get("x-request-id").and_then(|v| v.to_str().ok()),
            Some("11111111-2222-4333-8444-555555555555")
        );
        assert_eq!(
            captured.get("x-target-peer").and_then(|v| v.to_str().ok()),
            Some("terrance-household")
        );
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib propagation_tests
```

Expected: FAIL — `apply_correlation_headers` not defined.

- [ ] **Step 4: Write the helper + wire into forward_to_storage**

Add to `storage_proxy.rs`:

```rust
/// Attach X-Request-ID (and X-Target-Peer if present) to an outbound reqwest builder
/// using the current task-local RequestContext.
pub fn apply_correlation_headers(mut rb: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    if let Some(ctx) = crate::correlation::current() {
        rb = rb.header(crate::correlation::headers::REQUEST_ID, &ctx.request_id);
        if let Some(peer) = &ctx.target_peer {
            rb = rb.header(crate::correlation::headers::TARGET_PEER, peer);
        }
    }
    rb
}
```

Then inside `forward_to_storage`, wherever the outbound request is built with `client.request(method, url)` or similar, insert:

```rust
let rb = apply_correlation_headers(rb);
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib propagation_tests storage_proxy
```

Expected: test passes; existing storage_proxy tests also pass.

- [ ] **Step 6: Commit**

```bash
git add doorway/doorway-service/src/routes/storage_proxy.rs
git commit -m "feat(correlation): forward X-Request-ID + X-Target-Peer on storage proxy"
```

### Task 4.2: Route to target peer when `X-Target-Peer` names a federation peer

**Files:**
- Modify: `doorway/doorway-service/src/server/http.rs` (dispatch: check target-peer before local routing)
- Modify: `doorway/doorway-service/src/services/federation.rs` (add `resolve_peer_by_slug`)

Logic: if `X-Target-Peer` is set AND differs from the local `node_slug` AND matches a federation peer's slug → proxy to that peer's doorway URL. Otherwise handle locally (including RouteRegistry fallback).

- [ ] **Step 1: Find federation peer slug field**

```bash
grep -n "peer_id\|node_id\|slug" /projects/elohim/doorway/doorway-service/src/services/federation.rs | head
```

Note the peer struct's identifier field. If peers don't already carry a `slug`, derive one from `peer_id` (lowercase alphanumeric-hyphen) as a temporary mapping.

- [ ] **Step 2: Write the failing test**

```rust
// doorway/doorway-service/src/services/federation.rs  (bottom, #[cfg(test)])
#[cfg(test)]
mod peer_slug_tests {
    use super::*;

    #[tokio::test]
    async fn resolves_peer_url_by_slug() {
        let cache = PeerCache::default();
        cache.insert(PeerDoorway {
            peer_id: "terrance-household".into(),
            doorway_url: "https://terrance.peer".into(),
            // ... fill other required fields with defaults
            ..Default::default()
        }).await;
        let url = resolve_peer_by_slug(&cache, "terrance-household").await;
        assert_eq!(url.as_deref(), Some("https://terrance.peer"));
    }

    #[tokio::test]
    async fn returns_none_for_unknown_slug() {
        let cache = PeerCache::default();
        let url = resolve_peer_by_slug(&cache, "ghost").await;
        assert!(url.is_none());
    }
}
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib peer_slug_tests
```

Expected: FAIL — function not defined.

- [ ] **Step 4: Implement `resolve_peer_by_slug`**

```rust
// doorway/doorway-service/src/services/federation.rs
/// Resolve a peer slug to a doorway URL via the federation peer cache.
pub async fn resolve_peer_by_slug(cache: &PeerCache, slug: &str) -> Option<String> {
    cache
        .all()
        .await
        .into_iter()
        .find(|p| p.peer_id == slug)
        .map(|p| p.doorway_url)
}
```

(If `peer_id` is not a slug, add a normalized `slug` field or derive lazily. Follow whatever the `PeerCache` already exposes — this test drives the right shape.)

- [ ] **Step 5: Wire dispatch in `http.rs`**

Near the top of `handle_request`, after the RequestContext is available via task-local:

```rust
// Peer routing: if target peer is set AND it's not us AND we know it, proxy.
if let Some(ctx) = crate::correlation::current() {
    if let Some(target) = ctx.target_peer.as_deref() {
        if target != state.node_slug {
            if let Some(peer_url) = services::federation::resolve_peer_by_slug(
                &state.peer_cache, target,
            ).await {
                let proxied = routes::federation_proxy::proxy_to_peer(
                    &peer_url, req,
                ).await;
                return Ok(to_boxed(proxied));
            }
            // Unknown peer — fall through to local handling; include a header
            // in the response so the client knows we didn't route.
            tracing::warn!(target = %target, "X-Target-Peer not in federation cache; handling locally");
        }
    }
}
```

- [ ] **Step 6: Add `routes::federation_proxy::proxy_to_peer`**

```rust
// doorway/doorway-service/src/routes/federation_proxy.rs
//! Forward a request to another doorway as-is, preserving method, path, body,
//! and correlation headers.

use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper::{Request, Response, StatusCode};

pub async fn proxy_to_peer(
    peer_url: &str,
    req: Request<Incoming>,
) -> Response<Full<Bytes>> {
    let client = reqwest::Client::new();
    let url = format!("{}{}", peer_url.trim_end_matches('/'), req.uri().path_and_query().map(|p| p.as_str()).unwrap_or("/"));
    let method = req.method().clone();

    // Collect body
    let (parts, body) = req.into_parts();
    let body_bytes = match body.collect().await {
        Ok(b) => b.to_bytes(),
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Full::new(Bytes::from(format!(
                    r#"{{"error":"upstream-body","message":"{e}"}}"#
                ))))
                .unwrap();
        }
    };

    let mut rb = client.request(method, url).body(body_bytes);
    // Forward ALL headers as-is
    for (k, v) in parts.headers.iter() {
        rb = rb.header(k.as_str(), v.as_bytes());
    }

    match rb.send().await {
        Ok(resp) => {
            let status = resp.status();
            let mut out = Response::builder().status(status);
            for (k, v) in resp.headers().iter() {
                out = out.header(k.as_str(), v.as_bytes());
            }
            let bytes = resp.bytes().await.unwrap_or_default();
            out.body(Full::new(bytes)).unwrap()
        }
        Err(e) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Full::new(Bytes::from(format!(
                r#"{{"error":"upstream","message":"{e}"}}"#
            ))))
            .unwrap(),
    }
}
```

Register in `routes/mod.rs`:

```rust
pub mod federation_proxy;
```

- [ ] **Step 7: Run tests to verify they pass**

```bash
cd /projects/elohim/doorway/doorway-service
RUSTFLAGS="" cargo test --lib peer_slug_tests federation_proxy
```

Expected: both pass.

- [ ] **Step 8: Commit**

```bash
git add doorway/doorway-service/src/services/federation.rs \
        doorway/doorway-service/src/server/http.rs \
        doorway/doorway-service/src/routes/federation_proxy.rs \
        doorway/doorway-service/src/routes/mod.rs
git commit -m "feat(correlation): X-Target-Peer slug routes to federation peer's doorway"
```

---

## Phase 5: Elohim-Storage Mirror

Same inbound middleware, JSON logs, ring buffer, and `/admin/logs` — now on the peer runtime so tests can correlate across the hop.

### Task 5.1: Mirror the correlation module

**Files:**
- Create (mirror from doorway): `elohim/elohim-storage/src/correlation/context.rs`, `ring_buffer.rs`
- Modify: `elohim/elohim-storage/src/correlation/mod.rs`, `lib.rs`, `main.rs`

- [ ] **Step 1: Copy the correlation module from doorway**

```bash
cp /projects/elohim/doorway/doorway-service/src/correlation/context.rs \
   /projects/elohim/elohim/elohim-storage/src/correlation/context.rs
cp /projects/elohim/doorway/doorway-service/src/correlation/ring_buffer.rs \
   /projects/elohim/elohim/elohim-storage/src/correlation/ring_buffer.rs
```

- [ ] **Step 2: Fix up module paths in the copied files**

The files reference `crate::correlation` — that's unchanged. Update any `crate::server::AppState` references; storage's handler state is named differently. Search for those imports and adjust.

- [ ] **Step 3: Enable JSON feature for tracing-subscriber in storage Cargo.toml**

Edit `elohim/elohim-storage/Cargo.toml`:

```toml
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
```

- [ ] **Step 4: Add `uuid` to storage Cargo.toml if absent**

```bash
grep "^uuid" /projects/elohim/elohim/elohim-storage/Cargo.toml
```

If absent, add:

```toml
uuid = { version = "1.16", features = ["v4"] }
```

- [ ] **Step 5: Compile**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo build --lib
```

Expected: compiles.

- [ ] **Step 6: Run the ported tests**

```bash
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib correlation
```

Expected: the 6 context tests + 3 ring-buffer tests pass.

- [ ] **Step 7: Commit**

```bash
git add elohim/elohim-storage/src/correlation/ \
        elohim/elohim-storage/Cargo.toml Cargo.lock
git commit -m "feat(correlation): mirror correlation module into elohim-storage"
```

### Task 5.2: Wire middleware + `/admin/logs` into elohim-storage

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs` (top-of-handler wrap)
- Modify: `elohim/elohim-storage/src/main.rs` (subscriber + log_store + node slug)
- Create: `elohim/elohim-storage/src/api/admin_logs.rs`

- [ ] **Step 1: Compute node slug in main.rs**

Derive from `args.node_id` or similar config value — same pattern as doorway.

- [ ] **Step 2: Switch subscriber to JSON + register ring-buffer layer**

Mirror the doorway change (Phase 3 Task 3.1 & 3.2) — same `DOORWAY_LOG_FORMAT` trigger but read from `STORAGE_LOG_FORMAT`. Register `RingBufferLayer` with a 10k-entry `LogStore`.

- [ ] **Step 3: Scope requests inside RequestContext span**

Find the storage http handler entry point (in `src/http.rs` — it doesn't use `service_fn` directly but a similar pattern). Wrap it the same way as doorway, using `REQUEST_CTX.scope(ctx, ...).instrument(span)`.

Apply response-header echo in the same outermost wrapper.

- [ ] **Step 4: Update error helpers**

Find storage's error response builders (search for `StatusCode::` in `src/http.rs`). Apply the `merge_correlation` pattern from doorway Phase 2 Task 2.3.

- [ ] **Step 5: Add `/admin/logs` endpoint**

Mirror doorway's Phase 3 Task 3.3 — create `src/api/admin_logs.rs`, add a match arm in storage's http.rs dispatch, register the route in storage's manifest builder if admin endpoints go through the manifest (they may not — check `build_manifest()`).

- [ ] **Step 6: Write an integration test for end-to-end correlation**

```rust
// elohim/elohim-storage/tests/correlation_integration.rs
#[tokio::test]
async fn storage_echoes_client_request_id() {
    // Boot storage on ephemeral port, hit a health endpoint with x-request-id,
    // assert it's echoed + admin/logs returns entries for that ID.
    // ... (use existing storage test harness if present)
}
```

- [ ] **Step 7: Run tests + compile**

```bash
cd /projects/elohim/elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --lib --bins
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --test correlation_integration
```

Expected: all pass.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/http.rs \
        elohim/elohim-storage/src/main.rs \
        elohim/elohim-storage/src/api/admin_logs.rs \
        elohim/elohim-storage/src/api/mod.rs \
        elohim/elohim-storage/tests/correlation_integration.rs
git commit -m "feat(correlation): elohim-storage inbound request-id middleware + /admin/logs"
```

### Task 5.3: Smoke-test end-to-end correlation through a full stack

**Files:** none (verification only)

- [ ] **Step 1: Boot the full stack locally**

```bash
cd /projects/elohim/app/elohim-app
pnpm run hc:start
```

Wait until doorway is listening on 8888 and storage on 8090.

- [ ] **Step 2: Make a request through doorway that proxies to storage**

```bash
curl -v \
  -H "x-request-id: 11111111-2222-4333-8444-555555555555" \
  -H "x-target-peer: doorway-alpha" \
  http://localhost:8888/db/content 2>&1 | grep -i 'x-request-id\|x-served-by'
```

Expected: response has `x-request-id: 11111111-...` and `x-served-by: doorway-alpha` (or local node slug).

- [ ] **Step 3: Query doorway's admin/logs**

```bash
curl -H "X-Admin-Key: $ADMIN_API_KEY" \
  "http://localhost:8888/admin/logs?request_id=11111111-2222-4333-8444-555555555555"
```

Expected: returns entries recording the request.

- [ ] **Step 4: Query storage's admin/logs**

```bash
curl "http://localhost:8090/admin/logs?request_id=11111111-2222-4333-8444-555555555555"
```

Expected: returns entries from storage's side of the proxy, same request ID.

- [ ] **Step 5: If any of steps 2-4 fail, debug and fix before continuing. Commit no changes yet — this is a verification gate.**

---

## Phase 6: a2o Integration

Now that every runtime echoes IDs and serves logs by request-ID, teach the a2o framework to capture IDs, declare target peers per persona, and fetch backend logs on failure.

### Task 6.1: Persona → peer-slug mapping

**Files:**
- Create: `genesis/a2o/src/framework/persona-peer-mapping.ts`
- Create: `genesis/a2o/src/framework/__tests__/persona-peer-mapping.test.ts`
- Modify: `genesis/a2o/src/framework/world.ts` (load mapping into world parameters)

- [ ] **Step 1: Write the mapping as data**

```typescript
// genesis/a2o/src/framework/persona-peer-mapping.ts
/**
 * Persona → target-peer slug. The a2o framework uses this to set X-Target-Peer
 * on outbound requests so doorway knows which peer should handle them.
 *
 * Not all personas have a peer — visitors and anonymous scenarios send no slug.
 */
export const PERSONA_PEER: Record<string, string> = {
  terrance: 'terrance-household',
  mary:    'mary-household',
  shem:    'shem',             // shem is its own peer (the live P2P canvas)
  // Anonymous / fixture humans without a peer assignment: omit.
};

export function peerForPersona(persona: string): string | undefined {
  return PERSONA_PEER[persona.toLowerCase()];
}
```

- [ ] **Step 2: Write the failing test**

```typescript
// genesis/a2o/src/framework/__tests__/persona-peer-mapping.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { peerForPersona, PERSONA_PEER } from '../persona-peer-mapping.js';

describe('peerForPersona', () => {
  it('maps Terrance to terrance-household', () => {
    assert.equal(peerForPersona('Terrance'), 'terrance-household');
  });
  it('is case-insensitive on input', () => {
    assert.equal(peerForPersona('TIMOTHY'), PERSONA_PEER.terrance);
  });
  it('returns undefined for unmapped personas', () => {
    assert.equal(peerForPersona('NotARealHuman'), undefined);
  });
});
```

- [ ] **Step 3: Run test to verify it passes**

```bash
cd /projects/elohim/genesis/a2o
npx tsx --test src/framework/__tests__/persona-peer-mapping.test.ts
```

Expected: all tests pass.

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/src/framework/persona-peer-mapping.ts \
        genesis/a2o/src/framework/__tests__/persona-peer-mapping.test.ts
git commit -m "feat(a2o): persona → peer-slug mapping"
```

### Task 6.2: Capture X-Request-ID + X-Served-By from every response

**Files:**
- Modify: `genesis/a2o/src/framework/http-client.ts` (or wherever the shared `fetch` wrapper lives)
- Modify: `genesis/a2o/src/framework/devices/playwright-device.ts`
- Modify: `genesis/a2o/src/framework/world.ts` (add `requestTrace` storage on E2EWorld)

Each scenario gets a `requestTrace: RequestTraceEntry[]` recording every ID seen, along with the peer that answered.

- [ ] **Step 1: Define the trace type and add to world**

Inside `world.ts`:

```typescript
export interface RequestTraceEntry {
  requestId: string;
  servedBy?: string;
  targetPeer?: string;
  method: string;
  url: string;
  status: number;
  at: string; // ISO8601
}

export class E2EWorld {
  // ... existing fields ...
  public requestTrace: RequestTraceEntry[] = [];
  public recordRequest(entry: RequestTraceEntry): void {
    this.requestTrace.push(entry);
  }
}
```

- [ ] **Step 2: Wrap the shared fetch helper**

Find the shared `fetch`/`undici` call site and wrap so every request:

1. Adds `X-Request-ID` (generate UUID client-side) and `X-Target-Peer` (if `world.targetPeer` set)
2. Reads `x-request-id` and `x-served-by` from the response, calls `world.recordRequest(...)`

Example skeleton:

```typescript
import { randomUUID } from 'node:crypto';
import type { E2EWorld } from './world.js';

export async function tracedFetch(
  world: E2EWorld,
  url: string,
  init: RequestInit = {}
): Promise<Response> {
  const requestId = randomUUID();
  const headers = new Headers(init.headers);
  headers.set('x-request-id', requestId);
  if (world.targetPeer) headers.set('x-target-peer', world.targetPeer);
  const res = await fetch(url, { ...init, headers });
  world.recordRequest({
    requestId: res.headers.get('x-request-id') ?? requestId,
    servedBy: res.headers.get('x-served-by') ?? undefined,
    targetPeer: world.targetPeer,
    method: (init.method ?? 'GET').toUpperCase(),
    url,
    status: res.status,
    at: new Date().toISOString(),
  });
  return res;
}
```

Replace direct `fetch(...)` calls in step defs with `tracedFetch(this, ...)` progressively — at minimum for the `adminGet` helper and any API step helpers.

- [ ] **Step 3: Mirror for Playwright**

In `playwright-device.ts`, attach to the page's request-finished event:

```typescript
page.on('requestfinished', async (request) => {
  const response = await request.response();
  if (!response) return;
  const headers = response.headers();
  world.recordRequest({
    requestId: headers['x-request-id'] ?? '',
    servedBy: headers['x-served-by'],
    targetPeer: world.targetPeer,
    method: request.method(),
    url: request.url(),
    status: response.status(),
    at: new Date().toISOString(),
  });
});
```

Add `page.setExtraHTTPHeaders` to include `X-Target-Peer` per scenario (set in the `Before` hook).

- [ ] **Step 4: Set `world.targetPeer` from persona in Background step**

In `steps/common.steps.ts`, when the step `Given human "{name}" is logged in on doorway ...` executes:

```typescript
const peer = peerForPersona(name);
if (peer) this.targetPeer = peer;
```

- [ ] **Step 5: Manual verification**

```bash
cd /projects/elohim/genesis/a2o
npx cucumber-js -p alpha --tags '@e2e and @auth' --dry-run=false 2>&1 | tail -30
```

Open the test run's `world.requestTrace` — either via a small debug step or inspection — confirm entries are being collected.

- [ ] **Step 6: Commit**

```bash
git add genesis/a2o/src/framework/
git commit -m "feat(a2o): capture X-Request-ID + X-Served-By on every response"
```

### Task 6.3: Fetch backend logs on scenario failure

**Files:**
- Modify: `genesis/a2o/steps/common.steps.ts` (After hook)
- Create: `genesis/a2o/src/framework/log-fetch.ts`

On failure, for each unique `(servedBy, requestId)` pair in `world.requestTrace`, fetch `{doorway}/admin/logs?request_id=X` and (if reachable) the peer's storage logs. Write to `reports/backend-logs/{scenario}-{servedBy}-{requestId}.json`.

- [ ] **Step 1: Write the log-fetch helper**

```typescript
// genesis/a2o/src/framework/log-fetch.ts
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname } from 'node:path';
import type { RequestTraceEntry } from './world.js';

interface FetchLogsOptions {
  adminDoorwayUrl: string;
  adminApiKey?: string;
}

export async function fetchBackendLogs(
  trace: RequestTraceEntry[],
  scenarioSafeName: string,
  outDir: string,
  opts: FetchLogsOptions
): Promise<{ written: string[]; errors: string[] }> {
  const written: string[] = [];
  const errors: string[] = [];
  const seen = new Set<string>();
  for (const e of trace) {
    if (!e.requestId) continue;
    const key = `${e.servedBy ?? 'unknown'}::${e.requestId}`;
    if (seen.has(key)) continue;
    seen.add(key);
    const url = `${opts.adminDoorwayUrl}/admin/logs?request_id=${encodeURIComponent(e.requestId)}`;
    try {
      const res = await fetch(url, {
        headers: opts.adminApiKey ? { 'x-admin-key': opts.adminApiKey } : {},
      });
      if (!res.ok) { errors.push(`${url}: ${res.status}`); continue; }
      const body = await res.text();
      const outPath = `${outDir}/${scenarioSafeName}-${e.servedBy ?? 'unknown'}-${e.requestId}.json`;
      mkdirSync(dirname(outPath), { recursive: true });
      writeFileSync(outPath, body);
      written.push(outPath);
    } catch (err) {
      errors.push(`${url}: ${String(err)}`);
    }
  }
  return { written, errors };
}
```

- [ ] **Step 2: Invoke from the After hook on failure**

In `steps/common.steps.ts` `After` hook, after existing console-capture logic, add:

```typescript
if (scenario.result?.status === Status.FAILED && this.requestTrace.length > 0) {
  const safeName = scenario.pickle.name.replace(/[^a-z0-9-]/gi, '-');
  const { written, errors } = await fetchBackendLogs(
    this.requestTrace,
    safeName,
    'reports/backend-logs',
    {
      adminDoorwayUrl: process.env.E2E_DOORWAY_ALPHA ?? 'http://localhost:8888',
      adminApiKey: process.env.ADMIN_API_KEY,
    }
  );
  if (written.length) console.error(`Backend logs captured: ${written.length} files`);
  if (errors.length)  console.error(`Log-fetch errors: ${errors.length}`);
}
```

- [ ] **Step 3: Verify by forcing a failure**

Add a throwaway `@test-forced-failure` scenario, run, and confirm `reports/backend-logs/*.json` is created.

- [ ] **Step 4: Commit**

```bash
git add genesis/a2o/src/framework/log-fetch.ts \
        genesis/a2o/steps/common.steps.ts
git commit -m "feat(a2o): fetch backend logs by request-id on scenario failure"
```

### Task 6.4: Extend sprint-report aggregator to include backend-log findings

**Files:**
- Modify: `genesis/a2o/scripts/lib/aggregate.ts` (optionally surface backend-log hints)
- Create: `genesis/a2o/scripts/lib/load-backend-logs.ts`
- Create: `genesis/a2o/scripts/__tests__/load-backend-logs.test.ts`

Treat backend logs as additional context attached to `scenario-failure` findings — not as new findings themselves (avoids double-counting).

- [ ] **Step 1: Write the loader**

```typescript
// genesis/a2o/scripts/lib/load-backend-logs.ts
import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { join } from 'node:path';

export interface BackendLogEntry {
  scenario: string;
  servedBy: string;
  requestId: string;
  errorCount: number;
  firstErrorMessage?: string;
}

const FILENAME_RE = /^(.+)-([a-z0-9-]+)-([0-9a-f-]{36})\.json$/;

export function loadBackendLogs(dir: string): BackendLogEntry[] {
  if (!existsSync(dir)) return [];
  const out: BackendLogEntry[] = [];
  for (const name of readdirSync(dir)) {
    const m = name.match(FILENAME_RE);
    if (!m) continue;
    const [, scenario, servedBy, requestId] = m;
    const body = JSON.parse(readFileSync(join(dir, name), 'utf8'));
    const entries = Array.isArray(body.entries) ? body.entries : [];
    const errors = entries.filter((e: { level?: string }) => e.level === 'ERROR');
    out.push({
      scenario,
      servedBy,
      requestId,
      errorCount: errors.length,
      firstErrorMessage: errors[0]?.message,
    });
  }
  return out;
}
```

- [ ] **Step 2: Write the test**

```typescript
// genesis/a2o/scripts/__tests__/load-backend-logs.test.ts
import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { loadBackendLogs } from '../lib/load-backend-logs.js';

function makeDir() {
  const dir = mkdtempSync(join(tmpdir(), 'a2o-bl-'));
  writeFileSync(
    join(dir, 'Learning-Journey-doorway-alpha-11111111-2222-4333-8444-555555555555.json'),
    JSON.stringify({ entries: [
      { level: 'INFO', message: 'ok' },
      { level: 'ERROR', message: 'boom' },
    ]})
  );
  return dir;
}

describe('loadBackendLogs', () => {
  it('parses filename into scenario/servedBy/requestId', () => {
    const result = loadBackendLogs(makeDir());
    assert.equal(result.length, 1);
    assert.equal(result[0].servedBy, 'doorway-alpha');
    assert.equal(result[0].errorCount, 1);
    assert.equal(result[0].firstErrorMessage, 'boom');
  });
  it('returns empty on missing dir', () => {
    assert.deepEqual(loadBackendLogs('/nope'), []);
  });
});
```

- [ ] **Step 3: Extend `aggregate.ts`**

Attach backend-log metadata as a new field on `scenario-failure` findings. Don't create new Finding rows — just decorate:

```typescript
// In aggregate(): after building findings, before sort:
for (const bl of backendLogs) {
  // Best-effort: match backend log to a scenario-failure finding by scenario name.
  const target = findings.find(f =>
    f.source === 'scenario-failure' && f.scenarios.some(s => s.name.includes(bl.scenario))
  );
  if (target) {
    (target as Finding & { backendLogs?: BackendLogEntry[] }).backendLogs =
      [...((target as Finding & { backendLogs?: BackendLogEntry[] }).backendLogs ?? []), bl];
  }
}
```

(Add `backendLogs?: BackendLogEntry[]` to the `Finding` type and update schema + renderer.)

- [ ] **Step 4: Run the aggregator tests**

```bash
cd /projects/elohim/genesis/a2o
npx tsx --test scripts/__tests__/
```

Expected: all pass.

- [ ] **Step 5: Commit**

```bash
git add genesis/a2o/scripts/ genesis/a2o/schemas/sprint-report.schema.json
git commit -m "feat(a2o): sprint-report includes backend-log correlation per scenario failure"
```

---

## Self-Review Checklist

- [x] Phase 1 defines wire contract BEFORE any runtime code (schema-first)
- [x] Every phase produces a deployable increment with passing tests
- [x] `RequestContext` struct is consistent across all tasks (request_id, target_peer, served_by)
- [x] `REQUEST_CTX` task-local is established in Phase 2 Task 2.1 before being read in Phase 2 Task 2.3 and Phase 4 Task 4.1
- [x] `LogStore` is built in Phase 3 Task 3.2 before being queried in Phase 3 Task 3.3
- [x] `resolve_peer_by_slug` (Phase 4 Task 4.2) pre-dates the dispatch logic that uses it in the same task
- [x] Elohim-storage mirror (Phase 5) reuses doorway's module — no re-implementation
- [x] a2o capture hook (Phase 6 Task 6.2) is added before log-fetch (Task 6.3) needs it
- [x] Every error-response code path in doorway/storage is covered by the `merge_correlation` sweep
- [x] No placeholders — every step has runnable code or exact commands
- [x] Header names pulled from single source of truth (`correlation::headers` module)

## Execution Guidance

**Sprint-level split (one Objective per Phase):**

- Sprint 1 → Phase 1 + Phase 2 (~3 hours, wire contract + doorway request IDs are live)
- Sprint 2 → Phase 3 (~2 hours, doorway-side log retrieval works)
- Sprint 3 → Phase 4 (~3 hours, peer-slug routing + propagation)
- Sprint 4 → Phase 5 (~2 hours, storage mirror)
- Sprint 5 → Phase 6 (~3 hours, a2o picks it all up)

Total: roughly 13 hours sequential. Phases 1+2 and Phase 5 can run in parallel worktrees if desired; everything else is strictly sequential.

**Dependencies on Plan A:**
This plan depends on Plan A's aggregator existing (Task 6.4 modifies it). If Plans are executed in parallel, Phase 6 must wait for Plan A to be merged.

**Follow-up work out of scope for this plan:**
- Persisting ring buffer across restarts (current = lost on pod restart)
- Loki/Grafana integration (ring buffer is fine for now; external aggregation is future)
- Auth on `/admin/logs` beyond admin-key gate — OPA/JWT policies for read-only "Looker" roles
- Sampling/rate-limiting on log ingestion (not needed at current volume)
