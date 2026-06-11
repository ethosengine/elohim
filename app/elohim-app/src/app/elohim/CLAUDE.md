---
id: elohim-pillar-gospel
cites:
  - elohim-domain-gospel | the cross-cutting coordination subject this pillar renders — signal kinds, constitutional ratios, shared primitives (renders, never redefines) | sha256:68751b91f9749048 | path: elohim/sdk/domains/elohim/CLAUDE.md
---

# Elohim Pillar - Protocol Core

Infrastructure layer: data loading, agents, trust, source chain.

**Architecture:** `ELOHIM_PROTOCOL_ARCHITECTURE.md`

## Subject home & citation discipline (this pillar is a CONSUMER)

This Angular pillar renders the protocol core; the cross-cutting subject (cross-pillar `signalKinds`,
`constitutionalRatios`, shared coordination vocabulary) is owned at the cited subject home
`elohim-domain-gospel` (`elohim/sdk/domains/elohim/`). The cite is content-addressed: a change at the subject
home drifts this gospel STALE for re-verification.

**Where code citations to the subject belong:**
- When code encodes a cross-pillar coordination assumption (a signal kind, a constitutional ratio, a shared
  reach/mastery primitive), leave a `// subject: elohim-domain-gospel` breadcrumb at the assumption site.
- The shared models here (`protocol-core.model.ts`, `zome-wire-types.ts`) mirror substrate/wire types — when a
  wire shape moves upstream, cite the substrate rather than forking a hand-copy. (`agent.model.ts` and
  `source-chain.model.ts` migrated to `@elohim/service` — Slice 2.1/2.1b — and are re-exported through the barrel.)

## Models

| Model | Purpose |
|-------|---------|
| `protocol-core.model.ts` | Shared primitives (ReachLevel, AffinityScope, CrossPillarLinkType, …) |
| `elohim-agent.model.ts` | Constitutional AI guardian types |
| `trust-badge.model.ts` | TrustIndicator/TrustIndicatorSet for UI display |
| `zome-wire-types.ts` | snake_case Holochain zome wire types (boundary-only) |
| `rea-bridge.model.ts`, `economic-event.model.ts` | REA/ValueFlows economic coordination |

Representative rows — `models/` holds 20 files; `models/index.ts` is the barrel. Agent types
(`Agent`, `AgentProgress`, `MasteryLevel`) and source-chain types (`SourceChainEntry`, `EntryLink`,
`LamadLinkType`) migrated to `@elohim/service` (Slice 2.1/2.1b) and are re-exported through the barrel.

**Cross-pillar link vocabulary:** `CrossPillarLinkType` lives in `models/protocol-core.model.ts:610`
(14 values incl. `custom`) — the founding doc-table vocabulary was fully replaced, and the current set
has no consumers outside the model file yet; verify before building on it.

## Services

| Service | Purpose |
|---------|---------|
| `DataLoaderService` | Index/path/content reads — projection-first, ContentService fallback, IDB cache |
| `AgentService` | Current agent (session or authenticated), progress, attestation checks |
| `ElohimAgentService` | Constitutional AI invocation (pluggable backend) |
| `TrustBadgeService` | Compute trust indicators from attestations |
| `LocalSourceChainService` | Agent-centric localStorage chain — migrated to `@elohim/service` (Slice 2.1b), re-exported via `services/index.ts` |

Representative rows — `services/` now holds ~60 services (content, storage-api, projection-api,
governance, human-consent, profile, affinity-tracking, epr-nav, …); `services/index.ts` is the barrel.

## Key Types

```typescript
// Mastery progression (Bloom's Taxonomy) — canonical home: @elohim/service
// (app/elohim-library/projects/elohim-service/src/angular/models/agent.model.ts:219)
// Generated schema-enums.ts:300 carries an 11-value superset (adds recognize/recall/synthesize)
type MasteryLevel =
  | 'not_started' | 'seen' | 'remember' | 'understand'
  | 'apply' | 'analyze' | 'evaluate' | 'create';

// Geographic/jurisdictional visibility scope (models/protocol-core.model.ts:50)
type ReachLevel =
  | 'private' | 'invited' | 'local' | 'neighborhood'
  | 'municipal' | 'bioregional' | 'regional' | 'commons';
// DRIFT: the DNA-notarized schema enum (elohim/sdk/schemas/v1/enums/reach.schema.json) is a
// DIFFERENT 8 (private/self/intimate/trusted/familiar/community/public/commons) — known
// reconciliation backlog (genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md);
// do not "fix" either side to match the other here.

// Trust indicator for UI (models/trust-badge.model.ts:188)
interface TrustIndicator {
  id: string;
  polarity: 'positive' | 'negative';
  priority: number; // 1-100 display prominence
  icon: string;
  label: string;
  description: string;
  color: BadgeColor;
  verified: boolean;
  source: IndicatorSource;
  timestamp: string;
  sourceType: ContentAttestationType | BadgeWarning['type'];
}
```

## DataLoaderService Contract

Legacy index/path adapter — delegates to ProjectionApiService with ContentService fallback and an
IndexedDB cache. No longer the only data-source-aware service (see API Boundary Architecture below).

```typescript
getPath(pathId: string): Observable<LearningPath>;      // delegates to getContent() + parsePathView
getContent(resourceId: string): Observable<ContentNode>;
getPathIndex(): Observable<PathIndex>;
getContentIndex(): Observable<ContentIndex>;            // heavy (~1000 items); use checkReadiness() for liveness
```

## Holochain Adoption State

The storage/DHT architecture has landed (see API Boundary Architecture below): reads flow
projection → ContentService → IndexedDB cache behind `DataLoaderService`; zome calls go through
`HolochainClientService` (snake_case wire). Pre-Holochain local chains still exist:
`LocalSourceChainService` (`@elohim/service`) packages them via `prepareMigration()` →
`ChainMigrationPackage` for source-chain migration.

---

## API Boundary Architecture

### The Boundary Rule

**snake_case never leaves Rust. TypeScript works with camelCase only.**

All transformations (JSON parsing, boolean coercion, case conversion) happen in Rust's `views.rs`.
TypeScript receives clean, ready-to-use objects.

### Service Layer Stack

```
┌──────────────────────────────────────────────────────────────┐
│  UI Components (thin)                                        │
│  - Inject domain services                                    │
│  - Bind to observables                                       │
│  - Minimal logic                                             │
└──────────────────────────────────────────────────────────────┘
                           ↓
┌──────────────────────────────────────────────────────────────┐
│  Domain Services (lamad/, imagodei/, shefa/, qahal/)         │
│  - Business logic                                            │
│  - Orchestration                                             │
│  - Domain-specific queries                                   │
└──────────────────────────────────────────────────────────────┘
                           ↓
┌──────────────────────────────────────────────────────────────┐
│  API Services (elohim/services/)                             │
│  - StorageApiService: HTTP to elohim-storage (camelCase)     │
│  - HolochainClientService: WebSocket to zomes (snake_case)   │
│  - ProjectionApiService: Read-only projection cache          │
└──────────────────────────────────────────────────────────────┘
                           ↓
┌──────────────────────────────────────────────────────────────┐
│  Rust Boundary (elohim-storage)                              │
│  - views.rs: camelCase ↔ snake_case transformation           │
│  - http.rs: Routes using View/InputView types                │
│  - db/: Internal snake_case + String JSON                    │
└──────────────────────────────────────────────────────────────┘
```

### Key Services

| Service | Purpose | Data Format |
|---------|---------|-------------|
| `StorageApiService` | HTTP to elohim-storage SQLite | camelCase (clean) |
| `HolochainClientService` | WebSocket to Holochain zomes | snake_case (Rust) |
| `ProjectionApiService` | REST from doorway cache | camelCase (clean) |
| `DoorwayClientService` | Doorway proxy management | Mixed |

### StorageApiService Pattern

```typescript
// Send camelCase objects directly - no transformation needed
createContent(input: CreateContentInputView): Observable<ContentView> {
  return this.http.post<ContentView>('/db/content', input);
}

// Query params are also camelCase
getRelationships(query: RelationshipQuery): Observable<RelationshipView[]> {
  let params = new HttpParams();
  if (query.sourceId) params = params.set('sourceId', query.sourceId);
  if (query.relationshipType) params = params.set('relationshipType', query.relationshipType);
  return this.http.get<RelationshipView[]>('/db/relationships', { params });
}
```

### HolochainClientService Exception

Holochain zome calls use snake_case because zomes are Rust:

```typescript
// Intentional snake_case - Holochain zomes expect it
this.callZome('content_store', 'get_content', { content_id: id });
```

This is documented with `TODO: [HOLOCHAIN-ZOME]` comments in the codebase.

### Adapters (Derived Fields Only)

See `adapters/CLAUDE.md` for adapter patterns. Adapters compute derived fields
from API responses - they do NOT parse JSON or convert case.

```typescript
// Adapter adds computed field
export function withFullyConsentedFlag(view: HumanRelationshipViewBase) {
  return {
    ...view,
    isFullyConsented: view.consentGivenByA && view.consentGivenByB,
  };
}
```
