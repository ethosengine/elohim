---
id: vision-gap-grandma-vertical-stub
status: STUB — GREENLIGHT-TO-EXPAND (operator blessing required before plan-expansion)
authored: 2026-06-14
authored-against:
  - VISION-ALIGNMENT-2026-06-14.md §6 Decision F (the reframing)
  - P2P-DATAPLANE-CONTRACT-LEDGER-2026-06-14.md (do NOT re-own its files)
  - genesis/docs/superpowers/plans/2026-06-14-dataplane-diagnostic-plan.md (P-DIAGNOSTIC)
  - genesis/docs/superpowers/plans/2026-06-14-dataplane-proofs-plan.md (P-PROOFS)
  - genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md
cite-sealed: NO (working draft)
---

# The Grandma-Vertical — Placement-Gap as Household-Felt Surface + the Spine a2o Scene

> This is the **spine** the other four vision-gap stubs (O2 care-valueflow, O3 limit-respect
> governor, O4 home-for-AI, O5 data-agency) hang from. It is the single highest-leverage
> *coupling* move named by the vision-alignment pass (Decision F): turn the placement-gap signal
> from an admin Lens into a **household-facing felt surface**, and author the a2o scene that
> *pulls* priority across the whole vertical. It is a **PLAN-EDIT** to two in-flight dataplane
> plans (P-DIAGNOSTIC, P-PROOFS) + **ONE new a2o feature file** — not a new owned src tree.

---

## 1. Objective + the felt promise (one paragraph)

**Objective (couples O7→O1/O2/O3/O5):** The system already *computes* whether grandma's photos
are safe — `household_resilience.rs::compute` returns a `protectionStatus` of `at-risk | partial |
protected` per content, and `custody.rs` *emits a REA `placement-gap` event* the moment a holder
lapses. But that proof lights for the **operator** (the `/debug` stability Lens, the raw
`/api/v1/placement-gaps` read), not the **family**. The felt promise: **when grandma's edge node
dies, her family opens the app and *sees* — in plain, named language — that her photos are still
held by people who love her** ("Held by 3 households: the Dowells, Aunt Ruth, the church"), and
when they are NOT, they see *who could help* and a one-tap pro-social action — never a faceless red
SLA. This is the cybernetic proof (O7) *felt* as safety (O1), surfaced as named observed care (O2),
gated by limit-respecting refusal language (O3), and legible enough to act on (O5). It is the test
O9 actually sets: stasis where the grandmother can feel it.

---

## 2. Vision-vs-substrate GAP (promise vs code today)

| The protocol promises | What the code does today (file:line) |
|---|---|
| Grandma *feels* her photos are safe, in named terms | `household-resilience-view.schema.json:13` ships `protectionStatus` + `details.stewardHouseholds[]` — but the **only Angular consumer is the operator-facing debug Lens**; there is no household-addressed "your memories are held by …" component. The felt-resilience spec names this exact gap: `ResilienceSnapshotView` et al. "exist, **no consuming component** … **absent**" (felt-resilience spec:80). |
| A lapse is *felt* the moment it happens | `reconcile/custody.rs:250` already does `outcome.placement_gaps_emitted += 1` — the placement-gap **REA event already fires** when a commitment is unhonored (`custody.rs:541` test proves it). But it lands in `placement_gaps` (a Cat-C projection read at `api/placement_gaps.rs`) consumed by the admin surface, not pushed to the family. |
| The proof is *coupled* to the story | P-PROOFS authors a chaos scenario (`chaos-peer-churn.feature`) that proves "one object's K-of-N shards survive a single-household loss" (proofs-plan #7b) — but **no a2o scene narrates that proof from grandma's chair.** The proof and the felt experience are not yet the same artifact. |
| Refusal language respects the human's limit, not the operator's | `household_resilience.rs:46-60` returns a *degenerate at-risk view* when there is no manifest — honest, but worded as a verdict, not as "we can't see this yet / here's how to help." (O3 framing gap.) |

**Diagnosis:** every primitive exists. The gap is **address** (operator-eyes vs household-eyes) and
**narration** (the proof is not yet a story). This is connective tissue + one felt component +
one scene — *not* new substrate. That is exactly why it is the highest-leverage move: maximal
vision yield per unit of new code.

---

## 3. The MISSING BRIDGE / primitive (concrete)

**There is NO new DHT entry and NO new coordinator function.** The bridge is a **household-addressed
read-projection + felt component + the spine scenario**, in three concrete pieces:

1. **`HouseholdResilienceView.details` gains a `feltStatus` framing block** (additive, Cat-C):
   `{ headline: string, heldBy: { collectiveId, kind, label }[], reassurance: "protected" | "watching" | "needs-help" | "not-yet-seen" }`. This is the *named* projection of the booleans the
   admin already gets — it turns `protectionStatus: "at-risk"` into `"Aunt Ruth's copy went
   offline — 2 households still hold these photos"`. **It is computed by the SAME
   `household_resilience.rs::compute` fn** that exists today; this stub only adds the human-framed
   sub-block and the `kind`/`label` join already present in `resilience-snapshot-view.schema.json:47`.

2. **A household-facing Angular component** (`<elohim-memory-safety>` / "Family Vault" card) that
   consumes the felt block — the inverse of the operator Lens. Renders names, not nines.
   *(Frontend sibling; eyes-first via `pnpm look`. Named here, scoped in expansion.)*

3. **The spine a2o scenario** (§6) — the executable spec that couples P-PROOFS' chaos proof to
   the felt surface. **This scenario is the deliverable that pulls the other four stubs into
   priority** (each becomes "what does the family DO at this surface": care-emit O2, limit-respect
   O3, revoke/who-holds O5, AI-steward-as-named-holder O4).

**Is care a new primitive?** No — per `project_rea_compute_commitment_primitive` (gospel), the
placement-gap event is *already* an instance of the `Mishpat::Commitment` family (a holder's
`replicates-dwelling/-collective/-commons` commitment lapsed → gap event). The felt surface reads
that existing REA truth; it does not mint a new economic-event class. (The O2 *care-emit* stub —
its sibling — is where the **Observation→care-EconomicEvent emitter** is scoped; this spine stub
only *displays* the holding relationship that already exists as commitments.)

---

## 4. p2p-design-gate ANSWERS (all four — MANDATORY)

The reframe introduces **one new wire entity**: the `feltStatus` framing sub-block on
`HouseholdResilienceView`. Gated:

1. **Class → Operational (C), node-local read-model.** It is a per-request *projection* — a
   human-framed re-statement of `protectionStatus` + the steward-collective join that
   `household_resilience.rs` already computes. No notarization, no persistence, no truth of its
   own. (Consistent with how P-DIAGNOSTIC classes its anchor booleans: "New runtime entities are
   Cat-C node-local read-models … cite the class, do not re-litigate.") The *underlying* truth it
   reads — who-holds-what — is Cat-A (the holders' `Mishpat::Commitment` DHT entries) + Cat-A2
   (stewardship-allocation links) + Cat-A (`PeerStatus`), exactly the sources
   `household-resilience-view.schema.json:5` already names.

2. **Does a DHT entry type already exist to ride? → YES, ride three existing ones, mint none.**
   The holding relationship rides existing `Mishpat::Commitment` entries (the
   `replicates-*` family) + infrastructure-DNA `PeerStatus` (Cat-A) + `Agreement`-derived
   stewardship-allocation links (Cat-A2). **Mishpat headroom is the constraint to respect (~11/~100
   per CLAUDE.md); this stub consumes ZERO Mishpat headroom** because it adds no entry — it reads
   existing commitments. This is the correct DHT-native posture.

3. **Identity → no new identity is minted.** The felt block is keyed by the existing `contentId`
   (already content-addressed) and the existing `collectiveId` (already the join key in
   `resilience-snapshot-view.schema.json:52`). No UUID PK, no slug. (The placement-gap *row* keeps
   its existing UUID `id` per `placement-gap-view.schema.json` — that is an operational projection
   id, not protocol identity, and is unchanged.)

4. **Which coordinator fn CREATES it; which signal PROJECTS it?**
   - **Creates (the underlying truth):** the holder's `replicates-*` commitment is created by the
     existing stewardship/custody coordinator path; the *gap* is created by
     `reconcile/custody.rs` (`placement_gaps_emitted += 1`, `custody.rs:250`) — **already wired,
     not new.**
   - **Projects:** the existing `household_resilience.rs::compute` projection (HTTP
     `GET /api/v1/resilience/{content_id}/household`, `http.rs:10254`). The `feltStatus` block is
     an **additive field on that existing view** — same fn, same route, same Cat-C projection. The
     placement-gap REA event already flows to the topology/shefa signal surface
     (`api/placement_gaps.rs`); the felt component subscribes to that existing flow. **No new
     route, no new signal kind.**

**Verdict: NO new entity at the protocol layer. One additive Cat-C view field + one consuming
component + one scenario.** This is the DHT-native, headroom-zero, capture-resistant shape.

---

## 5. Existing substrate to build on (file:line) + what NOT to re-own

### Build ON (existing, verified):
- `elohim/elohim-storage/src/services/household_resilience.rs:24` — `compute(...)` already returns
  `protectionStatus` + `details.stewardHouseholds[]`. **Add the `feltStatus` sub-block here.**
- `elohim/sdk/schemas/v1/views/household-resilience-view.schema.json` — additive `feltStatus`
  object under `details` (or a sibling). Schema-before-code (sdk/schemas/CLAUDE.md).
- `elohim/sdk/schemas/v1/views/resilience-snapshot-view.schema.json:47` — already carries
  `stewardingCollectives[].{id,kind,label}` (the *names* the felt surface needs) and
  `placementGaps[]`. The collective-general framing ("household, church, patron-circle, DAO") is
  the correct grandma vocabulary already.
- `elohim/elohim-storage/src/reconcile/custody.rs:250` — placement-gap REA emit, **already live**;
  the felt surface consumes it, does not re-emit.
- `genesis/docs/superpowers/specs/2026-05-29-durability-topology-felt-resilience.md:68-80` — the
  progressive-disclosure spine (icon→bar→drill-down) is **substantially live**; the named gap is
  the top-level posture view "no consuming component … absent" (:80). This stub closes exactly
  that named gap, household-addressed.
- `genesis/a2o/features/resilience/` — `chaos-peer-churn.feature`, `household-reciprocity.feature`,
  `observable-distribution.feature` already exist; the new scene is a **sibling**, not a rewrite.

### Do NOT re-own (cite the ledgers — propose edits, never collide):
- **P-DIAGNOSTIC owns** `doorway/.../self_healing.rs`, `routes/health.rs`, `main.rs:483-494`,
  `p2p/mod.rs` anchor fields, `stability-status-view.schema.json`
  (dataplane-diagnostic-plan.md "OWNED FILES"). **This stub proposes ONE edit to P-DIAGNOSTIC:**
  add a follow-on task "household-addressed felt projection of the anchor/placement read-model"
  (its plan already names "household-legible, not an admin store" as the Decision-F intent). **Do
  not write into its owned files — hand the edit to its owner.**
- **P-PROOFS owns** `tests/placement_diversity_invariant.rs`, `tests/rs_reconstruct_property.rs`,
  and **`genesis/a2o/features/resilience/chaos-peer-churn.feature`** (it un-`@wip`s rows there).
  **This stub proposes ONE edit to P-PROOFS:** add an acceptance note that the new
  `grandma-photos-survive-node-loss.feature` (a *new file*, §6) is the felt-narration of its
  chaos proof, and cross-cite the backing deterministic test. **The new feature is a NEW file
  this stub proposes — it does NOT touch P-PROOFS' `chaos-peer-churn.feature`.**
- **D-ACTUATION (P9 REA spine)** / **D-DIAGNOSTIC read-model** — declared as **cross-plan
  CONSUMED edges** (this surface reads their outputs; it does not redefine the actuation trait or
  the status fields).
- **Federation ledger (FEDERATION-WEB2-LEDGER-2026-06-14.md):** the cross-edge coherence work
  (F-BOOTSTRAP/F-COHERENCE) is what makes "the family on the OTHER island can see grandma's
  photos are held" actually true. **Declared as a cross-plan dependency edge, not re-owned** —
  honor the museum tell (name the *plural* doorway; this stub's vocabulary says "the family's
  edges," never "the doorway").

---

## 6. The FIRST a2o SCENARIO (story-first — the spec)

**New file (proposed):** `genesis/a2o/features/resilience/grandma-photos-survive-node-loss.feature`

> This scenario IS the specification. Expansion is done when it passes. It is deliberately
> household-addressed (the actor is the *family*, not the operator) so it pulls the felt surface
> into existence and gives the other four stubs a concrete place to attach.

```gherkin
@vision @grandma-vertical @resilience @felt
Feature: Grandma's photos survive a node loss; the family sees they're held
  As a family member of someone whose edge node has gone offline
  I want to see, in named human terms, that her memories are still held by people who love her
  So that the system's resilience proof becomes felt safety, not a faceless SLA

  Background:
    Given grandma's photo album "summer-1974" is stewarded by 3 households
      | household        | kind      |
      | the Dowells      | household |
      | Aunt Ruth        | household |
      | First Church     | church    |
    And each household holds an active replicates-collective commitment for it

  # Couples O7's proof (P-PROOFS chaos) to O1 felt safety
  Scenario: Grandma's node goes offline but her photos remain protected
    When grandma's edge node goes offline
    And the family opens the Memory Safety surface for "summer-1974"
    Then they see the protection status "protected"
    And they see it is "Held by 3 households: the Dowells, Aunt Ruth, First Church"
    And no faceless error or SLA percentage is shown

  # Couples to O7's RS-reconstruct + placement-diversity proofs (P-PROOFS #7a/#7b)
  Scenario: A holder lapses but K-of-N survives — the family is reassured, not alarmed
    Given a placement-gap event has fired because "Aunt Ruth" stopped holding a shard
    When the family opens the Memory Safety surface for "summer-1974"
    Then they see the reassurance state "watching"
    And they see "These photos are safe — 2 households still hold a complete copy"
    And the message names who lapsed, not an opaque shard hash

  # Couples to O3 limit-respect: refusal language is the operator's limit, not a verdict on grandma
  Scenario: The system cannot yet see the album — it says so honestly
    Given "summer-1974" has never entered the distribution plane
    When the family opens the Memory Safety surface for "summer-1974"
    Then they see the state "not-yet-seen"
    And they see "We can't confirm these are backed up yet — here's how to invite a household to help"
    And they are NOT shown a red "at-risk" verdict

  # The attach point for O5 (data agency) and O2 (observed care) — named here, scoped in siblings
  Scenario: When protection is short, the family is offered a pro-social action
    Given "summer-1974" is held by only 1 household
    When the family opens the Memory Safety surface for "summer-1974"
    Then they see the reassurance state "needs-help"
    And they are offered "Invite a household to help hold these memories"
    # O5 expansion: this action surfaces "who holds my photos" + revoke
    # O2 expansion: accepting the invite emits an observed-care EconomicEvent
```

The first two scenarios are satisfiable by **P-PROOFS' existing chaos infrastructure + the
additive `feltStatus` projection** (no new substrate). Scenarios 3–4 are the **attach points** for
the O3/O5/O2 sibling stubs — they are written now so the spine names where the other gaps land.

---

## 7. Effort (S/M/L) + risk + why this serves the objective

**Effort: S–M.** Storage projection field (S — additive sub-block on an existing fn/view/schema +
codegen). Felt Angular component (M — new but consumes an existing view; eyes-first). The spine
`.feature` (S — new file). No new DHT entry, no coordinator fn, zero Mishpat headroom, no route.
The S-cost-for-vision-yield ratio is the whole argument for sequencing it *now, while the
read-model is being lit* (Decision F: "couple it now, or it lights for the wrong eyes").

**Risk: LOW, but two real ones.**
1. **Honesty risk (the felt surface must not lie).** The felt language must preserve the
   `distributionState: unmeasured` honesty (resilience-snapshot-view.schema.json:20) — "not-yet-seen"
   must NEVER render as a fake "protected." The scenario encodes this (Scenario 3). This is the
   P13 honest-denominator discipline made felt.
2. **Cross-edge truth risk.** On an *islanded* edge (the federation gap), the family on island B
   may not see island A's holders — so the felt surface could under-report protection. **Mitigation:
   declared as a cross-plan dependency on F-BOOTSTRAP/F-COHERENCE**; until they land, the felt
   surface must say "as seen from your edge" (honest scoping), never assert a global count it
   cannot verify.

**Why it serves the objective:** it is the *only* move in the current sprint that converts
infrastructure-for-its-own-sake into **coupled story+value+governance** (O9's actual test). The
resilience proof (O7) *becomes* the felt safety of the household (O1), surfaced as named observed
care (O2), gated by limit-respecting refusal (O3), and legible enough to act on (O5). It is the
spine because each of the other four stubs attaches to a scenario step here.

---

## 8. OPEN QUESTIONS for the operator (decisions only you can make)

1. **GREENLIGHT the spine?** This stub proposes editing two *in-flight* plans (P-DIAGNOSTIC +
   P-PROOFS) to add follow-on/acceptance tasks, plus one new `.feature`. Do you approve coupling
   the felt surface into *this* sprint (Decision F: "couple it now"), or hold it for the
   household-felt sprint that the alignment pass says must follow? **(Recommend: greenlight now —
   the read-model is being lit this sprint; coupling later means re-lighting.)**

2. **Felt vocabulary — who names the holders?** The surface shows "Held by the Dowells, Aunt Ruth,
   First Church." That display name comes from the collective `label` (resilience-snapshot:54).
   **Where does the family-facing label come from — the collective's self-chosen name, or a
   viewer-local alias?** (A real privacy + identity decision: a holder may not want their household
   name shown to every content viewer. This is an O5 boundary you own.)

3. **Refusal/limit language ownership (O3 seam).** The "not-yet-seen / needs-help" wording is the
   *operator's* limit framed pro-socially, not a verdict on grandma. **Do you want the limit-respect
   sibling stub (O3) to own the wording vocabulary as a shared resource, or does the felt component
   own its own copy?** (Affects whether the O3 governor and this surface share one phrasing
   primitive.)

4. **Scope of the spine's pull.** This scenario is written to *pull* the O2/O5/O4 stubs into its
   step 4 ("offered a pro-social action"). **Do you want those attach-point scenarios (3–4) authored
   now as `@wip` placeholders, or left as comments until each sibling stub is greenlit?** (Recommend:
   `@wip` placeholders — they make the coupling visible without claiming unbuilt behavior.)

5. **Manifesto coupling (deferred, flagged).** The alignment pass notes "family photos," "stasis,"
   and "cybernetic discipline" appear in NO core vision doc. This stub makes the family-photos scene
   *executable* — **do you want a manifesto addendum to follow** (manifesto-tier, OPERATOR-CALL
   only, never auto-applied), so the named vision and the built scene cohere?

---

*Working draft — NOT cite-sealed. GREENLIGHT-TO-EXPAND: the felt surface reads existing truth and
mints no new protocol entity, but the value/governance framing (Q2/Q3) and the sprint-coupling
(Q1) need the operator's blessing before plan-expansion.*
