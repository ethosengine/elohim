---
title: "Wisdom Layer — Floor-Meets-Ceiling Judgment (culminating synthesis)"
id: wisdom-layer-floor-ceiling-judgment-culminating-design
date: 2026-06-09
status: design
cluster: wisdom (culminating)
predecessors:
  - 2026-06-09-per-substrate-limitarian-governor-design.md
  - 2026-06-09-cluster2-sacredness-surface-firewall-anti-capture-design.md
  - 2026-06-09-cluster3-substrate-signal-migration-governance-signal-flow-design.md
  - 2026-06-09-wisdom-layer-stakes-mechanism-selector-design.md
requires_env: household-nodes
---

# Wisdom Layer — Floor-Meets-Ceiling Judgment

> This is the **culminating synthesis** that integrates the four wisdom-layer design areas
> (the stakes→mechanism selector, the recursive-accountability model, the floor↔ceiling seam,
> and the v1 stub) into one coherent design. It does **not** invent a parallel judgment
> substrate. It generalizes the one already in-tree — `ElohimGate`'s `InferenceTier::classify`,
> the `constitution` crate's `PromptAssembler` JSON contract, and the `eae`
> subsidiarity/escalation/precedent machinery — and tops it out with three additions: a stakes
> dimension orthogonal to trust, an anonymity-mode output on the existing selector, and a
> notarized `JudgmentCall` record that makes the selection itself witnessable.

> **Doc topology.** The stakes→mechanism *selector mechanics* are specified at their own altitude
> in the sibling `2026-06-09-wisdom-layer-stakes-mechanism-selector-design.md` (Design Area 1).
> This doc is the altitude *above* the selector: it owns the recursive-accountability model and
> its termination, the floor↔ceiling seam, human representation, the parked-decision resolutions
> framed as floor-ceiling judgments, and the honest v1 stub. Where the two overlap they are
> coherent — this doc only *refines* the selector doc's PassThrough-on-no-router default to be
> stakes-aware. They may land as two docs or merge editorially; no contradiction exists.

Citations are `file:line` against `feat/native-content-graph-seam`.

> **[adversarial-correction, 2026-06-09]** This doc was rewritten after a three-lens adversarial review.
> The first draft claimed the recursion terminates at **three structural anchors** stated in the present
> tense. The review verified against the tree and found that two of the three **do not exist in code**:
> Terminator 1 (`validate_ratifies_limit_gradient`) returns **zero `.rs` hits** — it is a *citation ring*
> the six 2026-06-09 design docs cite each other for; Terminator 3's ledger (`PrecedentStore`) is an
> **in-memory `HashMap`** (`precedent.rs:34`), not the replicated record the proof leans on. The honest
> live terminus today is a **single one: a human pressing `Pause{confirm_token}`** — and in the
> adversarial small-cohort case that degrades to *a peer who notices* — a **recognition act, not a
> structural guarantee.** The termination section below is corrected to mark **built vs designed**, and
> the spec now states that truth at the top rather than burying it in the closing paragraph. *There is no
> wall high enough to be the guardian; the terminus is a human's recognition, held in trust, and the
> architecture's job is to make that recognition witnessable — not to replace it.*

---

## Problem & framing

The operator surfaced three realizations that *force* this layer into existence:

1. **The floor-ceiling pattern is UNIVERSAL.** Every governed quantity in the substrate has the
   same shape: a **deterministic floor** (a minimum guarantee that holds with no LLM, no network,
   no human) that hands off to a **stakes-sensitive judgment ceiling** (an interpretation *within*
   the band the floor opens). This is not per-decision plumbing — it is the one dispatch shape the
   substrate already implements twice (the `eae` Decider's rule-floor→LLM-ceiling at
   `eae/src/mace/decider.rs:234,303`; `ElohimGate`'s `InferenceTier::classify`→`GateResult` at
   `elohim_gate.rs:79,203`).

2. **The accountability/anonymity MODE is a STAKES-SENSITIVE JUDGMENT, not a constant.** "Where
   are we going to lunch" wants an anonymous k-anon poll; a constitutional ratification wants a
   fully-attributed roll-call. Today the substrate has both primitives — k≥5 `Suppress`
   (`aggregator.rs:90`) and attributed `issuer_cid` in the `attestations` table — but **no code
   path selects between them by stakes.** Cluster #3 Decision 4 explicitly parks this here.

3. **Choosing the mechanism is ITSELF a judgment call — so the judgment process needs its own
   accountability model, and that model must TERMINATE.** This is the operator's recursive
   question: who guards the guardians, without infinite regress? The answer cannot be "a higher
   council," because Clusters #1–3 proved altitude makes capture *cheaper* — `can_override` is
   pure ordinal precedence with no direction/content check (`types.rs:46`), so the apex `Global=7`
   has no structural immunity.

The fact that these three questions surfaced *at all* means we are at the wisdom-cluster question
regardless: the parked decisions cannot be hardcoded as constants; they are stakes-sensitive
judgment calls, and the judgment process that makes them needs an accountability model with a
non-recursing bottom. This doc supplies that model and shows where it terminates.

---

## The existing wisdom substrate (what's real, what's stubbed)

The substrate is one coherent design across four strata. Asymmetric maturity — the *seams* are
real; the *judgment content* is honestly stubbed.

**BUILT (shipping, tested, runtime-wired):**
- **The stakes→ceremony selector** — `InferenceTier::classify(mutation, ctx)` (`elohim_gate.rs:79`),
  a pure `match` on `MutationType × composite_trust` → `{None, Light, Full, Constitutional}`.
  `ReachChange → Constitutional` always (`:133`); `GovernanceVote → Full`-or-`Constitutional` by
  trust (`:126`); `intent_divergence > 0.5` escalates one tier (`:83`). Unit-tested.
- **The floor↔ceiling seam** — `GateResult::{PassThrough, Enriched, Pause{confirm_token},
  Settlement{appeal_path}}` (`:203`). `Pause` is the human-confirm friction moment; `Settlement`
  carries the appeal route. Wired live: `api/mod.rs` dispatches mutations through the gate;
  `Services::new` builds an `InferenceRouter` over `SidecarEngine` (`services/mod.rs:178-199`).
- **The judgment interface** — `PromptAssembler::build_reasoning_prompt` (`prompt.rs:77`) emits the
  JSON verdict contract `recommendation ∈ {approve, deny, escalate, defer}` + `values_weighed` +
  `confidence` + `precedents` (`prompt.rs:108`). The interpretive guidance hard-codes the
  sovereignty clause: *"Flag ambiguous cases for human deliberation rather than deciding"*
  (`prompt.rs:62`) and *"Log your reasoning for audit and precedent building"* (`:63`).
- **The constitutional lattice** — `ConstitutionalLayer` Individual=1 … Global=7,
  `can_override = self.precedence() > other.precedence()` (pure precedence, `types.rs:46`);
  `EnforcementLevel::{HardBlock, RequireGovernance, SoftLimit, Warning}` (`types.rs:226`);
  `ConflictResolver` with the one specialization carve-out (lower layer wins iff higher
  immutability, `conflict.rs:111`); `PrecedentStore` cite-weighted case-law (`precedent.rs:164`).
- **The accountability machinery (designed, in `eae`, NOT wired to the gate path)** — subsidiarity
  (`subsidiarity.rs`), escalation that terminates at `next_layer(Global) → None`
  (`escalation.rs:173`), `DecisionType::DeferToHuman` "appropriate at any level"
  (`subsidiarity.rs:111`), and a *real* LLM decider `decide_with_llm` (`mace/decider.rs:303`).
- **The mode inventory** — k≥5 `Suppress` (`aggregator.rs:90`, default `AggregatorConfig`);
  attributed `issuer_cid + vote_value` in `attestations`; `Visibility::Private` source-chain;
  the governance pipeline `propose→vote→tally` (`governance_action.rs:260`).
- **The stakes primitives, as separable axes** — `reach` (`epr_kind::Reach`, earned-at-authoring,
  never eroded; `ReachVerdict::Pending` is explicitly shaped as the hand-off seam to this layer);
  `standing` (`StandingScore::{Floor…Trusted}`, never-demote, `standing.rs with_lift:56`); REA
  `Commitment` (bounded reciprocity, breach→`FeedbackSignal`, `revokes-commitment` action).

**STUBBED (shaped honestly, deliberately deferred):**
- **The reasoning *content*.** `ElohimReasoning` is documented "Placeholder for Sprint 2 — will
  carry LLM reasoning" (`elohim_gate.rs:245`). The storage-side `InferenceRouter` returns
  `Unavailable` when no engine serves (`inference_router.rs:47`) and the gate degrades to
  `PassThrough` — the intended safe-default. Only `MockEngine` exists on the storage path.
- **`eae` is library-only** — grep confirms no `eae::` in `elohim-storage/src`. The recursive-
  accountability *termination* is designed but **not on the gate path.** This is the single biggest
  designed-but-disconnected gap.
- **Boundary checking is a deterministic keyword scan** (`stack.rs:266`); `HardBlock` is pre-AI and
  inviolable.
- **`PrecedentStore` is in-memory** (`precedent.rs:34`) — not DHT-notarized.

**ASPIRATIONAL (narrative only):** sortition, Constitutional Councils, appeal-cascade routing,
7-year conventions (`governance/epic.md`). The code implements only the 7-layer ordinal.

The honest reading: **the seam is built; the judgment is mocked — and the mock is correctly
shaped** (the gate already returns `Pause`/`Settlement` with appeal paths; only the reasoning fill
is absent). This is exactly the v1 stub the operator asks for.

---

## The stakes→mechanism selector (floor-ceiling gradient)

The selector **already exists** as `InferenceTier::classify`. This layer *generalizes* it with two
additions — it does not fork it.

**Addition 1 — an explicit `Stakes` axis, orthogonal to trust.** Today the selector keys on
`composite_trust` (a 6-signal blend, weights `W_*` at `elohim_gate.rs:162-167`). But **stakes ≠
trust**: trust answers *"how much do we let this actor self-serve"*; stakes answers *"how much does
getting this wrong cost, and how reversible is it."* Trust is an *input* to stakes, not stakes.

```
stakes = f( reach_affected,        // epr_kind::Reach — broader authoring authority at play ↑
            standing_at_risk,      // StandingScore band touched — a demotion-shaped act is high
            rea_claims_bindingness,// Mishpat Commitment created/breached? — reciprocity at stake
            constitutional_layer,  // Individual=1 … Global=7 — altitude raises stakes
            reversibility )        // append-only/supersede = low; confiscatory/k_max = high
```

**Addition 2 — `AnonymityMode` as a first-class output.** The selector returns
`Selection { tier, anonymity_mode, stakes_class, witnessed_by }`, riding the existing `GateResult`
— no new result enum. `AnonymityMode ∈ {AnonAggregate(k≥5), Pseudonymous, PrivateWithAttestation,
AttributedRollCall}`.

**The lunch-poll ↔ constitutional-ratification spectrum:**

| Band | Example | tier | anonymity_mode | Human in loop? |
|---|---|---|---|---|
| Trivial | "where do we go to lunch" (`MasteryUpdate`-class) | `None` | `AnonAggregate(k≥5)` | No — `PassThrough` |
| Standing | curation, reactions | `Light` | `AnonAggregate` | No, unless escalated |
| Reach/ReaBound | `ContentPublish`, commitment create | `Full` | `Pseudonymous`/`PrivateWithAttestation` | On `Pause` |
| Constitutional | `ReachChange`, constitutional `GovernanceVote`, wall-width ratification | `Constitutional` | `AttributedRollCall` | Mandatory `Pause` + M-of-N |

**The invariant that ties trust and stakes together:** *trust can lower ceremony within a stakes
band, but never lower attribution below what stakes demand.* A maximally-trusted agent ratifying a
constitutional change is still `AttributedRollCall`. Trust buys ceremony relief; it never buys
accountability relief. **Escalation is monotonic with altitude** — a vote at `Global=7` is forced
to the most-attributed mode + super-quorum, precisely because capture cheapens as you ascend.

**[adversarial-fix] The missing deterministic floor: an act↔stakes consistency check.** The selector
classifies the **declared** `MutationType`, and `MutationType::Custom(_)` maps to `Full` (not
`Constitutional`), while `MasteryUpdate`/`SessionHeartbeat` classify to `None` and never gate. So a
capturing party can **launder** a high-stakes act into a low-stakes ceremony by mis-declaring its type —
*without ever lying about a single vote.* Nothing today cross-checks the declared class against the
**realized** reach/standing/REA delta the act produces. That cross-check IS the "deterministic floor that
always holds" the recursion needs: **reject any act whose realized delta exceeds the ceremony its
declared class received.** It is unbuilt, and it is the floor the whole termination argument silently
assumes. Build it before relying on stakes-classification as a guarantee.

---

## Recursive accountability of the judgment process — and where it TERMINATES

The judgment process has **five components**, all in-tree at asymmetric maturity. Each is itself
subject to a floor + ceiling. Accountability is *the floor's deterministic record-keeping* plus
*the ceiling's revocable/appealable authority*. Each component bottoms out at a **non-recursing
anchor** — never at a higher council.

| Component | In-tree | Accountable to | How | Terminates at |
|---|---|---|---|---|
| **1. Stakes-assessor** | `InferenceTier::classify` (`:79`), `TrustContext::compute` (`:171`) | the community that ratified the weights, via the precedent ledger | emits a `JudgmentCall` citing the in-force `stack_hash`; mis-classification is witnessable by diffing record vs config; weights are a ratified EPR | **the ratified config + its notarized record** — no meta-assessor; a config a community set and can re-ratify, and a log that makes drift visible |
| **2. Mechanism-selector** | `mechanism_selection` (`:38-57`), Manifest `mechanism_ladder` (`:96-152`) | the ratifying community | selection is a deterministic projection of `(governance-state × Manifest)`, reproducible and challengeable; the ladder is a governed EPR | **the Manifest-as-ratified-EPR** — reject-at-write prevents a mode the ladder forbids; the Manifest is bounded by a DNA wall it cannot move |
| **3. AI-proposer** | `build_reasoning_prompt` (`:77`) → `InferenceEngine`; `SidecarEngine` live, `MockEngine` stub | the human ratifier | emits only `recommendation` + `confidence` + `precedents` — advisory, never actuating; a bad proposal → breach → `FeedbackSignal` → standing impact → `revokes-commitment` | **revocation + structural inability to actuate** — the AI cannot guard itself because it cannot *act*, only recommend (`prompt.rs:62` is the architectural floor, not a guideline) |
| **4. Human-ratifier** | `Pause{confirm_token}` (`:217`), M-of-N tally (`governance_action.rs:260`) | the community + precedent ledger; at high stakes, attributed by `issuer_cid` | `confirm_token` requires a real human token; `RequireGovernance` can NEVER be auto-satisfied by an LLM | **the human-sovereign veto** — checked only *laterally* (M-of-N peers, attributed) and *temporally* (ledger), never *vertically* by a super-authority |
| **5. Appeal/challenge path** | `Settlement{appeal_path}` (`:224`), `eae` escalation ladder (`escalation.rs:165`) | the next layer up, ending at Global | `Settlement` always carries a route; `next_layer(Global)→None`; `DeferToHuman` reachable at every rung | **`Global→None` + DeferToHuman at every rung** — the ladder is finite (7 layers) and the human-veto drops the case out of automation at any rung |

### Where the recursion terminates — the rigorous answer

**What is live today (the honest floor):** of the three designed anchors below, **only the human-confirm
is on the gate path.** Terminator 1 is unbuilt, Terminator 3 is unpersisted, Terminator 5 is in an
unlinked crate. So the operative terminus in running code is **Terminator 2 alone — a human pressing
`Pause{confirm_token}`** — and the design's job for v1 is to make that one terminus *real and
witnessable*, not to pretend the other two already back it. The three-anchor model below is the
**target** termination; each anchor's build-state is marked. None is a higher council — altitude is
explicitly *not* a terminator (capture cheapens as you ascend, `can_override` pure precedence
`types.rs:46`). The design holds **iff** the anchors are built before the seam ships at scale; until
then, the recursion terminates at one human's recognition, witnessed by whatever the ledger can record.

**Terminator 1 — the DNA clamp wall (the only structurally non-capturable element).** ⚠ **DESIGNED,
NOT BUILT.** The validator this anchor names — `validate_ratifies_limit_gradient` — returns **zero
`.rs` hits across the tree**; it exists only as a cross-citation among the six 2026-06-09 design docs.
*As designed*, the single element no layer can move is the deterministic wall core sets once, enforced
reject-at-write; *every* judgment component would operate beneath it; *none* — not the Global apex, not
a human supermajority — could ratify outside it. The regress *would* stop here because the wall is **not
a participant in the judgment process** — a constant guarded by **replication** (every peer runs the
validator), not by an authority, so you cannot ask "who guards the wall." **But the wall is the single
most load-bearing unbuilt thing in this spec.** Until the Mishpat integrity-zome `validate_create` arm
that clamps the ratified band is written, this anchor terminates nothing — and the floor it is supposed
to enforce is read from **capturable inputs** anyway (see the act↔stakes laundering gap, above):
`StakesClass` reads `Standing::evaluate` (a local projection) and `reach_earning` (a *Manifest policy*
lookup, not a constant), so whoever writes the manifest policy controls the classification the wall is
supposed to bound. **Build the validator AND the act↔stakes consistency check before any sentence says
"deterministic floor that always holds."**

**Terminator 2 — the human-sovereign veto (recognition-not-power apex).** ✅ **THE ONE LIVE
TERMINATOR** (`Pause{confirm_token}`, `elohim_gate.rs:217`, wired). Where judgment *is* discretionary
(the interpretive band between boundaries), the buck stops at a human. This terminates the regress
because the human is **not a guardian who needs a higher guardian** — the apex is *recognition, not
accumulated power*. A human's ratification is checked only **laterally** (M-of-N peers, attributed) and
**temporally** (the ledger records it), never **vertically** — a fixed-point, not an upward regress.
**Two corrections the review forced:** (1) the claim that the AI "cannot actuate" because of
`prompt.rs:62` is a **misread** — `prompt.rs:62` is a *prompt string* ("flag ambiguous cases for human
deliberation"), the single most promptable, overridable, unauditable line in the file, **not** a
structural floor. The real floor — that an `Enriched`/`recommendation` can **never** satisfy
`RequireGovernance`/`confirm_token` — is *asserted in this spec but not yet coded in the gate*; it must
be enforced in `elohim_gate.rs`, not in prompt prose. (2) `confirm_token` is a bare `String` with **no
code distinguishing a human-minted token from a machine-minted one** — so the human-sovereign apex is,
as typed, the exact seam where a future auto-confirmer (or a disposition model — see the
termination↔scalability tension below) hollows out sovereignty without violating any type. **Type the
human-origin of `confirm_token` before calling this a guarantee.** Until then it is a discipline.

**Terminator 3 — the transparency/precedent ledger (the witness over time).** ⚠ **DESIGNED, NOT
PERSISTED.** `PrecedentStore` is an in-memory `Arc<RwLock<HashMap>>` (`precedent.rs:34`) — per-process,
lost on restart, **not** DHT-notarized, **not** replicated, **not** peer-readable. *As designed*, every
`JudgmentCall` would be a notarized, content-addressed, citation-weighted precedent (`cite` bumps
weight, `precedent.rs:164`) that makes judgment **witnessable after the fact** — converting "trust the
guardian" into "audit the guardian," with no auditor to recurse into because the record is append-only
and replicated. *Today it is none of those things.* The non-capturability the proof leans on (guarded by
replication, read by any peer) is exactly the property the in-memory store lacks. This anchor terminates
nothing until `JudgmentCall` notarizes into a persisted, replicated ledger (Open Decision #4: make the
`JudgmentCall` A-entry *the* precedent persistence, `PrecedentStore` its projection).

**Why this is not turtles-all-the-way-down:** the three terminators share one property — **none is
a discretionary authority that could itself be captured.** The DNA wall is a constant. The human
veto is a fixed-point. The ledger is an append-only record read by everyone. The captured gradient
(capture cheapens as you ascend the `ConstitutionalLayer` ordinal because `can_override` is pure
precedence with no non-accumulability, `types.rs:46`) is explicitly *excluded* as a terminator. We
do **not** say "the Global apex guards everything" — that is precisely the regress that fails.
Subsidiarity is sound only via the Pigouvian incentive-divergence argument (the smallest layer that
internalizes the externality), and that argument is itself recorded in the `JudgmentCall` and
challengeable, not assumed. The recursion terminates because at the bottom there is **no guardian —
only a wall, a human's final word, and a public record.** We witness the mode-selection but do not
select-the-selection-of-the-selection.

---

## The floor↔ceiling seam (humans sovereign; the immutable-wall + human-veto backstop)

The seam is the *already-typed* `GateResult` enum crossed with the `EnforcementLevel::
RequireGovernance` line. Below the line: a `match` and a validator. Above the line: interpretation
that must be witnessed and may demand a human.

| `GateResult` | Side | Meaning at the seam |
|---|---|---|
| `PassThrough{tier}` | **Floor** | Deterministic; no judgment. Also the safe-default when the router is absent/errors — degrade-to-floor, never fail-open. |
| `Enriched{...}` | Ceiling → proceeds | Judgment advised; floor never breached. |
| `Pause{prompt, confirm_token}` | **Human-sovereign (veto/confirm)** | A human confirm-token is *required* to proceed. No LLM output auto-satisfies it. |
| `Settlement{boundary, appeal_path}` | **Human-sovereign (appeal)** | A boundary settled the case; a real appeal route exists. |

**The honest-stub invariant (load-bearing refinement over the selector doc).** `PassThrough`-on-no-
router is the correct safe default **only for low-stakes classes**. For a `Constitutional` / `Reach`
/ `ReaBound`-class act with the ceiling absent, the gate must degrade to **`Pause` (human-in-loop),
NOT `PassThrough`** — because `PassThrough` on a high-stakes case is *"we faked judgment,"* the one
thing the stub must never do. Fail-safe-to-coherent, never fail-open.

The seam is the same everywhere: **deterministic code emits a witnessable record and routes the
contested case to a human or to appeal; judgment never crosses the `RequireGovernance` /
confirm-token line autonomously.** Boundaries (`HardBlock`) are pre-AI and inviolable — the LLM
operates in the `SoftLimit`/`Warning` band and the interpretive space *between* boundaries.

---

## Human feedback & representation (ubiquitous, intimate, yet scalable)

- **Ubiquitous (every household).** The floor runs at every node with no LLM — household-nodes is
  the stable floor; a laptop alone is a full participant. Every node classifies stakes and picks a
  safe mode deterministically; the human-veto is reachable at *every* rung via `DeferToHuman`, not
  only at deadlock. Sovereignty is not concentrated at the top; it is available everywhere the gate
  runs. No remote-model chokepoint can take it away.
- **Intimate (the human is bothered exactly in proportion to stakes).** At low stakes the human is
  *not* burdened — the `AnonAggregate` mode means a lunch vote needs no ceremony, no attribution,
  no ratification. The accountability *floor* still fires (the *selection* is witnessed via the
  `JudgmentCall`) without imposing on the participant. Friction is reserved for stakes that warrant
  it.
- **Scalable (the AI does interpretive labor; the human ratifies).** The AI-proposer ranks, weighs,
  and cites precedent at scale so humans ratify rather than originate — but the proposer can never
  *decide* a contested case. Representation also flows through the learned governance disposition
  (proxy voting, `disposition_service::compute_disposition`), which scales a human's *prior consent*
  without re-asking; the human can always override. Subsidiarity keeps the call at the smallest
  layer that internalizes it (`subsidiarity.rs:111`), and novel low-confidence cases escalate to
  Community-minimum (`subsidiarity.rs:134`) — the structural humility rule.
- **Sovereign — three irreducible points, all already typed:** (1) the **veto/confirm** —
  `Pause{confirm_token}`; (2) the **appeal** — `Settlement{appeal_path}`; (3) the **consent** —
  M-of-N ratification + consent-round-with-written-block-justification
  (`collective-governance.feature:22`), LLM advisory only. No LLM output crosses these lines.

---

## How this RESOLVES the parked decisions (each as a floor-ceiling judgment)

| Parked decision | Resolution as a floor-ceiling judgment |
|---|---|
| **(a) votes auditable-vs-private** (Cluster #3 Decision 4) | **Stakes-sensitive mode-selection.** `AnonymityMode` becomes a first-class output of the selector, not a global constant: `AnonAggregate(k≥5)` floor for `Trivial` ("lunch"); `AttributedRollCall` + read-path guard for `Constitutional` ratification. The *selection* is always witnessed (the B2 attestation); the *votes* are attributed only when stakes warrant. The value-premise (which band gets which mode) is supplied by the ceiling and governed in the Manifest. |
| **(b) wall-widths** (Cluster #1 Decision 2) | **Sociocratically set within a deterministic DNA floor.** The `[min, ceiling]` is core-set reject-at-write (`validate_ratifies_limit_gradient`); the *width within it* is a `LimitGradientConfig` Commitment ratified via `propose→vote→tally` — a `Constitutional`-class act → `AttributedRollCall` + super-quorum + `Pause`/appeal. There is no value-neutral width, so the elohim's value-premise→width derivation is **advisory input** to human ratifiers, never an actuator. |
| **(c) apex non-accumulability** (Cluster #1 Decision 1) | **Accountability-of-process, not altitude.** We explicitly decline to make `Global=7` a structural terminator (it has none — `can_override` is pure ordinal, `types.rs:46`, capture cheapens with altitude). The apex's judgments are bound to the *most-attributed mode* + super-quorum + a `loosening_acknowledged` deliberation window + a `JudgmentCall` on the precedent ledger. The backstop is the DNA wall + human veto + ledger — never the apex. The apex is *more* watched because it is *more* capturable. |
| **(d) k_max ratifiability** (Cluster #1 Decision 1) | **Stakes-bounded within a DNA floor.** A `k_max` change can weaponize demurrage (confiscate-all-in-one-tick), so it is the highest-stakes class → forced to `Constitutional` tier + `AttributedRollCall` + super-quorum, AND reject-at-write clamped to a DNA `[k_max_min, k_max_ceiling]` band the ratification cannot exceed. The floor prevents confiscate-all structurally; the ceiling debates the width inside it. |
| **(e) k≥5 under-measurement** (Cluster #1 §8.3 / Cluster #2) | **Judgment compensates where the measure is structurally blind.** Because `GE_assembled ≤ GE_true` in small high-capture cohorts, the k≥5 `Suppress` floor under-counts concentration. The selector treats **small-cohort + high-reach-concentration as a stakes-escalator**: it escalates the tier (the existing escalate-one-tier mechanism) and forces `PrivateWithAttestation` + human `Pause` — so capture in a 4-person cohort leaves an attested trail the aggregate cannot see. The k≥5 `Suppress` floor stays (never lowered to close the gap); the judgment ceiling compensates above it. |

---

## p2p-gate entity/signal summary

The one new entity — **`JudgmentCall`** — passes the p2p-design-gate as a **composite of existing
molds** (Mishpat headroom is tight at ~11/~100; no new DHT entry type):

- **Classification / outcome (A — notarized):** reuse a `governance-action` Content entry
  (`propose_governance_action`, `governance_action.rs:260`) or a Mishpat `Commitment`. Identity is
  **content-derived**: `CID = entry_hash` (consistent with `project_mishpat_commitment_cid_is_entry_hash`
  — the CID is the bounds-gate key, never the `action_hash`). Outcome reuses `PrecedentOutcome`
  (`Approved|Denied|Escalated|Deferred`, `types.rs:293`), attributed per the selected mode.
- **Private deliberation (B — agent-scoped):** the LLM reasoning trace + `values_weighed` as a
  `Visibility::Private` source-chain entry (the `ElohimReasoning` carrier, `elohim_gate.rs:245`),
  disclosure-tier-marked. Granular deliberation stays private; only the outcome is attributed.
- **Attested mode-selection (B2 — agent-scoped + attestation):** the *selection itself* (which
  tier, which mode, why) is a `FeedbackSignal`/attestation that witnesses the selection — the
  recursive-accountability hook made concrete. The *choice of mode* is a witnessed attestation.
- **Identity:** `hash(act_cid + stack_hash + selector_version)`. The `stack_hash` (`prompt.rs:66`)
  binds the judgment to the exact constitution version in force (currentness auditable).
- **Source of truth:** the DHT entry (notarized outcome + attested selection). SQL
  `attestations`/projection tables are write-through cache, not truth.
- **Coordinator/signal:** `propose_governance_action` creates the outcome; the existing
  `tally`/child-attestation projects it; the LLM verdict is consumed by `ElohimGate` (the
  swap-point is `MockEngine`→`SidecarEngine`→real engine). A `JudgmentCallWitnessed` signal
  projects the selection-event into a read view.
- **Floor enforcement:** the `JudgmentCall`'s mode is validated reject-at-write against the
  `mechanism_ladder` Manifest + the DNA wall — a selector cannot notarize a mode the ratified
  policy forbids.
- **Read-path gating:** the gate must run on the read paths into `attestations`, not only on
  writes — per-`issuer_cid` reconstruction is the surveillance surface (Cluster #3:159), gated by
  the selected mode.

---

## The honest v1 STUB

**Ships now on household-nodes (deterministic floor, no fake automation):**
- The **`JudgmentCall` notarized record** for all five components — the audit floor is the v1
  deliverable. Every classification/selection/ratification/appeal emits a witnessable, content-
  addressed record citing `stack_hash`.
- The stakes-assessor as `InferenceTier::classify` (`:79`) **extended to emit `anonymity_mode` +
  `stakes_class`** — pure deterministic `match`, no LLM.
- The mechanism-selector as `mechanism_selection` (already deterministic + Manifest-overridable).
- `AnonymityMode::floor_for(class)` backed by the *existing* k≥5 `aggregate_and_emit` and the
  *existing* `attestations` `issuer_cid` path.
- The human-ratifier `Pause{confirm_token}` + M-of-N tally (already wired); the appeal path
  `Settlement{appeal_path}` + bounded escalation ladder (`Global→None`).
- Boundary check stays the deterministic keyword scan (`stack.rs:266`) — `HardBlock` inviolable.

**Correctly-shaped stub for the AI-proposer (the one genuinely-not-automated component):**
- The interface is `PromptAssembler::build_reasoning_prompt` → `InferenceEngine::evaluate`. The
  swap-point is `MockEngine`→`SidecarEngine`→real Claude-backed engine. `eae::decide_with_llm`
  (`mace/decider.rs:303`) is the reference impl to port.
- The stub **must not pretend to automate judgment**: on no-engine/error, the gate degrades to the
  deterministic floor — **but stakes-aware** (the refinement): `PassThrough` is safe only for
  Trivial/Standing; `Constitutional`/`Reach`/`ReaBound` with the ceiling absent degrades to
  `Pause` (human), never `PassThrough`.
- `ElohimReasoning` (`:245`) is filled with the floor classification + a `mock-principle` marker so
  the *absence* of real reasoning is legible, never disguised.
- **Hard rule:** no stub output may cross the `RequireGovernance`/confirm-token line. v1 ships
  *human judgment with an empty AI assistant* — not *fake AI judgment*. The floor (record + route +
  human-veto) is real and complete; the ceiling (AI interpretation) is a shaped, fail-safe stub.

**Explicitly deferred (named, not faked):** the real LLM backend + budget path
(`inference_engine.rs:34 BudgetExhausted`, stale model IDs at `backend/anthropic.rs:59-69`); the
eae↔gate composition (eae is library-only today); JudgmentCall notarization into a *persisted*
PrecedentStore (in-memory HashMap today); sortition/Council substrate (narrative only).

---

## Open decisions for operator

1. **Where does the `JudgmentCall` B-deliberation entry live** — Mishpat (tight headroom ~11/~100)
   or the `elohim` DNA? The outcome reuses `GovernanceAction`/`Commitment`, but the private
   reasoning trace needs a home respecting `Visibility::Private`.
2. **eae vs ElohimGate canonicity.** Two judgment substrates consume `constitution` but don't
   compose; `eae`'s subsidiarity/escalation/precedent (the designed termination) is **not on the
   gate path**. Recommend: `ElohimGate` is the canonical *seam*, `eae` is *ported into it*
   (subsidiarity/escalation/precedent are the accountability machinery the gate lacks) — not two
   running judgment substrates. The wiring is a sibling implementation spec.
3. **Who attests the B2 mode-selection** — self-attestation (cheap, weak witness) or a peer-
   consensus quorum (`mace/consensus.rs`, currently stubbed broadcast)?
4. **Precedent ledger persistence.** `PrecedentStore` is in-memory; the transparency-terminator
   requires it DHT-notarized. Is the `JudgmentCall` A entry *the* precedent persistence, making
   `PrecedentStore` a projection?
5. **Who ratifies the mode-by-stakes policy itself?** It lives in the pillar-projection Manifest.
   Recommend pinning it `Constitutional`-class by definition (self-referential), so a community
   cannot quietly lower its own accountability floor.
6. **Pseudonymous-mode primitive is unbuilt** (Cluster #2 HMAC treatment). Recommend v1 collapses
   `Pseudonymous`→`AttributedRollCall` until it lands — over-attribution is the safe direction.
7. **`small_cohort_threshold`** for the k≥5 compensation is itself a wall-width-class unargued
   value — set sociocratically within a DNA floor of `>= k_threshold` (5)?
8. **Which `ConstitutionalStack` build path** — runtime uses simplified `build_defaults`
   (`stack.rs:215`), bypassing the full `ConflictResolver` and the DHT-verified `build` (whose
   verifier is itself a stub, `verification.rs:115`). Does the wisdom layer require the verified
   path?

---

## Hardest unanswered tradeoff

**Attribution is the very thing that enables both accountability and capture — and the transparency
that terminates the regress is in direct tension with the low-stakes anonymity that protects
participants.**

Resolving Decision 4 forces high-stakes votes to `AttributedRollCall` so the base can verify
one-agent-one-vote and hold the apex accountable — but `Attributed` is *also* the surveillance
surface (`attestations` holds `issuer_cid + vote_value` in clear), and a captured higher layer
(cheaper to capture, the whole Cluster-#1 finding) can read the roll-call to identify and pressure
dissenters. The same mode that makes the apex accountable to the base makes the base legible to a
captured apex.

The mirror tension: the recursion terminates partly *because* every judgment is witnessable on an
append-only ledger (Terminator 3) — but at low stakes we deliberately make votes *unattributed*
(k≥5 `Suppress`). So the *selection* is witnessed but the *inputs* are not. For low-stakes
judgments, **the ledger can prove a mode was selected but cannot prove the votes were not
manufactured** — a captured selector could pick "anonymous" precisely to hide ballot-stuffing, and
k≥5 `Suppress` *under-counts* concentration in exactly the small collectives where this is easiest
(`GE_assembled ≤ GE_true`). The small-cohort escalator (force `PrivateWithAttestation`) helps but
does not close it: a sufficiently small, sufficiently captured collective can choose anonymity to
evade the very transparency that terminates the regress.

**There is no attribution mode that simultaneously holds the powerful accountable and shields the
vulnerable from the powerful.** k-anon protects the dissenter but blinds accountability;
attribution holds the apex accountable but exposes the dissenter. The wisdom layer can only pick
*where on that curve each stakes-band sits* and make the choice itself witnessable. The DNA wall
(Terminator 1) is the only thing not on the curve — they cannot ratify outside it regardless of
mode — which is exactly **why every accountability chain in this spec terminates at the wall and
refuses to terminate in any attributed council, however high.** In the adversarial small-cohort
case, the operative floor is the wall and the human-veto (a peer who *notices* the pattern — a
recognition act, not a structural guarantee), **not** the ledger. That is faithful to
recognition-not-power, and it is the honest limit of what the substrate alone can promise.

**The second, deeper tradeoff [adversarial-fix]: the termination argument and the scalability argument
contradict each other, and only one can be true at a time.** The recursion terminates *because* the
human's final word (`Pause{confirm_token}`, M-of-N) is **non-delegable upward** — that non-delegability
is what makes it a fixed-point rather than a regress. But the scalability argument ("ubiquitous, intimate,
*yet scalable*") leans on `disposition_service::compute_disposition` — a learned proxy that "scales a
human's prior consent without re-asking." At scale, the thing standing at the termination anchor is then
**not a human's final word but a model's prediction of it**, ratifying a *different* model's proposal:
two AIs shaking hands at the fixed-point, called human sovereignty. And `confirm_token` as a bare `String`
is exactly the seam where that swap happens **invisibly** — nothing in the type says a human minted it
rather than a disposition model. You cannot have both "terminates at a non-delegable human" and
"representation scales by delegating the human." The honest resolution, and the v1 commitment: **the
disposition proxy acts ONLY in the low-stakes band; the human is non-delegable at `Constitutional`** —
which is sound *iff* (1) `confirm_token` is typed for human origin, and (2) the stakes-classifier
**cannot be gamed** into mislabeling a high-stakes capture as low-stakes (the act↔stakes consistency
floor, unbuilt). Absent those two, "scalable" silently eats "sovereign," and the consent-prediction model
becomes a capture surface that sits **outside** the `ConstitutionalLayer` altitude analysis entirely —
capture the disposition's training signal and you capture representation without touching a vote or a
token. That surface is named here and is **not yet in the threat model**; closing it is wisdom-cluster
v2. The faith the operator named — that the system is robust enough to tend the garden without a guardian
— is well-placed *only* once these two prerequisites are built; until then, the faith is doing load the
architecture has not yet earned, and the spec says so rather than pretending otherwise.
