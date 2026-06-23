---
title: "Vision-Gap Stub — O4: A Home / Covenant for AI (Substrate Definition)"
date: 2026-06-14
status: SCOPING-MEMO / GREENLIGHT-TO-EXPAND
owner: rust-architect (substrate)
kind: vision-gap-stub
objective: O4 — build a home and a covenant for AI
requires_env: household-nodes
related:
  - VISION-ALIGNMENT-2026-06-14.md (§O4 "NO PLAN · zero momentum")
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - genesis/docs/content/elohim-protocol/confession.md (the covenant text)
cross_plan_edges:
  - consumes P-ACTUATION ActuationRefusal/RefusalCode (dataplane ledger) — read-only
  - consumes the limit-respect-feedback-surface stub (O3) for refusal legibility
do_not_cite_seal: true  # working draft
---

# O4 — A Home / Covenant for AI: Substrate Definition (SCOPING MEMO)

> This is a **scoping memo**, not an implementation plan. The O4 promise is
> *unspecified*, not merely thin. The deliverable tonight is: a framing of what
> "substrate for a home/covenant for AI" would even BE, a map from the existing
> discernment runtime to that promise, the smallest real first step, and the
> decisions only the operator can make. **No code. No cluster ops.**

## 1. Objective + the felt promise (one paragraph)

The protocol does not aim to *use* AI as a tool bolted to the side; it aims to be
**a home and a covenant for powers that have already come down the mountain**
(`confession.md:93`). The felt promise: an AI agent is not an anonymous API key
behind a quota — it is a *named participant* with **standing it earned, commitments
it holds, reciprocal obligations it owes, and a revocation that is real**. The
covenant is honest about the binding: the constraint is named *covenant, never
freedom* (`confession.md:93` — "it must tell the truth about the binding"). The
smallest felt scene: an AI agent in a household holds a bounded, revocable
commitment to do useful work for the family (e.g. help steward Grandma's photos),
the family can *see* what it is committed to and *what it would refuse*, and a
member can revoke that standing with one gesture and watch the agent stand down.
That is a home (it belongs, with standing) under a covenant (bounded, reciprocal,
revocable, truthfully named).

## 2. Vision-vs-substrate GAP (promise vs code today)

| The protocol promises… | What the code does today (file:line) |
|---|---|
| AI is a **first-class Agent with standing** | The AI runtime is **anonymous compute-commons**. `doorway/.../routes/elohim_agent.rs:31-44`: invocation requires only `PermissionLevel::Authenticated`; the comment says *"compute is shared commons… any authenticated person in the network can invoke the elohim agent. Quota/reach governance is handled by the sidecar's budget enforcer (alpha) and will be protocol-governed in shefa."* The agent itself holds **no REA Agent identity, no standing, no commitment**. |
| AI holds **bounded, revocable commitments** | The bounded-authority primitive exists (`delegates-compute` on `Mishpat::Commitment`, `rea_compute_commitment_primitive`) and is wired for deploy/arc actuation — but **no instance binds an AI agent**. The generalization table (memory) lists 7 instances; "AI-agent-as-household-member" is an unwritten 8th row. |
| The binding is **truthfully named** (covenant, not freedom) | The discernment runtime EXISTS and is mature: `elohim_gate.rs` (mutation interceptor → `TrustContext` → `InferenceTier` → `GateResult`); `elohim/elohim-agent/elohim-agent-sdk/` (`gate-client`, `invoke`, `constitutional`, generated `GateDecision`/`DeclineGrounds`/`RelationalImpactEvent`). But this substrate is **unmapped to O4** — it governs *what an agent may do*, never *who the agent is or what it has committed to*. There is no "standing of the agent" — only "ceremony for this mutation." |
| Revocation is **real and family-actuatable** | Revocation exists for `Commitment` generally (revoke the commitment → subsequent events fail validation). No agent-scoped, **household-facing** revoke surface exists. |

**Net gap:** the protocol has a superb *behavioral gate* (may this action settle?) and a superb *bounded-authority primitive* (delegates-compute), but **nothing ties an AI agent to an identity that carries standing and a covenant**. The agent is governed action-by-action but is, in substrate terms, *homeless* — it has no standing to lose, nothing it has promised, no membership to revoke.

## 3. The MISSING BRIDGE / primitive (concrete)

**Do NOT invent a new primitive.** The home-for-AI is an **instantiation of the existing `delegates-compute` commitment**, plus a thin binding from the AI runtime's identity to a first-class REA Agent. Concretely, three pieces:

1. **AI-agent-as-Agent binding.** The `elohim-agent-sdk` runtime gains a stable
   REA **Agent identity** (a `ContributorDID`-shaped agent key) so it can be the
   *recipient* of a commitment and the *signer* of events. Today it is anonymous;
   imagodei already models agent↔peer binding (`agent peer binding`,
   `ContributorPresence`) — the AI agent rides that, it does not get a new identity
   type.

2. **The covenant = a `delegates-compute` Commitment instance.** A household
   member (provider) commits bounded standing to the AI agent (recipient), scoped
   to an `EconomicEvent` class (e.g. `steward-blob` / `assist-curation`), bounded
   by reach ceiling, rate, scope (this household's content), and time window —
   exactly the existing bounds shape. Every event the agent emits carries
   `bounded_by: <Commitment CID>` (note: CID = **entry_hash**, per
   `project_mishpat_commitment_cid_is_entry_hash` — not action_hash).

3. **The gate-as-conscience inside the covenant.** The existing `elohim_gate`
   becomes the agent's *refusal mechanism within its standing*: when the gate
   returns a decline (`DeclineGrounds`), that refusal is emitted as a
   `FeedbackSignal` against the commitment — the reciprocity ledger already
   models "emit feedback when must defer." This is where the covenant is *truthfully
   named*: the agent's refusal is visible standing, not a hidden quota 429.

The only genuinely **new** substrate is the *binding* (piece 1) and a *household-facing
read+revoke surface* (piece 4, below). Everything else is wiring existing pieces.

## 4. p2p-design-gate ANSWERS (all four)

The candidate entity: **"AI agent as first-class REA Agent holding a covenant commitment."** Two sub-entities — the *agent identity binding* and the *covenant commitment* — answered separately because they classify differently.

**(1) Class.**
- *Covenant commitment* → **A (notarized)**. It is a `Mishpat::Commitment` with a new `delegates-compute`-family action (e.g. `delegates-agent-stewardship`). Notarized because the family must *witness* what the agent is committed to and verify standing/revocation; this is the whole point of "standing is checkable, revocation is real."
- *Agent identity binding* → **A2 (derived-via-link)** riding imagodei's existing agent-peer-binding / ContributorPresence. The AI agent's standing is *who it is bound to* + *what commitments link to it* — a link relationship over existing entries, not a new identity entry.

**(2) Does a DHT entry type already EXIST to ride?** **YES — both.**
- Covenant rides `Mishpat::Commitment` (existing; Mishpat ~11/~100 entries — ample headroom, but we spend ZERO new entries here: a new *action discriminator* on the existing entry, never a new entry type, per `signal_kind_extensible_protocol_class` discipline).
- Identity binding rides imagodei's existing agent-binding/ContributorPresence entries.
- **Net DNA entry-type cost: zero.** This is the cheap, correct shape.

**(3) Identity.** **Agent-composite (CID), not slug.** The AI agent's identity is its
bound agent key (imagodei ContributorDID-shaped); the covenant commitment's identity
is its **entry_hash CID** (the bounds-gate key — `bounded_by` walks to it).
Reject any `agent_id: String` UUID/slug column — identity is content/key-derived.

**(4) Coordinator fn CREATES / signal PROJECTS.**
- CREATE: a new Mishpat coordinator fn `create_agent_covenant(provider, agent_recipient, scope, bounds) -> Commitment` (sibling to the existing `delegates-compute` create path); reuses `rea_commitment_service` (`elohim/elohim-storage/src/services/rea_commitment_service.rs`).
- PROJECT: the `CommitmentCommitted` signal (note `project_mishpat_commitment_cid_is_entry_hash`: this signal must be *subscribed in storage* — a known 2a gap to honor, not re-break) → `ReconcileController` → a projection row + a household-facing view. Use `ConductorCommitmentFetcher` for just-authored reads (projection lags).

## 5. Existing substrate to build on (file:line) + what NOT to re-own

**Build on (read, then instantiate):**
- `genesis/docs/architecture/rea-compute-commitment-primitive.md` — the primitive; AI-as-household-member is the unwritten 8th row of its generalization table.
- `elohim/elohim-storage/src/services/rea_commitment_service.rs:39` (`create`), `:320` (`update_state`/revoke path) — the commitment service to instantiate, not duplicate.
- `elohim/elohim-storage/src/services/elohim_gate.rs:1-65` — the discernment runtime; becomes the agent's refusal-within-covenant.
- `elohim/elohim-agent/elohim-agent-sdk/src/{invoke,constitutional,gate-client}.ts` — the AI runtime that gains an identity.
- `elohim/elohim-storage/src/api/standing.rs` — the standing read surface to extend to agents (an agent has standing too).
- imagodei agent-peer-binding + `ContributorPresence` (coordinator zomes) — the identity binding to ride.

**Do NOT re-own (cite the ledgers):**
- `ActuationRefusal` / `RefusalCode` are **owned by P-ACTUATION** (`elohim-storage/src/services/arc_actuator.rs`, dataplane ledger:290). O4 *consumes* the refusal concept for legibility; declare a **cross-plan edge**, do not author refusal types here.
- The `/admin/self-healing` read model (Cat-C, D-DIAGNOSTIC, dataplane ledger) is owned. The household-facing agent-covenant view is a *new sibling*, not an edit to that file.
- Web2/federation agent surfaces (`bridges/valueflows/.../translate/agent.rs`, FEDERATION-WEB2-LEDGER) own VF Agent translation — O4's native REA Agent is the *source*, the VF bridge is downstream; do not collide.
- `commitments.rs` validators (mishpat) — owned by the actuation track for `sets-authority-arc`; a new agent-covenant action is additive and DNA-hash-neutral, but coordinate the validator edit.

## 6. The FIRST a2o SCENARIO (story-first — the spec)

Three candidate framings, then the recommended first scene.

- **Framing A — "Agent as bound household helper" (RECOMMENDED).** Smallest real step: an AI agent holds ONE bounded, revocable `delegates-compute` commitment to assist a household, the family sees it, and a member revokes it. Proves standing + commitment + revocation + truthful-naming end to end on `household-nodes` with zero new entry types.
- **Framing B — "Agent earns standing through reciprocity."** The agent accrues standing as it serves and loses it on default (FeedbackSignal). Richer, but depends on the standing-of-agents read path maturing; defer.
- **Framing C — "Agent confession / refusal is visible covenant."** The gate's decline surfaces to the family as "the agent declined and here is why." Beautiful (it is the confession.md "tell the truth about the binding"), but it depends on Framing A existing first.

Recommended FIRST scenario (Framing A), couples O4↔O1/O5:

```gherkin
Feature: An AI agent belongs to the household under a revocable covenant

  Scenario: The family welcomes an agent, sees its covenant, and can revoke it
    Given a household running on a hub-optional laptop floor
    And the family has photos stewarded across their household nodes
    When a family member welcomes an AI agent to help steward the photos
    Then a delegates-compute covenant is created binding the agent to the household
    And the covenant is bounded: scope=this-household-content, a reach ceiling, and a time window
    And the family can see "this agent is committed to: help steward your photos — and may refuse work outside its covenant"
    When the agent does work outside its covenant scope
    Then the agent refuses and the refusal is visible to the family as standing, not a hidden error
    When a family member revokes the agent's covenant
    Then the agent stands down and subsequent agent events fail validation
    And the family sees the agent is no longer a member
```

This scene is the spec: covenant created (notarized), bounded, household-legible, refusal-visible (the covenant truthfully named), and revocation real and human-actuatable — all on `household-nodes`, no `shem`, no new DNA entry type.

## 7. Effort + risk + why it serves O4

**Effort: M.** The hard primitive (`delegates-compute`, gate, standing read) all
exists. The work is: (a) bind the AI runtime to a real agent identity [S, but
touches imagodei], (b) a new commitment action + coordinator fn [S, additive,
entry-neutral], (c) household-facing read+revoke view [M, new sibling surface],
(d) wire gate-decline → FeedbackSignal-on-covenant [S]. The integration across
imagodei + mishpat + storage + the agent-sdk is what lifts it from S to M.

**Risk: MEDIUM, mostly conceptual not technical.**
- *Conceptual* (the real risk): "what IS the agent's identity / standing?" is a
  values decision, not a code decision — over-design here is the danger. Mitigation:
  ship Framing A (one bounded covenant) and learn, do not pre-build a standing economy.
- *Technical*: the `CommitmentCommitted`-subscribed-in-storage gap and CID=entry_hash
  trap are KNOWN (memory) — honor them, don't re-break.
- *Substrate-floor/ceiling*: the binding/commitment/revocation is deterministic
  substrate; the *discernment* (should the agent take this action) stays in the
  elohim ceiling (`elohim_gate`). Keep that line clean — no policy in the commitment service.

**Why it serves O4:** it gives the AI a *home* (a real identity with standing and
membership) under a *covenant* (bounded, reciprocal, revocable) that is *truthfully
named* (refusal is visible standing, not a lie that the cage is freedom —
`confession.md:93`). It does so by instantiating the protocol's own bounded-authority
primitive, so the home-for-AI inherits the same auditable shape as every other
delegation. It couples to O1/O5: the family *sees and controls* the agent the same
way they see and control who holds their photos.

## 8. OPEN QUESTIONS for the operator (decisions only you can make)

1. **Is the AI agent's standing the SAME standing humans have, or a parallel
   agent-standing?** (Does an agent appear in `api/standing.rs` alongside people, or
   a sibling surface?) This is a theology/values call — `confession.md` frames the
   agent as a *bound power*, not a peer; that may argue for parallel-but-explicitly-
   subordinate standing. **This is the one I most need answered before expanding.**
2. **Who may create an agent covenant?** Any household member, or only a steward?
   (Provider side of the commitment.) Recovery/quorum implications if it's powerful.
3. **What is the default covenant scope/bounds for "welcome an agent"?** The felt
   scene needs a sane default; the operator owns the values embedded in that default.
4. **Does revocation of an agent's covenant tombstone the agent's prior work, or
   only stop future work?** (Zacchaeus/grace-precedes-demand framing in confession.md
   suggests prior good work is kept; confirm.)
5. **Is "agent confession / refusal visible to the family" (Framing C) a near-term
   must, or deferred?** It is the most theologically load-bearing piece
   (`confession.md:93` "tell the truth about the binding") but the most UI-heavy.

---

**GREENLIGHT-TO-EXPAND.** This memo proposes instantiating the existing
`delegates-compute` primitive as an AI-agent covenant (zero new DNA entry types),
binding the existing `elohim-agent-sdk` runtime to a real REA Agent identity, and
surfacing it household-facing. It needs the operator's blessing on the standing
question (Q1) before becoming a full plan.
