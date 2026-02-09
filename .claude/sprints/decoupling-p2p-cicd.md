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
| 4 | Seeder Version Negotiation | Not started | MEDIUM |
| 6 | P2P Activation | Not started | MEDIUM |
| 7 | Network Topology Modeling | Not started | FUTURE |
| 5 | DNA Build Isolation | Not started | **DEFERRED** |

**Execution Order**: ~~0 → 3 → 1 → 2~~ → 4 → 6 → 7 (Sprints 0, 1, 2, 3 complete; Sprint 5 deferred)

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

## Sprint 4: Seeder Version Negotiation

**Goal**: Seeder declares schema version; storage handles multiple versions.

**Scope**:
1. Add `/db/content/bulk/v2` endpoint that requires `schemaVersion`
2. Add version negotiation header (`X-Schema-Version`)
3. Modify seeder to send version and handle rejection
4. Add schema compatibility matrix to storage

**Files to Modify**:
- `holochain/elohim-storage/src/http.rs` - Add versioned bulk endpoint
- `holochain/elohim-storage/src/views.rs` - Add `SchemaVersionedInput` wrapper
- `genesis/seeder/src/seed.ts` - Add version header, handle 400 responses
- `genesis/seeder/src/api-client.ts` - Add retry with version downgrade

**Forcing Function**:
- Old `/db/content/bulk` endpoint deprecated (logs warning)
- Seeder fails fast if version incompatible (not silent corruption)
- Storage rejects unknown schema versions (explicit allowlist)

**Verification**:
```bash
# Test version negotiation
curl -H "X-Schema-Version: 2" -X POST http://localhost:8090/db/content/bulk/v2 \
  -d '[{"id":"test","title":"Test","schemaVersion":2}]'
# Should succeed

curl -H "X-Schema-Version: 99" -X POST http://localhost:8090/db/content/bulk/v2 \
  -d '[{"id":"test","title":"Test","schemaVersion":99}]'
# Should return 400 with "unsupported schema version"
```

---

## Sprint 5: DNA Build Isolation [DEFERRED]

**Status**: Deferred until data plane versioning (Sprints 0-4, 6-7) is complete.

**Goal**: DNA changes are explicitly versioned; don't cascade to other builds.

**Future Scope** (when resumed):
1. Separate DNA pipeline trigger (not part of orchestrator default)
2. Add DNA version to hApp manifest
3. Create RNA migration framework for DNA-to-DNA transcription
4. Add DNA compatibility check to Edge/App pipelines

**Rationale for Deferral**:
- DNA changes are already infrequent (stable)
- Data plane versioning (SQLite/libp2p) is higher priority
- RNA migration requires deeper design work
- Current DNA build isolation is "good enough" for now

**Prerequisites Before Resuming**:
- Sprint 6 (P2P Activation) complete
- Real-world experience with data plane version skew
- Clear requirements for DNA migration patterns

---

## Sprint 6: P2P Activation

**Goal**: Enable libp2p for content sync; content flows via P2P, not shared deployment.

**Scope**:
1. Enable P2P feature flag in elohim-storage
2. Spawn P2P event loop in main.rs
3. Create ContentLocation DHT entries on import
4. Wire shard requests through libp2p
5. Extend signal server for libp2p peer discovery

**Files to Modify**:
- `holochain/elohim-storage/src/main.rs` - Spawn P2P event loop (lines 242-292)
- `holochain/elohim-storage/src/import_api.rs` - Add ContentLocation DHT entry creation
- `holochain/elohim-storage/src/p2p/mod.rs` - Activate dormant code
- `doorway/src/signal/mod.rs` - Add `/signal/{pubkey}` for libp2p

**Forcing Function**:
- `--enable-p2p` flag required for inter-node content sync
- Seed verification checks P2P availability (not just local SQLite)
- ContentLocation entries required for content to be "published"

**Verification**:
```bash
# Verify P2P is running
curl http://localhost:8090/p2p/status
# Should show peer count, protocols

# Verify content sync
# Create content on alpha, verify it appears on staging via P2P
curl http://alpha-storage:8090/db/content/test-id
curl http://staging-storage:8090/db/content/test-id
# Both should return same content (synced via libp2p)
```

---

## Sprint 7: Network Topology Modeling

**Goal**: Use K8s StatefulSets to model real P2P network; enable topology testing.

**Scope**:
1. Convert edgenode Deployments to StatefulSets
2. Create headless Services for DNS-based peer discovery
3. Configure shared bootstrap/signal servers
4. Add network partition testing capability

**Files to Create**:
- `orchestrator/manifests/edgenode/alpha-statefulset.yaml`
- `orchestrator/manifests/edgenode/staging-statefulset.yaml`
- `orchestrator/manifests/edgenode/headless-service-alpha.yaml`
- `orchestrator/manifests/edgenode/headless-service-staging.yaml`

**Files to Modify**:
- `orchestrator/manifests/edgenode/*.yaml` - Convert Deployments to StatefulSets
- `holochain/edgenode/conductor-config.yaml` - Shared bootstrap URL

**Forcing Function**:
- StatefulSets provide stable network identities (required for P2P)
- Headless services enable peer discovery via DNS
- Network policies can simulate partitions

**Verification**:
```bash
# Verify StatefulSet stable identities
kubectl get pods -n elohim-alpha
# Should show: edgenode-alpha-0, edgenode-alpha-1

# Verify DNS discovery
nslookup edgenode-alpha.elohim-alpha.svc.cluster.local
# Should return all pod IPs

# Verify DHT gossip
kubectl logs edgenode-alpha-0 -c conductor | grep "peer discovered"
# Should show staging peers discovered
```

---

## Dependencies Between Sprints (Updated)

```
Sprint 3 (K8s Isolation) ─────── COMPLETE ✓
Sprint 0 (Schema Versioning) ── COMPLETE ✓
Sprint 1 (Artifact Storage) ─── COMPLETE ✓ (was already done)
Sprint 2 (Pipeline Parallel) ── COMPLETE ✓ (parallel execution, genesis cron deferred)
    │
    ▼
Sprint 4 (Seeder Versioning) ── NEXT
    │
    ▼
Sprint 6 (P2P Activation)
    │
    ▼
Sprint 7 (Network Topology)

[Sprint 5 (DNA Isolation) - DEFERRED]
```

**Recommended Execution** (updated 2026-02-08):
1. **DONE**: Sprints 0, 1, 2, 3 all complete (code + operational)
2. **Next**: Sprint 4 (seeder version negotiation)
3. **Future**: Sprints 6+7 (P2P activation and topology)

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
6. **Sprint 5**: DNA builds require explicit trigger flag
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
