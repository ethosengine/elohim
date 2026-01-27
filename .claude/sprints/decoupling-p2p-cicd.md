# Decoupling Sprint Plan: P2P-Ready CI/CD Architecture

## Vision
Transform the monorepo from tightly-coupled sequential builds to independent, version-aware components that can model real P2P network topology using Kubernetes.

## Current State (Problems)
1. **Pipeline Coupling**: DNA → Edge → App → Genesis runs sequentially; one failure blocks all
2. **Shared Infrastructure**: Alpha + Staging share `doorway-dev.elohim.host` (one pod)
3. **Ephemeral Storage**: `emptyDir` volumes lose data on restart
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

| Sprint | Focus | Forcing Function | Priority |
|--------|-------|------------------|----------|
| 0 | Schema Versioning Foundation | All subsequent work must be version-aware | **FIRST** |
| 3 | K8s Environment Isolation | Environments can't accidentally share | **IMMEDIATE** |
| 1 | Artifact Storage Decoupling | Pipelines must fetch versioned artifacts | HIGH |
| 2 | Pipeline Parallelization | Each pipeline independently triggerable | HIGH |
| 4 | Seeder Version Negotiation | Seeder declares version, storage handles multiple | MEDIUM |
| 6 | P2P Activation | Content syncs via libp2p, not deployment | MEDIUM |
| 7 | Network Topology Modeling | True P2P testing in K8s | FUTURE |
| 5 | DNA Build Isolation | DNA changes explicitly versioned | **DEFERRED** |

**Execution Order**: 0 → 3 → 1 → 2 → 4 → 6 → 7 (Sprint 5 deferred)

---

## Sprint 0: Schema Versioning Foundation

**Goal**: Establish version-aware data contracts before any other decoupling.

**Scope** (fits in one session):
1. Add `schemaVersion` field to seed JSON format
2. Add tolerant reader pattern to Rust InputView structs
3. Add `#[serde(default)]` to all optional fields
4. Create shared validation constants (ts-rs generated)

**Files to Modify**:
- `holochain/elohim-storage/src/views.rs` - Add `#[serde(default)]` and `#[serde(flatten)] extra: HashMap<String, Value>`
- `genesis/seeder/src/models/` - Add `schemaVersion: number` to all content types
- `genesis/seeder/src/validation-constants.ts` - Export from ts-rs bindings

**Forcing Function**:
- All InputView structs require `#[serde(deny_unknown_fields = false)]`
- CI adds lint check: "no required fields without defaults"
- Seeder fails if `schemaVersion` missing

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

## Sprint 3: K8s Environment Isolation ⭐ PRIORITY

**Goal**: Separate alpha/staging/prod into independent deployments with dedicated namespaces.

**Scope**:
1. Create dedicated namespaces: `elohim-alpha`, `elohim-staging`, `elohim-prod`
2. Create `edgenode-alpha.yaml` manifest (rename from dev, update URLs)
3. Create `edgenode-staging.yaml` manifest (new, independent)
4. Create `doorway-alpha.elohim.host` and `doorway-staging.elohim.host` DNS/Ingress
5. Add PersistentVolumeClaims (replace emptyDir)
6. Update app ConfigMaps with environment-specific doorway URLs
7. Migrate existing ethosengine resources to new namespaces

**Files to Create**:
- `holochain/manifests/namespaces.yaml` - Define all three namespaces
- `holochain/manifests/edgenode-alpha.yaml` - Alpha-specific (from edgenode-dev)
- `holochain/manifests/edgenode-staging.yaml` - Staging-specific (new)
- `holochain/manifests/pvc-alpha.yaml` - Persistent storage for alpha
- `holochain/manifests/pvc-staging.yaml` - Persistent storage for staging
- `elohim-app/manifests/ingress-alpha.yaml` - Alpha routing
- `elohim-app/manifests/ingress-staging.yaml` - Staging routing

**Files to Modify**:
- `elohim-app/manifests/configmap-alpha.yaml` - doorway URL → `doorway-alpha.elohim.host`
- `elohim-app/manifests/configmap-staging.yaml` - doorway URL → `doorway-staging.elohim.host`
- `elohim-app/manifests/*-deployment.yaml` - Update namespace to dedicated
- `holochain/Jenkinsfile` - Add staging deployment stage, update namespace references
- `genesis/Jenkinsfile` - Update `resolveDoorwayHost()` for alpha/staging split

**Forcing Function**:
- Delete shared `edgenode-dev.yaml` after alpha+staging work independently
- Ingress rules reject cross-environment routing (NetworkPolicy)
- PVCs ensure data persists (can't use emptyDir anymore)
- Namespace RBAC prevents cross-environment access

**Verification**:
```bash
# Verify namespaces created
kubectl get namespaces | grep elohim
# elohim-alpha, elohim-staging, elohim-prod

# Verify separate deployments
kubectl get deployments -n elohim-alpha
kubectl get deployments -n elohim-staging
# Should show independent pods

# Verify data persistence
kubectl delete pod -n elohim-alpha elohim-edgenode-alpha-0
# After restart, content should still exist (PVC)

# Verify isolation
kubectl exec -n elohim-alpha ... -- curl http://doorway-staging.elohim.host
# Should be blocked by NetworkPolicy
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

## Dependencies Between Sprints (Reordered)

```
Sprint 0 (Schema Versioning) ─── FOUNDATION
    │
    ▼
Sprint 3 (K8s Isolation) ─────── PRIORITY: Fix immediate pain
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

**Recommended Execution**:
1. **Week 1-2**: Sprint 0 (Schema foundation - lightweight)
2. **Week 2-4**: Sprint 3 (K8s isolation - immediate value)
3. **Week 4-5**: Sprints 1+2 in parallel (pipeline decoupling)
4. **Week 5-6**: Sprint 4 (seeder versioning)
5. **Week 6-8**: Sprints 6+7 (P2P activation and topology)

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
4. **Sprint 3**: Shared `edgenode-dev.yaml` deleted after staging works
5. **Sprint 4**: Old bulk endpoint deprecated with warning logs
6. **Sprint 5**: DNA builds require explicit trigger flag
7. **Sprint 6**: `--enable-p2p` required for inter-node sync
8. **Sprint 7**: Deployments converted to StatefulSets (can't revert easily)

---

## Quick Reference: First Two Sprints

### Sprint 0 Checklist (Schema Versioning)
- [ ] Add `schemaVersion` to seed JSON models
- [ ] Add `#[serde(default)]` to all optional InputView fields
- [ ] Add `#[serde(flatten)] extra: HashMap<String, Value>` for unknown fields
- [ ] Generate TypeScript types from ts-rs
- [ ] Add CI lint check for required fields

### Sprint 3 Checklist (K8s Isolation)
- [ ] Create namespaces: elohim-alpha, elohim-staging, elohim-prod
- [ ] Create edgenode-alpha.yaml (from edgenode-dev)
- [ ] Create edgenode-staging.yaml (new)
- [ ] Create PVCs for alpha and staging
- [ ] Update ConfigMaps with new doorway URLs
- [ ] Update Ingress for new DNS entries
- [ ] Update Jenkinsfiles for namespace changes
- [ ] Delete edgenode-dev.yaml after verification
