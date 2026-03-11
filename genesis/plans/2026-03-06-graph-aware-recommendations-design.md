# Graph-Aware Recommendations for Path Adaptation

_Design approved 2026-03-06_

## Problem

When a learner fails a mastery quiz, `PathAdaptationService.generateRecommendations()` flags the concept they struggled with but doesn't traverse the content graph to find prerequisite or reinforcing content (TODO at line 682). Recommendations are shallow — "you failed at X" rather than "you need Y before X."

## Vision: Two Layers of Graph Traversal

**Shallow (depth 1)** — what we're building now. Learner-to-content graph. Quick prerequisite/reinforcement lookup for adaptive learning in-session. Powers efficient context discovery, exploration, play, and adaptive learning.

**Deep (future, ElohimAgent-driven)** — learner-to-learner-to-content graph. Two people's mastery/affinity data traversed together via EPR reach and collective primitives. ElohimAgent synthesizes custom paths from the intersection — the Love Map path (currently hardcoded sample content) is the prototype. Deep traversal is orchestrated by ElohimAgent, not the adaptation service directly.

The shallow layer is a building block for the deep layer. A `maxGraphDepth` config in `PathAdaptationConfig` marks the seam between the two.

## Approach: Hybrid (Shallow Now, Config-Driven Depth)

Wire `RelationshipService` into `PathAdaptationService.generateRecommendations()` for depth-1 prerequisite and reinforcement lookups. Add `maxGraphDepth` config defaulting to 1. Structure the generation so deeper traversal (when ElohimAgent drives it) plugs in by changing the depth + the service call.

## Service Layer Changes

**File**: `elohim-app/src/app/lamad/quiz-engine/services/path-adaptation.service.ts`

In `generateRecommendations()` (replacing TODO at line 682):

1. Extract `conceptIds` from `triggerContext` (already available from failed quiz)
2. For each concept, call:
   - `RelationshipService.getRelationshipsByType(conceptId, 'PREREQUISITE')` — what should they have known first?
   - `RelationshipService.getRelationshipsByType(conceptId, 'REINFORCES')` — what covers this from another angle?
3. Rank results by relationship `confidence` score
4. Build `ContentRecommendation[]` with reasons:
   - `'prerequisite_gap'` for prerequisites
   - `'reinforcement'` for reinforcing content
   - `'struggled_with_concept'` for the original concept (fallback if no graph relationships exist)

**Config addition** to `PathAdaptationConfig`:

```typescript
maxGraphDepth: 1,
graphRelationshipTypes: ['PREREQUISITE', 'REINFORCES'],
```

No new services created. This is wiring existing services together.

## UI Surfaces

### Recommendation Cards as EPR Links

Each `ContentRecommendation` resolves to an EPR reference. Cards use `<app-epr-link>` for context-aware resolution:

- If prerequisite content is a step in the current path, navigates in-path
- If in another path, cross-references
- Otherwise standalone resource view
- Three-pillar hover preview via `epr-popover` (stewardship, governance, knowledge context)
- Verified content addressing via CID fingerprints

The `RecommendationListComponent` wraps each EPR link card with adaptation context:

- `'prerequisite_gap'` — "Foundation for [concept you struggled with]"
- `'reinforcement'` — "Another angle on [concept]"
- `'struggled_with_concept'` — "Review this before retrying"

### Two Surfaces, Same Component

**Inline after quiz failure**: Embedded in quiz result view. Header: "Strengthen Your Foundations". Shows immediately after failed attempt, before retry/cooldown UI.

**Path overview panel**: Embedded in `PathOverviewComponent`. Persists until dismissed or gate passed. Positioned near locked gate sections.

Both consume `PathAdaptationService.getRecommendations$()` (observable already exists).

## A2O Scenario Coverage

**New scenario** in `path-adaptation.feature`: When a learner fails a mastery quiz, the system looks up prerequisite content from the content graph and shows EPR-linked recommendation cards — both inline and in the path overview. Dismissing or passing the gate clears them.

**Unwip existing scenarios**: Remove `@wip` from core mastery gate scenarios (`@mastery-unlock`, `@attestation-gate`) since the service logic is tested and we're wiring the full flow.

## Implementation Scope

| Change | Files | Type |
|--------|-------|------|
| Graph-aware recommendation generation | `path-adaptation.service.ts` | Service wiring |
| `maxGraphDepth` config | `path-adaptation.service.ts` | Config addition |
| `RecommendationListComponent` | New component | Presentational |
| Quiz result integration | Quiz result template | Template |
| Path overview integration | `path-overview.component.ts/html` | Template |
| Service tests | `path-adaptation.service.spec.ts` | Tests |
| Component tests | `recommendation-list.component.spec.ts` | Tests |
| A2O scenarios | `path-adaptation.feature` | Scenarios |

## Not In Scope

- Deep graph traversal (future ElohimAgent work, uses reach + collective primitives)
- Discovery-to-path recommendation pipeline (separate CLAUDE-PICKS item)
- `RelationshipService` depth > 1 implementation
- Fog-of-war UX clarity (gate reason display — separate concern)
