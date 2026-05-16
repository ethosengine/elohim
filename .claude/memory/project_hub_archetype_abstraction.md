---
name: Hub abstraction — household and collective as separate implementations
description: Hub is an abstract substrate-topology interface; HouseholdHub and CollectiveHub are intentionally separate implementations because governance considerations differ in shape, not in settings. Mirrors the elohim-agent specialization pattern. Household-archetype fixtures are needed to model hub-and-spoke scaling; can start as @wip then add simulation harness.
type: project
originSessionId: 909de5de-3db0-4c88-af4b-12f47dd2762c
---
To model hub-and-spoke scaling rigorously (the inclusion math from `project_substrate_scale_ceiling`), the protocol needs Hub fixtures parallel to device archetypes. Today: device archetypes exist (`genesis/data/devices/devices.json`), collective archetypes exist (`genesis/data/collectives/collectives.json`, governance-shaped), humans have a `householdId` foreign key — but **no first-class household/hub fixture exists**. Conductor-groups (`account-packages/conductor-groups.json`) name buckets "Household" but are flat human-id lists for compute placement, not topology models.

**Architectural shape — Hub is abstract; HouseholdHub and CollectiveHub are intentionally separate implementations.**

Shared `Hub` interface (substrate-level only):
- **stewards** (humans with authority over the hub — never call them "members"), devices (device archetypes), hub-and-spoke topology
- connectivity profile (always-on / intermittent / periodic / offline-first)
- WAN profile (fiber/cable/cellular/satellite/LoRa fallback)
- inclusion role (does this hub carry external spokes?)

**Constitutional rule (Elohim constitution candidate — to be ratified in follow-up brainstorm):** hub hardware MUST be made accessible to its stewards. Access path must be inspectable, modifiable, retrievable, and not depend on a third party who is not a steward. If access is denied or revoked by anyone other than a steward, the stewards retain the capability to **quarantine** (mark unusable, reassign duties, treat data as unrecoverable until access restored) or **evict** (remove from hub composition, halt routing through the device, notarize the eviction). This is a constitutional power, not an operational override — inaccessibility is a violation, not a normal failure mode. (Substrate-not-yet-built: a `hub-topology.feature` under `@constitutional` scenarios is the planned home; @wip until simulation harness lands per the strategy below.)

**Hubs need a stronger encryption story than individual devices — the encryption boundary terminates at the hub↔spoke edge.** A hub is an always-on, physically-present, centralized theft target. If a hub is stolen, you lose data for every steward and every spoke that syncs through it. This makes hub at-rest encryption + key custody load-bearing in a way that device-level OS custody on phones/laptops is not.

Working hypothesis (to ratify in follow-up brainstorm): end-to-end-style encryption runs **peer↔hub**, terminates at the **hub boundary** on the way down to spokes, and **device-level OS custody** takes over on the spoke (the unencrypted-at-rest terminus we're already familiar with — the phone/laptop OS does the work). Hubs sit in a middle ground that needs explicit encryption design:
- At-rest encryption with steward-held keys
- Hardware-key-bound (TPM/secure-enclave on Tier 3) so theft yields ciphertext
- Key recovery via the same steward-quorum flow that handles eviction/quarantine (ties to `imagodei` recovery-seed + intimate-quorum, see `project_socially_derived_security`)
- Eviction probably also rotates keys (open question)
- CollectiveHub may need a different encryption profile than HouseholdHub (institutional vs intimate trust — open question)

Realm-specific implementations:
- **HouseholdHub** — intimate/trusted reach, steward-consent governance, household-economy, custodial-key-hosting for less-technical relatives, family-roles, `domain: "household"`
- **CollectiveHub** — familiar/community reach, community-vote or delegated-consent governance, community-economy or constitutional posture, institutional-role (church/school/center), elected-officer/appointed-steward, `domain: worship/curriculum/economy/...`

**Why intentionally separate, not parametric:** governance considerations don't degrade gracefully. A household's "we sync everything because we trust each other and live together" does not translate to a congregation hub where new spokes need consent to join, content visibility has institutional defaults, and removing a spoke is a community-governance event. Conflating them under one `Hub` type with a `realm` parameter would push governance into config flags and lose realm character. **Same pattern as elohim agents**: `human-elohim`, `household-elohim`, `collective-elohim` are separate specializations precisely because their contracts are different shapes, not different settings.

**Canonical hub archetypes to define (rough sketch — exact list belongs to a brainstorm):**
- Phone-only solo (no hub; hosted-account stage)
- Couple, no hub (sync via doorway or trusted neighbor's hub)
- Young family with one Tier 3 (canonical Stage-4 household)
- Multi-gen household (mixed stages, custodial-keyed grandma)
- Extended family, two hubs (cross-household redundancy)
- Church basement hub (1 family-node-extended + 100 intermittent spokes — the inclusion math made concrete)
- Refugee camp shared hub (constrained bandwidth, satellite/LoRa, offline-first)
- Disaster-mode household (Tier 3 offline, degraded operation)

**Simulation strategy:** real Tier 3 hardware doesn't exist for hub-shaped a2o scenarios. Two-phase approach:
1. **`@wip` first** — declare hub archetypes as fixtures, write scenarios against them, tag unrunnable until substrate + simulation harness lands. Cheap, preserves design intent.
2. **Simulation harness** as follow-up sprint — virtual hub processes (multiple `elohim-storage` instances per machine), simulated spokes (mock peers with declared connectivity profiles), simulated bandwidth/latency/availability per archetype.

**Where it lives (planned, not yet built):** `genesis/data/hubs/hubs.json` parallel to `devices.json`; `HubArchetype` (abstract) + `HouseholdHub` / `CollectiveHub` types in `genesis/a2o/src/framework/fixtures/hubs.ts`; `getHub(name)` accessor parallel to `getDevice(name)`. As of 2026-05-15 none of those files exist — the abstraction is design-staged.

**How to apply:**
- Use **stewards**, not "members," when referring to the humans with authority over a hub. Membership is a passive category; stewardship carries agency, accountability, and constitutional power including eviction.
- When designing fixtures or test scaffolding for hub-and-spoke scaling, never collapse household and collective hubs into one parametric type. Two implementations, shared narrow interface.
- When sketching a hub archetype, declare its governance contract explicitly (consent flow, visibility defaults, spoke-add/remove semantics) — these distinguish it from other hub types.
- Every hub archetype must declare an access path per device, with stewards as the authority — flag any design that defers hardware access to a non-steward third party.
- When designing hub data flows, treat the hub↔spoke boundary as the encryption-boundary edge by default: peer↔hub is end-to-end-style; spoke takes device-level OS custody. Don't accidentally re-introduce always-encrypted-at-rest on spokes (that's the device-OS layer's job) or unencrypted-at-rest on hubs (theft target).
- Hub-shaped a2o scenarios should be `@wip` until simulation harness exists; do not let them block other work or report false-positive passes.
- The Hub abstraction is the natural carrier for the inclusion claims in `hardware-spec.md` — when those claims need to be tested or parameterized, they instantiate as Hub archetypes.
- Mirror the elohim-agent specialization pattern when adding future hub types (e.g., enterprise-hub, civic-hub) — separate implementations, not parameters.
