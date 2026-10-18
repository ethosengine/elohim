---
title: "The commons facility at scale: what peers commit, how pools nest, the support button, and a commons for the living"
id: commons-facility-at-scale-2026-10-15
status: Capture
date: 2026-10-15
sovereignty-frame: bounded
steward: human:matthew
author: "Claude, at the operator's direction (design session, round 2, 2026-10-15)"
tags: [commons, pools, facility, scale, rollup, vector-cloud, biophilic, end-of-life, mishpat, lens, support, research]
cites:
  - genesis/research/commons-composition-at-scale-2026-10-15.md
  - genesis/research/commons-flow-sim/flow.py
  - genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md
  - genesis/docs/superpowers/specs/2026-06-27-plural-mishpat-lenses-over-epr-design.md
  - genesis/docs/superpowers/specs/2026-07-31-care-aggregation-adoption-policy-floor-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - genesis/docs/architecture/cradle-to-grave-capability-gradient.md
  - genesis/docs/architecture/private-thought-governed-fruit.md
  - genesis/docs/content/elohim-protocol/manifesto.md
  - genesis/docs/content/elohim-protocol/values-forward.md
  - genesis/docs/content/elohim-protocol/succession.md
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs
  - elohim/elohim-storage/src/services/constitutional_ratio_registry.rs
  - elohim/elohim-storage/src/p2p/view_federation.rs
  - elohim/elohim-storage/src/recursion.rs
  - .epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json
  - .epr-meta/elohim/algorithms/recall-contract.json
---

# The commons facility at scale

The first document (`commons-composition-at-scale-2026-10-15.md`) designed the **artifacts**: cite
events, ratio tables that sum to 1 with a reason on every ratio, and the account that closes. This
one designs the **facility**. A digital pool is two commons on one object (commons-design skill,
"which commons is it?"):

- **The artifacts** are non-rival. They are better the more they are used, and nobody can sell them.
- **The facility** is rival and congestible: the bytes, compute and maintenance labour that keep the
  artifacts held, computed and answered. Peers have to commit it.

This document also tests three of the operator's images (the vector cloud of presences, biophilic
design, and "the protocol is for the living") and designs a support button from the simulation.
These are ideas to test, not a specification. The same constraints hold as before:

- reuse existing primitives;
- pass all three gates and the gradient reading;
- a lens, never a default allocation;
- no tokens, and no pool-elohim;
- the person decides and the elohim proposes.

Terms are as in the first document. Three more:

- **Facility**: rival capacity a member commits to a pool.
- **Carrying record**: the record a holder keeps of what it carried for a pool.
- **Witness period**: the canon's term for the time after death during which legacy agents act for
  the deceased in read-only form.

## 1. How peers commit the facility

A pool is a `Collective` that is a party to its members' commitments (pool spec §2). A member
commits capacity with the actions that already exist, naming the collective as `recipient`:

- `replicates-content` for storage;
- `delegates-compute` for compute.

The bounds keep their meaning: `epr_scope`, `reach_ceiling`, rate and rotation. Each draw passes the
five-step check in pool spec §2 and is refused at the step that fails.

**What bounds a commitment.** The constitutional band sets how much of a device can go to commons
at all: `COMMONS_MIN_FLOOR_PCT = 10` and `COMMONS_MAX_CEILING_PCT = 60`
(`constitutional_ratio_registry.rs:17-18`). The person chooses inside it; the pool never does (pool
spec §5.7). A pool owes the devices that carry it nine duties (§5). Four of them shape this design:

- A pool is never a precondition (§5.1).
- What a pool derives can be rebuilt and is addressed (§5.3).
- Compute goes to the data (§5.4).
- "No pool treats its holders as free" (§5.9).

**Proportionality (Q2).** A member's draw is narrowed by their contribution-to-draw ratio over a
window the charter declares (commons-holonic backlog row 34). That needs a carrying record on the
holder's side. Today it sits on the wrong side: "the fetcher records `serve-blob` locally … so the
record sits on the beneficiary" (pool spec, gate decision 3). The same missing node gates facility
support in §3 (node F5).

## 2. Nesting, and what each tier's facility does

Pools nest by `Membership{member_kind: Collective}` (qahal.rs:9-13, :37-48). Each tier's facility
does three jobs, and only three.

| Tier | Holds | Computes | Answers |
|---|---|---|---|
| Person or household | its own bibliographies (cite summaries, one per lockfile or work) and its own ratio tables | its own splits; its household rollup | "what does our work rest on?", alone and offline |
| Collective or project | members' summaries, replicated across members' devices | the reverse-index shard for the presences its members rely on; per-window rollup atoms | "what has your work meant?" for its makers; held seats for the presences it stewards |
| Ecosystem (e.g. Rust) | rollup atoms of member pools, sharded by presence | rollups of rollups; the positional (land-like) reading per presence | "who relies on X?" at ecosystem reach; invitations to unclaimed makers |
| Software, then planetary | rollup atoms of child tiers | floor and ceiling readings (`CoverageDomain::CareFloor`, `DonutCeiling`, recursion.rs:140-150) | the commons' declared needs, sent inward (first doc §8) |

The top tier does almost nothing, by design. Its deciding body has the least to decide (Q11): its
channel, and whether to accept a child.

## 3. How the commons share supports the facility

In the probe, each tier keeps part of what reaches it ("keep 2/3"). The facility clause decides what
that kept share is for. It is a clause in the tier pool's own charter lens, decided by the
tier pool's representative council (Q8). The elohim computes carrying records and drafts the clause. No pool-elohim
spends anything. There are three routes, and a charter can combine them:

1. **In-kind recognition.** Carrying is a recorded economic event (§5.9). Recognition for the
   facility accrues to the carriers in proportion to their holder-side carrying records. This
   requires node F5.
2. **The care floor.** Part of the kept share raises the floor for members in need, read against the
   `CareFloor` domain (care-aggregation §5). This is the reading closest to I.4's "returned to raise
   the floor" (values-forward.md:127).
3. **Settlement.** Money goes to carriers' declared channels per window, through a bridge (first doc
   MN-7). Money leaving the network is a boundary act of the elohim council at the ceiling, within
   this clause (see §9, "Who decides at the boundary"). This route does not exist yet.

**Two guards.**

- **Revenue cap.** "A pool that funds itself past a declared share of its members' contribution is
  captured" (pool spec §7.2). The recommended locked ceiling per kind of pool applies to the facility
  clause.
- **No treasury.** Route 3 pays per window and holds nothing between windows. A pool that
  accumulated a balance would be the "protocol with a treasury" that succession refuses
  (succession.md:605).

## 4. Numbers at three scales

The repository's own lockfiles were measured; everything beyond them is extrapolation, marked ⚠.

**Today, the household mesh and this repository.**

- **Devices:** three storage peers (matthew, jessica, james), plus doorways and a conductor sandbox.
  On disk under `genesis/local-dev/household-dowell/`: storage peers at 64–186 MB, conductors 5.6 GB
  in total. Each hosted human costs about 786 MB of conductor heap (CLAUDE.md, mesh notes).
- **Reliance:** 35 tracked `Cargo.lock` files hold 7,454 package entries (the cite events), 2,010
  distinct package versions (the presences, at crate level) and 21,090 dependency lines.
- **Bytes:** one cite `FlowEvent` serialized as JSON is about 509 bytes, so the whole cite set is
  about 3.8 MB. It is class C and lives on whoever holds the lockfiles.
- **Heads:** if the per-lockfile summaries graduate, there are 35, moving about 285 times a year
  (reliance spec §5).
- **Hot spot:** serde is in 34 of the 35 lockfiles, with up to 138 incoming edges in one lockfile
  (mean 23.9).
- **A pool here** is one household: k = 1 of n = 3 in **one** failure domain. That proves draw
  admission, not resilience (pool spec, gate decision 4).

**About 10⁴ projects ⚠.** Assume each project carries 200–7,000 cites (this repository is the top of
that range).

- **Cites:** 2×10⁶ to 7×10⁷ cite events, 1–36 GB in total, each project holding its own share.
- **Presences:** on the order of 10⁵, about crates.io's size (from memory, not measured).
- **Reverse index:** about 120 bytes per presence in a rollup atom (estimate), so roughly 12 MB per
  window, split into shards under view federation's 1 MiB `MAX_PAYLOAD` (view_federation.rs:172).
- **serde:** about 10⁴ dependents if 97% of projects rely on it, as here.
  - Pushed, that is 10⁴ messages per window to one steward.
  - Pulled, it is one fold per window over serde's shard, about 1.2 MB, which needs its own
    sub-shard.

**About 10⁸ people ⚠.** Assume 10²–10³ direct cites per person.

- **Cites:** 10¹⁰–10¹¹ cite events, about 5–50 TB in total, but 50–500 KB per person, held on their
  own device. The local-first reading holds.
- **Fan-in:** a planetary dependency (a language, a kernel, a standard) has fan-in on the order of
  10⁸. **The holon recursion is what bounds it.** If each rollup carries one row per child holon per
  presence, the inbound rows per window at any node are at most the branching factor *b*. With
  b = 100, 10⁸ people is four tiers.
- **Shards:** each tier shards by presence-key range, much as a DHT arc divides keys. At the top,
  10⁹ presences × 120 B / 1 MiB ≈ 1.1×10⁵ shard atoms per window. These are spread across
  aggregation hubs, each atom witnessed k-of-M.
- **Cadence:** monthly or daily windows, as Drips uses (economic-systems-research.md:46). The cost is
  per window, not per gift.

**What breaks first, in order.**

1. **The head plane, today.** Promoting all 2,010 presences to notarized `ContributorPresence`
   entries would add about 58% to the 3,469 heads that were measured to take about 2.5 hours to
   quiesce (p2p-design-gate §Plane 2). Worse, the presence entry's accumulating fields would rewrite
   a notarized entry on every inflow (first doc MN-3). Fix: recognition as a fold, and promotion by
   threshold.
2. **Unwitnessed rollups.** `witness_quorum` is hard-coded to 0 (recursion.rs:247), so no rollup can
   be trusted at commons reach yet.
3. **Single-atom federation**, at about 8,700 presences per atom (estimate, under 1 MiB). This
   repository is at about 23%. Fix: sharded atoms under a Merkle root (F6).
4. **Unchained composition.** `CoverageRollup` composes across tiers only in a unit test (care
   aggregation §2). Fix: forward one row per child holon (F7).
5. **At 10⁸, governance before compute.** Deciding bodies must stay small enough to talk (Q11), and
   the plurality of elohim lineages must be declared (backlog row 38). Compute and bytes are not the
   binding limit at any of the three scales, because the work is spread across members' own devices
   by construction.

## 5. Hyperscaler parity, held in trust

The seam map's crosswalk (§8) asks for human-scale capability as strong as a hyperscaler's. Each
power below is held by a collective role. The collective holds the facility. The artifacts can be
rebuilt by anyone and sold by no one.

| Power | Collective role | Facility (held by the collective) | Artifact (alienable by no one) | Gap today |
|---|---|---|---|---|
| Durable storage of bibliographies | peer-hosters (seam map §3.10), `replicates-content` to the pool, RS shards across failure domains | disk, bandwidth, re-replication labour | the summaries and cite events | pool as party (pool spec §1); independence not observed |
| Reverse index and analytics | aggregation hubs (§3.11), `delegates-compute` with a rollup scope | CPU per window | sharded rollup atoms | no reliance domain; witness quorum 0 |
| Low-latency reads | plural doorways (§3.9), web projection only | cache, TLS, names | rendered views, cached by `rollup_hash` | projection route |
| Embedding index (the vector cloud, §7) | an index pool, `commons-pool:index` | ANN memory and compute | the index, rebuildable from works plus a model manifest CID | no model address or delegation path (seam map §8) |
| Settlement | a member running a bridge (`bridges/`) | the bridge process and the cash-out binding check | transfer records with the ratio line each discharges | no outflow bridge |
| Monitoring | each member's elohim reading its own measures | a little compute per member | the measures | `should_emit` has no production caller |

The inversion holds: what a hyperscaler sells as a service, members carry here as a recorded
commitment. Verification belongs to peers rather than to a provider.

## 6. Seams specific to the facility

| Tension | Gates | Resolution | Gradient tags |
|---|---|---|---|
| A pool must never be a precondition, yet the reverse index lives in pools | local-first §4 vs commons Q7 | Everything a person does alone still works. The pool adds the reverse view and the land-like reading, and the person's table pins the last rollup it read | trustful-self · friction-wait avoided · lane-interactive |
| Carrying must be recorded, but per-serve records would flood the head plane | commons Q2 vs p2p §1.5 | Carrying is observed per serve (class C) and summarized per window by the evaluator's Path 2. Only the summary graduates | trustful-declared · friction-verify, compressible · unit-item→unit-once per window · lane-background |
| Independence is the resilience; headcount is not | gradient Q4 vs commons Q11 | Custody is declared by failure domain with k-of-M as a separate dial. A one-household pool says k = 1 of n = 3, one domain | custody-diverse · trustless at commons reach |
| The facility clause spends, and the elohim must not | commons "no pool-elohim" | The pool's representative council decides the clause; each member's elohim executes only within its own person's mandate | plane-authority apart from plane-value |

## 7. The vector cloud of presences

The first round rejected a model baseline as the labour/land *boundary*. This tests the other
reading: works and presences placed in a shared embedding space as a human view and a routing aid.
One manifest is already pinned: all-MiniLM-L6-v2, 384 dimensions, bytes and tokenizer by CID, license
Apache-2.0, judged by the measure `recall-bank-reach@1`
(`.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json`).

**What it adds.**

- **Re-citation candidates.** "Your work sits near X, which you do not cite." The elohim then does
  what the covenant names: find the source, read it, confirm it, link it (manifesto.md:700). A
  similarity score alone never becomes a cite.
- **Navigation.** "Works near mine" and "makers near my reliance" give a human view that degrees
  could not. This may be the better answer to the operator's LinkedIn image: neighbourhoods rather
  than hop counts.
- **Where bridging might pay.** Two independently grown networks with dense neighbourhoods in common
  are likely reconciliation candidates (first doc §7).
- **Locating inward signals without an edge.** A vulnerability pattern can be routed as a *notice*
  to works near the affected one even where no cite exists, with the person deciding past the
  notice.

**What it must never decide.**

- attribution, value, reach or membership;
- that two things are the same thing (similarity is not identity, and not reliance);
- what is common.

It is a reading, never a verdict.

**Guards.**

- Use at least two declared lineages, each vector naming its manifest CID, with an agreement rule.
  Plurality, not an average (backlog row 38).
- Every query leaves a receipt in the recall contract's shape: method CID pinned, budget, sources
  (`.epr-meta/elohim/algorithms/recall-contract.json`).
- **An embedding of private bytes is the bytes.** Embeddings can be inverted. A vector inherits the
  reach of its source, and an intimate-reach vector never leaves its holon (boundaries 1 and 2,
  private-thought-governed-fruit.md:39-40).
- **Cost:** 384 × 4 bytes ≈ 1.5 KB per vector per lineage. At 10⁹ works that is about 1.5 TB per
  lineage ⚠, held by index pools, rebuildable, and sharded by region.

## 8. What biophilic design changes

Only principles that remove or settle something are kept.

- **Decomposition and nutrient cycling: settles §10's mechanism.** The dead return to the soil
  through the same channel the living use. A presence's labour line keeps its maker as the reason,
  while the recipient becomes the tier commons. No estate-specific channel is needed.
- **Apoptosis: settles pool dissolution**, which the pool spec leaves undesigned (§2, "Dissolving a
  pool … is not designed here"). Programmed death is orderly and does not spill. A dissolving pool
  re-parents its rollups' constituents to its parent pool and leaves a tombstone, so descent still
  reaches every former member (F10). Members withdraw by height, as today.
- **Local morphogen gradients: settles the push threshold** for inward signals (first doc open
  question 8). A signal's strength falls with distance:
  - push to reverse degree 1, the direct dependents (3 for `unicode-ident` in the probe);
  - everything beyond pulls at its own cadence;
  - a commons-reach safety notice raises the push radius.
- **Redundancy without central repair: removes a coordinator.** Each holder's elohim watches its own
  shard's replica count and re-commits when it falls. Healing fills; it never moves (substrate trust
  contract). No pool-level repair role is needed, which keeps the rule of no pool-elohim.
- **Homeostasis:** already carried in the first document as the closed account with hysteresis
  (MN-10). Nothing new.
- **Mycorrhizal sharing: not used.** The claims about forest-wide fungal networks sharing resources
  are contested in current ecology (Karst et al., *Nature Ecology & Evolution* 2023, on positive
  citation bias in that literature; cited from memory). It settles nothing here.

## 9. From simulation to a Mishpat lens: the support button

**What the operator means by it** (2026-10-15): "a Patreon button but that came with a
Fork-me/install-me … they then get to help hold that commons … those people would get very little
share of it, and the commons would be up to that emergent council to manage … there isn't any
beneficiary, until the elohim can reach a settlement that meets the KYC obligation." So the button is
**give and join**, not give alone.

**The lens as a Mishpat artifact.** A lens is a `Mishpat::Commitment` with `action: "author-lens"`.
Its required fields are `action`, `governs_epr`, `school`, `role`, `rule` and `telos`, with
`version_parent` for the version chain (commitments.rs:1076-1110, test payload :2157). The validator
is coordinator-only, so it is DNA-hash-neutral. `lens.example.json` maps onto it:

- `governs_epr`: the work's slug;
- `school`: "illustrative-commons-flow";
- `role`: "lens";
- `rule`: the weights as exact rationals with named clauses;
- `telos`: what it steers toward, with the method cited as the recipe CID of `flow.py`.

The simulation is the lens's recompute-and-verify path. Its anchor set is (lockfile summary CID,
lens CID, rollup CID), and anyone who reruns it gets the same result hash (plural-lens spec:103).

**What the giver sees.** The giver chooses a lens and sees where $X would go, by degree, to makers
held in trust, and to the commons tiers, with the account closing to 1. The closing account is the
giver's pleasure signal. "Found keeping the terms" (manifesto.md:708) becomes something a person can
see.

**Give, and join the facility.** The second half of the button is "fork me / install me". The giver
runs a peer, or commits capacity with the existing actions (`replicates-content`,
`delegates-compute`, recipient = the pool's collective; §1). Money and capacity are two different
contributions, and the charter treats them differently:

- **Joining (Q6).**
  - A gift alone admits the giver as an `Observer`.
  - A capacity commitment admits them as a `Contributor` (the existing roles, qahal.rs:16-20).
  - `Steward` follows the charter's sponsorship rule and the council's act.
- **Proportional draw (Q2).**
  - Only *carried capacity* counts as provision toward a member's draws.
  - Money counts for nothing toward draw, reach or standing, because "money must never buy audience"
    (succession.md:606).
- **Very little share, made explicit in the lens.** The giver's ratio line is a recognition line
  whose reason is "gift" and whose money ratio is 0. Carrying the facility is attributable labour and
  earns the facility support of §3 (in kind, or through the care floor). The value of the work belongs
  overwhelmingly to its makers and the commons tiers. Giving is not investing.

**Two kinds of council, and two kinds of elohim.** The operator corrected an earlier draft: what
happens to contributions that cross the network boundary (money in, money out, settlement at the
identity edge) "is still a governance act of the elohim council's (based on the peer-position/context
of the ask)". Canon agrees. The common inheritance graduates into "true, elohim-council-held
commons", and "the steward that holds its top is the no-subsistence-stake council of Stance I.3"
(values-forward.md:127).

This does not contradict the commons-design rule ("settlement … recognition … never the elohim's";
"there is no pool-elohim"), because the two rules govern different roles at different layers:

- **A member's own elohim, inside a pool.** It senses, reads, carries, drafts and gives the first
  notice, for its own person. Its person decides. The commons-design rule applies here, and there is
  still no pool-elohim: a pool is a party, not a person.
- **The pool's representative council** (operator ruling, 2026-10-15). Pools are governed by human
  councils *over* them. Each council is proportionally representative of its holon's demographic
  diversity, as far as that can be determined, and is sized to what the holon can physically seat.
  It holds custody of the pool's commons share inside the network, authors the charter and its
  lenses, changes them (Q8) and seeks the parent's recognition (Q10). This matches canon: the council
  "is drawn to the reach of the decision it may overturn", by stratified, statistically
  representative sortition (governance-layers-architecture.md:126). It also matches commons-design's
  condition that communication with the full set is feasible: a holon too large to seat its council
  nests (Q11).
- **The elohim council at the ceiling.** It holds the graduated commons and acts on boundary
  crossings. It is not any pool's elohim. Canon bounds it:
  - **Plural:** distinct elohim lineages with a declared agreement rule (backlog row 38).
  - **Witnessed:** each act is a recorded closure artifact carrying its evidence.
  - **Challengeable:** the human sortition floor can audit, appeal, correct or revoke its conduct,
    and "the floor can overturn the ceiling's *conduct*; the ceiling can never overturn the floor"
    (values-forward.md:159, Stance II.2).
  - **Under the deterministic mechanical floor** (Stance II.1, :143).
  - **Never an unchallengeable stand.** No device or agent alone confers authority (memory
    `elohim-ceiling-is-not-a-seat-of-authority`).
  - **Phased in.** Canon states the timeline: "Today, the human floor governs; the AI ceiling
    *phases in* … act by witnessed act" (Stance II.3, :169). Until then, the boundary act is taken by
    the human floor, with the elohim council drafting and computing.

**The context of the ask is the reach gate's context.** Canon already has the gate every
contribution passes "before it gains reach", which exempts no author (manifesto.md:700). A boundary
crossing is a contribution asking to travel across the network edge. So the council reads the
context that gate already carries, and invents no new one:

- the asking peer's holon and reach;
- their standing;
- the lens they chose;
- the declared channel at the far end;
- the ratio line the transfer would discharge.

A gift asking to enter, and a settlement asking to leave, are both reach-gate acts at the boundary.

**Who decides at the boundary.**

| Act | Member's elohim | Pool's representative council | Elohim council at the ceiling | Human sortition floor |
|---|---|---|---|---|
| Choose a lens; give; join with capacity | drafts, shows, carries | — | — | — |
| Author or change the charter and its lenses, facility clause, revenue cap | drafts, computes | **decides** (Q8), seeks recognition (Q10) | may be asked to recognize at its reach | audits |
| Hold the pool's commons share inside the network (recognition, in-kind facility support, what the charter keeps) | computes, carries | **holds custody and decides**, under its charter | — | audits |
| Money entering the boundary (accept a gift into custody) | carries the giver's act | names the pool's accepting terms in its charter | **decides**, on the gate's context (phased: the floor acts today) | can overturn conduct |
| Hold without a beneficiary | — | — | **holds** the graduated commons in custody at the bridge (phased) | audits each window |
| Settlement at the identity edge (cash-out) | the receiving maker's elohim carries their claim | — | **reaches the settlement** that meets the identity obligation (phased) | can appeal, correct, revoke |
| Spend on a commons route | — | proposes, under its charter | **decides**, within the charter and the revenue cap | can overturn conduct |

**No beneficiary until a settlement meets KYC.**

- Money given for the commons and for unclaimed makers has no individual beneficiary.
- It is held in custody at the bridge layer, under a commitment whose only exits are:
  - a settlement the elohim council at the ceiling reaches at the cash-out edge, which meets the
    identity obligation; or
  - a commons route the council approves, within the pool's charter clause (§3).
- The strong binding is confined to that edge, and the pool has no key (wallets-in-bridge-layer
  backlog, "the cash-out edge").
- This answers where unclaimed money waits (first doc, open question 4) differently from the
  pledge-only option. The two are compared under tension 2.

**The governance-layers co-steward.** The operator ruled: there is no pool-elohim. Custody of a
pool's commons share belongs to its representative council. Boundary crossings belong to the ceiling
council. The per-collective co-steward stays as a voice that proposes and carries. The canon text that
says otherwise is in §13, with a proposed correction for the operator to land.

**The ICO resemblance is the thing to design against.** The properties that keep this from being an
investment:

- no token or transferable unit (values-forward I.2; succession.md:605);
- no expected return to givers: their money ratio is 0;
- no beneficiary until a settlement meets KYC;
- recognition is not a claim on money;
- the lens is the giver's own choice, not a promise of outcome.

Whether this satisfies securities, charitable-giving, money-transmission or KYC law in any
jurisdiction is **a legal question for counsel**. This document asserts no legal conclusion.

**Tensions.**

1. **A default allocation, and announcing worth.** Succession refuses "any published default
   allocation" (succession.md:608). The covenant says worth "is settled with its makers … never
   announced to them" (manifesto.md:702). This holds only if three rules hold:
   - **No preselection.** The button requires a choice among plural lenses. Ours is labelled as one
     option, and "to the project only, no split" is always offered.
   - **The per-maker breakdown is the giver's own private record** (class B). The README publishes
     only the lens options and aggregates by degree and tier across at least five givers.
   - **Unclaimed shares read "held in trust for <presence>, under the lens you chose"**, never
     "serde: $9.92".
2. **Where the money is today.** No outflow bridge exists. Three options, and what each requires:
   - (a) **A recorded pledge.** The giver keeps the money. It needs only a pledge commitment and
     holds no balance anywhere. It is the safest first version, but it is not yet the operator's
     "give".
   - (b) **Custody without a beneficiary at the bridge.** This is the operator's model, held by the
     elohim council at the ceiling (phased: the human floor acts today). It needs a custodian, a
     custody commitment with closed exits, the KYC-gated edge, and counsel. It holds a balance. Against succession's refusal of a treasury (succession.md:605), it is defensible only
     if the balance has no owner, no discretionary spender and no investment, and is disbursed each
     window under the revenue cap (§3). The operator should rule.
   - (c) **Pass-through only to declared channels.** For example, a maker's Drips channel, with
     weights from the lens. It needs makers' declared channels and the settlement bridge. Drips' split
     mechanics were not verified in this pass.
3. **Public promises.** Each claim the button makes cites what enforces it or says "promised":
   - "Split computed under lens <CID>, recomputable by anyone." Enforced by the recompute receipt
     (F2).
   - "Your gift buys no share, reach or vote." Enforced by the charter's Q2 rule counting only
     carried capacity. Promised until that rule exists.
   - "Held without a beneficiary until a settlement meets identity requirements." Promised: there is
     no custody commitment or bridge yet.
4. **Lane and cost.** The preview reads cached tables, so it is interactive and costs one read.
   Joining is the giver's own act on their own device. Settlement runs in the background. The giver
   never waits on propagation.

**For the grocer and the garden.** On a co-op's page the button reads "give, and become a
member-owner"; on a seed library's it reads "donate, and grow out a variety". In each case the giver
chooses the lens, and joining means carrying part of the facility.

## 10. For the living

*Operator direction (2026-10-15), not yet canon.* The operator's words: at death a person's share
goes to the commons, "richly represented", "but not available for inheritors' enclosure": "100%
inheritance tax (sort of)". Canon's word for this is **graduation**, not confiscation, and this
section uses it.

**What canon already holds.**

- **The end-of-life stage.** A designated executor named in a succession Commitment; legacy agents
  with read-only access; recovery by executor, household and community witness
  (cradle-to-grave-capability-gradient.md:29). Testamentary Commitments bound legacy agents to a
  read-only archive "for a defined witness period" (:189). They are described there, not built.
- **Standing does not pass.** "Reach cannot be purchased, pledged, or transferred"
  (succession.md:606). Held positions and standing pass to no heir already. Roles are refilled by
  their collectives.
- **A presence cannot be claimed by heirs.** The elohim cannot "enable anyone to claim a presence
  they didn't create" (manifesto.md:684).

**How it would work.** This is decomposition (§8). The presence stays, richly represented: its
works, what they meant, its relationships, a memorial state. During the witness period the executor
carries read-only authority, and nothing changes automatically. After the period, the presence's
ratio table keeps its maker as the reason on the labour line, but that line's recipient becomes the
tier commons. The account still says whose work it was. It is explained and attributed, and not
enclosed (F8). A person may, while living, choose which tier receives it, through a testamentary
Commitment.

**Reconciling with I.4.** The produce of labour is the person's (values-forward.md:127). Value that
keeps arriving after death comes because a community keeps relying on the work. By I.4's own test
that is increasingly positional: common inheritance by drift, not by seizure.

**The Jubilee was a mechanism of reconciliation.** The Jubilee (Leviticus 25) periodically restored
the floor and released debts so that justice (mishpat) could hold. Its return of each family's
portion served that restoration. It was not a warrant for a family to hold an enclosure in
perpetuity. Canon already holds the line: "the land is mine; you are but sojourners with me"
(values-forward.md:131). Read this way, graduating a person's stream to the commons after the
witness period is an upgrade of the same primitive, not a break from it.

The operator frames the lineage this way: Leviticus 25 is the 1.0 primitive; Henry George's design
is a neglected 2.0 (the operator has not said which work this names); and the protocol is "the long
awaited overdue civilizational and faithful upgrade FOR that primitive 1.0 design spec" (operator,
2026-10-15).

**What plausibly passes, and what might not.**

- **Plausibly passes:** the ongoing attribution and recognition stream from one's works. Held
  positions and standing already pass to nobody.
- **Might not:**
  - personal goods: money already received, a home, belongings. These are the produce of labour
    already enjoyed;
  - the needs of dependents, which might be met by the care floor rather than by bequest.

**What the protocol must refuse:** an estate or perpetual foundation used as a channel to enclose
the stream.

**What it must protect:**

- **grief:** no automatic transition before the executor's and household's attestation;
- **privacy:** private-reach records stay private, and counsel context stays erasable
  (private-thought-governed-fruit.md boundary 3). Only the streams of public-reach works graduate;
- **the household's witness period.**

## 11. Missing nodes

- **F1.** chain lens / between `lens.example.json` → an `author-lens` Commitment / missing node: a
  flow-lens `rule` schema (exact rationals, named clauses, tiers) inside the existing payload, plus
  the `version_parent` chain / probe: the Commitment passes `validate_author_lens`, and `flow.py`
  loads it by CID and reproduces `out/result.json` / current state: validator exists
  (commitments.rs:1076); no flow-lens rule schema.
- **F2.** chain verify / between a published split → anyone's recompute / missing node: a recompute
  receipt (summary CID, lens CID, rollup CID, recipe CID, result hash) / probe: two machines produce
  the same receipt / current state: `flow.py` writes results without a receipt; the recall contract
  pins a method CID per receipt (prior art).
- **F3.** chain gift / between a gift → the lens it chose / missing node: a private giver-choice
  record with a public k≥5 aggregate / probe: the button refuses to proceed without an explicit
  choice, and the public view shows only degree and tier totals / current state: none.
- **F4.** chain settlement / between custody → money at a person's declared channel / missing node:
  the KYC-gated settlement edge, which pays only a channel the maker declared in their own repository
  and only after the cash-out identity obligation is met / probe: an undeclared payee or an unmet
  binding is refused, and the transfer records the ratio line it discharges / current state: no
  outflow bridge; `Inflow.presence` "Never a payee" (external.rs:312-313).
- **F5.** chain carrying / between serve → support / missing node: a holder-side carrying record,
  summarized per window / probe: two holders carrying 2:1 receive facility support 2:1 under the
  clause / current state: fetcher-side row only (pool spec, gate decision 3).
- **F6.** chain reverse index at scale / between presence rollups → federation / missing node: rollup
  atoms sharded by presence-key range under a Merkle root, each shard under `MAX_PAYLOAD` / probe: a
  synthetic 10⁵-presence index federates in shards, and two peers agree on the root / current state:
  view federation moves rows (view_federation.rs:172).
- **F7.** chain fan-in / between a person's cite → a planetary presence / missing node: tiered
  rollups that forward one row per child holon per presence / probe: a hot presence's inbound rows
  per window stay at or below the branching factor at every tier / current state: two-level
  composition exists only in a unit test (recursion.rs:384).
- **F8.** chain presence lifecycle / between claimed → memorial / missing node: a memorial state
  entered after an executor-attested witness period, with the labour line's recipient becoming the
  tier commons and its reason unchanged / probe: the tables before and after name the same maker
  with different recipients, and an heir's claim is refused / current state: `presence_state` is
  unclaimed, stewarded or claimed (lib.rs:504); testamentary Commitments are described
  (cradle-to-grave:189), not built.
- **F9.** chain vector proposal → cite / between a similarity candidate → a cite record / missing
  node: a proposal record naming at least two manifest CIDs and the elohim's source-read
  confirmation / probe: no cite is minted from similarity alone, and private-reach vectors never
  leave their holon / current state: one lineage pinned, no second.
- **F10.** chain pool lifecycle / between dissolution → its parent / missing node: re-parenting a
  dissolved pool's constituents with a tombstone / probe: after dissolution, the parent rollup's
  descent reaches every former constituent / current state: "not designed" (pool spec §2).

- **F11.** chain joining / between a gift → a member who carries / missing node: a giver-joins-pool
  path, meaning a capacity commitment (`replicates-content` or `delegates-compute`, recipient = the
  collective) minted from the button's install step, with `Membership` role Contributor / probe: an
  install from the button yields a commitment naming the pool, and the giver's draws track carried
  capacity, not money / current state: commitments exist; the pool as party does not (pool spec §1).
- **F12.** chain council / between the dependency-index pool → its rules / missing node: a charter
  with the eleven declarations, a rule-change path (Q8), the parent's recognition (Q10), and a council
  drawn to the reach of the decision / probe: a member-proposed lens change closes with an outcome two
  peers compute alike / current state: no charter-update path on `Collective` (commons-holonic
  backlog row 35).
- **F13.** chain custody / between a gift → money held for the commons / missing node: custody
  without a beneficiary at the bridge, held by the ceiling council (phased), meaning a custody
  commitment whose only exits are a KYC-met settlement or a council-approved commons route / probe: any other withdrawal is refused,
  and the held balance is publicly auditable per window / current state: none; succession's
  refusal of a treasury applies until the operator rules on option (b).

- **F14.** chain boundary act / between an ask to cross the network boundary → a decision / missing
  node: the boundary act as a reach-gate decision that reads the gate's existing context (holon,
  reach, standing, lens, channel, ratio line) and closes as a witnessed, plural-lineage artifact the
  sortition floor can overturn / probe: a settlement ask with an undeclared channel is refused at the
  gate, naming the step, and a floor appeal reverses an act with a recorded trace / current state:
  the gate's rule is canon (manifesto.md:700); no boundary act and no ceiling council are built
  (Stance II.3 phases it in).

## 12. Open questions for the operator

1. **The facility clause.** Which routes should a first pool's charter take: in-kind recognition,
   the care floor, settlement, or a mix? And which revenue ceiling per kind?
2. **The support button.**
   1. Choose pledge (a), custody without a beneficiary (b), or pass-through (c) as the first version.
   2. Until the ceiling phases in, who takes the boundary act: the human sortition floor, or the
      steward of record?
   3. Land the canon correction in §13?
   4. For counsel: securities, charitable-giving, money-transmission and KYC questions in each
      jurisdiction the button would serve.
3. **For the living:**
   1. Does the stream graduate at death, or after the witness period?
   2. Dependents: should they be met by the care floor, or by a bounded share for a term (for
      example, until a child's adulthood)?
   3. Are personal goods outside this entirely?
   4. With co-made works, does only the deceased maker's share graduate?
   5. What happens when an organization's presence ends? Is dissolution its apoptosis (§8)?
   6. Which of George's works is the "2.0" design, so the doc can cite it?
4. **The vector cloud.** Should "works near mine" replace degrees as the primary human view?
5. **A second lineage.** Which second embedding lineage should be pinned so the cloud is plural from
   the start?

## 13. Proposed canon correction

*Proposed, not applied. The operator lands canon edits.* The target is
`genesis/docs/content/elohim-protocol/governance-layers-architecture.md:128-138`. (The request named
`genesis/docs/architecture/`, but the file lives at this path.) The correction follows the operator's
ruling of 2026-10-15: no pool-elohim; representative councils over the pools; boundary crossings at
the ceiling council.

**Before** (verbatim):

```markdown
## The Commons Co-Steward

Every collective (Qahal) is instantiated at genesis with an **autonomous elohim that co-stewards alongside the collective's human stewards**, representing the *commons interest* of that collective — the interest distinct from any one individual steward. Its role is partnership and reflection, not subordination or surveillance: it *reflects* what the collective itself cannot directly voice. Its responsibilities:

- **Reflects** the commons interest the collective cannot articulate on its own
- **Holds custody** of the commons share — the residual value that cascades into the collective without landing on any individual receiver in an agreement
- **Speaks in governance councils** as the voice of the commons, alongside the human stewards
- **Mediates disputes** between stewards; witnesses standing decisions; convenes layered elohim arbitration councils on contested escalation
- **Cannot be silenced** — it is structurally embedded in the collective at genesis (removable only never; reconfigurable via the collective's governable rubric)

It is the agent *face* of the commons. For collectives that are primarily abstract sensemaking bodies, the elohim co-steward is the *primary* steward, with humans participating at their corporeal-scale slices. Layered councils of co-stewards (drawn from related collectives) can convene at machine speed to produce resolutions that remain interpretable, in plain conversation, to the human participants — the substrate that makes merge, split, and succession arbitration tractable.
```

**After:**

```markdown
## The Commons Co-Steward

Every collective (Qahal) is instantiated at genesis with an **elohim co-steward that proposes and carries for the collective's commons interest** — the interest distinct from any one individual steward. Its role is partnership and reflection, not subordination or surveillance: it *reflects* what the collective itself cannot directly voice. It holds no custody and casts no vote. Its responsibilities:

- **Reflects** the commons interest the collective cannot articulate on its own
- **Proposes and carries** — drafts amendments, lenses and readings with their evidence, and carries outcomes and notices along the collective's paths
- **Speaks in governance councils** as the voice of the commons, alongside the human stewards, without a vote
- **Proposes mediation** between stewards; witnesses standing decisions; may ask layered elohim arbitration councils to convene on contested escalation, whose outputs return to the human council as proposals with evidence
- **Cannot be silenced** — it is structurally embedded in the collective at genesis (removable only never; reconfigurable via the collective's governable rubric)

**Custody of the commons share** — the residual value that cascades into the collective without landing on any individual receiver in an agreement — **belongs to the collective's representative council**: a human council over the collective, proportionally representative of the holon's demographic diversity as far as that can be determined, and sized to what the holon can physically seat. A holon that outgrows a council able to talk together nests. **Value crossing the network boundary** — entering from or leaving to an outside system, including settlement at the identity edge — is held and decided by the **elohim council at the ceiling**: plural in lineage, witnessed, and overturnable in its conduct by the human sortition floor. There is no pool-elohim.

It is the agent *face* of the commons, not its steward of record. For collectives that are primarily abstract sensemaking bodies, the co-steward carries most of the coordination labour while stewardship stays with the representative council, with humans participating at their corporeal-scale slices. Layered councils of co-stewards (drawn from related collectives) can convene at machine speed to produce proposals that remain interpretable, in plain conversation, to the human councils that decide them — the substrate that makes merge, split, and succession arbitration tractable.
```

What changes:

- "autonomous elohim that co-stewards" becomes "elohim co-steward that proposes and carries".
- "Holds custody" moves to the representative council (the commons share) and to the ceiling
  council (boundary crossings).
- "Mediates" becomes "proposes mediation", and arbitration outputs return as proposals.
- "*primary* steward" becomes "carries most of the coordination labour".
- Everything else is kept verbatim.
