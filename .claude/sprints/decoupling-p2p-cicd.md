# Decoupling Sprint Plan: P2P-Ready CI/CD Architecture

## Vision
Transform the monorepo from tightly-coupled sequential builds to independent, version-aware components that can model real P2P network topology using Kubernetes.

## Current State (as of 2026-02-08)

**Resolved:**
1. ~~**Shared Infrastructure**: Alpha + Staging share `doorway-dev.elohim.host`~~ -- **FIXED** (Sprint 3)
2. ~~**Ephemeral Storage**: `emptyDir` volumes lose data on restart~~ -- **FIXED** (PVCs deployed)
3. ~~**Pipeline Coupling**: DNA → Edge → App → Genesis runs sequentially~~ -- **FIXED** (Sprint 2: parallel levels)
4. ~~**No Schema Versioning**: Seeder JSON has no `schema_version`~~ -- **FIXED** (Sprint 0: tolerant readers + schemaVersion)
5. ~~**Artifact Fragility**: `wget lastSuccessfulBuild` fetches from Jenkins~~ -- **FIXED** (Sprint 1: Harbor + floating tags)

**Remaining:**
6. **Dormant P2P**: libp2p is 95% built but not activated

## Target State
- Independent component builds with explicit versioning
- Separate K8s deployments per environment (alpha, staging, prod)
- Tolerant reader pattern for version skew between nodes
- P2P content sync via libp2p (not shared infrastructure)
- StatefulSets modeling real network topology

---

## Configuration Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Artifact Storage | **Harbor** | Already integrated with K8s, version tagging built-in |
| Namespaces | **Dedicated** | elohim-alpha, elohim-staging, elohim-prod for full isolation |
| Priority | **K8s Isolation First** | Fix immediate pain (alpha/staging coupling) |
| RNA Framework | **Deferred** | Focus on data plane versioning; DNA migration later |

---

## Sprint Index (Reordered by Priority)

| Sprint | Focus | Status | Priority |
|--------|-------|--------|----------|
| 3 | K8s Environment Isolation | **COMPLETE** | ~~IMMEDIATE~~ |
| 0 | Schema Versioning Foundation | **COMPLETE** | ~~NEXT~~ |
| 1 | Artifact Storage Decoupling | **COMPLETE** (was already done) | ~~HIGH~~ |
| 2 | Pipeline Parallelization | **COMPLETE** (genesis cron deferred as low-value) | ~~NEXT~~ |
| 4 | Seeder Version Negotiation | **COMPLETE** | ~~MEDIUM~~ |
| 5 | DNA Build Isolation | **COMPLETE** | ~~MEDIUM~~ |
| 6 | P2P Activation | **COMPLETE** | ~~MEDIUM~~ |
| 7 | Network Topology Modeling | **COMPLETE** | ~~FUTURE~~ |

**Execution Order**: ~~0 → 3 → 1 → 2 → 4 & 5 (parallel) → 6 → 7~~ (All sprints complete)

---

## Sprint 0: Schema Versioning Foundation -- COMPLETE

**Goal**: Establish version-aware data contracts before any other decoupling.

**Status**: Complete. All InputView structs versioned, TypeScript types regenerated, compile-time lint enforces discipline.

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| `#[serde(default)]` on InputView fields | DONE | 100% of optional fields covered (audit confirmed) |
| `schema_version: u32` on all InputViews | DONE | 11 structs in `views.rs`, default=1 via `default_schema_version()` |
| Tolerant reader guarantee | DONE | Serde ignores unknown fields by default (no `deny_unknown_fields`); tests prove it |
| `schemaVersion` on TS seed entities | DONE | 3 interfaces + 4 builder sites in `seed-entities.ts` |
| Generate TypeScript types from ts-rs | DONE | `generate-types.sh` run; all 11 InputView types export `schemaVersion: number` |
| Compile-time lint for schema_version | DONE | `all_input_views_have_schema_version_field` test covers all 11 structs; fails to compile if new struct omits field |
| `#[serde(flatten)] HashMap` | SKIPPED | Unnecessary: serde already ignores unknown fields. Would disable fast-path deserialization and interact poorly with `rename_all = "camelCase"`. |

**Files Modified**:
- `holochain/elohim-storage/src/views.rs` - Added `default_schema_version()` helper, `schema_version` field to all 11 InputView structs, 4 schema version tests
- `genesis/seeder/src/seed-entities.ts` - Added `schemaVersion` to 3 interfaces + 4 builder call sites
- `genesis/seeder/src/validation-constants.ts` - Export from ts-rs bindings

**Verification**:
```bash
# Test tolerant reader
curl -X POST http://localhost:8090/db/content/bulk \
  -d '[{"id":"test","title":"Test","schemaVersion":99,"unknownField":"ignored"}]'
# Should succeed, not reject unknown fields
```

---

## Sprint 1: Artifact Storage Decoupling (Harbor) -- COMPLETE (already implemented)

**Goal**: Remove Jenkins `lastSuccessfulBuild` artifact fetching; use versioned Harbor artifacts.

**Status**: Already fully implemented prior to sprint plan creation. Discovered during audit 2026-02-08.

**Evidence**:
- DNA pipeline (`holochain/dna/Jenkinsfile`): "Push to Harbor" stage using `oras` CLI with commit + floating tags
- Edge pipeline (`holochain/Jenkinsfile`): `fetchHappFromHarbor()` helper, floating tag strategy
- App pipeline (`Jenkinsfile`): Fetches WASM from Harbor via `oras pull`, pushes `elohim-site` images
- Steward pipeline (`steward/Jenkinsfile`): Fetches hApp from Harbor with fallback to local build
- VERSION file: Contains `HAPP_VERSION=1.0.0` and all other component versions
- No `lastSuccessfulBuild` pattern remains anywhere

---

## Sprint 2: Pipeline Parallelization -- COMPLETE

**Goal**: Enable parallel Edge + App builds; decouple Genesis from deployment.

**Status**: Parallel execution implemented. Genesis cron is optional (deferred as low-value).

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| Parallel Edge + App builds | DONE | `groupByDependencyLevel()` in orchestrator groups pipelines by dependency level; `parallel` block executes same-level builds concurrently |
| Dependency-aware ordering | DONE | DNA builds first (level 0), then Edge + App in parallel (level 1), then Genesis last |
| Fail-fast across levels | DONE | If any pipeline in a level fails, subsequent levels are aborted |
| Genesis runs after all builds | DONE | Excluded from levels, triggered sequentially after all builds succeed |
| Manual re-seed capability | DONE | Genesis trigger check allows `UserIdCause` (manual Jenkins build) |
| Dependency propagation | DONE | `propagateDependencies()` auto-includes dependents when dependencies build |

### Decided Against

| Item | Decision | Rationale |
|------|----------|-----------|
| Webhook triggers on downstream pipelines | REJECTED | Contradicts single-webhook-receiver architecture. Orchestrator is the only pipeline receiving GitHub webhooks; downstream uses `overrideIndexTriggers(false)`. Adding webhooks would cause duplicate builds. |
| Remove `dependsOn` from PIPELINES map | REJECTED | Edge correctly depends on DNA (`dependsOn: ['elohim-holochain']`). This ensures DNA builds before Edge when both are triggered. Parallel execution handles this via dependency levels. |
| Genesis cron trigger | DEFERRED | Low value: orchestrator already auto-triggers genesis after builds. Cron would cause unnecessary re-seeds. Can add later if independent seed scheduling is needed. |

### Execution Plan (typical build)

```
Level 0:  [elohim-holochain]          (DNA, if triggered)
Level 1:  [elohim-edge + elohim]      (parallel)
Finally:  elohim-genesis              (seed + test)
```

**Files Modified**:
- `orchestrator/Jenkinsfile` - Added `groupByDependencyLevel()`, refactored Execute Builds stage to use `parallel` block within each level, genesis runs last

**Verification**:
```bash
# Orchestrator build log should show parallel execution:
#   Plan: elohim-holochain -> [elohim-edge + elohim] -> genesis
# Edge + App run concurrently in Level 1
```

---

## Sprint 3: K8s Environment Isolation -- COMPLETE

**Goal**: Separate alpha/staging/prod into independent deployments with dedicated namespaces.

**Status**: Complete. Code changes deployed, namespaces/policies applied, staging merged.

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| K8s namespaces created | DONE | `elohim-alpha`, `elohim-staging`, `elohim-prod` all Active |
| `edgenode-alpha.yaml` | DONE | Full manifest: namespace, ConfigMap, secrets, PVCs, Deployment, Ingress |
| `edgenode-staging.yaml` | DONE | Full manifest: namespace, ConfigMap, secrets, PVCs, Deployment, Ingress |
| PVCs for alpha | DONE | `holochain-data-alpha` (10Gi), `storage-data-alpha` (5Gi) embedded in manifest |
| PVCs for staging | DONE | `holochain-data-staging` (10Gi), `storage-data-staging` (5Gi) embedded in manifest |
| App ConfigMaps | DONE | `orchestrator/manifests/elohim-app/alpha/configmap.yaml` → `doorway-alpha.elohim.host` |
| | | `orchestrator/manifests/elohim-app/staging/configmap.yaml` → `doorway-staging.elohim.host` |
| Edge Ingress (alpha) | DONE | `doorway-alpha.elohim.host`, `signal.doorway-alpha.elohim.host` |
| Edge Ingress (staging) | DONE | `doorway-staging.elohim.host`, `signal.doorway-staging.elohim.host` |
| Jenkinsfile updates | DONE | All 4 Jenkinsfiles updated: namespace refs, stage names, health URLs |
| `doorway-dev` refs eliminated | DONE | Zero `doorway-dev` references remain in any Jenkinsfile |
| Genesis doorway resolution | DONE | `resolveDoorwayHost()` returns cluster-local service per environment |
| Orchestrator endpoints | DONE | VERSION_ENDPOINTS + HEALTH_ENDPOINTS use `doorway-alpha`/`doorway-staging` |
| Staging deployment stages | DONE | Edge + App pipelines have dedicated staging deploy stages |
| dev → staging promotion | DONE | Pushed to dev, ready for merge to staging branch |

### Additional Items Completed (2026-02-08)

| Item | Status | Evidence |
|------|--------|----------|
| Delete `edgenode-dev.yaml` | DONE | `holochain/manifests/edgenode-dev.yaml` removed |
| Zero `doorway-dev` references | DONE | Last reference in IMPORT_PIPELINE_DEBUG.md updated to `doorway-alpha` |
| Version-control `namespaces.yaml` | DONE | `orchestrator/manifests/namespaces.yaml` declares alpha/staging/prod |
| NetworkPolicy cross-env isolation | DONE | `orchestrator/manifests/network-policies.yaml` denies cross-namespace ingress |
| App ingress namespace | N/A | All ingresses already in correct namespaces (`elohim-alpha`/`staging`/`prod`) |

### Operational Items -- COMPLETE

| Item | Status | Notes |
|------|--------|-------|
| Apply namespaces | DONE | `kubectl apply -f orchestrator/manifests/namespaces.yaml` |
| Apply network policies | DONE | `kubectl apply -f orchestrator/manifests/network-policies.yaml` |
| Apply staging ConfigMap | DONE | `kubectl apply -f orchestrator/manifests/elohim-app/staging/configmap.yaml` |
| Merge dev → staging | DONE | Staging builds deploying all components |

---

## Sprint 4: Seeder Version Negotiation -- COMPLETE

**Goal**: Seeder declares schema version; storage handles multiple versions.

**Status**: Complete. Leveraged Sprint 0's existing infrastructure (InputViews, `SUPPORTED_SCHEMA_VERSIONS`, `validate_schema_versions()`). No `/v2` endpoint needed — existing endpoints now fully validate.

**What was done**:
1. Created `CreateMasteryInputView` (fixes camelCase/snake_case bug in mastery bulk endpoint)
2. Added `validate_schema_versions()` to all 4 missing bulk handlers (presences, events, mastery, allocations)
3. Added `X-Schema-Version` header validation on all 7 bulk endpoints (warn if missing, reject if unsupported)
4. Added `X-Supported-Schema-Versions` response header on all 7 bulk endpoints
5. Added `GET /db/schema` capability discovery endpoint
6. Seeder sends `X-Schema-Version: 1` on all 6 bulk endpoints
7. Seeder performs pre-flight schema version check via `GET /db/schema`
8. Seeder detects schema version errors with clear messages

**Files Modified**:
- `holochain/elohim-storage/src/views.rs` - `CreateMasteryInputView` + lint test
- `holochain/elohim-storage/src/http.rs` - Schema validation, header extraction, `/db/schema` route
- `holochain/elohim-storage/src/services/response.rs` - `ok_with_schema_info()` helper
- `genesis/seeder/src/doorway-client.ts` - Headers, `getSchemaInfo()`, error handling
- `genesis/seeder/src/seed.ts` - Pre-flight schema check

---

## Sprint 5: DNA Build Isolation -- COMPLETE

**Goal**: DNA changes don't cascade to Edge rebuilds; version metadata injected into hApp artifacts.

**Status**: Complete. `cascades: false` prevents DNA-only changes from auto-triggering Edge. RNA migration framework was already built (`holochain/rna/` ~2000 lines).

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| `cascades: false` on DNA config | DONE | `orchestrator/Jenkinsfile` PIPELINES map |
| `propagateDependencies()` respects flag | DONE | Checks `depConfig.cascades` before auto-including dependents; default `true` for backward compat |
| Dependency graph comment updated | DONE | Header documents `[cascades: false]` behavior |
| Version metadata in hApp manifest | DONE | `holochain/dna/Jenkinsfile` injects `# Build: HAPP_VERSION=x commit=y` comment before `hc app pack` |
| Version compatibility log in Edge | DONE | `holochain/Jenkinsfile` logs expected HAPP_VERSION and fetched tag after `fetchHappFromHarbor` |
| RNA migration framework | ALREADY DONE | `holochain/rna/` has ~2000 lines: self-healing entries, provider architecture, migration functions, templates |

### Behavior After Changes

| Scenario | Before | After |
|----------|--------|-------|
| DNA-only change | DNA + Edge + Genesis (~25 min) | DNA + Genesis (~12 min) |
| Edge-only change | Edge + Genesis | Edge + Genesis (unchanged) |
| DNA + Edge in same commit | DNA → Edge → Genesis | DNA → Edge → Genesis (ordering preserved) |
| `rebuild-all` | Everything | Everything (bypasses propagation) |

### Why `rebuild-all` still works
`rebuild-all` mode hard-codes the pipeline list, bypassing `propagateDependencies()` entirely. The `cascades` flag only affects `auto` mode changeset-driven builds.

**Files Modified**:
- `orchestrator/Jenkinsfile` - `cascades: false` flag, `propagateDependencies()` logic, header comment
- `holochain/dna/Jenkinsfile` - Version metadata injection before `hc app pack`
- `holochain/Jenkinsfile` - DNA version awareness log in Edge Node build stage

---

## Sprint 6: P2P Activation -- COMPLETE

**Goal**: Wire up dormant libp2p infrastructure so elohim-storage actually uses it.

**Status**: Complete. Bootstrap dialing, SyncManager wiring, and `/p2p/status` endpoint all active.

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| Bootstrap node dialing | DONE | `start()` parses multiaddr strings, dials bootstrap nodes, adds to Kademlia routing table |
| SyncManager wired to HttpServer | DONE | `main.rs` calls `http_server.with_sync_manager(node.sync_manager().clone())` when P2P enabled |
| `/p2p/status` endpoint | DONE | Returns JSON: peer_id, listen_addresses, connected_peers, bootstrap_nodes, sync_documents |
| `P2PHandle` (Send+Sync safe) | DONE | `watch` channel decouples non-Send swarm from HttpServer; status refreshed on connection events + 30s interval |
| Sync API routes reachable | DONE | `/sync/v1/{app_id}/docs` etc. now work when `--enable-p2p` is set (SyncManager was never wired before) |
| Build with + without p2p feature | DONE | `cargo build` and `cargo build --no-default-features` both succeed |
| All tests pass | DONE | 128 tests (123 lib + 5 integration) |

### Architecture Decision: P2PHandle

libp2p `Swarm` types are not `Send`, so `P2PNode` cannot be stored directly in `HttpServer` (which needs `Send` for `tokio::spawn`). Solution: `P2PHandle` wraps a `tokio::sync::watch::Receiver<P2PStatusInfo>` — a Send+Sync snapshot updated by the P2P event loop on connection changes and a 30-second timer.

### Files Modified
- `holochain/elohim-storage/src/p2p/mod.rs` — `P2PStatusInfo` struct, `P2PHandle`, bootstrap dialing in `start()`, `refresh_status()`, status updates in event loop
- `holochain/elohim-storage/src/http.rs` — `p2p_handle` field, `with_p2p_handle()` builder, `/p2p/status` route + handler
- `holochain/elohim-storage/src/main.rs` — Wire `sync_manager` + `p2p_handle` into HttpServer when P2P enabled
- `holochain/elohim-storage/src/lib.rs` — Export `P2PHandle`, `P2PStatusInfo`

### Verification
```bash
# Build succeeds with p2p feature (default)
cargo build -p elohim-storage

# Build succeeds without p2p feature
cargo build -p elohim-storage --no-default-features

# All tests pass
cargo test -p elohim-storage

# Manual test (after local build)
./target/debug/elohim-storage --enable-p2p --p2p-port 9000
curl http://localhost:8090/p2p/status
# {"peer_id":"12D3Koo...","listen_addresses":["/ip4/0.0.0.0/tcp/9000"],"connected_peers":0,"bootstrap_nodes":[],"sync_documents":0}

curl http://localhost:8090/sync/v1/test-app/docs
# 200 with document list (empty initially)
```

---

## Sprint 7: Network Topology Modeling -- COMPLETE

**Goal**: Use K8s StatefulSets to model real P2P network; enable topology testing.

**Status**: Complete. Deployments converted to StatefulSets, headless services added, P2P env vars enabled, cross-namespace NetworkPolicies in place, Jenkinsfile deploy helper auto-detects resource type.

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| Convert alpha Deployment → StatefulSet | DONE | `serviceName`, `podManagementPolicy: Parallel`, `updateStrategy: RollingUpdate` |
| Convert staging Deployment → StatefulSet | DONE | Same changes as alpha |
| Headless service (alpha) | DONE | `elohim-edgenode-alpha-headless` with `clusterIP: None`, P2P port 9876 |
| Headless service (staging) | DONE | `elohim-edgenode-staging-headless` with `clusterIP: None`, P2P port 9876 |
| P2P env vars on elohim-storage | DONE | `ENABLE_P2P=true`, `P2P_PORT=9876`, `DISABLE_MDNS=true` on both envs |
| P2P container ports | DONE | `storage-http:8090` + `p2p:9876` on elohim-storage container |
| P2P port on ClusterIP services | DONE | Port 9876 added to both alpha + staging ClusterIP services |
| Cross-namespace NetworkPolicy | DONE | `allow-p2p-from-staging` (alpha NS) + `allow-p2p-from-alpha` (staging NS) |
| Jenkinsfile StatefulSet detection | DONE | `deployEdgeWithManifest()` auto-detects `kind: StatefulSet` from manifest template |
| Prod untouched | DONE | `prod.yaml` remains a Deployment until alpha/staging validated |

### Files Modified
- `orchestrator/manifests/edgenode/alpha.yaml` — Deployment→StatefulSet, headless service, P2P env/ports
- `orchestrator/manifests/edgenode/staging.yaml` — Same changes as alpha
- `orchestrator/manifests/network-policies.yaml` — P2P cross-namespace rules (port 9876)
- `holochain/Jenkinsfile` — `deployEdgeWithManifest()` auto-detects resource type

### Verification (operational, post-deploy)
```bash
# 1. Verify StatefulSet pods
kubectl get statefulset -n elohim-alpha
kubectl get pods -n elohim-alpha
# Expected: elohim-edgenode-alpha-0  Running

# 2. Verify headless service DNS
kubectl exec -n elohim-alpha elohim-edgenode-alpha-0 -c elohim-storage -- \
  nslookup elohim-edgenode-alpha-headless.elohim-alpha.svc.cluster.local

# 3. Check P2P status
kubectl exec -n elohim-alpha elohim-edgenode-alpha-0 -c elohim-storage -- \
  curl -s localhost:8090/p2p/status | jq

# 4. Verify cross-namespace connectivity
kubectl exec -n elohim-alpha elohim-edgenode-alpha-0 -c elohim-storage -- \
  nc -zv elohim-edgenode-staging-headless.elohim-staging.svc.cluster.local 9876

# 5. Verify existing functionality
curl https://doorway-alpha.elohim.host/health
curl https://doorway-staging.elohim.host/health
```

---

## Dependencies Between Sprints (Updated)

```
Sprint 3 (K8s Isolation) ─────── COMPLETE ✓
Sprint 0 (Schema Versioning) ── COMPLETE ✓
Sprint 1 (Artifact Storage) ─── COMPLETE ✓ (was already done)
Sprint 2 (Pipeline Parallel) ── COMPLETE ✓ (parallel execution, genesis cron deferred)
    │
    ├──► Sprint 4 (Seeder Versioning) ── COMPLETE ✓
    │
    └──► Sprint 5 (DNA Isolation) ────── COMPLETE ✓ (RNA already built)
              │
              ▼
         Sprint 6 (P2P Activation) ──── COMPLETE ✓
              │
              ▼
         Sprint 7 (Network Topology) ── COMPLETE ✓
```

**Recommended Execution** (updated 2026-02-09):
1. **DONE**: All sprints (0-7) complete (code + operational where deployed)

---

## Session Scope Guidelines

Each sprint is designed to fit in one agent session (~50-100 tool calls):

| Sprint | Estimated Files | Estimated Lines | Complexity |
|--------|-----------------|-----------------|------------|
| 0 | 4-5 files | ~200 lines | Medium (serde patterns) |
| 1 | 5-6 files | ~150 lines | Low (pipeline config) |
| 2 | 4 files | ~100 lines | Low (pipeline config) |
| 3 | 8-10 files | ~400 lines | High (K8s manifests) |
| 4 | 4-5 files | ~250 lines | Medium (API versioning) |
| 5 | 5-6 files | ~200 lines | Medium (migration hooks) |
| 6 | 5-6 files | ~300 lines | High (P2P activation) |
| 7 | 6-8 files | ~350 lines | High (StatefulSets) |

---

## Anti-Regression Safeguards

Each sprint includes safeguards that make regression difficult:

1. **Sprint 0**: CI lint check rejects required fields without defaults
2. **Sprint 1**: `fetchHappArtifact()` function deleted entirely
3. **Sprint 2**: `groupByDependencyLevel()` replaces sequential execution; `parallel` block in Execute Builds stage
4. **Sprint 3**: Shared `edgenode-dev.yaml` deleted; all `doorway-dev` refs eliminated; dedicated PVCs per environment; orchestrator endpoints point to per-environment URLs
5. **Sprint 4**: Old bulk endpoint deprecated with warning logs
6. **Sprint 5**: `cascades: false` flag on DNA config; `propagateDependencies()` checks it; version metadata baked into hApp YAML comments
7. **Sprint 6**: `--enable-p2p` required for inter-node sync
8. **Sprint 7**: Deployments converted to StatefulSets (can't revert easily)

---

## Quick Reference: Current Sprint Checklists

### Sprint 3 Checklist (K8s Isolation) -- COMPLETE
- [x] Create namespaces: elohim-alpha, elohim-staging, elohim-prod
- [x] Create edgenode-alpha.yaml (from edgenode-dev)
- [x] Create edgenode-staging.yaml (new)
- [x] Create PVCs for alpha and staging (embedded in manifests)
- [x] Update ConfigMaps with new doorway URLs
- [x] Update Ingress for new DNS entries (edge node ingress in manifests)
- [x] Update Jenkinsfiles for namespace changes
- [x] Eliminate all `doorway-dev` references from Jenkinsfiles
- [x] Update orchestrator VERSION_ENDPOINTS + HEALTH_ENDPOINTS
- [x] Rename "Deploy Edge Node - Dev" → "Deploy Edge Node - Alpha"
- [x] Fix SonarQube issues (sophia-renderer, related-concepts, budget-reconciliation)
- [x] Push all fixes to dev
- [x] Delete edgenode-dev.yaml (removed `holochain/manifests/edgenode-dev.yaml`)
- [x] Eliminate ALL `doorway-dev` references from repo (last one was in IMPORT_PIPELINE_DEBUG.md)
- [x] Version-control namespaces (`orchestrator/manifests/namespaces.yaml`)
- [x] NetworkPolicy for cross-env isolation (`orchestrator/manifests/network-policies.yaml`)
- [x] App ingress namespaces verified correct (all use `elohim-alpha`/`staging`/`prod`)
- [x] **OPERATIONAL**: `kubectl apply -f orchestrator/manifests/namespaces.yaml`
- [x] **OPERATIONAL**: `kubectl apply -f orchestrator/manifests/network-policies.yaml`
- [x] **OPERATIONAL**: `kubectl apply -f orchestrator/manifests/elohim-app/staging/configmap.yaml`
- [x] **OPERATIONAL**: Merge dev → staging and verify staging health

### Sprint 0 Checklist (Schema Versioning) -- COMPLETE
- [x] Add `#[serde(default)]` to all optional InputView fields (100% coverage confirmed)
- [x] Add `schema_version: u32` to all 11 InputView structs (default=1)
- [x] Add `schemaVersion` to seed entity TS interfaces + builders
- [x] Tolerant reader tests (5 tests: default, explicit, unknown fields, multi-struct, all-struct lint)
- [x] Generate TypeScript types from ts-rs (`generate-types.sh` -- all 11 types export `schemaVersion`)
- [x] Compile-time lint (`all_input_views_have_schema_version_field` -- fails to compile if new struct misses field)
- [~] ~~`#[serde(flatten)] HashMap`~~ SKIPPED (serde already ignores unknown fields; flatten hurts perf)
