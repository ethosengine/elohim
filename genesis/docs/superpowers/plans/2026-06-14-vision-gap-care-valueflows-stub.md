---
title: "O2 — Native Observed-Care → REA Valueflow Emitter (vision-gap stub)"
status: GREENLIGHT-TO-EXPAND
kind: vision-gap-stub
objective: O2 — trust-economy as REA ValueFlows of intimate OBSERVED CARE
authored: 2026-06-14
authored_by: rust-architect (vision-gap pass)
gap_source: VISION-ALIGNMENT-2026-06-14.md (completeness-critic verified gap #1)
requires_env: household-nodes
seal: none (working draft — do NOT cite-seal)
cross_plan_edges:
  - consumes: D-DIAGNOSTIC (household-facing read-model) — the care-stream renders alongside the placement-gap felt surface
  - parallels: O3 limit-respect governor (observe→govern loop shares the Observation read side)
  - does-not-collide-with: P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14, FEDERATION-WEB2-LEDGER-2026-06-14
---

# O2 — Native Observed-Care → REA Valueflow Emitter

**GREENLIGHT-TO-EXPAND stub.** Objective + gap + missing bridge + p2p-design-gate answers + existing substrate + first a2o scenario + effort/risk + the decisions only the operator can make. No code, no cluster ops. The value/governance core needs the operator's blessing before expansion.

---

## 1. Objective + the felt promise

O2 asks the protocol to hold a **trust-economy of intimate observed care** as REA ValueFlows: Margaret's 25 hours sitting with her mother, the older child making breakfast for the younger one, the neighbor who drives someone to dialysis — recorded not as money, not as gamified points, but as **economic events with on-chain standing**: "this care happened, it was witnessed, and the family can see it as mutual value." The felt promise: a household opens its surface and sees that care given inside it is *real* in the same ledger that records who stewards the photos and who runs the node — care is first-class economic substance, witnessed by people who were there, never extracted or scored by a platform. The protocol already promises this in its REA economic epic ("REA doesn't ask 'how much money?' It asks 'what actually happened?'"); today the answer for care is *nothing happened*, because there is no native emitter.

---

## 2. Vision-vs-substrate GAP (what's promised vs what the code does)

| Layer | Promised | Today |
|---|---|---|
| DHT entry | EconomicEvent notarizes care | **EXISTS** — `EconomicEvent` entry on elohim/content_store DNA (`content_store_integrity/src/lib.rs:1116`), with `action`, `resource_classified_as_json`, `effort_quantity_*`, `has_duration`, and `substrate_signal` fields. The `work` REA action ("Contribute labor (stewardship, review, curation)", `:243`) and classifications `stewardship` / `care-token` / `time-token` (`:271-289`) are **already in the validated vocabularies.** |
| Coordinator | A fn to notarize a care event | **EXISTS but generic** — `create_rea_economic_event` (`content_store/src/lib.rs:12124`). No care-specific entry path; nothing translates an Observation into one. |
| Native emitter | Observation → care EconomicEvent | **MISSING.** This is the gap. |
| Where care lives today | — | EconomicEvent is *consumed* only by `bridges/valueflows` as a **read fixture** (`schema/economic_event.rs:26` — `EconomicEventGql::fixture`, "M1 tracer-bullet; M3 will return real hREA data"). It is an EPR-REA ↔ VF-GraphQL **translation surface**, never a household-core emitter. |
| Observation substrate | Witnessed evidence primitive | **EXISTS** — `Observation` wire type (`observation/wire.rs:21`) with `observation_kind`, `subject_cid`, `observer_household_cid`, `observer_collective_cid`, `payload_json`, `signature`; append-only per-observer log (`observation/log.rs`); iroh + libp2p planes; `observations` SQL projection. **But no projector reads a care-kind observation and emits an EconomicEvent.** |

**The gap in one sentence:** the protocol has a witnessed-evidence layer (Observation) and a care-capable economic-event layer (EconomicEvent), and **nothing connects them** — care is observable and economically expressible, but never made economic.

---

## 3. The MISSING BRIDGE / primitive (concrete)

A **native observed-care projector**: a coordinator-adjacent service in elohim-storage that consumes a `care` Observation and emits a `work`-action EconomicEvent through the existing conductor path, bounded by an existing Commitment, with `observation_refs` linking the event back to the witnessing observation(s).

**Decision made here (see §8 for the operator's veto): care is NOT a new primitive.** It is an **instantiation of the existing EconomicEvent + Commitment family** — the same "one substrate primitive instantiated across many domains" pattern as `project_rea_compute_commitment_primitive` (deploy / hosting / household chores / moderation / authorship). Care is the *household-chores / caregiving instantiation* of that family. Concretely:

1. **A new Observation kind** `household:care-given` (manifest vocabulary, NOT a new entry type — `observation_kinds` in the pillar manifest, per elohim-storage CLAUDE.md "New observation kinds are declared in pillar manifests"). Payload (`payload_json`): `{ caregiver_cid, recipient_cid, care_kind, effort_hours, note }`.
2. **A native emitter service** `services/care_event_emit_service.rs` — sibling to the existing `economic_event_emit_service.rs`, reusing its `build_event_input` + bounds-validation + conductor-write spine. It maps the care Observation to `EmitEconomicEventInput { action: "work", provider: caregiver_cid, receiver: recipient_cid, effort_quantity_value: hours, resource_classified_as: ["care-token"] (or "time-token"), substrate_signal: "attention", ... }` and carries `observation_refs: [iroh://<observer>@<log>#<offset>]`.
3. **A care Commitment instantiation** — a new `signal_kind`-style `action` on the existing Mishpat `Commitment` entry (e.g. `commits-care` or reuse the household-chores instantiation already enumerated in the compute-commitment memory) bounding *who may witness/attribute care for whom* (the household's consent envelope — Margaret's family pre-authorizes that household members witness each other's care). This is the **care-class** stream; it MUST stay isolated from compute-class breach signals (substrate-invariant — see CLAUDE.md "Never cross-contaminate care-class and compute-class signals").

**Where the projection lives:** the observe→event projection is a **storage service** (discernment-adjacent), NOT a zome coordinator fn. Rationale: the substrate floor stays deterministic; *deciding that an observation constitutes attributable care* is a judgment that belongs near the elohim ceiling, and the storage service is the natural seam where it reads the Observation, applies the consent-Commitment bounds, and calls the *existing* deterministic `create_rea_economic_event` coordinator. The zome stays a pure notary; the service holds the observe→govern→emit logic.

---

## 4. p2p-design-gate ANSWERS (all four)

**(1) Class — A2 (derived-via-existing-entry), NOT a new entry type.**
The care EconomicEvent rides the existing `EconomicEvent` entry on elohim/content_store. The witnessing rides the existing `Observation` primitive (kind = manifest vocab). The consent envelope rides the existing Mishpat `Commitment` entry (new `action` discriminator, NOT a new entry type — same as `replicates-dwelling` / `delegates-compute` added actions, mishpat at 9/~100). **Net new DHT entry types: ZERO.** The DNA entry budget is untouched — this is exactly the "new social vocabulary lands as action/signal_kind additions, never new entry types" discipline.

**(2) Does a DHT entry type already exist to ride?**
**Yes, three.** EconomicEvent (`content_store_integrity:1116`), Observation (storage-substrate primitive + iroh-blob log), Commitment (`mishpat_integrity:275`). No new entry type required; the only DNA-touching change is a new `action` discriminator on Commitment (mishpat headroom: 9/~100 entries, and this rides an EXISTING entry's action field — defense-in-depth validator arm added to `commitment_action_requirements`, `mishpat_integrity:812`).

**(3) Identity — content-derived (CID).**
The Observation's universal reference is already `(observer_cid, log_cid, log_offset)` → `iroh://<observer>@<log>#<offset>` (`observation/wire.rs:18`). The EconomicEvent CID is content-derived (entry_hash, per `project_mishpat_commitment_cid_is_entry_hash` — return the **entry_hash** as CID, not action_hash). The consent Commitment CID = its entry_hash. No UUIDs, no slugs.

**(4) Which coordinator fn CREATES it; which signal PROJECTS it.**
- Creates: the **existing** `create_rea_economic_event` (`content_store/src/lib.rs:12124`) — called by the new `care_event_emit_service` after consent-bounds pass. The care Commitment is created by the existing Mishpat commitment coordinator with the new `commits-care` action.
- Projects: the post-commit `EconomicEventCommitted` signal → `ReconcileController` → care-stream SQL projection (a household-scoped read view joining the EconomicEvent projection on `resource_classified_as = care-token`). **Note the known subscriber gap:** `project_conductor_signal_msgpack_decode_class` records that the REA/content signal subscribers still drop holo_hash byte-array fields — the care projection must subscribe through the *fixed* decode path, not the broken one. This is a cross-plan dependency on the signal-decode fix.

---

## 5. Existing substrate to build on + what NOT to re-own

**Build on (file:line):**
- `elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:1116` — `EconomicEvent` entry (ride it).
- `:224` `REA_ACTIONS` (`work` at `:243`), `:271` `RESOURCE_CLASSIFICATIONS` (`care-token`, `time-token`, `stewardship`) — vocabularies already validated.
- `elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:12124` — `create_rea_economic_event` coordinator (reuse).
- `elohim/elohim-storage/src/services/economic_event_emit_service.rs` — `build_event_input` + bounds-validation + conductor-write spine (clone the pattern; do NOT fork the bounds logic).
- `elohim/elohim-storage/src/observation/{wire.rs,log.rs,projector.rs}` — Observation primitive + per-observer log (add the `household:care-given` kind + the care projector).
- `elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:275` `Commitment` + `:812` `commitment_action_requirements` (add the `commits-care` defense-in-depth arm, mirroring `replicates-dwelling`).
- `project_rea_compute_commitment_primitive` (memory) — the one-primitive-many-instantiations frame; care is the caregiving instantiation.

**Do NOT re-own (cite the ledgers):**
- **P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14** owns the Observation transport planes (iroh `IROH_OBSERVATION_ALPN` / libp2p `OBSERVATION_LOG_PROTOCOL_ID`), the blob substrate, and the placement-gap signal. This stub *consumes* the Observation read side; it must NOT redefine the observation wire/transport or the bounds-validator. Declare a cross-plan edge, do not collide.
- **FEDERATION-WEB2-LEDGER-2026-06-14** + `bridges/valueflows` own the VF-GraphQL translation surface and the hREA cell provisioning. This stub makes EconomicEvent *real* on the native side; when it lands, the valueflows bridge's `EconomicEventGql::fixture` can be replaced by a real read of native care events — but that swap is the bridge's milestone (M3+), NOT this stub's scope.
- **D-DIAGNOSTIC** (self-healing control-plane read-model) owns the household-facing felt surface; the care stream renders *alongside* the placement-gap surface (the §frame reframe). Consume D-DIAGNOSTIC's read-model shape; do not build a parallel one.

---

## 6. The FIRST a2o SCENARIO (story-first — this is the spec)

`genesis/a2o/features/shefa/observed-care-becomes-mutual-value.feature` (proposed — `@requires:household-nodes`):

```gherkin
Feature: A family member's care is observed and becomes visible mutual value
  As a household that cares for each other
  We want care given inside the household to be witnessed and recorded
  So that care is real in the same ledger as everything else we steward
  — never scored, never extracted, just made visible as mutual value.

  Background:
    Given Margaret's household runs a node with her family as members
    And the household holds a care-consent commitment
      """
      household members may witness and attribute each other's care
      """

  Scenario: Witnessed caregiving becomes a notarized care economic event
    Given Margaret spends 25 hours over a week caring for her mother
    And a household member witnesses and records this care
    When the care observation is projected
    Then a "work"-action EconomicEvent is notarized on the household's chain
    And it is classified as "care-token" with effort of 25 hours
    And the event carries an observation_ref back to the witnessing observation
    And the event is bounded by the household's care-consent commitment

  Scenario: The family sees care as mutual value, not a score
    Given Margaret's 25 hours of care are notarized
    When a family member opens the household care surface
    Then they see Margaret's care recorded as witnessed mutual value
    And no ranking, leaderboard, or point-total is shown
    And the care stream is shown separately from compute and storage signals

  Scenario: Care outside the consent envelope is refused at the source
    Given a stranger outside the household attempts to attribute care to Margaret
    When the care emit is attempted
    Then the bounds check refuses it before the conductor is contacted
    And no EconomicEvent is created
```

The third scenario encodes the substrate-invariant: a refused attribution is the *consent envelope's* limit (the household's), enforced by the existing bounds-validator spine — and it keeps care-class strictly isolated from compute-class.

---

## 7. Effort + risk + why this serves O2

**Effort: M (Medium).** Bounded *because the hard parts already exist.* No new DHT entry type; the EconomicEvent entry, the care vocabularies, the coordinator, the bounds-validator, the Observation primitive, and the conductor-write spine are all present. The net new surface is: one Observation kind (manifest), one storage service (`care_event_emit_service.rs`, ~mirrors the existing emit service), one Commitment `action` + its defense-in-depth validator arm, one household-scoped care SQL projection, and the a2o scenarios. The largest unknown is the signal-decode subscriber gap (cross-plan dependency).

**Risk: MEDIUM-LOW**, with two named risks:
- *Care-class / compute-class contamination* — the substrate-invariant. Mitigated by keeping the care stream a distinct `resource_classified_as`/`signal_kind` lane and a separate projection; the third a2o scenario regression-guards it.
- *The signal-decode subscriber gap* (`project_conductor_signal_msgpack_decode_class`) — the care projection must ride the fixed decode path. If that fix isn't landed, the care projection silently drops the holo_hash fields. **This is the dependency to sequence first.**

**Why this serves O2 (and the §frame reframe):** this is the single most direct move from "care is enabling-only" to "care is first-class." It couples O2 to O1/O3/O5: the same household surface that shows "Grandma's photos are held" (O1, the placement-gap felt surface) shows "Margaret's care is witnessed" (O2) — *coupled story + value + governance on one surface*, which is exactly O9's attractor (humble living, coupled, capture-resistant). Care becomes economic substance with on-chain standing, witnessed by people who were there, never extracted by a platform.

---

## 8. OPEN QUESTIONS for the operator (decisions only you can make)

1. **Is care a new primitive or an instantiation of the existing EconomicEvent/Commitment family?** This stub *recommends* instantiation (Path A2, zero new entry types — care is the caregiving instance of the compute-commitment primitive family). This is the load-bearing decision; everything else follows from it. **Do you bless instantiation, or do you want care to be a distinct primitive (and accept the entry-type spend)?**

2. **`care-token` vs `time-token` vs `stewardship` as the resource classification** — all three already exist in the validated vocabulary. `care-token` ("Witnessed caregiving acts") is the literal fit; `time-token` ("Hours contributed to community") is the effort-hours fit. Which is canonical for caregiving — or both, with `care-token` the class and `time-token` the unit?

3. **The consent envelope (the care Commitment).** Care attribution is *intimate*. Who may witness and attribute care for whom must be bounded by a household consent commitment. Should the consent envelope be **household-scoped by default** (any household member may witness any other's care), or **per-pair opt-in** (Margaret must consent to being observed)? This is a governance/dignity decision, not a technical one — and it shapes O5 (data agency: care *about me* is data about me).

4. **Visibility default.** Should witnessed care be visible to the **whole household**, **only the caregiver + recipient**, or **caregiver-controlled**? The a2o scenario assumes whole-household; you may want the dignity default inverted.

5. **The anti-gamification line.** O2 is explicitly "observed care, never scored." The second a2o scenario asserts *no ranking/leaderboard/point-total*. Do you want that as a hard substrate invariant (the projection structurally cannot rank), or a presentation-layer norm? (Recommendation: substrate invariant — the care projection emits witnessed events, never aggregated scores.)

6. **Sequencing against the signal-decode fix.** The care projection depends on the fixed conductor-signal decode path (REA subscriber still broken per memory). Do you want this stub to *wait* on that fix, or to *carry* the REA-subscriber fix as its first task?

---

*Working draft. Greenlight-to-expand only — no implementation until the §8 decisions are made.*
