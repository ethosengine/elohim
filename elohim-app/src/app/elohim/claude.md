# Elohim Pillar - Protocol Core

Infrastructure layer: data loading, agents, trust, source chain.

**Architecture:** `ELOHIM_PROTOCOL_ARCHITECTURE.md`

## Models

| Model | Purpose |
|-------|---------|
| `protocol-core.model.ts` | Shared primitives (ReachLevel, GovernanceLayer, etc.) |
| `agent.model.ts` | Agent, AgentProgress, MasteryLevel (Bloom's Taxonomy) |
| `elohim-agent.model.ts` | Constitutional AI guardian types |
| `trust-badge.model.ts` | TrustIndicator for UI display |
| `source-chain.model.ts` | Holochain-style entry/link types |

## Services

| Service | Purpose |
|---------|---------|
| `DataLoaderService` | JSON fetching (Holochain adapter point) |
| `AgentService` | Agent profiles, progress, attestation checks |
| `ElohimAgentService` | Constitutional AI invocation |
| `TrustBadgeService` | Compute trust indicators from attestations |
| `LocalSourceChainService` | Agent-centric localStorage (pre-Holochain) |

## Key Types

```typescript
// Mastery progression (Bloom's Taxonomy)
type MasteryLevel =
  | 'not_started' | 'seen' | 'remember' | 'understand'
  | 'apply' | 'analyze' | 'evaluate' | 'create';

// Content visibility scope
type ReachLevel =
  | 'private' | 'invited' | 'local'
  | 'community' | 'federated' | 'commons';

// Trust indicator for UI
interface TrustIndicator {
  polarity: 'positive' | 'negative';
  icon: string;
  label: string;
  verified: boolean;
}
```

## DataLoaderService Contract

The ONLY service that knows about data sources. All others depend on this.

```typescript
getPath(pathId: string): Observable<LearningPath>;
getContent(resourceId: string): Observable<ContentNode>;
getPathIndex(): Observable<PathIndexEntry[]>;
getContentIndex(): Observable<ContentIndexEntry[]>;
```

When migrating to Holochain, only this service changes.

## Holochain Migration

- `id` fields become action hashes
- Progress moves to agent's private source chain
- Attestations become DHT entries with crypto verification
- `LocalSourceChainService` data migrates via `prepareMigration()`

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
