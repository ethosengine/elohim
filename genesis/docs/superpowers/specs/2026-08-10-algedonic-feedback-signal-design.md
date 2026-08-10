---
title: Algedonic Feedback Signal — pain as a first-class, floor-protected feedback type
id: algedonic-feedback-signal
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
graduation-trigger: slice-1 (delivery flow) landed AND slice-2 (zome+floor) ratified OR superseded
created: 2026-08-10
topic: [algedonic, feedback-signal, viability, counter-evidence, floor-protected, limit-governor, band-edge, concern-address, rea, witness]
cites:
  - sense-respond-governance-classifier | The Sense-and-Respond Governance Classifier | sha256:c716a519ee6cc953 | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md
  - vision-gap-limit-governor-stub | Vision-Gap STUB | sha256:14ea8f3e81cd87c8 | path: genesis/docs/superpowers/plans/2026-06-14-vision-gap-limit-governor-stub.md
  - dna-signal-as-epr-envelope | DnaSignal as EPR Envelope | sha256:507652ee91a75aa1 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-15-dna-signal-as-epr-envelope.md
  - trust-as-efficiency-signal | Trust is an Efficiency Signal | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md
  - evidence-ladder-push-left | Evidence Ladder + Push-Left Pressure | sha256:ac39aeb003dada60 | path: genesis/docs/superpowers/specs/2026-08-10-evidence-ladder-push-left-design.md
  - genesis/research/beer-designing-freedom-elohim-critique-2026-06-04.md
  - eprfs-witnessed-interaction-primitive | The eprfs Witnessed-Interaction Primitive | sha256:6a24773ffd7b83f4 | path: genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md
  - elohim/sdk/schemas/v1/p2p/feedback-signal.schema.json
---

# Algedonic Feedback Signal

**Operator directive (2026-08-10):** exceptions should be designed as algedonic
signals to REA, reporting TO the intention/promise nodes from any node in the
deployment — as an interface/contract every EPR must consider, even if only to
declare "this has no algedonic signal handler, and here is why." Preferably a
feedback TYPE (alongside judgment-valence feedback) with producer / stock /
limit / consumer requirements — never a parallel path ontology.

## 1. Position (MAP: D2 Evidence Primitives; edges into D1, D7)

This spec completes three already-canonized, unimplemented sketches through one
existing entry type — it invents no new path:

- The limit-governance vision names the gap verbatim: *"`SignalKind` has no
  algedonic / band-edge-approach variant"* and proposes `Approach` — "the
  signal before the line." The limit-governor stub's elevate step reads *"at
  `threshold_pct` raise an algedonic `FeedbackSignal`."*
- The sense-respond governance classifier carries §10.3 "Algedonic bypass"
  (threshold-triggered escalation), and the reach-ontology split declares the
  invariant: *"the algedonic signal floor bypasses every elohim."*
- The Beer/VSM critique records Gap 2: every pain path today is mediated; no
  un-mediated channel exists in source.

## 2. Ontology: a third feedback type (viability), orthogonal to valence

The protocol already types two feedbacks: **judgment valence**
(`GraduatedFeedback.position: i8` signed agree/disagree; `Vote{±1}` magnitude)
and **standing consequence** (`FeedbackSignal.standing_impact`, all-debit
today). The algedonic kind is a third type: a **viability signal** — pain
reporting that a promise is approaching or has crossed a bound — orthogonal to
judgment. A node in pain is not accusing anyone; it is reporting its own
sensed state against a declared limit.

## 3. The entity: new signal kinds on the EXISTING `FeedbackSignal`

Classification (p2p-design-gate output, reviewed 2026-08-10 in-session):

- **Category A (notarized) via the existing `FeedbackSignal` entry type**
  (elohim DNA, `content_store_integrity/src/feedback_signal.rs`). The
  `signal_kind` whitelist is string-extensible without moving the DNA hash.
  Two new kinds: **`algedonic-approach`** (band-edge; the `Approach` sketch)
  and **`algedonic-breach`** (bound crossed / self-heal mechanism exhausted).
- **Producer/stock/limit/consumer are schema requirements**, generalized from
  the existing `feedback-signals/rate-limit-exceeded.schema.json` precedent:
  `declarer` (producer — the self-reporting node's `agent_cid`),
  `evidence.stock` (the measured quantity), `evidence.limit` +
  `evidence.bound_ref` (the bound and the commitment/manifest CID that
  declared it), `target` (consumer — the CID of the promise threatened;
  commitment CID = entry_hash), `severity` ∈ info|warn|critical.
- **Identity**: content-derived CID (the `WitnessedInteraction`
  canonical-bytes pattern). Dedup keys remain bare-sha fingerprints — internal
  index keys, never addresses.
- **Emission bounds (head-plane honesty)**: emit on band-crossing with
  hysteresis; fingerprint-deduped; ONE open signal per (declarer, target,
  kind); retired on recovery (clean-streak rule, lifted from the dev-plane
  sentinels). Pain is a held state, not a stream.
- **Network stakes**: all four stages; delivery routes through
  **`FloorClass::CounterEvidence`** (trust pricer) — un-cheapenable FullChain
  verification at every stage including Simulacra, pinned by the existing
  full-product-space property test. This is Beer's un-mediated requirement
  implemented as a verification floor: pain cannot be filtered, aggregated
  away, or priced down by any elohim. Closes Gap 2.
- **Standing**: machine self-reports carry `standing_impact: advisory` — pain
  never debits standing by itself; consequences flow only through the
  witnessed governance path.
- **Rejected**: a new coupling leg on the EPR `Envelope` (coupling legs sit in
  `canonical_bytes()` — a required leg breaks every kind's signing contract
  for an additive per-type obligation); a new DHT entry type (headroom spent
  for nothing the whitelist can't do); an 8th `SubstrateSignal` member (moves
  the DNA hash).

## 4. The obligation: every EPR considers the algedonic path

Three existing attachment mechanisms, layered — "declare a handler, or explain
its absence" becomes a validated state, not silence:

1. **Per content-type**: an `algedonicHandler` field in the app-manifest
   schema — `{signalKinds, consumer}` OR `{none: "<reason>"}` — structurally
   identical to the live claims/observations ≥1-negative-polarity rule.
2. **Per code decision-point**: concern class **C15 algedonic-channel** in the
   concern canon, bound via `seam-registry.yaml` with the existing
   `answered | partial | unbound | n-a`+justification states. Recurrence
   admission evidence: the heal-leg silent `break` (2026-08-09), the
   gate-skip no-measure runs (#1337/#1338), adam's projection catch-up
   exhaustion (2026-07).
3. **At authoring time**: an `.epr-meta` `policy:` row (`ask`-class) on the
   surfaces where new `EprKind`s / content-types are born
   (`elohim/epr/src/kind.rs`, `elohim/sdk/domains/*/manifest/`).

## 5. Slices

- **Slice 1 — the development-plane projection (the delivery flow; plan:
  `2026-08-10-algedonic-slice1-delivery-flow-plan.md`).** The dev loop is the
  first deployment to wire, because it is the one hurting now: findings in
  the sentinel ledgers gain a `concern` address; a CI no-measure (gate-skip)
  becomes an addressed finding instead of silence; the pre-push gate makes
  measurement-by-deploy unwritable; the habits renderer joins live pain +
  last evidence per concern. Schema foothold: the two `feedback-signals/`
  instance schemas land (contract-first, no zome change).
- **Slice 2 — protocol wiring (separate plan, after slice-1 evidence).**
  Zome whitelist extension + kind-gates in `create_feedback_signal`
  (algedonic requires `evidence` + `bound_ref`); `CounterEvidence` floor
  routing + property-test extension; storage projection + emitters at the
  self-heal exhaustion sites; C15 minting; app-manifest `algedonicHandler`
  field; `.epr-meta` authoring policy.

## 6. Non-goals

- No parallel pain pipeline: Prometheus stays the local instrument deciding
  WHETHER to emit; the dev-tooling sentinels stay the dev-plane mirror; the
  DHT `FeedbackSignal` is the only cross-node addressed carrier.
- No valence overload: algedonic kinds never carry agree/disagree semantics.
- No unbounded emission: a node that cannot bound its own pain reporting is
  itself in breach of C6a bounded-work.
