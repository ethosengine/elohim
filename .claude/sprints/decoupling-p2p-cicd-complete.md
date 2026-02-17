# Decoupling Sprint Plan: P2P-Ready CI/CD Architecture (COMPLETE)

> **Status**: All sprints (0-9) complete. This is the archive.
> **Active work**: See [scaling-identity.md](./scaling-identity.md) for Sprint 10+.

## Vision
Transform the monorepo from tightly-coupled sequential builds to independent, version-aware components that can model real P2P network topology using Kubernetes.

## Sprint Index

| Sprint | Focus | Status | Key Outcome |
|--------|-------|--------|-------------|
| 0 | Schema Versioning Foundation | COMPLETE | `schema_version` on all InputViews, tolerant readers |
| 1 | Artifact Storage Decoupling | COMPLETE | Harbor registry, floating tags, no `lastSuccessfulBuild` |
| 2 | Pipeline Parallelization | COMPLETE | Dependency-level parallel execution |
| 3 | K8s Environment Isolation | COMPLETE | Dedicated namespaces, per-env manifests, NetworkPolicies |
| 4 | Seeder Version Negotiation | COMPLETE | Schema validation on bulk endpoints, pre-flight checks |
| 5 | DNA Build Isolation | COMPLETE | `cascades: false`, version metadata in hApp |
| 6 | P2P Activation | COMPLETE | Bootstrap dialing, SyncManager, `/p2p/status` |
| 7 | Network Topology Modeling | COMPLETE | StatefulSets, headless services, P2P ports |
| 8 | Doorway Separation | COMPLETE | Doorway Deployment extracted from P2P StatefulSet |
| 9 | Dual Scaling Axes | COMPLETE | ConductorRegistry, PROJECTION_WRITER, read replicas, volumeClaimTemplates |

## Dependency Graph

```
Sprint 3 (K8s Isolation) ─────── COMPLETE
Sprint 0 (Schema Versioning) ── COMPLETE
Sprint 1 (Artifact Storage) ─── COMPLETE (was already done)
Sprint 2 (Pipeline Parallel) ── COMPLETE
    │
    ├──► Sprint 4 (Seeder Versioning) ── COMPLETE
    │
    └──► Sprint 5 (DNA Isolation) ────── COMPLETE
              │
              ▼
         Sprint 6 (P2P Activation) ──── COMPLETE
              │
              ▼
         Sprint 7 (Network Topology) ── COMPLETE
              │
              ▼
         Sprint 8 (Doorway Separation) ── COMPLETE
              │
              ▼
         Sprint 9 (Dual Scaling Axes) ── COMPLETE
```

## Anti-Regression Safeguards

1. **Sprint 0**: CI lint check rejects required fields without defaults
2. **Sprint 1**: `fetchHappArtifact()` function deleted entirely
3. **Sprint 2**: `groupByDependencyLevel()` replaces sequential execution
4. **Sprint 3**: Shared `edgenode-dev.yaml` deleted; dedicated PVCs per environment
5. **Sprint 4**: Old bulk endpoint deprecated with warning logs
6. **Sprint 5**: `cascades: false` flag; version metadata in hApp YAML comments
7. **Sprint 6**: `--enable-p2p` required for inter-node sync
8. **Sprint 7**: Deployments converted to StatefulSets
9. **Sprint 8**: Doorway containers removed from StatefulSet; Ingress points to doorway service
10. **Sprint 9**: PROJECTION_WRITER flag gates signal subscriber; ConductorRegistry in MongoDB; Ingress read/write split; volumeClaimTemplates replace static PVCs

---

## Sprint Details

### Sprint 0: Schema Versioning Foundation

**Goal**: Version-aware data contracts before any decoupling.

**What shipped**:
- `schema_version: u32` on all 11 InputView structs (default=1)
- `schemaVersion` on TS seed entity interfaces + builders
- Tolerant reader guarantee (serde ignores unknown fields)
- Compile-time lint: `all_input_views_have_schema_version_field`
- TypeScript types regenerated via ts-rs

**Files**: `holochain/elohim-storage/src/views.rs`, `genesis/seeder/src/seed-entities.ts`, `genesis/seeder/src/validation-constants.ts`

---

### Sprint 1: Artifact Storage Decoupling (Harbor)

**Goal**: Remove Jenkins `lastSuccessfulBuild`; use versioned Harbor artifacts.

**What shipped**: Already implemented. All pipelines use `oras` CLI with Harbor, floating tag strategy, commit-based versioning.

---

### Sprint 2: Pipeline Parallelization

**Goal**: Enable parallel Edge + App builds; decouple Genesis.

**What shipped**:
- `groupByDependencyLevel()` for topological sort
- `parallel` block executes same-level builds concurrently
- DNA (level 0) → Edge + App (level 1) → Genesis (sequential after)
- `propagateDependencies()` auto-includes dependents

**Files**: `orchestrator/Jenkinsfile`

---

### Sprint 3: K8s Environment Isolation

**Goal**: Separate alpha/staging/prod into independent K8s deployments.

**What shipped**:
- Namespaces: `elohim-alpha`, `elohim-staging`, `elohim-prod`
- Per-environment manifests (edgenode + doorway + app ConfigMaps)
- Per-environment Ingress: `doorway-alpha.elohim.host`, `doorway-staging.elohim.host`
- NetworkPolicy for cross-env isolation
- All `doorway-dev` references eliminated

**Files**: `orchestrator/manifests/edgenode/alpha.yaml`, `staging.yaml`, `namespaces.yaml`, `network-policies.yaml`, all 4 Jenkinsfiles

---

### Sprint 4: Seeder Version Negotiation

**Goal**: Seeder declares schema version; storage validates.

**What shipped**:
- `validate_schema_versions()` on all 7 bulk endpoints
- `X-Schema-Version` / `X-Supported-Schema-Versions` headers
- `GET /db/schema` capability discovery
- Seeder pre-flight schema check

**Files**: `holochain/elohim-storage/src/views.rs`, `http.rs`, `services/response.rs`, `genesis/seeder/src/doorway-client.ts`, `seed.ts`

---

### Sprint 5: DNA Build Isolation

**Goal**: DNA changes don't cascade to Edge rebuilds.

**What shipped**:
- `cascades: false` on DNA pipeline config
- `propagateDependencies()` respects flag
- Version metadata injected into hApp manifest comments
- RNA migration framework already existed (~2000 lines)

**Files**: `orchestrator/Jenkinsfile`, `holochain/dna/Jenkinsfile`, `holochain/Jenkinsfile`

---

### Sprint 6: P2P Activation

**Goal**: Wire up dormant libp2p infrastructure.

**What shipped**:
- Bootstrap node dialing in `start()`
- SyncManager wired to HttpServer
- `/p2p/status` endpoint (peer_id, addresses, peers, bootstrap, sync docs)
- `P2PHandle` (Send+Sync safe via watch channel)

**Files**: `holochain/elohim-storage/src/p2p/mod.rs`, `http.rs`, `main.rs`, `lib.rs`

---

### Sprint 7: Network Topology Modeling

**Goal**: StatefulSets model real P2P network topology.

**What shipped**:
- Deployment → StatefulSet conversion (alpha + staging)
- Headless services for DNS-based peer discovery
- P2P env vars: `ENABLE_P2P=true`, `P2P_PORT=9876`, `DISABLE_MDNS=true`
- Cross-namespace NetworkPolicy for P2P port 9876
- `deployEdgeWithManifest()` auto-detects StatefulSet vs Deployment

**Files**: `orchestrator/manifests/edgenode/alpha.yaml`, `staging.yaml`, `network-policies.yaml`, `holochain/Jenkinsfile`

---

### Sprint 8: Doorway Separation

**Goal**: Extract doorway from P2P StatefulSet into independent Deployment.

**What shipped**:
- `doorway/alpha.yaml`, `doorway/staging.yaml` (new Deployment manifests)
- Doorway containers removed from edgenode StatefulSets
- ws-proxy expanded to bridge admin (8444→4444) + app (8445→4445)
- `deployEdgeWithManifest()` gains `resourceName` parameter
- ClusterIP service exposes conductor ports for doorway

**Architecture**:
```
Browser → Ingress → Doorway Deployment → ClusterIP → P2P StatefulSet
```

**Files**: `orchestrator/manifests/doorway/alpha.yaml`, `staging.yaml`, `orchestrator/manifests/edgenode/alpha.yaml`, `staging.yaml`, `holochain/Jenkinsfile`

---

### Sprint 9: Dual Scaling Axes

**Goal**: Establish both scaling axes (projection reads + conductor pool) as architectural fact.

**Design Principle**: No Modes — every doorway instance is a full doorway. The only differentiation is `PROJECTION_WRITER=true/false`.

**What shipped**:

Doorway Rust code:
- `projection_writer: bool` flag (env: `PROJECTION_WRITER`, default: `true`)
- `conductor_urls: Option<String>` (env: `CONDUCTOR_URLS`, comma-separated)
- `ConductorRegistry` module: DashMap agent→conductor mapping, MongoDB backing, least-loaded selection
- Admin API: `GET /admin/conductors`, `/admin/conductors/{id}/agents`, `/admin/agents/{key}/conductor`
- Signal subscriber gated by `projection_writer` flag
- Health endpoint: pool_size, ProjectionRole, writer-aware readiness

K8s manifests:
- `volumeClaimTemplates` replace static PVCs (alpha + staging)
- Staging edgenode: `replicas: 2` (models 2-conductor pool)
- `doorway/staging-read.yaml` (new): read replica Deployment with `PROJECTION_WRITER=false`, 2 replicas
- Ingress read/write split: cache/store/blob/stream → readers, everything else → writer
- `CONDUCTOR_URLS` wired to headless service DNS names

CI/CD:
- `holochain/Jenkinsfile`: staging-read deploy step

**Files**: `doorway/src/config.rs`, `conductor/mod.rs`, `conductor/registry.rs`, `routes/admin_conductors.rs`, `routes/mod.rs`, `routes/health.rs`, `server/http.rs`, `main.rs`, `lib.rs`, `orchestrator/manifests/edgenode/alpha.yaml`, `staging.yaml`, `orchestrator/manifests/doorway/alpha.yaml`, `staging.yaml`, `staging-read.yaml`, `holochain/Jenkinsfile`, `doorway/SCALING.md`

---

## Configuration Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Artifact Storage | Harbor | Already integrated with K8s, version tagging built-in |
| Namespaces | Dedicated | `elohim-alpha`, `elohim-staging`, `elohim-prod` for full isolation |
| Priority | K8s Isolation First | Fix immediate pain (alpha/staging coupling) |
| RNA Framework | Deferred | Focus on data plane versioning; DNA migration later |
| Scaling Model | Dual-axis | Projection (Axis 1, replicas) + Identity (Axis 2, conductor pool) |
| Projection Writer | Single flag | `PROJECTION_WRITER=true/false` — no modes, all instances are full doorways |
