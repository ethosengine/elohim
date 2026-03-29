# Compute Reporting Enrichment — Design

## Problem

Storage has no version/commit info on any endpoint. `ComputeReport.version` is always empty.
P2P identify only carries package version, not commit hash. No standard labeling of what
internal information should be visible at what level. Debugging a deployed peer requires
SSH or log scraping.

## Vision

Every piece of internal information is labeled with a standard log level (`error` through
`trace`). Today the filter is a query parameter. Tomorrow it's a reach attestation — like
a doctor needs permission to see your records. The labels don't change; only the access
model evolves.

## Design

### 1. `BuildInfo` in `elohim-compute`

New struct owned by the shared crate, populated at compile time via `env!()` / `option_env!()`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    pub version: String,        // CARGO_PKG_VERSION
    pub commit: String,         // GIT_COMMIT_SHORT (7 chars)
    pub commit_full: String,    // GIT_COMMIT_FULL (40 chars)
    pub build_time: String,     // BUILD_TIMESTAMP (ISO 8601)
    pub rustc_version: String,  // RUSTC_VERSION
    pub service: String,        // "elohim-doorway", "elohim-storage"
}
```

`ComputeReport.version: String` becomes `ComputeReport.build: BuildInfo`.

### 2. `DetailLevel` enum — standard log levels

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DetailLevel {
    Error,  // Always visible — active problems
    Warn,   // Conditions peers should know about
    Info,   // Standard operational identity (default)
    Debug,  // Internals for diagnosing issues
    Trace,  // Full internal state
}
```

The hierarchy is ordered: `Error < Warn < Info < Debug < Trace`. A request at level `debug`
sees everything at `debug` and below.

### 3. Field-level labeling

Convention, not runtime machinery. Each field in `ComputeReport` and its nested types
carries a doc comment declaring its level:

```rust
pub struct ComputeReport {
    /// [info] Service identity and build info
    pub build: BuildInfo,
    /// [error] Current health state
    pub health: ServiceHealth,
    /// [error] Why the service is in this state
    pub health_reason: String,
    /// [info] When the service started
    pub started_at: DateTime<Utc>,
    /// [info] Seconds since start
    pub uptime_seconds: u64,
    /// [debug] Resource usage snapshot
    pub resources: ResourceSnapshot,
    /// [debug] Per-peer health snapshots
    pub peers: Vec<PeerHealthSnapshot>,
    /// [trace] Service-specific internal state
    pub extensions: Value,
}
```

A `filter(report, level) -> Value` function in `elohim-compute` strips fields above
the requested level by serializing to `serde_json::Value` and removing labeled keys.

### 4. Query parameter (today's access model)

```
GET /health                    → error + warn + info (default)
GET /health?detail=debug       → + debug fields
GET /health?detail=trace       → everything
```

No authentication required today (all endpoints are behind k8s network policy).
When attestation-gated access arrives, the query param is replaced by the
attestation's granted level — same filtering, different input.

### 5. Storage parity

| Gap | Fix |
|-----|-----|
| No version/commit | Add `GIT_COMMIT_*` + `BUILD_TIMESTAMP` build args to storage Dockerfile |
| No `/version` endpoint | Add `/version` route returning `BuildInfo` |
| `/health` is ad-hoc JSON | Replace with `ComputeReport` (info-level by default) |
| `ComputeReport.version` empty | Replace with `BuildInfo`, populate from env |
| P2P identify lacks commit | Change agent_version to `elohim-storage/{version}+{commit_short}` |

### 6. P2P peer advertisement

The libp2p identify protocol already exchanges agent versions. Enrich to:

```
elohim-storage/0.1.0+215bc559
```

This is visible to all connected peers automatically. For richer health exchange,
the existing `PeerHealthSnapshot` in `elohim-compute` carries what peers need.
Full compute reports are fetched on-demand via HTTP between trusted peers,
not gossiped — they're too large and change too frequently for DHT.

### 7. Future: attestation-gated access

The labeling established here becomes the vocabulary for reach attestations:

```
Attestation {
    grantor: peer_id_A,       // node operator
    grantee: peer_id_B,       // requesting peer
    scope: "compute:debug",   // level granted
    expires: DateTime,        // time-bounded
}
```

The network needs self-healing (peers diagnosing each other), but must respect
privacy (my internal state is mine to share). The attestation model resolves this:
you opt in to sharing diagnostic detail with specific peers, revocably.

The elohim agent layer can request diagnostic attestations when investigating
issues — but the node operator (or their policy) decides whether to grant them.

## Field labeling reference

| Level | Field | Location |
|-------|-------|----------|
| error | `health`, `healthReason`, `error` | ComputeReport |
| warn | `reconnectAttempts`, `natStatus` | PeerHealthSnapshot, P2PStatusInfo |
| info | `build`, `startedAt`, `uptimeSeconds`, `serviceId` | ComputeReport |
| info | `peerCount`, `listenAddresses`, `relayMode` | P2PStatusInfo |
| debug | `resources`, `peers` | ComputeReport |
| debug | `requests`, `activeConnections`, `managedStorageBytes` | ResourceSnapshot |
| debug | `signalsReceived`, `lastSignalAt` | PeerHealthSnapshot |
| debug | `cacheHitRate`, `extractionCacheSize` | extensions |
| trace | `extensions` (full) | ComputeReport |
| trace | `semaphorePermits`, `perRouteLatency`, `dbPoolStats` | extensions |

## P2P design gate

All endpoints in this design are **Category C (operational/ephemeral)** — local runtime
state served from memory, not persisted to any DHT. No new entry types needed.

The future attestation model (section 7) introduces a notarized entity (`Attestation`
with scope `compute:debug`). That work MUST go through the p2p-design-gate skill when
it moves from vision to implementation.

## Non-goals

- Runtime log level changes via endpoint (RUST_LOG handles this)
- Gossiping compute reports over P2P (too large, too frequent)
- Attestation implementation (future sprint — this design lays the vocabulary)

## Files affected

| File | Change |
|------|--------|
| `elohim/elohim-compute/src/lib.rs` | Re-export `BuildInfo`, `DetailLevel` |
| `elohim/elohim-compute/src/report.rs` | Add `BuildInfo`, `DetailLevel`, `filter()`, replace `version` with `build` |
| `elohim/elohim-compute/src/health.rs` | Add `build_info()` to `HealthReporter` trait |
| `elohim/elohim-storage/Dockerfile` | Add `GIT_COMMIT_*`, `BUILD_TIMESTAMP` build args |
| `elohim/elohim-storage/src/http.rs` | Add `/version`, enrich `/health` with `ComputeReport` |
| `elohim/elohim-storage/src/p2p/behaviour.rs` | Enrich identify agent_version with commit |
| `doorway/doorway-service/src/routes/health.rs` | Use `BuildInfo` instead of raw strings |
| `doorway/doorway-service/src/routes/status.rs` | Use enriched `ComputeReport` |
| `Jenkinsfile` / `elohim/holochain/Jenkinsfile` | Pass build args to storage Docker build |
