---
title: Coupling Delay as a First-Class Observed-and-Governed EPR Primitive (Beer's transport-lag)
id: coupling-delay-observed-governed-primitive-design
status: design
created: 2026-06-09
companion: 2026-06-09-per-substrate-limitarian-governor-design.md (cluster #1 — whose stability this half-closes)
depends_on:
  - 2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md (the anchor-3 floor this must not breach)
  - the observer epic (2026-05-11-observation-event-layer-design.md)
note: >
  Inline file:line are draft pointers against feat/native-content-graph-seam. cite-seal is the finishing
  step. Corrected against a three-lens adversarial review (control-theory / distributed-systems-honesty /
  capture-economics). The review caught a floor-ceiling breach in the original synthesis — fixes marked
  [adversarial-fix]. Governor-spec feedback edits (§"Feedback") are cite-gated, NOT free-text patches.
---

# Coupling Delay — Beer's Transport-Lag as an Observed-and-Governed EPR Primitive

> Stafford Beer's Viable System Model gives a worker authority over their subsystem's components, its
> inputs and outputs, **and the delays between one output and another input.** The protocol models *when*
> an event happened and *how long it lasted*, and it has the coupling *edges* (`input_of`/`output_of`,
> `fulfills`) — but never the **time *on* the edge**. This spec adds that missing **relational-duration**
> type. It is honest about a hard boundary: the delay the governor can *enforce* and the delay the VSM
> thesis is *about* sit on opposite sides of the P2P clock, and no attestation buys across it in the
> regime that matters.

---

## 0. The honest one-paragraph version

Three corrections from the adversarial pass reshape the claims, so read them first. **(1) The genuine win is real but narrow:** *loop delay* (the governor's own sense→act lag) is honestly measurable because it never leaves one node — making it observable-and-alarmed converts the governor's timing failure from *invisible-and-unbounded* into *measured-and-alarmed*. Worth shipping. **(2) "Cadence = measured delay budget" does NOT close cluster #1's stability argument — it closes the observability half and *relocates* the threshold half:** `τ_loop` is measured, but the bound `τ_loop < β·T_c` has `β` hand-set, `T_c` dependent on the undefended inflow exponent `ε`, and the stability *form* asserted by analogy (the real super-linear delayed-integrator is a nonlinear DDE whose global stability is an open question). **(3) The original mechanism breaches the wisdom spec's anchor-3 floor** ("the AI cannot actuate, only recommend"): deriving the decay tick period from an *Elohim-estimated* `T_c` hands the Elohim the **system's clock** between quarterly ratifications — the capture-via-latency attack wearing Beer's halo. The fix forces the `T_c`/inflow estimate to be a human-ratified, honesty-classed setpoint. **And the deepest structural truth:** the delay the governor can enforce with teeth (intra-node) and the cross-agent transport lag the Liberty Machine is *about* are on opposite sides of the no-global-clock boundary — Beer's fulfillment is real for intra-agent/same-chain subsystems and is *advisory-only* for the inter-subsystem lags Cybersyn was about. You cannot seize the global clock; you can only tend the local loops and make the rest witnessable.

---

## 1. The gap: a missing relational-duration type

**Modeled today** (verified): `EconomicEvent.has_point_in_time` (when), `has_duration` (the event's own span), `Process.has_beginning`/`has_end`, `Commitment.due`, Mishpat `valid_from`/`valid_until`. The **coupling edges** exist: `input_of`/`output_of` through a `Process`, `fulfills_json` from a realizing event to its commitment (`rea_projection.rs:174-205,258-294`).

**Absent:** the *time on the edge*. Every temporal field is an **instant or a span on a single record**; none is a **relational duration** belonging to a *coupling between two records* — no output→input transport lag, no `realized − committed` fulfillment latency. The only "lag on a coupling" measured anywhere is the projection-pipeline lag `ProjectorLagView` (`projector/status.rs:53-67`, `compute_lag_seconds` `:148-160`) — and that is the reuse anchor, because it is *also* the only honest one (below).

The governor (cluster #1) is the live victim: `apply_decay` is HTTP-poke-only (`api/token.rs:114-128`, no periodic spawn), so the loop delay between observing a concentration breach and applying friction is **unbounded** — its §4 stability proof is a `τ=0` argument. *"A fresh metric driving an unapplied multiplier is a stale governor wearing a fresh face"* is exactly `τ → ∞`.

---

## 2. Delay is THREE distinct quantities — never conflated

| | EXPECTED | REALIZED | LOOP |
|---|---|---|---|
| what | the promised lag | the observed lag on a coupling | the governor's sense→act lag |
| source | `due − has_beginning` on **one** Commitment | `fulfilling.has_point_in_time − commitment.has_beginning` | `t_apply_decay − t_concentration_snapshot` |
| clock | none (arithmetic on one record) | **honest only intra-chain / observer-relative** | **single node — fully honest** |
| class | A (it *is* the Commitment) | A2 (derived-via-link, projection) | **C (node-local control state)** |
| stakes | self-asserted promise | honesty-class-gated | the governor's input |

`SLIP = REALIZED − EXPECTED`, meaningful only when both are honest. **LOOP delay is the favorable case** the governor depends on — both endpoints minted by one storage node's clock — and it is *not* economic realized delay; it must never merge with it.

**[adversarial-fix] EXPECTED is the `due` leg only.** `due − has_beginning` (fulfillment-deadline) and `valid_until − valid_from` (authority-expiry) are **different couplings** — a `delegates-compute` commitment's `valid_until` is when the delegation lapses, not when an output was promised. SLIP is only meaningful against a *fulfillment deadline*. v1 uses the REA `due` leg; the Mishpat authority-window is a separate quantity, out of scope for slip.

---

## 3. The no-global-clock resolution (load-bearing honesty)

Every economic timestamp is **caller-supplied** (`commitments.rs:17-22` deliberately replaces `sys_time()` with a caller-controlled `signed_at`); agent clocks skew **hours** (`feedback_subagent_liveness_clock_skew`). So `output.has_point_in_time(A) − input.has_point_in_time(B)` is `duration ⊕ unbounded_skew` — garbage of unknown sign. A four-state honesty enum is what survives (`ProjectorLagView`'s tri-state extended):

```
measurability: SameChain | ObserverRelative | CrossAgentBounded | CrossAgentUnmeasurable
realized_secs: Some ONLY for SameChain & ObserverRelative   // cross-agent point-magnitude REFUSED
skew_bound_secs: Some for CrossAgentBounded → interval [realized−2ε, realized+2ε]
order: causal sign only (did input follow output) — fail-closed if violated
```

1. **`SameChain` — magnitude honest *against skew*, NOT against a lying signer [adversarial-fix].** Commitment + fulfilling event on one agent's chain share one clock under one signature. But `signed_at` is caller-controlled and **backdatable** — an agent can fabricate its own slip to dodge a debit. So same-chain magnitude is *skew-honest + non-repudiably-attributed, self-report-trusted.* **A debit-bearing slip MUST be a *witnessed* same-chain slip** (an `ObserverRelative` co-witness), never a bare self-asserted one. (`ProjectorLagView` is honest here because a node is not adversarial to *itself*; an economic actor is.)
2. **`ObserverRelative` — magnitude honest, scoped to the witness.** One witness's own `observed_at` for both endpoints → a real receive-delta; explicitly *that observer's* lag, confidence rising with observer diversity.
3. **`CrossAgentBounded` — interval only**, inside an attested skew envelope `ε`; `[realized−2ε, realized+2ε]`, never a point.
4. **`CrossAgentUnmeasurable` — no magnitude, ever.** Causal/sequence order only (Lamport gives order, never duration) + the expected delay from the single record. **The design refuses to emit a scalar here.**

**The honesty class gates the stake.** Same-chain witnessed slip → at most `DebitSoft` on repeat; cross-agent → ordering-only, at most `Advisory`. The selector must never assign a debit to a delay whose honesty class can't bear it.

**[adversarial-fix] Signed delta, not `max(0)`.** The economic leg must NOT inherit `compute_lag_seconds`'s `max(0)` verbatim — that clamp conflates *early fulfillment* (`realized < expected`, a good signal) with *causal violation* (bad). Compute a **signed** delta with an **explicit causal-order check separate from the magnitude clamp**.

---

## 4. The control leg — what it actually closes (and what it does not)

**[adversarial-fix] Demotion: this closes the OBSERVABILITY + ALARM half, and RELOCATES (does not eliminate) the threshold half.** Be precise about each.

**Closed (the genuine win):** `τ_loop = t_apply_decay − t_concentration_snapshot` is **fully honest** (single node), so:
- `apply_decay` gains a *cadence* (driven by the `ReconcileController`, `main.rs:2119-2178`) instead of being HTTP-poke-only — closing the unbounded-loop-delay gap.
- `τ_loop` is **computed in-controller from in-memory timestamps, NOT via the `observations` projection [adversarial-fix]** — otherwise the metric reporting your loop-delay is itself stale by the projection lag (the meta-delay recursion: `ProjectorLagView` measuring the pipeline the governor would measure economic lag *through*). Bypass it.
- `τ_loop = τ_observe + τ_actuate + **τ_ratify_writeback** [adversarial-fix]` — all three intra-node. The ratify-writeback term (community ratifies a new `C_target` → the actuator reads it) is **NOT a deferrable side question**; it is the *setpoint-propagation* half of the same loop, and it rides the **dead `ratified_by`/`ratified_at` seam** (governor `:72,:336`). A governor applying friction promptly against a *stale `C_target`* is the stale-governor pathology in its other guise. Closing that seam is a **blocking dependency**, not Open-Decision-optional.
- Over-budget (or the open-loop `None`-applied state) raises a NEW `SignalKind::DelayBreach` `FeedbackSignal` (B2, signed) that **bypasses the quarterly re-ratification cadence** — a loop running slower than its stability bound cannot wait.

**Relocated, not eliminated (the threshold half) [adversarial-fix]:** the stability claim `τ_loop < β·T_c(C_target)` is *not* a closed loop:
- `β` is **hand-set** (`0.3`, a convention) — a new free knob where `ticks_per_horizon` was.
- `T_c = 1/f'(b*)` is **never computed**; `f'(b*)` requires the fixed point `b*`, which requires the inflow `I = O(b^{1+ε})`, whose exponent `ε` cluster #1 §4.2 already declared an **unverifiable operator assumption.** "Every term lives in `LimitGradientConfig`" is false — `ε` does not.
- The stability *form* `τ < T_c` is **asserted by analogy.** The linear delayed integrator's real margin is `kτ < π/2` (`τ < ~1.57·T_c`); the governor's `f(b)` is **super-linear (`γ>0`) → a nonlinear DDE**, where the linear margin holds only locally near `b*` and the global behavior (limit cycles, not just divergence) is **uncharacterized.** The per-tick confiscation is a **sampled-data system** (ZOH adds `τ_budget/2` phase lag — itself part of the delay it is budgeted against).

**Honest statement for the spec:** *"We made `τ_loop` observable, bounded, and alarmed (real, on the stable floor). We did NOT derive the stability threshold for the super-linear-friction-over-GE plant; the threshold `β·T_c` is a relocated magic number — better-placed (a band humans ratify around a measured plant constant) but not eliminated, and the global super-linear-DDE stability is an open question."* The convergence test must include a **destabilization arm** (set `τ_budget > T_c`, observe ringing/divergence) — the only thing that validates the *control* claim rather than the *plumbing* claim.

---

## 5. The floor-ceiling fix — who is sovereign over the clock [adversarial-fix · the breach]

The original mechanism wired the decay tick period to `τ_budget = β·T_c(C_target)`, where `T_c` is an **Elohim-computed estimate** (from the observed inflow exponent) and `β` is hand-set, with the human ratifying only a quarterly band. **That breaches the wisdom spec's anchor-3 floor** ("the AI cannot actuate, only recommend", `…wisdom-layer-floor-ceiling…:201`): the Elohim's observation of `T_c` **actuates the system's clock** with no human in the per-tick loop. Whoever controls the `T_c`/inflow estimate controls the tempo — and can stretch the wall-clock between redistributions while the §4.3 energy invariant stays green: **the stale-governor failure reconstituted one meta-level up by the mechanism meant to cure it.** The `T_c` estimate — the most consequential quantity in the design — got *no* honesty class while a household chore's latency got all three.

**The fix is forced. Choose per substrate:**
- **(i) Human ratifies the period directly** (retain the wisdom anchor; lose adaptivity), OR
- **(ii) Human ratifies a *band*, the Elohim adapts within it, AND the Elohim's `T_c`-estimate drift *toward a band edge* raises a human-witnessable algedonic signal *before* it re-tempos** — a restoring force on the *meta-loop*, with its own delay budget. The `DelayBreach` fires on `τ_realized ≥ τ_budget` (a breach *inside* the band); this adds a **band-edge-approach** signal (the capture-relevant event).

**And:** the `T_c`/inflow-exponent estimate becomes a **stake-bearing, honesty-classed, human-ratified setpoint** — subject to the same discipline as every other governed quantity — *not* a free-running observation that silently sets the clock. Until (i) or (ii) is built, the honest statement is: **"the human is sovereign over the clock only at band-edges and only at re-ratification cadence; between those the Elohim sets the tempo within the band — a delegation of the clock, not retention of it."** Do not claim recognition-not-power for the cadence without building (ii).

**[adversarial-fix] Do not extend `validate_ratifies_limit_gradient`.** The wisdom spec proved it returns zero `.rs` hits — a citation-ring that "terminates nothing" (`…wisdom…:218,231`). You cannot extend a vapor validator into a runtime-`τ` check and call the result a deterministic floor. The delay-margin check is an advisory at ratification + the live `DelayBreach`, not a DNA wall, until that validator is actually built.

---

## 6. The data model (p2p-gated)

Zero new DHT entry types; one new wire artifact (`SignalKind::DelayBreach`).

| Quantity | Class | Identity | Source of truth | Coordinator/Signal | Reuses |
|---|---|---|---|---|---|
| **Expected** | A (it IS the Commitment) | Commitment CID = `entry_hash` (`project_mishpat_commitment_cid_is_entry_hash`) | DHT Commitment; SQL write-through | existing commitment coordinator; `due` projected | `rea_projection.rs:180` — no new field |
| **Realized** | A2 (derived-via-link) | content-derived `(commitment_cid, fulfilling_event_cid)` + kind + observer | DHT edges + endpoint records (NOT the SQL projection) | new `observation_kind`s via generalized `compute_lag_seconds` | `status.rs:148-160`, `observations` table, `observation_diversity_summary` |
| **Loop** | **C (operational, node-local — NOT projectable as an observation [adversarial-fix])** | `(snapshot_id, decay_application_id)` | storage node's own clock; ephemeral control state | `ReconcileController` decay tick | `compute_lag_seconds` shape, `concentration_snapshot`, `apply_decay` |
| **DelayBreach** | B2 (agent-scoped + attestation) | signal CID over `(target_cid, kind, signed_by)` | the signing agent | NEW `SignalKind::DelayBreach` on `/elohim/epr-atom/1.0.0` | `feedback_signal.rs:93-146`; `kind.rs:61` `&[Governance]` |

**[adversarial-fix]** Loop delay is **pure node-local C** — control state, *never* an A2 observation. If audit is wanted, the **B2 `DelayBreach`** carries it, not an `observations`-table row (the original "C but also projectable" waved at two classifications for one quantity).

**Observation leg:** two `observation_kind`s — `economic:commitment-fulfilment-latency` (across `fulfills`) and `economic:coupling-delay` (across `output_of:P → input_of:P`) — projected into the existing `observations` table, k-anonymous over contributing agents before any standing-bearing use (`observation_diversity_summary` thresholds).

---

## 7. VSM / Liberty Machine — the honest verdict

Cybersyn died of **variety-starvation**: modeling each subsystem's I/O was too slow/costly, so the regulator could never match the environment's variety (Ashby), and it degraded into a centralized ops room. On-demand human+Elohim co-authoring is the variety amplifier it lacked — **but [adversarial-fix] it amplifies the variety of *representation* (sensing/modeling), not the variety of *response* (actuation).** The teeth-bearing actuator is intra-node loop delay; honest cross-agent actuation does not exist. So co-authoring **closes Cybersyn's modeling-speed gap, NOT the Ashby response gap** — "for the first time the Ashby math can close" is an over-claim; the response half remains exactly where Cybersyn left it.

**The structural mismatch (the centerpiece finding, not a footnote):** *the delay the governor can enforce with teeth (intra-node loop delay) and the delay the VSM thesis is about (cross-agent transport lag — A's output sitting idle before B consumes it) are on opposite sides of the P2P clock boundary.* The teeth live where the thesis doesn't. And **bounded-skew attestation does not rescue it:** the oscillation-causing delays are on the order of `T_c`, which *shrinks* under preferential attachment exactly when concentration is high — so the skew window `±2ε` swamps the quantity precisely in the regime where regulation matters most. **The skew window is wide exactly when the time-constant is short** — structural, not a parameter choice.

**So the answer to "is continuous human+Elohim co-authoring the fulfillment of the Liberty Machine?":** Yes, **for subsystems whose viability-critical delays are intra-agent or same-chain** (a household's own commitment→fulfillment loop; a node's own governor) — conditional on (a) modeling the delays and (b) the floor-ceiling fix of §5. For the genuinely **inter-subsystem** transport lags Cybersyn was *about*, the substrate offers **causal ordering + human-ratified expected budgets + advisory breach-flags — never an automatic regulator with teeth, and no attestation buys across in the regime that matters.** That is the honest steady state, not a v1 expedient. You cannot seize the global clock; the cadence, too, returns to human recognition.

---

## 8. Feedback into the limitarian-governor spec (cite-gated, not free-text)

These edits to `2026-06-09-per-substrate-limitarian-governor-design.md` **must run through cite tooling** (`feedback_managed_surface_edit_discipline`), not hand-patches:
1. New **§4.5 "Stability with loop delay"** — the delayed integrator `db/dt = I(t) − f(b(t−τ))`; the two-condition loop-closure (energy AND timing); §4 re-framed as the `τ=0` special case; **the honest demotion** (threshold relocated, super-linear-DDE global stability open).
2. `ticks_per_horizon = horizon / τ_budget` (derived) **+ the §5 floor-ceiling fix** (the `T_c` estimate is a human-ratified honesty-classed setpoint, or human ratifies the period).
3. Wire `apply_decay` onto the `ReconcileController` cadence; **close the `ratified_by` writeback seam as a blocking dependency** (it's a `τ_loop` term).
4. Add `SignalKind::DelayBreach` + the band-edge-approach signal (§5 (ii)).
5. The convergence test gains the **destabilization arm** (`τ_budget > T_c` → ringing) and **re-applies inflow each tick** (mirror governor `:578-589`) — a decay-only test proves nothing.
6. Correct the stale T19 framing (T21 wired the tending sweep; the gap is governor-driven cadence).

---

## 9. v1 slice (household-nodes, no shem)

Intra-node loop + same-chain only; cross-agent stays ordering-only, out of scope. **One closed-loop test, one node:**
1. `apply_decay` fires on a clock without an HTTP poke (cadence gained).
2. `τ_loop` measured **in-controller** (not via projection), `Some(n)`, `n < τ_budget`.
3. Concentration monotonically descends toward `C_target` across ≥2 ticks **with rich-get-richer inflow re-applied each tick** (mirror governor `:578-589` — a decay-only test is vacuous).
4. **Destabilization arm:** set `τ_budget > T_c`; assert the plant *rings* (validates the control claim, not just plumbing).
5. Breach path: stall the tick → assert a signed `SignalKind::DelayBreach` with `τ_realized`/`τ_budget` populated.

Companion same-chain economic assertion: a household commitment with authored `due`, a co-authored fulfilling event past `due`, projected `economic:commitment-fulfilment-latency` with `measurability: SameChain`, `slip_secs = Some(positive)` — but **the debit is withheld unless the slip is witnessed** (§3 self-report caveat).

---

## 10. Open decisions for operator

1. **Floor-ceiling fix (§5): (i) human ratifies the period, or (ii) band + band-edge-approach algedonic signal?** This is a soundness decision, not preference — until one is built, the Elohim is sovereign over the clock between ratifications.
2. **`β`, `T_c`-estimate honesty class, skew-bound `ε`** — all TBD-operator; `T_c`-estimate must carry a honesty class (§5).
3. **`DelayBreach` default `standing_impact`** — Advisory default, DebitSoft only on *witnessed-repeat*; cross-agent never past Advisory.
4. **Bounded-skew attestation source** — recommend **none** (cross-agent stays ordering-only). The §7 finding says attestation can't rescue the high-concentration regime regardless.
5. **Ratify-writeback latency** — fold into `τ_loop` (it's intra-node, measurable) — recommend yes, as a blocking dependency, not deferred.
6. **Stability-condition enforcement** — advisory-at-ratification + live `DelayBreach`; **NOT** a DNA-validator wall extending the vapor `validate_ratifies_limit_gradient`.

---

## 11. Hardest unanswered tradeoff

**The quantity the governor most needs is the one fully honest; the quantity that fulfils the Beer/VSM vision is the one fundamentally unmeasurable P2P** — and they are not the same quantity. The control leg closes on `household-nodes` precisely by *not* depending on the cross-agent delays the Liberty-Machine framing is about. There is no third option yielding honest cross-agent magnitude with zero shared-time assumption (Lamport gives sequence, never duration). **Recommendation: accept cross-agent delay as permanently advisory** (ordering + human-ratified budgets + breach-flags; human re-ratification, never an automatic debit, is the only cross-agent enforcement) — because it preserves the no-trust floor and keeps the Elohim in recognition-not-power. The nested second-order tradeoff (the cadence-estimator's own honesty) resolves the same way and is the §5 fix: **humans ratify the band; the Elohim adapts within it and must *signal before* it steers to an edge; and how wide the band is becomes one more operator TBD parked exactly where cluster #1 parks every numeric.** The fruit goes back on the tree here too: even the system's clock is not the AI's to seize.
