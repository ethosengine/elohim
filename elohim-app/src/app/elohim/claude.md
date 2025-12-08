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
