---
id: thin-client-backend-migration
status: Draft
cites:
  - 2026-05-25-cross-pillar-import-cleanup.md   # the related doc this derives from
---

# Thin-Client Backend-Migration Plan

> **Plan status:** Audit-driven backlog. Each ticket below is a discrete backend-migration that removes client-side substrate-policy or substrate-orchestration code.
>
> **Canon basis:** `genesis/docs/architecture/pillar-bundle-split-runbook.md` §6.14; `app/elohim-elements/CLAUDE.md`; `app/elohim-library/CLAUDE.md`.
>
> **Audit basis:** Sweep of `app/elohim-elements/*/src` + `app/elohim-library/projects/*/src` for §6.14 anti-patterns (post-commit `6494969fc`).
>
> **P2P Design Gate output:** §1.5 below — every entity classified before any HTTP route or storage projection is proposed. The substrate is far more designed than the client-side smells suggest; most of this plan is *projecting existing entry types into views*, NOT creating new entry types.

---

## §1 — Why this plan exists

The cross-pillar import cleanup sprint surfaced one stateful-orchestrator anti-pattern (the Slice 2.2b deferral) and named the discipline. A follow-up audit revealed the pattern is **systemic** — the protocol has accumulated substantial client-side substrate-policy code that should live in the substrate.

This is the breadcrumb trail. Each finding below is a TODO for the backend.

The thin-client discipline (legitimate client-side scope): **UX + accessibility + sense-and-respond.** Everything else in the client is a smell.

---

## §1.5 — P2P Design Gate (mandatory before tickets)

Before any ticket below was scoped, each entity was walked through `.claude/skills/p2p-design-gate/SKILL.md`. The substrate has substantially more design than my initial framing suggested:

### Existing substrate primitives (entry types confirmed in DNAs)

| Entry type | DNA / zome | What it covers |
|---|---|---|
| `Manifest` (kinds: `pillar-projection`, `standing-policy`, `tending-policy`, `onboarding`, `app`) | elohim / content_store_integrity | **Substrate-published policy** — the kind/payload_json substrate that policy tickets land into. |
| `FeedbackSignal` | elohim / content_store_integrity | Graduated governance signals (squelch / correction / retraction / quarantine / vouch). |
| `AttentionTending` (`visibility = "private"`) | elohim / content_store_integrity | **Already private source-chain attention entry**, never gossiped to DHT, TTL-bounded, classification-tagged. T16 aggregates post-k-anonymity. |
| `CollectiveFilterPattern` | elohim / content_store_integrity | **k-anonymous aggregate** of private AttentionTending entries — substrate already publishes the privacy-preserving aggregation. |
| `ContentMastery`, `HumanProgress`, `PracticePool`, `MasteryChallenge`, `LearningPath`, `PathChapter`, `PathStep`, `Content` | elohim / content_store_integrity | The lamad learning-state substrate — `SessionHumanService` aggregates are derivable from these. |
| `LearningSignal`, `MediationLog` | elohim / content_store_integrity | Cross-cutting governance / mediation signals. |
| `GovernanceState`, `GraduatedFeedback`, `Precedent`, `Discussion`, `OpinionStatement`, `ChallengeOutcome` | mishpat / mishpat_integrity | Governance state surface — `MechanismSelectionService` derivable from this. |
| `Commitment` (with `delegates-compute` action) | mishpat / mishpat_integrity | REA commitment primitive — recognition-weight wiring lands here. |
| Source chain entries (Holochain native) | imagodei DNA | The 595-line `LocalSourceChainService` localStorage simulation has a real DHT-backed source already. |

### Entities each migration ticket touches — gate output

| Ticket | Entity name | Classification | Source of truth | DHT entry type needed | Storage projection needed |
|---|---|---|---|---|---|
| M-POLICY-1 | AccumulationStatus view | **Category C** (operational projection) | Computed from `FeedbackSignal` entries + Manifest `standing-policy` payload. | None — uses existing `FeedbackSignal`. | YES — new projection (table or materialized view) derived from FeedbackSignal aggregate × Manifest payload. |
| M-POLICY-1 | Accumulation threshold policy | **Category A** (Notarized) | `Manifest{kind: "standing-policy", payload_json: {accumulation_thresholds: {...}}}` | None — uses existing `Manifest` entry type. | Existing manifest projection table. |
| M-POLICY-2 | MechanismSelection view | **Category C** (operational projection) | Computed from `GovernanceState` × `Content` (contentType field) × `Proposal` (if active) × Manifest `pillar-projection` payload (qahal). | None — uses existing entries. | YES — new projection. |
| M-POLICY-2 | Mechanism-selection rule policy | **Category A** | `Manifest{kind: "pillar-projection", pillar: "qahal", payload_json: {mechanism_ladder: {...}}}` | None — uses existing. | Existing. |
| M-POLICY-3 | Recognition weight policy | **Category A** | `Manifest{kind: "standing-policy", payload_json: {recognition_weight_by_level: {...}}}` | None — uses existing. | Existing. |
| M-POLICY-4 | Dwell-time qualification policy | **Category A** | `Manifest{kind: "tending-policy", payload_json: {dwell_qualification_ms: 3000, ...}}` | None — `tending-policy` is an EXISTING manifest kind. | Existing. |
| M-REA-1 | LamadEventIntent (wire shape, not entity) | n/a — request body for coordinator-mediated EconomicEvent creation | — | None. | None (substrate composes the existing EconomicEvent shape from intent). |
| M-REA-1 | EconomicEvent | **Category A** (existing) | elohim DNA (`EconomicEvent` entry type already on the DNA). | None — already exists. | Already exists. |
| M-REA-2 | AttentionTending entry | **Category B (Agent-Scoped, private source chain)** | imagodei agent source chain — **ENTRY TYPE ALREADY EXISTS** with `visibility = "private"`. | None — already there. | Already there (agent-scoped). |
| M-REA-2 | CollectiveAttentionAggregate | **Category A** (already exists as `CollectiveFilterPattern`) | elohim DNA, T16 aggregator post-k-anonymity. | None — already there. | Already there. |
| M-REA-3 | Recognition-given EconomicEvent | **Category A** (existing) | elohim DNA `EconomicEvent`. | None. | Already there. |
| M-AGGR-1 | SessionHumanView | **Category C** (projection) | Derived from agent's `EconomicEvent` stream + `HumanProgress` + `ContentMastery` + `Human`. | None — all entry types exist. | YES — new projection. |
| M-AGGR-1 | UpgradePromptView | **Category C** | Derived from SessionHumanView × Manifest `onboarding` policy (existing manifest kind). | None. | YES — new projection. |
| M-AGGR-2 | SourceChainEntryView, EntryLinkView | **Category A / A2** (Notarized / Derived) | Native Holochain source chain (imagodei DNA — entry types built into the protocol). | None — DNA-native. | Already there (substrate has source chain access). |
| M-AGGR-3 | ContentEngagementStatsView | **Category C** | Derived from `EconomicEvent` stream filtered by content_id + lamadEventType. | None. | YES — new projection. |
| M-ELEM-1 | AuthorityResolution | n/a — view consumed by element, derived from doorway's existing `/auth/me` | — | None. | Already exists. |

### Anti-pattern check — confirmed none of these apply to the proposed work

- ✗ UUID primary key for notarized entity — none proposed; the operational projection tables use `dht_anchor_hash` (existing convention) to reference the source-of-truth EntryHash.
- ✗ REST route as design starting point — every ticket starts with entry type + projection + Manifest payload before naming the route.
- ✗ CID stored as relational FK — none.
- ✗ Standalone table for agent state — M-REA-2 explicitly uses the existing private `AttentionTending` entry instead of inventing a server-side dedup table.
- ✗ Three address formats undefined — every reference resolves through existing entries (EntryHash / CID / agent pubkey as already established in substrate).
- ✗ Missing source-of-truth declaration — every entity above declares its source explicitly.
- ✗ Creating new entry type when one exists — **explicitly avoided**. NO new entry types proposed. Every "new view" is a projection of EXISTING entries.
- ✗ Putting granular data on DHT — M-REA-2 keeps attention private; aggregation via existing `CollectiveFilterPattern`.

### Design constraints discovered

1. **Manifest is the policy substrate.** All four policy tickets (M-POLICY-1..4) reduce to *authoring a Manifest entry with the correct kind + payload_json and writing a storage projection that reads it*. No new entry types needed. The kinds (`standing-policy`, `tending-policy`, `pillar-projection`, `onboarding`) are already whitelisted in the `MANIFEST_KINDS` constant at `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/manifest.rs:38`.
2. **AttentionTending is already the substrate-correct attention entry.** The client `AttentionTrackerService` should be writing AttentionTending entries to the agent's private source chain, not orchestrating server-side dedup. T16 already handles the aggregation across agents via k-anonymity.
3. **REA EconomicEvent composition belongs in the elohim DNA coordinator** — already has handlers; the client should send intent and let the substrate compose the action/provider/receiver shape from substrate-known `PROTOCOL_EVENT_MAPPINGS` (already in codegen-shared substrate vocabulary).
4. **No DHT capacity pressure.** All work uses existing entry types. Lamad ~73/~100, Mishpat 11/~100 headroom is preserved.

---

## §2 — Audit findings (organized by category)

### §2.1 — Substrate policy as client constants

The substrate is the steward of policy. Hardcoded thresholds in client code make every consumer's interpretation drift independently.

| # | Smell location | Hardcoded policy | Substrate-correct home |
|---|---|---|---|
| **F-POLICY-1** | `app/elohim-library/projects/elohim-service/src/angular/services/signal-accumulation.service.ts:36-38` | `totalSignals >= 20 && consensusStrength < 0.7` (ready-for-sensemaking); `>= 10 && < 0.3` (controversy); `>= 30 && >= 0.85` (settled) | `Manifest{kind: "standing-policy"}` carries thresholds; storage projection derives `AccumulationStatus` from FeedbackSignal aggregate × policy. |
| **F-POLICY-2** | `app/elohim-library/projects/elohim-service/src/angular/services/mechanism-selection.service.ts:41-59` | `SETTLED_STATES`, `MECHANISM_LEVEL_MAP`, `FEEDBACK_CONTENT_TYPES` | `Manifest{kind: "pillar-projection", pillar: "qahal"}` carries the mechanism ladder; projection derives `MechanismSelection` from `(GovernanceState, Content.contentType, Proposal?)`. |
| **F-POLICY-3** | `app/elohim-app/src/app/qahal/services/governance-recognition.service.ts:37` | `WEIGHT_BY_LEVEL` — recognition weight per mechanism level | `Manifest{kind: "standing-policy"}` carries the weight curve; mishpat `Commitment` coordinator reads it to compose the EconomicEvent. |
| **F-POLICY-4** | `app/elohim-library/projects/elohim-rea-runtime/src/lib/attention-tracker.service.ts:23` | `DWELL_THRESHOLD_MS = 3000` | `Manifest{kind: "tending-policy"}` (existing kind) carries the dwell qualification; substrate evaluates via existing `AttentionTending` flow. |

### §2.2 — REA event composition client-side

| # | Smell location | Pattern | Substrate-correct home |
|---|---|---|---|
| **F-REA-1** | `app/elohim-library/projects/elohim-rea-runtime/src/lib/event.service.ts:123-265` (11 record* methods) | Methods compose `{action: REAActions.PRODUCE, provider, receiver, lamadEventType, ...}` shape client-side then POST to `createEconomicEvent`. | Doorway route accepts a high-level `LamadEventIntent` discriminated union; elohim DNA coordinator composes the EconomicEvent (already an entry type) from intent using substrate-known `PROTOCOL_EVENT_MAPPINGS`. Compare `SignalEmitService` — *good* shape, single POST. |
| **F-REA-2** | `app/elohim-library/projects/elohim-rea-runtime/src/lib/attention-tracker.service.ts:79-130` | Owns `sessionViewed` dedup `Set`, dwell-timer orchestration, calls `eventService.recordContentInteraction()`. | Client writes `AttentionTending` (existing private source-chain entry) directly via coordinator; substrate's existing T16 aggregator handles privacy-preserving aggregation via existing `CollectiveFilterPattern`. The whole `sessionViewed` Set and dwell timer retire. |
| **F-REA-3** | `app/elohim-app/src/app/qahal/services/governance-recognition.service.ts:61-78` (`recordParticipation`) | Composes `RecognitionTrigger`, applies `WEIGHT_BY_LEVEL` (F-POLICY-3) client-side, calls `recognitionApi.distribute()`. | Mishpat `Commitment` coordinator (existing entry type) accepts `{entityType, entityId, humanId, mechanismLevel, participationType}`; reads the standing-policy Manifest, composes the EconomicEvent. |

### §2.3 — Aggregated state computed client-side

| # | Smell location | Pattern | Substrate-correct home |
|---|---|---|---|
| **F-AGGR-1** | `app/elohim-library/projects/elohim-identity/src/lib/session-human.service.ts` (1100 lines; 13+ record* methods) | Local stats accumulated via `incrementStat()`; localStorage activity history; upgrade-prompt triggers based on local state. | `SessionHumanView` (Category C) — storage projection derives stats from agent's `EconomicEvent` / `ContentMastery` / `HumanProgress` (all existing entries). `UpgradePromptView` (Category C) — projection from `SessionHumanView` × `Manifest{kind: "onboarding"}` (existing manifest kind). `record*` methods become thin intent emitters that hit M-REA-1. |
| **F-AGGR-2** | `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts` (595 lines) | Full Holochain source chain simulation in localStorage. Docstring acknowledges: *"When Holochain is ready, swap this service for HolochainSourceChainService"*. | Use the real Holochain source chain via imagodei DNA. Entries already exist DNA-native; client subscribes to substrate-published source-chain views. |
| **F-AGGR-3** | `app/elohim-library/projects/elohim-rea-runtime/src/lib/event.service.ts:335-370` | `countContentViews`, `countContentCompletions` — query EconomicEvents and count client-side. | `ContentEngagementStatsView` (Category C) — projection from `EconomicEvent` filtered by `content_id` + `lamadEventType`. Existing entries. |

### §2.4 — Lit element doing API calls

| # | Smell location | Pattern | Substrate-correct home |
|---|---|---|---|
| **F-ELEM-1** | `app/elohim-elements/elohim-imagodei/src/elohim-imagodei-portal-shell.ts:186` | `await fetch(this.authorityEndpoint, { credentials: 'include' })` directly inside `_discoverAuthority()` | Host pre-fetches `/auth/me` (already a doorway endpoint, no substrate work); binds `authority: AuthorityResolution \| null` to element `@property`. Element renders given state. |

### §2.5 — Helpers that ARE legitimate (do NOT migrate)

- CLI content-pipeline scoring (`relationship-extractor.service.ts`) — batch-extraction, not runtime substrate policy
- Thin HTTP wrappers (`distribution.service.ts`, `resilience.service.ts`, `observation.service.ts`, `governance-api.service.ts`, `doorway-client.service.ts`)
- `SignalEmitService` — the *good* shape: single POST, substrate composes
- Chaperone signing-keypair generation (`doorway-connection-strategy.ts`) — substrate-by-design (Holochain requires per-client signing creds)
- Diagram components in `lamad-ui` — pure visualization
- All Lit elements in `elohim-elements/*/src` EXCEPT portal-shell

---

## §3 — Migration tickets (substrate-first sequencing)

Every ticket follows the design order from `.claude/skills/p2p-design-gate/SKILL.md` Step 3: **(a) Coordinator function → (b) Post-commit signal + storage projection → (c) HTTP route LAST**.

### Ticket M-POLICY-1: Server-side AccumulationStatus

**Smell:** F-POLICY-1.

**Step 3a — Coordinator zome:** No new function. The accumulation-threshold policy is authored as a `Manifest{kind: "standing-policy"}` entry using the *existing* `create_manifest` coordinator function (elohim DNA `content_store` zome). The payload_json schema for `standing-policy` is extended to include an `accumulation_thresholds` object (`{readyForSensemaking: {minTotalSignals, maxConsensusStrength}, controversyDetected: {...}, settled: {...}}`).

**Step 3b — Post-commit signal + projection:**
- `Signal::ManifestUpdated{kind: "standing-policy"}` already fires (existing manifest signal). Storage handler reads the payload and caches the threshold values.
- New projection: storage-side computation joins the agent-level `FeedbackSignal` aggregate (already projected) with the standing-policy thresholds and emits an `AccumulationStatusView` per `(entityType, entityId)`. The projection writes to a new SQLite materialized-view table `accumulation_status` (operational — `-- Source of truth: derived projection of FeedbackSignal × Manifest`).

**Step 3c — HTTP route:** `GET /api/v1/governance/{entityType}/{entityId}/accumulation` returns `AccumulationStatusView`. Doorway adds the route; nothing else changes at the HTTP layer.

**Schema artifact:** `elohim/sdk/schemas/v1/views/accumulation-status.schema.json` — new view schema. Rust struct in `elohim-storage/src/views.rs` with `#[derive(TS)]`. Schema contract test added.

**Client cleanup (post-substrate):**
- Delete `app/elohim-library/projects/elohim-service/src/angular/services/signal-accumulation.service.ts` entirely.
- Update consumer (`feedback-mechanism-gateway.component.ts`) to read `AccumulationStatusView` via `GovernanceApiService.getAccumulationStatus(entityType, entityId)`.

**Dependencies:** None.

---

### Ticket M-POLICY-2: Server-side MechanismSelection

**Smell:** F-POLICY-2.

**Step 3a (DHT entry type + coordinator function):** Author `Manifest{kind: "pillar-projection", pillar: "qahal"}` entry whose payload_json carries the mechanism ladder (settled-states list, mechanism-level map, feedback-inviting content types). Uses the *existing* `create_manifest` coordinator function (elohim DNA content_store zome — no new entry type).

**Step 3b (post-commit signal + storage projection):** Storage projection joins `GovernanceState` (mishpat zome) × `Content` (elohim zome) × `Proposal?` (mishpat zome) with the pillar-projection Manifest. Emits `MechanismSelectionView` per `(entityType, entityId)`. New materialized-view table `mechanism_selection`.

**Step 3c (HTTP route, last):** `GET /api/v1/governance/{entityType}/{entityId}/mechanism` — serves the projection of (`GovernanceState` × `Content` × `Proposal?` × `Manifest`{kind:"pillar-projection"}) DHT entry type composition as `MechanismSelectionView`. No source-of-truth at the HTTP layer.

**Schema artifact:** `elohim/sdk/schemas/v1/views/mechanism-selection.schema.json`.

**Client cleanup:**
- Delete `app/elohim-library/projects/elohim-service/src/angular/services/mechanism-selection.service.ts`.
- Update `feedback-mechanism-gateway.component.ts` consumer.
- Update `qahal/services/index.ts` re-export.

**Dependencies:** None. Parallel to M-POLICY-1.

---

### Ticket M-REA-1: Substrate-side EconomicEvent composition

**Smell:** F-REA-1.

**Step 3a:** Extend the elohim DNA `content_store` zome's existing `create_economic_event` coordinator (or add `create_economic_event_from_intent` if the existing function takes the fully-composed shape) to accept a high-level `LamadEventIntent` discriminated union. Substrate composes the EconomicEvent (existing entry type — Category A) using substrate-known `PROTOCOL_EVENT_MAPPINGS` (already in the codegen-shared protocol-event-types schema). Validates and notarizes.

**Step 3b:** No new entry. The existing `Signal::EconomicEventCreated` post-commit hook already projects to elohim-storage. No new table needed — the existing `economic_events` projection serves.

**Step 3c:** Doorway route `POST /api/v1/lamad/events` accepts `LamadEventIntent`, calls the coordinator, returns the projected `EconomicEventView`.

**Schema artifact:** `elohim/sdk/schemas/v1/intents/lamad-event-intent.schema.json` — *intent* schema (not a view; describes the request body). The view side already exists.

**Client cleanup:**
- Reduce `event.service.ts` to a single `emitEvent(intent: LamadEventIntent): Observable<EconomicEventView>` method.
- Delete 11 of the 12 record* methods. The deprecated `recordContentView` / `recordContentComplete` shims can either be deleted or thinned to call `emitEvent`.
- Update consumers (AttentionTracker, agent.service in elohim-app, others) to pass intents.

**Dependencies:** None for substrate. Client cleanup blocked on the doorway route landing.

---

### Ticket M-REA-2: Native AttentionTending writes

**Smell:** F-REA-2 + F-POLICY-4.

**Step 3a:** *No new coordinator function needed.* `create_attention_tending` already exists at `elohim/holochain/dna/elohim/zomes/content_store/src/attention_tending.rs`. The DWELL qualification policy lands as `Manifest{kind: "tending-policy"}` (existing kind!) using the existing `create_manifest` coordinator.

**Step 3b:** No new projection table needed — `AttentionTending` is `visibility = "private"`, lives on agent source chain, never gossiped. T16 already produces `CollectiveFilterPattern` aggregates post-k-anonymity (existing entry type + flow). The substrate is already designed for this.

**Step 3c:** `POST /api/v1/attention/tending` doorway route accepts `CreateAttentionTendingInput` and proxies to the coordinator. Browser flows that don't yet have direct Holochain access use this proxy; Tauri/conductor-direct flows call the coordinator natively.

**Schema artifact:** `elohim/sdk/schemas/v1/intents/attention-tending-intent.schema.json` (already aligned with existing `CreateAttentionTendingInput` Rust type).

**Client cleanup:**
- Reduce `AttentionTrackerService` to: capture mount + elapsed time, POST AttentionTending input via the doorway proxy. ~30 lines, not 140.
- Delete `sessionViewed` Set (substrate already dedupes — re-tending updates existing entry per the docstring at `attention_tending.rs:30-34`).
- Delete `DWELL_THRESHOLD_MS` constant (substrate policy via Manifest).
- Delete dwell-timer orchestration (substrate qualifies).
- Eliminate the dependency on `event.service.ts:recordContentInteraction` — content-view tracking IS AttentionTending, not a separate EconomicEvent.

**Dependencies:** None — the substrate already has everything. Add doorway proxy route + author tending-policy Manifest, then client cleanup.

**Key insight:** This ticket is the most-mis-framed in my initial plan draft. I was inventing a "server-side dedup" pattern when the substrate had already designed exactly the right shape: private source-chain entries + k-anonymous aggregation post-T16. The client is currently bypassing the designed shape entirely.

---

### Ticket M-REA-3: Substrate-side governance recognition

**Smell:** F-REA-3 + F-POLICY-3.

**Step 3a:** Recognition-weight policy lands as `Manifest{kind: "standing-policy"}` with a `recognition_weight_by_level` payload. Mishpat `Commitment` coordinator extends to accept `{entityType, entityId, humanId, mechanismLevel, participationType}` — reads the standing-policy Manifest, composes the EconomicEvent shape, calls the existing `create_economic_event` coordinator.

**Step 3b:** No new entry types. Existing `Commitment` post-commit projection + `EconomicEvent` projection already cover this.

**Step 3c:** Doorway route `POST /api/v1/mishpat/recognition/participation` accepts the intent, calls the coordinator.

**Schema artifact:** `elohim/sdk/schemas/v1/intents/recognition-participation-intent.schema.json`.

**Client cleanup:**
- Delete `app/elohim-app/src/app/qahal/services/governance-recognition.service.ts` entirely.
- Delete `RecognitionApiService.distribute()` if only used here (audit before deleting — likely only called from governance-recognition).
- Update `reaction-bar.component.ts` + `graduated-feedback.component.ts` to POST to the new doorway route.

**Dependencies:** M-POLICY-3 substrate policy publication (folded into this ticket since both author the same standing-policy Manifest).

**Note:** This ticket plus M-POLICY-1 + M-POLICY-2 together close the original Slice 2.2b deferral. Once all three land, the three remaining `@app/qahal/*` imports in `content-viewer.component.ts` retire — the Lit swap becomes trivially clean per §6.14.

---

### Ticket M-AGGR-1: SessionHumanView projection

**Smell:** F-AGGR-1.

**Step 3a:** *No new coordinator function.* All inputs are existing entries: agent's `EconomicEvent` stream, `ContentMastery` per-content records, `HumanProgress` per-agent state, `Human` identity.

**Step 3b:** New projection in elohim-storage. Aggregates per-agent counts from `EconomicEvent` filtered by `lamadEventType` (existing field): `nodesViewed`, `nodesWithAffinity`, `pathsStarted`, `pathsCompleted`, `stepsCompleted`. Joins with `HumanProgress` for `journeyStartedAt` etc. Emits `SessionHumanView` (Category C — operational projection of substrate truth). New materialized-view table `session_human_view` (`-- Source of truth: derived projection of EconomicEvent stream × HumanProgress`).

A second projection for `UpgradePromptView`: reads `Manifest{kind: "onboarding"}` (existing manifest kind!) to know which prompts exist + their trigger conditions; joins with SessionHumanView to determine which are active. Emits `UpgradePromptView`.

**Step 3c:** `GET /api/v1/identity/{agentId}/session` returns `SessionHumanView`. `GET /api/v1/identity/{agentId}/upgrade-prompts` returns `UpgradePromptView`.

**Schema artifact:** `session-human-view.schema.json` + `upgrade-prompt-view.schema.json`.

**Client cleanup:**
- Reduce `SessionHumanService` from 1100 lines to ~200 lines: session-identity (sessionId, accessLevel, isAnonymous) only; subscribes to the two views; emits intent via M-REA-1.
- Delete 13 `record*` methods (substrate has the events via M-REA-1).
- Delete `incrementStat`, `triggerUpgradePrompt`, localStorage-as-stats-storage.

**Dependencies:** M-REA-1 (REA events must flow through substrate for projections to derive views).

---

### Ticket M-AGGR-2: Holochain source chain cutover

**Smell:** F-AGGR-2.

**Step 3a:** *Substrate is done.* Holochain native source chain in imagodei DNA. The 595-line `LocalSourceChainService` simulation was always meant to retire — its docstring at line 24 says so.

**Step 3b:** Read paths from substrate via doorway: `GET /api/v1/source-chain/{agentId}/entries`, `GET /api/v1/source-chain/{agentId}/links`. The corresponding `SourceChainEntryView` + `EntryLinkView` shapes get codegen'd from the DNA. The chain-metadata is read from the agent's Holochain context.

**Step 3c:** Doorway routes as above. Writes go via the agent's local coordinator (Tauri direct) or proxied through doorway (browser).

**Schema artifact:** `source-chain-entry-view.schema.json`, `entry-link-view.schema.json`.

**Client cleanup:**
- Delete `app/elohim-library/projects/elohim-service/src/angular/services/local-source-chain.service.ts` (all 595 lines).
- Replace with thin `HolochainSourceChainService` (HTTP wrapper around doorway routes).
- Delete `prepareMigration()` packaging code — entries are in DHT from creation, no migration needed.

**Dependencies:** Imagodei DNA source-chain doorway routes. Coordinate with `rust-architect`.

---

### Ticket M-AGGR-3: ContentEngagementStatsView projection

**Smell:** F-AGGR-3.

**Step 3a:** No new coordinator. Existing `EconomicEvent` entries provide the data.

**Step 3b:** Projection in elohim-storage joins `EconomicEvent` filtered by `(content_id, lamadEventType IN ('content-view', 'content-complete'))` and groups by content_id. Emits `ContentEngagementStatsView` per content. New materialized-view table.

**Step 3c:** `GET /api/v1/lamad/content/{contentId}/engagement` — serves the projection of `EconomicEvent` entries (filtered + grouped) as `ContentEngagementStatsView`. No source-of-truth at the HTTP layer; `EconomicEvent` entry type already lives in elohim DNA.

**Schema artifact:** `content-engagement-stats-view.schema.json`.

**Client cleanup:**
- Delete `event.service.ts:countContentViews`, `countContentCompletions`, `countEventsForContent`.
- Consumers query the view directly.

**Dependencies:** M-REA-1.

---

### Ticket M-ELEM-1: Portal-shell host-pre-fetch refactor

**Smell:** F-ELEM-1.

**Substrate work:** None. `/auth/me` is already a doorway endpoint.

**Client cleanup:**
- Remove `_discoverAuthority()` from `elohim-imagodei-portal-shell.ts` (lines 184-203).
- Add `@property() authority: AuthorityResolution | null = null`.
- Add `@fires authority-needed` event for hosts that haven't pre-fetched.
- Hosts using portal-shell (`app/imagodei-portal/`, anywhere else) pre-fetch and bind.
- Update story fixtures to provide `authority` directly.

**Dependencies:** None — pure client-side refactor.

---

## §4 — Sequencing & waves

### Wave A — Foundational substrate (parallelizable)

These have no client-cleanup dependency on each other:

- **M-REA-1** — intent-to-EconomicEvent surface (KEYSTONE — most other tickets depend on it)
- **M-POLICY-1** — AccumulationStatus projection + standing-policy Manifest
- **M-POLICY-2** — MechanismSelection projection + pillar-projection Manifest
- **M-REA-2** — Native AttentionTending writes + tending-policy Manifest (independent — substrate already has everything)
- **M-ELEM-1** — Portal-shell cleanup (client-side only)

### Wave B — EconomicEvent-derived projections

Land after M-REA-1's EconomicEvent surface is server-composed:

- **M-REA-3** — Governance recognition (consumes M-REA-1 + M-POLICY-3 Manifest)
- **M-AGGR-3** — Content engagement stats (consumes M-REA-1)

### Wave C — Identity + source chain

- **M-AGGR-1** — Server-side session/journey state (consumes M-REA-1)
- **M-AGGR-2** — Holochain source-chain cutover (independent substrate effort)

### Wave D — Slice 2.2b closure

After M-POLICY-1 + M-POLICY-2 + M-REA-3:

- Retire 3 `@app/qahal/*` imports in `content-viewer.component.ts`
- Swap to `<elohim-feedback-mechanism-gateway>` / `<elohim-graduated-feedback>` / `<elohim-reaction-bar>` Lit elements
- Update content-viewer.component.spec.ts to mock the substrate views (not the orchestration services)
- Delete the 2 already-migrated qahal services from library + the original `governance-recognition.service.ts`
- §6.14 stateful-orchestrator-deferral comment retires

---

## §5 — Acceptance criteria (per ticket)

A ticket closes when:

1. **P2P Design Gate output recorded** — entity classification, source of truth, content-address strategy, anti-pattern check (the §1.5 table above for all tickets in this plan).
2. **Schema** — new view (or intent) schema lands in `elohim/sdk/schemas/v1/views/` or `.../intents/`; conventions per `.../views/CONVENTIONS.md`.
3. **Manifest payload schema (policy tickets only)** — if the ticket authors a Manifest entry, the `payload_json` shape gets its own schema at `elohim/sdk/schemas/v1/manifest-payloads/<kind>.schema.json`.
4. **Rust struct** in elohim-storage with `#[serde(rename_all = "camelCase")]` + `#[derive(TS)]`.
5. **Schema contract test** in `elohim/elohim-storage/tests/schema_contract.rs`.
6. **TS codegen** — `pnpm run schema:codegen:ts` regenerates types in `@elohim/storage-client`.
7. **Coordinator extension (if any)** — new coordinator function or extension of existing; integrity-zome validation updated; sweettest two-conductor seatbelt added if substrate write is new.
8. **Storage projection** — projection writer in elohim-storage; the materialized-view table carries `-- Source of truth: derived projection of <inputs>` comment.
9. **Doorway route** — added to `doorway/doorway-service/src/routes/`; route serves the projection (not the source); tests added.
10. **Client cleanup** — smell file deleted or thinned per ticket; tests updated; both bundles build green.
11. **A2o scenario** added under `genesis/a2o/features/<pillar>/` if user-visible behavior changed.

---

## §6 — What changed from initial draft (operator correction)

The first draft of this plan was framed in HTTP routes + storage projections without first asking the DHT-entry-type questions. The P2P Design Gate (`.claude/skills/p2p-design-gate/SKILL.md`) caught this — see the hook output flagging 10+ "API route without entry type" and "new storage schema without source-of-truth declaration" issues.

The rewrite (this version) walked every entity through the gate. The key discoveries that reshaped the plan:

1. **No new DHT entry types needed.** Every "entity" the plan creates is either (a) Category C (operational projection of existing entries) or (b) Category A but using the existing `Manifest` entry type with one of its 5 whitelisted kinds.
2. **AttentionTending and CollectiveFilterPattern already exist.** The M-REA-2 ticket was originally framed as "server-side dedup of public events" — completely wrong. The substrate already designed private source-chain attention entries + k-anonymous aggregation. The client is currently *bypassing the designed shape*.
3. **Manifest is the policy substrate.** All four F-POLICY tickets reduce to "author Manifest{kind: ..., payload_json: ...} + write a projection that reads it." The kinds (`standing-policy`, `tending-policy`, `pillar-projection`, `onboarding`, `app`) are already whitelisted and cover every policy concern in the audit.
4. **Lamad/Mishpat DNA capacity is not under pressure** — but it never would have been because no new entry types are needed.

This pattern — "the client built an orchestration layer before the substrate was ready, and then the substrate WAS designed but the client never cut over" — explains every major finding in this plan. The substrate's design is more complete than the client's consumption of it.

---

## §7 — Why this plan matters beyond the cleanup

The thin-client discipline + substrate-as-steward shape ([stewardship-over-sovereignty](epr:stewardship-over-sovereignty) §3) only holds if it's enforced in the actual codebase. Every ticket above is a place where the substrate currently delegates a steward-shaped responsibility to clients — every client gets it slightly differently, no client can be audited, and "the protocol" effectively means "whichever client renders it." This plan closes the gap.

Future pillar splits inherit a thinner client surface — `shefa`, `qahal`, `avodah`, `imagodei`, `account`, `doorway` (per pillar-EPR design §0) each will have less "we accidentally built backend logic on the client" debt to clean up because the discipline is now documented (CLAUDE.md files + §6.14), tooled (P2P Design Gate skill), AND demonstrated (this plan's substrate-first reading).

---

## §8 — References

- §6.14 of `genesis/docs/architecture/pillar-bundle-split-runbook.md` — the anti-pattern canon
- `app/elohim-elements/CLAUDE.md` — UI-substrate scope discipline
- `app/elohim-library/CLAUDE.md` — library scope discipline
- `genesis/docs/architecture/elohim-sdk.md` — five-library SDK Category-C commitment
- `genesis/docs/architecture/stewardship-over-sovereignty.md` — substrate-as-steward
- `genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md` — sprint that surfaced the pattern
- `.claude/skills/p2p-design-gate/SKILL.md` — the gate this plan runs through
- DNA entry-type sources: `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/{lib.rs,manifest.rs,feedback_signal.rs,attention_tending.rs}`; `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs`
- Audit commit: `6494969fc`
