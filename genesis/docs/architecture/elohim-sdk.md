# The Elohim SDK

> **Canon status:** Foundational. Read [stewardship-over-sovereignty](epr:stewardship-over-sovereignty), [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive), and [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) first. This doc names the operational consumption surface that those canon principles produce in TypeScript.

---

## §1 — What the Elohim SDK Is

The Elohim SDK is the **TypeScript and Lit element library surface** that pillar bundles and downstream applications build against. It is five libraries — each scoped to a load-bearing concern — that together let a consumer reach the substrate without reaching INTO the substrate.

What it is **not**:

- **Not the protocol.** The protocol lives in Holochain DHT entries (the elohim, mishpat, lamad zomes) and in the `elohim-storage` Rust service that projects those entries into queryable shapes. The SDK is the operational consumption layer above that.
- **Not a source of truth.** Every symbol the SDK exposes is reconstructible from substrate. The libraries hold no state of their own; they hold the shapes and operations consumers use to work with substrate state.
- **Not pillar code.** Pillar bundles (lamad, shefa, qahal, avodah, imagodei, account, doorway) consume the SDK; they are not the SDK. Pillar-local code stays in pillar source.

A developer building a pillar, a doorway integration, or a third-party elohim-aware client reads this canon to learn what to build against. The SDK is the developer-facing answer to "where does the substrate end and my application begin?"

---

## §2 — Why This Canon Exists

The substrate-as-steward shape ([stewardship-over-sovereignty](epr:stewardship-over-sovereignty) §3) demands a clean SDK surface for a structural reason: when consumers reach into substrate internals, they take on responsibilities that belong to the substrate. They begin owning data shapes, holding cache invariants, enforcing reach gates — all things the substrate stewards on their behalf. The SDK boundary is what keeps that stewardship intact.

The pillar-EPR decomposition (`genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` §1) requires bundle independence: each pillar must build standalone, without source-reach into any other pillar. The shared seam between pillars is the SDK. Without a documented SDK boundary, the seam is whatever yesterday's import path happened to be, and bundle independence is fiction.

The cross-pillar import cleanup sprint (`genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md`) is the moment this boundary became canon. The sprint retired 261 cross-pillar imports from `app/lamad/` source by routing each one to its substrate-correct SDK home. The five libraries named below are the result of that classification work — not invented for this doc, but **named** here so the next pillar split (shefa, qahal, avodah, imagodei, account, doorway, per the pillar-EPR design's §0) follows the established home rather than re-deriving.

The REA compute-commitment primitive ([rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) §5) appears throughout the substrate as one shape with many scopes. Z.D Phase 1 landed the primitive's first concrete instance (`delegates-compute` action discriminator with bounds and reciprocity — schemas at commits `b2380b899`, `7f66391b6`, `bf2efd191`). The SDK exposes that primitive as `@elohim/rea-runtime`; consumers in any pillar that touch REA events flow through that library, not through pillar-local re-derivation.

---

## §3 — The Five Libraries

### §3.1 — `@elohim/service` — shared operational services

**Filesystem:** `app/elohim-library/projects/elohim-service/`
**Consumed by:** every pillar bundle, every Angular surface that touches substrate data flow.

This is the home for **Angular services consumed across two or more pillars that hold no UI surface of their own**. The principal services and what they do:

| Service | Concern |
| --- | --- |
| `DataLoaderService` | Cross-cutting substrate read coordinator — most-consumed service in lamad (35 import sites). |
| `AgentService` + `Agent` model | Agent state, mastery tier computation, gate-comparison helpers. |
| `EprResolverService` | Substrate-API consumption of EPR references; the foundational primitive for Slice 2.6 of the cleanup sprint (E disposition). |
| `ContentService`, `ContentResolverService` | Content row fetching + projection adapters. |
| `StorageClientService`, `StorageApiService` | RPC against `elohim-storage` (whether sidecar at `:8090` in Tauri or proxied via doorway in the browser). |
| `LocalSourceChainService` | Holochain source chain client — local-author state. |
| `GovernanceSignalService`, `GovernanceService`, `HumanConsentService` | Governance-layer signal emission and consent stewardship. |
| `ElohimAgentService`, `ElohimPresenceService` | Elohim agent state and presence. |
| `DoorwayClientService` | Browser-side HTTP client against the doorway projection layer. |
| `AffinityTrackingService`, `TrustBadgeService`, `ProjectionApiService`, `LensRegistryService`, `IndexeddbCacheService`, `ContextAssemblyService` | Operational helpers. |
| `provideElohimClient` provider, `ELOHIM_CLIENT` token | Dependency-injection plumbing for the elohim client surface. |
| Integrity-adjacent adapters (`storage-types.adapter` etc.) | Computed/derived field decoration ON TOP OF wire shapes (see §3.3 — adapters never transform the wire). |

**Dependency-inversion pattern.** Some services in `@elohim/service` need a piece of state that lives in a specific pillar. The reverse-dep migration of `ElohimPresenceService` (Slice 2.1 of the cleanup sprint) is the worked example: the service needs `LearnerContextService` from `app/lamad/`, but a library cannot depend on an app. Resolution: `@elohim/service` defines a `LearnerContextProvider` interface; the lamad bundle's bootstrap registers the concrete implementation. The library knows the shape; the pillar provides the value. Future services that face the same direction problem follow this pattern.

**The `@app/elohim/models` barrel splits across two libraries.** During the cleanup sprint Wave 1 manifest, the barrel's exports were enumerated: most symbols (consent helpers, JSON-LD, agent model, presence model) belong in `@elohim/service`; two symbols (`REAAction`, `LamadEventType`) belong in `@elohim/rea-runtime`. Barrels do not map 1:1 to libraries. Enumerate; route each export to its substrate-correct home.

### §3.2 — `elohim-core` — Lit element library for cross-pillar UI primitives

**Filesystem:** `app/elohim-elements/elohim-core/`
**Consumed by:** every bundle that renders cross-pillar UI surface; every doorway projection.

This is the home for **theme-agnostic Lit custom elements consumed by two or more pillars**. The "theme-agnostic" qualifier is structural — these elements expose tokens (CSS custom properties) but never bind brand. The brand binding lives in `app/elohim-library/projects/graphos`'s Library B.

Elements that belong here:

| Element | Concern |
| --- | --- |
| `<elohim-epr-link>` | HyperCard navigation primitive — chip / inline / card / popover variants; progressive 4-layer loading; right-click context menu (per pillar-EPR design §4). Replaces the Angular `EprLinkComponent` thin wrapper and absorbs the `EprPopoverComponent` concern that was previously its programmatic helper. |
| `<elohim-page-chrome>` + `<elohim-default-omnibar>` | Bundle root chrome with slotted omnibar contract. |
| `<elohim-context-menu>` | Accessible fold-down menu (Shift+F10, keyboard nav, ARIA). |
| `<elohim-skeleton>`, `<elohim-mention-base>`, `<elohim-button>`, `<elohim-card>`, `<elohim-badge>` | UI atoms. |
| `<elohim-reaction-bar>`, `<elohim-graduated-feedback>` | Cross-pillar interaction surfaces migrating out of pillar-local Angular into this library (per cleanup sprint Slice 2.2). |
| `<elohim-epr-relationships-panel>`, `<elohim-elohim-navigator>`, `<elohim-content-analytics>`, `<elohim-feedback-mechanism-gateway>` | Cross-pillar relationship and navigation surfaces. |

**Capability-profile contract.** Every element in `elohim-core` declares its Capability Profile coverage per `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`. The profile is a frozen context object naming `lens × theme × contrast × locale × stimulus × textuality × standings`. The substrate (per [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) §6) expects elements to render the same surface differently across that gradient — never hiding what a lower lens shows, only revealing. Elements without a capability-profile declaration are not eligible for `elohim-core`.

**Library A / Library B boundary.** `app/elohim-library/CLAUDE.md` is canon for this boundary. `component-architect` writes Library A stories (the blank-slate proof that the element works without brand binding); `graphos-designer` writes Library B stories (the brand binding via story decorator). The element itself lives in `elohim-core`; the brand binding never enters the element source. If a brand binding requires a CSS custom property the element does not expose, file a `component-architect` follow-up — do not reach inside.

### §3.3 — `@elohim/storage-client` — wire-format types and storage RPC helpers

**Filesystem:** `elohim/sdk/storage-client-ts/`
**Consumed by:** every consumer of substrate data — pillar bundles, doorway service workers, third-party clients.

This is the home for **TypeScript wire-format types that mirror Holochain DHT entries or `elohim-storage` projection rows, plus the RPC helpers used to read and write them**. The library is generated, not authored: `cargo test export_bindings` in `elohim/elohim-storage/` produces the camelCase TypeScript interfaces from the Rust `View` types that carry `#[derive(TS)]`.

What belongs here:

- View types — `ContentView`, `EprProjectionView`, `EconomicEventView`, `CommitmentView`, every wire-format projection row.
- Constitutional substrate primitives — `ReachLevel`, `AffinityScope`, `IntimacyLevel`, `ConsentState`, `GovernanceLayer`, `GeographicLayer`, `AgentType`, `Attestation`, `ConstitutionalConstraint`, `TokenSpecification`, `CrossPillarLink`, `Pillar`, `PROTOCOL_VERSION` — every shape mirroring a Holochain DHT entry.
- Zome wire shapes — `ContentMasteryWire`, `PracticePool`, `MasteryChallenge`, etc.
- IPLD-compatible EPR shapes — `EprHead`, `EprLamadContext`, `EprShefaContext`, `EprQahalContext`, `EprRelationship`, `IpldLink`, `cidToLink`, `linkToCid`.
- Integrity-anchor TS contracts — `BlobMetadataAnchor`, `FederationRegistryAnchor`, `NodeRegistryAnchor`, `IIntegrityAnchor` (TypeScript mirrors of the Holochain HDI integrity anchors, per cleanup sprint Wave 1 manifest operator-input #6).

**JSON-Schema-first IoC.** The wire shape is the contract; the TypeScript and Rust both conform to it. Schemas live in `elohim/sdk/schemas/v1/views/`, governed by the conventions at `elohim/sdk/schemas/v1/views/CONVENTIONS.md`. A schema contract test at `elohim/elohim-storage/tests/schema_contract.rs` catches drift between schema and Rust struct; codegen at `elohim/sdk/schemas/scripts/codegen-ts.mjs` produces the TypeScript interfaces and distributes them into bundle-local `src/generated/` directories. The pre-push hook validates codegen freshness automatically.

**Generated vs distributed.** The library proper (`elohim/sdk/storage-client-ts/src/generated/`) holds the canonical types. Codegen distributes copies into each bundle's `src/generated/` directory (see `INTERFACE_FILES` and `GENERATED_OUTPUT_DIRS` in `codegen-ts.mjs`); per the cleanup sprint Slice 2.5, lamad consumers import from the bundle-local `@/generated/*` form, NOT from `@elohim/storage-client` directly, when working with generated artifacts. This keeps codegen distribution as the established pattern and avoids transitive `@app/generated/*` aliases.

**Snake_case never crosses the boundary.** The Rust structs carry `#[serde(rename_all = "camelCase")]`. TypeScript receives camelCase wire shapes with parsed JSON and proper booleans. There is no `JSON.parse`, no case conversion, no `toWire`/`fromWire` translation in the SDK or in consumers. Adapters in `@elohim/service` (`storage-types.adapter` and similar) decorate the wire with **computed or derived fields only** — they never reshape the wire.

### §3.4 — `@elohim/identity` — auth, session, profile, attestation, identity-guard primitives

**Filesystem:** `app/elohim-library/projects/elohim-identity/` (new library created by Slice 2.3 of the cleanup sprint).
**Consumed by:** every pillar that touches who-the-human-is. Every doorway projection that enforces a reach gate.

This is the home for **identity primitives that the substrate's mediation layer ([cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) §4) requires consumers to consult**. Identity is its own SDK surface because life-stage capacity transitions — the §2 table of the cradle-to-grave canon — make identity load-bearing in a way that a generic "user service" cannot carry. A grandmother in recovery, an adolescent graduating to full agency, a senior under stewardship migration, and an end-of-life executor all consume the same library, but each is rendered through a different mediation pattern. The library names those patterns explicitly so consumers cannot invent ad-hoc identity flows.

What belongs here:

| Symbol | Concern |
| --- | --- |
| `SessionHumanService` + `SessionHuman` model | Current-session identity (10 import sites in lamad as of cleanup sprint Wave 1). |
| `IdentityService` + `Identity` model | Cross-pillar identity primitive. |
| `ProfileService` + `HumanProfile` (+ `JourneyStats`, `CurrentFocus`, `DevelopedCapability`, `TimelineEvent`, `ContentEngagement`, `NoteWithContext`, `ResumePoint`, `PathWithProgress`, `PathsOverview`, `ProfileSummaryCompact`) | The imagodei profile surface — what the human has done, what they are working on, what is open. |
| `IdentityAttestation` and attestation models | Attestations the substrate-layer recovery primitives consume. |
| `identityGuard` and related route guards | Angular route guards enforcing identity context. |

**Standing-curve flattening.** Per cradle-to-grave §3 and `cradle-to-grave-capability-gradient` §5's "Google Superadmin for Stewardship" pattern, the `@elohim/identity` library exposes the recovery-quorum primitives (intimate-circle quorum, extended community consensus, governance act, network-witness) as readable shapes consumers can render. A bundle that needs to surface "you can recover this through your people" calls into `@elohim/identity`, not into a pillar-local re-implementation. The grandma standard ([stewardship-over-sovereignty](epr:stewardship-over-sovereignty) §5) is enforced at the SDK boundary because the library is what UI consumers reach for.

### §3.5 — `@elohim/rea-runtime` — EconomicEvent services, Commitment action types, signal-bearing primitives

**Filesystem:** `app/elohim-library/projects/elohim-rea-runtime/` (new library created by Slice 2.4 of the cleanup sprint).
**Consumed by:** every pillar that emits or reads REA events; every consumer of the `delegates-compute` primitive.

This is the home for **the runtime layer that surfaces the REA compute-commitment primitive to TypeScript consumers**. The primitive's substrate ([rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive)) is Rust + Holochain; the SDK layer here is what pillar code uses to emit `EconomicEvent`s, register `Commitment`s with action discriminators, and consume `FeedbackSignal` streams.

What belongs here:

| Symbol | Concern |
| --- | --- |
| `EventService` | hREA EconomicEvent service against `StorageApiService`; action types `'use'`, `'produce'`, `'transfer'`, `'cite'`, `'appreciate'` (per the service docstring). |
| `ECONOMIC_EVENT_FACTORY`, `STEWARDED_RESOURCES`, `EXCHANGE`, `COMPUTE_EVENT`, `CUSTODIAN_METRICS`, `DATA_PROTECTION` DI tokens | Per-action-type stewardship interfaces. |
| `AttentionTrackerService`, `ResourceExplorerService` | Observability over event streams. |
| `REAAction`, `LamadEventType` | The two enums split out of the `@app/elohim/models` barrel during the cleanup sprint Wave 1 operator-input #3 — these are REA-shaped, not service-shaped. |

**Why REA is its own SDK surface.** The substrate-floor + elohim-ceiling shape ([rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) §1; condensed in `project_substrate_floor_elohim_ceiling` memory) means substrate handles bounded REA primitives deterministically while the runtime layer adds discernment. `@elohim/rea-runtime` is where that discernment is implemented in TypeScript. Folding it into `@elohim/service` would conflate "what substrate enforces" with "what the runtime mediates," and the two have different change cadences: substrate primitives change when the protocol's authority shape evolves; runtime mediations change when consumer experience evolves. Separate libraries, separate change cadences.

**Z.D Phase 1 ready.** The REA compute-commitment schemas (`delegates-compute.schema.json` and `republish-epr.schema.json`) landed on dev as commits `b2380b899`, `7f66391b6`, `bf2efd191` before the cleanup sprint dispatched. The library absorbs the primitive from day one — every future `delegates-compute` instance (per the [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) §5 generalization table: hosting projection, household chore stewardship, qahal moderation, content authorship delegation, compute lending, recovery delegation, guardianship, end-of-life succession) gets its TypeScript surface here.

---

## §4 — Placement Principle

When you have a new symbol to home, decide:

| Question | Yes → |
| --- | --- |
| Is it a wire-format type that mirrors a Holochain DHT entry or `elohim-storage` projection row? | `@elohim/storage-client` |
| Is it a Lit element / theme-agnostic UI primitive consumed by two or more pillars? | `elohim-core` |
| Is it an Angular service consumed by two or more pillars, no UI surface? | `@elohim/service` |
| Is it auth / session / profile / identity-guard / attestation? | `@elohim/identity` |
| Is it an REA EconomicEvent service, Commitment action type, or signal-bearing primitive? | `@elohim/rea-runtime` |
| Is it pillar-local with no cross-pillar consumer? | Stay in pillar source. |
| Does the symbol need state from across the bundle seam? | Consider §5 (substrate-API consumption) BEFORE library DI. |

Two cross-cutting rules:

1. **One symbol, one home.** A symbol does not live in two libraries. If two libraries seem to need it, the library boundary is wrong; redraw it.
2. **Check the symbol's primary dependencies, not its current directory.** Historically-misplaced symbols (the canonical case: `profile.service` living under `elohim/services/` but primarily depending on `@app/imagodei/models/profile.model` — cleanup sprint Wave 1 operator-input #1) are routed by their dependency graph, not by where they accidentally lived.

---

## §5 — Substrate-API Consumption Patterns

Some cross-pillar concerns should **not** be in any library. They should ride the substrate.

### §5.1 — Doorway HTTP

The doorway is the gateway projection of substrate state. When pillar A needs data that pillar B holds, and that data is already projected by `elohim-storage` and served by the doorway, pillar A consumes via doorway HTTP — not via a library binding to pillar B's service.

This keeps the bundle seam clean (pillar A does not import pillar B's service shape) and preserves the substrate's stewardship of the data flow (the doorway is what enforces reach gates and caching, per the pillar-EPR decomposition design's §5 scenarios).

### §5.2 — EPR resolution

Cross-pillar content and relationship references resolve through `<elohim-epr-link>` (in `elohim-core`) plus `EprResolverService` (in `@elohim/service`). The Lit element renders the link; the service walks the substrate to resolve the EPR. Consumers do not chase relationships through pillar-specific service injection.

The HyperCard semantics of the EPR-link (pillar-EPR design §7) — chip / inline / card / popover — let the consumer pick the disclosure shape without coupling to the target pillar's render surface. The substrate resolves; the element renders; the consumer asks for either content or behavior, never both shaped to a specific pillar.

### §5.3 — When to prefer substrate over SDK

The SDK is for **shapes and operations**, not source-of-truth. When the question is "what is the current value of X?" the SDK is the right surface — `EprResolverService.resolve()`, `DataLoaderService.load()`, `StorageApiService.get()`. When the question is "who decides X?" the substrate is the right surface — walk the EPR to its bounding Commitment ([rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) §4), find the constituting authority, verify it is active.

The protocol does not own data; the substrate stewards it. The SDK reflects that ([stewardship-over-sovereignty](epr:stewardship-over-sovereignty) §3). A consumer who reaches the SDK for "who decides X?" is asking the wrong layer the wrong question.

---

## §6 — Cradle-to-Grave Inheritance

Each SDK library serves the cradle-to-grave gradient ([cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) §2) in a different way. The libraries are not life-stage-specific; the gradient flows through them.

| Library | What it carries across life stages |
| --- | --- |
| `@elohim/identity` | Knows the human's standing across life stages — ward, adolescent, adult, senior, end-of-life. Exposes the recovery quorum shapes so consumers can render the graduated authority stack ([cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) §3) consistently. Every elohim mediation role (counsel, specialist, commons-elohim co-steward, per §4) has a surface here. |
| `@elohim/rea-runtime` | Records the value flows that change with life stage — a ward's `act-on-behalf` events, an adult's `delegates-compute` events, a senior's `transition-stewardship` events, an executor's testamentary events. The same primitive, the same library, instantiated per stage. |
| `@elohim/storage-client` | Provides the substrate-correct wire shape regardless of who is consuming. A ward, an adult, and an executor all consume the same `EprProjectionView` shape; the standing-gate is enforced at substrate-projection time, not at SDK time. |
| `@elohim/service` | Orchestrates cross-pillar reads through the elohim mediation pattern. When a consumer needs context-assembled state (a child's learning timeline, a senior's stewardship dashboard), `@elohim/service` is the orchestrator that surfaces the substrate's view. |
| `elohim-core` | Renders the consistent UX across all life-stage surfaces. The Capability Profile (§3.2) makes the disclosure-lens gradient (`minimal` → `simple` → `standard` → `detail` → `debug` → `trace`) the substrate-honored rendering primitive. A child sees `simple`; a kernel developer sees `trace`; the element is the same; the consumer code is the same. |

The SDK's discipline IS the protocol's dignity. The same shape, served correctly across capacity. The same primitive, instantiated across life stage. The substrate's uniformity becomes the SDK's uniformity, and the SDK's uniformity becomes the consumer's predictability.

---

## §7 — The Cross-Pillar Import Cleanup Sprint

The Elohim SDK boundary became canon during the cross-pillar import cleanup sprint (2026-05-25). The sprint took 261 cross-pillar imports out of `app/lamad/` source, classified each into one of eight dispositions (L / C / S / I / R / H / E / D / X), executed file-disjoint parallel migration slices, and removed the transitional path aliases that had let lamad reach into elohim-app at compile time.

What the sprint produced:

- **The five libraries named above.** Two pre-existed (`@elohim/service`, `elohim-core`, `@elohim/storage-client`); two are new (`@elohim/identity`, `@elohim/rea-runtime`). Each library's public-api is now the SDK's published surface.
- **The placement principle and substrate-API consumption patterns.** Captured in this canon doc, applied by every future pillar split.
- **The pillar-bundle-split runbook.** Captured as a sibling canon doc (`pillar-bundle-split-runbook.md`) so the next split — shefa, qahal, avodah, imagodei, account, or doorway — follows the runbook instead of re-deriving the lessons.

The sprint's foundational artifacts (this canon doc + the runbook) are the durable value. The lamad split is the **first** of seven planned pillar splits per the pillar-EPR decomposition design. Each subsequent split inherits the SDK boundary documented here.

---

## §8 — Implementation Reference

| Surface | Path |
| --- | --- |
| `@elohim/service` library | `app/elohim-library/projects/elohim-service/` |
| `elohim-core` Lit library | `app/elohim-elements/elohim-core/` |
| `@elohim/storage-client` (TypeScript wire types) | `elohim/sdk/storage-client-ts/` |
| `@elohim/identity` library | `app/elohim-library/projects/elohim-identity/` |
| `@elohim/rea-runtime` library | `app/elohim-library/projects/elohim-rea-runtime/` |
| JSON Schema source (governs all wire types) | `elohim/sdk/schemas/v1/views/` + conventions at `.../views/CONVENTIONS.md` |
| Schema → TypeScript codegen | `elohim/sdk/schemas/scripts/codegen-ts.mjs` (`INTERFACE_FILES` + `GENERATED_OUTPUT_DIRS`) |
| Schema contract test (drift detector) | `elohim/elohim-storage/tests/schema_contract.rs` |
| Library A / Library B story conventions | `app/elohim-library/CLAUDE.md` |
| Capability Profile element contract | `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` |
| REA compute-commitment substrate (Rust) | `elohim/holochain/dna/mishpat/zomes/coordinator/commitments/src/delegates_compute.rs` + `elohim/elohim-storage/src/services/rea_commitment_service.rs` |

---

## §9 — References

### Canon (this directory)

- [stewardship-over-sovereignty](epr:stewardship-over-sovereignty) — why substrate-as-steward demands a clean SDK boundary.
- [rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) — the substrate primitive that `@elohim/rea-runtime` surfaces to TypeScript consumers.
- [cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) — the life-stage gradient that flows through every SDK library, anchoring why `@elohim/identity` is its own surface.

### Operational canon

- `pillar-bundle-split-runbook.md` (this directory) — the operational runbook for splitting a pillar bundle, anchored against this SDK boundary.

### Specs

- `genesis/docs/superpowers/specs/2026-05-25-pillar-epr-decomposition-design.md` — the pillar-EPR design that motivates bundle independence and names the future pillar splits.
- `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` — the Capability Profile primitive `elohim-core` elements honor.
- `genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md` — Z.D, the first concrete instance of the REA primitive `@elohim/rea-runtime` surfaces.

### Plans

- `genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md` — the sprint that produced this boundary.
- `genesis/docs/superpowers/notes/2026-05-25-cross-pillar-cleanup-dispositions.md` — the Wave 1 disposition manifest that informed every per-library member listed in §3.

### Memory anchors (agent-side)

- `project_elohim_dna_as_sdk_boundary` — DNA-as-SDK shape that this canon's TypeScript surface mirrors.
- `project_first_class_graph_pattern` — the graph shape the wire types in `@elohim/storage-client` express.
- `project_schema_first_ioc` — the JSON-Schema-first IoC pattern `@elohim/storage-client` operates under.
- `project_no_sovereignty_stewardship_over_ownership` — the vocabulary discipline the SDK enforces.

---

## §10 — Closing Note

The five libraries are the SDK. The SDK is the seam between substrate and consumer. The seam is what makes pillar bundle independence real and what makes the protocol's "AI deployment maturity" framing navigable to a developer new to the codebase.

Without this boundary, every consumer re-derives where the substrate ends. With it, the substrate ends where this doc says it does, and the rest of the protocol — pillars, doorways, third-party clients, future life-stage surfaces yet unbuilt — extends from a substrate that is consistent across every consumer.

Build against the SDK. Do not reach past it. If the SDK appears inadequate for your case, that is the moment to extend this canon — not the moment to import across the seam.
