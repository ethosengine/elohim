# B18c Follow-up: Cross-Pillar Dependency Resolution

**Status:** MVP cross-import approach used — lamad bundle builds via tsconfig path
aliases pointing directly at elohim-app source tree.

**Impact:** lamad bundle bundles elohim-app source code transitively. This means:
- lamad's bundle size includes elohim pillar services/models
- Breaking changes in elohim-app WILL break the lamad bundle build
- The dependency direction is still tightly coupled

This is intentional for MVP. The proper resolution requires Tasks B1–B8.

## Dispositions Applied

### COPY (done)
- `SeoService` → `app/lamad/src/app/shared/services/seo.service.ts`
- `src/environments/environment.ts` → local lamad environment file

### CROSS-IMPORT via tsconfig alias (MVP shortcut — needs follow-up)
All symbols below are resolved via `@app/elohim/*`, `@app/imagodei/*`,
`@app/qahal/*`, `@app/shefa/*` path aliases pointing to elohim-app source.

**EXTRACT** (highest priority — spec Tasks B1/B2/B6/B8):
- `DataLoaderService` (35 sites) → elohim-core Loader primitive (Task B1)
- `AgentService` (19 sites) → elohim-core identity bedrock
- `MasteryLevel`, `MasteryTier`, `AgentProgress`, mastery utils (21 sites) → elohim-core types
- `ReachLevel`, `REACH_LEVEL_VALUES`, `reachEncompasses`, `GeographicContext` (10 sites) → elohim-core
- `EprResolverService`, `StepRef`, `CrossPathMatch` (7 sites) → elohim-core (Task B1 Loader)
- `LEARNER_BACKEND` InjectionToken (7 sites) → elohim-core
- `LocalSourceChainService` (6 sites) → elohim-core
- `StorageClientService` (5 sites) → elohim-core
- `ContentBackendService` (5 sites) → elohim-core (moves with DataLoaderService)
- `AffinityTrackingService` (10 sites) → HTTP-API or elohim-core
- `GovernanceSignalService` (6 sites) → HTTP-API
- `SessionHumanService` (4 sites) → elohim-core Session (Task B2)
- `IdentityService` (2 sites) → elohim-core identity bedrock
- `identityGuard` (lamad.routes.ts) → elohim-core guard

**STAY-CROSS-PILLAR via custom element** (needs Tasks B6, B8, B20):
- `ElohimNavigatorComponent` (1 site: lamad-layout) → future `<elohim-page-chrome>`
- `EprRelationshipsPanelComponent` (2 sites) → future `<elohim-epr-relationships-panel>`
- `EprLinkComponent` (1 site) → future `<elohim-epr-link>`
- `EprPopoverComponent` (1 site) → future `<elohim-epr-popover>`
- `FeedbackMechanismGatewayComponent` (2 sites) → future `<qahal-feedback-gateway>`
- `GraduatedFeedbackComponent`, `ReactionBarComponent` (1 site each) → future qahal elements

**DUPLICATE** (deferred — low priority):
- `ContentAnalyticsComponent` → copy into `app/lamad/src/app/shared/`
- `GateFeedbackTriggerComponent` → copy into `app/lamad/src/app/shared/`
- `CategoryAffinityStats` model → copy into lamad models
- `ResumePoint`, `PathsOverview`, `TimelineEvent` (imagodei profile models) → copy

**HTTP-API** (deferred — needs HTTP call refactor per call site):
- `EventService` (shefa, 2 sites) → POST /api/v1/economic-event
- `AttentionTrackerService` (shefa, 1 site) → POST /api/v1/attention
- `StorageApiService` (3 sites) → `@elohim/storage-client` SDK direct
- `GovernanceService` (2 sites) → POST /api/v1/governance/signal
- `RecordFeedbackParams` (shefa barrel) → inline type or duplicate

## Schema Drift Fixed

`content-icons.ts` was missing `'element-registry'` (ContentType) and
`'element-registry-manifest'` (ContentFormat) that were added to schema-enums
but not propagated. Added in B18b to unblock the build.

## Codegen Script TODO

`app/lamad/src/app/generated/content-node-types.ts` imports `ContentView` via
a relative path that was valid in elohim-app but is wrong in the new location.
Fixed manually in B18b with `@app/generated/content-view`.
**Fix the codegen script** (`elohim/sdk/domains/lamad/scripts/codegen.mjs`) to
emit `@app/generated/content-view` instead of `../../generated/content-view`.
