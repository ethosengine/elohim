---
title: "The eprfs Witnessed-Interaction Primitive — Local Witness → Peer-Validated → REA-Aggregated on the Object CID"
id: eprfs-witnessed-interaction-primitive
tier: spec
status: Draft
created: 2026-07-15
maintainers: Matthew Dowell + Opus 4.8
class: process-meta
process_subdomain: governance-substrate
topic: [eprfs, witnessed-interaction, rea, valueflows, attention, energy, denomination, proof-of-attention, consumption, offline-first, hub-optional, reach-affirmation, propagation-velocity, anti-enclosure, un-enclosable, goodhart, recognition-discovery-firewall, dht-witness]
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
sovereignty-frame: descriptive  # cites values-forward's REJECTION of a self-sovereign apex (descriptive/adversary frame), never asserts it — suppresses sovereignty-guard-signal
refines:
  - genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md
cites:
  - sense-respond-governance-classifier | the sibling instance — proves this skeleton on the HARDEST substrate (contested meaning); §1.2 skeleton + instance table, §10.4 agent-governance-action as witnessed event, the honesty discipline this inherits | path: genesis/docs/superpowers/specs/2026-07-15-sense-respond-governance-classifier-design.md
  - elohim-seam-map-concern-routing | where the light-runtime routes — a Track-3 spoke + T1 notary, not a new seam; the participation-track vs seam distinction | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - substrate-trust-contract-runbook | verify-locally-then-serve; client clocks advisory; the network-assigned timestamp is the only trustworthy clock; fills-never-moves | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - values-forward | friction-gradient limitarianism (Stance II.4) — the reach/propagation-velocity model; the human floor sovereign, never a self-sovereign apex | path: genesis/docs/content/elohim-protocol/values-forward.md
  - stewardship-over-sovereignty | credit is stewardship reconciliation, not ownership/payout; the recovery ladder + bounded_by spine | path: genesis/docs/architecture/stewardship-over-sovereignty.md
---

# The eprfs Witnessed-Interaction Primitive

> **What this is.** The general primitive of which the sense-and-respond governance classifier
> (`sense-respond-governance-classifier-design`) is one instance. A **light-runtime witnesses interaction events on
> content-addressed objects; peer-sync turns local witness into a validated (notarized) witness; the events aggregate as
> REA records on the object's CID, denominated in the substrate the interaction actually spends.** The classifier proved
> the skeleton on the *hardest* substrate — contested meaning. This spec generalizes it to the *countable / costed*
> substrates — delivered bytes, metered joules, minutes listened, pages read — and to the *effect* substrate (a later
> citation, a passed assessment), where the witnessed quantity is a **count**, not a classification, and the adversarial
> surface is **consumption fraud + attention-capture**, not contested meaning.

## 1. Vision & thesis

### 1.1 The general primitive

Almost any file can be an object of witnessed interaction. An `.mp3` is listened to; an `.mkv` is watched; an `.epub` is
read; a hosted blob is *served*; a rule is *affirmed*. One generalizable shape sits underneath all of them — *local
witness → peer-validated → REA-aggregated on the object CID, substrate-denominated.* This is the anti-enclosure,
human-scale commons move: honest reconciliation of the value of attention and energy, aggregated on content **owned by no
one**, so creators and stewards can be fairly credited **without a platform capturing the ledger.**

### 1.2 The two commitments that make the accounting honest

Every prior-art system that attached payable or reach-granting value to a self-reportable consumption signal was farmed
the instant it was valuable — Brave/BAT bot farms, Steemit vote-selling, streaming farms, engagement-as-outrage. The
lesson (Goodhart's Law + Beer's POSIWID) is unambiguous: **paying for the attention proxy is the corrupting step, not
measuring it.** We build *toward* that boundary, not around it.

- **Commitment I — Witness, don't pay.** No witnessed event ever *automatically mints* value or reach. Aggregated
  witnessed spend feeds *recognition* through the existing ratified-and-contestable `StewardshipAllocation` machinery,
  which a human/collective can dispute and reverse. **There is no pool to drain because there is no pool.**
- **Commitment II — Denominate in the witnessable, never in the claimed** *(the load-bearing decision; §4.1, ratify D1)*.
  Credit is denominated in substrate a second, disinterested, or costed party can independently corroborate — **delivered
  bytes, metered joules, or an observable downstream effect.** Self-reported attention-minutes are recorded and reconciled
  *for the person*, but are **advisory by the structure of the world, permanently** — never a witnessed quantity carrying
  economic or reach weight. This is what lets us have honest creator-credit accounting *without* minting the gameable
  proof-of-attention bounty.

### 1.3 The governance classifier is one instance of this primitive

The sibling spec instantiated this exact skeleton on the hardest substrate. Its "interaction event" is "an edit touched an
apex-concept frame"; its witness is the guard + classifier; its REA signal is affirm/dismiss (a `FeedbackSignal`) on the
**rule CID**. §7 shows the two are not analogous but **isomorphic up to a choice of magnitude algebra** (which selects
the DHT carrier — `EconomicEvent` vs `FeedbackSignal`). And the
classifier's §10.4 already carries this primitive's deepest reflexive case: *an agent's governance action is itself a
witnessed-interaction event on the DHT* — its resolution notarized, its provenance audited via `agent_info()` (graduated;
v1 self-reported / advisory-forgeable per classifier §10.4D), its
compute source a pluggable toggle. The guardian is witnessed by the same ladder it applies to the bytes it guards.

### 1.4 Reach is re-affirmed at adoption; propagation velocity is the governed quantity

The upper witness ladder (§3) **is the reach-affirmation gradient.** Authoring is private; the reach-bearing **"post"
event is when content leaves the machine and is adopted by a peer**, and **reach is re-affirmed as it climbs**. Precisely:
reach-*affirmation* — a peer *staking standing* — begins at **CORROBORATED** (RECORDED is self-signed, no standing staked;
CONVERGED is *distribution*, a peer merely carrying the record at weight 0 — not yet weighted reach). The **speed** of
propagation across the peer graph is the governed quantity — a judgement call + a compute curve where **trust earns
efficiency**, driven by the object's **accumulated prior standing** (its *history*, not the current not-yet-corroborated
event, whose `witness_weight` is 0 until a disinterested party signs — §6.5): a high-standing object propagates fast and
cheap; low standing or an unresolved abstention costs propagation *speed*, never the right to author or record locally. This is friction-gradient limitarianism
(`values-forward` Stance II.4) applied to adoption velocity — the honest, non-blocking enforcement that works *before* the
silicon (peer-native inference) and sharpens into a real reach-ceiling exactly when peer-native inference arrives to sit
in the compute curve (the classifier's §10.4 graduation trigger). No one is silenced; reach is *earned*.

> **Reviewer orientation.** This spec resolves four contradictions that surfaced across its design corpus (the
> denomination joint, the witness's disinterestedness, `use` vs `consume`, whether a canonical aggregation exists) and
> folds in its critiques' smallest-fixes. Two are **preconditions, not options** — §4.4 (aggregate-without-a-subject) and
> §6.4 (recognition⊥discovery firewall) must land **before any code touches the wire**, or the primitive ships the exact
> surveillance / attention-capture it exists to refuse. They are marked **[PRECONDITION]**.

## 2. The skeleton and the instance table

### 2.1 The skeleton — four slots + one ladder

The primitive is the tuple **`(object_cid, substrate, action, magnitude)`** walked up a three-rung ladder. The four slots
are independently instantiable; the ladder is invariant across every instance.

| Slot | Definition |
|---|---|
| **object** | the content-addressed thing spent-upon — the REA *Resource*; the aggregation anchor is always its **CID**, never a path/name |
| **substrate** | which dimension is spent — a member of the DHT-notarized `SubstrateSignal` enum (`attention, compute, storage, bandwidth, energy, time, resource`). **CORRECTION (frame-witness-primitive-architecture §2.3):** governance is **NOT** an 8th substrate member — adding one *moves the DNA hash* (integrity-zome change). Governance rides as a **magnitude variant** (`Vote`/`Classification`) over an existing substrate, never a new resource dimension. |
| **action** | REA verb — `use` (non-depleting) \| `consume` (depleting) \| `produce` \| `cite` \| `affirm` \| `dismiss` |
| **magnitude** | algebra-tagged: `Count{value, unit}` (additive monoid, ℝ≥0) or `Vote{sign}` (ordinal tally, signed) |

The **ladder — `local witness → peer-validated → REA-aggregated on the object CID` — is a named state machine (§3)** that
every instance's witness component walks, independent of what it senses.

### 2.2 The auto-dedup theorem (inherited, load-bearing)

Because the aggregation anchor is the object's CID, aggregation is **auto-deduplicating**: two witnesses naming the same
`id@version` resolve to the same CID → the same aggregation node → measures accumulate on **one** object across every
device and directory. No path/name-keyed side-ledger can offer this. A re-encoded track with new bytes is *correctly* a
different object with its own profile. This dedups the **object**, not the **event** — event identity is a separate,
harder problem (§5.3, and an open seam — completeness critique a.5).

### 2.3 Instance table

| instance | interaction event | witness | substrate + magnitude | carrier | v1 credit posture |
|---|---|---|---|---|---|
| **governance** (sibling) | an edit touched an apex-concept frame | the guard + classifier hook | `governance` / `Vote{±1}` | `FeedbackSignal` | advisory-sensing only |
| **`.mp3` / `.mkv` / `.epub`** | minutes listened / watched, pages read | light-runtime meter | `attention` / `Count{min\|pages}` | — (advisory local tick) | **advisory-forever** (un-witnessable solo) |
| **delivery** | peer B served N bytes of CID X | serving peer's byte counter (costed, 2nd-party) | `bandwidth` / `Count{bytes}` | `EconomicEvent(use)` | witnessable → credited (behind §6 disinterest floor) |
| **energy** | N joules metered serving/transcoding | serving/hosting node meter | `energy` / `Count{joules}` | `EconomicEvent(consume)` | witnessable → credited (largely aspirational v1, §8) |
| **effect** | a citation authored / assessment passed after consuming | the citing/assessing act's own notarized entry | `effect` / `Count{citations\|passes}` — **never attributed attention** (B1) | `EconomicEvent` + `attestation:mastery`/`cite` | witnessable → credited *as the effect artifact*; the **lone-device escape hatch** (§5.4) |

The countable rows split into two honesty classes: **witnessable** (delivery / energy / effect — a second party or an
observable consequence exists) and **advisory-forever** (solo attention — no second party, no cost; un-forgeable as to
authorship, un-provable as to truth). §4 draws that line and defends it.

## 3. The witness ladder + consumption anti-forgery

### 3.1 Four rungs, one seam per rung

```
RECORDED     light-runtime writes a local tick to the Automerge / tamper-evident log. NEVER a DHT entry.
(advisory,   Signed by the RECORDER's own key → unforgeable AUTHORSHIP, zero corroboration of TRUTH.
 weight 0)   Offline-durable immediately.
CONVERGED    a CRDT sync round carries the record to a peer / household node. p2p_published_at: None →
(amber,      trust_label = amber. Weight still 0. A peer UPGRADES the record; it never AUTHORIZED it.
 weight 0)
CORROBORATED a DISINTERESTED party (§3.3) issues attestation:consumption-record, proof_evidence.class: witness.
(amber+,     Issuer key ≠ recorder key AND issuer has no value-flow interest in the object. The first genuine witness.
 weighted)
NOTARIZED    the witnessed pair promotes to an EconomicEvent (use|consume); its content-addressed pair-CID is
(green,      DHT-anchored; project_consumption credits at standing-weighted, disinterest-gated weight.
 credited)
```

Status is a **three-tier trust label, never a boolean** (`trust_label(dht_anchor.is_some(), p2p_published_at.is_some())` →
notarized > published > unconfirmed). A converged-only record is functional but **must never read as notarized.** These
four rungs are also the §1.4 reach-affirmation gradient: each rung is a further peer re-affirming reach, and propagation
velocity up the ladder is trust-earned (§6.5).

### 3.2 The record shape (RECORDED rung — local plane only)

This is **not** a DHT entry type and must never masquerade as one — self-reports don't belong on the notary floor.

```
LocalConsumptionTick {
  session_id:      Cid,          // = CID over {recorder, object_cid, coarse_time_bucket, device_nonce}
  object_cid:      Cid,          // the object; REA Resource
  substrate:       SubstrateSignal,
  unit:            Unit,         // minutes | pages | bytes | joules
  coverage_span:   ObjectSpan,   // half-open interval on the OBJECT's own coordinate (§3.5)
  monotonic_ms:    u64,          // duration on the MONOTONIC clock (trustworthy)
  wall_clock:      Option<i64>,  // device wall-clock; ADVISORY, LOCAL-ONLY, stripped at sync (§4.4)
  denominated_as:  "claimed",    // HONEST label: this is a claim, not a corroboration
  offline:         bool,
}
```

**Fleet-safety invariant (verbatim):** *empty-never-projects* — an abstaining/unmetered field stays *absent*, never
`""`/`0`, because empty values win the CRDT LWW merge and poison peers.

### 3.3 The seam — `attestation:consumption-record` (the CORROBORATED rung)

Add **one** subtype to the codegen'd `ATTESTATION_KINDS` whitelist — a DNA-lineage integrity-zome change, **not cheap**,
honestly costed. Issued through the existing `issue_attestation` coordinator
(`content_store/src/attestation.rs`) with `subject_kind:"content"`, `subject_cid = object_cid`, and
**`proof_evidence = { class:"witness", session_id, observed_substrate, quantity }`** — the persisted discriminator a
verifier reads is `proof_evidence.class` (the input's `proof_class` field is not merged into the stored metadata), and
Floor 8 (`attestation_validator.rs`) *rejects* a `proof_evidence` present without `class`. Widening the allowed
`proof_evidence.class` vocabulary is itself a Floor-8 integrity-zome (DNA-lineage) edit, not a free pass-through. Why it
is a genuine witness:

- **Issuer identity is unforgeable, never caller-supplied:** `issuer_cid = agent_info()?.agent_initial_pubkey` — the
  corroborating peer's own conductor signs it.
- **A new disinterest predicate (no existing base to "strengthen").** There is today *no* general content-subject
  self-signer guard: the only no-self guard is `create_vouch`'s signal-on-signal `signer_pubkey ==
  target_signal.signer_pubkey`, and content-targeting signals have no self-exclusion (`retraction` in fact *requires*
  self-equality). So the predicate below is **net-new coordinator logic (T8, not HDI)**, with `create_vouch` as
  *precedent for the pattern*, not a base being extended: **`signer ∉ {provider, receiver}` — the corroborating witness
  must have no value-flow interest in the event it attests.** A distinct key is *not* enough: the delivering node
  earns hosting credit, so its signature establishes *that a transfer occurred* (occurrence + identity) but confers
  **zero crediting weight** on the object it is interested in. Crediting weight requires a **third, disinterested**
  corroborator (or an effect-witness, §5.4). A creator streaming their own content to their own second device produces
  two *interested* signatures — two signatures, zero credit.

### 3.4 `evidence_class` is receiver-derived, never attacker-written

An attacker writing their own record must not stamp it `Instrumented`. `evidence_class` is **computed by the receiving /
converging node** from the corroboration set actually present (count of disinterested corroborators, presence of an
effect-witness), or absent. It is a property of witness-set cardinality, not a payload field. The advisory rollup (§6.2)
displays **witness-corroboration count, not consumer reach** — "12 minutes corroborated by 1 self-reported key" renders
visibly weaker than "…by 4 disinterested keys" — so a single-device farm inflates a number the UI simultaneously marks
un-corroborated. **Honest limit:** under §4.4 blinding this counts *witnesses*, not *consumers* — a 4-key witness cartel
(S2/§8) renders identically to 4 genuine consumers, so **breadth-of-consumers is a graduated signal** (needs the
set-membership machinery), never a v1 claim.

### 3.5 Two quantities, because they resist different lies

- **`coverage_span_union`** — the union of `ObjectSpan`s reached; loop-resistant (re-listening the same 3 minutes counts
  once) and **bounded by object length.**
- **`attention_spend`** — the raw monotonic sum (a re-listen counts twice; real time was spent).

Credited quantity is **coverage-capped per session, self-contained** — bounded by *that session's own*
`coverage_span_union`, computable with **zero global knowledge** (§6.3 — why the cap needs no privileged aggregator).
Cross-object streaming-farm resistance is not the cap's job; it moves onto witness-diversity + standing (§6). The
`raw/witnessed` divergence is itself a fraud signal (§6.5).

### 3.6 The honest anti-forgery boundary, stated once

The ladder proves **that ≥2 distinct keys — at least one disinterested — staked their signed standing on a corroborated
interaction.** It does **not**, and structurally cannot at v1, prove: that a human *attended* (solo attention is
un-witnessable, §4.2); or that the corroborators were not a colluding operator with multiple keys (Sybil/collusion
resistance is aggregate standing's job + graduated humanness credentials — §8, not this layer's). Calling anything here
"proof of attention" is the exact overclaim the honesty discipline exists to catch. The persisted discriminator is
`proof_evidence.class`, set to the honest value `witness` (never `proof`); the rung is "Record-corroborated," not
"Attested"; the subtype is `consumption-record`, not `consumption-witness`. (Widening `proof_evidence.class`'s vocabulary
is a Floor-8 integrity-zome / DNA-lineage edit — §3.3.) **These naming fixes land at v1** — the misleading affordance is
on the v1 surface.

## 4. The denomination model

### 4.1 Measure the spend, never mint a reward

A witnessed interaction is denominated in the substrate it *actually and witnessably spends*. The magnitude records
real-world cost — finite minutes, physical joules, bytes actually moved — **a debit against the spender, in strict REA
`consume`/`use` semantics, never a credit accrued to anyone.** We never invent a unit; we record the one the universe
already charged. This is the sharpest line vs BAT/Steemit: inflating the census requires spending the real resource
(self-defeating for attention; literally burning joules for energy).

### 4.2 Witness-strength is a property of the substrate

Different substrates have different natural co-witnesses, so they climb the ladder at different rates — the correct
security model, not a nuisance to normalize away.

| substrate | who else independently observed the spend? | strength | posture |
|---|---|---|---|
| `bandwidth` / `energy` / `storage` | the serving/hosting node metered actual delivery — **costed, two-party** | strong, but counterparty is *interested* (needs a disinterested third for credit, §3.3) | promotable **only via disinterested corroboration** |
| `attention` (solo, single device) | **no one** — only the recorder's own device saw the dwell | weakest, un-witnessable | **advisory-forever**, weight 0, private-dashboard-only |
| **effect** (a later citation/pass following consumption) | the later citation / passed assessment is itself notarized + observable | effect-witnessed | credited **as the effect artifact** (`Count{citations\|passes}`), never as attributed attention (§5.4; B1) |
| `governance` (affirm/dismiss) | the hook observes the edit independently of the agent | hook-observed, standing-weighted | advisory-sensing at v1 (sibling) |

**This asymmetry is a feature against Goodhart:** the most gameable substrate (solo attention) is exactly the one held at
advisory weight; the least gameable (energy/effect) is the one promoted. An adversary who wants weight must spend on the
hard-to-fake substrates.

### 4.3 `use` vs `consume` — the REA action, resolved

Attention spent on a **commons** object does **not deplete the object** — so the REA-honest action is **`use`** (the
resource survives). We reserve **`consume`** for genuinely depleting substrates: metered energy literally burned, a
metered-license play that decrements. Default is `use`. `use`-denominated engagement aggregates as *engagement
recognition*; `consume`-denominated energy aggregates as *resource expenditure* — different swimlanes (§4.5), never summed.

### 4.4 [PRECONDITION] Aggregate-without-a-subject — the census is a sum, not a dossier

As naively designed, the primitive ships a public, permanent, DHT-notarized, per-consumer reading/listening/watching
history — **strictly worse than a platform log, because it is immutable (correction-by-new-event only) and
world-readable.** For an anti-enclosure commons, disqualifying. The cure is agent-centric creation applied honestly:

1. **Consume records are private-by-default on the recorder's own source chain / local plane; the raw `(agent, object,
   time)` triple is NEVER gossiped.**
2. **The notarized aggregation record binds `receiver = object_cid` and the witnessed magnitude — it drops consumer
   identity in the clear.** The creator-credit path needs *that witnessed spend occurred on the object*, not *who spent*.
3. **Consumer identity, where it must appear, is a per-object rotating pseudonym or a blinded commitment** — disclosed
   minimally to the chosen disinterested corroborator, never queryable as a per-agent history.
4. **`wall_clock` is stripped at the convergence boundary** — load-bearing for surveillance, worthless for trust (the
   network-assigned action timestamp is the only trustworthy clock, §5.3). Keep it in the local-only doc if the user
   wants their own history.
5. **Private-reach content never enters the sync plane** (`reach_is_distribution_safe` — only `community|public|commons`
   sync). Consumption of private content is witnessed locally and never leaves the device.
6. **[B4] Blinding the consumer is not enough — the *corroborator* set leaks too.** §3.3 publishes each corroborator's
   real conductor key on-chain, and the disinterested corroborator of *consumption* is socially proximate to the
   consumer, so a DHT-permanent, world-readable set of real corroborator keys on a low-population CID (a dissident
   `.epub`, a stigmatized-health `.mp3`) *is* a map of that content's social cluster — and the household serving node
   (§5.4) accumulates the pseudonym→object disclosures the ledger refuses (one hub holding the dossier the commons
   promised didn't exist). So the *graduated* set-membership / threshold proof ("**≥K disinterested keys witnessed**",
   §8) is **pulled forward to v1 for the witness side** — the record proves cardinality without publishing which keys;
   until it lands, **notarizing corroborator identity in the clear is forbidden for any non-`public`/`commons` object.**

**The blinding-vs-independence tension, named (D4):** Sybil-weighting needs *stable-enough* witness identity to verify N
corroborators are independent; anti-surveillance needs *no* stable consumer identity. Resolution: **witnesses keep
pseudo-stable identity (independence is checkable); consumers are blinded (history is not reconstructable).** The census is
a *sum without a subject.*

### 4.5 The Goodhart firewall — measure layer ⟂ recognition layer

```
┌─ MEASURE LAYER ─────────────────────────────────────────────┐
│ witnessed events → per-CID Object Substrate Profile          │
│ • public, un-paid, un-owned CENSUS of real spend             │
│ • graph-derived, per-evaluator, NO canonical number (§6.1)   │
│ • gaming it = spending the real resource (self-limiting)     │
│ • consumer-blinded (§4.4)                                    │
└──────────────────────────────────────────────────────────────┘
        │  standing-weighted, disinterest-gated, LOSSY, NON-LINEAR
        ▼
┌─ RECOGNITION LAYER (governed, contestable) ─────────────────┐
│ StewardshipAllocation.recognitionAccumulated per contentId   │
│ • NOT a linear function of the measure                       │
│ • ratified (elohimRatifiedAt) + disputable (disputeId)       │
│ • dignity floor + accumulation ceiling                       │
│ • [PRECONDITION] firewalled from discovery/ranking (§6.4)    │
└──────────────────────────────────────────────────────────────┘
```

- **Substrates never collapse into one fungible number.** No protocol-level exchange rate between minutes, joules, bytes.
  The object's value is irreducibly multi-dimensional (currency swimlanes, not one token). Any cross-substrate comparison
  is a **governed, explicit, contestable act** (a `Mishpat::Precedent` or a per-collective policy manifest), never a
  hard-coded constant — collapsing the vector to a scalar *is* the enclosure move (§6.1, D3).
- **The measure→recognition map is deliberately non-linear and governed.** Goodhart needs a tight automatic proxy→reward
  coupling; we break it (standing-weighted, ceiling-bounded, ratified, reversible). Ten thousand fabricated minutes from
  blinded/low-standing keys project to ≈0 recognition.
- **Attention/consumption may NEVER cross into governance standing or reach.** The shefa gospel guard is absolute: *earned
  standing requires demonstrated mastery + sustained curation, never attention or consumption.* This must graduate from a
  `// subject: shefa-domain-gospel` review convention into a **typed impossibility** — the `governance`-substrate standing
  projection provably cannot take a count-substrate `EconomicEvent` as an input edge (D5). Until it is a typed edge, the
  firewall is *asserted, not proven*, and this is the highest-value hardening target.

## 5. Per-type extraction, offline reconciliation, hub-optional

### 5.1 Seam placement

The light-runtime is a **Track-3 spoke** — a phone/laptop/wearable that records interaction events and later syncs. It is
**not** the mod/plugin seam (the runtime *is* the process, not an injection) and **not** a new seam. It also runs **T1**
(DHT-notary identity, full-width across every device) and *may* graduate to **T2** (running `elohim-storage`/sync itself).
Concrete home: `steward/device` (Tauri shell) for the ephemeral spoke; `steward/node` for the always-on counterpart that
*accelerates* convergence but is never required. "Light-runtime" is a **footprint dial** (a cargo feature on the existing
runtime), not a new artifact.

### 5.2 The extractor registry (generalization of the labeling-function bank)

An extractor is a weak, **abstaining** sensor over a raw signal window — the direct generalization of the classifier's
labeling functions:

```rust
pub trait ConsumptionExtractor: Send + Sync {
    fn media_kinds(&self) -> &[MediaKind];
    /// Returns None (ABSTAIN) when the window carries no honest interaction
    /// (player paused, tab backgrounded, no page turn). Abstention is first-class,
    /// never a zero-tick — empty-never-projects.
    fn extract(&self, w: &RawSignalWindow) -> Option<InteractionTick>;
}
```

Content-type/MIME → `MediaKind` → extractor; unknown types fall to a **dwell-only fallback** (mount/leave `elapsedMs`,
reusing the scheme already live in `shefa/services/attention-tracker.service.ts`). **The governance-frame classifier is
just one more registered entry** whose `extract` emits a `Classification` tick instead of a `Count` tick — one registry
hosts both the countable instances and the classifier instance. Overlapping extractors **vote** like a labeling-function
ensemble. Emission mirrors the built pattern: **extractor tick → EventBus domain event → Automerge projection**
(`spawn_consumption_projection_listener`, shaped on `spawn_content_projection_listener`), never a direct network write.

### 5.3 Offline reconciliation and the untrusted clock

- **Record locally, no network in the loop.** The runtime writes straight to a local Automerge doc
  (`doc_id="consume:{object_cid}"`) — durable immediately, offline-complete, zero hub. RECORDED; a lone laptop never
  leaves it without ever losing function.
- **Converge, dedup by content-addressed identity.** On the sync round `session_id` (a CID over session invariants)
  collapses re-arrivals to the same CRDT node — idempotent replay, structural dedup (§2.2 applied to the event). Only
  broadcast-tier content enters the plane.
- **Duration is monotonic, not wall-clock.** "12 minutes" = `sum(monotonic_ms)`, defensible even if the wall-clock lies.
- **The *witnessed* timestamp is network-assigned.** *When* a session happened is trustworthy only at NOTARIZED, from the
  DHT action timestamp. An offline session recorded Tuesday, synced Friday, is honestly Friday-provenance /
  Tuesday-advisory — never Tuesday-authoritative. An offline buffer's self-claimed minutes never credit anyone regardless
  of magnitude (weight 0 until corroborated), which defangs buffer-inflation economically.

### 5.4 Hub-optional — a rung ladder with an economic escape hatch

Each rung *adds trust weight*; **none gates participation.** A lone laptop — no hub, offline forever — RECORDS its own
consumption, denominates its own attention locally, shows the person their own dashboard: complete function, surfaced as
*unconfirmed*, never notarized. But the primitive's *economic point* is crediting the creator, and CORROBORATED needs a
disinterested peer — a genuinely lone device would be frozen out of the accounting (the exact connected-vs-isolated
capture smell hub-optional exists to catch).

> **Invariant (hub-optional economic floor):** credited consumption is *always* reachable by a lone device via a later
> observable **effect** — a citation it authors, a `mastery` assessment it passes, a governance frame it touches — which
> emits an effect-witness credited **as its own artifact** (a citation, a pass — never as attributed attention, B1),
> **asynchronously, hub-free.** A hub only makes credit *faster*; it is never *required*.

**The effect emitter is disinterest/COI-gated too (B5).** The §3.3 `signer ∉ {provider,receiver}` gate governs
*corroborators*; the effect path is a *separate* credit route it never touched — so an effect entry (citation/assessment)
whose **author or grader is a beneficiary of (or stewarded-by) the target CID's `StewardshipAllocation` confers ZERO
consumption credit.** Only an independently-authored assessment or a third-party citation credits — closing the "creator
ships content + a self-authored trivially-passable assessment, every auto-pass self-credits" farm. (The hatch still ships
**paired with** the §6.4 discovery firewall, since even gated effect-credit must not feed ranking.)

**Residual gap, honestly named (S5):** the effect hatch credits *productive* consumption (the consumer cites / is
assessed / governs), **not *receptive* consumption** — a purely receptive lone consumer (a sermon `.mp3` played offline,
never cited or assessed) produces no effect, so their creator gets zero credit, re-creating a connectivity/production
bias for the most isolated audiences. Receptive-audience credit routes to the **graduated** `attestation:humanness` /
device-attestation path, not to any v1 claim of universality.

### 5.5 The iroh two-spoke convergence gap (honestly named)

The only *proven* convergence driver is libp2p `initiate_sync_round`, which in practice has only run with an always-on
(hub-shaped) node in the loop; two lone iroh-only laptops have **no proven reconciliation path** (`TransportBackend::Dual`
made gossip *receive* transport-neutral, but there is no periodic iroh round driver). So the hub-optional *guarantee*
honestly covers **RECORDED** (proven, offline-complete) and **CONVERGED-via-any-peer-running-a-round-driver** (proven);
**peer-to-peer convergence between two driver-less iroh spokes is NOT YET a participation guarantee.** Smallest fix: any
T3 spoke with unsynced records **opportunistically runs one sync-round driver tick on reconnect** (driving a round is a
transient action, not a standing role) — closing the two-laptop case without either being a hub. Until built, **asserted,
not proven.**

## 6. REA aggregation on the object CID + un-enclosable economics + anti-gaming

### 6.1 The aggregation is a per-evaluator derived view — there is no canonical number

**Storage is projection, not truth.** Witnessed events live on the DHT (notarized `EconomicEvent`s) and the CRDT plane
(pre-corroboration ticks); the accounting is a *derived view* recomputed on arrival, exactly as
`standing_projector::project_signal` recomputes a `standing_view` row. The projected row is keyed on `evaluator_pubkey`:

```
ConsumptionLedgerRow {
  evaluator_pubkey,             // WHOSE view — the pluralism axis
  object_cid, substrate,        // one row per (object, substrate)
  witnessed_quantity,           // Σ (event_qty × witness_weight) — the credited total
  raw_quantity,                 // Σ event_qty un-weighted (advisory, claimant-view only)
  distinct_disinterested_keys,  // labelled "(NOT Sybil-resistant pre-identity)" at v1
  last_event_at, policy_manifest_cid,
}
```

**Pluralism is the un-enclosability keystone:** different evaluators legitimately see different profiles because each
projects through *their* manifest subscriptions and the standing *they* assign. There is no global consumption number for
anyone to own, sell, or clawback. A "canonical" figure is a *convergence* of many peers' independent projections, never a
single authoritative table. Where a witnessed pair must promote to a notarized `EconomicEvent`, the promoted event is
**content-addressed: pair-CID = hash(tick_CID ⊕ corroboration_CID)**, so any node holding both halves derives the
byte-identical event — no projection is privileged (rejecting the "whoever-pairs-first-is-the-ledger" design). Bare events
with no corroboration project with `weight = 0`.

### 6.2 The advisory rollup is diversity-first

The v1 UI rollup surfaces **witness-key-diversity, not summed magnitude** (§3.4), so an inflated single-device number
reads visibly un-corroborated. Un-witnessed consumption renders only in the claimant's own private dashboard ("you've read
40% of this book") — genuinely useful, zero fraud surface, credits no one.

### 6.3 The coverage-cap stays decentralized

The cap is **per-session, self-contained** — bounded by that session's own coverage measure at record time, computable
with zero global knowledge. No "per-period global fold," no privileged aggregator, no re-enclosure. It bounds single-object
loop inflation only; cross-object farm resistance lives in §6.5–6.6.

### 6.4 [PRECONDITION] The recognition ⊥ discovery firewall

The entire safety case rests on "witness, don't pay → no bounty." That is a **non-sequitur unless recognition is inert** —
and recognition is the natural input to content discovery/ranking. Farming witnessed consumption to farm *visibility* is
the enclosure attack in a recognition costume, and it pays *now*, even with no token, using **aged/purchased-standing
keys** (not fresh Sybils). Therefore:

> **Hard invariant (typed, not conventional): `consumption-recognition ─✗→ discovery / ranking / surfacing`.** At v1,
> consumption-recognition is visible **only** in the claimant's and the creator's own views and is **never** an input to
> any cross-peer ranking function. Its sole downstream consumer is the graduated settlement layer (§6.6), itself gated.

This extends the standing firewall (§4.5) from the *governance* axis to the *visibility* axis, and it is the fix that
makes "no payout → no bounty" actually true.

**[B2] The edge is already OPEN — this precondition must SEVER a live one, not assume greenfield.**
`contributor_presences.rs` already does `order_by(recognition_score.desc())` behind a `min_recognition_score` filter,
and `recognition_score = affinity*0.6 + citation_count*0.4` — so `citation_count`, which the §5.4 effect hatch pumps, is
*already* an input to a ranking/surfacing query. Written forward-only ("never an input"), the firewall compiles green
while the leak persists. So the concrete edge to cut is **`contributor_presences::recognition_score`**: consumption/
effect-derived recognition must live in a field **provably absent from any `order_by`/ranking**, and a **regression test
must assert no consumption-lineage value reaches `recognition_score`** (or any surfacing sort key). This is a v1 code
change, not a future invariant.

### 6.5 The anti-gaming weight function

`witness_weight` is where anti-gaming lives (bootstrap constants, `standing`-derived, mirroring `DefaultDebitWeightPolicy`):

```rust
fn witness_weight(evaluator, event) -> f64 {
    if event.corroborators.is_empty() { return 0.0; }              // un-witnessed → 0 (bounty kill)
    let mut w = 0.0;
    for c in event.distinct_corroborators() {
        if c == event.recorder { continue; }                        // no-self-witness
        if c == event.provider || c == event.receiver { continue; } // §3.3 disinterest gate
        w += match Standing::evaluate(evaluator, c, conn) {         // per-evaluator, pluralist
            Unknown | Computed(Floor)   => 0.0,   // v1: distinct keys are near-free → count NOTHING
            Computed(Low)     => 0.25,
            Computed(Neutral) => 1.0,
            Computed(High)    => 1.5,
            Computed(Trusted) => 2.0,
        };
    }
    w.min(WITNESS_SATURATION)   // diminishing returns — no single CID pumped unboundedly
}
```

Four properties: **(1)** self-witness structurally forbidden (generalized no-self-vouch, T8-coordinator-gated, not HDI —
the HDI validator can't do cross-entity lookups); **(2)** interested counterparties contribute zero credit (§3.3
wash-trade kill); **(3)** at v1, **witness-*count* contributes nothing until humanness/uniqueness graduates** — only
*earned standing* corroborates, so free distinct keys don't buy corroboration; **(4)** saturation caps per-event credit,
so strength comes from *breadth of independent objects genuinely spent-upon*, not depth of collusion on one. This function
is also the propagation-velocity curve (§1.4): weight rises as disinterested standing accrues — trust earns adoption speed.

### 6.6 Credit is reconciliation, not payout — and un-enclosable

Credit flows through machinery that **already exists** — no second ledger. `witnessed_quantity` apportions per the existing
`StewardshipAllocation.allocationRatio` split (author 0.8 / editor 0.2), accruing to
`ContributorPresenceView.recognitionByContent`, contestable via `governanceState`/`disputeId`/`elohimRatifiedAt` before
final. A creator who hasn't joined accrues to an `unclaimed` presence that transfers on verified claim.

- **v1 credit is a recognition *reconciliation*** — a running, contestable, correctable-by-new-event acknowledgment, **not
  a mintable/transferable/spendable balance.** Nothing to cash out → no bounty to farm.
- **Settlement is graduated** — conversion into any spendable mutual-credit sits behind the SETTLEMENT/STEWARDSHIP layers,
  gated on identity + anti-gaming maturity + the constitutional **dignity-floor / accumulation-ceiling** bounds.
  Deliberately **not** built in v1.
- **Conflict-of-interest edge exclusion:** a corroboration where the corroborator *or* recorder is a beneficiary of (or
  stewarded-by) the target CID's `StewardshipAllocation` contributes **zero** recognition weight — you may be credited for
  corroborating *someone else's* content, never for bootstrapping your own.

**Un-enclosability, layer by layer:** agent-centric creation (no central mint); computed-not-stored profile (you cannot
enclose a number that does not exist); currency swimlanes (no fungible master unit to corner); immutable
correction-by-new-event; bridge-not-owner to external VF/hREA (each translation logs a `TranslationPoint` with
`semantic_cost`). Two enclosure points that reappear one layer down are closed explicitly: **(a)** the recognition
**ratifier** and the cross-substrate **exchange-rate manifest** must be **plural/sortition-based, contestable via
`Mishpat::Precedent`**, never single-role admin config; **(b)** every served projection carries a **commitment**
`witnessed_quantity = f(event_cid_set_hash, policy_manifest_cid, evaluator)` so a consumer can *verify* a doorway's number
and recompute from public events rather than *trust* it — enclosure-by-serving-position becomes auditable, not
authoritative.

## 7. Relationship to the governance classifier

The two are **not analogous — they are the same primitive up to a choice of magnitude algebra.**

| denomination slot | consumption instance (`.mkv` / delivery) | governance instance (classifier) |
|---|---|---|
| `object_cid` | the video / content CID | the rule CID (`Mishpat::Precedent`) |
| `substrate` | `bandwidth` / `attention` | `governance` |
| `action` | `use` \| `consume` | `affirm` \| `dismiss` |
| `magnitude` | `Count{47,"minutes"}` — additive monoid, ℝ≥0 | `Vote{±1}` — ordinal tally, signed |
| `observed_by` | light-runtime meter / serving-node counter | the guard + classifier hook |
| carrier | `EconomicEvent` | `FeedbackSignal` |
| aggregation | standing-weighted tally over object CID (raw sum advisory-only) | standing-weighted tally over rule CID |

The two carriers are the additive-monoid and ordinal-tally realizations of one `(object_cid, substrate, action,
magnitude)` envelope. Everything the sibling proved on the hard substrate applies unchanged: the
local→peer-validated→notarized ladder; the no-self-vouch guard (here *strengthened* to `signer ∉ {provider, receiver}`);
advisory-at-v1; hook/instrument-observed-not-agent-claimed; the asymmetric-automation rule (weight rises as corroborators
arrive without ratification; never tighten into a mechanical penalty without a human).

**And the reflexive case closes the loop:** the classifier's §10.4 makes *an agent's own governance action* a
witnessed-interaction event on the DHT — its resolution a `governance-action:frame-resolution` record, its provenance the
unforgeable `agent_info().agent_initial_pubkey`, its compute a pluggable source (terminal → API-key → peer-native) audited
on the DHT. That is *this primitive applied to the guardian*: the object is the rule/content CID, the substrate is
`governance`, the witness is the coordinator verifying the signer, the ladder is the same four rungs. The classifier
governs bytes; this primitive witnesses their consumption; and both witness the agents that operate them — one ladder.

**What each owns:** the **classifier** owns the Frame Ontology, the defeater/detector split, Layer-A/Layer-B, and the
escalation ladder — contested-meaning machinery with *no analog in a count* (for counts, the escalation ladder collapses:
a minute is a minute, nothing to escalate). What transfers is the *witness* ladder, the CID auto-dedup theorem,
advisory/graduated, and abstention (a paused player emits no tick, exactly as a labeling function abstains). **This spec**
owns the numeric magnitude + `hasDuration`; offline reconciliation (proven fresh here — the classifier's hook fires only
where connected); the witness-strength-by-substrate table (§4.2); and the denominate-in-witnessable, attention-advisory-
forever discipline (§4.1) the classifier did not need because its witness — the hook — is structurally external, whereas
solo consumption's is not. **The one structural asymmetry:** governance and delivery/energy have a built-in second party;
solo attention does not — which is precisely why the witness slot's strength is allowed to *vary by substrate*.

## 8. v1-buildable vs graduated (honest)

| concern | v1-buildable (dev-tooling / household-node — the stable floor) | graduated (brit / eprfs / Mishpat DHT) |
|---|---|---|
| **light-runtime capture** | local Automerge `LocalConsumptionTick`, offline, monotonic-clock, advisory, `wall_clock` local-only | + notarization on the slower DHT path |
| **witness** | dwell-fallback + `.epub` page-turn + governance-frame extractors; energy/bandwidth co-metered by household serving node | independent-peer/coordinator `attestation:consumption-record`; cryptographic delivery proof (`proof_evidence.class:"proof"`) |
| **identity** | self-reported `{claude\|codex\|gemini\|human\|device}` / device key — **spoofable, advisory-weight** | coordinator-derived `agent_info()` pubkey both sides |
| **disinterest gate** | `signer ∉ {provider,receiver}`, coordinator-enforced (**NOT** integrity-validated; advisory-weight is the mitigation) | T8-coordinator + manifest-wired Attestation Floors 2/4/6 (today `accept-all TODO(C.3)`) |
| **carrier** | `EconomicEvent(use/consume)` local write → Automerge → CRDT converge | + DHT-anchored content-addressed pair-CID, `dhtAnchorHash` |
| **aggregation** | `consumption_projector` + per-evaluator `ConsumptionLedgerRow`; **bootstrap** weights | manifest-driven weights (the T17 analog) |
| **weight** | **advisory only** — no payout, no reach, ever; witness-count contributes 0 | standing-weighted; graduated-Sybil-resistant; still bounded/ratified/reversible |
| **credit** | `recognitionByContent` reconciliation via existing `StewardshipAllocation` split — **no settlement** | settlement to spendable mutual-credit under dignity-floor/ceiling + sortition ratifier |
| **discovery firewall** | typed `recognition ─✗→ ranking`; recognition visible only to claimant + creator | same invariant, enforced across federated surfacing |
| **surveillance** | consumer-blinded census; private content never syncs; `wall_clock` stripped at sync | set-membership proofs for "≥K disinterested keys witnessed" without binding who consumed |
| **offline convergence** | proven via a peer running the libp2p round driver | two driver-less iroh spokes: opportunistic reconnect-tick (**asserted-not-proven** until built) |
| **energy substrate** | **largely aspirational** — per-playback device-energy metering is hardware-specific, mostly unavailable | metered joules where hardware exposes it; higher advisory floor than attention |

**Honesty ledger — asserted, not proven (inherited discipline):** that standing-weighting makes recognition
Sybil-resistant against *colluding pairs* (the ladder proves ≥2 distinct keys, ≥1 disinterested; it does not prove they
weren't one operator — collusion resistance is aggregate standing + graduated `attestation:humanness` + the sortition
floor); **specifically NOT-YET-RESISTED: a reciprocal-corroboration ring** (A→B→C→A, each a *legitimately distinct,
real, high-standing* identity corroborating CIDs it is not provider/receiver/beneficiary of) passes every per-event gate
— the disinterest gate has no notion of cross-CID reciprocity. The mitigation is a **bidirectional-corroboration-density
dampener** in `witness_weight` (down-weight keys that corroborate each other's objects, §6.5/G3), and until it lands this
class is *named, not closed*. That the measure→recognition non-linearity is *sufficient* to defuse Goodhart (v1 sidesteps
it by attaching **zero** payout). **Not-yet-built:** `consumption_projector` + `ConsumptionLedgerRow`; the `attestation:consumption-record`
subtype (DNA-lineage); the light-runtime `ContentConsumed` domain event + projector; the content-addressed pair-CID; the
typed recognition⊥discovery and consumption⊥standing edges; the consumer-blinding wire change; the iroh reconnect-tick
driver. **Structural v1 gap:** the disinterest gate is coordinator-only, not integrity-validated — the enforced property is
"signed by a key other than consumer/provider/receiver," nothing about collusion or attestation legitimacy — *which is
exactly why v1 weight is advisory and unpaid.* The gap is disclosed; the advisory posture is the mitigation.

## 9. Open decisions for the human architect

- **D1 [MUST RATIFY FIRST] — The denomination ceiling (Option A vs B).** This spec commits **Option A**: credit only the
  *witnessable* (delivery / energy / effect); attention-minutes recorded but *permanently advisory*, never in
  `resourceQuantityValue`. It is the only option consistent with the anti-corruption requirement and the only one that
  generalizes the classifier's external-hook property honestly. **Option B** (credit claimed attention attested by any
  second key) is intuitive but un-witnessable for solo consumption and quietly reintroduces the proof-of-attention bounty.
  Adopting A means telling creators: *"we credit that your work was fetched, served, or demonstrably acted upon — not that
  it was savored."* **Recommendation: ratify Option A** (this spec is incoherent under B). This is a values fork the
  operator/collective owns.
- **D2 — Is `attestation:consumption-record` honestly named for solo consumption?** For genuinely private solo
  consumption no peer witnessed the *act* — the peer witnessed the *record's integrity and propagation*. This spec commits
  to record-integrity semantics and restricts credited consumption to co-present / effect-witnessed / device-attested.
  Confirm the narrowing.
- **D3 — Where does the cross-substrate exchange rate live?** §4.5 forbids a protocol constant. `Mishpat::Precedent`
  (contestable case law), per-collective policy manifest (config), or per-evaluator (each community weights its own
  swimlanes)? **Lean: per-collective manifest, contestable via Mishpat, sortition-ratified** — confirm. *Anti-dominance
  (N1):* a single widely-subscribed manifest becomes de-facto canonical by Schelling-point even without a protocol
  constant, so **manifest-adoption concentration is itself a contestable governance signal** (`Mishpat::Precedent`); and
  the served `witnessed_quantity` commitment (§6.6b) needs a **light-client verification mode** — carrying the standing
  inputs to recompute, or checkable against a threshold attestation — so a light T3 consumer can *verify* without holding
  the whole per-key standing graph.
- **D4 — The blinding-vs-independence resolution.** Confirm the mechanism (per-object rotating pseudonym vs blinded
  commitment vs threshold/homomorphic membership proof) and the exact point past which even the aggregate reveals nothing
  re-identifiable.
- **D5 — Structural firewall vs convention.** Should `consumption ─✗→ standing/reach` and `recognition ─✗→ discovery` be
  **typed impossibilities** (the projection provably cannot take the forbidden edge as input) rather than review
  breadcrumbs? **Recommendation: yes, typed** — the highest-value hardening target.

## 10. Decomposition seed

Gaps in dependency order — the **[PRECONDITION]** pair gates everything downstream: **(P1)** consumer-blinding + the
aggregate-without-a-subject wire shape + **the ≥K set-membership witness proof (B4, corroborator keys never in the clear
for non-`public`/`commons`)** (§4.4); and **(P2)** the typed `recognition ─✗→ discovery` + `consumption ─✗→ standing`
edges — which must **SEVER the live `contributor_presences::recognition_score` order-by edge** (B2: `citation_count` is
already a ranking input) with a regression test, not merely declare a forward invariant (§6.4, §4.5) — *both land before
any wire code*; **(G1)** the extractor registry + `ConsumptionExtractor` trait + dwell fallback (generalizing the
classifier's labeling-function bank) + `LocalConsumptionTick` Automerge doc, offline, monotonic-clock; **(G2)** the
the `Count`/`Vote`/`Classification` magnitude algebra unifying the two carriers (governance = a `Magnitude` variant, **NOT** a `SubstrateSignal` member — DNA-hash-moving, frame-witness-primitive-architecture §2.3); **(G3, DNA-lineage)**
the `attestation:consumption-record` subtype (payload `proof_evidence.class:"witness"`, B3) + the `signer ∉
{provider,receiver}` disinterest gate (net-new coordinator logic, S3; coordinator-level at v1) + a
**bidirectional-corroboration-density dampener** in `witness_weight` (S2, reciprocal-ring resistance); **(G4)** the
`consumption_projector` + per-evaluator `ConsumptionLedgerRow` (corroboration-count-not-consumer-reach rollup, S4) + the
content-addressed pair-CID; **(G5)** the effect-witness lone-device escape hatch, **COI-gated on the effect emitter (B5)
and paired with P2**, `effect`-denominated never attention (B1) (§5.4); **(G6)** the iroh reconnect-tick round driver
(§5.5); **(G7, graduated)** notarization + coordinator identity + settlement under dignity-floor/ceiling + sortition
ratifier + the pulled-forward set-membership proof. P1–P2 + G1–G2 are the household-testable spine; G3–G6 wire the witness
ladder; G7 is graduated.

*Honest one-line summary:* a light-runtime records local consumption offline — durable, private, and **advisory by the
structure of the world**; it becomes a *witnessed* fact only when corroborated by a **disinterested** party (`signer ∉
{provider, receiver}`) or an **observable downstream effect**, promotes to a ValueFlows `use`/`consume` event
**denominated in delivered bytes / metered joules / demonstrable effect — never claimed attention**, aggregates on the
object CID as a **consumer-blinded, per-evaluator census with no canonical number**, and feeds creator recognition through
the existing *ratified, contestable, reversible* stewardship machinery — **never an automatic mint, never touching
governance standing, firewalled from discovery.** *Witness, don't pay; measure the spend, govern the credit; credit that
the work was fetched, served, or acted upon — not that it was savored.*
