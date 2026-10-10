---
name: ostrom-commons-design
description: "Consultable design gate for anything that pools, shares or holds a resource in common — a commons pool, a shared index or model, a collective's charter, a sharing or stewardship feature. Walks the design through Elinor Ostrom's design principles in the eleven-part wording she adopted last (Cox et al. 2010) as functions to satisfy and declarations a charter must carry, never as mechanisms; adds the three conditions the principles presuppose and the places a digital commons bends. Use when 'design a commons pool', 'is this a commons or open access', 'score this collective/charter against Ostrom', 'sharing feature', 'what must the pool declare', or before the p2p-design-gate on any `commons-pool:<kind>` resource. NOT for data-entity classification (p2p-design-gate) or cost/trust reading (gradient-reading)."
metadata:
  runtime: antigravity
  sourceRuntime: elohim-agent
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/ostrom-commons-design.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/ostrom-commons-design"
---
# Ostrom Commons Design

A commons that lasts is held by a bounded community under rules it can change, watched by its own members, with consequences that start low and a cheap local place to settle disputes, recognized by the layer above it and nested in it. That is Elinor Ostrom's finding from the long-enduring commons, restated as eleven design principles (her Nobel lecture, 2009, adopting Cox, Arnold & Villamayor-Tomás 2010). This skill walks a design through them. It asks for **functions and declarations**, never mechanisms: Ostrom's own method found that "if there are 10 successful ways to monitor, the correlation between any one way and group performance will be weak" (Wilson, Ostrom & Cox 2013). Her footnote: "Perhaps I should have used the term 'best practices.'"

Load it into the session that holds the design: a pool, a shared index or model, a collective's charter, a sharing or stewardship feature. It is a reading method, not an instrument. It builds nothing and scores nothing.

- The source pass, with every claim graded and every instrument at `file:line`: `genesis/research/ostrom-design-principles-commons-governance-2026-10-10.md`.
- The design home for pools: `genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md` (a pool is a `Collective` that holds a `commons-pool:<kind>` resource and is party to its members' commitments).
- Canon it leans on: Stance IV.2 (justice is restored capability; "to not respect the limits is to limit your own reach"), governance-layers §Constitutional Councils (the council is drawn to the reach of the decision), `succession.md` §(b) (recognition is a success condition, not a compromise).
- Backlog rows the gaps already ride: `commons-holonic-stewardship-backlog` rows 32–37; `measure-family-borrows-backlog` row 34.

## When this fires

Before the design is proposed, and before `p2p-design-gate` prices its entities, whenever a design:

- creates or amends a pool, a `Collective` charter, or a `commons-pool:<kind>` resource;
- lets members contribute to, draw from, or carry something held in common (storage, compute, an index, a model, a tile archive, a treasury, a mutual-credit line);
- adds a sharing, pooling, stewardship or federation feature;
- names a group that will govern a resource at any reach.

If the thing has an owner who can sell it, this skill does not apply; it is private property with guests. If nobody can exclude anyone, it is open access and Hardin applies; this skill's first finding will be that Q6 fails.

## First: which commons is it?

A digital pool is two commons on one object (Hess & Ostrom 2007). Separate them before asking anything else:

| Part | Nature | Governed by |
|---|---|---|
| **Facility** — members' bytes, compute, bandwidth, and the labor of keeping the thing fresh | Rival, congestible, depletable per period: a true common-pool resource. In tree: `Rivalry::PartiallyRival`, `Excludability::ConditionallyExcludable`, `GoodsClassification::CommonPool` with `requires_consent_allocation()` (`elohim-storage/src/services/resource_nature.rs`) | The eleven questions below |
| **Artifacts and ideas** — the tiles, the index, the model, the records | Non-rival, enclosable, pollutable, and better the more they are used | The seven rights: which of *access, contribution, extraction, removal, management, exclusion* members hold, and the statement that *alienation* (selling or leasing management and exclusion) is held by no one |

Two consequences. An outsider who reads is not a free rider; "in fact, they enhance the quality of the resource by using it." And the subtractable resource is capacity and maintenance labor, so *provision* rules are about those, not about the artifacts.

## The eleven questions

Ask them of one pool at a time, in this order — the evidence weights congruence and proportionality first and nesting last. For each: the function the principle serves, the instrument in the tree as **one** way to serve it, what the pool's charter must declare, and the probe that would show the principle present. A question passes only when the declaration exists **and** the probe can run. A declaration with no probe is `unwired`, in the habit register's sense.

| Q | Principle | Function it serves | Instrument in tree (one way) | The pool declares | Probe |
|---|---|---|---|---|---|
| 1 | 2A congruence with local conditions | Rules fit this pool's people and resource, not a template | `author-lens` Commitment, role `floor`/`ceiling`, with a telos (`mishpat/src/commitments.rs`); per-holon ceiling | Its own lens: appropriation rule, provision rule, telos | The lens CID is in the charter and resolves |
| 2 | 2B proportionality | What a member may draw is proportional to what they provide | Holder-side carrying record (pool spec, gate decision 3); the constitutional band on giving (`COMMONS_MIN_FLOOR_PCT`) | The proportionality rule, and the window it is read over | Two members at 2:1 contribution and a 1:2 draw: the second draw is narrowed |
| 3 | 4B monitoring the resource | Members can see the condition of the thing they hold | `IndexMeasure`; recipe CID; unreadable count | One measure: fill, freshness, unreadables, replica independence | The measure renders for a member without a steward's help |
| 4 | 5 graduated sanctions | A first violation meets a notice; repetition meets more; return is always open | Stance IV.2; reach narrowing on standing (`reach_earning.rs`) | A ladder of at least three rungs, notice first, authored as a lens the members agreed to | A simulated second violation lands on rung two, never rung three |
| 5 | 4A monitoring users | Members can see who drew what | The holder sees a byte draw; a compute draw is a public record naming the requester; membership is public | Who can list draws, at what reach | A member lists the last N draws without a steward |
| 6 | 1A user boundaries | Who is in, who is out, how one leaves | `Membership` with `withdrawn_at_block_height`; the current-membership test (latest-state-wins) in `qahal_coordinator.rs` | Join rule (sponsor, waiting period), leave rule | A withdrawn member's draw is refused at the membership step |
| 7 | 1B resource boundaries | What the pool holds and what it does not | `commons-pool:<kind>` resource; recipe CID; arc or byte ranges | The holdings: kinds, ranges, recipes | A draw outside the declared holdings is refused naming the boundary |
| 8 | 3 collective choice | Members can change the rules | (none today — no charter-update path on `Collective`; row 35) | How a rule changes: who proposes, who decides, at what reach, by what closure artifact | A member-proposed lens change lands and two peers compute the same new charter head |
| 9 | 6 conflict resolution | A fast, cheap, local arena, and an appeal upward | `StewardshipAppeal` (zero callers); mishpat `Challenge`; Commons Co-Steward mediation | The arena, its reach, and where appeal goes | A challenge opens and closes inside the pool's reach, witnessed |
| 10 | 7 recognition of rights | The pool's rules bind at its reach and the parent holon says so | `CollabAgreement` with counter-attestation; the council drawn to reach; row 32 | Which parent recognizes the charter, and what it reserves | The parent's attestation of the charter resolves |
| 11 | 8 nesting and scale | The pool is small enough to talk, or it nests | The reach ladder; subsidiarity | Its reach, its member ceiling, and the pool above it | Member count ≤ declared ceiling, or a parent pool is named |

## Beneath the eleven

The principles explain "under what conditions trust and reciprocity can be built and maintained" (Cox et al. 2009). Three conditions they presuppose, from Ostrom's experiments, must hold or the eleven are decoration:

- **Communication is feasible with the full set of participants.** The substrate cannot supply this. A thousand-member pool fails it on day one. Q11 is its proxy: size to the condition, or nest.
- **Reputations are known.** Supplied: standing is a relational record and reach is earned over time.
- **Sanctioning is agreed by the participants themselves.** "When participants themselves agree to a sanctioning system they frequently do not need to use sanctions at a high volume"; imposed sanctions "may reduce cooperation." Q4's "the members agreed to" is load-bearing, and a council drawn to the reach of the decision is how the floor agrees.

## Reading the result

The principles are configural (Baggio et al. 2016): no one of them is necessary and sufficient; enduring commons averaged about 8.7 of 11 present, failures about 4.3; and the **absence of Q1, Q2, Q3 or Q4** (congruence, proportionality, resource monitoring, graduated sanctions) strongly predicts failure.

Report, in this shape and no other:

```
Pool: <name>  reach: <level>  parts: facility=<…> artifacts=<…>
Present (declaration + probe): Q6 Q7 …            n/11
Declared, unwired (no probe):   Q1 …
Absent:                         Q2 Q3 Q4 Q8 …
Failure-predicting absences:    Q2 Q4
Rights on artifacts held by members: access contribution extraction removal management exclusion
Alienation: held by no one  (or: NAMED — this is not a commons)
Gaps, in mintable shape: chain / between A→C / missing node B: <assertion + probe> / current state
```

Never a score. A bare 0–11 number is the blueprint critique come true; the count and the named gaps are the deliverable, and each gap is a cluster row (`commons-holonic-stewardship-backlog`) or a habit, not prose.

## Guards

- **A principle is not a mechanism.** Every "instrument in tree" above is one of Ostrom's ten ways. A design that satisfies the function another way passes. A design that copies the instrument and misses the function fails.
- **Not a blueprint.** "Each design principle stipulates a range of conditions that could satisfy it, rather than a particular rule." Do not demand numbers (member counts, penalty sizes) the charter has not chosen; demand that the charter choose them.
- **No sanction in code alone.** Rules embedded in code are ex-ante and cannot read the context of an offense; a sanction must "start very low." The ladder is a pool-authored lens; the first rung is a notice; a refusal is never the first rung.
- **Monitor appropriation, never persons.** Q3 and Q5 watch draws and the resource. A pool that watches its members has swapped Ostrom for surveillance, and a pool whose elohim holds the only monitoring and sanction tools has become the platform the open-data literature warns about.
- **No tokens.** A contribution record is an REA event with a declared reach, never a transferable unit. Token-weighted voting "creates incentives for collusion and vote-buying"; every token-incentivized commons the earlier survey examined failed.
- **"Owned by no one" means alienable by no one.** Hardin's error was presuming the commons "were owned by no one." Ostrom's commons are held by their members. Say which rights members hold; say alienation is held by nobody; do not say ownerless.
- **Expect to formalize as you grow.** Growing commons formalize their rule-change process in order to decentralize (Forte; Schweik & English; Rozas). Q8's closure artifact is not a concession to a chain; it is what every scaling commons built.
- **Small-N honesty.** Five of eleven are present for a pool in the tree today (Q6, Q1's grammar, Q5, Q10's external half, Q11). Do not let a design claim more than its probes can show.

## Companions

- `p2p-design-gate` prices the entities this skill asks the charter to declare; run it after, not instead.
- `gradient-reading` reads what a draw costs and who should hold a thing; Q11's holder set is its question 4.
- `valueflow-authoring` writes the commitments a contribution and a draw are made of.
