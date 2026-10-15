---
title: "Commons composition at scale: attribution that closes its account, from one work to the planetary commons"
id: commons-composition-at-scale-2026-10-15
status: Capture
date: 2026-10-15
sovereignty-frame: bounded
steward: human:matthew
author: "Claude, at the operator's direction (design session, 2026-10-15)"
tags: [commons, attribution, cite, rea, lens, gradient, vsm, algedonic, presence, rollup, bridges, settlement, research]
cites:
  - "elohim-protocol-manifesto | manifesto | sha256:bdfd593210e7fea0 | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "values-forward | values-forward | sha256:0bfa006fd772f7d9 | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "succession | succession | sha256:4ae356e1bf2112e6 | path: genesis/docs/content/elohim-protocol/succession.md"
  - "reliance-as-cite-shared-provenance-primitives-design | Reliance as cite | sha256:01960d43aba26895 | path: genesis/docs/superpowers/specs/2026-10-15-reliance-as-cite-shared-provenance-primitives-design.md"
  - "commons-pool-as-collective-party-design | A commons pool is a collective that is party to its members' commitments | sha256:c194ddbe3dc3b2a6 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - "care-aggregation-adoption-policy-floor-design | Care Aggregation on the Deterministic Floor | sha256:5bd65c4f72fd79c2 | path: genesis/docs/superpowers/specs/2026-07-31-care-aggregation-adoption-policy-floor-design.md"
  - "recursive-architecture-design | THE RECURSIVE ARCHITECTURE | sha256:053f260af9989d4b | path: genesis/docs/superpowers/specs/2026-06-14-recursive-architecture-design.md"
  - "plural-mishpat-lenses-over-epr-design | Plural Mishpat Lenses over an EPR | sha256:14dab62ad1fb5f87 | path: genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md"
  - "requisite-variety-guidestar-epr-family-composition | Requisite variety as the guidestar | sha256:e1cf9e52fbe95c11 | path: genesis/docs/superpowers/specs/2026-08-12-requisite-variety-guidestar-epr-family-composition.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "private-thought-governed-fruit | private-thought-governed-fruit | sha256:2945eb608257be1e | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - genesis/data/timeline/backlog/commons-pool-as-economic-authority-wallets-in-bridge-layer.md
  - genesis/data/timeline/backlog/citation-apparatus-claim-grain-typed-edges-reach-gate.md
  - genesis/data/timeline/CONVENTIONS.md
  - elohim/epr-rea/src/model.rs
  - elohim/epr-rea/src/external.rs
  - elohim/epr/src/witness.rs
  - elohim/elohim-storage/src/recursion.rs
  - elohim/elohim-storage/src/services/aggregator.rs
  - elohim/elohim-storage/src/graduation/summary_event.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs
  - bridges/valueflows/valueflows-types/src/lib.rs
  - elohim/brit/docs/specs/2026-06-29-epr-meta-composition-snapshot-canonical-cites-design.md
  - elohim/elohim-storage/research/economic-systems-research.md
  - genesis/research/commons-flow-sim/flow.py
---

# Commons composition at scale

The operator asked how commons compose. Attribution should flow outward from one person's work,
through what that work relies on, until it dissolves into the planetary commons. It should respect
the line political economists drew between labour and land, and no peer should become a bottleneck.
He offered images, not a specification: LinkedIn degrees, Six Degrees of Kevin Bacon, "a rich
network of drips", a "vector cloud of presences", $1,000 thinning to a billionth. This note tests each
image against canon and against a runnable probe over the repository's own `elohim/Cargo.lock`. Where
an image holds, it is kept; where it does not, the note says so and proposes what fits.

**The short answer.** Make every split a table of ratios that sum to exactly 1, each ratio carrying
its reason. A person judges that table once, when they adopt a lens version, and every later flow
reuses it. Read the result by the *subject* of each reason. A maker means labour. A community means
the land-like, positional part. A reason that names nobody means unexplained value, which is placed
at the commons with its descent and is not erased. The account closes when the explained part and
the placed remainder reach 1. That closing is the positive signal the operator asked for, and it runs
both ways along the same reliance edges.

Terms used throughout. A **presence** holds recognition for a maker not yet on the network, until
they claim it (manifesto.md:627). A **cite** is the record that a work draws on something without
consuming it (`ReaVerb::Cite`, witness.rs:314). A **lens** is a versioned, method-cited valuation rule
that people choose, contest and replace; plural lenses coexist (plural-lens spec §3). **Reach** is how
far a thing travels beyond its author's circle, from private to commons. A **holon** is a whole that
is part of a larger whole: a person, household, collective, region. The **head plane** is the
recurring per-item cost every notarized item adds on every peer (p2p-design-gate §1.5). A **rollup**
is a content-addressed aggregate that keeps pointers down to what it aggregates (`CoverageRollup`,
recursion.rs:192). **Class C** means operational: delete it and rebuild it from records that are kept.

## 1. The question

Can value that enters at one work be attributed outward through degrees of reliance? It should stop
being anyone's in particular at a principled point and reach the planetary commons. It must do this
without any peer computing the whole graph, without a default allocation, and with persons deciding
what only persons may decide.

## 2. What canon already settles

- **The seat is held, and the valuation deferred.** "For unclaimed presences, this is held in
  constitutional trust" (manifesto.md:659). The protocol "records the facts and defers the
  valuation" (:661). For software: "each one recorded with how the work relies on it, each with a
  presence held in trust until its makers claim it" (:702).
- **Citing earns reach.** Reach "is earned partly by that support" (:696). Retraction reaches "the
  claims that rested on the retracted source", and "a prompt correction is honored as a contribution"
  (:696).
- **The labour/land line.** "What you make — the produce of your labor — is yours"; what no
  individual made is common inheritance. The sorting test is **reproducibility**: the common
  inheritance is "non-reproducible and positional, valuable only because a community clustered around
  it" (values-forward.md:127). Graduation is "a negotiated schedule, never a seizure" (:131).
- **Refusals.** No "token, a treasury, or a single fungible rail" (succession.md:605). Reach cannot
  be purchased (:606). Refuse "any published default allocation" (:608).
- **The arrow points outward.** The protocol "maximizes externality emission" (manifesto.md:766).
- **Aggregates must be recomputable.** "Every T2/T4 aggregate MUST declare its T1 anchor set and a
  recompute-and-verify path" (plural-lens spec:103).
- **Wallets live in the bridge layer.** The strong identity binding survives only at "the cash-out
  edge" (wallets-in-bridge-layer backlog, "honest residue").
- **The ground floor.** The reliance spec's table puts code, a co-op grocer and a garden network on
  one set of records. Its missing node 7, from member records to a commons, is where this note
  picks up.

## 3. The method: trust prices compute

The `gradient-reading` skill is the method here, not one lens among several. Its rule: "Verification
is paid once, as close to the human judgment as possible, and everything downstream relies on that
minted trust."

**The ratio table is the carried proof.** When a person adopts a lens version for their own work,
the table of ratios for each node is fixed by three content-addressed inputs: the node's cite record
(its lockfile summary), the lens version, and the reverse-index rollup reading that the land-like
share uses. The table is judged once. Every later inflow multiplies by it and pays only a cheap
check: the ratios sum to 1, and each reason resolves. The unit conversion runs from per-payment to
per-relationship and per-lens-version. In the probe, 12 inflows under one lens judged 303 tables and
reused them 3,333 times (§14).

**Three planes stay apart.** Bytes are the cite events: self-certifying and cheap. Meaning is the
lens: small, and adopted with ceremony. Settlement is money at a bridge. Settlement never waits on a
lens election. It uses the last adopted tables, marked pending if a new version is contested. A
person never waits on background propagation. The precision floor needs an amount, so it belongs to
settlement and is applied where amounts are derived (`apply_floor` in the probe), never inside the
table.

**The guard.** The ceremony is compressed, never skipped. No shortcut here weakens a signature, a
reach check or a witness check. The carried table is checked at every hop. It is just not re-judged.

## 4. The seams

Three gates were run on this design: `local-first`, `p2p-design-gate` and `elohim-commons-design`.
They pull against each other in eight places. Tags use the vocabulary in `CONVENTIONS.md`
§Trust context through §Cost unit. The format is relationship · friction · plane · unit · phase ·
lane.

| # | Tension | Gates | Resolution: who yields, and why | Gradient tags | Missing |
|---|---|---|---|---|---|
| 1 | One person's attribution must work alone, but the land-like share needs "who relies on X", an aggregate | local-first vs commons Q3 | The aggregate yields. The forward split runs on the device from the person's own lockfile and adopted lens. The positional ratio reads a pinned rollup CID, marked stale with its `as_of_heads`. If none is held, that ratio is zero and the table says why. The rollup adds; it is never a precondition | trustful-self · friction-wait avoided · plane-value on plane-projection · unit-once · phase-steady · lane-interactive | a reliance rollup domain (MN-4) |
| 2 | Per-split records must stay off the head plane, but members must see the resource's condition and draws without a steward | p2p gate §1.5 vs commons Q3/Q5 | Storage yields; visibility is kept by recomputation. Splits are class C, rebuilt from cite summaries (35 heads for the repository, per the reliance spec) plus lens CIDs. Only a per-window rollup atom per pool and the settlement transfers are heads | trustful-declared · friction-verify, compressible · plane-value kept off plane-notary · unit-item→unit-once per window · phase-steady · lane-background | a Path 2 caller (reliance spec node 6); a rollup-atom carrier (MN-6) |
| 3 | Persons hold the lens, the vote and the settlement, but scale wants machine-speed propagation | commons "Never" column vs scale | Neither yields on its own ground. A person judges once per lens version; propagation decides nothing and only multiplies carried tables. Settlement runs under a standing mandate the payer approved, batched per window. Any rung past notice returns to persons | trustful-declared/earned · friction-verify→one act per relationship · plane-authority apart from plane-value · unit-once per lens version · phase-transition on a new version · lane-background | lens bound to a node (MN-9) |
| 4 | There is no pool-elohim, yet an unclaimed presence's outflow needs a lens | commons vs flow | The steward pool's members adopt one lens for the presences they hold (charter Q1); any member's elohim computes it. On claim, the makers' own lens replaces it | trustful-earned · friction-verify · plane-authority · unit-once per pool lens · phase-transition at claim · lane-background | `steward_id` is one optional string (lib.rs:522) |
| 5 | A hot fan-in node (serde, proc-macro2) against O(out-degree) work per hop | p2p cost vs flow | Owed, never routed to trust. Recognition is pulled: a fold on read, cached by `rollup_hash`. Money is pushed only at the edge, once per window per declared channel. Tiers aggregate on receipt | friction-mechanical · plane-value · unit-peer (fan-in)→unit-once per window · phase-growth · lane-background | none for recognition; settlement bridge (MN-7) |
| 6 | The reverse index needs holders. More holders is not more resilience | gradient Q4 vs commons Q11 | Custody by independence. Holders are chosen across failure domains, with the k-of-M threshold set as a separate dial. At commons reach this is trustless ceremony, where it belongs: witnessed rollup atoms | trustless · friction-verify, owed · plane-custody · unit-peer · phase-steady · lane-background | `witness_quorum: 0` hard-coded (recursion.rs:247) |
| 7 | Settlement must not wait on the meaning plane's ceremony | gradient fused-planes | Settlement reads the adopted tables and pays declared channels. A disagreement about a lens resolves in the next window and never holds the money path. The strong identity binding is owed here, at the bridge | trustless at the bridge · friction-verify, owed · fused-planes avoided · unit-call per channel per window · lane-interactive (payer's approval) | outflow bridge (MN-7) |
| 8 | A public reverse index against what payers reveal by paying | local-first "what does it learn" vs commons Q5 | Separation is priced on purpose. The payer's own split record is private (class B) until they publish it. A pool counts payers only at k≥5 (`aggregator.rs:90`). Recognition at a public package's presence is public | trustful-declared · friction added on purpose · plane-attention/value · unit-item | aggregator has zero production callers (care-aggregation §2) |

**The ratio-plus-explanation invariant sits on these seams.** It makes a split locally checkable with
the node's own records and the lens (row 1). It keeps per-split records rebuildable, since a table is
a pure function of three CIDs (row 2). It leaves the lens a person's adopted choice, named by CID in
every ratio (rows 3 and 4). Four places it breaks:

- **Reach.** A grocer's reason may be a private invoice. A verifier outside that reach can check
  that the ratios sum to 1, but sees only the reason's hash. The check degrades to Attested-Private,
  as `DepEdge` already does (model.rs:477-481).
- **Form, not truth.** A fabricated cite passes both checks. Its truth rests on the cite record's own
  governor: cargo for `Governor::Compiler`, or human judgment at the reach gate for `CiteSeal`
  (model.rs:450-467).
- **Exactness costs digits.** Amount denominators reached 31 digits with the floor and 54 without.
  The bounded alternative is integer minor units, with the rounding remainder placed as
  `below-floor`, which is the same invariant.
- **Explained is not labour** (§6).

## 5. The degrees view

**Degrees of what.** In the probe, the entry work (28 workspace crates treated as one work) relies
on 463 crates. The table counts nodes by shortest-path degree:

| Degree | 1 | 2 | 3 | 4 | 5 | 6 | 7 |
|---|---|---|---|---|---|---|---|
| Nodes | 56 | 167 | 124 | 67 | 27 | 19 | 3 |
| Share of the $1,000 kept by makers at that degree | 38.5% | 6.5% | 0.35% | 0.003% | 0 | 0 | 0 |

The entry work keeps 50%. The node count does not explode the way LinkedIn's does, because a
dependency closure is finite and heavily shared: the "third degree" is mostly the same crates met
again. What explodes is *paths*. There are 20,906 distinct paths into `unicode-ident`, and a naive
per-path push would send 67,076 messages.

**Six Degrees is a warning.** Everything is near everything (the diameter here is 7), so distance
cannot define the commons. Nothing becomes common inheritance because it is far away.

**What a person sees, by reach.**

- **Forward**, for a citer: their own bibliography, the first-degree cites they authored. Deeper
  degrees are computed on demand, as a detail.
- **Reverse**, for a maker: "Here's what your work has meant to people" (manifesto.md:716), read
  from the reverse index.

Degree is not a holon level, and neither is reach. A tier commons is a holon of *stewards* (§7).
Recommendation: lead with the forward bibliography and the reverse "what it meant". Keep degrees as
a depth view on demand.

## 6. The dissolve point and labour/land

**Candidate 1: a precision threshold.** It fails the reproducibility test as a definition. A share
below one cent of serde's labour is still serde's labour: reproducible, and made by someone. A
threshold is an *epistemic* boundary (attribution would cost more than it is worth), not an
ontological one. Kept as a declared cost choice, it is honest. Sub-floor shares pool at the tier
with descent, and a finer lens can re-attribute them later.

**Candidate 2: "explanation runs out" (the ratio invariant).** Read every ratio's reason by its
subject:

- a **maker** (the node's own presence, or a cited upstream): labour;
- a **community** (the reverse-index rollup, where many works clustered around one): the positional,
  land-like part. This passes the reproducibility test. A standard everyone relies on is valuable
  because they rely on it;
- **nobody**: unexplained. Either `no-record` (the node rests on more than any record names, such as
  a leaf's language, toolchain, mathematics) or `below-floor` (nobody at this precision).

**Does the trust gradient coincide with the labour/land line?** Partly. The two lines meet at the
deep end. Language and mathematics have no maker anyone could cite, so no one can mint trust in an
attribution to a maker, and that value is common inheritance by both readings. They diverge in two
places:

- **Land is explainable.** Positional value has a cheap, verifiable reason: a count in a rollup.
  "Explained" therefore includes land, so explained ≠ labour.
- **Unexplained includes unrecorded labour.** A supplier the grocer forgot to record is someone's
  work. Calling it land would seize it, which is exactly what I.4 forbids.

So the commons holds unexplained value **in trust with descent**, as custodian and not owner. It
becomes common inheritance on the negotiated schedule I.4 names, if no explanation arrives.

**One incentive the invariant fixes.** In the first draft of the probe, a leaf kept everything, which
rewards declaring no reliances. Under the invariant, a leaf keeps its labour share, and its "rests
on" share goes to the commons as `no-record`. Leaving cites out no longer enriches the one who left
them out (test `test_omitting_cites_does_not_enrich_the_omitter`).

**Real-run reading.** Of $1,000:

- $974.73 is explained: $952.84 to makers' labour, plus positional shares.
- $25.27 is unexplained but placed: $24.04 `no-record`, $1.23 `below-floor`.
- Unplaced: $0. The account closes to exactly 1.

### A probe for knowledge works: plural model baseline

The operator asked whether an average of quality models could scaffold the global commons. One
reading: what several independent models reproduce without being told measures how far a claim has
diffused into the shared record. That would give knowledge works a sensor like the reverse-index
fan-in gives code. It survives only as a lens clause and a probe, never as the boundary.

- **The reproducibility test is easy to read backwards.** In I.4, *reproducible* marks labour, and
  the common inheritance is the *non*-reproducible, positional part (values-forward.md:127). A claim
  every model can regurgitate is not common because it is reproducible. It is common to the degree
  a community clustered around it. So model agreement is a reason whose subject is a community. It
  can raise a work's positional ratio, explained as "diffused, per lineages L1…Ln agreeing under
  rule R". It does not move the maker's labour share. Quicksort is everywhere, and it still had a
  maker.
- **A reading, never a verdict.** Compute confers no authority, and a more capable model earns the
  same audit, not more deference (memory `elohim-ceiling-is-not-a-seat-of-authority`). The baseline
  is a declared set of distinct lineages with an agreement rule, published as a lens anyone can
  contest or replace. An average of models is a monoculture and rebuilds the chokepoint (commons
  design, plurality condition; backlog row 38).
- **The scaffold is enclosed work.** The models were trained with their sources' names cut
  (manifesto.md:700). The baseline must be cited by model manifest, like
  `.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json`. Using it makes the
  re-citation debt more pressing. Its priority order is a lens choice, not a given:
  - a thin baseline marks novel produce whose maker is findable;
  - a thick baseline marks exactly where the covenant's debt is largest.
- **Bias inverts the intent.** Models over-represent some languages, cultures and eras. Knowledge
  common in an oral tradition or an under-digitized language would read as novel, and recognition
  would flow to whoever wrote it down. That is a new enclosure. Any lens using this clause must name
  its coverage gaps and default those claims to the community reason, not the transcriber.
- **Known is not true, and common is not worthless.** Agreement measures diffusion only.

Not run in this pass: no models were queried. The next probe is three lineages over a fixed claim
set from this repository's own docs, reporting per-claim agreement against the citations the docs
already carry.

## 7. Composition and bridging tiers

**Nesting.** A pool is a `Collective` (pool spec §2). A pool joins a wider pool by
`Membership{member_kind: Collective}` (qahal.rs:9-13, :37-48). A tier's pass upward is a clause in
its own charter lens: what it keeps for its facility's care, and its parent. That clause belongs to
the tier pool's representative council (a human council over the pool, proportionally
representative of its holon), not to the payer. The probe's tiers keep 2/3 and pass 1/3 (Rust ecosystem →
software → planetary). This is one illustrative lens; subsidiarity suggests most stays local.

**Bridging is reconciliation, and reconciliation mints the tier.** Two independently grown networks
reconcile by forming a higher layer between them. This is the operator's own answer on who writes
the constitution (noosphere-poc research note:22): pluralist floors under aggregated spaces. The
primitives exist:

- `CollabAgreement` joins collectives with a declared `commons_pool_tribute` (qahal.rs:52-63), which
  is the bridging tier's own pass to the commons.
- The valueflows bridge's ledger already records what a translation costs. `TranslationKind`
  (IdentityShape, FieldRename, SemanticBridge, Reconciliation, Sidecar) is at
  valueflows-types/src/lib.rs:60. `SemanticCost` (Mechanical, JustifiedDistinct, UnclearYet) is at
  :87, and `OntologicalCommitment` at :108.
- brit's composition spec classifies a cross-repository target as `kind: external` via a pinned
  parent `NodeSeed` and a consume check ("A.imports ⊆ B.exports", spec:84, :115).

**Reconciliation must preserve:**

- **Both explanations.** A bridge maps reason to reason and never overwrites one network's lens with
  the other's.
- **The unmapped part, known and placed.** Whatever cannot be mapped is carried with its
  `SemanticCost`, not erased.
- **The bridge's own value.** The labour of reconciling is attributable work. The position the
  bridge creates leans common by I.4.
- **Withdrawal.** Either side can withdraw and leave a trace, using the membership's withdrawal
  height.

The probe bridged `elohim/Cargo.lock` with `doorway/doorway-service/Cargo.lock`:

- 235 presences were relied on by both. 170 are at identical versions (IdentityShape). 65 have the
  same crate name at different versions, a Reconciliation the bridge must declare.
- 47 presences on one side and 148 on the other remain unreconciled, placed in their own network.
- $395 of one network's flow and $315 of the other's reach shared presences.

That overlap is the "vector cloud": a presence's vector is its recognition from each network. The
bridging tier is where those components are added under a declared mapping.

**Gradient.** First contact between two networks is trustless ceremony: every term glossed, every
mapping judged (the cross-repository review rule). A mapping judged and repeated becomes
trustful-earned and is carried, so a reconciliation is a trust act paid once at the bridge. Two
co-ops with different supplier codes, or two seed libraries with different accession schemes, fit
the same shape. Lot code to lot code is FieldRename; a landrace held under two names is
Reconciliation; a variety only one library holds stays placed in its own network.

## 8. Signals both ways

The recursive spec maps each holon's elohim to Systems 3, 4 and 5 (recursive spec §1.2). The same
reliance edges carry signals outward and inward. The reverse index is what lets inward signals be
routed without a global broadcast.

**Outward (edge → commons):** value and recognition along carried ratio tables; condition readings
rolling up as k-of-M witnessed rollups with `CoverageRollup` descent.

**Inward (commons → edge):**

- **Recognition notices** to makers along the reverse index (manifesto.md:716).
- **Retraction and correction.** These are pulled, not pushed, with eager push only for high-reach
  content (citation-apparatus backlog, observations 7–9). The probe retracted `unicode-ident`. It has
  202 dependents across reverse degrees 1–8 (3, 38, 52, 67, 19, 12, 11, 3), and the entry work is 3
  hops back. Pushed: 585 messages, now. Pulled: no messages, but 1,220 own-cite checks per window
  across all holons, and the farthest dependent learns after 8 windows. In this flow, $0.57 rested on
  it.
  - Price, pulled: trustful-declared · friction-verify (a content-address compare) · unit-call per
    window · lane-background.
  - Price, pushed: unit-peer, paid once.
  - A safety retraction at commons reach earns the push. An ordinary correction rides the pull.
- **Algedonic pain** to those who rely on a node: a load-bearing upstream whose maintainers are
  exhausted (the xz-utils shape), an unheld seat, a dead cite, an unresolved reason, a retraction not
  yet reached. Care and value can then flow back.
- **Floor and ceiling signals.** The commons declares its needs: the care floor and the
  donut-ceiling domains (`CoverageDomain`, recursion.rs:140-150).

**Pleasure is the primary pole: the account closing.** "All things are known" here means explanation
completeness. Every ratio has a resolvable reason, and the unexplained remainder is itself known and
placed. It is reachable even when most value dissolves to the commons. The pleasure is the account
closing, not every share reaching an individual.

- **Outward:** a holon feels its reliance fully cited ("found keeping the terms", manifesto.md:708).
- **Inward:** a maker's held presence lights up as recognition arrives.
- **Across a bridge:** two networks' accounts close through the declared mapping, read at the
  bridging holon.

What the person sees is quiet by default and intelligible on demand, never a dashboard (commons Q3).
A closed account shows as a mark beside the bibliography. A gap shows as one notice that names the
missing reason and offers to cite it.

**Proposed sensors, in the algedonic-designer's 4-tuple.** No new channel.

- `reliance-account-closed@1`
  - Bound: unplaced = 0 and dead reasons = 0.
  - Concern address: the holon's own lens adoption.
  - Reader: the holon's own elohim, which carries rung one (the notice). The person decides past it.
- `reliance-account-closed-across@1`
  - The same measure, read at a bridging holon over the declared mapping.

`algedonic::should_emit` has no production caller and no clearing predicate (guidestar spec:342-343).
These sensors therefore need hysteresis before any emits, which is node MN-10.

**Hard guard: known artifacts, never surveilled persons.** Completeness is per holon, at that
holon's own reach. It describes the provenance of works and flows, never knowledge about people.
Thoughts never enter a notarized plane (private-thought-governed-fruit.md:39). Activity ledgers are
held by the holon they describe, with no cross-holon join without each holon's own crossing act
(:40). An intimate flow can be fully known to its holon and unobservable from outside. No global view
of persons exists in this design, and the k≥5 rule bounds what a pool may count.

## 9. What biology suggests

The operator asked whether DNA, an encyclopedia shredded and reassembled, simplifies the model. The
analogy is kept about works and flows only. Heredity language never applies to persons, and the
protocol frames identity as imago dei, not lineage. Two parts help, one is refuted by measurement,
and the rest add vocabulary but no simplification.

- **Kept: inheritance as exact ratios.** Pedigree shares (1/2, 1/4, …) sum to 1, and each has a
  known explanation. This is the ratio-plus-explanation invariant, and it confirms exact rationals
  as the natural arithmetic. It misleads in one place: biological shares are expectations under
  random recombination, while ours are declared by an adopted lens. Nothing about value is
  inherited by chance.
- **Kept: a genome is a recipe, run locally.** The genome stores a generative rule, not the
  organism. A cell reads a morphogen gradient to know its role. That is the same shape as storing
  the ratio table and regenerating every split (class C), with a node reading its rollup to know its
  positional share. It removes nothing not already removed, but it is a plain way to explain why no
  per-split record is kept. It misleads in one place: cells adopt no rule, and here every rule is a
  person's choice.
- **Refuted here: an identical-ancestors point as the dissolve point.** In genealogy there is a
  depth past which everyone then alive is an ancestor of everyone now alive, or of no one (Rohde,
  Olson & Chang, "Modelling the recent common ancestry of all living humans", *Nature* 431:562–566,
  2004; cited from memory, not re-fetched in this pass). If reliance graphs mixed like that, the
  commons boundary would come from structure instead of a tuned floor. The probe (`--mixing`)
  tracks which of the 56 first-degree branches each node's value came through, and compares that
  against the background. Total-variation distance stays at 0.92–0.97 at every degree, and a
  typical node is reached through only 2–3 branches. **There is no mixing depth in this graph.** It
  is shallow (depth 7) and shaped like hubs and spokes, while genealogies double every generation
  across a finite, mixing population. The floor stays a declared choice. Whether a mixing depth
  appears across a whole ecosystem's inflows is a further probe.
- **Vocabulary only.** Shotgun assembly and alignment map onto reconciliation: overlaps are shared
  CIDs, and scored gaps are `SemanticCost`. Horizontal gene transfer maps onto `DERIVED_FROM`.
  Repair and homeostasis map onto correction and the closed account. Each is already in the design.

The analogy carries no settlement or valuation rule, and none is taken from it.

## 10. No bottleneck

**Cost reading** for the path from an inflow event to recognition at a presence:

- **Mechanism:** a per-split computation and message.
- **Relationship:** trustful-declared for code (cargo enforces the edge); trustless at the cash-out.
- **Unit conversion, measured:** per-path (67,076) → per-node-per-window (960 edges touched,
  303 nodes split).
- **Phase:** a lockfile release or a new lens version is a transition (re-judge the affected
  tables). Everything else is steady.
- **Lane:** background, except the payer's approval and a maker's claim.
- **Hot spot:** `proc-macro2`, fan-in 37. It received 11 messages in the floored window and 37
  without the floor. On crates.io the same node has far larger fan-in, so the receive side pulls:
  recognition is a fold on read, cached by `rollup_hash`.

**Hyperscaler parity, held in trust.** The seam map's crosswalk (§8) names `CoverageRollup` as the
analytics analog, and a `Collective` party as the shared-capacity analog. The **facility** (compute,
storage, freshness labour: rival) is held by a collective. The **artifacts** (index, rollups, ratio
tables: non-rival) are held by no one in the sense of alienation: nobody can sell them, and anyone
can rebuild them.

| Power needed | Hyperscaler | Held in trust over the p2p substrate | Today's gap |
|---|---|---|---|
| Global reverse index | one database | A rollup per pool keyed by presence, over members' lockfile summaries, federated tier to tier | no reliance `CoverageDomain`; `witness_quorum` 0 |
| Aggregation over millions of cites | map-reduce | Compute goes to the data (pool spec §5.4). Each member's elohim folds its own cites; aggregation hubs fold the summaries | Path 2 caller; rollup chaining (care-aggregation §5.3) |
| Availability | replicated managed store | Peer-hosters replicate summaries and rollup atoms by `replicates-content` to the pool | the pool as party (pool spec §1) |
| Low-latency queries | CDN | Plural doorways project rollup atoms to the web, cached forever by `rollup_hash` because they are immutable | a projection route; view federation moves rows, `MAX_PAYLOAD` 1 MiB (view_federation.rs:172; the seam map's 256 KiB is stale) |
| Correctness | trust the provider | k-of-M witnessed rollups that two peers recompute to the same hash | f64 magnitudes (MN-2) |

The last row is the inversion the seam map names: verification held by peers, where a hyperscaler
asks to be trusted.

## 11. Lens, not allocation

Every weight in the probe is one illustrative lens, labelled so in `lens.example.json`. Who decides
what:

| Act | Elohim | Person |
|---|---|---|
| Compute tables, show the account, draft a lens, flag a gap (rung one) | yes | sees it on demand |
| Adopt the lens for one's own outflows | proposes | **decides** |
| Lens for held presences | computes | the steward pool's **representative council** decides (Q8) |
| Tier pass clause | computes | the tier pool's **representative council** |
| Positional share | computes the rollup | **negotiated** between makers and the tier commons, and contestable |
| Settlement | executes within a standing mandate | the payer **approves** the mandate |
| Claim a presence | invites (manifesto.md:676) | the **makers** claim it |

Plural lenses coexist. Each table names its lens CID, so two payers using different lenses produce
different flows, and each one verifies. An aggregate declares its anchor set (cite summaries plus
lens CIDs) and its recompute path.

**Why citing is rewarded, not taxed.**

- **Trust makes it cheap.** A compiler-governed cite is free: cargo already enforces it. A
  judgement cite (`CiteSeal`) is paid once at the reach gate and carried.
- **The citer earns reach, never a cut of the flow.** A cut would create cite-farming, and money
  must not buy audience (succession.md:606).
- **The cited are invited.** Recognition accrues, and the attractor applies (manifesto.md:741).
- **The account closing is the felt reward.**

Equal splits invite one project to split into many crates. In the probe, the serde family took
$43.08 against serde's own $9.92. Grouping by presence, meaning by upstream project, is the fix
(MN-8).

## 12. The settlement edge

**Entry.** $1,000 enters through a bridge as an external inflow (`Inflow`, external.rs:311).
Internally, nothing moves but ratio tables and recognition, which are REA records, never tokens.

**Exit.** Money leaves only through a bridge, only to a channel the maker declared in their own
repository: a Drips `FUNDING.json`, sponsorship files. That declaration is the strong binding, and it
is needed only here.

**Shares to presences with no declared channel.** These stay with the payer as a standing commitment.
The money is unmoved and no treasury forms. Recognition keeps accruing indefinitely
(manifesto.md:741).

**Is Drips the right shape?** Drips streaming is the right *external* channel when a maker chose it.
Streaming is the wrong *internal* shape: per-second flows are per-event cost on a fungible rail. Even
Drips splits in batches, monthly or daily (economic-systems-research.md:46), which confirms windowed
settlement.

**brit's inherited `FUNDING.json`.** It is the ancestor's declaration (gitoxide's), not brit's. It is
right for the share explained by `DERIVED_FROM` gitoxide, and wrong for brit's own labour.

## 13. The grocer's month

A co-op grocer takes $1,000 in a month. Most of it settles the ordinary way, as `transfer`s against
invoices for rival goods (tomatoes, rent, wages), and no cite flow touches it. The attribution flow
covers only the non-rival slice the co-op chooses to honour under its adopted lens:

- the deli's recipe cards cite a cookbook and a supplier's method;
- the farm's produce cites its seed lines (an Open Source Seed Initiative variety, which cites its
  breeder);
- the breeder cites the landrace it selected from.

The landrace was made by generations of farmers no record names. Its "rests on" share is
`no-record`, and it is genuinely common inheritance. The shop's site value is George's land in the
literal sense: positional, with a community as its reason. A forgotten supplier shows as a gap in the
co-op's account, a notice and not a seizure.

Two co-ops reconciling into a regional network map lot codes reason to reason (§7). The records are
the same, ratios summing to 1 with reasons, so the design is not code-shaped. The reliance spec's
caution stands: practitioners have not validated this.

## 14. The simulation's real-run results

`genesis/research/commons-flow-sim/` holds `flow.py` (standard library only; it parses the lockfile
directly), `lens.example.json` and `test_flow.py`. There are 14 tests over a hand-made lockfile with
answers worked by hand in the test docstring, and all pass. Run:
`python3 genesis/research/commons-flow-sim/flow.py --bridge-with doorway/doorway-service/Cargo.lock
--inflows 12 --signal-from unicode-ident --mixing`. The exit code was 0, and JSON was written to `out/`.
Excerpt:

```
lockfile elohim/Cargo.lock: 491 packages; 28 workspace crates are the entry work; 463 external
crates relied on; 1220 reliance edges; deepest shortest-path degree 7
inflow 1,000.00 USD; entry keeps 1/2, upstream keeps 3/4, split=equal, positional cap 1/5
(half at 20 dependents), precision floor 0.01 USD
  deg reached       received kept by makers   positional    no-record  below-floor
    0       1     1,000.0000       500.0000       0.0000       0.0000       0.0000
    1      56       531.0781       384.7565      18.0694      16.6849       0.0000
    2     167        89.7848        64.5234       3.7536       6.6065       0.9082
    3      77         4.7701         3.5248       0.0704       0.7402       0.3194
    4       2         0.0428         0.0316       0.0006       0.0056       0.0050
top: serde 9.92, libc 8.17, pin-project-lite 7.88, tokio 7.82, bytes 7.59 (all degree 1)
smallest recipient: bit-set 0.8.0 keeps 0.007974164723 USD (exact 8788/1102059)
recipients 303 (incl. entry); reached nothing, every split toward them below the floor: 161
  rust-ecosystem-commons   received 47.163553  kept 31.442368  passed up 15.721184
  software-commons         received 15.721184  kept 10.480789  passed up  5.240395
  planetary-commons        received  5.240395  kept  5.240395
account: explained 0.974730 + unexplained but placed with descent 0.025270 + unplaced 0 = 1; closed
planetary-commons holds 5.240394738 USD = 0.005240395 of the inflow
nodes that split: 303  edges touched: 960 (sent 635, pooled below floor 325)
naive per-path push: 67,076 messages; most paths into one node: 20,906 (unicode-ident)
hot spot: proc-macro2, fan-in 37; messages this window: 11
ratio tables judged 303; 12 inflows reused them 3,333 times
largest amount denominator 31 digits; f64 would fail an exact sum-to-1 check 0 of 303 times
makers 952.836447354 + commons 47.163552646 = 1000 USD; exact match: True
```

**Without the floor** (`--floor 0`): all 463 crates receive something. The smallest share is
windows_aarch64_gnullvm's $8.0×10⁻⁷, which is 8×10⁻¹⁰ of the inflow. This is the operator's "1/billionth",
visible because the arithmetic is exact. The planetary commons then holds $5.13. With the floor, the
work drops from 1,220 edges to 960, and 161 crates are cut off.

**Honest readings.**

- The planetary commons receives about half a percent, not a billionth. Under this lens the tiny
  numbers belong to deep makers, and the commons' share is set by its tiers' pass clauses.
- The f64 risk was *not* shown by this lens: zero failed checks, and identical sums in four orders.
  Exact magnitudes are still argued for, on two grounds: a check that needs a tolerance is not a
  check, and a `rollup_hash` agreed by two peers needs exactness. The evidence for harm is absent.
- The lockfile over-approximates reliance. Windows-only crates received $1.51 on a Linux build. Dev
  dependencies count too.
- One lens is applied everywhere. In the design, each node's table is under its own steward's lens.

## 15. Missing nodes

- **MN-1.** chain value flow / between `FlowEvent{Cite}` → a settled amount / missing node: a ratio
  table record, keyed by (cite summary CID, lens CID, rollup CID), whose lines sum to exactly 1 and
  each name a resolvable reason and lens clause; probe: an epr-rea port of `verify_split` passes on a
  bridge-minted Process and refuses a doctored line / current state: no split record exists;
  `CollabAgreement.commons_pool_tribute` is an f64 compared within 1e-9 (qahal_coordinator.rs:828).
- **MN-2.** chain magnitudes / between `Magnitude::Count` → an agreed `rollup_hash` / missing node: an
  exact value magnitude (a rational, or integer minor units with a declared scale and the remainder
  placed) / probe: two peers fold the same tables in different orders and get one hash / current
  state: f64 in `Magnitude` (witness.rs:143), `GraduatedSummary.total_quantity`
  (summary_event.rs:30), and `ContributorPresence.recognition_score` (lib.rs:516). Harm is not shown
  by the probe.
- **MN-3.** chain recognition / between inflow events → a presence's accrued recognition / missing
  node: recognition as a fold (class C), not an incremented notarized field / probe: the displayed
  recognition equals a recompute from inflows plus lens CIDs, with no DHT write per inflow / current
  state: accumulating fields on the notarized `ContributorPresence` (lib.rs:511-519) would put hot
  presences on the head plane.
- **MN-4.** chain reverse index / between member lockfile summaries → "who relies on X" / missing
  node: a reliance `CoverageDomain` keyed by presence, carrying fan-in and participation (k of M) /
  probe: two peers compute the same `rollup_hash` from the same summaries / current state: five
  domains, none for reliance (recursion.rs:140-150); `witness_quorum` hard-coded 0 (:247).
- **MN-5.** chain nesting / between a pool → its parent pool / missing node: a tier pass clause in
  the charter lens and its REA event / probe: a pool's pass verifies as a ratio table with reasons /
  current state: `Membership(MemberKind::Collective)` exists; there is no charter-update path
  (commons-holonic backlog row 35).
- **MN-6.** chain bridging / between two pools on separate DHTs → a bridging tier / missing node: a
  rollup atom whose constituents span both DHTs, plus a reason-to-reason mapping ledger reusing
  `TranslationKind`/`SemanticCost` / probe: the elohim↔doorway shared-presence rollup (235
  presences; 65 needing reconciliation) is reproduced by peers of both, with the unmapped part
  placed / current state: `CollabAgreement` exists; view federation moves rows only.
- **MN-7.** chain settlement / between recognition at a presence → money out / missing node: a
  settlement commitment paid only to a channel declared in the maker's own repository / probe: the
  outflow bridge pays a `FUNDING.json` channel and records the transfer against the ratio line it
  discharges, and refuses an undeclared payee / current state: `Inflow.presence` is "Never a payee"
  (external.rs:312-313); no outflow bridge exists.
- **MN-8.** chain presence identity / between crate → upstream project / missing node: a presence
  keyed by repository URL, grouping its crates / probe: the serde family's $43.08 lands at one
  presence / current state: the lockfile names no project; reliance spec node 4 would carry it.
- **MN-9.** chain lens in force / between an unclaimed presence's outflow → its lens / missing node:
  a lens bound to held presences by the steward pool, replaced on claim / probe: verification names a
  lens CID per node, and that CID changes when the presence is claimed / current state: `author-lens`
  exists in mishpat; nothing binds a lens to a presence.
- **MN-10.** chain algedonic / between the measure `reliance-account-closed@1` → a notice to the
  holon's person / missing node: an emitter with hysteresis (emit and clear) reading per holon at its
  own reach / probe: a dead cite fires one notice, and repairing it clears that notice without
  re-firing / current state: `should_emit` (elohim/epr/src/algedonic.rs:381) has no production caller and no clearing predicate
  (guidestar spec:342-343).
- **MN-11.** chain inward signal / between a retraction at an upstream → its dependents / missing
  node: a reach-tiered rule, pull by default and push at commons reach / probe: the unicode-ident
  retraction reaches all 202 dependents within 8 windows when pulled, at once when pushed / current
  state: the citation-apparatus backlog states the rule; nothing implements it.

## 16. Open questions for the operator

1. **Human view.** Lead with the forward bibliography and the reverse "what your work meant", with
   degrees as a depth detail. Agree?
2. **The three reason subjects.** Maker = labour, community = land, nobody = held in trust with
   descent. Should the floor be declared as a cost choice and not as the boundary?
3. **Positional share.**
   - Who negotiates it: the relied-on project's makers and the tier commons?
   - Fan-in is headcount, not independence. Is it acceptable as the first sensor?
4. **Unclaimed money.** A payer-held commitment (recommended) or escrow at the bridge? After what
   horizon, if any, does unclaimed money graduate to the commons? Recognition waits indefinitely;
   money held forever is a liability.
5. **The planetary commons in money.** Should it have a declared channel, an in-kind route that
   raises the floor, or recognition only? Succession refuses a treasury.
6. **Ancestor channels.** Should brit's inherited `FUNDING.json` count for the gitoxide-derived
   share only?
7. **Reliance source.** Read reliance from the lockfile (declared, all platforms) or from the build's
   actual use (filtered to the target)?
8. **Push threshold.** Which reach earns an eagerly pushed retraction?
9. **Mint pass.** The research directory asks that each surviving node fold into a backlog cluster
   row. This pass edited no backlog. Which clusters should MN-1 to MN-11 ride? Candidates are
   commons-holonic stewardship and the valueflow fabric.

**Addendum, round 2.** The rival half (the hosting, compute and maintenance that peers commit to a
pool), scale numbers at three sizes, the vector cloud, biophilic principles, the operator's "for the
living" direction, and the support button designed from this probe are in
`genesis/research/commons-facility-at-scale-2026-10-15.md`. The simulation now lives at
`genesis/research/commons-flow-sim/`. Round 2 also revises §11's settlement row. Money crossing the network
boundary is a boundary act of the elohim council at the ceiling: plural, witnessed, overturnable by
the human sortition floor, and phased in. A member's own elohim still only drafts and carries (second
document §9, "Who decides at the boundary"). The first document's "`MAX_PAYLOAD` 256 KiB" is
corrected to 1 MiB (view_federation.rs:172).

## Outputs (mint pass, applied 2026-10-15)

Missing nodes MN-1 to MN-11 were folded into backlog clusters under `genesis/data/timeline/backlog/`: `dependency-graph-cited-contributor-economy` (observations 6 to 9: MN-1, MN-4, MN-7, MN-8), `measure-family-borrows-backlog` (row 39: MN-2), `arch-scale-risk-backlog` (row 11: MN-3), `commons-holonic-stewardship-backlog` (rows 39 to 41: MN-5, MN-6, MN-9), `2026-08-10-algedonic-phase2-network-phase3-dedupe` (row 8: MN-10), and `citation-apparatus-claim-grain-typed-edges-reach-gate` (observation 11: MN-11). The placement table is in `genesis/research/2026-10-15-covenant-commons-working-mou.md`, section "Mint pass".
