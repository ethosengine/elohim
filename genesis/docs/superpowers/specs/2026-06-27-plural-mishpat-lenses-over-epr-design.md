---
title: Plural Mishpat Lenses over an EPR — the deterministic lens-market governance primitive
id: plural-mishpat-lenses-over-epr-design
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: cartographer
graduation-trigger: decompose-complete OR superseded-by-implementation
topic: [governance, mishpat, epr, policy-plurality, cybernetics, limitarianism, lens-market, regime-drift, participation-tracks, deterministic-contract]
domain: D7
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-23-multi-collective-collaboration-epr-design.md
refines:
  - genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-stakes-mechanism-selector-design.md
  - genesis/docs/superpowers/specs/2026-06-09-coupling-delay-observed-governed-primitive-design.md
cites:
  - wisdom-layer-floor-ceiling-judgment-culminating-design | the VERTICAL judgment ladder (deterministic floor + judgment ceiling); this pattern is its HORIZONTAL complement, and reuses its deterministic-floor concept as the constitutional recursion-terminator | sha256:f5d694c382a76c1f | path: genesis/docs/superpowers/specs/2026-06-09-wisdom-layer-floor-ceiling-judgment-culminating-design.md
  - per-substrate-limitarian-governor-design | the BUILT justice hand (George/Robeyns concentration demurrage); affinity composes its concentration measure as a lens input | sha256:5d10a556e2ec7a14 | path: genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md
  - epr-meta-compose-gate | the many-rules-over-one-target + CID-as-authority + bad-rule override-count drift precedent the contention metric reuses | sha256:e1ce92d0374fdff1 | path: genesis/docs/superpowers/specs/2026-06-25-epr-meta-compose-gate-design.md
  - resilience-facings-select-fold-aggregate-design | the select-fold-aggregate facing machinery a lens observation-half IS (plural and win-win by construction, never collapses) | sha256:8f2136ecd8678e6c | path: genesis/docs/superpowers/specs/2026-06-19-resilience-facings-select-fold-aggregate-design.md
  - lens-complete-epr-resolution-four-leg-coupling-design | the four-leg coupling law = the EPR attachment mold a lens binds onto (A2 link on the EPR CID) | sha256:79f821217c1c8e11 | path: genesis/docs/superpowers/specs/2026-06-07-lens-complete-epr-resolution-four-leg-coupling-design.md
  - operational-weave-facing-lens-design | the VSM recursion (System 1-5) mapping the self-hosting which-election recursion reuses; sibling Beer surface | sha256:fc432fea065dca00 | path: genesis/docs/superpowers/specs/2026-06-19-operational-weave-facing-lens-design.md
  - rea-economic-facing-lens-design | the REA economic facing the bounty fulfillment EconomicEvent composes with (D9) | sha256:b83ead21be13bbaa | path: genesis/docs/superpowers/specs/2026-06-19-rea-economic-facing-lens-design.md
  - reach-projection-facing-lens-design | the collective-distributive lens over edge+reach relations; a justice-surface analog for distribution reading | sha256:d42aab6a7663c6d3 | path: genesis/docs/superpowers/specs/2026-06-19-reach-projection-facing-lens-design.md
  - deterministic-reach-archetype-floor-design | the deterministic-floor archetype the constitutional terminator mirrors (determinism-before-judgment) | sha256:a2ee1687a1759a0f | path: genesis/docs/superpowers/specs/2026-06-10-deterministic-reach-archetype-floor-design.md
  - vision-gap-limit-governor-stub | the dignity-facing self-declared ceiling (the INVERSE of limitarianism); a sibling governor, not the limitarianism home | sha256:14ea8f3e81cd87c8 | path: genesis/docs/superpowers/plans/2026-06-14-vision-gap-limit-governor-stub.md
  - stewardship-over-sovereignty | the identity-ontology floor: the apex authority is community-grounded stewardship, never crypto self-sovereign — the constitutional apex obeys this by construction | sha256:995eb2079924ea2e | path: genesis/docs/architecture/stewardship-over-sovereignty.md
  - cradle-to-grave-capability-gradient | graduated/mediated agency (children, wards, seniors) the election franchise and authority model must hold for | sha256:1a5b2f7e6433230f | path: genesis/docs/architecture/cradle-to-grave-capability-gradient.md
  - elohim-seam-map-concern-routing | the participation tracks T1-T4 the two-layer law (DHT notary floor vs big-data aggregate projection) maps directly onto | sha256:54b5809fb8e688d1 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - weave-epic-arc-design | sibling VSM/compute-contract arc; shares the compose-dont-fork near-zero-new-DHT-types discipline | sha256:69966fdcc15dd7ba | path: genesis/docs/superpowers/specs/2026-06-20-weave-epic-arc-design.md
  - elohim-ceiling-design | the justice-as-capability (Mishpat sees, not punishes) framing the lens-market enacts | sha256:24925a4c8e1d9420 | path: genesis/docs/superpowers/specs/2026-06-23-elohim-ceiling-design.md
maintainers: Matthew Dowell + Opus 4.8
created: 2026-06-27
---

# Plural Mishpat Lenses over an EPR

> **One line.** Many deterministic Mishpat policies ("lenses") attach horizontally to one EPR — each
> the valid sensemaking of a different collective or school of thought — observed plurally and win-win
> by construction, given teeth only where conflict forces an election, and *self-renewing*: rising
> contention deterministically opens a bounty for fresh lenses, so no single policy can ossify into
> systemic poison the way a 1970-right prescription became a 2000s-wrong one.

## 0. Status & scope

This is a **vision-deferred** D7 (collective coordination & governance) pattern: the roadmap ranks
network-scale collective governance *below* the single-household seed, so this spec **captures the full
pattern as canonical architecture** and specifies a **thin, household-provable, hash-neutral Wave-1
slice** that can ship now on `household-nodes`. The network-scale waves are decomposed but tagged
`@requires:` the capabilities they need, so they hold without false-failing while Wave-1 stays on the
plate.

It **composes** — it does not fork. ~75% of the machinery already exists as built or specced
primitives; the genuine net-new is two things only: (1) the **forward index** "all lenses governing
EPR X" + a telos-fitness selector over it, and (2) the **cross-surface regime-drift trigger** that
fuses the viability and justice surfaces. Everything else is assembly.

## 1. Motivation — why plurality, and why it is the anti-degradation engine

Two intellectual lineages name two different things and neither alone is enough:

- **Henry George / Adam Smith (and the rent-generalizers — Robeyns's limitarianism, Hudson's
  rent-from-position)** ask *what is **just***: a substance ontology of earned vs unearned return,
  distribution, and the threshold at which a holding-position becomes private taxing power. Normatively
  saturated; blind to whether the system it judges can survive or be steered.
- **Stafford Beer / cybernetics** asks *what stays **alive***: a relational ontology of feedback loops
  and viability — can the system regulate itself fast enough to absorb the variety coming at it?
  Viability-rich; normatively empty. A well-regulated extraction engine is viable *and* monstrous.

A governance substrate needs **both hands on the table**: the moralist to say what to steer toward, the
cyberneticist to say whether steering is even possible. The protocol already carries each hand as a
*separate* built primitive (the limitarian governor; the coupling-delay observed-governed primitive).
What it lacks is the joint.

The deeper failure this pattern targets is **generational regime-drift**: a policy that is the right
prescription in one regime (deregulation in a sclerotic 1970s economy) keeps being applied by inertia
long past its fit, until it becomes the systemic poison of the next regime (2000s concentration). No
single-policy mechanism — however well-chosen — escapes this, because the choosing happens once and the
world keeps moving. **Plurality is the escape.** If a Georgist lens, a Beerian lens, and a limitarian
lens all stay *live and valid* over the same resource, each true in its own regime, then:

- no lens is The Orthodoxy, so none can capture the surface;
- the *right* lens for the current regime rises by **earned affinity** (measured, not decreed);
- a lens that has exited its valid regime loses affinity *and* its mismatch shows up as rising
  **contention**, which deterministically **summons its successor** via a bounty.

The system renews itself without anyone declaring a winner. That self-renewal is the whole point.

## 2. The two-layer law (the load-bearing constraint)

> **The Holochain DHT is a notary, not a database. It scales at human level, so it is the integrity
> floor, never the data layer. Big-data aggregates live above it and *derive their integrity from* it.**

This maps directly onto the canonical **participation tracks** (seam-map concern-routing atlas):

| Track | Role here | What lives here | Scale |
|-------|-----------|-----------------|-------|
| **T1 — DHT notary floor** | integrity + authority | lens rules, EPR↔lens bindings, certified elections, bounty commitments, periodic attested ballot-tallies, the constitutional floor | human (~100s–1000s entries) |
| **T2 — substrate aggregate (libp2p/iroh)** | big-data folds | affinity across every context, contention spreads, cross-EPR/cross-collective regime analytics | horizontal |
| **T4 — doorway projection** | web read views | rendered lens-market views for browsers | read-mass |

**The integrity-derivation rule (this is the meaning of "the integrity DHT provides for"):** *every
T2/T4 aggregate MUST declare its T1 anchor set and a recompute-and-verify path.* The big-data layer is
**trustless** — any peer can re-fold it against the notarized floor and detect a lie. This is
`P1 — storage as reconciliation controller` generalized to aggregates: the DHT is the manifest; the
aggregate layer is the controller that eagerly reconciles to it. An aggregate that cannot be
re-derived from T1 anchors is a bug, not a feature.

## 3. The pattern — two orthogonal lenses

The design introduces two *new* lenses onto the governance surface (distinct from the **vertical**
floor-ceiling judgment ladder, which is already specced and is a single-axis collapse). These two are
**horizontal** and orthogonal:

1. **Mechanism lens — deterministic contracts firing on EPR signals.** A policy is *not* an LLM
   judgment call; it is a deterministic rule that fires when an EPR emits a signal. This is the
   Solidity "contract-at-an-address" intuition made elohim-native: the contract lives as a notarized
   `Mishpat::Commitment` bound to the EPR's **own CID**, and the EPR's own signal stream is the trigger
   — there is **no global VM and no global clock**. It is the `delegates-compute` / `DelayBreach` shape
   generalized to "policy fires on signal."

2. **Topology lens — horizontal one-to-many → plural valid narratives.** Many policies over the *same*
   EPR, side by side, each the sensemaking of a different collective or school of thought. This is the
   self-renewing engine of §1.

These compose: *many deterministic signal-fired contracts, coexisting horizontally, each carrying one
school's valid reading.*

### 3.1 What "win-win plural" means mechanically (the resolved fork)

Confirmed design intent is a **B+C hybrid**:

- **Observation is always plural (B).** Every collective sees the EPR through every lens; sensemaking
  never collapses. (This is *free* — see §6: a lens's observation half is a facing-fold, and folds do
  not collapse.)
- **Warm standby (C).** Non-active lenses remain authored and live; they re-activate when context
  shifts. Land-value-tax is high-affinity in a developing economy and low-affinity-but-warm in a
  US-platform regime where anti-trust is exercised more; when the regime turns, the warm lens rises
  again.
- **Teeth resolve only on hard conflict.** Where a lens must *gate a shared action* and collectives
  strongly disagree, a **Mishpat election** (ranked-choice / agreement voting / quadratic) resolves it.
  Authority is the *collective franchise* — never the EPR-holder alone, never crypto self-sovereignty.

## 4. The self-renewing loop (on the tracks)

```
   ┌────────────────────────────────────────────────────────────────────┐
   │  (1) AUTHOR        many lenses bind to one EPR CID            [T1]   │
   │  (2) OBSERVE       plural, win-win — facing-folds never collapse [T2]│
   │  (3) AFFINITY      context-relative usage weight (earned)     [T2]   │
   │  (4) ELECT         teeth only, on hard conflict (RCV/agreement)[T1]  │
   │  (5) CONTENTION    controversy-spread fold → signed breach    [T2→T1]│
   │  (6) BOUNTY        contention opens R&D pull-bounty for fresh  [T1]  │
   │                    lenses → shadow-evaluate → promote on evidence    │
   └──────────────────────────────── 6 → 1 ────────────────────────────┘
```

The anti-degradation telos becomes **structural**: a stale lens loses affinity (3) *and* its mismatch
raises contention (5), which summons its successor (6). George's hand rides affinity (the justice
reading); Beer's hand rides contention (the viability/regulation-strain reading); they jointly drive
renewal — the fusion neither lineage achieves alone.

## 5. Entity model — the P2P Design Gate output

Realized per **Approach 3** (a lens is split into two halves by its natural track) and sequenced
**1 → 3** (ship the hash-neutral subset first). **Hash-neutrality is the deployment gate:** new
coordinator *actions* on `Mishpat::Commitment` + new facing folds + B2 attestations hot-swap via
`update_coordinators` with **no DNA reinstall** (no agent re-key, provable on `household-nodes` now);
new integrity *entry/link types* move the DNA hash → operator-gated reinstall. The design stays
hash-neutral except for one isolated, deferrable risk (the forward-index link type).

| Entity | Class | Address | Track / Source of truth | Coordinator (reuse) | Projection | Hash-neutral? |
|--------|-------|---------|-------------------------|---------------------|------------|---------------|
| **Lens** (deterministic policy/contract) | A (Notarized) | **CID = `entry_hash`** (`bafyrei…` dag-cbor; rule+trigger+telos immutable; new rule = new CID, version-chained) | T1 / DHT | `mishpat::author_lens` (new `Commitment` action) | `lenses` (dht_anchor ✓) | ✅ |
| **Lens↔EPR binding** (forward index — *net-new*) | A2 (Derived/Link) | anchored on EPR `EntryHash`; tag `{role: floor\|ceiling\|lens, context-scope, school}` | T1 / DHT link | link in `author_lens`; reverse-index query "all lenses governing EPR X" | `epr_lens_bindings` (dht_anchor → parent EPR) | ✅ if reuses scope link; ⚠ new LinkType = hash move (isolated) |
| **Ballot / Exercise** | B2 (Agent-scoped + Attestation) | Agent-scoped composite `(agent_cid, lens_cid, context)` | private source-chain (raw) + DHT (tally attestation, reuse imagodei `Attestation`) | `mishpat::cast_ballot` → `certify_tally` | `ballots` (no anchor) + `ballot_tallies` (dht_anchor ✓) | ✅ |
| **Affinity** | C (Operational) | n/a (keyed `(lens_cid, context-scope)`) | T2 / SQLite fold; reconstruct = re-fold tallies+selections | facing fold | `lens_affinity` (no anchor) | ✅ |
| **Contention** | C (score) + signed signal (breach) | n/a / `SignalKind::ContentionBreach` (analog `DelayBreach`) | T2 fold → T1 signal | facing fold + signal emit | `epr_contention` (no anchor) | ✅ |
| **Election** | A (Notarized governance decision) | CID (certified outcome immutable) | T1 / DHT | `mishpat::convene_election` / `certify_election` (new `Commitment` action `ratifies-election`) | `elections` (dht_anchor ✓) | ✅ |
| **Bounty** (fresh-lens pull-bounty) | A (Notarized) | CID = `entry_hash` | T1 / DHT | `mishpat::open_bounty` (reuse `Mishpat::Commitment`, action `bounty-fresh-lens`; `bounded_by` the breach, `in_scope_of` the EPR); fulfilled by REA `EconomicEvent` | `lens_bounties` (dht_anchor ✓) | ✅ |
| **Constitutional floor** | (not a queryable entity) | — | T1 integrity-zome validation (DNA-walled) + seeded non-revocable lens-Commitments | integrity validation | — | floor *is* the hash |

**Anti-pattern checks passed:** CID = `entry_hash` not `action_hash` (gospel); no UUID primary keys;
entity-type reuse over creation (Mishpat headroom preserved); no granular ballots on the DHT (B2 keeps
raw private); cross-namespace identity resolves through canonical `agent_cid` (`AgentPeerBinding`),
never raw-compared against transport ids; **no `self-sovereign` apex tier** — authority is
collective-election (community-grounded), apex is imago-dei (see §7).

## 6. A lens has two halves (the Approach-3 split)

- **Sensemaking half = a registered facing-fold (T2).** Pure, diesel-free, read-only,
  `select → fold → aggregate` over one materialized EPR snapshot. **Plural and win-win by
  construction** — two scholars, one text, different hermeneutics; folds never collapse. Adding a
  Georgist and a Beerian lens to an EPR is registering two folds: **zero new DHT anything, fully
  hash-neutral.** This is the topology lens delivered almost entirely by the existing
  `elohim-facings` crate.
- **Teeth half = a deterministic `Commitment`-contract firing on EPR signals (T1).** Instantiated
  *only where binding is actually needed* (the rare gate-a-shared-action case). This is the mechanism
  lens.

The cost of the split: "a lens" is two coordinated objects (a fold + a contract), not one you can point
at. The benefit: plurality is the default and teeth are the exception — which is exactly the confirmed
B+C intent, and it leans on built code.

## 7. The recursion & the constitutional floor

The choice of *which election/measurement mechanism* fits a situation is **itself** a
many-lenses-over-one-surface decision — plural lenses over the mishpat surface. The pattern is
**self-hosting**: it governs its own meta-layer. This is not new machinery — it is **Beer's VSM
recursion** (System 1–5 repeating at every level), already mapped onto the protocol by the
operational-weave facing, and it is the same fractal as steward↔EPR.

Recursion must **terminate**, or it is infinite meta-elections and nothing resolves. Beer's answer is
System 5 — identity/closure, the level not up for operational re-negotiation. Realized as a **thin
constitutional floor** in the integrity zome (DNA-walled, hash-defining, not coordinator-overridable):

1. a default controversy metric (the agree/disagree **spread**, Reddit-style),
2. a default resolution (agreement / ranked-choice voting),
3. the **imago-dei apex** — community-over-individual authority and a dignity floor — that **no
   election can vote away**.

Everything *above* the floor is lens-plural and recursively governable; the floor is the fixed point
that makes the recursion well-founded. The apex satisfies the **identity-sovereignty ontology guard by
construction**: authority is community/institutional (`stewardship-over-sovereignty.md`,
`cradle-to-grave-capability-gradient.md`), never crypto "self-sovereign," and the model holds for
graduated/mediated agency (children, wards, seniors). *The apex content is the architect's to set;* the
thin-floor termination is approved.

## 8. Contention, deterministically

Contention is a **controversy** measure, not a net score — the **spread** between agree and disagree
(500↑/500↓ is maximally contended; 1000↑/0↓ is not), optionally enriched with **intensity** (quadratic
voting) or **preference structure** (ranked-choice). High volume + high balance = a contested regime.
The candidate signals (blendable; which to use is itself a recursively-selected method-lens terminating
at the floor default):

- **override / dissent count** — directly reusing the `.epr-meta` `bad-rule` precedent (a rule
  overridden too often *is* the drift signal);
- **election margin** — repeated *close* outcomes (no stable majority);
- **election frequency** — the same EPR forcing elections too often;
- **affinity volatility** — no lens holding a stable contextual lead.

The contention *score* is a T2 fold (Category C); the *threshold-crossing* emits a signed
`ContentionBreach` (analog to the built `DelayBreach`), and the bounty it spawns is notarized at T1, so
the bounty has a verifiable cause.

**The net-new fusion (the cross-surface trigger §0 names).** The regime-drift signal that opens a
bounty is *not contention alone* — it is the **joint** condition: an incumbent lens's **affinity
decaying** (the justice surface, §3) *while* **contention rises** (the viability/dissent surface).
Either alone is a false positive — a quiet backwater has low contention *and* low fitness; a healthy
live debate has high contention *but* a stable affinity leader. Their **conjunction** is the "this lens
has exited its valid regime" detector that no existing primitive carries: `DelayBreach` is
viability-only, the limitarian `valid_until` is a clock-only ("renewal defends against staleness, not
capture"). `ContentionBreach` is the *carrier* signal; its **firing predicate is the joint
affinity-decay ∧ contention-rise condition**. That conjunction is the George/Beer fusion in code, and it
is the genuinely new primitive this spec adds.

## 9. Elections & bounties — teeth and renewal

- **Election** (teeth): convened only on hard conflict over a gating action; ballots are B2 (private +
  attested tally); the certified outcome is a notarized governance decision (A). The method is a
  recursively-selected method-lens (RCV / agreement / quadratic), defaulting to the constitutional
  floor.
- **Bounty** (renewal): a `ContentionBreach` opens a **pull-bounty for fresh lenses** — the gospel REA
  compute-commitment primitive (`Mishpat::Commitment`, `bounded_by` the breach, `in_scope_of` the EPR).
  A candidate lens runs **shadow / observe-only** against live dynamics (its facing-fold scores are
  computed but its teeth do not fire), is compared to incumbents, and **promotes on evidence** —
  fulfilling the bounty with an REA `EconomicEvent`. This is the policy-as-experiment leg, composing
  cleanly with the shefa/REA economy (D9).

## 10. Design decisions & alternatives considered

- **Approach 1 — Commitment-action overlay (minimal, fully hash-neutral).** Everything via new
  `Commitment` actions + facing folds; forward-index reuses the scope link. Ships today; semantic
  stretch (a lens-rule wearing a Commitment costume).
- **Approach 2 — first-class `Lens` integrity entry type (clean; costs a DNA hash move).** Legible and
  native, but a DNA reinstall re-keys agents (operator-gated, prod-heavy) — can't prove on prod without
  migration.
- **Approach 3 — facing-plurality + signal-contract split (chosen).** Most faithful to the B+C intent
  (plural observation default, teeth exception), leans hardest on built code, stays hash-neutral.

**Decision: Approach 3, sequenced 1 → 3 → (2 only if ever needed).** Ship the hash-neutral core now;
treat the first-class `Lens` entry type as a deferred, operator-gated DNA wave taken *only* if semantic
clarity proves worth a reinstall. This matches the roadmap's vision-deferred placement.

## 11. Wave plan

- **Wave 1 — household-provable, hash-neutral (`@requires: household-nodes`).** One EPR; **≥2
  facing-lenses** (proving plural win-win observation); **one deterministic Commitment-contract**
  (proving teeth fire on an EPR signal); an **affinity fold + contention fold** with the
  integrity-recompute path (proving the two surfaces); a **manual election** path; a **bounty stub**.
  The entire loop proven at human scale, no DNA reinstall.
- **Wave 2 — the forward-index + telos-fitness selector (the net-new primitive).** Reverse-index "all
  lenses governing EPR X"; affinity-weighted contextual selection. Hash-neutral if scope-link reuse
  holds; a new LinkType (and only then a DNA wave) if forced.
- **Wave 3 — automated `ContentionBreach` → bounty → shadow-evaluate → promote** loop.
  (`@requires:` cross-node mesh for cross-collective contention — `alpha-cluster-6peer` / `shem`.)
- **Wave 4 — recursive method-lens selection** (which election mechanism), with constitutional-floor
  termination hardened in the integrity zome (operator-gated if it touches the DNA hash).

A uniformly-blocked wave is held by an `@requires:` tag on its gap-items; Wave 1 carries
`household-nodes` and stays on the plate (mixed-plan convention: no doc-level `requires_env`).

## 12. Degradation, security, testing

- **Fail-closed per lens (the `EprRouter`-empties lesson):** one poisoned/invalid lens degrades *its
  own row* in the market, never empties the whole lens set. Selection skips an unresolvable lens; it
  never aborts the EPR.
- **Gaming-resistance:** affinity and contention inputs are **attested tallies (B2)**, never raw
  self-reported counts; a Sybil inflating "exercises" cannot move affinity without notarized
  participation. Bounty fulfillment requires a *promoted* (elected/evidenced) lens, not a self-claimed
  one.
- **The big-data layer cannot lie:** every T2/T4 aggregate is recomputable against T1 anchors (§2);
  reconciliation catches divergence (`P1`).
- **Testing:** a2o scenarios under `genesis/a2o/features/qahal/` — plural-observation (two lenses, two
  valid readings, no collapse), election-on-conflict, contention→bounty; plus a sweettest on the
  deterministic Commitment-contract firing on an EPR signal (Mishpat zome). CI-green ≠ binding-correct:
  verify the household loop on a live `household-nodes` render.

## 13. Open questions

- **Forward-index link reuse vs new LinkType** — does an existing scope/`bounded_by` link cover
  EPR→Lens (hash-neutral), or is a new integrity LinkType required (the lone DNA-move risk)? Resolve in
  Wave-2 planning against the current Mishpat link inventory.
- **Default contention metric: constitutional or seeded-replaceable?** The floor fixes *a* default; is
  the spread metric itself un-electable, or a seeded lens that a collective may replace above the floor?
  (Leaning: spread is constitutional-default; intensity/preference enrichments are replaceable lenses.)
- **Positional-rent (Hudson) measurement** — `fold_edge_criticality` / `fold_net_diff` exist but are
  tagged resilience/reciprocity, and economic attribution is blocked while `AgentPeerBinding` is
  unsigned. A justice lens that reads rent-from-position over the EPR coupling graph is representable
  but needs that attribution unblocked — likely its own future lens, not Wave 1.

## 14. Relationships

- **refines** the wisdom-layer **stakes-mechanism-selector** (this is its third lens — generalizing
  selection from stakes → telos-fitness and adding plurality; it fills the selector's self-flagged
  "unargued value-laden coefficients" hole by making affinity *earned, not decreed*) and the
  **coupling-delay observed-governed primitive** (the viability hand + the `DelayBreach` trigger
  template).
- **cites** the per-substrate **limitarian governor** (the justice hand), the **facings family**
  (resilience / rea-economic / reach / operational-weave — the observation machinery + VSM recursion),
  the **`.epr-meta` compose-gate** (the many-rules-over-one-target + CID-as-authority + `bad-rule`
  drift precedent), the **lens-complete four-leg coupling** (the EPR attachment mold), the
  **deterministic reach archetype floor**, the **limit-governor stub**, the **stewardship-over-
  sovereignty** + **cradle-to-grave-capability-gradient** canon (the identity guard), the **seam-map
  concern-routing atlas** (participation tracks), the **Weave epic arc** (sibling VSM/compute-contract
  surface), and the **elohim-ceiling** spec (the justice-as-capability framing).
- **fills Gap-Ledger holes** (architecture/MAP.md §3): *chain-layer consensus mechanics* (the election
  franchise/authority) and *governance multi-factor merge check* (itself a multi-policy weighting).
