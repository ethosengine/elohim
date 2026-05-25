# Cross-Pillar Cleanup — Wave 1 Disposition Manifest

> Generated as Wave 1 of plan
> `genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md`.
> Every cross-pillar import in `app/lamad/src` is classified into one of 8
> dispositions; Wave 2 slice agents execute against this manifest.

---

## Operator-input items (resolve before Wave 2 dispatch)

**Pre-resolved via operator "carry on":**
- `@elohim/identity` = **new library** (per `cradle-to-grave-capability-gradient.md` §4 elohim mediation roles)
- `@elohim/rea-runtime` = **new library** (per `rea-compute-commitment-primitive.md`)
- **Sequencing** = run in parallel with the dev-branch integration shake-out. Z.D Phase 1 substrate already landed on dev (commits `b2380b899`, `7f66391b6`, `bf2efd191`) so the new `@elohim/rea-runtime` library can absorb the REA compute-commitment primitive from day one.

**New decisions surfaced during classification:**

1. **`profile.service.ts` belongs in `@elohim/identity`, not `@elohim/service`.** It lives under `app/elohim-app/src/app/elohim/services/` today but its principal dependency is `@app/imagodei/models/profile.model`. The "elohim cross-cutting" placement is historical, not structural. Classified **I**; flagged for Slice 2.3.

2. **`elohim-presence.service.ts` has a reverse dependency on `@app/lamad`.** Lamad consumes it 3× — that's the import we're migrating. But the service itself imports `LearnerContextService` from `@app/lamad`. If it relocates to `@elohim/service` as-is, the library would depend on the lamad app (wrong direction). Slice 2.1 must invert the dependency: define a `LearnerContextProvider` interface in `@elohim/service` and have lamad register a concrete implementation. Classified **L** with the inversion call-out.

3. **`@app/elohim/models` bare-barrel import in `signal-harness.service.ts` resolves to REA symbols** (`REAAction`, `LamadEventType`). When the barrel moves, those two symbols belong in `@elohim/rea-runtime`'s public-api rather than `@elohim/service`'s — Slice 2.4 owns them. Single import line classified **R** with a split-barrel note for Slice 2.1 (must NOT re-export these from `@elohim/service`).

4. **EPR-popover follows EPR-link.** `EprPopoverComponent` is programmatically instantiated by `EprLinkComponent`; the two are tightly coupled. When lamad swaps to `<elohim-epr-link>` (E), the popover concern moves into the Lit element rather than continuing as a separate Angular component. Slice 2.6's E migration handles both as one unit.

5. **`@app/qahal/models/human-affinity.model` is a pure-type import** (HumanAffinity is a content-affinity wire shape, not UI). Despite living under qahal/, classified **L** (rehome to `@elohim/service`) — the qahal UI components that produce affinity are separately classified C, but the data type is cross-cutting.

6. **`integrity/` anchor types** (BlobMetadataAnchor, FederationRegistryAnchor, NodeRegistryAnchor — TypeScript contracts mirroring Holochain HDI integrity anchors) classified **S** rather than L. These are substrate wire-shapes, not Angular service helpers. Slice 2.5 carries them into `@elohim/storage-client`.

7. **Three intentionally-skipped cleanups** noted for Slice 2.5 (S) — `@app/generated/*` is currently mirrored at `app/lamad/src/generated/` by codegen (added in `48ad0f548`). Lamad's consumers should switch to the bundle-local `@/generated/*` form, not migrate to `@elohim/storage-client` — keeping codegen distribution local is the established pattern. Slice 2.5's actual work for the @app/generated/* slice is consumer-rewrite-only, not symbol-relocation.

No D (duplicate) classifications. No X (delete) classifications — every import resolves to live code.

---

## Per-disposition summary

| Code | Source modules | Import lines | Consumer files affected | Target |
| --- | --- | --- | --- | --- |
| **L** | 38 | ~190 | ~95 | `@elohim/service` |
| **C** | 7 | 11 | ~6 | `elohim-core` Lit library |
| **S** | 10 | 27 | ~18 | `@elohim/storage-client` + bundle-local `src/generated/` |
| **I** | 8 | 28 | ~14 | `@elohim/identity` (NEW) |
| **R** | 6 | 10 | ~5 | `@elohim/rea-runtime` (NEW) |
| **H** | 0 | 0 | 0 | — |
| **E** | 2 | 2 | 2 | `<elohim-epr-link>` Lit element |
| **D** | 0 | 0 | 0 | — |
| **X** | 0 | 0 | 0 | — |
| **Total** | **71** | **268** | **119** | — |

(Some imports overlap source modules — `268` reflects per-import-line classifications; `71` is per-row in the tables below where some rows split into multiple slices.)

---

## Count reconciliation vs B0 audit

| Source | Audit (2026-05-25 pre-integration) | This manifest (post-integration) | Delta |
| --- | --- | --- | --- |
| Total cross-pillar imports | 159 | 261 (raw `sort -u` lines) | +102 |
| `@app/elohim/*` | 127 | 216 | +89 |
| `@app/imagodei/*` | 11 | 24 | +13 |
| `@app/qahal/*` | 5 | 5 | 0 |
| `@app/shefa/*` | 5 | 9 | +4 |
| `@app/generated/*` | 3 | 7 | +4 |

**Cause:** Integration of `design/peer-oauth-portal` into dev (commits `a224c3e79`..`e18c4cb48`) and a number of follow-up fixes landed new lamad consumers that import elohim-pillar services for auth + EPR resolution flow. The plan's pre-sprint readiness checklist already noted updated counts (`~228 @app/elohim, ~28 @app/imagodei, ~11 @app/shefa, ~9 @app/qahal`); this manifest's enumeration matches the readiness checklist's order-of-magnitude. The delta is real and not an audit error — it's the working tree at sprint start.

**Implication for slice sizing:** Slices 2.1 (L) and 2.3 (I) are larger than the plan anticipated. The plan's "~80–120" for L and "~11" for I should be read as "~190 import lines / ~38 source modules" and "~28 import lines / ~8 source modules" respectively. The work shape is the same; the volume scaled with integration.

---

## Manifest — grouped by disposition

### L slice — `@elohim/service` (Slice 2.1, angular-architect)

**Target:** `app/elohim-library/projects/elohim-service/src/lib/<service-or-model>.ts` + add to `public-api.ts`.

| Source | Symbol family | Consumer count | Notes |
| --- | --- | --- | --- |
| `@app/elohim/services/data-loader.service` | `DataLoaderService` | 35 | HIGHEST priority — migrate first. Most-consumed service in lamad. |
| `@app/elohim/models/agent.model` | `Agent`, `MasteryTier`, `MasteryLevel`, `BLOOM_LEVEL_VALUES`, `getMasteryTier`, `isAboveGate`, `compareMasteryLevels`, `AgentAttestation`, `FrontierItem` | 21 | Has business-logic helpers (not pure wire) — L not S. |
| `@app/elohim/services/agent.service` | `AgentService` | 19 | Pairs with agent.model. |
| `@app/elohim/services/affinity-tracking.service` | `AffinityTrackingService` | 10 | |
| `@app/elohim/services/epr-resolver.service` | `EprResolverService` | 7 | Substrate-API service; foundational to Slice 2.6's E migrations. Migrate BEFORE Slice 2.6 starts. |
| `@app/elohim/interfaces` (barrel) | `IDataLoader`, `IStorageApi`, `IStorageWriter`, `IBlobFetcher`, `IEprResolver`, etc. | 7 | Barrel — re-export from `@elohim/service`'s public-api. |
| `@app/elohim/services/storage-client.service` | `StorageClientService` | 6 | |
| `@app/elohim/services/storage-api.service` | `StorageApiService` | 6 | |
| `@app/elohim/services/local-source-chain.service` | `LocalSourceChainService` | 6 | Holochain source chain client. |
| `@app/elohim/services/governance-signal.service` | `GovernanceSignalService` | 6 | |
| `@app/elohim/adapters/storage-types.adapter` | `toStorageType`, `fromStorageType`, etc. | 6 | Adapter functions. |
| `@app/elohim/services/content.service` | `ContentService` | 5 | |
| `@app/elohim/services/trust-badge.service` | `TrustBadgeService` | 4 | |
| `@app/elohim/providers/elohim-client.provider` | `ELOHIM_CLIENT`, `provideElohimClient`, related DI tokens | 4 | |
| `@app/elohim` (bare barrel) | `ContextAssemblyService`, `ContextAssemblyResult`, `ContentBirthContext`, `CreatePayload` | 4 | Barrel — these symbols belong in L. |
| `@app/elohim/services/holochain-client.service` | `HolochainClientService` | 3 | |
| `@app/elohim/services/elohim-presence.service` | `ElohimPresenceService` | 3 | **INVERSION REQUIRED.** This service imports `LearnerContextService` from `@app/lamad`. Slice 2.1 must define a `LearnerContextProvider` interface in `@elohim/service` and have lamad register the concrete implementation, OR leave this service in elohim-app and have lamad consume via H (doorway HTTP). **Default:** invert in @elohim/service. |
| `@app/elohim/services/doorway-client.service` | `DoorwayClientService` | 3 | |
| `@app/elohim/models/trust-badge.model` | `TrustBadge` | 3 | |
| `@app/elohim/models/json-ld.model` | `JsonLdNode`, etc. | 3 | |
| `@app/elohim/models/human-consent.model` | `HumanConsent`, `IntimacyLevel`, `ConsentState`, `ElevationRequest`, `hasMinimumIntimacy`, `isConsentActive`, `canElevate` | 3 | Has helper fns — L not S. |
| `@app/elohim/utils` (barrel) | `epr-codec`, `epr-ref`, `id-generator`, `access-control.helper` | 2 | |
| `@app/elohim/services/human-consent.service` | `HumanConsentService` | 2 | |
| `@app/elohim/services/governance.service` | `GovernanceService` | 2 | |
| `@app/elohim/services/elohim-agent.service` | `ElohimAgentService` | 2 | |
| `@app/elohim/models/open-graph.model` | `OpenGraphCard` etc. | 2 | UI-metadata; L. |
| `@app/elohim/utils/epr-ref` | `EprRef`, `parseEprRef`, etc. | 1 | |
| `@app/elohim/services/projection-api.service` | `ProjectionApiService` | 1 | |
| `@app/elohim/services/lens-registry.service` | `LensRegistryService` | 1 | |
| `@app/elohim/services/indexeddb-cache.service` | `IndexeddbCacheService` | 1 | |
| `@app/elohim/services/content-resolver.service` | `ContentResolverService` | 1 | |
| `@app/elohim/models/elohim-presence.model` | `ElohimPresence` etc. | 1 | |
| `@app/elohim/models` (bare barrel) | (only `REAAction`, `LamadEventType` — see #3 above) | 1 | **SPLIT.** Symbols go to R, NOT L. Slice 2.1 must NOT re-export these from `@elohim/service`. |
| `@app/elohim/interfaces/governance.interface` | `IGovernance` etc. | 1 | |
| `@app/elohim/interfaces/content-attestation.interface` | `IContentAttestation` | 1 | |
| `@app/qahal/models/human-affinity.model` | `HumanAffinity` | 1 | Pure type — rehome to `@elohim/service` cross-pillar models. |

**Migration order (within L):** start with `data-loader.service` (35 consumers) → `agent.model` (21) → `agent.service` (19) → `epr-resolver.service` (7, needed by Slice 2.6) → everything else. Author public-api re-exports incrementally so consumers can switch progressively.

**Slice 2.1 acceptance signal:** Zero `@app/elohim/*` imports remain in `app/lamad/` or `app/elohim-app/` source, except the inverted dep in `elohim-presence.service` (which becomes a registered provider).

---

### C slice — `elohim-core` Lit library (Slice 2.2, component-architect)

**Target:** `app/elohim-elements/elohim-core/src/elohim-<name>.ts` + Library A + Library B stories + consumers swap to custom-element usage.

| Source | Component | Consumer count | Notes |
| --- | --- | --- | --- |
| `@app/elohim/components/epr-relationships-panel/epr-relationships-panel.component` | `EprRelationshipsPanelComponent` | 3 | Cross-pillar relationship UI — distinct from EPR link/popover (E). |
| `@app/elohim/components/gate-feedback` (subdir) | `GateFeedbackTriggerComponent`, `GateFeedbackModalComponent` | 2 | |
| `@app/qahal` (barrel) | `FeedbackMechanismGatewayComponent` | 2 | Re-exported from qahal index — qahal community feedback UI gateway. |
| `@app/elohim/components/elohim-navigator/elohim-navigator.component` | `ElohimNavigatorComponent` | 1 | Cross-pillar navigation primitive. |
| `@app/elohim/components/content-analytics/content-analytics.component` | `ContentAnalyticsComponent` | 1 | Cross-pillar analytics surface. |
| `@app/qahal/components/reaction-bar/reaction-bar.component` | `ReactionBarComponent` | 1 | Per-plan C candidate. |
| `@app/qahal/components/graduated-feedback/graduated-feedback.component` | `GraduatedFeedbackComponent` | 1 | Per-plan C candidate. |

**Slice 2.2 acceptance signal:** Zero `@app/qahal/components/*` or `@app/elohim/components/*` (cross-pillar UI primitives) imports remain in lamad. Consumers use `<elohim-<name>>` custom elements (with `CUSTOM_ELEMENTS_SCHEMA` + thin Angular wrappers where the upgrade pattern requires it).

---

### S slice — `@elohim/storage-client` + bundle-local `generated/` (Slice 2.5, code-reviewer)

**Target:** Wire-format types into `elohim/sdk/storage-client-ts/src/`; generated artifacts stay distributed via codegen to `app/lamad/src/generated/` (already configured by `48ad0f548`).

| Source | Symbol family | Consumer count | Notes |
| --- | --- | --- | --- |
| `@app/elohim/models/protocol-core.model` | `ReachLevel`, `AffinityScope`, `IntimacyLevel`, `ConsentState`, `GovernanceLayer`, `GeographicLayer`, `AgentType`, `Attestation`, `ConstitutionalConstraint`, `TokenSpecification`, `CrossPillarLink`, `Pillar`, `PROTOCOL_VERSION` etc. | 9 | Constitutional substrate primitives mirroring Holochain entries. |
| `@app/elohim/models/zome-wire-types` | `ContentMasteryWire`, `PracticePool`, `MasteryChallenge`, `LamadContributorRecognitionWire`, `parseContentMix`, `transformZomeResponse` etc. | 5 | Direct zome wire format. |
| `@app/elohim/models/epr-head.model` | `EprHead`, `EprLamadContext`, `EprShefaContext`, `EprQahalContext`, `EprRelationship`, `IpldLink`, `cidToLink`, `linkToCid` | 2 | IPLD-compatible EPR wire shape. |
| `@app/elohim/models/source-chain.model` | `HumanConsentContent` and other source-chain entry shapes | 2 | Holochain source chain wire. |
| `@app/elohim/integrity` (barrel) | `BlobMetadataAnchor`, `FederationRegistryAnchor`, `NodeRegistryAnchor`, `IIntegrityAnchor` | 2 | HDI integrity anchor TS contracts. |
| `@app/generated/schema-enums` | (generated) | 2 | **CONSUMER REWRITE ONLY** — already distributed to `app/lamad/src/generated/` by codegen; rewrite import to `@/generated/schema-enums` (or relative). |
| `@app/generated/household-resilience-view` | (generated) | 2 | Same — consumer rewrite only. |
| `@app/generated/distribution-summary` | (generated) | 2 | Same — consumer rewrite only. |
| `@app/generated/resilience-snapshot-view` | (generated) | 1 | Same — consumer rewrite only. |

**Slice 2.5 acceptance signal:** Zero `@app/generated/*` aliases needed in `app/lamad/tsconfig.json`. Wire-type imports resolve from `@elohim/storage-client` (or its barrel). The schema:codegen:ts pipeline's `INTERFACE_FILES` list, if changed, is up to date and `pnpm run schema:codegen:ts --verify` reports no drift.

---

### I slice — `@elohim/identity` (NEW library, Slice 2.3, angular-architect)

**Target:** New Angular library at `app/elohim-library/projects/elohim-identity/`. Scaffold first (use `ng generate library` or copy from `elohim-service` skeleton), register in workspace `angular.json` + root `tsconfig.json`, then migrate.

| Source | Symbol family | Consumer count | Notes |
| --- | --- | --- | --- |
| `@app/imagodei/services/session-human.service` | `SessionHumanService` | 10 | HIGHEST priority for I slice. |
| `@app/imagodei/models/session-human.model` | `SessionHuman` etc. | 5 | Pairs with session-human.service. |
| `@app/imagodei/services/identity.service` | `IdentityService` | 4 | |
| `@app/elohim/services/profile.service` | `ProfileService` | 4 | **HISTORICALLY MISPLACED** under elohim/. Migrate to @elohim/identity — its primary dep is `@app/imagodei/models/profile.model`. |
| `@app/imagodei/models/attestations.model` | `IdentityAttestation` etc. | 2 | |
| `@app/imagodei/models/profile.model` | `HumanProfile`, `JourneyStats`, `CurrentFocus`, `DevelopedCapability`, `TimelineEvent`, `TimelineEventType`, `ContentEngagement`, `NoteWithContext`, `ResumePoint`, `PathWithProgress`, `PathsOverview`, `ProfileSummaryCompact` | 1 | Used by profile.service (same slice). |
| `@app/imagodei/models/identity.model` | `Identity` etc. | 1 | |
| `@app/imagodei/guards/identity.guard` | `identityGuard` | 1 | Auth route guard. |

**Slice 2.3 acceptance signal:** Zero `@app/imagodei/*` or `@app/elohim/services/profile.service` imports remain in `app/lamad/` or `app/elohim-app/`. The new `@elohim/identity` library is registered in `pnpm-workspace.yaml`, `angular.json`, and root `tsconfig.json`. Library build green (`pnpm --filter @elohim/identity build`).

---

### R slice — `@elohim/rea-runtime` (NEW library, Slice 2.4, rust-architect)

**Target:** New Angular library at `app/elohim-library/projects/elohim-rea-runtime/`. Same scaffold pattern as Slice 2.3. Z.D Phase 1 substrate (commits `b2380b899`, `7f66391b6`, `bf2efd191`) already landed — the REA compute-commitment primitive is available from day one.

| Source | Symbol family | Consumer count | Notes |
| --- | --- | --- | --- |
| `@app/shefa/services/event.service` | `EventService` | 4 | hREA EconomicEvent service via StorageApiService. Per the service docstring: REA action types `'use'`, `'produce'`, `'transfer'`, `'cite'`, `'appreciate'`. |
| `@app/shefa` (barrel) | `ECONOMIC_EVENT_FACTORY`, `STEWARDED_RESOURCES`, `EXCHANGE`, `COMPUTE_EVENT`, `CUSTODIAN_METRICS`, `DATA_PROTECTION`, etc. | 2 | DI tokens for REA action-type stewardship interfaces. |
| `@app/shefa/services/attention-tracker.service` | `AttentionTrackerService` | 1 | |
| `@app/shefa/services/resource-explorer.service` | `ResourceExplorerService` | 1 | |
| `@app/shefa/models` (barrel) | REA-shape model re-exports | 1 | |
| `@app/elohim/models` (bare barrel, only `REAAction` + `LamadEventType`) | `REAAction`, `LamadEventType` | 1 | **SPLIT FROM L** — these two symbols specifically belong in @elohim/rea-runtime, not @elohim/service. See operator-input #3. |

**Slice 2.4 acceptance signal:** Zero `@app/shefa/*` imports remain in `app/lamad/`. The new `@elohim/rea-runtime` library is registered and builds clean. `REAAction` and `LamadEventType` resolve from `@elohim/rea-runtime` only (NOT also from `@elohim/service`).

---

### H slice — Doorway HTTP API (Slice 2.6, claude)

**Empty.** No cross-pillar import in lamad currently resolves via a missing doorway HTTP route. Lamad's substrate-API consumption is already mediated through `EprResolverService` (L) and `DoorwayClientService` (L). If a Slice 2.1 migration surfaces a service that SHOULD be H rather than L (e.g., something lamad doesn't need direct DI access to), the slice agent escalates.

---

### E slice — `<elohim-epr-link>` Lit element (Slice 2.6, claude)

**Target:** Replace Angular `<app-epr-link>` consumer usage with `<elohim-epr-link>` Lit element directly.

| Source | Component | Consumer count | Notes |
| --- | --- | --- | --- |
| `@app/elohim/components/epr-link/epr-link.component` | `EprLinkComponent` (Angular thin wrapper around `<elohim-epr-link>`) | 1 | The wrapper is already transitional — its docstring says so. Lamad consumer can use the Lit element directly. |
| `@app/elohim/components/epr-popover/epr-popover.component` | `EprPopoverComponent` | 1 | Programmatically created BY `EprLinkComponent`. When lamad swaps to `<elohim-epr-link>`, the popover concern moves INTO the Lit element (already implemented there per the EprLinkComponent docstring). No separate E migration needed for popover; it's swept by the link migration. |

**Slice 2.6 acceptance signal:** Zero `@app/elohim/components/epr-link/*` or `@app/elohim/components/epr-popover/*` imports remain in `app/lamad/`. Lamad's templates use `<elohim-epr-link>` directly (declared via `CUSTOM_ELEMENTS_SCHEMA` in the consuming components).

---

### D slice — Duplicates

**Empty.** No imports justify in-pillar duplication.

---

### X slice — Deletes

**Empty.** Every imported symbol resolves to live code consumed by lamad.

---

## Migration ordering hints (cross-slice)

1. **Slice 2.1 (L) — start `epr-resolver.service` early.** Slice 2.6 (E) depends on lamad consuming `EprResolverService` from `@elohim/service`; if 2.6 runs against the old `@app/elohim/services/epr-resolver.service` path while 2.1 is mid-flight, consumer rewrites collide.

2. **Slice 2.2 (C) — thin Angular wrappers stay during transition.** When migrating qahal Angular components (`reaction-bar`, `graduated-feedback`) to Lit elements, leave a thin Angular wrapper in qahal so non-lamad consumers (elohim-app, future bundles) don't break. The wrapper can be retired in a follow-up cleanup.

3. **Slice 2.3 (I) — scaffold THEN migrate.** Do not start migrating `session-human.service` (10 consumers) until the new library is registered and a trivial export is verified building end-to-end. Scaffold failure mid-migration causes consumer-rewrite churn.

4. **Slice 2.4 (R) — same pattern as 2.3.** Scaffold first, verify, then migrate.

5. **Slice 2.5 (S) — generated/* consumer rewrites are mechanical.** Can run anytime; produces no library work, just import-path edits in lamad. Verify with `pnpm run schema:codegen:ts --verify`.

6. **Slice 2.7 (docs) — run in parallel from the start.** No code dependencies. Storyteller authors `elohim-sdk.md` (canon) and `pillar-bundle-split-runbook.md` (operational) using the manifest as the structural input. The runbook captures the 6 gotchas from the peer-OAuth-portal integration listed in the plan's Slice 2.7 section.

---

## Notes for Wave 2 slice agents

- **Reclassification is allowed.** If during migration a slice agent finds an import classified differently than its actual shape demands (e.g., a service I marked L turns out to have UI-coupling that makes it C), log the reclassification in the slice commit message and migrate per the actual shape. Do NOT re-trigger Wave 1.
- **Test specs migrate with their subjects.** This was a 2026-05 integration lesson — orphaned specs blocked the pre-push gate. The plan calls this out under "Pre-existing test debt audited" in the readiness checklist.
- **Bidirectional consumer rewrites.** Every L/C/S/I/R migration touches BOTH `app/lamad/` AND `app/elohim-app/` consumers. The pre-push gate runs both bundles' tests.
- **Library scaffolding commits separately.** When Slice 2.3 / 2.4 create new libraries, the scaffold commit (workspace registration + skeleton + lockfile reconciliation) lands before the migration commits — keeps blame surface clean.
- **The bare-barrel split (operator-input #3).** Slice 2.1 and Slice 2.4 must coordinate on `@app/elohim/models` barrel imports — Slice 2.4 takes `REAAction` and `LamadEventType`; Slice 2.1 must NOT add them to `@elohim/service`'s public-api. Suggest Slice 2.4 lands first, then Slice 2.1 sees the resolved symbol home when running its consumer-rewrite grep.

---

## References

- Plan: `genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md`
- B0 audit: `genesis/docs/superpowers/notes/2026-05-25-lamad-pillar-dependency-audit.md`
- Raw enumeration: `/tmp/lamad-imports.txt` (261 unique import lines)
- Canon: `genesis/docs/architecture/stewardship-over-sovereignty.md`, `.../rea-compute-commitment-primitive.md`, `.../cradle-to-grave-capability-gradient.md`
- Pillar-EPR design: `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md`
