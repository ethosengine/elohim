---
id: vision-design-felt-spine
title: "The Polished Felt Experience as the Spine — a path/pivot design pass (D13 + 'polished end-user-experience')"
status: PROPOSAL — operator blessing required (escalation of D13 from yes/no to path/pivot)
authored: 2026-06-14
escalates-from:
  - SPRINT-KICKOFF-2026-06-14.md §5 D13 (the tactical yes/no this supersedes)
  - genesis/docs/superpowers/plans/2026-06-14-vision-gap-grandma-vertical-stub.md (the stub this carries upward)
carries-vision: the north star (quilt-tier dataplane · mutual compute agreements · collectives-serve-humans · high-integrity DHT · fractal stewards/hubs · governance contracts · donut care-economy · capture-resistant stasis)
cite-sealed: NO (working draft — do not seal)
---

# The Felt Experience IS the Spine — not a veneer on the substrate

> **The escalation in one line:** D13 asks "couple the felt surface into this sprint, yes/no?" That
> is the wrong altitude. The vision's *first clause* — "a p2p dataplane that provides a polished
> end-user-experience" — is not item one of nine; it is the **load-bearing spine the other eight
> serve**. The pivot: stop treating the felt layer as a read-projection bolted onto an
> admin read-model, and **invert the dependency arrow** — let grandma's felt moment be the
> acceptance test that *pulls* the byte-quilt, the arc-commitment, and the care-economy into
> existence, each cashing out as a moment she can feel. This pass is the connective tissue that
> makes the architecture pulled-by-the-human, not pushed-by-the-swarm.

---

## Part 1 — What the VISION REQUIRES here (which clauses, specifically)

The north star names a single felt thread that touches **every** clause. Read it as one
grandmother's afternoon, not nine objectives:

> Grandma opens the app. Her photos load **instantly** *(quilt-tier replicated dataplane)*. She
> SEES they are **held by people who love her** — "the Dowells, Aunt Ruth, First Church" *(collectives
> continue to serve the humans that use it; high-integrity DHT she can build trust on)*. That holding
> is a **negotiated, revocable promise** — Aunt Ruth committed to hold them; the church committed to
> hold them *(mutual compute agreements; governance contracts that set policies)*. When Aunt Ruth's
> copy lapses, grandma is not shown a red SLA — she is shown **who could help, and a one-tap
> pro-social action** *(fractal stewards & hubs scaling sensemaking; donut care-economy where value
> is minted from observed care)*. She controls **who holds her memories** *(agency back to one's
> data; capture-resistant against the messy real world)*. And the whole thing stays **calm** — no
> overwhelm, no faceless failure, no extractive institution in the loop *(capture-resistant stasis)*.

Each of the felt moment's **five beats** is a hard requirement, and each is the *cash-out* of a
substrate capability that the swarm-side plans (P-ARC, P-RECONCILE, P-PROOFS, F-COHERENCE) build
but never SHOW:

| Felt beat (what grandma experiences) | Vision clause it satisfies | The substrate capability it CASHES OUT |
|---|---|---|
| **B1 — photos load instantly, even when her node is dead** | polished EUX · quilt-tier dataplane | the **byte-quilt** (RS(4,7) erasure-coded blobs, CID-addressed) |
| **B2 — "Held by 3 households: the Dowells, Aunt Ruth, First Church"** | collectives-serve-humans · high-integrity DHT | the **DHT trust-plane** (commitments + stewardship links, validated entries she can trust) |
| **B3 — that holding is a promise, kept by named people** | mutual compute agreements · governance contracts | **arc/custody-as-REA-coverage-commitment** (`Mishpat::Commitment` replicates-*) |
| **B4 — when a holder lapses: who-could-help + one tap, never a red SLA** | donut care-economy · fractal stewards/hubs | **care-minting** (Observation→EconomicEvent) + the placement-gap event already firing |
| **B5 — she controls who holds her memories** | agency back to one's data · capture-resistance | **data-agency** (withdraws-provide actuation + who-holds read) |

The vision REQUIRES all five to be **felt as one coherent surface across her collective's
stewards** — not five admin Lenses. The test the operator actually set (O9): *stasis the
grandmother can feel.*

---

## Part 2 — Is the substrate CAPABLE? Where the limits live (file:line)

**Verdict: every primitive the felt thread needs EXISTS. The limits are not capability gaps — they
are three precise LAYERING ARTIFACTS, each a fork/inversion candidate, not a wall.** I read the real
source for each beat.

### B1 — instant load: byte-quilt is BUILT (capable)
- `elohim/elohim-storage/src/sharding.rs:7` — `encoding="rs-4-7", N+M erasure-coded shards`;
  `sharding.rs:97-99` — `rs_data_shards: 4, rs_parity_shards: 3` (RS(4,7): any 4 of 7 shards
  reconstruct). `sharding.rs:131` selects `rs-4-7` above the 64 MB threshold.
- Serve path: `elohim/elohim-storage/src/http.rs:833` — `GET /blob/{hash}` reassembles from shards;
  `http.rs:274` notes the T17 peer-heal sequence behind it; `http.rs:222` the iroh fallback.
- **The artifact (not a wall):** there is **no felt binding between "the byte arrived from a
  surviving shard" and "grandma saw her photo load."** Instant-load works at the wire; it is never
  *narrated*. CDN immutable-Cache-Control on the miss path (D8, P-DEFENSE) is the only piece pending,
  and it is buildable now. **B1 is capable today.**

### B2/B3 — named holders + the promise: DHT truth EXISTS; the felt projection is HALF-WIRED
- `elohim/elohim-storage/src/services/household_resilience.rs:24` — `compute()` returns
  `protection_status` ∈ {protected, partial, at-risk} + `details.steward_households[]`. **The truth
  is computed.**
- `household_resilience.rs:225-236` (in `snapshot`) maps stewards to `StewardingCollectiveEntry` —
  **but `kind: "household"` is hardcoded and `label: None`** (`:233-234`, comment: *"Future
  collective-kinds will require a lookup into the collectives table"*). The schema already carries
  the names: `resilience-snapshot-view.schema.json` `stewardingCollectives[].{id,kind,label}` with
  the exact grandma vocabulary ("household, church, patron-circle, dao"). **The names exist in the
  schema and the collectives table; the projection does not join them yet.**
- **THE LIMIT (the named gap):** `2026-05-29-durability-topology-felt-resilience.md:80` — the
  top-level posture wire types "exist, **no consuming component** … **absent**." Confirmed: the only
  Angular consumers of `HouseholdResilienceView` are operator/explorer surfaces
  (`resource-explorer.component.ts`, `protocol-omni.component.ts`); there is **no household-addressed
  "your memories are held by…" component.** This is the spine gap — **address (operator-eyes vs
  household-eyes) + narration (the booleans are not yet names).**
- **A second, deeper artifact (the honesty seam):** `household_resilience.rs:48-60` — when no
  manifest exists, `compute` returns a *degenerate `protection_status: "at-risk"`*. That is honest
  to the operator but **lies to grandma**: "never measured" renders identical to "measured and
  failing." The `snapshot` fn already fixed this one layer up (`:150-155`,
  `distribution_state: "unmeasured"`), but `compute` itself still collapses the two. A felt surface
  reading `compute` would show grandma a false red. **This is a fork candidate inside our own code.**

### B3 — the promise as a governed coverage commitment: PRIMITIVE EXISTS, the knob does NOT
- The holding-as-promise rides `Mishpat::Commitment` (`replicates-dwelling/-collective/-commons`),
  the gospel-tier REA compute-commitment primitive (`project_rea_compute_commitment_primitive`).
  The placement-gap event — the moment a promise lapses — **already fires**:
  `elohim/elohim-storage/src/reconcile/custody.rs:250` (`outcome.placement_gaps_emitted += 1`),
  proven by `custody.rs` test `others_commitment_unhonored_emits_placement_gap`.
- **THE LIMIT (the arc worked-example, confirmed in source):** the *coverage* a steward holds is
  governed by arc, and arc is the layering artifact the operator already named. `arc_policy.rs`
  **computes a fractional aim** (`:1`, `target_arc_factor = derive(mem_ceiling, archetype, observed_N,
  corpus, local_share)`; `CoverageParams { r_floor: 3, r_target: 7 }` — *the same RS(4,7) numbers as
  the byte-quilt*). But `arc_actuator.rs:119` **refuses it**: `"target_arc_factor=… is not actuatable:
  the deployed lever is {0,1} only (no fractional arc until kitsune2 sharding lands)"`. And the
  commitment schema **has no knob slot** for it: `2026-06-13-conductor-authority-arc-auto-policy.md:142`
  — `delegates-compute.schema.json` `bounds` is content-publishing fields (`epr_scope, reach_ceiling,
  rate_per_hour, rotation_ttl_days`), validated at `commitments.rs:467-490`; **"the vision's
  `scope: conductor.arc_factor` is aspirational."** *The substrate can SHARD the keyspace (DhtArc is
  continuous) but cannot yet GOVERN the share as a commitment.*

### B4 — care-minting: EconomicEvent spine EXISTS, the care emitter does NOT
- `create_rea_economic_event` exists (`content_store/src/lib.rs:12124`, emits
  `ReaEconomicEventCommitted`); the storage-side bounds-validated emit wrapper landed in the
  acquisition slice. The care action is a **parameterization, not a new entry** — D9 already settled
  `commits-care` as an instantiation of the EconomicEvent/Commitment family (0 new DHT entries;
  `2026-06-14-vision-gap-care-valueflows-stub.md:69`).
- **THE LIMIT:** the **Observation→care-EconomicEvent emitter** is a stub (the O2 sibling). The
  *display* of holding-as-care is free (B2/B3 read existing commitments); the *minting* of new care
  value when a family accepts an invite needs that one storage service.

### B4 cross-cutting — the signal-decode bug that silently breaks ALL of the above
- `MEMORY.md` (project_conductor_signal_msgpack_decode_class) + SPRINT-KICKOFF MF14: holo_hash
  byte-arrays are **silently dropped** on the `rmp → serde_json::Value/String` path. The REA/mishpat/
  content signal subscribers are still on the broken path. **Every human-facing projection rides
  these signals.** This is task-0, a bug fix, not a fork.

**Summary of limits:** one buildable-now wiring gap (the felt component + label join + the
`compute` honesty fork), one genuine substrate fork (arc-as-governed-commitment), one storage
service (care emitter), one bug (signal-decode). **No wall. Four forks/inversions of increasing
depth.**

---

## Part 3 — The PATH / PIVOT / FORK LADDER (cheapest → deepest)

Each rung names cost · blast radius · what it unlocks for the felt spine.

### Rung 0 — Fix the signal-decode subscriber (BUG, gating everything)
- **Cost:** S (the diagnosis is in MEMORY; the fix is per-subscriber). **Blast radius:** the REA/
  mishpat/content subscribers in elohim-storage. **Unlocks:** the projections that feed B2/B3/B4
  stop silently dropping holo_hashes — without this, the Family Vault renders empty even when truth
  exists. **This is not a fork; it is the precondition.** (= MF14/H0/GATE-C.)

### Rung 1 — Light the felt surface against existing truth (BUILDABLE NOW — the spine slice)
- **Cost:** S–M. **Blast radius:** one additive Cat-C `feltStatus` block on `household_resilience.rs`
  + the `kind`/`label` collective join (already in the schema, `:233-234` is where it's stubbed);
  one new Angular `<elohim-memory-safety>` "Family Vault" component (eyes-first via `pnpm look`);
  one new `.feature`. **Includes the honesty fork:** make the felt block read `distribution_state`
  (the `snapshot` already computes it `:150`) so "not-yet-seen" never renders as "at-risk" — fold
  the `compute` degenerate-return (`:48-60`) into the same honesty the `snapshot` already has.
- **Unlocks:** B1 (narrated instant-load) + B2 (named holders) + the honest "watching"/"needs-help"/
  "not-yet-seen" states. **This is the grandma-vertical stub, blessed and inverted:** it ships as the
  *acceptance test that pulls priority*, not a follow-on. Satisfiable on P-PROOFS chaos infra today.

### Rung 2 — Mint care from the felt action (storage service; needs D9 blessing)
- **Cost:** M (one `care_event_emit_service` calling `create_rea_economic_event`, consent-bounded).
  **Blast radius:** new service in elohim-storage + the O2 stub expansion. **Unlocks:** B4 — when a
  family accepts "invite a household to help," an observed-care EconomicEvent is **minted** — the
  donut's value-creation moment, felt. This is the first beat where the felt surface *creates value*,
  not just displays it. Couples to the placement-gap event already firing (`custody.rs:250`).

### Rung 3 — Data-agency: who-holds + revoke as a felt control (needs D-ACTUATION + D11)
- **Cost:** M. **Blast radius:** the `withdraws-provide` actuation arm (rides D-ACTUATION's
  `Actuation`/`RefusalCode` contract) + a who-holds read. **Unlocks:** B5 — grandma controls who
  holds her memories, with **succeeds-with-residual-report** semantics (D11: a refusal would
  re-import the operator-veto smell O5 exists to kill). This is capture-resistance made felt: the
  human, not an admin, holds the revocation lever.

### Rung 4 — THE FORK: arc-as-governed-coverage-commitment (genuine substrate work)
- **Cost:** L (a roadmap commitment, not a sprint task). **Blast radius:** a custom kitsune2 sharding
  module + a `bounds` schema extension on the commitment + a coverage-invariant enforcer. **Three
  sub-rungs (the operator's own ARC worked example, confirmed against source):**
  - **(4a) probe rung** — `NetworkConfig.advanced` / the `arc_policy.rs` gauge already computes the
    aim; surface it. Maybe no fork. Low blast.
  - **(4b) custom module** — write our own kitsune2 sharding policy setting `tgt_storage_arc_hint =
    f(mem, N, corpus)` with a `∪arcs = full` coverage invariant. kitsune2 is **modular by design**;
    this is "write our policy," not "fork Holochain." This is THE vision-aligned path — it makes
    `arc_actuator.rs:119`'s refusal obsolete and `arc_policy.rs`'s computed-fractional aim
    *actuatable*.
  - **(4c) governance fork** — extend the `delegates-compute`/`replicates-*` commitment `bounds`
    schema (`commitments.rs:467-490`) with a coverage-range slot, so **a steward commits
    (`Mishpat::Commitment`) to hold arc-range X**, and the `∪arcs = full` invariant is enforced
    *through governance contracts*. Arc becomes negotiated / audited / revocable — exactly B3.
- **Unlocks:** B3 in full — the "promise" grandma sees is a real, revocable, governed coverage
  commitment, not a hardcoded `kind: "household"`. **And the deeper TWO-QUILT pivot** (from the ARC
  worked example): the **DHT is the trust/value/governance plane** (small validated commitment +
  stewardship entries — B2/B3's *names and promises*, wants near-full arc) and the **byte-quilt
  (RS(4,7), sharding.rs)** carries the heavy corpus (grandma's *photos* — B1, CID-addressed,
  custody-tracked). **The felt spine is exactly the seam between the two quilts:** B1 reads the
  byte-quilt, B2/B3 read the DHT-quilt, and the Family Vault component is the single surface that
  fuses them into one felt safety.

---

## Part 4 — The recommended ESCALATION (defended) + what it COMMITS US TO

**Recommendation: invert the dependency arrow and ship Rungs 0→1 THIS sprint as the spine's
acceptance test; commit Rung 4 to the roadmap as the protocol's named fork.**

The defense, in three moves:

1. **Invert D13 from "couple, yes/no" to "the felt scene is the acceptance gate."** Do not author
   the grandma scenario as a follow-on to P-DIAGNOSTIC's read-model. Author it FIRST, so the
   read-model is **forced to light for household-eyes** (the right address) rather than admin-eyes.
   The stub's own §6 scenarios become the spec; "done" is when grandma's scene passes, not when the
   Lens renders. This is the cheapest move with the highest vision yield — it reorients the whole
   sprint around a felt test without adding substrate. **Recommend: GREENLIGHT (supersedes D13's
   yes/no with "yes, and it leads").**

2. **Fold the `compute` honesty fork into Rung 1.** The degenerate-at-risk return
   (`household_resilience.rs:48-60`) is a latent lie the felt surface would amplify. Fixing it is
   S-cost and is *required* by the felt promise (Scenario 3: "not-yet-seen, NOT at-risk"). This is a
   small fork **inside our own code**, buildable now.

3. **Name the arc-as-governed-commitment FORK as a roadmap commitment, not a sprint deferral.** The
   operator's own ARC worked example already proved the layering artifact is not physics. D1 settled
   "REJECT fractional on kitsune2 0.3.2/0.4.1 (clamps {0,1}); GREENLIGHT the corpus-off-DHT spike."
   This pass escalates that: the corpus-off-DHT spike *is the two-quilt pivot*, and its endgame is
   **(4b) the custom kitsune2 sharding module + (4c) the coverage-commitment governance fork.** Mark
   it as a genuine fork on the roadmap — the moment the protocol stops borrowing kitsune2's
   "always-full-arc" default and writes its own resource-aware, governed coverage policy.

**What this COMMITS US TO:**
- **A new felt primitive (buildable now):** the `feltStatus` Cat-C projection + `<elohim-memory-safety>`
  Family Vault component — the *single human-addressed seam between the two quilts*. Zero new DHT
  entries, zero Mishpat headroom.
- **A roadmap fork (named, not built this sprint):** **arc-as-REA-coverage-commitment** — the custom
  kitsune2 sharding module (4b) + the commitment `bounds` coverage-range schema extension (4c). This
  is the protocol's first "write our own policy layer on a modular substrate" commitment for the
  trust-plane, the sibling of the corpus-off-DHT byte-plane spike.
- **A sequencing principle (the inversion itself):** *the felt scene is the acceptance test for the
  substrate work it consumes.* Every swarm-side plan (P-ARC, P-PROOFS, F-COHERENCE) now has a felt
  beat it must cash out, or it is not done. This is the spine made structural.

---

## Part 5 — COUPLING: how it ties story + value + governance

The felt spine is the **connective tissue** precisely because each beat fuses the three legs the
EPR substrate requires of every Content atom (knowledge + value + governance,
`epr-kind.schema.json:5`). The Family Vault is not a dashboard — it is the **donut rendered for one
grandmother**:

- **Story (knowledge):** the scene `grandma-photos-survive-node-loss.feature` *is* the spec. The
  resilience proof (O7, P-PROOFS chaos) and the felt experience (O1) become the **same artifact** —
  the proof is narrated from grandma's chair. The story leg is the surface itself: named holders,
  honest states, no faceless SLA.
- **Value (the donut, minted from care):** B2/B3 *display* the holding relationship (existing
  commitments — zero new value). B4 *mints* it: accepting "invite a household to help" emits a
  `commits-care` EconomicEvent (Rung 2, D9). Value is created **inside the felt moment** — the
  care-economy's mint event is grandma's family choosing to help, observed and recorded. The donut's
  inner ring (the floor: photos must be held) and outer ring (the ceiling: no extractive custodian)
  are both visible: protection without an institution.
- **Governance (the promise, negotiated and revocable):** B3 is a `Mishpat::Commitment` — Aunt Ruth
  *committed*, the church *committed*, and the `∪arcs = full` coverage invariant (Rung 4c) is the
  **governance contract that sets the policy and enforces the decision**. B5 is the revocation lever
  in grandma's hand (data-agency, Rung 3) — capture-resistance is felt as *control*, not promised in
  a manifesto. When a holder lapses, the placement-gap event (`custody.rs:250`) is the governance
  system **noticing a broken promise** and surfacing a pro-social repair, never a punishment.

**Capture-resistant stasis, felt:** the surface stays calm because it is honest (Rung 1's honesty
fork — "not-yet-seen" never lies), because no single party is seizable (holding is plural and
governed), because the human holds revocation (B5), and because the failure mode is "who could
help," not "you have failed an SLA." The system stays in stasis *because* the grandmother can feel
it is safe — which is the only test O9 actually sets.

**The fractal:** this exact coupling — story+value+governance fused at one felt surface — is the
template. Grandma's photo album is the proving ground (smallest hub, the household). The same
Family-Vault seam, at the factory hub, is the steward's dashboard of governed coverage commitments
across a collective; at the commons, it is the donut of the whole trust-economy. **One substrate,
three instantiations** (the gospel pattern): *care-as-commitment (B4) ≡ coverage-as-commitment (B3,
the arc fork) ≡ compute-as-commitment (the existing `delegates-compute` spine).* The felt spine is
where they all become visible to a human at once.

---

## Open questions for the operator (decisions only you can make)

1. **Bless the INVERSION?** D13 asked "couple, yes/no." This pass escalates to "the felt scene
   *leads* — it is the acceptance test for D-DIAGNOSTIC's read-model, forcing it to light for
   household-eyes." **Recommend: yes.** (Cost is reorientation, not new substrate.)
2. **Commit the ARC FORK to the roadmap?** Name arc-as-governed-coverage-commitment (custom kitsune2
   sharding module 4b + commitment `bounds` coverage-range 4c) as a genuine fork, sibling to the
   corpus-off-DHT byte-plane spike (D1). **Recommend: yes — mark it a fork, not a deferral**, so it
   does not drift to permanent zero.
3. **Honesty fork in `compute`?** Fold `distribution_state` into the `feltStatus` block so the
   degenerate-at-risk return (`household_resilience.rs:48-60`) cannot render a false red to a family.
   **Recommend: yes, in Rung 1** — it is required by the felt promise, not optional.
4. **Felt-vocabulary ownership (privacy seam, unchanged from the stub):** does the family-facing
   holder label come from the collective's self-chosen name or a viewer-local alias? A real O5
   boundary — a holder may not want their household name shown to every content viewer.
5. **Manifesto coupling (flagged, OPERATOR-CALL only):** "family photos," "the donut," and "stasis
   the grandmother can feel" appear in no core vision doc. This pass makes the scene executable —
   do you want a manifesto addendum so the named vision and the built spine cohere?

---

*Working draft — NOT cite-sealed. The Rung-0/1 spine is buildable now and reads existing truth; the
Rung-4 arc fork is a genuine roadmap/substrate commitment. This pass recommends boldly: invert the
arrow, ship the felt seam as the acceptance test, and name the arc-as-governed-commitment fork so
the trust-plane gets the same "write our own policy" treatment the byte-plane is already getting.*
