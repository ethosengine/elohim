---
status: Draft
cites:
  - ../specs/2026-05-25-pillar-epr-decomposition-design.md   # the design spec this plan implements
---

# Lamad Pillar Dependency Audit (for bundle split)

**Generated:** 2026-05-25
**Author:** Task B0 (pillar-EPR-decomposition plan)
**Spec:** `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` §11.4
**Purpose:** Identify cross-pillar imports BEFORE splitting lamad into its own bundle at `app/lamad/` (Tasks B17–B18).
**Scope:** read-only analysis; no code changes in this task.

## Summary

**Lamad pillar file inventory** (`app/elohim-app/src/app/lamad/`):

| Category | Count |
|---|---|
| Components (`.component.ts`) | 25 |
| Services (`.service.ts`) | 37 |
| Models (`.ts` under `models/`) | 26 |
| Total TS files (non-spec) | 182 |
| Total `.spec.ts` files | 98 |

**Cross-pillar import totals** (non-spec, non-markdown):

| Source | Imports | Unique import paths |
|---|---|---|
| `@app/elohim/...` (60-service pillar) | **127** | 46 distinct subpaths |
| `@app/imagodei/...` | 11 | 7 distinct subpaths |
| `@app/qahal/...` | 5 | 4 distinct subpaths |
| `@app/shefa/...` | 5 | 5 distinct subpaths |
| `@app/generated/...` | 3 | 2 distinct subpaths |
| `../../../services/` (root `app/services/`) | 6 | 1 symbol (`SeoService`) |
| `../../../environments/environment` | 2 | env var bag |
| `../../../models/...` (root) | none (all model imports stay within lamad) | – |

**Total cross-pillar imports needing disposition:** ~159 lines across ~50 files.

**Unique imported symbols requiring decisions:** ~50 distinct symbols (see §"Disposition decisions" below).

## Dependencies on @app/elohim pillar

Grouped by import path, ordered by frequency:

| Import path | Hits | Representative symbols |
|---|---|---|
| `@app/elohim/services/data-loader.service` | 35 | `DataLoaderService` |
| `@app/elohim/models/agent.model` | 21 | `MasteryLevel`, `MasteryTier`, `AgentProgress`, `compareMasteryLevels`, `isAboveGate`, `ATTESTATION_GATE_LEVEL`, `MASTERY_LEVEL_VALUES` |
| `@app/elohim/services/agent.service` | 19 | `AgentService` |
| `@app/elohim/services/affinity-tracking.service` | 10 | `AffinityTrackingService` |
| `@app/elohim/models/protocol-core.model` | 10 | `ReachLevel`, `REACH_LEVEL_VALUES`, `reachEncompasses`, `GeographicContext` |
| `@app/elohim/services/epr-resolver.service` | 7 | `EprResolverService`, `StepRef`, `CrossPathMatch` |
| `@app/elohim/interfaces` | 7 | `LEARNER_BACKEND`, `governance.interface`, `content-attestation.interface` |
| `@app/elohim/services/storage-api.service` | 6 | `StorageApiService` |
| `@app/elohim/services/local-source-chain.service` | 6 | `LocalSourceChainService` |
| `@app/elohim/services/governance-signal.service` | 6 | `GovernanceSignalService` |
| `@app/elohim/models/zome-wire-types` | 6 | `ContentMasteryWire`, `parsePointsByTrigger`, `parseImpactByContent` (+ practice + learning-points types) |
| `@app/elohim/adapters/storage-types.adapter` | 6 | `EconomicEventView`, `ContentMasteryView`, `RelationshipView` |
| `@app/elohim/services/storage-client.service` | 5 | `StorageClientService` |
| `@app/elohim/services/content.service` | 5 | `ContentService` |
| `@app/elohim/services/trust-badge.service` | 4 | `TrustBadgeService` |
| `@app/elohim/services/profile.service` | 4 | `ProfileService` |
| `@app/elohim/providers/elohim-client.provider` | 4 | (spec-only; non-spec count: 0) |
| `@app/elohim` (barrel) | 4 | `ContextAssemblyService`, `ContentBirthContext`, `ContextAssemblyResult`, `CreatePayload` |
| `@app/elohim/services/holochain-client.service` | 3 | `HolochainClientService` |
| `@app/elohim/services/elohim-presence.service` | 3 | `ElohimPresenceService` |
| `@app/elohim/models/trust-badge.model` | 3 | `TrustLevel`, `TrustBadge`, `calculateTrustLevel` |
| `@app/elohim/models/human-consent.model` | 3 | `IntimacyLevel`, `hasMinimumIntimacy` |
| `@app/elohim/components/epr-relationships-panel/...` | 3 | `EprRelationshipsPanelComponent` |
| `@app/elohim/utils` | 2 | `generateExtensionId`, `generateMapId` |
| `@app/elohim/services/human-consent.service` | 2 | `HumanConsentService` |
| `@app/elohim/services/governance.service` | 2 | `GovernanceService` |
| `@app/elohim/services/elohim-agent.service` | 2 | `ElohimAgentService` |
| `@app/elohim/services` (barrel) | 2 | (spec-only; non-spec count: 0; appears in docs) |
| `@app/elohim/models/source-chain.model` | 2 | `MasteryRecordContent`, `SourceChainEntry`, `PathNegotiationContent` |
| `@app/elohim/models/json-ld.model` | 2 | `JsonLdMetadata` |
| `@app/elohim/models/epr-head.model` | 2 | `EprHead`, `EprRelationship` |
| `@app/elohim/integrity` | 2 | `BlobMetadataAnchor` |
| `@app/elohim/components/gate-feedback` | 2 | `GateFeedbackTriggerComponent` |
| `@app/elohim/utils/epr-ref` | 1 | `parseEpr` |
| `@app/elohim/services/lens-registry.service` | 1 | `LensRegistryService` |
| `@app/elohim/models/open-graph.model` | 1 | `OpenGraphMetadata` |
| `@app/elohim/models/elohim-presence.model` | 1 | `ElohimPresenceMoment` |
| `@app/elohim/models` (barrel) | 1 | `REAAction`, `LamadEventType` |
| `@app/elohim/components/epr-popover/epr-popover.component` | 1 | `EprPopoverComponent` |
| `@app/elohim/components/epr-link/epr-link.component` | 1 | `EprLinkComponent` |
| `@app/elohim/components/elohim-navigator/elohim-navigator.component` | 1 | `ElohimNavigatorComponent` |
| `@app/elohim/components/content-analytics/content-analytics.component` | 1 | `ContentAnalyticsComponent` |

Plus relative-path equivalents (`from '../../elohim/services/...'`): 3 hits — `DoorwayClientService` (×2: blob-streaming, blob-verification) + `StorageClientService` (×1: blob-manager). These are semantically identical to `@app/elohim/services/*` imports — they exist because `blob-*` services pre-date barrel adoption.

## Dependencies on other pillars

### @app/imagodei (11 hits, 7 paths)

| Symbol | File(s) |
|---|---|
| `identityGuard` | `lamad.routes.ts` |
| `IdentityService`, `isNetworkMode` | `components/lamad-home`, `components/profile-page` |
| `SessionHumanService` | `services/content-mastery.service`, `services/assessment.service`, `services/mastery-stats.service`, `components/profile-page` |
| `Attestation` | `quiz-engine/services/discovery-attestation.service` |
| `ResumePoint`, `PathsOverview`, `TimelineEvent` | `components/profile-page` |
| `isNetworkMode` (from `models/identity.model`) | `components/profile-page` |

### @app/qahal (5 hits, 4 paths)

| Symbol | File(s) |
|---|---|
| `FeedbackMechanismGatewayComponent` | `lesson-view`, `content-viewer` |
| `GraduatedFeedbackComponent` (and friends) | `content-viewer` |
| `ReactionBarComponent` | `content-viewer` |
| `CategoryAffinityStats` | `meaning-map` |

### @app/shefa (5 hits, 5 paths)

| Symbol | File(s) |
|---|---|
| `EventService` | `services/lamad-event.service`, `components/attention-flow` |
| `AttentionTrackerService` | `components/content-viewer` |
| `RecordFeedbackParams` (et al, barrel) | `services/signal-harness.service` |
| `ExplorerBreadcrumb`, `ExplorerNode`, `LensProvider` | `services/scope-sequence-lens.provider` |

### @app/avodah, @app/account, @app/doorway

None. Lamad has zero direct dependencies on these pillars.

## Dependencies on shared / cross-cutting code

| Path | Symbol | Used by |
|---|---|---|
| `../../../services/seo.service` (= `app/services/seo.service`) | `SeoService` | 6 components: `not-found`, `content-viewer`, `path-overview`, `path-navigator`, plus `lamad-home`-area variants |
| `../../../environments/environment` | `environment` config bag | `blob-cache-tiers.service`, `wasm-cache.service` |
| `@app/generated/schema-enums` | `ContentFormat`, `ContentType` enums | `models/content-node.model` |
| `@app/generated/distribution-summary` | `DistributionSummary` type | `models/content-node.model` |

The root `app/services/` directory holds five cross-cutting Angular utility services (analytics, config, dom-interaction, seo, theme). Only `seo.service` is used by lamad.

## Disposition decisions per imported symbol

Notation:
- **EXTRACT** → move to `app/elohim-elements/elohim-core/` (the cross-cutting library every bundle imports per spec §4).
- **DUPLICATE** → pillar-internal copy in `app/lamad/`, accept small drift.
- **HTTP-API** → drop the direct import; the lamad bundle calls doorway over HTTP (storage-client SDK already handles the wire surface).
- **CORE-ALREADY** → no-op for lamad split; symbol already belongs in elohim-core conceptually, just confirm import path post-extract.
- **DEAD** → import is unused or only in markdown docs; remove before split.
- **STAY-CROSS-PILLAR** → another pillar legitimately provides this; the lamad bundle resolves it via an Angular wrapper around an `<elohim-*>` element (per spec §4.3 / §B20).

### elohim pillar symbols

| Symbol / path | Disposition | Rationale |
|---|---|---|
| `DataLoaderService` (`@app/elohim/services/data-loader.service`) — **35 hits** | **EXTRACT** | Per spec §4.1, the Loader primitive is an elohim-core deliverable (Task B1). DataLoaderService is the existing Angular incarnation; it must move into elohim-core (or its Angular wrapper layer) so every bundle uses one. |
| `AgentService` (`@app/elohim/services/agent.service`) — 19 hits | **EXTRACT** | Identity/agent primitive — spec §1 says imagodei is "doubled" (bedrock in elohim-core, pillar for enrollment). AgentService is the bedrock surface used by every pillar; belongs in elohim-core. |
| `MasteryLevel`, `MasteryTier`, `AgentProgress`, `compareMasteryLevels`, `isAboveGate`, `ATTESTATION_GATE_LEVEL`, `MASTERY_LEVEL_VALUES` (`@app/elohim/models/agent.model`) — 21 hits | **CORE-ALREADY** (model) → **EXTRACT** to `elohim-core/models/` | These are SDK-shape types that match Rust wire enums. They are pure data — extract to elohim-core types package. Pure types have no runtime cost. |
| `ReachLevel`, `REACH_LEVEL_VALUES`, `reachEncompasses`, `GeographicContext` (`@app/elohim/models/protocol-core.model`) — 10 hits | **EXTRACT** | Protocol-core model — by name and substance, belongs in elohim-core. |
| `AffinityTrackingService` (`@app/elohim/services/affinity-tracking.service`) — 10 hits | **HTTP-API** | Affinity is a write-side signal recorded against the substrate. The lamad bundle should call the storage API directly (or a thin client) rather than carry the service class. |
| `EprResolverService`, `StepRef`, `CrossPathMatch` (`@app/elohim/services/epr-resolver.service`) — 7 hits | **EXTRACT** | EPR resolution is the spec's central new primitive (§4.1 Loader + §5 scenarios). Must be elohim-core. |
| `LEARNER_BACKEND` (`@app/elohim/interfaces`) — 7 hits | **EXTRACT** | An InjectionToken pointing at the learner backend. Bundle-agnostic plumbing — elohim-core provides the token, each bundle binds it at boot. |
| `StorageApiService` (`@app/elohim/services/storage-api.service`) — 6 hits | **HTTP-API** | Already a thin HTTP wrapper. Replace with `@elohim/storage-client` SDK usage inside the lamad bundle. (Or extract to elohim-core if we want a single Angular wrapper everywhere — leaning HTTP-API for cleaner bundle independence.) |
| `LocalSourceChainService` (`@app/elohim/services/local-source-chain.service`) — 6 hits | **EXTRACT** | Local source-chain mirror is the Holochain-source-of-truth cache; needed by every pillar (mastery, feedback, attestations). Belongs in elohim-core. |
| `GovernanceSignalService` (`@app/elohim/services/governance-signal.service`) — 6 hits | **HTTP-API** | Governance signals are a substrate write — HTTP call to doorway suffices in the bundle. |
| `ContentMasteryWire`, `parsePointsByTrigger`, `parseImpactByContent` and zome-wire types (`@app/elohim/models/zome-wire-types`) — 6 hits | **EXTRACT** | Wire types are SDK-boundary truth. Belong in elohim-core (or in `@elohim/storage-client` if already generated there — check codegen overlap). |
| `EconomicEventView`, `ContentMasteryView`, `RelationshipView` (`@app/elohim/adapters/storage-types.adapter`) — 6 hits | **HTTP-API** (use `@elohim/storage-client` directly) | These are re-exports of generated SDK types; lamad bundle can import from `@elohim/storage-client` directly, eliminating the adapter dependency. |
| `StorageClientService` (`@app/elohim/services/storage-client.service`) — 5 hits | **EXTRACT** | Storage client wrapper — bedrock; bundle-agnostic. |
| `ContentService` (`@app/elohim/services/content.service`) — 5 hits | **EXTRACT** | Wraps DataLoader for content fetches; if DataLoader moves to core, ContentService moves with it. Alternative: **DUPLICATE** — content fetch shape may differ per pillar. Recommend EXTRACT for MVP, revisit if drift emerges. |
| `TrustBadgeService` (`@app/elohim/services/trust-badge.service`) — 4 hits | **EXTRACT** | Trust is a cross-pillar primitive; spec §1 puts trust at the bedrock layer. |
| `ProfileService` (`@app/elohim/services/profile.service`) — 4 hits | **EXTRACT** | Identity-adjacent; pairs with AgentService. Bedrock. |
| `ContextAssemblyService`, `ContentBirthContext`, `ContextAssemblyResult`, `CreatePayload` (`@app/elohim` barrel) — 4 hits | **EXTRACT** | Content-creation context assembly — needed by any pillar that authors content. Bedrock. |
| `HolochainClientService` (`@app/elohim/services/holochain-client.service`) — 3 hits | **EXTRACT** | Conductor client — bedrock. Used by Tauri-mode for direct conductor calls. |
| `ElohimPresenceService`, `ElohimPresenceMoment` (`@app/elohim/services/elohim-presence.service` + model) — 4 hits | **EXTRACT** | Elohim presence (per-user agent presence) is bedrock identity surface. |
| `TrustLevel`, `TrustBadge`, `calculateTrustLevel` (`@app/elohim/models/trust-badge.model`) — 3 hits | **EXTRACT** | Pairs with TrustBadgeService. Pure types + helpers. |
| `IntimacyLevel`, `hasMinimumIntimacy` (`@app/elohim/models/human-consent.model`) — 3 hits | **EXTRACT** | Consent primitives — bedrock. |
| `HumanConsentService` (`@app/elohim/services/human-consent.service`) — 2 hits | **EXTRACT** | Pairs with the model above. |
| `EprRelationshipsPanelComponent` (`@app/elohim/components/epr-relationships-panel/...`) — 3 hits | **STAY-CROSS-PILLAR** via Angular wrapper around `<elohim-epr-relationships-panel>` | Per spec §4.3 + Task B20: the EPR-link/related primitives ship as Lit elements from elohim-core. The Angular components currently in the elohim pillar become wrappers — lamad uses the wrapper or the element directly. |
| `EprPopoverComponent`, `EprLinkComponent` (epr-popover, epr-link components) — 2 hits | **STAY-CROSS-PILLAR** via Angular wrapper around `<elohim-epr-link>` | Same as above. Spec Task B8 + B20. |
| `ElohimNavigatorComponent` (`@app/elohim/components/elohim-navigator/...`) — 1 hit | **STAY-CROSS-PILLAR** via `<elohim-page-chrome>` | Per spec §4.1 + Task B6: page chrome is an elohim-core element. Lamad consumes it as a custom element. |
| `ContentAnalyticsComponent` (`@app/elohim/components/content-analytics/...`) — 1 hit | **DUPLICATE** | One-off render component; pillar-internal copy keeps bundle independence. Low extraction cost; low duplication cost. |
| `GovernanceService`, `GovernanceActionApiService` (`@app/elohim/services/governance.service`) — 2 hits | **HTTP-API** | Governance is a substrate-side concern; bundle calls doorway. |
| `GateFeedbackTriggerComponent` (`@app/elohim/components/gate-feedback`) — 2 hits | **DUPLICATE** (interim) | Feedback gate UI is currently coupled to a specific governance vocabulary. Duplicate for MVP; converge to a `<elohim-gate-feedback>` element in a later sprint. |
| `ElohimAgentService` (`@app/elohim/services/elohim-agent.service`) — 2 hits | **EXTRACT** | Elohim-agent (per-user elohim) is bedrock. |
| `MasteryRecordContent`, `SourceChainEntry`, `PathNegotiationContent` (`@app/elohim/models/source-chain.model`) — 2 hits | **EXTRACT** | Source-chain wire types — bedrock with `LocalSourceChainService`. |
| `JsonLdMetadata` (`@app/elohim/models/json-ld.model`) — 2 hits | **EXTRACT** | Pure data shape. |
| `OpenGraphMetadata` (`@app/elohim/models/open-graph.model`) — 1 hit | **EXTRACT** | Pure data shape. |
| `EprHead`, `EprRelationship` (`@app/elohim/models/epr-head.model`) — 2 hits | **EXTRACT** | EPR primitive types — central to the new architecture. Bedrock. |
| `BlobMetadataAnchor` (`@app/elohim/integrity`) — 2 hits | **EXTRACT** | Integrity types; bedrock. |
| `generateExtensionId`, `generateMapId` (`@app/elohim/utils`) — 2 hits | **EXTRACT** | Pure ID helpers. |
| `parseEpr` (`@app/elohim/utils/epr-ref`) — 1 hit | **EXTRACT** | EPR reference parsing — pairs with EPR types. |
| `LensRegistryService` (`@app/elohim/services/lens-registry.service`) — 1 hit | **EXTRACT** | Spec §1 introduces "lens" (pillar projection of a primitive). Bedrock. |
| `REAAction`, `LamadEventType` (`@app/elohim/models` barrel) — 1 hit | Split: `REAAction` **EXTRACT**, `LamadEventType` **DUPLICATE** (lamad-internal vocabulary) | REAAction is REA-canon; LamadEventType is lamad-vocabulary that may not belong outside lamad. |
| `DoorwayClientService` (relative `../../elohim/services/doorway-client.service`) — 2 hits | **EXTRACT** | Doorway HTTP client wrapper — bedrock. |
| `@app/elohim/providers/elohim-client.provider` — 4 hits | **DEAD** (all spec/markdown only — 0 non-spec hits) | Appears only in `.md` docs; remove from audit scope. |
| `@app/elohim/services` (barrel, `GovernanceService`) — 2 hits | **DEAD** (markdown only) | `LAMAD_API_SPECIFICATION_v1.0.md:1524` and `claude.md:82`. No code dependency. |
| `@app/elohim/interfaces/governance.interface` — 1 hit | **HTTP-API** | Interface for governance calls; replace with HTTP call. |
| `@app/elohim/interfaces/content-attestation.interface` — 1 hit | **EXTRACT** | Content-attestation contract — substrate-adjacent type. |
| `@app/elohim/services/indexeddb-cache.service` — 1 hit | **EXTRACT** | Cross-pillar local cache primitive. |
| `@app/elohim/services/content-resolver.service` — 1 hit | **EXTRACT** | Sibling of EprResolverService. |
| `@app/elohim/services/projection-api.service` — 1 hit | **HTTP-API** | Per Task B16, /api/v1/epr/{id} replaces direct projection-api calls in the bundle. |

### imagodei pillar symbols

Per spec §1: "Imagodei is doubled — bedrock in elohim-core (session/identity/capability), pillar for enrollment/recovery/canonical-self-view."

| Symbol | Disposition | Rationale |
|---|---|---|
| `identityGuard` (`@app/imagodei/guards/identity.guard`) | **EXTRACT** | Route guard for any pillar that gates by identity. Bedrock. |
| `IdentityService`, `isNetworkMode` (`@app/imagodei/services/identity.service`, `@app/imagodei/models/identity.model`) | **EXTRACT** | Identity service is the bedrock half of imagodei per spec §1. Used by 4+ lamad files. |
| `SessionHumanService` (`@app/imagodei/services/session-human.service`) | **EXTRACT** to elohim-core Session primitive (Task B2) | Per spec §4.1 Task B2: Session is an explicit elohim-core deliverable. SessionHumanService is the existing Angular incarnation; B2 should subsume it. |
| `Attestation` (`@app/imagodei/models/attestations.model`) | **EXTRACT** | Pure data type; bedrock. |
| `ResumePoint`, `PathsOverview`, `TimelineEvent` (`@app/imagodei/models/profile.model`) | **DUPLICATE** in lamad | Profile-shape projections specific to how lamad renders the profile page; clone shape into a lamad-owned model to avoid coupling. (Alternative: leave them in imagodei and import via HTTP if profile-page moves to imagodei pillar bundle — but currently it's lamad-rendered.) |

### qahal pillar symbols (cross-pillar UI primitives lamad embeds)

| Symbol | Disposition | Rationale |
|---|---|---|
| `FeedbackMechanismGatewayComponent` (2×) | **STAY-CROSS-PILLAR** via custom element | Qahal owns the feedback-gateway UX; lamad embeds. Expose as `<qahal-feedback-gateway>` so lamad bundle consumes via DOM contract, not Angular import. (Until then, MVP can DUPLICATE the small wrapper into lamad — but the lamad bundle WILL need qahal scripts loaded, so a properly-extracted element is the durable answer.) |
| `GraduatedFeedbackComponent` (et al), `ReactionBarComponent` | **STAY-CROSS-PILLAR** via custom element | Same pattern. These are qahal-vocabulary widgets; lamad shouldn't own them. |
| `CategoryAffinityStats` (`@app/qahal/models/human-affinity.model`) | **DUPLICATE** in lamad | Pure data shape; cheap to clone, expensive to extract a model package. |

### shefa pillar symbols (REA economy / event surface)

| Symbol | Disposition | Rationale |
|---|---|---|
| `EventService` (`@app/shefa/services/event.service`) — 2× | **HTTP-API** | EconomicEvent recording is a substrate write; lamad bundle calls doorway directly. |
| `AttentionTrackerService` (`@app/shefa/services/attention-tracker.service`) | **HTTP-API** | Attention tracking emits substrate signals; HTTP-call from bundle. |
| `RecordFeedbackParams` (`@app/shefa` barrel) | **EXTRACT** or **DUPLICATE** | Pure type; cheap either way. Lean DUPLICATE because shefa is its own pillar. |
| `ExplorerBreadcrumb`, `ExplorerNode`, `LensProvider` (`@app/shefa/models`) | **DUPLICATE** in lamad | Lens-provider contract for scope-sequence; lamad implements a shefa-defined interface. Either DUPLICATE the type (3 small interfaces) or extract LensProvider type to elohim-core's lens scaffolding (it's already a §1 primitive). Recommend **EXTRACT** of `LensProvider` only; DUPLICATE the explorer-node DTOs. |

### Root cross-cutting (`app/services/`, `app/generated/`, `app/environments/`)

| Symbol | Disposition | Rationale |
|---|---|---|
| `SeoService` (`app/services/seo.service`) — 6 hits | **EXTRACT** to elohim-core or **DUPLICATE** | SEO meta-tag setter is bundle-utility, not pillar-vocabulary. Recommend EXTRACT to a shared utility surface (elohim-core or a tiny `@elohim/web-utils` package). |
| `@app/generated/schema-enums`, `@app/generated/distribution-summary` | **CORE-ALREADY** (path change) | These are codegen output; same generator should emit into elohim-core's generated dir, or both bundles point to the same shared generated package. |
| `environment` (`app/environments/environment`) | **DUPLICATE** | Each bundle ships its own environment file. Standard Angular practice. |

## Risk assessment

Counting from the disposition table above:

| Tier | Count | Examples |
|---|---|---|
| **HIGH-RISK** (likely to break things if done wrong) | ~6 | DataLoaderService, AgentService, EprResolverService, SessionHumanService, LocalSourceChainService, StorageClientService — these have 5+ call sites each, touch reactive state, and are easy to subtly fork during extraction |
| **MEDIUM-RISK** (mechanical with side effects) | ~20 | The remaining EXTRACT-tagged services & cross-pillar UI primitives that need element-wrapper extraction (Tasks B6, B8, B20) |
| **LOW-RISK** (simple search/replace or pure types) | ~25 | All pure-type EXTRACTs (MasteryLevel, ReachLevel, EprHead, JsonLdMetadata, etc.); barrel re-exports; the DEAD removals |
| **DEAD code to remove pre-split** | 3 entries | `@app/elohim/providers/elohim-client.provider` (markdown only), `@app/elohim/services` barrel in `LAMAD_API_SPECIFICATION_v1.0.md` and `claude.md` (docs only) |

## Open items for the operator

Per the overnight directive, I made these calls myself; flagging only TRULY contested choices:

1. **DataLoaderService disposition** — I called EXTRACT, but it could equally be HTTP-API (replace DataLoader's role with direct storage-client SDK calls). EXTRACT preserves the current shape, HTTP-API forces cleanup. With 35 call sites, EXTRACT is the lower-risk MVP move and matches spec §4.1 Loader (which is conceptually the same surface, just bedrock).

2. **ContentService vs ContentService** — lamad imports `@app/elohim/services/content.service` but ALSO has its own `app/lamad/services/content.service`. Two different services with the same name. The lamad-side one is the canonical lamad content service; the elohim-side one is a more primitive content fetcher. Naming collision should be resolved during B18 (rename elohim's to `ContentBackendService` or similar; otherwise the move will produce confusing imports).

3. **DuplicateOrExtract for `Authorization` / GovernanceService** — currently tagged HTTP-API because governance writes are substrate operations. If a UI surface needs to RENDER governance state often, an extracted client may be ergonomic. Defer to operator; HTTP-API is safe for MVP.

4. **Adapter dependency direction** — `@app/elohim/adapters/storage-types.adapter` re-exports `@elohim/storage-client` types. The adapter exists per the boundary rule ("adapters add computed fields only, never transform"). When lamad moves to its own bundle, the right move is to import directly from `@elohim/storage-client` and drop the adapter dependency — but if the adapter DOES add computed fields used by lamad, those computations need to move too. **Verify during B18:** does `storage-types.adapter` add any methods/fields beyond passthrough?

## Recommended split sequence

This minimizes intermediate broken states for Task B18:

### Phase 1 — Pre-split cleanup (no bundle change yet)

1. **Remove DEAD imports** from `lamad/claude.md`, `lamad/LAMAD_API_SPECIFICATION_v1.0.md` (markdown only; trivial).
2. **Resolve `ContentService` name collision** — rename `app/elohim/services/content.service.ts` → `content-backend.service.ts` (or merge into DataLoaderService). Single repo-wide PR.
3. **Verify storage-types.adapter passthrough** — if any lamad import actually relies on computed adapter fields, document them; if pure passthrough, switch lamad imports to `@elohim/storage-client` directly.

### Phase 2 — elohim-core extraction (per Task B1–B8 + B20)

Order extracts so the highest-fanout, lowest-risk types move first:

4. **Pure types & utils first** (LOW-RISK; ~30 import sites updated): MasteryLevel, ReachLevel, JsonLdMetadata, OpenGraphMetadata, TrustBadge types, IntimacyLevel, source-chain wire types, EprHead/EprRelationship, BlobMetadataAnchor, generateExtensionId, generateMapId, parseEpr. Each is a "move file + update imports" change.
5. **Bedrock services** (HIGH-RISK; coordinate carefully): LocalSourceChainService, StorageClientService, HolochainClientService, DoorwayClientService — the I/O layer.
6. **Identity bedrock**: AgentService, ProfileService, IdentityService, identityGuard, ElohimAgentService, ElohimPresenceService.
7. **Spec-driven primitives** (Tasks B1, B2): Loader (subsuming DataLoaderService + ContentResolverService + EprResolverService + ContentService), Session (subsuming SessionHumanService).
8. **Element extracts** (Tasks B6, B8, B20): `<elohim-page-chrome>`, `<elohim-epr-link>`, `<elohim-context-menu>`, and Angular wrappers around them. After this, the four elohim-pillar Angular UI components (EprLink, EprPopover, EprRelationshipsPanel, ElohimNavigator) become thin shells over the elements.

### Phase 3 — Bundle move (Task B17 + B18)

9. **Scaffold `app/lamad/`** Angular workspace (Task B17). Bundle imports `@elohim/elohim-core`, `@elohim/storage-client`, declares HTTP base for everything else.
10. **Lift-and-shift lamad files** to `app/lamad/src/app/`. Path aliases inside lamad stay relative; cross-pillar imports become either elohim-core imports, custom-element consumption, or HTTP calls per the disposition table.
11. **DUPLICATE the small per-pillar shims**: ContentAnalyticsComponent, GateFeedbackTriggerComponent, CategoryAffinityStats, ExplorerBreadcrumb/Node, ResumePoint/PathsOverview/TimelineEvent. Lamad-internal copies; accept drift.
12. **HTTP-ify the write-side services**: AffinityTrackingService → POST /api/v1/affinity, GovernanceSignalService → POST /api/v1/governance/signal, EventService → POST /api/v1/economic-event, AttentionTrackerService → POST /api/v1/attention, etc. Use `@elohim/storage-client` SDK if it already covers these endpoints.

### Phase 4 — Verification (Task B19+)

13. Remove `/lamad/**` routes from elohim-app (Task B19).
14. Run app vitests for both bundles; both should be green.
15. Cypress smoke against the lamad bundle served by doorway via projection (Tasks B22, B23 feature scenarios).

---

**Estimated B18 effort breakdown** (informational; for shift sizing):

- ~50 files to relocate + their .spec.ts pairs
- ~159 import lines to rewrite
- ~30 pure-type updates (mechanical)
- ~10 service-layer updates (semantic — needs review per service)
- ~10 element-wrapper or HTTP-call rewrites (semantic — needs review)
- 3 dead-code deletions

This is sizeable but not blocking. With the elohim-core extracts (Tasks B1–B8) landing first, B18 becomes mostly a path-rewrite exercise plus the HTTP-ification of the write-side services.
