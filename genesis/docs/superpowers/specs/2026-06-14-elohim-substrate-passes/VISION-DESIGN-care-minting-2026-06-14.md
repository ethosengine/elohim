# VISION DESIGN PASS — Minting care-value: observed care → REA ValueFlows (the donut)

**Date:** 2026-06-14
**Author:** Rust Architect (truth layer)
**Status:** PROPOSAL for operator blessing — working draft, NOT cite-sealed, NOT a decision, NOT code.
**Scope:** D9 escalated. O2 + O6 + O9 fused — how care-VALUE is *minted* from observed care, as REA/ValueFlows, so the "care-based economy stories where value is minted" and the "donut-like commons" hold.

> This pass ESCALATES the path/pivot the vision requires. It supersedes the tactical 2026-06-14 sprint-kickoff yes/nos. The question is not "can we mint a token" — we already do (`token_mint_service.rs` is live). The question is whether the **observe → mint** seam preserves care's *meaning* while making its *value* real, governable, and capture-resistant. That seam is where the deepest fork lives.

---

## 1. What the VISION REQUIRES here

The north star clause that owns this pass: *"the trust-economy, the care-based economy stories where value is minted ... a donut-like commons ... Coupled story+value+governance, so the system can stay in stasis when actuating a capture-resistant state against the real world, its externalities, and its messiness."*

Decomposed into substrate requirements:

1. **Care must be SEEN before it is minted.** Margaret's 25 hours/week of childcare (resilience README:44) is value the current architecture "cannot recognize as value at all." The vision requires an *observation* primitive that witnesses care, not a transaction primitive that prices it. Minting must be *downstream of witnessing*, never the witnessing itself.
2. **Minting must not collapse care into "just another transaction."** A care-token that is fungible-on-mint, instantly tradeable, priced by a market clearing function, reproduces exactly the attention-auction the protocol exists to refuse (resilience README:35). The vision requires care-value to be **minted as recognition, not as currency** — non-extractable at the moment of mint, with liquidity (if any) governed downstream.
3. **The donut shape: a commons floor + a regenerative ceiling.** Value flows must guarantee a *floor* (no contributor's care goes unrecognized below a dignity minimum) and enforce a *ceiling* (no single steward captures unbounded recognition share — the anti-monopoly invariant). This is Raworth's donut rendered as REA distribution policy.
4. **High-integrity DHT notarization of the value-claim, not the value-amount.** The trust-economy clause requires that *who did the care, witnessed by whom, when* is DHT-notarized and non-repudiable. The *amount minted* is a projection governed by policy — it can be re-derived, re-weighted, corrected. The fork is in **what is Cat-A (notarized) vs what is Cat-C (recomputable)**.
5. **Governance contracts set the minting POLICY.** Mint rate, floor ratio, ceiling ratio, event weights — these are not constants in Rust. They are the substrate-floor's deterministic *evaluation* of a policy the elohim-ceiling (qahal governance) *sets*. Capture-resistance means the minting function is auditable, parameterized by governed policy, and revocable.
6. **Care-class stays isolated from compute-class.** Compute breach must never debit care attribution; care minting must never gate compute placement (`project_compute_commitments_bounded`). The minting pipeline rides the care-class stream exclusively.

---

## 2. Is the substrate CAPABLE? Dig to WHY (file:line)

**Surprising and important: the bones are almost all here.** This is NOT a greenfield. The escalation is about *seam discipline*, not absence.

### What is LIVE (read the real source)

- **Care is already a first-class resource classification.** `content_store_integrity/src/lib.rs:284` — `"care-token", // Witnessed caregiving acts` sits in the 16-value `RESOURCE_CLASSIFICATIONS` whitelist. The word "witnessed" in the comment is load-bearing: the design intent is observe-first.
- **The REA value spine is deployed.** `EconomicEvent` (lib.rs:1116), `Commitment` (lib.rs:1381), `Appreciation` (lib.rs:1425 — "recognition of value created by another ... flow to creators when their work is used"), `Claim` (lib.rs:1448), `Settlement` (lib.rs:1483). `EconomicEvent.substrate_signal` (lib.rs:1150-1152) already carries "which protocol substrate dimension this event consumed (attention/compute/storage/…)" validated against `SUBSTRATE_SIGNALS` — the care/compute discrimination axis exists at the entry level.
- **A minting kernel is LIVE.** `elohim-storage/src/services/token_mint_service.rs` — `mint_for_recognition` (micro tier: deterministic weight × allocation_ratio × mint_rate) writes an immutable `token_mint_events` row + credits a balance; and **`discernment_mint`** (Tier 2): "elohim agents witness cross-domain patterns — consistency across time, cascading impact, stewardship beyond any single event ... The observation itself is the source of truth" (token_mint_service.rs comment ~line 145). This is *exactly* the substrate-floor/elohim-ceiling split: micro-mint is deterministic substrate; discernment-mint is the ceiling's judgment, with a required `elohim_attestation` + `reasoning_trace` audit chain.
- **The observe → mint PIPELINE is LIVE.** `recognition_pipeline_service.rs` — five named stages: Normalize → Resolve → Weight → **Limit (floor/ceiling)** → Settle. Stage 4 (`apply_limits_with_config`, line 276) implements **the donut in code today**: `floor_ratio` (minimum share, "no contributor below dignity floor"), `ceiling_ratio` (maximum share, "no steward captures unbounded"), with **excess-from-ceiling redistributed proportionally to non-capped stewards** (lines 303-396). That redistribution IS the regenerative-commons mechanic. Stage 5 settles to `economic_events` + accumulates recognition.
- **The Observation primitive is LIVE.** `elohim-storage/src/observation/{wire,log,projector}.rs` — peer-witnessed evidence; the observer's iroh-blob append-only log (`log.rs`, rolling BLAKE3 root) is the source of truth, SQL is the projection (`projector.rs`, idempotent on `(observer_cid, log_cid, log_offset)`). The wire `Observation` (wire.rs:21) carries `observation_kind`, `subject_cid`, `payload_json`, observer context (household/collective/region/archetype/compute_class), and a `signature`. This is the witnessing substrate the vision demands.

### WHERE THE LIMIT LIVES (the exact seam)

The capability gap is NOT "we can't mint care." It is that **the live minting pipeline is wired to the *learning-recognition* path, and the *care-observation → care-mint* seam is not closed.** Concretely, three precise gaps:

1. **The observe→mint bridge is absent for care.** `recognition_pipeline_service.rs:32` `RecognitionTrigger` keys on `content_id` + learning `event_type` (`content-view`, `citation`, etc. — `token_mint_service.rs:46-57` `event_weight`). There is **no path from an `Observation` of kind `care:*` → a `RecognitionTrigger` → a mint** classified `care-token`. Care observations land in `observations` (projector.rs) and stop. The pipeline's input newtype is content-shaped, not observation-shaped.

2. **The care/compute isolation is documented discipline, NOT structural enforcement** — confirmed at the schema layer. Resilience README:468 states it plainly: `RESOURCE_CLASSIFICATIONS` (lib.rs:271-290) "mixes 16 values across both classes"; the compute-vs-care isolation "enforces it through the projection layer's discipline ... rather than through a typed schema partition." A care-mint pipeline that reads from the same flat classification list as compute breach has *no validator that can reject a crossing*. This is the named "closing edge" (README:468) and a care-minting pipeline makes it load-bearing for the first time.

3. **Minting policy is hardcoded, not governed.** `token_mint_service.rs:24` `DEFAULT_MINT_RATE: f32 = 1.0`; `:25-28` the four `WEIGHT_*` constants; `recognition_pipeline_service.rs:401` `apply_limits` v0 is a *passthrough* (no floor/ceiling) — only `apply_limits_with_config` takes ratios, and nothing wires governed ratios into it. The donut's floor and ceiling are *parameters with no policy source*. This is where capture-resistance lives or dies: a hardcoded mint rate is a substrate constant a future operator could quietly change; a *governed* mint policy is a Mishpat decision with an audit trail.

**The diagnosis (ARC-style):** "Care can't be minted as value" is a *wiring-and-policy artifact*, not a substrate physics limit. One layer down, every primitive exists — care-token classification, the Observation log, the mint kernel, the donut limit-stage. What's missing is (a) the **observation→trigger adapter for the care-class**, (b) a **typed care-class/compute-class partition** so the isolation is structural, and (c) a **governed-policy source** for mint rate / floor / ceiling. The substrate already speaks care; it does not yet *govern* care-minting or *isolate* it by type.

---

## 3. PATH / PIVOT / FORK LADDER (cheapest → deepest)

### Rung 0 — Wire the care observe→mint adapter (BUILDABLE NOW, no DHT change)
**Cost:** ~1 service module + 1 reconcile route. **Blast radius:** elohim-storage only.
Add `CareObservation` as an `observation_kind` namespace (`care:childcare`, `care:eldercare`, `care:meal`, `care:transport` — declared in a pillar manifest under `observation_kinds`, per elohim-storage/CLAUDE.md). Add an adapter: when the `ObservationProjector` lands a `care:*` row, the `ReconcileController` routes it to a new `CareRecognitionTrigger` (sibling to `RecognitionTrigger` but keyed on `observer_cid`/`subject_cid` + witnessed-care payload, classified `care-token`) through the *existing* pipeline (Normalize→Resolve→Weight→Limit→Settle). Stage 5 settles an `EconomicEvent` with `substrate_signal: "care"` and `resource_classified_as_json: ["care-token"]`, then `mint_for_recognition` mints into the agent's balance.
**Unlocks:** Margaret's 25 hrs/week becomes a witnessed, minted, queryable recognition flow — the single most load-bearing vision payoff — **without any DNA change.** The donut's floor/ceiling apply via `apply_limits_with_config`.

### Rung 1 — Make minting policy GOVERNED, not hardcoded (BUILDABLE NOW + a new signal_kind)
**Cost:** policy-loader + a `Commitment`-shaped mint-policy record. **Blast radius:** storage + one `signal_kind` whitelist edit.
Replace the hardcoded `DEFAULT_MINT_RATE` / `WEIGHT_*` / passthrough-limit with a `MintPolicy` resolved per accounting scope (per collective/qahal). The policy IS a governed artifact: a Mishpat governance decision (proposal → vote) that sets `{mint_rate, floor_ratio, ceiling_ratio, event_weights}` for a scope. Notarize the *policy decision* as an existing governance entry; project it into a `mint_policies` table the pipeline reads. Mint rate becomes auditable + revocable. The substrate-floor stays deterministic (it *evaluates* the policy); the elohim-ceiling *sets* it.
**Unlocks:** capture-resistance — no one can quietly re-weight care. Different collectives can run different donut geometries (a caregiving co-op floors care-token high; a learning collective weights citation high). Governance contracts literally "set policies."

### Rung 2 — STRUCTURAL care-class / compute-class partition (FORK CANDIDATE — DNA validator change)
**Cost:** typed split on `RESOURCE_CLASSIFICATIONS` + validators that reject crossings. **Blast radius:** elohim DNA integrity zome → DNA hash change → network event (per dna/CLAUDE.md). This is the genuine fork-tier commitment.
Today (README:468) the 16 classifications are flat; isolation is review-discipline. Partition them into a typed two-class union — `CareClass = {care-token, time-token, stewardship, recognition, attention, creator-token, learning-token}` vs `ComputeClass = {compute, infrastructure-token, steward-token, currency}` — and add a validator that **rejects an `EconomicEvent` whose `substrate_signal` (care/compute) contradicts its `resource_classified_as` class.** A compute-breach FeedbackSignal then *cannot* debit a care-token attribution; the validator refuses the crossing at the DHT floor.
**Unlocks:** the resilience epic's central honesty hedge (README:468) goes GAP→LIVE — "a hardware failure cannot silently re-rank a contributor's standing" becomes *structurally true*, not just disciplined. This is the highest-integrity rung and the one that earns the trust the trust-economy clause requires.

### Rung 3 — Care-as-REA-COMMITMENT (the operator-native pivot, mirrors the ARC arc-as-commitment move)
**Cost:** instantiate the compute-commitment primitive for care. **Blast radius:** seed-data + signal_kind, NO new entry type.
This is the deepest *aligned* move, and it's the exact analog of the ARC worked example's arc-as-coverage-commitment. Just as arc became `Commitment(action: hold-arc-range)`, care becomes `Commitment(action: provide-care)` between two Agents (Margaret → the household/child's collective), classified `care-token`, with reciprocal obligations and a state lifecycle. The *observed care* (Observation log) **fulfills** the commitment (an `EconomicEvent` with `fulfills_json: [commitment_id]`, `bounded_by: commitment_cid`). Minting is then *settlement of a witnessed fulfillment of a governed commitment* — the richest, most capture-resistant shape. Care-as-commitment ≡ compute-as-commitment ≡ arc-as-commitment: **one substrate, three instantiations** (extends the `project_rea_compute_commitment_primitive` generalization table by one row).
**Unlocks:** care enters the same bounded-reciprocity, on-chain-standing, revocable-audit-trail frame as compute and arc. The donut floor/ceiling become *commitment terms governed by the collective*. This is "collectives continue to serve the humans that use it" expressed as protocol law.

---

## 4. Recommended ESCALATION (defended) + what it COMMITS US TO

**Recommend: Rungs 0 + 1 now (buildable, this sprint-class), Rung 2 as a named DNA-fork roadmap commitment, Rung 3 as the architectural north for care.**

Defense:

- **Rung 0 is the felt payoff with zero protocol risk.** It closes the one seam the vision most demands (witnessed care → minted recognition) reusing live primitives. It is the cheapest possible "make Margaret visible." Do it first; it de-risks everything above it by proving the observe→mint path end to end on the care-class.
- **Rung 1 is non-negotiable for capture-resistance.** A care economy whose mint rate is a Rust constant is *not* capture-resistant — it has a single point of quiet capture. Governed mint policy is the minimum bar for the "capture-resistant stasis" clause. It costs one `signal_kind` and a policy table; it is the highest value-per-cost rung.
- **Rung 2 is the genuine fork commitment, and it should be ESCALATED as a roadmap item, not done reflexively.** It changes the DNA hash → network event → coordinated reinstall (dna/CLAUDE.md upgrade governance). It is on-mission (it's the resilience README's own named closing edge, README:468) but it is a *spend*. Recommendation: commit to it on the roadmap, sequence it with the next planned DNA-hash bump, and do NOT do it as an isolated migration. **This is the one item I escalate as a true fork decision requiring operator blessing** — a typed-partition validator is near-irreversible on a deployed DHT.
- **Rung 3 is the architecture we build *toward*, not a near-term build.** Declaring care-as-commitment as the north means Rung 0's adapter should emit `EconomicEvent`s shaped so they can *later* carry `fulfills_json`/`bounded_by` without rework — i.e., design the care-mint event with the commitment-fulfillment slot reserved, even before commitments exist (the ContributorPresence transfer-slot pattern: reserve the seam now, fill it later).

**What this COMMITS US TO:**
- **A new primitive instantiation (not a new entry type):** care-as-commitment is a new *row* in the `rea_compute_commitment_primitive` generalization table — `provide-care` action on the existing `Commitment` entry. Memory `project_rea_compute_commitment_primitive` gets one row added.
- **A roadmap fork:** the typed care-class/compute-class partition (Rung 2) — a DNA-hash-changing validator addition, sequenced with a planned reinstall, blessed by operator. Migrates resilience README:468 GAP→LIVE.
- **A governed-policy surface:** `MintPolicy` as a Mishpat-set, scope-keyed artifact — minting rate/floor/ceiling leave Rust constants and become governance decisions.
- **NO new DNA entry type for any of Rungs 0/1/3** — care-minting is `signal_kind` + `observation_kind` + `resource_classified_as` vocabulary on live primitives, the protocol's own extension discipline.

---

## 5. COUPLING — story + value + governance as one whole

This is where the technical serves the felt + economic + governance whole, and where the donut closes.

**STORY (the felt).** Margaret provides 25 hours of childcare. Her household-hub's elohim *witnesses* it — an `Observation` of kind `care:childcare`, signed, appended to the witness's iroh-blob log (the source of truth), `subject_cid` pointing at Margaret's presence. The witnessing is an *act of being seen*, not a price. The narrative layer (lamad/qahal stories) renders "Margaret's care is recognized by her community" — and that sentence is now *true at the substrate*, not aspirational. Care is seen *before* it is valued, which is the entire point: the protocol's posture is the inverse of the attention-auction (resilience README:35) — invisible-to-extraction, sharp-on-what-matters.

**VALUE (the minted, the donut).** The witnessed care fulfills Margaret's `provide-care` Commitment (Rung 3 north) → an `EconomicEvent` (`substrate_signal: "care"`, `resource_classified_as: ["care-token"]`, `fulfills_json: [commitment]`) → the recognition pipeline mints. The **donut is the Limit stage** (`apply_limits_with_config`): the **floor** guarantees Margaret's care clears a dignity minimum (no caregiver's witnessed hours mint to ~zero); the **ceiling** prevents any single steward from capturing unbounded recognition share, and **redistributes the excess to non-capped contributors** (lines 303-396) — the regenerative-commons mechanic, value flowing back into the ring rather than accumulating at the rim. Crucially, the minted care-token is **recognition, not currency at the moment of mint** — it credits standing/balance (`token_balances`), it does not auto-liquidate. Care does not become "just another transaction" because the *mint* is non-extractable; any downstream liquidity is a *separate governed* step, not the witnessing.

**GOVERNANCE (the policy, the capture-resistance).** The mint rate, floor ratio, ceiling ratio, and event weights are NOT substrate constants — they are a `MintPolicy` *set by the collective's qahal* through a Mishpat governance decision, scoped per accounting context (Rung 1). A caregiving co-op floors care-token recognition high; a learning collective weights citation. The substrate-floor *evaluates* the policy deterministically; the elohim-ceiling *sets and revises* it through governed proposal/vote. And the **care-class/compute-class typed partition (Rung 2)** makes the isolation structural: shem's power supply dying (a compute-class breach) *cannot* re-rank Margaret's care standing, because the validator refuses the crossing — not because a reviewer remembered to keep them apart. Capture-resistance is thus three-layered: the *witness* is signed and DHT-anchorable (can't be forged), the *policy* is governed and audited (can't be quietly captured), and the *class isolation* is typed (can't be contaminated by external messiness — a hardware failure, an externality).

**The fractal.** This shape holds at every reach: a household witnesses a member's care; a collective (church, co-op, factory-hub) sets its own donut geometry; the commons-reach floor is itself a governed policy. Hubs from households to factories "scale the sensemaking" by being the *accounting scopes* the MintPolicy keys on — the fractal stewardship is literally the per-scope policy resolution. One substrate, three instantiations (arc · compute · care), one observe→mint→govern loop, in stasis against the real world's messiness because every layer is witnessed, governed, and typed.

---

### Appendix — primary source citations (read, real)

- `content_store_integrity/src/lib.rs:271-290` — `RESOURCE_CLASSIFICATIONS` (flat 16, includes `care-token` :284)
- `content_store_integrity/src/lib.rs:1116-1154` — `EconomicEvent` (incl. `substrate_signal` :1150)
- `content_store_integrity/src/lib.rs:1381-1413` — `Commitment`
- `content_store_integrity/src/lib.rs:1425-1434` — `Appreciation` (recognition flows to creators)
- `elohim-storage/src/services/token_mint_service.rs` — live mint kernel (micro + discernment tiers)
- `elohim-storage/src/services/recognition_pipeline_service.rs:266-396` — 5-stage pipeline, donut Limit stage
- `elohim-storage/src/observation/{wire.rs:21, log.rs, projector.rs}` — witnessing primitive (iroh-blob log = truth, SQL = projection)
- `genesis/docs/content/elohim-protocol/resilience/README.md:44, :92, :468` — Margaret persona; commitment doctrine; the care/compute isolation closing-edge
- `.claude/memory/project_rea_compute_commitment_primitive.md` — the primitive care-as-commitment extends
- `.claude/memory/project_compute_commitments_bounded` — care/compute isolation invariant
