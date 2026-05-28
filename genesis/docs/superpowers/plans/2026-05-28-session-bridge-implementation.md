# Session-Bridge Implementation Plan — Visitor + Peer Graduation Substrate

> **Plan status:** Roadmap. No code. Each ticket below is a discrete substrate-first slice that lands the session-bridge primitive described in the spec.
>
> **Spec basis:** `genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md` (commit `9a0c55a61`).
>
> **Sister plan:** `genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md` — task M-AGGR-2's "deferred deletion" of `LocalSourceChainService` is the immediate work this plan closes. The 4 deferred write-path consumers route through the bridge instead of a direct localStorage simulation.
>
> **P2P Design Gate output:** §1.5 below — every entity classified before any HTTP route, storage table, or coordinator call is proposed. The bridge introduces NO new DHT entry types by design — it is the substrate of *tentative* state that precedes notarization.

---

## §1 — Why this plan exists

The cross-pillar import cleanup sprint (2026-05-25..28) executed the thin-client backend-migration plan through Wave C. Ticket M-AGGR-2 cut the 4 read-path consumers of `LocalSourceChainService` to `HolochainSourceChainService`, then stopped at task #14 — *delete `LocalSourceChainService`* — because 4 write-path consumers still depended on it:

- `app/lamad/src/app/services/path-negotiation.service.ts` (691 lines) — commits path-negotiation entries
- `app/lamad/src/app/services/content-mastery.service.ts` (766 lines) — records mastery attainment entries
- `app/lamad/src/app/services/mastery-stats.service.ts` (465 lines) — writes derived mastery aggregations
- `app/elohim-app/src/app/elohim/services/human-consent.service.ts` (555 lines) — commits consent decisions

The original framing was "rip them off the simulation, write directly to substrate coordinators." That framing didn't survive contact with the realer question: *what should an anonymous browser visitor or an OAuth-identified visitor be able to express before they have a peer-native identity to attach the writes to?*

The spec answers: a **session-bridge** primitive holds tentative intent during pre-canonical lifecycle states (anonymous → oauth → peer-native-sampling) and replays it through the canonical coordinators at a graduation ceremony. Task #14 is then not "delete a simulation" — it's "migrate the 4 consumers to a substrate-correct primitive that has its own first-class shape."

This plan converts spec §1–§11 into substrate-first tickets, sequenced so any phase stall preserves the prior committed work, and gated so the existing `LocalSourceChainService` survives in place until its substrate-correct replacement is ready.

---

## §1.4 — Upstream-surface verification audit (2026-05-28)

Before any ticket dispatches, the plan's upstream assumptions were verified against `dev`. Findings below; per-finding actions are folded into the ticket bodies that depended on them.

### Verified ✓

| Assumption | Where | Status |
|---|---|---|
| `LamadEventIntent` JSON wire schema exists | `elohim/sdk/schemas/v1/intents/lamad-event-intent.schema.json` | ✓ Landed. Carries `agentId` + `lamadEventType` discriminant + ~30 enum values (content-view, mastery, stewardship-begin, etc.). B-PILLAR-SHEFA's `StagedEconomicEventIntent` is byte-identical modulo title. |
| M-REA-1 doorway route `POST /api/v1/lamad/events` | Doorway storage-proxy + route-registry pattern; referenced from `app/elohim-library/projects/elohim-rea-runtime/src/lib/event.service.ts:11+81+130` | ✓ Landed. Route lives in the generic storage-proxy registry, not a dedicated file. `EventService.emitEvent(intent)` is the canonical client entry. B-PILLAR-SHEFA's graduation replays through this existing route. |
| `MANIFEST_KINDS` constant + whitelist mechanism | `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs:37` + validation check at line 96 | ✓ Landed. Whitelist-as-constant pattern confirmed. B-APPRAISE Phase 2's `"graduation-record"` kind addition is a one-line append. |
| The 4 deferred consumer line counts | path-negotiation 691 / content-mastery 766 / mastery-stats 465 / human-consent 555 / LocalSourceChainService 605 | ✓ Unchanged since plan-write. B-CONSUMERS scope is stable. |
| `HolochainSourceChainService` read-path coverage | `app/elohim-library/projects/elohim-service/src/angular/services/holochain-source-chain.service.ts` (81 lines) | ✓ Adequate. Exposes `getEntries(agentId)` + `getLinks(agentId)` + `filterByType(entries, label)` — the three surfaces the 4 ex-consumers need for member-state reads. |
| `WisdomInvocationInput` + `WisdomInvocationResponse` schemas | `elohim/sdk/schemas/v1/inputs/wisdom-invocation-input.schema.json` + `.../views/wisdom-invocation-response.schema.json` | ✓ Landed. Wire format for elohim-agent appraisal already canonical. |
| `ElohimAgentService` + `invoke_wisdom` + `WisdomPhase` stub-as-default | `elohim/elohim-agent/elohim-agent-service/src/wisdom.rs` + `service.rs` + `capability/types.rs` | ✓ Landed and rich. Phase-observed-from-outcome rule (DevContext stub vs ElohimActive real-LLM) means the bridge can ship appraisal surface with stub responses today and flip to real inference without code changes. **Critical for Q7 refinement — see §6.** |
| `ElohimCapability` enum (28 capability variants across content, knowledge map, care, governance, path, family, feedback profile, place, reach negotiation) | `elohim/elohim-agent/elohim-agent-service/src/capability/types.rs` | ✓ Landed. The substrate already names which capabilities elohim agents bring to appraisal work; bridge composes with this taxonomy, doesn't invent a parallel one. |
| Capability Profile spec canonizes `appStandings` + Sub-project #4 deferral | `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md:376` + line 683 | ✓ Confirmed. The user's Q15 reframing maps cleanly to Sub-project #4 (the spec's existing forward-reference). **Critical for Q15 refinement — see §6.** |

### Drift discovered ✗

| Assumption | Reality | Action taken |
|---|---|---|
| `@elohim/identity` is a stable landing target for `VisitorSessionService` | The library's `public-api.ts` has a `PENDING` block: `identity.model`, `profile.model`, `profile.service`, `identity.service`, `identity.guard` are all blocked on Slice 2.1 (L-slice) deps (`agency.model`, `@app/elohim/models/{agent,json-ld,open-graph}`, out-of-scope imagodei services). Only `content-access.model`, `session-human.model`, `session-human.service`, `attestations.model` are LANDED. | **B-DOORWAY Step 3d constraint added:** `VisitorSessionService` MUST NOT inherit deps on the PENDING modules. It composes against `session-human.service` (LANDED) only. `LifecycleAwareReachGuard` similarly avoids `identity.guard` until that lands. Documented in §1.6 below. |
| The bridge needs its own `AppraisalAgent` trait | The substrate already has `ElohimAgentService` + `invoke_wisdom` + `WisdomInvocationInput/Response` schemas + `ElohimCapability` taxonomy + phase-observed-from-outcome stub flow | **B-APPRAISE ticket rewritten:** drop the proposed `AppraisalAgent` trait; compose against existing `ElohimAgentService.invoke_wisdom`. The bridge work becomes batch-extending the wisdom-invocation input to carry the staged-intent pool as a batch, NOT inventing a parallel appraisal surface. See §6 Q7. |
| Q15 framing = "extend the standings axis with pre-member lifecycle values" | The Capability Profile spec splits Standings into **protocol-core (HARD-enforced)** + **app-declared (SOFT-enforced)** and has a canonical deferral (Sub-project #4) for the app-manifest `appStandings` schema. The user's reframing is sharper: elohim-core declares its capability cells; app-manifest declares appStandings + capability-resolution rules; visitor's resolved profile on arrival determines rendering; missing-cell feedback flows back to the app-EPR. | **Q15 reframed in §6 below:** the bridge's lifecycle values become **protocol-core Standings** (anonymous / oauth-identified / peer-native-sampling / peer-native-member). App-manifest's appStandings schema work stays where the canon already places it: Sub-project #4 of the Capability Profile spec. Missing-cell-as-EPR-feedback is the broader vision; not for this sprint. |

### Inherited dependencies confirmed

- M-REA-2 landed (`AttentionTracker` slimmed to ~30 lines, posts to native `/attention/tending` route per commit `e61b89668`). B-CONSUMERS doesn't touch attention-shaped writes — confirmed clean.
- M-REA-3 landed (mishpat `create_recognition_event_from_participation` coordinator + `/api/v1/mishpat/recognition/participation` route per commits `e1c0ac873` + `d27d92098`). B-PILLAR-MISHPAT's rejection-only surface doesn't compete with M-REA-3's substrate work.
- `elohim-agent-sdk` exists as a TypeScript package (`elohim/elohim-agent/elohim-agent-sdk/` with `Dockerfile` + `src/` + `package.json`). Long-term, the bridge's appraisal surface could surface through this SDK; v1 work goes through doorway HTTP routes per the existing `EventService` pattern.

---

## §1.5 — P2P Design Gate (mandatory before tickets)

Every entity the bridge introduces was walked through `.claude/skills/p2p-design-gate/SKILL.md`. The headline result confirms the spec's substrate framing: **the bridge introduces NO new DHT entry types**. Tentative state is not notary state by definition — it is operational projection. Canonical state appears at graduation time and uses existing entry types in the destination pillars.

### Entity classifications

| Entity | Classification | Source of truth | DHT entry type | Storage projection | Address strategy |
|---|---|---|---|---|---|
| `SessionLifecycle` (enum + per-variant fields) | **Category C** (operational) | Local SQLite — doorway session pool (web2 paths) OR sampler's elohim-storage diesel (peer-native sampling path) | None | YES — `session_lifecycle` table per host runtime | Slug/UUID (session_id; no content to hash; the session IS its identifier) |
| `StagedIntent` envelope (generic) | **Category C** | Local SQLite per host runtime, keyed by session_id | None | YES — `staged_intent` table per host runtime | Slug/UUID (stage_receipt_id) |
| `StagedMasteryIntent` (lamad variant) | **Category C** | Same `staged_intent` table; pillar discriminator + JSON payload | None for the staged shape itself — graduates to existing `ContentMastery` entry (Category B2, agent-scoped + attestation) | Existing `staged_intent` row | Slug/UUID (stage_receipt_id) |
| `StagedPathExploredIntent` (lamad variant) | **Category C** | Same `staged_intent` table | Graduates to existing `HumanProgress` entry update | Existing `staged_intent` row | Slug/UUID |
| `StagedConsentIntent` (imagodei variant) | **Category C** | Same `staged_intent` table | Graduates to existing imagodei `Consent` entry; `is_actionable()` returns false from `Anonymous` lifecycle | Existing `staged_intent` row | Slug/UUID |
| `StagedMembershipApplicationIntent` (qahal variant) | **Category C** | Same `staged_intent` table | Graduates to existing qahal `MembershipApplication` entry (Category A, notarized) | Existing `staged_intent` row | Slug/UUID |
| `StagedEconomicEventIntent` (shefa variant) | **Category C** | Same `staged_intent` table; intent shape identical to M-REA-1's `LamadEventIntent` | Graduates via M-REA-1's existing `POST /api/v1/lamad/events` coordinator → existing `EconomicEvent` entry | Existing `staged_intent` row | Slug/UUID |
| `SamplingCache` | **Category C** | Sampler's local pantry slot (peer-native path) OR doorway's projection cache (web2 path) | None — reconstructable from target context's manifest substrate | YES — `sampling_cache` table per host runtime | Slug/UUID (cache_root, session-scoped) |
| `GraduationOffer` | **Category C** | Ephemeral; computed at graduation time | None | NO persistent table — ephemeral object held in service memory; serialized to caller; can be re-computed | Slug/UUID (offer_id, expiry-tracked) |
| `GraduationManifest` | **Category C** (with optional Category A handle) | Service return value; OPTIONALLY notarized as `Manifest{kind:"graduation-record"}` (existing `Manifest` entry type, new whitelisted kind — see Design Constraint 2 below) | None for the manifest object; OPTIONAL existing `Manifest` entry for the appraisal record | Service return value + optional projection of the appraisal record | Slug/UUID (manifest_id) |

### Anti-pattern check — confirmed none apply

- ✗ UUID primary key for a notarized entity — every staged-intent UUID is for an *operational* row; the canonical entry produced at graduation gets its `ActionHash` from the coordinator and is referenced as `dht_anchor_hash` on any related projection.
- ✗ REST route as design starting point — every ticket below starts at the bridge trait surface (3a-equivalent), then the storage projection (3b), then the HTTP route or Tauri-direct call (3c).
- ✗ CID stored as a relational FK — the staged_intent rows reference content by content_id slug (existing convention) and resolve to canonical EntryHashes ONLY at graduation time.
- ✗ Standalone table for agent state — staged intent is session-scoped, not agent-state. Anonymous and OAuth lifecycles have no peer-native agent identity yet; their state correctly lives in the session-bridge's operational tables.
- ✗ Three address formats undefined — every entity above declares Slug/UUID (sessions and stages have no content to hash; they have no canonical EntryHash to reference until graduation).
- ✗ Missing source-of-truth declaration — every storage projection in this plan carries the comment `-- Source of truth: local (operational, pre-canonical staging — graduates to {target_entry_type} on commitment)`.
- ✗ Creating new entry type when one exists — **explicitly avoided**. NO new entry types proposed for the bridge itself. The optional appraisal record reuses the existing `Manifest` entry type with a new `kind` value.
- ✗ Putting granular data on the DHT — staged intent is by design private-to-the-host-runtime and never gossiped. Sampling-cache slices are private to the sampler.

### Design constraints discovered

1. **The bridge is non-notarized BY DESIGN.** Spec §0 + §2 frame this primitive as the substrate of tentative participation — explicitly pre-canonical. Treating any of its primary entities as Category A would betray the design. The gate confirms operational projection is the substrate-correct shape.

2. **Optional graduation-record manifest is the ONLY DHT-adjacent move.** §4's `GraduationManifest::appraisal_record: Option<EntryHash>` gestures at notarizing the appraisal for the participant's records. Implementation reuses the existing `Manifest` entry type with kind `"graduation-record"` — to be whitelisted in `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs:MANIFEST_KINDS` alongside existing `standing-policy` / `tending-policy` / `pillar-projection` / `onboarding` / `app`. This is the only DNA touch in the plan. No new entry type; one new manifest kind whitelisted.

3. **No DHT capacity pressure.** Lamad ~73/~100, Mishpat 11/~100 entirely untouched. The new manifest kind costs one constant entry in a whitelist, not an entry type.

4. **Storage projection ownership splits cleanly.** Doorway-side runtimes (web2 paths: anonymous + OAuth-identified) own `session_lifecycle` / `staged_intent` / `sampling_cache` tables in doorway-service's existing SQLite. Sampler-side runtimes (peer-native sampling) own the same three table shapes in elohim-storage's diesel. The bridge trait abstracts over the storage backend; runtimes provide their own.

5. **The four pillar staged-intent shapes inherit existing canonical entries.** The bridge doesn't redesign mastery, consent, membership, or economic-event entries. It holds simpler pre-canonical shapes and replays through the existing coordinators (M-REA-1 for shefa; the existing `create_human_progress` / `create_content_mastery` / etc. for lamad; existing `create_consent` for imagodei; existing `create_membership_application` for qahal). The bridge is the orchestrator, not a substrate redesign.

6. **Mishpat has no staged-intent shape in v1.** Spec §3 names four pillar variants — lamad, imagodei, qahal, shefa. Spec §7 explicitly notes governance-shape entries (`AttentionTending`, `FeedbackSignal`, `Commitment`, `GovernanceState`) are NOT staged-intent shapes — they require accrued standing and are agent-authored, not visitor-stageable. A `B-PILLAR-MISHPAT` ticket exists in the user's draft sequence; the gate output reclassifies it as "no v1 staged-intent shape; reject any pre-membership governance intent with a reach-gap reason." See §6 deferred decisions.

---

## §1.6 — SDK boundary placement

The bridge spans two layers: a Rust primitive that lives in `crates/session-bridge/` and a TypeScript consumption surface that landing-targets the five-library Elohim SDK (per `genesis/docs/architecture/elohim-sdk.md`). The placement decisions below are explicit so subsequent tickets don't re-derive them, and so third-party developers reading the SDK canon find an unambiguous home for session-bridge symbols.

### Rust crate placement

| Crate | Path | Justification |
|---|---|---|
| `session-bridge` | `crates/session-bridge/` | Per spec §2: `bridges/` translates between **adjacent canonical substrates** (atproto, valueflows); the session-bridge translates between **tentative and canonical** of the same substrate. Different convention. Lives at `crates/` alongside `elohim-sdk`, `doorway-client`, `elohim-storage-client`. |

### TypeScript SDK placement

Per the SDK canon §4 placement principle: *"Is it auth / session / profile / identity-guard / attestation? → `@elohim/identity`."* Session lifecycle IS identity-graduated. The bridge's TypeScript consumption surface belongs in `@elohim/identity` alongside `SessionHumanService` + `IdentityService` + `ProfileService` + `IdentityAttestation`. Putting it in `@elohim/service` would conflate cross-pillar substrate-data services (where `@elohim/service` is canon) with identity-mediation primitives (where `@elohim/identity` is canon).

| Symbol | Home | Justification |
|---|---|---|
| `VisitorSessionService` (Angular) | `@elohim/identity` (`app/elohim-library/projects/elohim-identity/`) | Identity lifecycle primitive; siblings to `SessionHumanService`. Per cradle-to-grave §6 "knows the human's standing across life stages" — pre-member is the new pre-stage; same library carries it. **Verification audit constraint (§1.4):** the library's `public-api.ts` has a PENDING block — `identity.model`, `profile.model`, `profile.service`, `identity.service`, `identity.guard` are blocked on L-slice deps (`agency.model`, out-of-scope imagodei services). `VisitorSessionService` MUST compose against `session-human.service` (LANDED) only; it does not import any PENDING module. `LifecycleAwareReachGuard` similarly avoids `identity.guard` until that lands. |
| `SessionLifecycle`, `StagedIntent` envelope, `GraduationOffer`, `GraduationManifest` (TS types) | `@elohim/storage-client` (`elohim/sdk/storage-client-ts/src/generated/`) via the JSON-schema codegen path | Wire shapes; the same generated-types pipeline that distributes view types per `INTERFACE_FILES` in `codegen-ts.mjs`. Per SDK canon §3.3 "snake_case never crosses the boundary"; bridge codegen follows the same rule. |
| Pillar-specific staged intents (`StagedMasteryIntent`, `StagedConsentIntent`, etc.) | `@elohim/storage-client` generated; pillar-bundle-local DI wrappers | Generated wire types live in storage-client; the Angular service that stages a specific pillar's intent lives in that pillar's bundle (lamad consumer in `app/lamad/src/app/services/`; imagodei consumer in `app/elohim-app/src/app/elohim/services/`). |
| `LifecycleAwareReachGuard` (route guard) | `@elohim/identity` | New route-guard primitive that gates Angular routes by lifecycle state (e.g., a route allowed for OauthIdentified+; an Anonymous visitor sees a graduation prompt). Sibling to `identityGuard`. |

### Why this matters for B-DOORWAY (correction to v1 draft)

The B-DOORWAY ticket as initially drafted placed `VisitorSessionService` in `@elohim/service`. **This is corrected to `@elohim/identity` throughout the plan below.** The 4-consumer migration (B-CONSUMERS) inherits this correction: each consumer injects `VisitorSessionService` from `@elohim/identity`, not `@elohim/service`.

### Cross-Rust-crate consumption

| Consumer | How it consumes `session-bridge` |
|---|---|
| `doorway/doorway-service` | Adds `session-bridge` to its `Cargo.toml`; implements `BridgeStorage` against its SQLite; impls `SessionBridge` for the web2 paths (anonymous + OAuth). |
| `elohim/elohim-storage` | Adds `session-bridge`; impls `BridgeStorage` against diesel; impls `SessionBridge` for peer-native sampling. |
| Pillar coordinator zomes | Don't depend on `session-bridge` directly; their existing coordinator functions get called by `GraduationCeremony` impls. Bridge sits ABOVE the zomes, not inside them. |
| Future third-party Rust runtimes | Add `session-bridge` to their `Cargo.toml`; provide their own `BridgeStorage`; register their `GraduationCeremony` impls into the registry (see B-REGISTRY). |

---

## §1.7 — App-manifest vocabulary extension

The app-manifest substrate (`elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` + per-pillar `elohim/sdk/domains/<pillar>/manifest.json`) is the protocol's extensibility surface — pillars declare their content types, signal kinds, attestations, and projections there. The session-bridge introduces two new manifest sections that make staged-intent vocabulary **manifest-driven** rather than hardcoded in the bridge crate. This is what unlocks third-party extensibility.

### New manifest sections

**`vocabulary.stagedIntents`** (new optional section per pillar manifest)

Mirrors the shape of `vocabulary.contentTypes`. Each entry declares:
- `name` — the intent shape's discriminator (e.g., `"staged-mastery-intent"`)
- `description` — what the intent represents pre-canonically
- `intentSchema` — `$ref` to the intent payload schema (in `elohim/sdk/domains/<pillar>/schemas/`)
- `graduatesTo` — the canonical entry type the intent replays into (e.g., `"ContentMastery"`)
- `actionableFrom` — array of lifecycle states the intent is actionable from (`["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"]`)
- `resolutionMode` — `"deterministic"` | `"negotiated"` | `"either"` — informs the bridge whether to invoke appraisal
- `coupling` — three-leg coupling parallel to contentTypes' shape (knowledge / value / governance), so manifest validation enforces the same structural discipline. Governance leg names which standing-level the graduated entry will be authored under.

**`graduation`** (new optional top-level section per pillar manifest)

Declares the per-pillar graduation policy:
- `deterministicCeremony` — name of the Rust impl that handles deterministic resolutions (looked up via the registry at runtime)
- `negotiatedCeremony` — optional name for negotiated resolutions; if absent, only deterministic is supported
- `appraisalAgent` — `"home-elohim"` | `"commons-elohim"` | `"neutral-counsel"` (default: `"home-elohim"` per §6 Q7) — which elohim appraises negotiated resolutions
- `notarizeAppraisal` — `"always"` | `"on-request"` | `"never"` (default: `"on-request"` per §6 Q8) — whether the `Manifest{kind:"graduation-record"}` entry is authored

### App-manifest schema update

The `app-manifest.schema.json` adds these sections as optional properties with `additionalProperties: false` discipline. Existing manifests without `stagedIntents` or `graduation` continue to validate (the bridge treats their pillars as non-stageable, mirroring the B-PILLAR-MISHPAT pattern).

### Per-pillar manifest landing sequence

Each B-PILLAR ticket below carries a manifest-update sub-step:

| Pillar | Manifest path | What lands |
|---|---|---|
| lamad | `elohim/sdk/domains/lamad/manifest.json` (+ split files) | `vocabulary.stagedIntents` with `staged-mastery-intent` + `staged-path-explored-intent`; `graduation` declaring deterministic ceremony names |
| imagodei | `elohim/sdk/domains/imagodei/manifest.json` | `vocabulary.stagedIntents` with `staged-consent-intent`; `graduation` declaring deterministic ceremony |
| qahal | `elohim/sdk/domains/qahal/manifest.json` | `vocabulary.stagedIntents` with `staged-membership-application-intent`; `graduation` with both deterministic + (Phase 2 gated) negotiated ceremonies |
| shefa | `elohim/sdk/domains/shefa/manifest.json` | `vocabulary.stagedIntents` with `staged-economic-event-intent`; `graduation` declaring it delegates to M-REA-1's existing coordinator |
| mishpat | (no change in v1) | The mishpat manifest stays silent on stagedIntents; bridge rejects mishpat stages with the existing reach-gap reason |
| avodah | (future) | Avodah currently declares `"domain": "shefa"` — when avodah surfaces a visitor-stageable work intent, it adds its own `vocabulary.stagedIntents` |
| third-party pillar | their own manifest | Same shape; bridge picks them up at registry boot |

### Codegen pipeline integration

The per-pillar codegen scripts (`elohim/sdk/domains/<pillar>/scripts/codegen.mjs`) extend to emit:
- `staged-intents.ts` — TypeScript discriminated union of the pillar's staged-intent shapes, with type guards (`isStagedMasteryIntent()`, etc.)
- `graduation-policy.ts` — TypeScript constants for the pillar's graduation policy (informational; runtime trust is the manifest itself)

These distribute to the standard codegen output dirs per `GENERATED_OUTPUT_DIRS` (Angular bundle + library + seeder).

### Protocol-level codegen integration

`elohim/sdk/schemas/scripts/codegen-ts.mjs` extends `INTERFACE_FILES` to include the bridge wire types from §1.6: `session-lifecycle.ts`, `staged-intent-envelope.ts`, `graduation-offer.ts`, `graduation-manifest.ts`. Distributed to:
- `genesis/seeder/src/generated/` (seeder may stage intent for fixture seeding scenarios)
- `app/elohim-app/src/app/generated/`
- `app/elohim-library/projects/elohim-service/src/generated/`
- `app/elohim-library/projects/elohim-identity/src/generated/` (new — added so `VisitorSessionService` consumes locally-generated types per the cleanup sprint Slice 2.5 pattern)

---

## §1.8 — Third-party pillar onboarding contribution path

The architectural value of the manifest + registry + SDK boundary design is that a third-party elohim-native developer can author an onboarding experience without touching the bridge crate, the doorway routes, or the elohim-storage runtime. The path below names what they DO touch.

Consider a hypothetical new pillar `tikvah` (hope/expectation) that wants to let visitors express tentative interest in a future-state pledge:

### Step 1 — Author the staged-intent schema

`elohim/sdk/domains/tikvah/schemas/staged-pledge-intent.schema.json` — JSON Schema describing the intent payload. Cross-references protocol enums (`Reach`, `SubstrateSignal`, etc.) and any tikvah-specific objects.

### Step 2 — Declare in the tikvah manifest

`elohim/sdk/domains/tikvah/manifest.json` adds:

```
"vocabulary": {
  "stagedIntents": {
    "staged-pledge-intent": {
      "description": "Tentative future-state pledge ...",
      "intentSchema": { "$ref": "./schemas/staged-pledge-intent.schema.json" },
      "graduatesTo": "Pledge",
      "actionableFrom": ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"],
      "resolutionMode": "negotiated",
      "coupling": { ... three-leg coupling per manifest convention ... }
    }
  }
},
"graduation": {
  "deterministicCeremony": "tikvah::PledgeDeterministicCeremony",
  "negotiatedCeremony": "tikvah::PledgeNegotiatedCeremony",
  "appraisalAgent": "commons-elohim",
  "notarizeAppraisal": "always"
}
```

### Step 3 — Author the Rust ceremony impls

In the tikvah pillar's Rust crate, implement `GraduationCeremony` for the pledge intent. The trait surface is exported by `session-bridge`; the impl lives in the pillar crate. Publish a `register_tikvah_ceremonies(&mut registry)` function that the runtime calls at boot.

### Step 4 — Register at runtime startup

The doorway-service composition root (and the elohim-storage equivalent) extends to call `tikvah::register_tikvah_ceremonies(&mut bridge_registry)` at startup. This is a one-line addition per consuming runtime; the bridge crate doesn't change.

### Step 5 — Angular consumer

The tikvah pillar's Angular bundle injects `VisitorSessionService` from `@elohim/identity` and stages pledge intents via the existing `bridge.stage()` surface. The pillar discriminator (`"tikvah"`) is resolved from the manifest at runtime.

### Step 6 — Library A + Library B stories

The tikvah pillar's Library A stories include the staged-pledge intent with realistic fixtures pulled from the generated TypeScript types. Library B stories bind the Elohim brand. Storybook automatically discovers them via the existing glob.

### What stays the bridge's responsibility

- Lifecycle state machine (the bridge owns the transitions)
- Storage abstraction (the bridge crate's `BridgeStorage` trait)
- Discard semantics (fail-closed; the bridge owns it)
- Cross-context profiling guardrails (§6 Q11 — the bridge enforces, even for third-party intents)
- OAuth promotion flow (the bridge mediates; the doorway provides the OAuth surface)

### What's the pillar's responsibility

- Intent payload schema authoring
- Manifest declaration
- `GraduationCeremony` Rust impl
- Registry registration line
- Angular consumer
- Storybook coverage

The design intentionally factors the work so the substrate handles the load-bearing concerns (lifecycle + storage + guardrails) and the pillar handles the meaning (what intent, what canonical entry, what appraisal).

---

## §1.9 — Coherence with design vision

This section names the canon principles the session-bridge must align with, and the smallest-correct extension to each canon that the bridge requires.

### Stewardship-over-sovereignty

[stewardship-over-sovereignty](epr:stewardship-over-sovereignty) §3 names the substrate-as-steward shape. The session-bridge is a direct instantiation: tentative participation is something the substrate stewards on the participant's behalf, not something a client orchestrates. The bridge's storage abstraction, discard semantics, and cross-context guardrails are all stewardship moves the substrate enforces. The "always visible to the participant" guardrail (§6 of the spec) is exactly the readable-stewardship principle.

**Smallest-correct canon extension:** none. The bridge fits the canon as-written.

### Cradle-to-grave capability gradient

[cradle-to-grave-capability-gradient](epr:cradle-to-grave-capability-gradient) §2 names life stages (ward → adolescent → adult → senior → end-of-life) with mediation patterns per stage. The session-bridge introduces a **pre-stage gradient** that precedes the cradle: visitor → oauth-identified → peer-native-sampling → peer-native-member. The pre-stage gradient is structurally distinct (it's about onboarding into the protocol, not life-stage transitions within it) but inherits the same mediation discipline (each transition has a defined ceremony; the substrate enforces the grade).

The relationship between the two gradients composes: a senior under recovery quorum (life-stage gradient) might temporarily inhabit a peer-native-sampling lifecycle (pre-stage gradient) while their quorum decides on a stewardship transition. The substrate handles the composition; the bridge handles the pre-stage; existing recovery primitives handle the life-stage.

**Smallest-correct canon extension:** the cradle-to-grave canon adds a §2-prefix paragraph naming the pre-stage gradient as the substrate's "approach to the cradle" surface, with a forward-reference to the session-bridge spec. No structural change to the existing §2 table.

### Capability Profile (elohim-core element contract)

`genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` names the rendering profile: `lens × theme × contrast × locale × stimulus × textuality × standings`. The session-bridge introduces lifecycle as a rendering concern — an `<elohim-shefa-balance-card>` rendered to an Anonymous visitor (no canonical balance yet) looks different from the same element rendered to a PeerNativeMember (full canonical balance).

The right canon move is NOT to add a new dimension (which is invasive — every element re-declares). It's to **extend the existing `standings` axis** to carry pre-member lifecycle values alongside the existing standing grades. The standings axis already orders rendering by reach/standing accrual; pre-member lifecycle naturally extends the bottom of that ordering (visitor < oauth-identified < sampling < member-zero-standing < member-accruing-standing < ... ).

**Smallest-correct canon extension:** the Capability Profile spec's standings axis extends to include the four pre-member values. Elements that want to render onboarding-aware UI add the pre-member values to their `@capability` JSDoc; elements that don't simply don't render in those cells. Library A's existing `Unstyled` + `CustomTheme` proofs gain anonymous-lifecycle variants where it's meaningful.

### Stewardship vocabulary discipline

Per `project_no_sovereignty_stewardship_over_ownership` memory: no "own / ownership / sovereign" vocabulary; use "steward / contributor / authored." The bridge's vocabulary inherits this:
- "participant" (not "user")
- "stage" (not "submit")
- "graduate" (not "commit" — except in the git sense)
- "discard" (not "delete" — discard implies the participant's deliberate choice)
- "host runtime" (not "server")
- "session lifecycle" (not "session state")

The plan uses this vocabulary consistently; the bridge crate's Rust types use it; the JSON schemas' descriptions use it.

### REA compute-commitment primitive

[rea-compute-commitment-primitive](epr:rea-compute-commitment-primitive) §5 names the primitive as "one shape with many scopes." The bridge's `StagedEconomicEventIntent` is one of those scopes — the REA primitive held pre-incarnation. The bridge does not redesign REA; it holds the existing M-REA-1 intent shape across a lifecycle the REA substrate doesn't (and shouldn't) reason about. At graduation, M-REA-1's coordinator authors the canonical EconomicEvent. The bridge is upstream of REA, not parallel to it.

**Smallest-correct canon extension:** none.

### Elohim councils + wisdom as load-bearing primitive

`project_elohim_councils_capture_apex` + `project_dissolution_principle_sensemaking_collectives`: wisdom holds the structural top of authority. The negotiated graduation surface (B-APPRAISE Phase 2) is where this gets exercised at participant-graduation scale. Phase 2 specifically wires elohim inference into a canonical resolution — the substrate asks wisdom to appraise the batch and propose. The participant retains refusal rights (the Half-Price Books counter); the substrate gives wisdom a seat at the appraisal counter, not a unilateral decision.

**Smallest-correct canon extension:** none. B-APPRAISE Phase 2 is the first concrete instance the bridge produces; future appraisal surfaces inherit the pattern.

---

## §2 — Audit findings (what this plan inherits)

### §2.1 — Task #14 carry-over (the four deferred consumers)

| Consumer | Lines | Current write-path role | Bridge-aware destination |
|---|---|---|---|
| `app/lamad/src/app/services/path-negotiation.service.ts` | 691 | Calls `sourceChain.createEntry(...)` with `path-negotiation` content; reads back via `getEntriesByType` | Stage `StagedPathExploredIntent` via `bridge.stage()` while pre-member; write directly to substrate coordinator when member |
| `app/lamad/src/app/services/content-mastery.service.ts` | 766 | Per docstring, supports "visitor mode: localStorage" via `LocalSourceChainService`; commits mastery attainment | Stage `StagedMasteryIntent` while pre-member; write to substrate when member |
| `app/lamad/src/app/services/mastery-stats.service.ts` | 465 | Reads + writes derived aggregations to `LocalSourceChainService` | Read from session-bridge intent pool when pre-member; read from canonical projections when member; never writes (aggregations derive) |
| `app/elohim-app/src/app/elohim/services/human-consent.service.ts` | 555 | Commits `Consent` entries | Stage `StagedConsentIntent` from OAuth+ (anonymous rejects per `is_actionable()`); write directly when member |

**Re-export chain to clean up:**
- `app/elohim-app/src/app/elohim/services/index.ts:10-11` — re-exports `LocalSourceChainService` from `@elohim/service`
- `app/elohim-library/projects/elohim-service/src/public-api.ts:32` — exports `LocalSourceChainService` to the library public API
- `app/elohim-library/projects/elohim-service/src/index.ts:160` — same, secondary export

All three retire when `LocalSourceChainService` is deleted (B-DELETE).

### §2.2 — Test-spec carry-over

| Spec file | Migration |
|---|---|
| `app/lamad/src/app/services/path-negotiation.service.spec.ts` | Mock the session-bridge instead of `LocalSourceChainService` |
| `app/lamad/src/app/services/content-mastery.service.spec.ts` | Same |
| `app/lamad/src/app/services/mastery-stats.service.spec.ts` | Same |
| `app/elohim-app/src/app/elohim/services/human-consent.service.spec.ts` | Same |
| `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.spec.ts` | Delete with the service |

### §2.3 — What's legitimately staying in place during the bridge land

- `HolochainSourceChainService` (`app/elohim-library/projects/elohim-service/src/angular/services/holochain-source-chain.service.ts`) — the M-AGGR-2 read-path replacement. The bridge consumers below MAY read through this for canonical member-state views; the bridge does not replace the read-path service.
- Doorway's existing OAuth surfaces (`doorway/doorway-service/src/auth_routes.rs` + `identity.rs`) — the bridge promotes anonymous → oauth-identified by binding to these surfaces, not by replacing them.
- M-REA-1's `POST /api/v1/lamad/events` route — the bridge's shefa graduation replays through this existing route, not a parallel implementation.

---

## §3 — Migration tickets (substrate-first sequencing)

Every ticket follows the design order from `.claude/skills/p2p-design-gate/SKILL.md` Step 3: **(3a) Trait / coordinator surface → (3b) Storage projection + signal → (3c) HTTP route or Tauri-direct call LAST**.

Per-phase commit discipline: each lettered step is a commit boundary. A ticket stalls cleanly between phases.

Watchdog discipline (per `feedback_multi_agent_pvc_pacing` memory): no full-workspace cargo builds inside ticket phases. Each phase builds only the crates it modifies. Workspace build runs only at ticket close.

---

### Ticket B-CRATE: `crates/session-bridge/` skeleton

**Spec section:** §2 (crate public surface), §3 (intent envelope shape).

**Step 3a — Trait surface, types, no consumers:** Add `crates/session-bridge/` (separate Cargo crate, not under `bridges/` — convention per spec §2: `bridges/` translates between adjacent canonical substrates; this primitive translates between tentative and canonical of the SAME substrate). Crate exports:
- `SessionLifecycle` enum with four variants (Anonymous, OauthIdentified, PeerNativeSampling, PeerNativeMember) plus shared timestamp + expiry fields.
- `StagedIntent` trait (generic over `Pillar` + `GraduatedEntry`; carries `target_context()` + `is_actionable()` predicates).
- `GraduationCeremony` trait (one impl per intent type; async `graduate()` returning `Result<Vec<EntryHash>, GraduationFailure>`).
- `SessionBridge` trait (lifecycle ops: `open_anonymous`, `promote_to_oauth`, `open_sampling`, `stage`, `graduate`, `discard`).
- `CeremonyRegistry` trait + concrete `RuntimeCeremonyRegistry` (per §1.8 third-party path): runtime registry that maps `(pillar_discriminator, staged_intent_kind) → Box<dyn GraduationCeremony>`. Registered at composition-root time by each consuming runtime; lookups happen at `graduate()` call time. Empty registry = no ceremonies = empty `GraduationManifest`; the trait surface ships in this ticket, the registrations land in B-PILLAR tickets.
- `SamplingCache`, `GraduationOffer`, `GraduationManifest`, `StageReceipt`, `BridgeError`, `GraduationFailure` value types.
- `PillarTag` marker trait + zero-cost discriminator enum (extensible — third-party pillars register their own tags via the registry).
- `ContextHandle` newtype (wraps qahal/commons identifier; opaque to the bridge).

**Step 3b — Storage backend abstraction:** Define `BridgeStorage` trait with `load_session` / `save_session` / `stage` / `read_staged_intent_for_session` / `clear_session_state` async ops. The bridge crate provides:
- An in-memory `MemBridgeStorage` for unit tests.
- A trait-only contract for production backends (doorway-service + elohim-storage implement separately in their own crates).

No signal projection at this layer — the bridge itself is not a signal source. Consumers wrap the bridge with whatever signal/event surface they need (e.g., doorway emits SSE, browser-side service emits BehaviorSubject).

**Step 3c — Public surface only:** No HTTP routes. No Tauri commands. The crate ships traits + types + the in-memory test backend. Consumers wire up routes/IPC in later tickets.

**Schema artifacts:**
- `elohim/sdk/schemas/v1/intents/session-bridge/lifecycle.schema.json` — shared `SessionLifecycle` wire format.
- `elohim/sdk/schemas/v1/intents/session-bridge/stage-receipt.schema.json` — receipt returned from `stage()`.
- `elohim/sdk/schemas/v1/intents/session-bridge/graduation-offer.schema.json` — offer object.
- `elohim/sdk/schemas/v1/intents/session-bridge/graduation-manifest.schema.json` — receipt of graduated intent.

Per-pillar staged-intent schemas land in their respective B-PILLAR-* tickets.

**Acceptance — when this ticket closes:**
1. `crates/session-bridge/` builds clean (cargo build + clippy with `-D warnings` + cargo fmt --check).
2. Trait surface covers spec §2 verbatim (operator review against spec §2 type signatures).
3. JSON wire-format schema files added (no new DHT entry types — every entity in this ticket classified Category C in §1.5); `pnpm run schema:codegen:ts` produces TS interface types in `@elohim/storage-client` (no-op consumer until B-DOORWAY).
4. Unit tests cover `MemBridgeStorage` lifecycle + a `StagedIntent` predicate matrix per spec §3 patterns 1-4.
5. Crate is excluded from the `elohim/Cargo.toml` workspace `members` list to avoid coupling to the WASM RUSTFLAGS override (per CLAUDE.md "RUSTFLAGS Override Required" gotcha).

**Dependencies:** None. This is the foundation ticket.

**Commits expected:** ~3 (crate skeleton + traits including registry; storage backend abstraction + in-mem impl + tests; schemas + codegen wiring).

---

### Ticket B-MANIFEST: App-manifest vocabulary extension + protocol codegen wiring

**Spec section / canon basis:** §1.7 above (new vocabulary sections); `elohim/sdk/schemas/CLAUDE.md` (schema-before-code rule); `elohim/sdk/domains/lamad/CLAUDE.md` (per-pillar codegen pattern).

**Step 3a — Protocol-level schema extension:** Update `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json` to add two new top-level properties:
- `vocabulary.stagedIntents` (optional map) — per §1.7 shape. Required nested properties: `description`, `intentSchema` (`$ref`), `graduatesTo`, `actionableFrom` (array of lifecycle enum), `resolutionMode`, `coupling` (mirroring the existing three-leg coupling discipline for content types).
- `graduation` (optional object) — per §1.7 shape. Properties: `deterministicCeremony` (string ID for runtime registry lookup), `negotiatedCeremony` (optional string ID), `appraisalAgent` (enum), `notarizeAppraisal` (enum).

Existing manifests without these sections continue to validate (additive change, no breaking-change semantics). Add the lifecycle-state enum at `elohim/sdk/schemas/v1/enums/session-lifecycle-state.schema.json` so the `actionableFrom` array references a canonical enum.

**Step 3b — Protocol-level codegen extension:** Update `elohim/sdk/schemas/scripts/codegen-ts.mjs`:
- Add bridge wire types to `INTERFACE_FILES`: `session-lifecycle.ts`, `staged-intent-envelope.ts`, `graduation-offer.ts`, `graduation-manifest.ts`, `stage-receipt.ts`.
- Distribute to the existing three locations PLUS a fourth: `app/elohim-library/projects/elohim-identity/src/generated/` (matches §1.6 SDK placement for `VisitorSessionService` consumption).
- Add the lifecycle enum to the enum-codegen path so `SessionLifecycleState` (`Anonymous` | `OauthIdentified` | `PeerNativeSampling` | `PeerNativeMember`) ships as a CORE_* / ALL_* constant set per the existing enum pattern at `schemas/CLAUDE.md` "Adding a New Enum."

**Step 3c — Per-pillar codegen extension:** Update each pillar's `elohim/sdk/domains/<pillar>/scripts/codegen.mjs` to emit:
- `staged-intents.ts` — TypeScript discriminated union of the pillar's staged-intent shapes (read from manifest's `vocabulary.stagedIntents`), with type guards (`isStagedMasteryIntent()`, etc.).
- `graduation-policy.ts` — TypeScript constants for the pillar's graduation policy (informational; runtime trust is the manifest entry itself).

These emit ONLY if the pillar manifest declares `vocabulary.stagedIntents`. Pillars without the section get no generated file (no breaking change to existing manifests).

**Step 3d — Schema validation tests:** Add schema-contract assertions in `elohim/sdk/schemas/scripts/test-schema.mjs` confirming:
1. A manifest with `stagedIntents` validates clean.
2. A manifest missing `stagedIntents` still validates clean.
3. A `stagedIntents` entry missing required `graduatesTo` fails validation with a clear error.
4. The `actionableFrom` array entries are validated against the lifecycle enum.

**Acceptance:**
1. `pnpm run schema:test` passes including the four new manifest-contract assertions.
2. `pnpm run schema:codegen:ts` regenerates bridge wire types to all four distribution locations.
3. Existing per-pillar manifests continue to validate clean (additive change).
4. The new `SessionLifecycleState` enum appears in `schema-enums.ts` across all distribution locations.
5. The `graduation-record` manifest payload kind whitelisted in `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs:MANIFEST_KINDS` (the one DNA touch per §1.5 Design Constraint 2 — landed here so B-APPRAISE Phase 2 can author the entry).

**Dependencies:** None. Parallel to B-CRATE. (Both ship the substrate that subsequent tickets compose against.)

**Commits expected:** ~3 (app-manifest schema + lifecycle enum + new payload kind whitelist; protocol-level codegen extension + distribution; per-pillar codegen extension + schema tests).

**Watchdog:** Node-only codegen + schema tests; no Rust workspace builds until B-CRATE absorbs the new wire types in its trait surface.

---

### Ticket B-DOORWAY: Doorway HTTP wrapping + browser-side `VisitorSessionService`

**Spec section:** §5 (doorway consumer wrapping, web2 visitor path).

**Step 3a — Doorway-side `SessionBridge` implementation:** Add `doorway/doorway-service/src/session_bridge/` module. Implements `BridgeStorage` against doorway-service's existing SQLite store (a new table set, separate from existing projection tables; see Step 3b). Implements `SessionBridge` trait with web2-shaped lifecycle:
- `open_anonymous` issues a session_id (UUID) bound to a `ContextHandle`.
- `promote_to_oauth` triggers when an authenticated OAuth callback arrives on the existing auth routes; migrates staged intent from anonymous → oauth-identified.
- `open_sampling` returns `BridgeError::WrongRuntime` — sampling lifecycle is sampler-side, not doorway-side. (Spec §1's transition table confirms peer-native sampling never originates at doorway.)
- `graduate` looks up the `CeremonyRegistry` (composed at startup — see Step 3a-bis) for each staged intent's `(pillar, intent_kind)` pair, invokes the matching `GraduationCeremony` impl. For B-DOORWAY, the registry is composed but EMPTY (per-pillar registrations land in B-PILLAR tickets); graduation against the empty registry returns an empty `GraduationManifest`. The wiring is in place; the resolutions populate in later tickets.

**Step 3a-bis — `CeremonyRegistry` composition root:** Extend the doorway-service startup (around `doorway/doorway-service/src/main.rs` initialization) to construct a `RuntimeCeremonyRegistry`, call each pillar's `register_ceremonies(&mut registry)` function (no-ops in this ticket; populated in B-PILLAR-*), and inject the registry into the bridge instance. The composition root is the ONE place where the closed set of supported pillars is wired; per §1.8, third-party pillars register here with a single new line.

**Step 3b — Storage projection + signal:**
- New migration `doorway/doorway-service/migrations/<timestamp>_session_bridge.sql` creates three tables: `session_lifecycle`, `staged_intent`, `sampling_cache` (sampling cache shape lands here too even though doorway never opens sampling sessions — keeps the schema uniform with elohim-storage's eventual native impl).
- Each table carries a source-of-truth comment: `-- Source of truth: local (operational, pre-canonical staging — graduates to canonical entries via session-bridge GraduationCeremony impls)`.
- Doorway emits no DHT-adjacent post-commit signal for bridge operations; bridge state is operational and never gossiped. The browser observes state via SSE-style HTTP routes (existing pattern at `doorway/doorway-service/src/sse.rs`).

**Step 3c — HTTP surface (LAST):** Add `doorway/doorway-service/src/routes/visitor_session.rs` per spec §5:
- `POST /api/v1/visitor/session/open` — body `{contextHandle}` returns `{sessionId, expiresAt}`.
- `POST /api/v1/visitor/session/{sessionId}/oauth-promote` — wraps existing OAuth callback.
- `POST /api/v1/visitor/session/{sessionId}/stage/{pillar}` — body is pillar-specific `StagedIntent` JSON; returns `{stageReceiptId}`.
- `GET /api/v1/visitor/session/{sessionId}/staged-intent` — returns full intent pool (per spec §6 "always visible to the participant" guardrail).
- `POST /api/v1/visitor/session/{sessionId}/graduate` — body `{newAgentPubKey, qahalContextHandle}`; returns `GraduationOffer` first; client confirms via PATCH; returns `GraduationManifest`. (Two-step accept-then-execute, per spec §4 Half-Price Books flow.)
- `PATCH /api/v1/visitor/session/{sessionId}/graduate/{offerId}/accept` — body `{acceptedResolutionIds: [...]}` for partial-accept; returns `GraduationManifest`.
- `DELETE /api/v1/visitor/session/{sessionId}` — discard.

Each route has tests under `doorway/doorway-service/tests/visitor_session/`; tests use the in-memory `MemBridgeStorage` plus a doorway-test conductor.

**Step 3d — Browser-side wrapper:** Add `app/elohim-library/projects/elohim-identity/src/lib/visitor-session.service.ts` (`VisitorSessionService`). Per §1.6 SDK boundary placement, this is identity-graduated; lives in `@elohim/identity` alongside `SessionHumanService` + `IdentityService`. Thin HTTP wrapper around the routes above; exposes a reactive `staged-intent$` BehaviorSubject for UI binding; no orchestration logic per `app/elohim-library/CLAUDE.md` thin-client discipline. Export via `@elohim/identity`'s `public-api.ts`.

**Step 3e — Route-guard primitive:** Add `app/elohim-library/projects/elohim-identity/src/lib/lifecycle-aware-reach.guard.ts` (`LifecycleAwareReachGuard`). Angular route guard that gates routes by required lifecycle state (e.g., a route allowed for `OauthIdentified+`; an Anonymous visitor is redirected to the graduation prompt). Sibling to existing `identityGuard`. Export via `@elohim/identity`'s public-api.

**Acceptance:**
1. Migration applies cleanly on a fresh doorway-service SQLite.
2. The 6 HTTP routes are reachable, return correct shapes per spec §5, and pass route tests with the no-ceremonies graduation path returning empty manifests.
3. `VisitorSessionService` is exported from `@elohim/identity` and consumers can import it; no orchestration logic is added (review for thin-client compliance per library CLAUDE.md).
4. JSON wire-format types from B-CRATE codegen as TS types (per §1.7 codegen extension); `VisitorSessionService` uses them, no local interfaces.
5. `LifecycleAwareReachGuard` exported; tests cover redirect behavior for each lifecycle.

**Dependencies:** B-CRATE.

**Commits expected:** ~4 (doorway storage impl + migration; HTTP routes + tests; browser-side service + tests; schema codegen wiring at the consumer side).

**Watchdog:** doorway crate build only; do NOT trigger full elohim-storage or holochain workspace builds during this ticket.

---

### Ticket B-PILLAR-LAMAD: Lamad staged-intent shapes + GraduationCeremony

**Spec section:** §3 lamad pattern (mastery + path-explored).

**Step 3a — Two `StagedIntent` impls + two `GraduationCeremony` impls:**
- `StagedMasteryIntent` carries `{content_id, mastery_level, felt_at, context}`. `is_actionable()` returns true from `OauthIdentified` and later lifecycles (mastery against an unknown agent is meaningless). Graduates to the existing `ContentMastery` entry on the agent's source chain (Category B2 per existing classification).
- `StagedPathExploredIntent` carries `{path_id, step_indexes_visited, committed_step_index, context}`. `is_actionable()` returns true from `OauthIdentified` onward. Graduates to existing `HumanProgress` entry update.
- Each `GraduationCeremony` impl calls the existing lamad coordinator function via doorway's existing zome-call surface; the bridge does not duplicate substrate composition.
- v1 = deterministic graduation only. Negotiated path (multiple paths sampled, reflection notes attached) is gated on B-APPRAISE.
- Add a `register_lamad_ceremonies(&mut registry)` function exported from the lamad pillar crate; doorway-service + elohim-storage composition roots call it at startup (per §1.8 third-party path).

**Step 3a-bis — Lamad manifest declaration:** Update `elohim/sdk/domains/lamad/manifest.json` to add `vocabulary.stagedIntents`:
- `"staged-mastery-intent"` entry → `intentSchema` references the new schema; `graduatesTo: "ContentMastery"`; `actionableFrom: ["OauthIdentified", "PeerNativeSampling", "PeerNativeMember"]`; `resolutionMode: "deterministic"`; coupling mirrors the existing assessment / concept three-leg shape.
- `"staged-path-explored-intent"` entry → same shape pattern; `graduatesTo: "HumanProgress"`.

Add the `graduation` top-level section declaring `deterministicCeremony: "lamad::DeterministicMasteryAndPathCeremony"`; no `negotiatedCeremony` until B-APPRAISE Phase 2. Run `pnpm run lamad:codegen` to emit the new `staged-intents.ts` discriminated union with type guards.

**Step 3b — No new storage projection.** The staged intent rows live in the existing `staged_intent` table from B-DOORWAY (pillar discriminator column distinguishes); the canonical entries land in the existing lamad projections (already covered by M-AGGR-2's read paths).

**Step 3c — HTTP surface piggybacks on B-DOORWAY's `stage` + `graduate` routes.** The pillar discriminator in the URL path (`/stage/lamad`) routes to lamad-pillar deserialization + the lamad ceremony impl. No new routes.

**Schema artifacts:**
- `elohim/sdk/domains/lamad/schemas/staged-mastery-intent.schema.json` (per §1.7 — pillar-specific staged-intent schemas live in the pillar's domain dir, not the protocol-level `intents/` dir)
- `elohim/sdk/domains/lamad/schemas/staged-path-explored-intent.schema.json`

**Acceptance:**
1. Both staged-intent shapes serialize/deserialize cleanly across the doorway boundary.
2. Two-conductor sweettest covers the deterministic graduation: a freshly-incarnated agent's source chain shows the graduated `ContentMastery` + `HumanProgress` entries.
3. Predicate matrix tests cover `is_actionable()` for all four lifecycle states.

**Dependencies:** B-CRATE + B-DOORWAY.

**Commits expected:** ~2 (intent + ceremony impls + schemas; sweettest + predicate tests).

---

### Ticket B-PILLAR-IMAGODEI: Imagodei consent staged-intent + GraduationCeremony

**Spec section:** §3 imagodei pattern (consent).

**Step 3a — One `StagedIntent` + one `GraduationCeremony`:**
- `StagedConsentIntent` carries `{subject, decision, decided_at, context}`. `is_actionable()` returns false from `Anonymous` (spec §3 imagodei constraint: "anonymous consent is meaningless — no identifiable consenter"). Returns true from `OauthIdentified` and later. Graduates to existing imagodei `Consent` entry.
- Graduation calls the existing imagodei `create_consent` coordinator via doorway's zome-call surface.

**Step 3b — No new storage.** Existing `staged_intent` table; pillar discriminator routes deserialization. Existing imagodei consent projection holds the canonical entry post-graduation.

**Step 3c — HTTP surface piggybacks** on B-DOORWAY's `stage` + `graduate` routes via the `imagodei` pillar discriminator.

**Schema artifact:** `elohim/sdk/schemas/v1/intents/session-bridge/staged-consent-intent.schema.json`.

**Acceptance:**
1. Anonymous-lifecycle stage attempts return `BridgeError::IntentNotActionable` per the spec §3 constraint; the test surface explicitly covers this rejection.
2. OAuth-identified stages accumulate; graduation produces the `Consent` entry on the freshly-incarnated source chain.
3. The `human-consent.service.ts` migration target is conceptually unblocked (the actual cutover happens in B-CONSUMERS).

**Dependencies:** B-CRATE + B-DOORWAY.

**Commits expected:** ~2 (intent + ceremony + schema; predicate + sweettest).

---

### Ticket B-PILLAR-QAHAL: Qahal membership-application staged-intent + GraduationCeremony

**Spec section:** §3 qahal pattern (membership-application).

**Step 3a — One `StagedIntent` + one `GraduationCeremony`:**
- `StagedMembershipApplicationIntent` carries `{applying_to, sponsor_witnesses, stated_intent, applied_at}`. `is_actionable()` true from `OauthIdentified` onward (a member-applying-to-X can only be a member-applying-from-known-identity, however nascent). Graduates to existing qahal `MembershipApplication` entry (Category A — notarized membership claim).
- Sponsor-witness accrual happens during sampling/pre-member lifecycles; the intent shape carries the running list. Graduation submits the application with whatever sponsor list has accrued.
- v1 deterministic: every intent in the pool graduates 1:1 to a `MembershipApplication` entry. Negotiated mode (qahal-elohim co-steward appraises whether the application meets the qahal's standards) is gated on B-APPRAISE.

**Step 3b — Existing `staged_intent` table.** Canonical entry lands in qahal's existing `membership_applications` projection (already in mishpat zome — qahal DNA shares the entry type per spec §11 + memory `project_qahal_graduated_capability_surface`).

**Step 3c — HTTP surface piggybacks** with `qahal` pillar discriminator.

**Schema artifact:** `elohim/sdk/schemas/v1/intents/session-bridge/staged-membership-application-intent.schema.json`.

**Acceptance:**
1. Sponsor-witness accrual is observable through the `GET /staged-intent` surface (the participant can see who has sponsored them so far).
2. Graduation produces a `MembershipApplication` entry on the qahal-B DHT; the qahal's coordinator validates it through existing validation logic.
3. Two-conductor sweettest models a sampler-A's session graduating into membership in qahal-B (the seed scenario for the peer-native sampling path).

**Dependencies:** B-CRATE + B-DOORWAY.

**Commits expected:** ~2.

---

### Ticket B-PILLAR-SHEFA: Shefa economic-event staged-intent + GraduationCeremony

**Spec section:** §3 shefa pattern.

**Step 3a — One `StagedIntent` + one `GraduationCeremony`:**
- `StagedEconomicEventIntent` is intentionally identical to M-REA-1's `LamadEventIntent` (per spec §3 + §7 "M-REA-1's `LamadEventIntent` surface"). The bridge holds the same intent shape pre-incarnation; the substrate composes the canonical `EconomicEvent` post-incarnation.
- Graduation replays each staged intent through M-REA-1's existing `POST /api/v1/lamad/events` route (which the M-REA-1 coordinator backs). The bridge does NOT duplicate substrate composition logic.
- `is_actionable()` true from `OauthIdentified` onward (economic events require an actor with at least an OAuth-identifiable provenance).

**Step 3b — Existing `staged_intent` table + existing `economic_events` projection** (the latter unchanged — M-REA-1 already projects it).

**Step 3c — HTTP surface piggybacks** with `shefa` pillar discriminator. Graduation issues internal POSTs to the M-REA-1 route.

**Schema artifact:** `elohim/sdk/schemas/v1/intents/session-bridge/staged-economic-event-intent.schema.json`. Should be byte-identical to the existing `lamad-event-intent.schema.json` modulo the schema title; ensure with a schema-equivalence test.

**Acceptance:**
1. Staged shefa intents serialize identically to M-REA-1's `LamadEventIntent` (round-trip test asserts this).
2. Graduation replays through the existing M-REA-1 route; canonical `EconomicEvent` entries appear in the substrate projection.
3. No parallel REA composition logic is introduced (review confirms the ceremony delegates to M-REA-1).

**Dependencies:** B-CRATE + B-DOORWAY + M-REA-1 must be landed.

**Commits expected:** ~2.

---

### Ticket B-PILLAR-MISHPAT: Mishpat staged-intent (DEFERRED, no v1 shape)

**Spec section:** §3 — does NOT define a mishpat staged-intent shape. §7 explicitly notes governance entries (`AttentionTending`, `FeedbackSignal`, `Commitment`, `GovernanceState`) are agent-authored with accrued standing — NOT visitor-stageable.

**Gate output reframe (from user's draft):** The user's ticket draft listed "B-PILLAR-* (one ticket each: lamad, imagodei, qahal, shefa, mishpat)" — five tickets. The gate analysis reclassifies mishpat as a v1 non-target:
- No staged-intent shape is defined in spec §3 for mishpat.
- Spec §7 establishes that mishpat-shaped writes require existing peer-native standing, which pre-member visitors lack by definition.
- For v1, the bridge rejects any attempt to stage a mishpat intent with `BridgeError::PillarNotStageable` carrying the reach-gap reason "governance moves require accrued standing in the target context."

**Minimal v1 surface (this ticket lands):** Add the `mishpat` pillar discriminator to the rejection list. Add a test asserting that `POST /api/v1/visitor/session/{sessionId}/stage/mishpat` returns 422 with the reach-gap reason. No `StagedIntent` impl; no `GraduationCeremony` impl.

**Deferred to v2 (see §6 deferred decisions):** If a future use case surfaces a mishpat-shaped visitor intent (e.g. visitor signing onto a public petition without yet being a qahal member), revisit and design a `StagedPublicSignatureIntent` shape. v1 does not anticipate this.

**Acceptance:**
1. The mishpat stage-rejection route test passes.
2. Plan §6 entry "mishpat staged intent shape" is updated to "v1 = explicit rejection; v2 = revisit if use case surfaces."

**Dependencies:** B-CRATE + B-DOORWAY.

**Commits expected:** ~1.

---

### Ticket B-APPRAISE: Compose graduation with existing elohim-agent wisdom-invocation substrate (v1 deterministic; batch upgrade in Phase 2)

**Spec section:** §4 (negotiated resolution + appraisal). **Refined per verification audit §1.4** — the substrate already has the appraisal primitives; this ticket composes against them rather than introducing a parallel surface.

**Substrate primitives this ticket composes against (verified in §1.4):**
- `elohim/elohim-agent/elohim-agent-service/src/wisdom.rs` — `invoke_wisdom` async surface; `WisdomInvocationInput { constitution_cid, framing_cid, context_keys, context_json, event_summary }`; `WisdomInvocationResponse { decision, phase, reasoning, side_effects }` where decision is `Allow | Decline | Escalate | Verdict | NeedDeeper`.
- `WisdomPhase` — phase-observed-from-outcome: `DevContext` (stub fallback when no LLM backend) vs `ElohimActive` (real inference). Bridge ships with `DevContext` responses out of the box; flips to `ElohimActive` the moment any operator wires `ANTHROPIC_API_KEY` / `OPENAI_API_KEY`.
- `ElohimCapability` enum (28 variants) — taxonomy the bridge references for "which capabilities does this appraisal exercise."
- `elohim/sdk/schemas/v1/inputs/wisdom-invocation-input.schema.json` + `.../views/wisdom-invocation-response.schema.json` — canonical wire formats.

**Step 3a — Phase 1 (deterministic-only, no substrate change):**

Land `GraduationOffer` / `NegotiatedResolution` types from B-CRATE with `negotiated_resolutions` always empty. Every staged intent classifies as `DeterministicResolution`. The Half-Price Books offer is a strict 1:1 mapping; refusal works; partial-accept works. No wisdom invocation in Phase 1. Bridge ships a usable graduation path with zero new appraisal substrate.

**Step 3a-bis — Phase 2 (batch + rich-context wisdom-invocation extension):**

Extend the existing wisdom-invocation surface to handle BATCH input rather than a single event. Concretely:
- Extend `wisdom-invocation-input.schema.json` with optional `staged_intent_batch` field (array of staged-intent envelopes) + optional `session_context` field (lifecycle + sampling-cache snapshot + consented participant signals). Existing single-event invocations continue to validate (additive change).
- Regenerate Rust types via `cargo test export_bindings`; existing `invoke_wisdom` callers compile unchanged.
- Bridge composes the `WisdomInvocationInput` at graduation time:
  - `constitution_cid` resolves from the target context's manifest-declared constitution
  - `framing_cid` resolves from the per-pillar `graduation` declaration (per §1.7) — pillar-specific framing CIDs let each pillar control how appraisal is framed
  - `context_keys` enumerates the rich-context dimensions the bridge populates (lifecycle, sampling cache, participant profile slice, target-context standing-curve, prior sponsor witnesses for qahal-membership intents, etc.)
  - `context_json` is the serialized subset
  - `event_summary` is a deterministic human-readable summary of the batch
  - `staged_intent_batch` is the full session pool (NEW field)
  - `session_context` is the lifecycle + cache snapshot (NEW field)
- Bridge calls `ElohimAgentService.invoke_wisdom(input)` (existing surface).
- Bridge maps `WisdomDecision` into the `GraduationOffer`'s deterministic / negotiated / rejected / discarded split per spec §4 algorithm.
- `WisdomPhase` propagates onto `GraduationOffer::elohim_appraisal_notes` so the participant sees whether the appraisal came from stub (DevContext) or real inference (ElohimActive). The participant always knows what shape of authority they're refusing or accepting.

**No new `AppraisalAgent` trait. No parallel substrate.** The bridge becomes another caller of `invoke_wisdom`, alongside existing callers (constitutional gate; content review; recognition; etc.).

**Step 3a-tris — Per-pillar elohim-role manifest declaration (per Q7 deferral):**

The app-manifest's `graduation` section (per §1.7) carries a `framingCid` field — the per-pillar wisdom-framing identifier. Which elohim runs the appraisal (Q7 — DEFERRED, not resolved this sprint) is determined by how that framing CID is composed at the pillar's discretion. Lamad might use its home-elohim; qahal might use a commons-co-steward framing; a third-party pillar might use a custom elohim with capabilities the substrate's taxonomy hasn't seen. The bridge doesn't choose; the manifest declares; the wisdom-invocation substrate routes.

**Step 3b — Optional appraisal-record manifest:**

When the participant accepts the offer, the bridge OPTIONALLY notarizes the appraisal as a `Manifest{kind: "graduation-record"}` entry on the graduating identity's source chain (private). The kind whitelist landed in B-MANIFEST. Payload schema lands here: `elohim/sdk/schemas/v1/manifest-payloads/graduation-record.schema.json` (carries `wisdom_invocation_id`, `decision`, `phase`, `reasoning` summary — enough for the participant to audit). The `appraisal_record: Option<EntryHash>` field on `GraduationManifest` populates with the ActionHash; opt-in per graduation event (default: opt-in per §6 Q8).

**Step 3c — HTTP surface unchanged.** Graduation routes from B-DOORWAY already expose `GraduationOffer` / `GraduationManifest`; Phase 1 returns offers with empty `negotiated_resolutions`; Phase 2 returns populated ones plus `WisdomPhase` annotation.

**Schema artifacts:**
- Phase 2: `wisdom-invocation-input.schema.json` extension (batch fields) — DELTA to existing schema, not a new file
- Phase 2: `elohim/sdk/schemas/v1/manifest-payloads/graduation-record.schema.json` (new)

**Acceptance:**
1. Phase 1 lands with deterministic-only graduation flowing end-to-end; participant can accept, refuse, partial-accept; no wisdom invocation surface touched.
2. Phase 2 lands the wisdom-invocation-input batch extension; the bridge composes batch + rich context; calls existing `invoke_wisdom`; maps `WisdomDecision` to `GraduationOffer` shape.
3. Phase 2 ships with `WisdomPhase::DevContext` stub responses out of the box; integration test confirms an `ANTHROPIC_API_KEY` env flips to `WisdomPhase::ElohimActive` without code change.
4. The `Manifest{kind: "graduation-record"}` entry validates against the new payload schema; sweettest verifies the entry round-trips through the DHT.
5. No new `AppraisalAgent` trait introduced; no parallel substrate (review against §1.4 verification finding).

**Dependencies:** Phase 1 has no dependencies beyond B-CRATE + B-DOORWAY. Phase 2 depends on B-MANIFEST (kind whitelist) + the per-pillar manifest `framingCid` declarations from Wave C.

**Commits expected:** ~2 (Phase 1 land; Phase 2 wisdom-invocation extension + bridge compose + graduation-record payload schema).

---

### Ticket B-CONSUMERS: Migrate 4 deferred LocalSourceChain consumers to bridge-aware writes

**Spec section:** §9 (task #14 closure).

This is the immediate closure of M-AGGR-2's deferred deletion. Each consumer migration is independent; they can ship in parallel commits.

**Step 3a/3b/3c (consumer-level — pure client work):**

Each consumer follows the same pattern:
1. Inject `VisitorSessionService` from `@elohim/identity` (per §1.6 SDK placement) in `@app/elohim-app` consumers; lamad consumers go through a thin `LamadVisitorSessionService` DI wrapper in `app/lamad/src/app/services/` that re-exports the `@elohim/identity` symbol (lamad is its own ng workspace).
2. Replace `LocalSourceChainService.createEntry(...)` calls with `bridge.stage(...)` calls, parameterized by the pillar staged-intent shape (generated TS types per §1.7 codegen).
3. For read paths (`getEntriesByType`, etc.), keep the existing `HolochainSourceChainService` for member-state reads; route pre-member reads through `bridge.readStagedIntent()`.
4. Test specs swap mocks from `LocalSourceChainService` → `VisitorSessionService` (and the lamad re-export wrapper where used).

**Per-consumer notes:**

- **`human-consent.service.ts`** (`app/elohim-app/src/app/elohim/services/`) — direct `VisitorSessionService` injection from `@elohim/identity`. Anonymous-state consent attempts now surface a "consent requires identification — please sign in" UX prompt (the bridge returns `BridgeError::IntentNotActionable` per B-PILLAR-IMAGODEI's predicate). Update `human-consent.service.spec.ts` to mock the bridge.
- **`content-mastery.service.ts`** (`app/lamad/src/app/services/`) — depends on the lamad workspace's wrapper. The docstring at line 47 ("Visitor mode: localStorage via LocalSourceChainService (no account needed)") rewrites to "Visitor mode: bridge-staged via VisitorSessionService (no account needed; graduates at incarnation)."
- **`path-negotiation.service.ts`** (`app/lamad/src/app/services/`) — same wrapper pattern. Note: the `hasMinimumIntimacy(...)` consent check at line 9 should continue to gate negotiation; the bridge merely changes the storage backend for the negotiation entries.
- **`mastery-stats.service.ts`** (`app/lamad/src/app/services/`) — read-only consumer; reads through the bridge for pre-member sessions; reads through `HolochainSourceChainService` for member-state.

**Acceptance per consumer:**
1. All tests in the consumer's spec file pass with the bridge mock.
2. `pnpm run lint` clean for both `app/elohim-app` and `app/lamad` workspaces.
3. The 4 imports of `LocalSourceChainService` retire (verified by `grep -rn LocalSourceChainService app/` returning only the service definition itself + the deprecation docstring).

**Dependencies:** B-PILLAR-LAMAD + B-PILLAR-IMAGODEI must be landed. B-DOORWAY must expose the routes.

**Commits expected:** ~5 (one per consumer + one for the lamad-workspace DI wrapper).

**Watchdog:** Angular workspace builds only (`pnpm --filter @app/elohim-app build`, `pnpm --filter @app/lamad build`); do NOT trigger Rust builds during this ticket.

---

### Ticket B-DELETE: Delete `LocalSourceChainService`

**Spec section:** §9 step 5.

Trivial after B-CONSUMERS lands clean. Deletes:
- `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts` (605 lines)
- `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.spec.ts`
- The re-export at `app/elohim-app/src/app/elohim/services/index.ts:10-11`
- The exports at `app/elohim-library/projects/elohim-service/src/public-api.ts:32` and `index.ts:160`

Also delete model file IF unused by `HolochainSourceChainService`:
- `app/elohim-library/projects/elohim-service/src/angular/models/source-chain.model.ts` (verify by grep before deleting; the model types `SourceChainEntry`, `EntryLink`, etc. may continue to be used by `HolochainSourceChainService` — keep what's still referenced).

**Acceptance:**
1. `grep -rn LocalSourceChainService app/` returns zero matches.
2. Both library + app + lamad workspaces build clean.
3. The deprecation docstring lineage retires.

**Dependencies:** B-CONSUMERS complete.

**Commits expected:** ~1.

---

### Ticket B-STORAGE: elohim-storage native `SessionBridge` wrapping (peer-native sampling path, Rust-native)

**Spec section:** §6 (elohim-storage consumer wrapping).

**Step 3a — Rust-native `SessionBridge` implementation:**

Add `elohim/elohim-storage/src/session_bridge/` module. Implements `BridgeStorage` against elohim-storage's existing diesel surface. Implements `SessionBridge` trait with peer-native-shaped lifecycle:
- `open_sampling` issues a session_id bound to the sampler's existing peer-native identity + a target `ContextHandle`. Acquires the target's app manifest via the existing manifest substrate; RS-decodes the necessary projection slices into a sampler-local `pantry` slot per the quilt vocabulary (`project_quilt_pantry_vocabulary`).
- `open_anonymous` / `promote_to_oauth` return `BridgeError::WrongRuntime` — those lifecycles are doorway-side. This is the symmetric inverse of B-DOORWAY's reciprocal `WrongRuntime` on `open_sampling`.
- `graduate` issues a libp2p coordinator call to the target context's qahal coordinator (per B-SAMPLING) — the sampler isn't a member yet, so the call is shaped as a `membership-application + initial-state-bundle` request.
- Tauri-direct callers (steward desktop) hit `SessionBridge` methods directly through the storage-client SDK; no HTTP layer.

**Step 3b — Storage projection:** New diesel migration `elohim/elohim-storage/migrations/<timestamp>_session_bridge.sql` creates:
- `sampling_session` table (mirrors doorway's `session_lifecycle` shape but scoped to peer-native sampling lifecycles only)
- `sampling_staged_intent` table (mirrors doorway's `staged_intent`)
- `sampling_cache` table (mirrors doorway's; same shape)

Each table carries the source-of-truth comment. **Critically: this storage NEVER projects to the DHT** — staged intent + sampling cache are sampler-local state, invisible to the target context per spec §6 (sampling is invisible to the host until graduation by design).

Per-pillar `GraduationCeremony` impls register against the storage-bridge as well as the doorway-bridge (the impls themselves are shared between runtimes — they're trait impls in the bridge crate; the bridge implementations differ in storage, not in pillar-specific graduation logic).

**Step 3c — No HTTP route.** Tauri callers invoke the trait directly. Browser callers MUST go through doorway (sampling is peer-native by definition).

**Schema artifacts:** Shared with B-DOORWAY (same per-pillar staged-intent schemas; same lifecycle schema). No new schemas.

**Acceptance:**
1. Diesel migration applies clean on a fresh elohim-storage SQLite (per `feedback_diesel_migration_timestamp_collision` memory: ensure timestamp is unique against the existing migration set).
2. A sampler-side integration test models sampling a foreign qahal: open sampling session → stage 3 mastery intents + 1 path-explored intent → discard → assert all state cleared (fail-closed per spec §6 guardrails).
3. A second integration test models the graduation: open sampling → stage → graduate → verify canonical entries land in the target via the existing per-pillar coordinator paths.
4. Cross-context profiling guardrail: a test asserts that two sampling sessions on the same elohim-storage targeting different contexts do NOT cross-pollinate staged intent (per spec §6 "no cross-context profiling").

**Dependencies:** B-CRATE + B-PILLAR-* (the trait impls).

**Commits expected:** ~3 (storage impl + migration; cross-context guardrail tests; integration tests).

**Watchdog:** elohim-storage crate build only (RUSTFLAGS=`--cfg getrandom_backend="custom"` per CLAUDE.md gotcha). Do NOT trigger workspace build.

---

### Ticket B-SAMPLING: Peer-native libp2p sampling-handshake protocol

**Spec section:** §6 (peer-native sampling path) + §9 step 7.

**Step 3a — libp2p protocol behaviour:**

Add a new request-response codec in `elohim/elohim-storage/src/p2p/session_bridge.rs` covering:
- `SamplingHandshakeRequest { sampler_agent_id, target_context }` → response with `app_manifest` + RS-decoded projection slice manifest.
- `SamplingCacheFetch { cache_root, slice_id }` → response with the slice bytes (drawn from the target's existing quilt-pantry surface — no new substrate primitive).
- `GraduationRequest { sampler_agent_id, target_context, staged_intent_bundle, new_membership_application }` → routed to the target context's qahal coordinator; response with `GraduationManifest`.

Per `project_libp2p_protocols` (referenced via skill `libp2p-protocols`): MessagePack codec; conventional request-response shape; behaviour composed into the existing elohim-storage libp2p swarm.

**Step 3b — Inventory + signal:** The target context's elohim-storage emits a `SamplingObserved` signal ONLY when graduation arrives (i.e., the host first learns a sampler existed when they ask to join). This honors spec §6 "sampling is invisible from the host's perspective by design." The signal projects to the existing inventory surface; no new table.

**Step 3c — No HTTP route.** Sampling-handshake is libp2p-only. Web2 visitors do not sample; they OAuth-identify at a doorway and graduate via the doorway-wrapped path.

**Acceptance:**
1. Two-node integration test (per `project_alpha_topology_bootstrap_pair`) models a sampler on node-A acquiring qahal-B's manifest + projection slices from node-B's elohim-storage.
2. Graduation test: sampler-A submits `GraduationRequest`; qahal-B's coordinator validates + accepts; sampler-A becomes a member of qahal-B; their staged intent appears as canonical entries on the qahal-B DHT.
3. Host-invisibility test: the sampling phase alone produces no observable signal on qahal-B's host node; only the graduation request triggers the `SamplingObserved` signal.
4. Cross-context guardrail test (libp2p layer): a sampling session for qahal-B cannot read qahal-C's projection slices through the same sampling protocol.

**Dependencies:** B-STORAGE + B-PILLAR-QAHAL.

**Commits expected:** ~3 (codec + behaviour; graduation request flow + tests; host-invisibility + cross-context guardrail tests).

**Watchdog:** elohim-storage crate build only + steward/node crate build (the libp2p behaviour binds at the node layer per the existing pattern).

---

## §4 — Sequencing & waves

The waves below honor the constraint that any phase stall preserves the prior committed work.

### Common steps for every B-PILLAR ticket

Each B-PILLAR ticket has three universal steps in addition to its pillar-specific Rust impls. These are factored out here so they're not repeated verbatim per ticket:

1. **Manifest declaration** — the pillar's `elohim/sdk/domains/<pillar>/manifest.json` gains a `vocabulary.stagedIntents` entry per staged-intent shape (per §1.7 schema) and a `graduation` top-level section declaring the ceremony registry IDs. Run `pnpm run <pillar>:codegen` to emit the generated `staged-intents.ts` discriminated union + type guards.
2. **Schema artifact** — the staged-intent payload schemas live in `elohim/sdk/domains/<pillar>/schemas/` (per §1.7 — pillar-owned vocabulary lives in pillar domain dirs, not the protocol-level `intents/` dir). The schemas cross-reference protocol enums (`Reach`, `SubstrateSignal`, etc.) and the new `SessionLifecycleState` enum from B-MANIFEST.
3. **Registry registration** — each pillar crate exports a `register_<pillar>_ceremonies(&mut registry)` function. The doorway-service composition root + the elohim-storage composition root (when B-STORAGE lands) each call all pillar registers at startup. This is the one-line third-party extension surface per §1.8.

### Wave A — Foundation (parallelizable)

- **B-CRATE** — `crates/session-bridge/` skeleton including `CeremonyRegistry` trait. Lands first; everything depends on it.
- **B-MANIFEST** — app-manifest schema extension + protocol-level codegen wiring + `graduation-record` payload kind whitelist. Parallel with B-CRATE; no shared file dependencies.

### Wave B — Visitor path stand-up

- **B-DOORWAY** — HTTP surface + browser-side service in `@elohim/identity` (per §1.6 SDK placement) + `CeremonyRegistry` composition-root wiring (empty registry at this stage).

### Wave C — Pillar staged-intent shapes (parallelizable; all depend on B-CRATE + B-MANIFEST + B-DOORWAY)

- **B-PILLAR-LAMAD** — mastery + path-explored intents (deterministic).
- **B-PILLAR-IMAGODEI** — consent intent (anonymous rejection enforced; deterministic).
- **B-PILLAR-QAHAL** — membership-application intent (Phase 1 deterministic; Phase 2 negotiated gated on B-APPRAISE Q7).
- **B-PILLAR-SHEFA** — economic-event intent (delegates to M-REA-1).
- **B-PILLAR-MISHPAT** — rejection-only surface (v1 has no staged shape; manifest stays silent on `stagedIntents`).

Each pillar's registration step also wires their ceremony into the doorway-service composition root from Wave B.

### Wave C+ — Negotiated graduation surface (no operator decision needed)

- **B-APPRAISE Phase 1** — empty `negotiated_resolutions` shipped through existing offer surface; everything stays deterministic. Lands as part of Wave C tail (no operator gating).

### Wave D — Original task #14 closure

- **B-CONSUMERS** — migrate 4 deferred consumers (injecting `VisitorSessionService` from `@elohim/identity`); depends on B-PILLAR-LAMAD + B-PILLAR-IMAGODEI.
- **B-DELETE** — delete `LocalSourceChainService`; depends on B-CONSUMERS.

After Wave D, the immediate spec-§9 goal is met: the long-promised deletion happens; the substrate-correct primitive backs the 4 ex-consumers; the visitor-graduation path is fully wired end-to-end. The SDK boundary correction (§1.6) is in production; the app-manifest extension (§1.7) is live; future pillars author against the documented contract.

### Wave E — Negotiated graduation upgrade (gated)

- **B-APPRAISE Phase 2** — single-elohim appraisal; `graduation-record` manifest notarization wired (the payload-kind whitelist landed in B-MANIFEST); depends on §6 Q7 operator decision.

### Wave F — Peer-native sampling

- **B-STORAGE** — elohim-storage native bridge + composition-root registration of all pillar ceremonies (re-uses the per-pillar register functions from Wave C); depends on B-CRATE + Wave C pillar impls.
- **B-SAMPLING** — libp2p handshake protocol; depends on B-STORAGE + B-PILLAR-QAHAL.

Wave F is genuinely new substrate work. If Wave F stalls (the most risk-laden ticket sequence), Waves A–D have already delivered the visitor-graduation path + closed task #14 + landed the third-party extension surface.

### Suggested kickoff ordering for a sprint

| Day | Wave | Tickets |
|---|---|---|
| 1 | A (parallel) | B-CRATE, B-MANIFEST |
| 2–3 | B | B-DOORWAY |
| 3–4 | C (parallel) | B-PILLAR-LAMAD, B-PILLAR-IMAGODEI, B-PILLAR-QAHAL, B-PILLAR-SHEFA, B-PILLAR-MISHPAT |
| 4 | C+ | B-APPRAISE Phase 1 |
| 5 | D | B-CONSUMERS (parallel across the 4 consumers) → B-DELETE |
| 6–7 | F | B-STORAGE |
| 7–8 | F | B-SAMPLING |
| (gated) | E | B-APPRAISE Phase 2 (when §6 Q7 lands) |

---

## §5 — Acceptance criteria (per ticket)

A ticket closes when:

1. **P2P Design Gate output recorded** — every new entity classified per §1.5 above; new classifications added as the implementation surfaces them.
2. **Schema artifacts** — new intent / view / payload schemas land under `elohim/sdk/schemas/v1/intents/session-bridge/` (or `manifest-payloads/` for B-APPRAISE Phase 2); conventions per `elohim/sdk/schemas/v1/views/CONVENTIONS.md`.
3. **Rust types in the bridge crate or runtime** — `#[serde(rename_all = "camelCase")]` + `#[derive(TS)]` where the type crosses the Rust-to-TypeScript boundary.
4. **Schema contract test** — added to `elohim/elohim-storage/tests/schema_contract.rs` for any view/payload that crosses the storage boundary; added to a new `doorway/doorway-service/tests/session_bridge_schema_contract.rs` for doorway-only types.
5. **TS codegen** — `pnpm run schema:codegen:ts` regenerates types in `@elohim/storage-client`; consumers use the generated types, no local interfaces (per `app/elohim-library/CLAUDE.md` mock-data discipline).
6. **Storage migration (where applicable)** — diesel migration applies clean; the new table carries `-- Source of truth: local (operational, pre-canonical staging — graduates to {target_entry_type} on commitment)`.
7. **Trait surface stable** — the `SessionBridge` / `StagedIntent` / `GraduationCeremony` trait signatures match spec §2 verbatim; deviations are documented in this plan inline before the impl ships.
8. **HTTP route (where applicable)** — registered in `doorway/doorway-service/src/routes/visitor_session.rs`; route tests cover happy path + failure modes; cross-context guardrails tested.
9. **Sweettest (where applicable)** — two-conductor sweettest covers any graduation flow that touches a DHT entry (B-PILLAR-LAMAD, B-PILLAR-IMAGODEI, B-PILLAR-QAHAL, B-APPRAISE Phase 2 graduation-record manifest). Per `feedback_sweettest_cross_agent_consistency`: tests use `exchange_peer_info` + `await_consistency` for cross-agent observation.
10. **A2o scenario added** under `genesis/a2o/features/<pillar>/` if user-visible behavior changed. The visitor-graduation flow is a high-value a2o target; B-DOORWAY ticket close MUST include at least one a2o scenario covering anonymous→OAuth-identified→peer-native-member.
11. **Memory hygiene** — update memory `feedback_subagent_silent_impl_drops` discipline: before close, count the trait impls expected vs landed (5 pillars planned; 4 with full graduations; 1 rejection-only).
12. **Watchdog discipline** — no full-workspace cargo builds during ticket phases; per-crate builds only; workspace build at ticket close.

---

## §6 — Deliberately deferred (operator decisions)

The spec §8 holds 11 open questions. This plan ships smallest-reasonable defaults for each so implementation can proceed without prejudging the canonical answer. Each default is reversible by a follow-up operator design pass.

### From the original spec §8 (Q1–Q6)

**Q1 — Sampling cache expiry policy.**
- *Operator decision:* time-based vs storage-pressure vs per-app-manifest configurable.
- *v1 default:* hardcoded defaults per lifecycle — Anonymous: 60 minutes; OauthIdentified: 24 hours; PeerNativeSampling: 7 days. Storage-pressure LRU eviction superimposes (eviction kicks in when sampling-cache total exceeds 100MB per host runtime). Per-app-manifest override is deferred.
- *Implementation surface:* hardcoded constants in B-CRATE; replaced with config-driven values when operator decides.

**Q2 — Zero-standing-permissible writes during sampling.**
- *Operator decision:* per-pillar matrix of "during sampling, this write goes to: never / session-bridge intent pool / sampling-local cache / requires-graduation-first."
- *v1 default:* NO category of zero-standing-permissible writes. Every write goes through `bridge.stage()`. Ephemeral UI prefs (themes, layouts) live in browser localStorage outside the bridge's purview.
- *Implementation surface:* documented in the visitor-session API doc; revisit when a concrete use case for a zero-standing write surfaces.

**Q3 — Graduation rollback / partial-success UX.**
- *Operator decision:* whether `failed_mid_graduation` triggers automatic retry, manual retry, or just a visible status.
- *v1 default:* `failed_mid_graduation` triggers a visible status surface. The participant sees the partial manifest, can manually retry the failed subset, and the bridge holds the failed intents in the session pool until expiry. No automatic retry.
- *Implementation surface:* `GET /api/v1/visitor/session/{sessionId}/graduate-status` route added in B-DOORWAY; UI surface deferred to a follow-up Angular ticket.

**Q4 — OAuth ephemeral mode for anonymity preservation.**
- *Operator decision:* whether to support "OAuth-identify for content access, but don't persist the OAuth subject past session expiry."
- *v1 default:* OAuth subject persists for the session lifetime (24 hours) then discards with session expiry. No long-term persistence past the session. Adequate for the not-surveillance posture; full ephemeral mode (memory-only, never written to SQLite) deferred.
- *Implementation surface:* documented as a session lifetime semantics note; full ephemeral mode adds an `ephemeral: bool` flag to `open_anonymous` in a v1.5 follow-up.

**Q5 — Sampling cache size scale rules.**
- *Operator decision:* whether sampling-cache size scales with target-context size or stays bounded.
- *v1 default:* hard cap of 100MB per sampling session (bounded). Target-context-size scaling deferred. If a sampler hits the cap, the session is degraded — further slice fetches return `BridgeError::SamplingCapacityExceeded` and the participant is prompted to graduate-or-walk-away.
- *Implementation surface:* `SAMPLING_CACHE_BYTES_MAX` constant in B-CRATE; per-context override via app-manifest field deferred.

**Q6 — Sampling as a federation primitive.**
- *Operator decision:* whether long-running sampling becomes a federation pattern.
- *v1 default:* sampling is "sample-then-join-or-walk-away." Long-running federation sampling is out of scope. The substrate enforces this via the expiry policy in Q1 — sampling sessions cannot persist indefinitely without graduating.
- *Implementation surface:* none; revisit if federation use case surfaces a need for indefinite sampling.

### From the appraisal/negotiation framing in spec §4 (Q7–Q11)

**Q7 — Which elohim appraises (REFRAMED per operator guidance + verification audit).**
- *Operator guidance (2026-05-28):* Which elohim appraises is **context-dependent**. An elohim-agent with an app-manifest role does the appraisal — could be custom per pillar, but still bridges context from the visitor's session into the target context. The bridge's job is **not** to choose the appraiser; it's to iterate the elohim gate so it handles **batch** input and **rich-context** gathering from the staged-intent pool, with the actual inference call stubbed for now.
- *Verification (§1.4):* The substrate ALREADY has the appraisal primitives — `ElohimAgentService` + `invoke_wisdom` + `WisdomInvocationInput/Response` + `ElohimCapability` enum + the phase-observed-from-outcome stub-as-default rule (`WisdomPhase::DevContext` when no LLM is wired; `WisdomPhase::ElohimActive` when one is). The bridge does NOT introduce its own `AppraisalAgent` trait. It composes against the existing wisdom-invocation surface.
- *v1 default (refined):* B-APPRAISE Phase 1 stays deterministic-only (no wisdom invocation). B-APPRAISE Phase 2 **extends the existing `WisdomInvocationInput` to accept batch input** — a `staged_intent_pool` field carrying the full session's staged intents instead of a single event. The `context_keys` + `context_json` fields already exist on the schema; the bridge populates them with the visitor's session lifecycle + the target context's standing-curve + the consented participant context. The inference call returns `WisdomPhase::DevContext` stub responses out of the box; the moment any operator wires an LLM backend (ANTHROPIC_API_KEY / OPENAI_API_KEY env), `WisdomPhase::ElohimActive` flips on with zero code change.
- *Which elohim is the appraiser (deferred — operator decision NOT being resolved):* the app-manifest declares the role of the elohim-agent that runs appraisal for THAT pillar (potentially custom). The bridge looks up the manifest-declared elohim role at graduation time and invokes through the existing service. Three-elohim ceremony (home + commons + neutral, per `project_elohim_councils_capture_apex`) is one possible configuration; the substrate doesn't preclude it. The choice belongs to each pillar's manifest, not the bridge.
- *Implementation surface (refined):* B-APPRAISE Phase 2 = (a) extend `wisdom-invocation-input.schema.json` with optional `staged_intent_batch` field; (b) Rust types regenerate from schema; (c) bridge's graduation flow composes the batch from the session pool + rich context, calls `invoke_wisdom`, maps the `WisdomDecision` (Allow / Decline / Escalate / Verdict / NeedDeeper) into the `GraduationOffer`'s deterministic / negotiated split. No new trait. No new substrate.

**Q8 — Appraisal auditability and reproducibility.**
- *Operator decision:* whether appraisals are reproducible, notarized, comparable.
- *v1 default:* the optional `appraisal_record` (notarized as `Manifest{kind: "graduation-record"}` per Design Constraint 2) defaults to opt-in per graduation event. Notarization records the appraisal narrative + inputs hash. Re-appraisal is supported by re-running the offer ceremony; output comparison is the participant's responsibility (manual diff). Cryptographic reproducibility (deterministic appraisal output) is deferred.
- *Implementation surface:* the `graduation-record` manifest kind in B-APPRAISE Phase 2; cryptographic reproducibility revisits if appraisal-determinism becomes a requirement.

**Q9 — Cost of refusal (preventing appraisal-shopping).**
- *Operator decision:* what friction prevents repeated re-graduation attempts to extract more favorable appraisals.
- *v1 default:* NO friction. The participant can re-request a graduation offer freely; the bridge does not track prior-offer history. Appraisal-shopping resistance is deferred to a §8 follow-up spec. Rationale: v1 has deterministic-only Phase 1 (no negotiated resolutions = no appraisal to shop); Phase 2's negotiated resolutions surface this question explicitly when they ship.
- *Implementation surface:* none in v1. Phase 2 might add a `prior_offer_count` field to the session lifecycle if shopping resistance becomes a v1.5 requirement.

**Q10 — Counter-offer mechanics on the participant side.**
- *Operator decision:* whether the participant can propose a novel alternative not on the menu, and whether that's a re-appraisal request or a new staged intent.
- *v1 default:* counter-offer = a new staged intent through the same offer-construction cycle (the simpler treatment per the spec's own suggestion). Novel counter-proposals are not a distinct primitive in v1.
- *Implementation surface:* documented in the visitor-session API doc; revisit if a UX surface emerges where the simpler treatment feels brittle.

**Q11 — Substrate-level enforcement of no-cross-context-profiling.**
- *Operator decision:* whether the no-cross-context-profiling guardrails (spec §6) are enforced cryptographically, by audit, or by convention.
- *v1 default:* enforcement is by **convention + unit test + code review**. The session-bridge crate ships a `CrossContextLeakageTest` asserting that two sessions on the same storage backend with different `target_context` values do NOT share staged intent or sampling cache state (the B-STORAGE acceptance criterion #4 above). Cryptographic enforcement (per-context encryption keys, opaque session-pool boundaries) is deferred to a doorway-spec follow-up.
- *Implementation surface:* the leakage test as documented; cryptographic boundary revisits if a threat model surfaces convention-only enforcement as inadequate.

### Plan-discovered deferrals (additions to the operator decision queue)

These surfaced during the gate analysis above and are NOT in the spec §8 list. They wait for operator review.

**Q12 — Mishpat staged-intent shape (if any).**
- *Background:* §3 names four pillar variants; mishpat is omitted. §7 explicitly notes mishpat-shaped entries require accrued standing. The user's draft sequence listed B-PILLAR-MISHPAT alongside the four other pillars.
- *v1 default:* rejection-only surface (per B-PILLAR-MISHPAT ticket above). Reach-gap reason returned to caller.
- *Operator decision needed if/when:* a concrete use case for a visitor-stageable mishpat intent surfaces (e.g. petition-signing without prior membership).

**Q13 — Pillar-discriminator extensibility.**
- *Background:* the URL path `POST /api/v1/visitor/session/{sessionId}/stage/{pillar}` could hardcode the five known pillars (lamad/imagodei/qahal/shefa/mishpat) OR resolve dynamically from the manifest registry.
- *v1 resolution (from §1.7 + §1.8 refinement):* the pillar discriminator is **manifest-driven**, not hardcoded. The doorway-service composition root reads the registered pillar manifests at startup and accepts any pillar discriminator whose manifest declares `vocabulary.stagedIntents`. Third-party pillars are first-class: they land their manifest + ceremony impl + registry registration, the discriminator becomes accepted automatically.
- *Operator decision still pending:* what trust does the substrate require for a third-party pillar's manifest to be accepted? Manifest-signing? Stewardship attestation? See Q14 below.

**Q14 — Third-party pillar manifest authorization.**
- *Background:* §1.8 names the third-party pillar onboarding path. The substrate accepts any pillar that publishes a valid manifest + ceremony impl. But "any" creates a trust gap: a malicious pillar could declare a staged-intent shape that graduates into entries on existing pillars' DHTs.
- *v1 default:* third-party pillars only land if the runtime operator (doorway / steward / hub) deliberately adds the pillar crate to their Cargo dependencies AND invokes the pillar's `register_<pillar>_ceremonies` at composition root. This is a hard-coded compile-time + composition-root authorization gate. Manifest-signing / stewardship-attestation / qahal-witness authorization is deferred.
- *Operator decision needed:* what does federation-shaped pillar authorization look like when third-party pillars want to ship without operator-side recompilation? Probably a `Manifest{kind:"pillar-authorization"}` entry that vouches for a pillar's staged-intent contract; needs design work.

**Q15 — Capability surface for lifecycle-aware rendering (REFRAMED per operator guidance + verification audit).**
- *Operator guidance (2026-05-28):* elohim-core should have its OWN capability levels (intrinsic — the element knows what cells it can render in). The app-manifest should provide PLACEHOLDERS to consider the user's capabilities for that kind of rendering — so a visitor gets a UX that matches their capability grant **on arrival**. If the app-manifest does not implement the necessary surfaces for accessibility, that becomes feedback to the app-EPR that might force implementation of the missing surface capabilities. **This is how elohim-core creates interfaces for implementation in compliant app-manifests.** Not for this sprint, but it's the model.
- *Verification (§1.4):* The Capability Profile spec ALREADY canonizes this model. §5 splits Standings into:
  - **§5.1 Protocol-core Standings (HARD-enforced)** — `pilot`, `contributor`, `steward`, `elohim-support`. If declared required and not held, the element refuses to render via `<elohim-standing-refused>` slot.
  - **§5.2 App-declared Standings (SOFT-enforced)** — declared in app-manifest under `appStandings`. If required and not held, element renders `<elohim-standing-placeholder>` (graceful degradation; not load-bearing for protocol).
  - **Sub-project #4 (spec line 683, DEFERRED in the spec itself)** — *"Full app-manifest schema for `appStandings` (required/optional, descriptions, defaults, placeholder copy)."* This is exactly the substrate the operator's reframing describes. The deferral is canon; the bridge doesn't pre-empt it.
- *v1 default (refined):* the bridge contributes the four lifecycle values as **protocol-core Standings** (anonymous / oauth-identified / peer-native-sampling / peer-native-member). These join the existing core Standings list (pilot / contributor / steward / elohim-support) at the §5.1 tier, HARD-enforced via `<elohim-standing-refused>`. The bridge's `SessionLifecycle` resolves to exactly one of these Standings at any moment; the Capability Profile carries it; elements declaring lifecycle-required cells get refused or rendered per the existing element-contract pattern.
- *What this sprint does NOT touch:* the app-manifest `appStandings` schema (Sub-project #4 of the Capability Profile spec). Pillars adopt that schema on the spec's timeline, not the bridge's. The bridge's pre-member Standings are protocol-core, so they work even without `appStandings` being formalized.
- *What this sprint enables for future work:* missing-cell-as-EPR-feedback flow — when an element declares a cell it can render in and the app-manifest's capability resolution can't deliver the visitor to that cell on arrival, a FeedbackSignal entry against the app-EPR records the gap. Future pillar implementations of the missing surface capability close the loop. Tracked here so it doesn't get lost; substrate work goes through the Capability Profile spec's Sub-project #4 + feedback-information-flows design.
- *Implementation surface (this sprint):* a one-paragraph follow-up patch to the Capability Profile spec §5.1 adding the four lifecycle Standings to the protocol-core enumeration. No structural change. No element-contract churn. Captured as a doc PR alongside the B-CRATE landing.

**Q16 — Cradle-to-grave canon extension.**
- *Background:* §1.9 names the pre-stage gradient (visitor → oauth-identified → peer-native-sampling → peer-native-member) as the substrate's "approach to the cradle" surface. The existing cradle-to-grave canon §2 doesn't name it.
- *v1 default:* the bridge lands without updating the cradle-to-grave canon. A follow-up doc patch adds a §2-prefix paragraph naming the pre-stage gradient with a forward-reference to the session-bridge spec.
- *Operator decision needed:* trivial — confirm the canon extension. Probably a 10-minute review-and-merge.

**Q17 — Avodah onboarding integration timing.**
- *Background:* avodah currently declares `"domain": "shefa"` in its manifest. The session-bridge's per-pillar surface assumes one manifest per pillar; avodah may want its own staged-intent shape (a `staged-work-pledge-intent`?) but currently composes against shefa's vocabulary.
- *v1 default:* avodah does not land staged-intent vocabulary in v1; visitor-stageable work pledges (if any) ride shefa's `staged-economic-event-intent` shape.
- *Operator decision needed if/when:* avodah surfaces a distinct visitor-stageable intent shape that doesn't fit shefa's REA event surface.

---

## §7 — Why this plan matters beyond the immediate work

The thin-client backend-migration plan named one pattern (substrate orchestration that leaked client-side). This plan names a deeper one: **the protocol had no first-class vocabulary for tentative participation.** Every layer that needed it improvised — `LocalSourceChainService` simulated a source chain in localStorage; consumers branched on "visitor mode vs member mode" with no shared abstraction; identity graduation was framed as a one-step OAuth move when it's really at least two (anonymous → OAuth → peer-native).

The session-bridge primitive resolves that by naming the missing layer: a substrate of pre-canonical staged intent + a graduation ceremony that replays into canonical state at incarnation. The four lifecycle states (Anonymous, OauthIdentified, PeerNativeSampling, PeerNativeMember) become explicit; the bridge enforces the predicate matrix; consumers branch on a typed lifecycle, not on a brittle "isAnonymous" boolean.

This matters for three orbital reasons:

1. **It aligns onboarding with the qahal graduated capability surface.** The qahal pillar carries graduated capabilities (`project_qahal_graduated_capability_surface`); the session-bridge slots in BELOW the lowest membership tier as a "pre-tier" that participation crosses through. Without it, pre-member experience was a hack; with it, pre-member is a first-class participation grade.

2. **It honors the "grandma standard" recovery model.** Heavy account incarnation (`project_recovery_grandma_standard`) was always going to be at odds with low-friction sampling — the session-bridge resolves the tension by letting people sample, accumulate intent, and graduate ONLY when they're ready for the heavy ceremony. Refusal stays a first-class participation choice.

3. **It models the Half-Price Books appraisal surface for the negotiated graduation.** Per `project_elohim_councils_capture_apex` and `project_dissolution_principle_sensemaking_collectives`, elohim wisdom holds the structural top of authority in the protocol's mature shape. Negotiated graduation is one of the first substrate primitives where elohim inference actively shapes a canonical outcome (vs. observing one). B-APPRAISE Phase 2 is the proving ground for "wisdom as load-bearing primitive" at participant-graduation scale.

Future flows that fit the same shape — sampling between federated commons, deep-archive surfaces inviting graduation to active state, recovery-quorum incarnation — will inherit this primitive rather than re-inventing it. The bridge is the protocol's vocabulary for "not-yet-incarnated, but expressing intent."

---

## §7.5 — Scaling the primitive: substrate for native onboarding authors

The plan's structural ambition extends past the M-AGGR-2 cleanup. The session-bridge is intentionally an **author-facing substrate** — a load-bearing surface that elohim-native pillar developers compose against to build meaningful onboarding experiences without re-deriving the lifecycle / storage / discard / guardrail concerns. This section names what scaling looks like and why the design choices above support it.

### The author's contract

An elohim-native pillar developer who wants to surface a tentative-participation experience reads three documents:
1. The session-bridge spec — for the lifecycle semantics and Half-Price Books appraisal model.
2. The SDK canon (`genesis/docs/architecture/elohim-sdk.md`) — for the five-library boundary and the placement rule that surfaces `VisitorSessionService` in `@elohim/identity`.
3. The Capability Profile spec — for how their elohim-core elements render across the pre-member lifecycle gradient.

They author six artifacts (per §1.8):
- A staged-intent payload schema.
- A pillar-manifest `vocabulary.stagedIntents` + `graduation` declaration.
- A Rust `GraduationCeremony` impl in their pillar crate.
- A `register_<pillar>_ceremonies` registry registration.
- An Angular consumer that injects `VisitorSessionService`.
- Library A + Library B stories that render the intent across lifecycle states.

Nothing else. The bridge crate, the doorway routes, the storage abstraction, the discard semantics, the cross-context guardrails — all already exist. The pillar adds meaning; the substrate carries the load.

### What the scaling unlocks

| Scaling axis | Without the substrate | With the substrate |
|---|---|---|
| Number of onboarding experiences per pillar | 1 hand-rolled per pillar; each re-derives lifecycle handling | N declared in manifest; bridge dispatches |
| Number of pillars that ship onboarding | Bounded by who can afford the substrate work | Bounded only by manifest authoring + ceremony impl complexity |
| Federation between commons | Each federation re-invents tentative participation | Sampling lifecycle composes naturally with federation; B-SAMPLING + future federation specs share the same primitive |
| Third-party (non-Elohim-team) pillar authoring | Effectively prevented (too much substrate-internal work) | A documented contract; the bridge crate stays generic |
| Capability Profile coverage | Onboarding edge cases sit outside the profile; element behavior drifts | Pre-member lifecycle values live in the standings axis (Q15); profile coverage is uniform across member + pre-member |
| Cradle-to-grave gradient | The cradle has no documented "approach" surface | The pre-stage gradient (§1.9) is the substrate's documented approach to the cradle |
| Subsuming web2 onboarding | Each app reproduces web2 sign-up flows; visitors get a degraded experience | Doorway projects native onboarding-shape; the protocol's Half-Price Books appraisal is the user-facing surface |

### Coherence across waves

Each wave below leaves a usable surface for the next class of authors, so adoption doesn't wait for the whole plan to land:

- After **Wave A** lands, the `crates/session-bridge/` API + manifest extension exists. Pillar developers can start scoping their staged-intent shapes.
- After **Wave B** lands, the doorway visitor-session HTTP surface exists. An Angular bundle can start drafting `VisitorSessionService` consumption with mocked ceremonies.
- After **Wave C** lands, the four shipping pillars demonstrate the manifest + registry + ceremony pattern. Other pillar authors copy the pattern with confidence.
- After **Wave D** lands, the SDK boundary correction is in production. New pillars author against the documented contract.
- After **Wave E** lands, negotiated graduation demonstrates wisdom-as-load-bearing-primitive — the first protocol surface where elohim inference shapes a canonical outcome instead of observing one. Subsequent appraisal-bearing surfaces (recovery, restitution, mediation) inherit the pattern.
- After **Wave F** lands, federation-shape sampling is on the table. Cross-commons inter-participation patterns compose against the same bridge.

### Anti-patterns the substrate prevents

The bridge's existence makes certain ad-hoc shapes unattractive. Naming them here so future plan reviewers can spot drift:

- **`isAnonymous: boolean` branching.** Replaced by typed `SessionLifecycle` matching. A reviewer who sees `isAnonymous` in new code asks: why isn't this branching on `SessionLifecycle`?
- **Pillar-local pre-canonical storage tables.** Replaced by the bridge's `staged_intent` surface. A reviewer who sees a new "draft" or "pending" table in a pillar's diesel migrations asks: why isn't this a staged intent in the bridge?
- **Direct OAuth → DHT write paths.** Replaced by promote-to-oauth → graduate. A reviewer who sees an OAuth callback writing canonical entries asks: where's the graduation ceremony?
- **Hardcoded "visitor mode" UX branches.** Replaced by lifecycle-aware components binding via the Capability Profile standings axis (Q15). A reviewer who sees `if (visitor) { ... } else { ... }` asks: why isn't this rendering through the profile?
- **One-shot OAuth-as-incarnation flows.** Replaced by the two-step anonymous → oauth → peer-native gradient. A reviewer who sees a flow that conflates OAuth-identifying with peer-native-incarnating asks: which step is missing?

The substrate is most valuable when its existence reshapes what looks like correct code. The session-bridge is designed for that.

---

## §8 — References

- Spec: `genesis/docs/superpowers/specs/2026-05-28-session-bridge-design.md`
- Sister plan: `genesis/docs/superpowers/plans/2026-05-28-thin-client-backend-migration.md` — task M-AGGR-2's deferred deletion is what this plan closes
- P2P Design Gate: `.claude/skills/p2p-design-gate/SKILL.md`
- **SDK canon (§1.6 placement basis):** `genesis/docs/architecture/elohim-sdk.md` — five-library SDK boundary
- **Stewardship canon (§1.9 alignment):** `genesis/docs/architecture/stewardship-over-sovereignty.md`
- **Cradle-to-grave canon (§1.9 alignment + Q16 extension):** `genesis/docs/architecture/cradle-to-grave-capability-gradient.md`
- **REA primitive canon:** `genesis/docs/architecture/rea-compute-commitment-primitive.md`
- **App-manifest schema (§1.7 extension target):** `elohim/sdk/schemas/v1/manifest/app-manifest.schema.json`
- **Per-pillar manifests (§1.7 + per-pillar tickets):** `elohim/sdk/domains/{lamad,imagodei,qahal,shefa,mishpat,avodah,infrastructure,elohim}/manifest.json`
- **Capability Profile spec (Q15 extension target):** `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`
- Schema conventions: `elohim/sdk/schemas/v1/views/CONVENTIONS.md`
- Protocol schemas CLAUDE: `elohim/sdk/schemas/CLAUDE.md`
- Domain manifest CLAUDE (canonical pattern for B-PILLAR manifest updates): `elohim/sdk/domains/lamad/CLAUDE.md`
- Bridges convention (why this is NOT in `bridges/`): `bridges/CLAUDE.md`
- Manifest entry type + whitelisted kinds (B-MANIFEST + B-APPRAISE): `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs`
- Library scope discipline (thin-client invariant for B-CONSUMERS): `app/elohim-library/CLAUDE.md`
- elohim-identity library (B-DOORWAY landing target): `app/elohim-library/projects/elohim-identity/`
- Lamad reference client conventions: `app/lamad/CLAUDE.md`
- Memory references:
  - `project_qahal_graduated_capability_surface` — the graduated capability shape this primitive slots beneath
  - `project_account_layer_oauth_graduation` — refined here as two-step graduation
  - `project_peer_native_account_canonical_surface` — peer-native steward via account management
  - `project_recovery_grandma_standard` — heavy account ceremony, ambient onboarding
  - `project_elohim_as_counsel` — appraisal as one move wisdom makes (B-APPRAISE Q7)
  - `project_commons_elohim_co_steward` — one of the three appraiser candidates (B-APPRAISE Q7)
  - `project_forgetting_as_design` — discard semantics for sampling sessions
  - `project_doorway_full_facilitator_sprint` — doorway as the web2-facing surface (B-DOORWAY)
  - `project_doorway_is_federation_surface_atproto` — bridge pattern at doorway
  - `project_imagodei_three_surfaces` — identity surface decomposition
  - `project_socially_derived_security` — why account incarnation is heavy by design
  - `project_quilt_pantry_vocabulary` — sampling-cache slice storage (B-STORAGE)
  - `feedback_subagent_silent_impl_drops` — impl-count discipline at ticket close
  - `feedback_diesel_migration_timestamp_collision` — migration timestamp discipline
  - `feedback_sweettest_cross_agent_consistency` — sweettest discipline for cross-agent flows
  - `feedback_multi_agent_pvc_pacing` — watchdog: per-crate builds in ticket phases
  - `feedback_session_orchestrate_vs_implement` — sequencing discipline
- M-REA-1 commits (intent surface shared by `StagedEconomicEventIntent`): `aece1093c` → `8cc0b759f`
- M-AGGR-2 commits (read-side cutover; this plan completes the write-side): `6e184ef96` → `6c1fde7bc`
- Spec commit on `sprint/cross-pillar-cleanup`: `9a0c55a61`
