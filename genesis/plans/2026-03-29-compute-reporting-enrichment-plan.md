# Compute Reporting Enrichment — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Add `BuildInfo` and `DetailLevel` to `elohim-compute`, bring storage to parity with doorway's version/health reporting, and label all compute report fields with standard log levels.

**Architecture:** `elohim-compute` (shared crate) owns `BuildInfo`, `DetailLevel`, and a `filter()` function. Both doorway and storage populate `ComputeReport` with real build info. Storage gets a `/version` endpoint and enriched `/health`. P2P identify advertises commit hash. The Dockerfile and Jenkinsfile pass build args.

**Tech Stack:** Rust (serde, chrono, tracing), Docker build args, Jenkins pipeline

**Design doc:** `genesis/plans/2026-03-29-compute-reporting-enrichment-design.md`

---

### Task 1: Add `BuildInfo` and `DetailLevel` to `elohim-compute`

**Files:**
- Create: `elohim/elohim-compute/src/build_info.rs`
- Create: `elohim/elohim-compute/src/detail_level.rs`
- Modify: `elohim/elohim-compute/src/lib.rs`
- Modify: `elohim/elohim-compute/src/report.rs`

**Step 1: Write failing tests for BuildInfo**

In a new file `elohim/elohim-compute/src/build_info.rs`:

```rust
//! Build-time identity for any service in the fleet.

use serde::{Deserialize, Serialize};

/// Build-time identity populated via `env!()` / `option_env!()` at compile time.
/// Every service constructs this once at startup.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    /// Cargo package version (e.g., "0.1.0")
    pub version: String,
    /// Short git commit hash (7 chars)
    pub commit: String,
    /// Full git commit hash (40 chars)
    pub commit_full: String,
    /// ISO 8601 build timestamp
    pub build_time: String,
    /// Rustc version used for compilation
    pub rustc_version: String,
    /// Service name (e.g., "elohim-doorway", "elohim-storage")
    pub service: String,
}

impl BuildInfo {
    /// Construct from compile-time environment variables.
    /// Each binary calls this once with its own `env!()` values.
    pub fn new(service: &str) -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").to_string(),
            commit: option_env!("GIT_COMMIT_SHORT").unwrap_or("unknown").to_string(),
            commit_full: option_env!("GIT_COMMIT_FULL").unwrap_or("unknown").to_string(),
            build_time: option_env!("BUILD_TIMESTAMP").unwrap_or("unknown").to_string(),
            rustc_version: option_env!("RUSTC_VERSION").unwrap_or("unknown").to_string(),
            service: service.to_string(),
        }
    }

    /// User-agent string for P2P identify and HTTP headers.
    /// Format: `{service}/{version}+{commit}`
    pub fn user_agent(&self) -> String {
        if self.commit == "unknown" {
            format!("{}/{}", self.service, self.version)
        } else {
            format!("{}/{}+{}", self.service, self.version, self.commit)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_info_new_populates_version() {
        let info = BuildInfo::new("test-service");
        assert_eq!(info.service, "test-service");
        assert_eq!(info.version, env!("CARGO_PKG_VERSION"));
        // commit will be "unknown" in test env (no build args)
    }

    #[test]
    fn test_serializes_camel_case() {
        let info = BuildInfo::new("test-service");
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"commitFull\""));
        assert!(json.contains("\"buildTime\""));
        assert!(json.contains("\"rustcVersion\""));
    }

    #[test]
    fn test_roundtrip() {
        let info = BuildInfo::new("test-service");
        let json = serde_json::to_string(&info).unwrap();
        let decoded: BuildInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.service, "test-service");
        assert_eq!(decoded.version, info.version);
    }

    #[test]
    fn test_user_agent_without_commit() {
        let mut info = BuildInfo::new("elohim-storage");
        info.commit = "unknown".to_string();
        assert_eq!(info.user_agent(), format!("elohim-storage/{}", info.version));
    }

    #[test]
    fn test_user_agent_with_commit() {
        let mut info = BuildInfo::new("elohim-storage");
        info.commit = "abc1234".to_string();
        assert_eq!(info.user_agent(), format!("elohim-storage/{}+abc1234", info.version));
    }
}
```

**Step 2: Write DetailLevel**

In a new file `elohim/elohim-compute/src/detail_level.rs`:

```rust
//! Standard log-level detail tiers for compute observability.
//!
//! Maps 1:1 to standard log levels (error/warn/info/debug/trace).
//! Used to label fields in ComputeReport and filter responses
//! based on the requester's access level.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Detail level for compute report filtering.
/// Ordered: Error < Warn < Info < Debug < Trace.
/// A request at level Debug sees everything at Debug and below.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    /// Always visible — active problems affecting the network
    Error,
    /// Conditions peers should know about
    Warn,
    /// Standard operational identity (default)
    Info,
    /// Internals useful for diagnosing issues
    Debug,
    /// Full internal state
    Trace,
}

impl Default for DetailLevel {
    fn default() -> Self {
        Self::Info
    }
}

impl fmt::Display for DetailLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "error"),
            Self::Warn => write!(f, "warn"),
            Self::Info => write!(f, "info"),
            Self::Debug => write!(f, "debug"),
            Self::Trace => write!(f, "trace"),
        }
    }
}

impl FromStr for DetailLevel {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "error" => Ok(Self::Error),
            "warn" | "warning" => Ok(Self::Warn),
            "info" => Ok(Self::Info),
            "debug" => Ok(Self::Debug),
            "trace" => Ok(Self::Trace),
            _ => Err(format!("Unknown detail level: '{}'. Use: error, warn, info, debug, trace", s)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ordering() {
        assert!(DetailLevel::Error < DetailLevel::Warn);
        assert!(DetailLevel::Warn < DetailLevel::Info);
        assert!(DetailLevel::Info < DetailLevel::Debug);
        assert!(DetailLevel::Debug < DetailLevel::Trace);
    }

    #[test]
    fn test_default_is_info() {
        assert_eq!(DetailLevel::default(), DetailLevel::Info);
    }

    #[test]
    fn test_from_str() {
        assert_eq!("debug".parse::<DetailLevel>().unwrap(), DetailLevel::Debug);
        assert_eq!("TRACE".parse::<DetailLevel>().unwrap(), DetailLevel::Trace);
        assert_eq!("warning".parse::<DetailLevel>().unwrap(), DetailLevel::Warn);
        assert!("garbage".parse::<DetailLevel>().is_err());
    }

    #[test]
    fn test_display() {
        assert_eq!(format!("{}", DetailLevel::Debug), "debug");
    }

    #[test]
    fn test_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&DetailLevel::Trace).unwrap(), "\"trace\"");
    }

    #[test]
    fn test_deserializes() {
        let d: DetailLevel = serde_json::from_str("\"debug\"").unwrap();
        assert_eq!(d, DetailLevel::Debug);
    }
}
```

**Step 3: Update `report.rs` — replace `version: String` with `build: BuildInfo`**

In `elohim/elohim-compute/src/report.rs`, replace the `version` field:

```rust
// OLD:
pub version: String,

// NEW:
/// [info] Build-time identity
pub build: BuildInfo,
```

Update `ComputeReport::build()`:

```rust
// OLD:
version: String::new(),

// NEW:
build: build_info.clone(),
```

Change the `build()` method signature to accept `&BuildInfo`:

```rust
pub fn build(
    reporter: &dyn HealthReporter,
    build_info: &BuildInfo,
    resources: ResourceSnapshot,
    peers: Vec<PeerHealthSnapshot>,
    extensions: serde_json::Value,
) -> Self {
```

Add a `filter()` method:

```rust
/// Filter the report to only include fields at or below the given detail level.
/// Returns a serde_json::Value with fields above the level removed.
pub fn filter(&self, level: DetailLevel) -> serde_json::Value {
    let mut val = serde_json::to_value(self).unwrap_or(serde_json::Value::Null);
    if let serde_json::Value::Object(ref mut map) = val {
        // trace-only fields
        if level < DetailLevel::Trace {
            map.remove("extensions");
        }
        // debug-only fields
        if level < DetailLevel::Debug {
            map.remove("resources");
            map.remove("peers");
        }
    }
    val
}
```

**Step 4: Update `lib.rs` — add new modules and re-exports**

```rust
pub mod build_info;
pub mod detail_level;
// ... existing modules ...

pub use build_info::BuildInfo;
pub use detail_level::DetailLevel;
```

**Step 5: Run tests**

```bash
cd elohim/elohim-compute && cargo test
```

Expected: all existing tests fail on `version` → `build` rename. Fix them.

**Step 6: Fix existing tests in `report.rs`**

Update all test code that sets `report.version = "0.1.0"` to use the new `build` field.
Update `ComputeReport::build()` call sites to pass `&BuildInfo::new("test-service")`.

**Step 7: Run tests again**

```bash
cd elohim/elohim-compute && cargo test
```

Expected: ALL PASS

**Step 8: Commit**

```bash
git add elohim/elohim-compute/
git commit -m "feat(compute): add BuildInfo, DetailLevel, and report filtering"
```

---

### Task 2: Update doorway to use `BuildInfo`

**Files:**
- Modify: `doorway/doorway-service/src/routes/health.rs`
- Modify: `doorway/doorway-service/src/routes/status.rs`

**Step 1: Update `health.rs` — use `BuildInfo` in `VersionResponse`**

Replace the `VersionResponse` struct and `version_info()` function with a delegation to `BuildInfo`:

```rust
use elohim_compute::BuildInfo;

// Delete the VersionResponse struct entirely.
// Replace version_info() with:
pub fn version_info() -> Response<Full<Bytes>> {
    let info = BuildInfo::new("elohim-doorway");
    let body = serde_json::to_string(&info)
        .unwrap_or_else(|_| r#"{"version":"unknown","commit":"unknown"}"#.to_string());

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap()
}
```

Keep the `HealthResponse.version` field as-is (it's a simple string for k8s/seeder compat).

**Step 2: Update `status.rs` — pass `BuildInfo` to `ComputeReport::build()`**

At `status.rs:615`, change:

```rust
// OLD:
let compute = elohim_compute::ComputeReport::build(&reporter, resources, compute_peers, extensions);
compute.version = env!("CARGO_PKG_VERSION").to_string();

// NEW:
let build_info = elohim_compute::BuildInfo::new("elohim-doorway");
let compute = elohim_compute::ComputeReport::build(&reporter, &build_info, resources, compute_peers, extensions);
```

**Step 3: Run doorway tests**

```bash
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins
```

Expected: ALL PASS (353+)

**Step 4: Commit**

```bash
git add doorway/doorway-service/src/routes/health.rs doorway/doorway-service/src/routes/status.rs
git commit -m "refactor(doorway): use BuildInfo from elohim-compute for version endpoints"
```

---

### Task 3: Add `elohim-compute` dependency and `/version` endpoint to storage

**Files:**
- Modify: `elohim/elohim-storage/Cargo.toml`
- Modify: `elohim/elohim-storage/src/http.rs`

**Step 1: Add `elohim-compute` dependency to storage Cargo.toml**

```toml
elohim-compute = { path = "../elohim-compute" }
```

**Step 2: Add `/version` route to storage `http.rs`**

In the match block (near `/health`), add:

```rust
(Method::GET, "/version") => {
    let info = elohim_compute::BuildInfo::new("elohim-storage");
    let body = serde_json::to_string(&info).unwrap_or_else(|_| {
        r#"{"version":"unknown","commit":"unknown"}"#.to_string()
    });
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body)))
        .unwrap())
}
```

**Step 3: Enrich `/health` with `BuildInfo`**

In `handle_health()`, add build info to the response:

```rust
async fn handle_health(&self) -> Result<Response<Full<Bytes>>, StorageError> {
    let stats = self.blob_store.stats().await?;
    let build = elohim_compute::BuildInfo::new("elohim-storage");
    let body = serde_json::json!({
        "status": "ok",
        "build": build,
        "blobs": stats.total_blobs,
        "bytes": stats.total_bytes,
        "manifests": self.manifests.read().await.len(),
        "import_enabled": self.import_api.is_some(),
    });

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Full::new(Bytes::from(body.to_string())))
        .unwrap())
}
```

**Step 4: Add `/version` to `build_manifest()` exclusion comment**

The `/version` endpoint is infrastructure — confirm it's NOT added to `build_manifest()`.
Check that the existing comment at line ~5313 mentions `/version` as excluded.

**Step 5: Build and test storage**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test
```

Expected: compiles, all tests pass

**Step 6: Commit**

```bash
git add elohim/elohim-storage/Cargo.toml elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): add /version endpoint and BuildInfo to /health"
```

---

### Task 4: Enrich P2P identify with commit hash

**Files:**
- Modify: `elohim/elohim-storage/src/p2p/behaviour.rs:264`

**Step 1: Update identify agent_version**

At `behaviour.rs:264`, change:

```rust
// OLD:
.with_agent_version(format!("elohim-storage/{}", env!("CARGO_PKG_VERSION"))),

// NEW:
.with_agent_version(elohim_compute::BuildInfo::new("elohim-storage").user_agent()),
```

**Step 2: Build**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

Expected: compiles

**Step 3: Commit**

```bash
git add elohim/elohim-storage/src/p2p/behaviour.rs
git commit -m "feat(p2p): advertise commit hash in libp2p identify protocol"
```

---

### Task 5: Add build args to storage Dockerfile and Jenkinsfile

**Files:**
- Modify: `elohim/elohim-storage/Dockerfile`
- Modify: `elohim/holochain/Jenkinsfile:915-919`

**Step 1: Add build args to storage Dockerfile**

After the `ARG CACHE_BUST=unknown` line (~69), add:

```dockerfile
ARG GIT_COMMIT_SHORT=unknown
ARG GIT_COMMIT_FULL=unknown
ARG BUILD_TIMESTAMP=unknown
ARG RUSTC_VERSION=unknown
```

Change the build command (~79) to pass them as env vars:

```dockerfile
RUN echo "Building at ${CACHE_BUST}" && \
    GIT_COMMIT_SHORT=${GIT_COMMIT_SHORT} \
    GIT_COMMIT_FULL=${GIT_COMMIT_FULL} \
    BUILD_TIMESTAMP=${BUILD_TIMESTAMP} \
    RUSTC_VERSION=${RUSTC_VERSION} \
    cargo build --release
```

**Step 2: Update Jenkinsfile storage build command**

At `elohim/holochain/Jenkinsfile:915-919`, add the build args:

```groovy
BUILDKIT_HOST=unix:///run/buildkit/buildkitd.sock \\
    nerdctl -n k8s.io build --no-cache \\
    --build-arg CACHE_BUST=${GIT_COMMIT_HASH} \\
    --build-arg GIT_COMMIT_SHORT=\$(echo ${GIT_COMMIT_HASH} | cut -c1-7) \\
    --build-arg GIT_COMMIT_FULL=${GIT_COMMIT_HASH} \\
    --build-arg BUILD_TIMESTAMP=\$(date -u +'%Y-%m-%dT%H:%M:%SZ') \\
    --build-arg RUSTC_VERSION=\$(rustc --version 2>/dev/null | head -1 || echo unknown) \\
    -t elohim-storage:${IMAGE_TAG} \\
    -f elohim/elohim-storage/Dockerfile .
```

**Step 3: Commit**

```bash
git add elohim/elohim-storage/Dockerfile elohim/holochain/Jenkinsfile
git commit -m "ci: pass build info args to storage Docker build"
```

---

### Task 6: Add `?detail=` query parameter support to storage `/health`

**Files:**
- Modify: `elohim/elohim-storage/src/http.rs`

**Step 1: Parse `?detail=` from request URI**

In `handle_health()`, parse the query parameter:

```rust
// At the top of handle_health, parse detail level from query string
let detail = req.uri().query()
    .and_then(|q| q.split('&').find_map(|p| p.strip_prefix("detail=")))
    .and_then(|v| v.parse::<elohim_compute::DetailLevel>().ok())
    .unwrap_or_default();
```

Note: `handle_health` currently takes `&self` — it needs `&self, req: &Request<Incoming>` or access to the URI. Check the call site at line ~363 and thread the request URI through, or parse the query from the path string that's already available in the match block.

Since the match block already has `path` as a string, the simplest approach is to pass the full URI string and parse `detail` from it, or to change the route match to pass the query string.

**Step 2: Build a tiered response**

```rust
// Build full report
let stats = self.blob_store.stats().await?;
let build = elohim_compute::BuildInfo::new("elohim-storage");

let mut body = serde_json::json!({
    "status": "ok",
    "build": build,
    "health": "healthy",
});

// info level: basic operational data
if detail >= elohim_compute::DetailLevel::Info {
    body["blobs"] = serde_json::json!(stats.total_blobs);
    body["bytes"] = serde_json::json!(stats.total_bytes);
    body["importEnabled"] = serde_json::json!(self.import_api.is_some());
}

// debug level: resource details
if detail >= elohim_compute::DetailLevel::Debug {
    body["manifests"] = serde_json::json!(self.manifests.read().await.len());
    body["concurrencyLimit"] = serde_json::json!(MAX_CONCURRENT_REQUESTS);
    body["appIndex"] = serde_json::json!(self.app_index.read().await.len());
}

// trace level: full internal state
if detail >= elohim_compute::DetailLevel::Trace {
    body["semaphorePermits"] = serde_json::json!(self.request_semaphore.available_permits());
    body["dbPoolEnabled"] = serde_json::json!(self.db_pool.is_some());
    body["extractionCacheEnabled"] = serde_json::json!(self.extraction_cache.is_some());
}
```

**Step 3: Build and test**

```bash
cd elohim/elohim-storage
RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check
```

**Step 4: Commit**

```bash
git add elohim/elohim-storage/src/http.rs
git commit -m "feat(storage): support ?detail= query param on /health endpoint"
```

---

### Task 7: Final verification and push

**Step 1: Run all quality gates locally**

```bash
# doorway
cd doorway/doorway-service && RUSTFLAGS="" cargo test --lib --bins && RUSTFLAGS="" cargo clippy -- -D warnings && cargo fmt --check

# storage
cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo check && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo clippy -- -D warnings && cargo fmt --check

# compute
cd elohim/elohim-compute && cargo test && cargo clippy -- -D warnings && cargo fmt --check
```

**Step 2: Push**

```bash
git push
```

Wait for pre-push hooks to pass (doorway, elohim-storage, genesis, schema-validate).
