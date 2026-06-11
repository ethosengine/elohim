# ELOHIM-PILLAR Gospel Drift Repairs — DRAFT EDIT PLAN

Target: `/projects/elohim/app/elohim-app/src/app/elohim/CLAUDE.md` (id: elohim-pillar-gospel)
Verified against code 2026-06-11. Part A edits are true regardless of island-doc retirement (apply pre-gate).
Part B edits PRESUPPOSE island deletion — DEFERRED-UNTIL-GATE, do not apply now.

## Evidence ledger (all claims cited)

- `models/` holds 20 files (no `agent.model.ts`, no `source-chain.model.ts`): `ls app/elohim-app/src/app/elohim/models/`.
- `agent.model.ts` migrated to `@elohim/service` (Slice 2.1): `app/elohim-library/projects/elohim-service/src/angular/models/agent.model.ts` — `Agent` :32, `AgentProgress` :91, `MasteryLevel` :219-227 (exact 8-value Bloom enum as in gospel). Re-exported (types + helpers, NOT `MasteryLevel` itself) via `app/elohim-app/src/app/elohim/models/index.ts:28-45` ("migrated to @elohim/service — Slice 2.1"). Sole app import site of `MasteryLevel`: `app/elohim-app/src/app/imagodei/models/profile.model.ts:15` (`from '@elohim/service/angular/models/agent.model'`).
- A SECOND generated `MasteryLevel` exists: `app/elohim-app/src/app/generated/schema-enums.ts:300` — 11 values (`ALL_MASTERY_LEVELS` = the 8 Bloom + `recognize`/`recall`/`synthesize`).
- `source-chain.model.ts` migrated to `@elohim/service` (Slice 2.1b): `app/elohim-library/projects/elohim-service/src/angular/models/source-chain.model.ts` — `SourceChainEntry` :39, `EntryLink` :137, `LamadEntryType` :90, `LamadLinkType` :189, `ChainMigrationPackage` :359. Re-exported at `models/index.ts:104-105`.
- `ReachLevel` actual: `app/elohim-app/src/app/elohim/models/protocol-core.model.ts:50-58` — 8 geographic values (`private/invited/local/neighborhood/municipal/bioregional/regional/commons`); `REACH_LEVEL_VALUES` :63-72.
- Schema enum divergence: `elohim/sdk/schemas/v1/enums/reach.schema.json` enum = `["private","self","intimate","trusted","familiar","community","public","commons"]` (DNA-notarized, `content_store_integrity`). Known reconciliation backlog (Reach-enum-drift memory) — NOT adjudicated here.
- Services: `data-loader.service.ts`, `agent.service.ts`, `elohim-agent.service.ts`, `trust-badge.service.ts` exist in `app/elohim-app/src/app/elohim/services/`. `LocalSourceChainService` migrated to `@elohim/service` (Slice 2.1b) — re-export at `app/elohim-app/src/app/elohim/services/index.ts:10-11`; implementation `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts`, `prepareMigration(): ChainMigrationPackage | null` at :445. `services/` now holds ~60 services (`ls`).
- Service purpose rows re-verified from headers: AgentService "Manages the current agent (session or authenticated)" (`agent.service.ts` header), ElohimAgentService "Interface to autonomous constitutional guardians" (`elohim-agent.service.ts` header), TrustBadgeService "Computes UI-ready trust badges" (`trust-badge.service.ts` header).
- DataLoaderService signatures: `getPath(pathId: string): Observable<LearningPath>` `data-loader.service.ts:307` (delegates to `getContent()` + `parsePathView`, :302-313); `getContent(resourceId: string): Observable<ContentNode>` :408 (projection → ContentService fallback :412-419, IDB cache :446); `getContentIndex(): Observable<ContentIndex>` :646 (NOT `ContentIndexEntry[]`); `getPathIndex(): Observable<PathIndex>` :712 (NOT `PathIndexEntry[]`).
- `TrustIndicator` actual: `app/elohim-app/src/app/elohim/models/trust-badge.model.ts:188-222` — 11 fields (`id, polarity, priority, icon, label, description, color, verified, source, timestamp, sourceType`), not the 4-field shape in the gospel.
- `CrossPillarLinkType`: `app/elohim-app/src/app/elohim/models/protocol-core.model.ts:610` — 14 values incl. `custom`.
- `zome-wire-types.ts` header: "Centralized Holochain zome response types … WIRE FORMAT … snake_case" (`models/zome-wire-types.ts:1-11`) — correct stand-in for the "mirror substrate/wire types" sentence.
- Pointer lines (Part B): `app/elohim-app/src/app/elohim/CLAUDE.md:11`, `app/elohim-app/src/app/qahal/CLAUDE.md:14`, `app/elohim-app/src/app/imagodei/CLAUDE.md:11` (grep-confirmed exact). `ELOHIM_PROTOCOL_ARCHITECTURE.md` currently EXISTS at `app/elohim-app/src/app/elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md` (so today's pointers are live; repairs fire only when the island retires).

---

## PART A — APPLY-NOW edits (file: app/elohim-app/src/app/elohim/CLAUDE.md)

### Edit 1 — Subject-home sentence names a model that left the pillar (gospel :23-24)

old_string:
```
- The shared models here (`protocol-core.model.ts`, `agent.model.ts`) mirror substrate/wire types — when a
  wire shape moves upstream, cite the substrate rather than forking a hand-copy.
```

new_string:
```
- The shared models here (`protocol-core.model.ts`, `zome-wire-types.ts`) mirror substrate/wire types — when a
  wire shape moves upstream, cite the substrate rather than forking a hand-copy. (`agent.model.ts` and
  `source-chain.model.ts` migrated to `@elohim/service` — Slice 2.1/2.1b — and are re-exported through the barrel.)
```

### Edit 2 — Models table: two dead rows; compact rewrite (gospel :28-34)

old_string:
```
| Model | Purpose |
|-------|---------|
| `protocol-core.model.ts` | Shared primitives (ReachLevel, GovernanceLayer, etc.) |
| `agent.model.ts` | Agent, AgentProgress, MasteryLevel (Bloom's Taxonomy) |
| `elohim-agent.model.ts` | Constitutional AI guardian types |
| `trust-badge.model.ts` | TrustIndicator for UI display |
| `source-chain.model.ts` | Holochain-style entry/link types |
```

new_string:
```
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
```

### Edit 3 — NEW rail: cross-pillar link vocabulary (insert before `## Services`, gospel :36)

old_string:
```
## Services

| Service | Purpose |
```

new_string:
```
**Cross-pillar link vocabulary:** `CrossPillarLinkType` lives in `models/protocol-core.model.ts:610`
(14 values incl. `custom`) and is code-canonical — the old doc-table vocabulary was fully replaced.

## Services

| Service | Purpose |
```

### Edit 4 — Services table: LocalSourceChainService row + breadth note (gospel :40-44)

old_string:
```
| `DataLoaderService` | JSON fetching (Holochain adapter point) |
| `AgentService` | Agent profiles, progress, attestation checks |
| `ElohimAgentService` | Constitutional AI invocation |
| `TrustBadgeService` | Compute trust indicators from attestations |
| `LocalSourceChainService` | Agent-centric localStorage (pre-Holochain) |
```

new_string:
```
| `DataLoaderService` | Index/path/content reads — projection-first, ContentService fallback, IDB cache |
| `AgentService` | Current agent (session or authenticated), progress, attestation checks |
| `ElohimAgentService` | Constitutional AI invocation (pluggable backend) |
| `TrustBadgeService` | Compute trust indicators from attestations |
| `LocalSourceChainService` | Agent-centric localStorage chain — migrated to `@elohim/service` (Slice 2.1b), re-exported via `services/index.ts` |

Representative rows — `services/` now holds ~60 services (content, storage-api, projection-api,
governance, human-consent, profile, affinity-tracking, epr-nav, …); `services/index.ts` is the barrel.
```

### Edit 5 — Key Types: MasteryLevel provenance (gospel :49-52; values verified UNCHANGED)

old_string:
```
// Mastery progression (Bloom's Taxonomy)
type MasteryLevel =
  | 'not_started' | 'seen' | 'remember' | 'understand'
  | 'apply' | 'analyze' | 'evaluate' | 'create';
```

new_string:
```
// Mastery progression (Bloom's Taxonomy) — canonical home: @elohim/service
// (app/elohim-library/projects/elohim-service/src/angular/models/agent.model.ts:219)
// Generated schema-enums.ts:300 carries an 11-value superset (adds recognize/recall/synthesize)
type MasteryLevel =
  | 'not_started' | 'seen' | 'remember' | 'understand'
  | 'apply' | 'analyze' | 'evaluate' | 'create';
```

### Edit 6 — Key Types: ReachLevel wrong values + schema-enum drift annotation (gospel :54-57)

old_string:
```
// Content visibility scope
type ReachLevel =
  | 'private' | 'invited' | 'local'
  | 'community' | 'federated' | 'commons';
```

new_string:
```
// Geographic/jurisdictional visibility scope (models/protocol-core.model.ts:50)
type ReachLevel =
  | 'private' | 'invited' | 'local' | 'neighborhood'
  | 'municipal' | 'bioregional' | 'regional' | 'commons';
// DRIFT: the DNA-notarized schema enum (elohim/sdk/schemas/v1/enums/reach.schema.json) is a
// DIFFERENT 8 (private/self/intimate/trusted/familiar/community/public/commons) — known
// reconciliation backlog; do not "fix" either side to match the other here.
```

### Edit 7 — Key Types: TrustIndicator shape drift (gospel :59-65; found while correcting the same block)

old_string:
```
// Trust indicator for UI
interface TrustIndicator {
  polarity: 'positive' | 'negative';
  icon: string;
  label: string;
  verified: boolean;
}
```

new_string:
```
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

### Edit 8 — DataLoaderService Contract: 2 of 4 return types drifted + stale framing (gospel :68-79)

old_string:
```
The ONLY service that knows about data sources. All others depend on this.

```typescript
getPath(pathId: string): Observable<LearningPath>;
getContent(resourceId: string): Observable<ContentNode>;
getPathIndex(): Observable<PathIndexEntry[]>;
getContentIndex(): Observable<ContentIndexEntry[]>;
```

When migrating to Holochain, only this service changes.
```

new_string:
```
Legacy index/path adapter — delegates to ProjectionApiService with ContentService fallback and an
IndexedDB cache. No longer the only data-source-aware service (see API Boundary Architecture below).

```typescript
getPath(pathId: string): Observable<LearningPath>;      // delegates to getContent() + parsePathView
getContent(resourceId: string): Observable<ContentNode>;
getPathIndex(): Observable<PathIndex>;
getContentIndex(): Observable<ContentIndex>;            // heavy (~1000 items); use checkReadiness() for liveness
```
```

(Note: getPath/getContent signatures verified UNCHANGED at data-loader.service.ts:307/:408; index
methods return `PathIndex`/`ContentIndex` wrapper objects at :712/:646, not entry arrays.)

### Edit 9 — Holochain Migration section: stale-future framing → present-tense truth (gospel :81-86)

old_string:
```
## Holochain Migration

- `id` fields become action hashes
- Progress moves to agent's private source chain
- Attestations become DHT entries with crypto verification
- `LocalSourceChainService` data migrates via `prepareMigration()`
```

new_string:
```
## Holochain Adoption State

The storage/DHT architecture has landed (see API Boundary Architecture below): reads flow
projection → ContentService → IndexedDB cache behind `DataLoaderService`; zome calls go through
`HolochainClientService` (snake_case wire). Pre-Holochain local chains still exist:
`LocalSourceChainService` (`@elohim/service`) packages them via `prepareMigration()` →
`ChainMigrationPackage` for source-chain migration.
```

---

## PART B — DEFERRED ref-repairs (DO NOT APPLY — presuppose island deletion; DEFERRED-UNTIL-GATE)

`ELOHIM_PROTOCOL_ARCHITECTURE.md` exists today at `app/elohim-app/src/app/elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`,
so all three pointers are currently live. Apply these ONLY when the island docs retire.

### D1 — app/elohim-app/src/app/elohim/CLAUDE.md:11  [DEFERRED-UNTIL-GATE]

old_string:
```
**Architecture:** `ELOHIM_PROTOCOL_ARCHITECTURE.md`
```

new_string:
```
**Architecture:** pillar composition — root `CLAUDE.md` §Domain Pillars; founding lineage —
`genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-pillar-architecture-founding-arc.md`
```

### D2 — app/elohim-app/src/app/qahal/CLAUDE.md:14  [DEFERRED-UNTIL-GATE]

old_string:
```
**Architecture:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`
```

new_string:
```
**Architecture:** pillar composition — root `CLAUDE.md` §Domain Pillars; founding lineage —
`genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-pillar-architecture-founding-arc.md`
```

### D3 — app/elohim-app/src/app/imagodei/CLAUDE.md:11  [DEFERRED-UNTIL-GATE]

old_string (IDENTICAL text to D2 but in imagodei/CLAUDE.md — old/new pair is per-file, no uniqueness clash across files):
```
**Architecture:** `elohim/ELOHIM_PROTOCOL_ARCHITECTURE.md`
```

new_string:
```
**Architecture:** pillar composition — root `CLAUDE.md` §Domain Pillars; founding lineage —
`genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-pillar-architecture-founding-arc.md`
```

### Part B open items

- OPEN QUESTION: the history record `genesis/docs/content/elohim-protocol/history/2026-06-11-elohim-pillar-architecture-founding-arc.md` does NOT exist yet (verified absent 2026-06-11). D1-D3 assume the recompose authors it at gate time; if it lands under a different slug, update the three replacement lines before applying.
- OPEN QUESTION (adjacent, out of mandate): `app/elohim-app/src/app/qahal/CLAUDE.md:13` also points at `QAHAL_API_SPECIFICATION_v1.0.md` — another candidate island doc; not covered by this plan.
- Note: the pillar dir also contains `app/elohim-app/src/app/elohim/ARCHITECTURE.md` (separate file from ELOHIM_PROTOCOL_ARCHITECTURE.md); no gospel pointer references it, so no repair needed here.
