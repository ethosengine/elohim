# Decoupling Sprint Plan: P2P-Ready CI/CD Architecture

## Vision
Transform the monorepo from tightly-coupled sequential builds to independent, version-aware components that can model real P2P network topology using Kubernetes.

## Current State (as of 2026-02-05)

**Resolved:**
1. ~~**Shared Infrastructure**: Alpha + Staging share `doorway-dev.elohim.host`~~ -- **FIXED** (Sprint 3)
2. ~~**Ephemeral Storage**: `emptyDir` volumes lose data on restart~~ -- **FIXED** (PVCs deployed)

**Remaining:**
3. **Pipeline Coupling**: DNA → Edge → App → Genesis runs sequentially; one failure blocks all
4. **No Schema Versioning**: Seeder JSON has no `schema_version`; Rust structs have tight serde coupling
5. **Artifact Fragility**: `wget lastSuccessfulBuild` fetches from Jenkins (race conditions, timeouts)
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
| 3 | K8s Environment Isolation | **~85% COMPLETE** | ~~IMMEDIATE~~ |
| 0 | Schema Versioning Foundation | **~10% COMPLETE** | **NEXT** |
| 1 | Artifact Storage Decoupling | Not started | HIGH |
| 2 | Pipeline Parallelization | Not started | HIGH |
| 4 | Seeder Version Negotiation | Not started | MEDIUM |
| 6 | P2P Activation | Not started | MEDIUM |
| 7 | Network Topology Modeling | Not started | FUTURE |
| 5 | DNA Build Isolation | Not started | **DEFERRED** |

**Execution Order**: ~~0 → 3~~ → finish Sprint 3 cleanup → 0 → 1 → 2 → 4 → 6 → 7 (Sprint 5 deferred)

---

## Sprint 0: Schema Versioning Foundation -- ~10% COMPLETE

**Goal**: Establish version-aware data contracts before any other decoupling.

**Status**: Partial. `#[serde(default)]` widely used (89 occurrences in views.rs) but tolerant reader pattern and schemaVersion not yet added.

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| `#[serde(default)]` on InputView fields | PARTIAL | 89 occurrences in `views.rs`; coverage audit needed |

### What Remains

| Item | Priority | Notes |
|------|----------|-------|
| Add `schemaVersion` to seed JSON models | HIGH | `genesis/seeder/src/seed-entities.ts` interfaces need field |
| Complete `#[serde(default)]` audit | MEDIUM | Verify ALL optional fields covered in `views.rs` |
| Add `#[serde(flatten)] extra: HashMap<String, Value>` | HIGH | Tolerant reader for unknown fields |
| Generate TypeScript types from ts-rs | MEDIUM | `cargo test export_bindings` (ts annotations exist) |
| Add CI lint check for required fields | LOW | Enforce schema discipline |

**Files to Modify**:
- `holochain/elohim-storage/src/views.rs` - Complete `#[serde(default)]` + add `#[serde(flatten)] extra: HashMap<String, Value>`
- `genesis/seeder/src/seed-entities.ts` - Add `schemaVersion: number` to all content types
- `genesis/seeder/src/validation-constants.ts` - Export from ts-rs bindings

**Verification**:
```bash
# Test tolerant reader
curl -X POST http://localhost:8090/db/content/bulk \
  -d '[{"id":"test","title":"Test","schemaVersion":99,"unknownField":"ignored"}]'
# Should succeed, not reject unknown fields
```

---

## Sprint 1: Artifact Storage Decoupling (Harbor)

**Goal**: Remove Jenkins `lastSuccessfulBuild` artifact fetching; use versioned Harbor artifacts.

**Scope**:
1. Create Harbor project `elohim-artifacts` for versioned hApp storage
2. Modify DNA pipeline to push `elohim.happ` with version tag to Harbor
3. Modify Edge/App/Steward pipelines to fetch from Harbor by version
4. Add `HAPP_VERSION` to VERSION file

**Files to Modify**:
- `holochain/dna/Jenkinsfile` - Add Harbor push stage after hApp build
- `holochain/Jenkinsfile` - Replace `fetchHappArtifact()` with Harbor fetch
- `Jenkinsfile` (app) - Replace WASM fetch with Harbor
- `steward/Jenkinsfile` - Replace hApp fetch with Harbor
- `VERSION` - Add `HAPP_VERSION=x.y.z` line

**Harbor Setup**:
```bash
# Create Harbor project (one-time)
curl -X POST "https://harbor.ethosengine.com/api/v2.0/projects" \
  -d '{"project_name":"elohim-artifacts","public":false}'
```

**Forcing Function**:
- Remove `fetchHappArtifact()` function entirely
- CI fails if `HAPP_VERSION` not found in VERSION file
- Harbor artifacts require explicit version tag (no `latest` allowed)

**Verification**:
```bash
# Verify artifact in Harbor
curl https://harbor.ethosengine.com/api/v2.0/projects/elohim/repositories/happ/artifacts
# Should show versioned tags, not "latest"
```

---

## Sprint 2: Pipeline Parallelization

**Goal**: Enable parallel Edge + App builds; decouple Genesis from deployment.

**Scope**:
1. Modify orchestrator to run Edge + App in parallel (not sequential)
2. Add branch-specific webhook triggers to Edge/App pipelines
3. Convert Genesis to scheduled job (not deployment-triggered)
4. Add manual re-seed capability

**Files to Modify**:
- `orchestrator/Jenkinsfile` - Parallel stage for Edge + App (lines 226-248)
- `holochain/Jenkinsfile` - Add webhook trigger for `holochain/**` changes
- `Jenkinsfile` (app) - Add webhook trigger for `elohim-app/**` changes
- `genesis/Jenkinsfile` - Add cron trigger, remove orchestrator dependency

**Forcing Function**:
- Orchestrator `PIPELINES` map removes `dependsOn` for Edge/App
- Genesis pipeline has `triggers { cron('H */6 * * *') }`
- Each pipeline can run standalone without orchestrator

**Verification**:
```bash
# Trigger Edge pipeline directly (not via orchestrator)
curl -X POST "https://jenkins/job/elohim-edge/job/dev/build"
# Should succeed without DNA rebuild
```

---

## Sprint 3: K8s Environment Isolation ⭐ ~85% COMPLETE

**Goal**: Separate alpha/staging/prod into independent deployments with dedicated namespaces.

**Status**: Substantially complete. Core isolation is operational. Cleanup items remain.

### What's Done

| Item | Status | Evidence |
|------|--------|----------|
| K8s namespaces created | DONE | `elohim-alpha`, `elohim-staging`, `elohim-prod` all Active |
| `edgenode-alpha.yaml` | DONE | Full manifest: namespace, ConfigMap, secrets, PVCs, Deployment, Ingress |
| `edgenode-staging.yaml` | DONE | Full manifest: namespace, ConfigMap, secrets, PVCs, Deployment, Ingress |
| PVCs for alpha | DONE | `holochain-data-alpha` (10Gi), `storage-data-alpha` (5Gi) embedded in manifest |
| PVCs for staging | DONE | `holochain-data-staging` (10Gi), `storage-data-staging` (5Gi) embedded in manifest |
| App ConfigMaps | DONE | `elohim-app/manifests/alpha/configmap.yaml` → `doorway-alpha.elohim.host` |
| | | `elohim-app/manifests/staging/configmap.yaml` → `doorway-staging.elohim.host` |
| Edge Ingress (alpha) | DONE | `doorway-alpha.elohim.host`, `signal.doorway-alpha.elohim.host` |
| Edge Ingress (staging) | DONE | `doorway-staging.elohim.host`, `signal.doorway-staging.elohim.host` |
| Jenkinsfile updates | DONE | All 4 Jenkinsfiles updated: namespace refs, stage names, health URLs |
| `doorway-dev` refs eliminated | DONE | Zero `doorway-dev` references remain in any Jenkinsfile |
| Genesis doorway resolution | DONE | `resolveDoorwayHost()` returns cluster-local service per environment |
| Orchestrator endpoints | DONE | VERSION_ENDPOINTS + HEALTH_ENDPOINTS use `doorway-alpha`/`doorway-staging` |
| Staging deployment stages | DONE | Edge + App pipelines have dedicated staging deploy stages |
| dev → staging promotion | DONE | Pushed to dev, ready for merge to staging branch |

### What Remains

| Item | Priority | Notes |
|------|----------|-------|
| Apply staging ConfigMap to K8s | **PRE-MERGE** | `kubectl apply -f elohim-app/manifests/staging/configmap.yaml` |
| Merge dev → staging | **NEXT** | First staging build will deploy all components |
| Delete `edgenode-dev.yaml` | AFTER VERIFY | Anti-regression safeguard; delete after staging verified healthy |
| App ingress namespace migration | LOW | `elohim-app/manifests/ingress.yaml` still uses `ethosengine` namespace |
| Version-control `namespaces.yaml` | LOW | Namespaces deployed but not in repo |
| NetworkPolicy for cross-env isolation | LOW | Nice-to-have; not blocking |

**Verification** (after staging merge):
```bash
# Health checks
curl -sf https://doorway-alpha.elohim.host/health
curl -sf https://doorway-staging.elohim.host/health
curl -sf https://staging.elohim.host

# Version match
curl -sf https://staging.elohim.host/version.json
curl -sf https://alpha.elohim.host/version.json
```

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
- `holochain/manifests/edgenode-alpha-statefulset.yaml`
- `holochain/manifests/edgenode-staging-statefulset.yaml`
- `holochain/manifests/headless-service-alpha.yaml`
- `holochain/manifests/headless-service-staging.yaml`

**Files to Modify**:
- `holochain/manifests/edgenode-*.yaml` - Convert to StatefulSet
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
Sprint 3 (K8s Isolation) ─────── ~85% DONE ✓
    │
    ├── Finish: merge dev→staging, verify, delete edgenode-dev.yaml
    │
    ▼
Sprint 0 (Schema Versioning) ── ~10% DONE, NEXT UP
    │
    ├──► Sprint 1 (Artifact Storage) ───┐
    │                                    │
    │    Sprint 2 (Pipeline Parallel) ◄──┘
    │         │
    ▼         ▼
Sprint 4 (Seeder Versioning) ◄───────────┘
    │
    ▼
Sprint 6 (P2P Activation)
    │
    ▼
Sprint 7 (Network Topology)

[Sprint 5 (DNA Isolation) - DEFERRED]
```

**Recommended Execution** (updated 2026-02-05):
1. **NOW**: Finish Sprint 3 -- merge dev→staging, verify staging health, delete `edgenode-dev.yaml`
2. **Next session**: Sprint 0 (Schema foundation -- tolerant readers + schemaVersion)
3. **Following**: Sprints 1+2 in parallel (pipeline decoupling)
4. **Then**: Sprint 4 (seeder versioning)
5. **Future**: Sprints 6+7 (P2P activation and topology)

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
3. **Sprint 2**: Orchestrator `dependsOn` removed from config
4. **Sprint 3**: ~~Shared `edgenode-dev.yaml` deleted after staging works~~ -- PENDING: delete after staging verified
   - DONE: All `doorway-dev` refs eliminated from Jenkinsfiles (can't accidentally route to wrong env)
   - DONE: Dedicated PVCs per environment (can't use emptyDir anymore)
   - DONE: Orchestrator endpoints point to per-environment URLs
5. **Sprint 4**: Old bulk endpoint deprecated with warning logs
6. **Sprint 5**: DNA builds require explicit trigger flag
7. **Sprint 6**: `--enable-p2p` required for inter-node sync
8. **Sprint 7**: Deployments converted to StatefulSets (can't revert easily)

---

## Quick Reference: Current Sprint Checklists

### Sprint 3 Checklist (K8s Isolation) -- ~85% COMPLETE
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
- [ ] Apply staging ConfigMap to K8s cluster (`kubectl apply -f elohim-app/manifests/staging/configmap.yaml`)
- [ ] Merge dev → staging and verify staging health
- [ ] Delete edgenode-dev.yaml after staging verified healthy
- [ ] Migrate app ingress from `ethosengine` namespace (low priority)

### Sprint 0 Checklist (Schema Versioning) -- ~10% COMPLETE
- [~] Add `#[serde(default)]` to all optional InputView fields (89 occurrences exist; audit needed)
- [ ] Add `schemaVersion` to seed JSON models
- [ ] Add `#[serde(flatten)] extra: HashMap<String, Value>` for unknown fields
- [ ] Generate TypeScript types from ts-rs
- [ ] Add CI lint check for required fields
