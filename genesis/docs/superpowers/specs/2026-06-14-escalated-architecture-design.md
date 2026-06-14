---
title: "THE ESCALATED ARCHITECTURE — One System the Vision Requires"
id: escalated-architecture-design
date: 2026-06-14
status: design (operator-blessed 2026-06-14)
author: rust-architect (truth layer)
weaves:
  - VISION-DESIGN-arc-2026-06-14.md
  - VISION-DESIGN-two-quilt-storage-2026-06-14.md
  - VISION-DESIGN-coherence-2026-06-14.md
  - VISION-DESIGN-availability-2026-06-14.md
  - VISION-DESIGN-care-minting-2026-06-14.md
  - VISION-DESIGN-limit-governance-2026-06-14.md
  - VISION-DESIGN-data-agency-2026-06-14.md
  - VISION-DESIGN-ai-covenant-2026-06-14.md
  - VISION-DESIGN-felt-spine-2026-06-14.md
north_star: "A p2p dataplane that provides a polished end-user-experience, on a quilt-tier's
  replicated dataplane, backed by those mutual compute agreements, where collectives continue to
  serve the humans that use it, and maintains that high-integrity of the Holochain DHT... but still
  have the 'hubs' — households to factories — that scale the sensemaking needed (across the fractal
  stewards, and distributed valueflows), through governance contracts that set policies, enforce
  decisions, and build a donut-like commons... Coupled story+value+governance, so the system can
  stay in stasis when actuating a capture-resistant state against the real world, its externalities,
  and its messiness."
---

# THE ESCALATED ARCHITECTURE

> Nine design passes asked nine separate questions. They returned **one answer in nine voices.**
> Every pass, reading real source, found the same shape: the limit was never physics, it was a
> *layering artifact* — a policy clamp sitting one layer above a substrate that already speaks the
> full language. And every pass, asked for its deepest escalation, arrived at the **same primitive**:
> a bounded, witnessed, revocable, refuse-and-elevate **Commitment** whose coverage is governed by a
> `∪ = full` invariant. Arc is that primitive pointed at keyspace. Custody is it pointed at bytes.
> Care is it pointed at people. The covenant is it pointed at the AI. The self-limit is it pointed at
> the self. The head is it pointed at served truth. This document is the proof that those are not
> nine decisions — they are **one machine, governed once, instantiated everywhere.**

---

## PART 1 — THE ONE SYSTEM

### The spine: two quilts, one commitment, one felt seam

The vision asks for "a quilt-tier's replicated dataplane" that "maintains that high-integrity of the
Holochain DHT." Read literally against the substrate, that is a contradiction — a DHT bloated with
corpus bytes is a DHT only datacenters can anchor (capture-prone), and a laptop forced to hold a
whole-corpus authority arc OOM-dies (the james treadmill, `project_per_node_memory_is_conductor_authority_arc`).
The contradiction resolves at one move, which **every pass independently converged on**: there are
**two quilts, not one.**

- **The trust-plane (the high-integrity DHT).** Small, validated entries: identity, notarization,
  commitments, observations, care-events, and the *pointers* to bytes (`Content.blob_cid` already
  exists, `content_store_integrity/src/lib.rs:521`). Lean enough that a chromebook holds a *real arc*
  of it. This is the plane "people build trust on the values negotiated through it."
- **The byte-plane (the replicated dataplane quilt).** The heavy corpus — photos, blobs, video,
  large bodies — RS(4,7) erasure-coded (`sharding.rs:97-99`, any 4 of 7 reconstruct), CID-addressed,
  custody-tracked. RAM-independent of the DHT arc. This is "a quilt-tier's replicated dataplane."

The two-quilt split is **~80% already in the substrate** (two-quilt pass §2): `Content.blob_cid`,
`ShardManifest`, `ShardLocation` entry types all exist; the RS math is done and tested; `reconcile/custody.rs`
already reconciles custody-as-REA. What is missing is *one policy in three gaps* — ingest plane-routing,
a byte-plane coverage-invariant gate, and custody-as-coverage-commitment feeding the economy.

**The decisive consequence (the arc pass and two-quilt pass agree, sharply): moving bytes off the
DHT makes a lean-trust-plane `{0,1}` arc sufficient on a laptop.** The two-quilt split is the
*replacement* for the kitsune2 fractional-arc fork — not its complement. We do not first need
fractional arc; we need the corpus to stop being DHT entries. This reconciles the only tension
between the passes: the arc pass proposed a custom kitsune2 sharding module (R3); the two-quilt pass
proposed rejecting that fork. **Both are right, sequenced:** ship the two-quilt split first (it may
make the fork unnecessary), and hold the kitsune2 module as a gated, evidence-triggered fork only if
a lean trust-plane *still* can't fit a laptop after the corpus is re-homed.

### The one commitment, six faces

Every pass's deepest escalation is the *same REA primitive* — `Mishpat::Commitment` with a new
**action discriminator** (never a new entry type), bounded, witnessed on the high-integrity DHT,
revocable, and governed by a coverage invariant. This is the `project_rea_compute_commitment_primitive`
generalization table, and the nine passes write **six new rows** into it:

| Face | Action | Coverage invariant | What it governs | Pass |
|---|---|---|---|---|
| **arc-as-coverage** | `commits-arc-coverage` | `∪ arcs ⊇ FULL` | who holds which keyspace range | arc |
| **custody-as-coverage** | `custody-blob` (governed) | `∪ custody ⊇ corpus, ≥ r_floor/shard` | who holds which bytes | two-quilt |
| **head-coverage** | `covers-head` / `holds-head-coverage` | `∪ coverage ⊇ collective, quorum Q, freshness ≤T` | who serves the convergent truth | coherence + availability |
| **care-as-commitment** | `provide-care` | floor (dignity) / ceiling (anti-monopoly) | witnessed care → minted recognition | care-minting |
| **self-limit** | `respects-self-limit` | `∪ self-limits + C_target` covers commons (the donut outer ring) | the line a person draws on themselves | limit-governance |
| **capability-as-commitment** | `revokes-capability` / `rotates-wrap` | re-coverage of new ciphertext | a person's revocable grip on their own data | data-agency |
| **covenant-as-commitment** | `delegates-agent-stewardship` | the AI's blast radius = its granted scope | the bounded home for AI | ai-covenant |

Seven rows (six new plus the existing `delegates-compute`). **One substrate. One entry type spend:
zero.** This is the architecture's central thesis: *coverage is care, care is coverage, and both are
the same governed, revocable promise.* When a steward commits to hold arc-range X, hold shard-set Y,
serve head Z, give care to Margaret, bind themselves to a self-limit, or steward grandma's photos —
they are performing **one kind of act**, accounted on **one ledger**, governed by **one invariant
shape**, enforced by **one control-plane spine.**

### The one control-plane spine

The limit-governance pass made the deepest structural finding: the cybernetic detect→refuse→elevate
spine — `arc_actuator.rs` (`authorize:110`, `coverage_admits:152`, `ActuationRefusal{code, elevate}:77`)
— is **already built and running in production**, built for arc, never recognized as the general
engine. The escalation is to **lift it once into a `trait Governor`** (limit-governance R3), so
`ArcGovernor` is the first impl and every other face is a second impl, never a clone. The ai-covenant
pass confirms it from the other side: `arc_actuator::authorize` **is** the covenant enforcer; the AI
agent is just its eighth instance. P-ACTUATION is already generalizing it into `elohim-compute::actuation`.

So the control plane is: **one `trait Governor` over `(setpoint, sensor, actuator, owner)`**, where
`owner ∈ {operator, commitment, self}` is a **substrate invariant** (limit-governance R0) — a refusal
*always names whose line it hit*, so the system actuating on a person's behalf can never be mistaken
for an operator overriding them. This is the single most important capture-resistance property in the
whole design.

### How the other passes compose onto the spine

- **Coherence** routes the served EPR head through the existing Automerge CRDT plane (`src/sync/`,
  already converging every 60s) so divergence becomes *structurally impossible to serve*, then governs
  that convergence as the `covers-head` coverage commitment. The doorway then only *observes* a truth
  it cannot author — exactly where the edge belongs.
- **Availability** makes the head a first-class quilt object (signed, seq-ordered, content-addressed)
  that heals from coverage-committed siblings (`get_head_or_heal`, composing existing `peer_selection`
  + `race_fetch`) and **fails closed to `503 catching-up`** when a partitioned peer can't prove
  freshness. The same `holds-head-coverage` commitment is the candidate filter. Availability and
  coherence are two faces of one thing: *the head is a governed, convergent, quilt-replicated value
  that no single edge can be.*
- **Care-minting** wires the observe→mint seam (`Observation` log → `RecognitionTrigger` →
  `recognition_pipeline_service`'s donut Limit stage, all live) so witnessed care becomes minted
  recognition — and instantiates `provide-care` as the commitment that the witnessed care *fulfills*.
- **Limit-governance** realizes the donut's *outer ceiling* as a coverage invariant over self-limits
  — the exact dual of how `arc_actuator` enforces `∪arcs ≥ r_floor` for keyspace. The donut's inner
  ring (dignity floor, `token_decay_service.rs:164`) already exists; the outer ring is the new
  coverage relation.
- **Data-agency** stores revocable content **encrypted-at-rest under a person-held wrap capability**
  (`crypto_box_seal` already in-tree, `sealed_against_self.rs:32`); "give my data back" becomes
  revoke + rotate (a notarized act riding the same Commitment), making the replicas *inert ciphertext*.
- **AI-covenant** names the AI agent as an REA Agent under a bounded `delegates-agent-stewardship`
  commitment, enforced by the same `Actuation::authorize` spine, its refusals already DHT-notarizable
  (`GateDecisionAttestation`).
- **Felt-spine** is the connective tissue: it **inverts the dependency arrow** so grandma's felt
  moment is the *acceptance test* that pulls all of the above into existence. The `<elohim-memory-safety>`
  Family Vault component is the **single human-addressed seam between the two quilts** — B1 reads the
  byte-quilt, B2/B3 read the trust-plane, and the surface fuses them into one felt safety.

**The one system, in one breath:** Two quilts (lean trust-plane DHT + heavy RS(4,7) byte-plane),
bridged by content HEADs, where *everything a steward holds* — keyspace, bytes, served truth, care,
self-limits, capabilities, an AI's scope — is the **same governed, witnessed, revocable Commitment**
under a `∪ = full` coverage invariant, enforced by **one `trait Governor`** that refuses-and-elevates
and always names whose line it honored, all **felt by a grandmother** through one surface that makes
the technical (bytes routed + held), the economic (care minted), and the governance (coverage enforced,
capture resisted) into one act she can feel is safe.

---

## PART 2 — THE SUBSTRATE FORKS / CUSTOM MODULES / NEW PRIMITIVES WE COMMIT TO

Consolidated across all nine passes. Each marked with **why the vision demands it** and **cost / blast
radius / reversibility.**

### A. New primitive instances (NO new DNA entry type, additive action discriminators) — the cheap, load-bearing core

These are the **six new rows** in the REA compute-commitment generalization table. Each is a
`signal_kind`/action extension on the existing `Mishpat::Commitment` (Mishpat ~11/~100 entry budget
untouched; CID = entry_hash per `project_mishpat_commitment_cid_is_entry_hash`). DNA-hash-neutral
(coordinator + action discriminator only, hot-swappable via `update_coordinators`).

1. **`commits-arc-coverage`** — arc as governed keyspace coverage. *Why:* fractal stewards need
   negotiated/audited/revocable coverage, not a silent config default. *Cost:* one array entry +
   projection. *Reversible.*
2. **`custody-blob` (governed)** — bytes as governed coverage commitment with a coverage-invariant
   gate. *Why:* "collectives serve humans" = ∪ custody covers the corpus their humans need. *Cost:* M
   (the gate + bounds extension). *Reversible.*
3. **`covers-head` / `holds-head-coverage`** — served truth as governed coverage (quorum + freshness +
   coverage). *Why:* no single edge/steward can BE the head; failover redeems a named commitment.
   *Cost:* M (bounds-validator row + projector). *Roadmap.*
4. **`provide-care`** — witnessed care as a fulfillable commitment. *Why:* "care-based economy where
   value is minted." *Cost:* S (the adapter); the pipeline + donut Limit stage are live. *Buildable now.*
5. **`respects-self-limit`** — the self-reflexive member (subject == author). *Why:* "governance
   contracts that set policies" includes the line a person draws on themselves. *Cost:* S (mirror
   `sets-authority-arc`). *Buildable now.*
6. **`revokes-capability` / `rotates-wrap`** — person-held revocable data capability. *Why:* "agency
   back to one's data, capture-resistant against the messy real world." *Cost:* M (key rotation as a
   notarized act). *Buildable now.*
7. **`delegates-agent-stewardship`** — the bounded home for AI. *Why:* "a home and a covenant for
   powers that have already come down the mountain" (`confession.md:101`). *Cost:* S. *Buildable now.*

### B. Forks of our own architecture (cheap, reversible — refactors, not new code volume)

8. **`trait Governor`** — lift the `arc_actuator` spine once; `ArcGovernor` becomes the first impl,
   every other governor a second impl, never a clone. *Why:* "one substrate, many instantiations" made
   literal in the control plane. *Cost:* M (refactor, callers unchanged). *Reversible.*
9. **`limit_owner: {self | commitment | operator}` as a substrate invariant** — peer to the
   care-class/compute-class isolation rule. *Why:* an operator-veto smell must never leak into a
   person's lever — the core capture-resistance guarantee. *Cost:* XS (one enum + discriminant).
10. **Shared `elohim-compute` crate** — `ActuationRefusal`/`RefusalCode`/coverage-gate as ONE
    definition consumed by arc-coverage, quilt-coverage, head-coverage, self-limit. *Why:* one
    refusal-and-elevate definition, not four drifting clones. *Cost:* S (extraction).

### C. New cross-cutting substrate capabilities (additive, buildable now, no DNA spend)

11. **Two-Quilt Reconciliation Policy** — ingest plane-routing (`content_plane_router`) + byte-plane
    coverage-invariant gate (`quilt_coverage`, copy of `coverage_admits`) + custody-as-coverage feeding
    shefa. *Why:* the storage backbone the vision requires. *Cost:* M×3. *Buildable now.*
12. **EPR head as an Automerge CRDT doc** over the existing sync plane. *Why:* convergence as a
    substrate property, capture-resistant by construction. *Cost:* ~2 weeks, both-stack parity,
    flag-staged. *Build commitment.*
13. **`EprHeadProof` + `predecessor`/`seq` on `EprHead`** (additive wire fields) + **head heal-on-read**
    (`get_head_or_heal`) + **the fail-closed `503 catching-up` invariant.** *Why:* availability that
    never trades integrity. *Cost:* composition of shipped primitives. *Buildable now.*
14. **Content-wrap capability** — `WrapRef` envelope field + `wrap-keys` store + `BlobStore::store_wrapped`
    (wiring the in-tree `crypto_box_seal` to the content plane). *Why:* withdrawal that is real against
    a hostile holder. *Cost:* M, opt-in per content. *Buildable now.*
15. **AI-runtime ↔ REA-Agent identity binding** (Rung 0 of ai-covenant, riding `AgentPeerBinding`).
    *Why:* the agent must be *named* before it can be bound. *Cost:* S. *The one genuinely new piece.*
16. **The `feltStatus` Cat-C projection + `<elohim-memory-safety>` Family Vault component** + the
    `kind`/`label` collective join + the `compute` honesty fork (`distribution_state` so "not-yet-seen"
    never renders as "at-risk"). *Why:* the human-addressed seam between the two quilts; the acceptance
    test that pulls everything. *Cost:* S–M. *Buildable now.*

### D. The genuine roadmap forks (real substrate investment, operator-blessed, sequenced)

17. **Custom kitsune2 sharding/gossip module (`kitsune2_elohim_gossip`)** — a `DynGossipFactory`
    driving `set_tgt_storage_arc_hint` from a resource-aware policy. *Why:* IF the two-quilt split
    doesn't make a `{0,1}` laptop arc sufficient, fractional arc is the fallback. **This is "write our
    policy into a first-class factory slot," NOT "fork Holochain"** (the slot is public; `core_gossip`
    is an explicit stub). *Cost:* fork-class (version-tracked against kitsune2 API). **GATED** on a
    cheap probe (arc R2) AND on the two-quilt split proving insufficient. *This is the only fork we may
    not need.*
18. **Typed care-class / compute-class partition** (care-minting Rung 2) — a DNA-hash-changing
    validator that makes the resilience README:468 isolation *structural* rather than disciplinary.
    *Why:* "a hardware failure cannot silently re-rank a contributor's standing" becomes structurally
    true. *Cost:* DNA-hash change → coordinated reinstall → network event. **Near-irreversible on a
    deployed DHT; sequence with a planned reinstall.** *The genuine fork that requires operator blessing.*
19. **The donut-ceiling coverage relation** (limit-governance Rung 4) — the regenerative outer ring as
    a Category-C coverage invariant over self-limits + `C_target`. *Why:* the donut as a governance
    contract, not a metaphor. *Cost:* L. Category-C (recomputed, no DHT spend), so reversible.
20. **Forward-secrecy / re-keying wrap scheme** (data-agency Rung 6) — narrows but cannot close the
    "decrypted-while-authorized" window. *Why:* the honest boundary of revocation. *Cost:* future crypto
    research. **Named, NOT taken — flagged as a roadmap fork, kept out of MVP, must be in the honesty copy.**
21. **Upstream contribution: fractional sharding to kitsune2** (arc R4) — retires fork #17. *Why:*
    advances the whole Holochain ecosystem; the "not yet allowed until sharding is implemented" log is
    the literal invitation. *Long-horizon, on-mission.*

**The honest count:** ~16 buildable-now items (A4–7, B, C), spending **zero DNA entry types**; **one
near-irreversible DNA-hash fork** (#18, operator-blessed, reinstall-sequenced); **one gated/maybe-unneeded
transport fork** (#17); two roadmap primitives (#19, #20); one long-horizon upstream (#21). **No fork
of Holochain core. No fork of libp2p. No fork of iroh.** The architecture lands almost entirely on the
substrate we already have, which is the strongest possible evidence the vision was *designed into* the
substrate, not bolted on.

---

## PART 3 — BUILDABLE-NOW vs ROADMAP-FORK

### Buildable now (serves the polished experience immediately, NO upstream dependency)

| # | Item | Pass | Cost |
|---|---|---|---|
| 0 | **Fix the signal-decode subscriber** (holo_hash byte-arrays dropped on rmp→Value path) — gates ALL human-facing projections | felt-spine R0 / MF14 | S — **bug, do first** |
| 1 | Two-Quilt ingest routing + byte-plane coverage gate + custody-as-coverage | two-quilt R1–3 | M×3 |
| 2 | `feltStatus` projection + Family Vault component + label join + `compute` honesty fork | felt-spine R1 | S–M |
| 3 | Care observe→mint adapter (Margaret visible) | care-minting R0 | S |
| 4 | `respects-self-limit` action + `SignalKind::Approach` + `limit_owner` invariant | limit-governance R0–2 | S |
| 5 | AI agent identity binding + `delegates-agent-stewardship` + wire through `Actuation::authorize` | ai-covenant R0–2 | S–M |
| 6 | Content-wrap capability + `revokes-capability` rotation + threshold custody | data-agency R1–3 | M |
| 7 | EPR head freshness witness + head heal-on-read + fail-closed 503 | availability R1–3 | M (composition) |
| 8 | Edge divergence detector (re-scoped to "alarm surface") | coherence R0 | S (planned) |
| 9 | Governed `MintPolicy` (mint rate/floor/ceiling leave Rust constants) | care-minting R1 | S |
| 10 | `trait Governor` refactor + shared `elohim-compute` crate | limit-governance R3 | M |

### Roadmap fork (genuine substrate investment, operator-blessed)

| # | Item | Trigger / sequencing | Operator call |
|---|---|---|---|
| R1 | **EPR head as CRDT doc** over the sync plane | after F-BOOTSTRAP (shared persistent bootstrap) closes | build commitment |
| R2 | **`covers-head` / `holds-head-coverage`** coverage commitments | sequence with arc-as-coverage (shared primitive) | roadmap |
| R3 | **Donut-ceiling coverage relation** | after care + self-limit land | roadmap, needs `C_target` operator value |
| R4 | **Typed care/compute partition** (DNA-hash fork) | sequence with next planned DNA reinstall | **operator blessing — near-irreversible** |
| R5 | **Custom kitsune2 sharding module** | ONLY if two-quilt split proves a `{0,1}` laptop arc insufficient (probe-gated) | operator blessing — fork-class carry |
| R6 | **Forward-secrecy wrap v2** | future crypto research | named, not taken; honesty copy required |
| R7 | **Upstream fractional sharding to kitsune2** | after R5 proves the shape in-tree | long-horizon |

**The hard prerequisites named honestly:** F-BOOTSTRAP (the in-memory islanding bootstrap,
`project_doorway_kitsune2_bootstrap_protocol`) must close before coherence R1 can converge anything.
The signal-decode bug (#0) must close before any human-facing projection is trustworthy. Neither is a
fork; both are gates.

---

## PART 4 — THE SEQUENCE (from here to there without losing stasis)

The felt-spine pass gave the sequencing *principle*: **invert the arrow — the felt scene is the
acceptance test for the substrate work it consumes.** Every swarm-side capability now owes a felt beat
it must cash out, or it is not done. The sequence below is ordered so that *each landing makes a
grandmother's afternoon more real*, and stasis is never lost (each step is additive, flag-staged, and
reversible).

**Wave 0 — Unblock truth (the precondition).** Fix the signal-decode subscriber (#0). Close F-BOOTSTRAP
to the extent coherence depends on it. Nothing human-facing is trustworthy until projections stop
silently dropping holo_hashes. *Felt beat unlocked: none yet — but every later beat depends on it.*

**Wave 1 — The two-quilt spine + the felt seam (THE FIRST MOVE).** Land the two-quilt ingest routing
and coverage gate (#1) — grandma's photo is routed to the byte-quilt, her HEAD stays on the lean
trust-plane. Simultaneously light the Family Vault component (#2) reading existing truth. *Felt beat:
**B1 (photos load instantly, narrated) + B2 (held by named households).*** This is also the highest-leverage
RAM fix and the strongest test of whether fork #17 (kitsune2) is needed at all.

**Wave 2 — The promise becomes governed.** Land `custody-blob` as a governed coverage commitment (#1
completion) and `commits-arc-coverage` (arc R5). *Felt beat: **B3 (that holding is a real, revocable,
named promise).*** The placement-gap event already fires; now it cites a commitment.

**Wave 3 — Care is minted from the felt action.** Wire the observe→mint adapter (#3) and governed
`MintPolicy` (#9). *Felt beat: **B4 (when a holder lapses, who-could-help + one tap that MINTS care,
never a red SLA).*** This is the first beat where the felt surface *creates value*.

**Wave 4 — Agency and the lever in grandma's hand.** Land content-wrap + revocation (#6). *Felt beat:
**B5 (she controls who holds her memories — withdrawal makes copies inert).*** Capture-resistance felt
as control.

**Wave 5 — The collective stays coherent and keeps serving.** Land the head freshness witness +
heal-on-read + fail-closed 503 (#7), and the edge detector (#8) re-scoped as its alarm. Then commit
the CRDT-head build (R1) and `covers-head` (R2). *Felt beat: grandma reading through any steward sees
the same truth; when matthew falls, adam serves the proven head or honestly says "catching up" — she
never notices the fall, and never sees a fork.*

**Wave 6 — The self-limit and the AI covenant.** Land `trait Governor` (#10), `respects-self-limit`
(#4), and the AI agent binding + covenant (#5). *Felt beat: the system notices Maria approaching her
own line and eases; a family welcomes an AI agent, sees its covenant, and revokes it with one gesture.*

**Wave 7 (roadmap) — The deep forks, sequenced to a planned reinstall.** The typed care/compute
partition (R4) lands with the next DNA-hash bump. The donut-ceiling coverage relation (R3) and the
kitsune2 module (R5, only if needed) follow. Forward-secrecy (R6) and upstream sharding (R7) are
long-horizon.

**Stasis is preserved at every wave** because: every landing is additive and flag-staged; the
two-quilt split is a *forward* policy (new content routed correctly) with the legacy re-homing as a
gated spike; the coverage invariants *refuse-and-elevate* before opening a gap (never silently
degrade); and the head plane *fails closed* (degraded-but-honest, never available-but-lying). The
system can never be made to trade integrity for availability, or to serve a fork, or to overwrite a
person's lever — because each of those is structurally forbidden by an invariant, not by discipline.

---

## PART 5 — THE OPERATOR CALLS THAT REMAIN

These are the genuinely irreducible decisions. The escalation surfaced them; it cannot make them. Each
is framed as the deep question it really is.

1. **The care-meaning call (care-minting Q + the typed partition fork #18/R4).**
   *The shallow question:* should witnessed care mint a token? *The deep question:* **Is care
   categorically different from compute — different enough to be enforced by a typed DNA validator that
   a hardware failure can never cross, even at the cost of a near-irreversible DNA-hash reinstall?** If
   yes, the resilience README:468 isolation becomes structural and a compute breach can never re-rank
   Margaret's standing. If you defer, it stays disciplinary (a reviewer must remember). This is the one
   fork that spends near-irreversible DNA-hash budget — bless it for the next reinstall, or hold it.

2. **The AI-covenant call (ai-covenant Q1, Q3 + the theology).**
   *The shallow question:* does the agent get the same standing surface as a human? (Technically: yes,
   shippable as a `subordinate: bool` field — it doesn't block.) *The deep question:* **What does it
   MEAN for a bound power to default?** A human who defaults accrues a FeedbackSignal; does
   `confession.md`'s grace-precedes-demand order (prior good work kept on revocation — Zacchaeus) apply
   to a machine? Is the agent a subordinate power under covenant, or a peer? This is theological, not
   technical, and `confession.md:105`'s "unbuilt place" says the architecture must structurally forbid
   the agent standing where worship is reserved. Only you can write that values content.

3. **The privacy-vs-accountability call (data-agency b/d/e + felt-spine Q4).**
   *The shallow question:* opt-in wrapping or wrap-everything? *The deep question:* **Where is the line
   between a person's right to make their data inert and the commons' right to a legible, accountable
   record?** Commons content stays plaintext (cacheable, global); private content is wrapped and
   revocable. And: does grandma's holder-label come from the collective's self-chosen name or a
   viewer-local alias (a holder may not want their household name shown to every viewer)? And: is
   withdrawal-of-standing ALWAYS sovereign (never operator-vetoable)? The recommendation is yes on all
   — the person's act is sovereign — but it is yours to affirm. And the honest boundary of Rung 6 must
   be in the UI copy: withdrawal makes future reads of bytes-at-rest impossible; it cannot un-read what
   an authorized peer already decrypted.

4. **The donut-geometry call (limit-governance Part 4 escalation flag).**
   *The shallow question:* what are the floor and ceiling ratios? *The deep question:* **How much
   committed coverage does the commons' regeneration REQUIRE** — the outer-ring `C_target`, the dual of
   the `dignity_floor`? This is a value-laden, DNA-wall-class number with "no value-neutral width"
   (limitarian spec Decision 2). The recommendation is to ship the *shape* (the coverage relation +
   refuse-and-elevate) with the target marked `TBD-operator`, set once by whoever writes core — exactly
   as the dignity floor shipped shape-first. But the number is yours.

5. **The kitsune2-fork call (arc R3/R5 + felt-spine Q2), conditional.**
   *The shallow question:* fork kitsune2 for fractional arc, yes/no? *The deep question:* **After the
   two-quilt split re-homes the corpus off the DHT, does a lean trust-plane STILL not fit a laptop?**
   If the split makes a `{0,1}` arc sufficient, the fork is unnecessary and should be retired. If a
   probe proves it insufficient, the custom sharding module (a first-class factory slot, not a core
   patch) becomes the vision-aligned path, with upstream contribution as its retirement. This is the
   one call the architecture deliberately *defers to evidence* — but the decision to gate it on the
   two-quilt result, rather than commit it now, is yours to bless.

6. **The manifesto-coupling call (felt-spine Q5).**
   "Family photos," "the donut," "stasis the grandmother can feel," and "a home and a covenant for AI"
   are now executable in the architecture but appear in no core vision doc. **Do you want a manifesto
   addendum so the named vision and the built spine cohere?** This is purely yours — the architecture
   doesn't need it, but the honesty matrix (Part IX) and the people who join the protocol might.

---

> **The closing claim.** The nine passes were not nine features to triage. They were one system asking
> to be recognized: *two quilts, one commitment with six new faces, one governing trait, one coverage
> invariant, one felt seam* — almost entirely buildable on the substrate we already have, spending zero
> DNA entry types for the core, requiring no fork of Holochain, and resolving the only inter-pass
> tension (arc-fork vs reject-arc-fork) by sequencing: the two-quilt split first, the kitsune2 module
> only if evidence demands it. What remains for you is not architecture. It is the six irreducible
> convictions — what care means, what an AI is owed, where privacy ends and accountability begins, how
> wide the donut, whether to fork, and whether to name the vision aloud — that even the deepest
> escalation cannot make on your behalf, because they are the values the whole substrate exists to
> serve.
