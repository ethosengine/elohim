---
name: gradient-reading
description: Use when ranking or explaining measured performance costs, when asking where trust should make a path cheap, when deciding how a new object should declare its reach, holders, freshness or custody, or when labelling backlog entries with the performance, trust, friction, custody, plane, unit, phase and lane tags. Reads a costly path through eight questions to separate cost that trust or separation can compress from cost that is owed unconditionally. Triggers - "read these costs through the gradient", "where is trust being paid for twice?", "who should hold this and how many?", "label the performance concerns", "is this a fused plane?".
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/gradient-reading"
---

# Gradient Reading

Trust prices compute. Verification is paid once, as close to the human judgment as possible, and
everything downstream relies on that minted trust instead of re-deriving it. Where that holds, cost
falls as trust rises. This skill is the method for finding where it does not hold yet, and for
saying how an object should declare itself so that it can.

Load it into the session that holds the context: the one that just profiled, or the one designing
the object. It is a reading method, not an instrument. It measures nothing.

- Definitions of every tag: `genesis/data/timeline/CONVENTIONS.md` §Performance, §Trust context and
  friction, §Custody shape, §Plane, §Cost unit, phase and lane. Reuse them; do not restate them.
- The map: `genesis/data/timeline/backlog/performance-concern-index.md`.
- Canon: `genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md`.

## The eight questions

Ask them of one path at a time: its two ends, and the thing someone was trying to do.

| # | Question | Tag family |
|---|---|---|
| 1 | What mechanism is the cost? | `perf-*` |
| 2 | Who stands at each end, by the relationship that already holds? | `trustful-self`, `trustful-declared`, `trustful-earned`, `trustless` |
| 3 | What kind of cost is it? | `friction-verify`, `friction-wait`, `friction-mechanical`, `friction-blind` |
| 4 | How widely is the thing held, and is breadth bought as headcount or independence? | `custody-everyone`, `custody-subset`, `custody-diverse` |
| 5 | Which plane's work is paying, and on which plane's path? | `plane-*`, `fused-planes` |
| 6 | What does the cost multiply by? | `unit-*` |
| 7 | When is it paid? | `phase-*` |
| 8 | Who is waiting? | `lane-*` |

## Reading the answers

| The answers | What it means | What to name |
|---|---|---|
| Trustful, and re-verifying or waiting | A candidate for compression: a carried proof in place of a re-resolution, one trust act per relationship in place of one per item | The fact being re-derived, and who already holds it |
| Mechanical friction, any relationship | Owed unconditionally: a livelock, a leak, a contention storm, an unbounded queue, a readiness probe that lies | The measured cost. Never route it to trust |
| Fused planes | A cheap plane is waiting on an expensive one. Bytes are large and self-certifying; meaning is small and takes the ceremony | Which plane pays, and whose path it is charged on |
| Held by everyone | Breadth is the cost | The smallest holder set that meets the object's resilience need |
| Cost grows with history, peers or items | A unit conversion is available: per item to per peer, per call to once, per history to a bounded range | The multiplier, and the path's own cadence |
| Transition phase | Stagger or warm, do not optimise | The state change that triggers it |
| Borrowed lane | Background work is standing where a person waits | What the person was waiting for, and what was in the lane |
| Trustless and verifying | The ceremony is where it belongs | Nothing. Leave it |

## Guards

- **Ceremony is compressed, never skipped.** A proposal that goes fast by weakening a signature,
  issuance, revocation, reach or witness check is the wrong shape. So is raising a deadline or a
  limit to make a number fit; a limit raise is a design signal.
- **A label is not a verdict.** It names a relationship or a mechanism. It never means measured,
  confirmed, cured or accepted. A high-cost edge is not a judgment of the peer on it.
- **Every dataplane path is trust-blind today.** The trust handshake classes every sync edge
  `public`. Label the relationship that holds, and say when a proposal depends on a signal the floor
  cannot read yet. In genesis, trust is declared: make the declaration explicit and watch the floor
  respect it. Never park work waiting for earned trust.
- **Resilience is independent failure domains, not headcount.** The threshold (k of n) is a separate
  dial from the membership: any one holder sufficing to recover is any one sufficing to take over.
  Independence has to be observed, not assumed.
- **Separation has a price.** Confidential discovery, aggregate counts, unlinkable payment and
  custody by assignment each add friction on purpose. State it beside the gain. Privacy is qualified
  by the witnessed-harm limit.
- **Check what the entry rules out.** Several entries say in their own words that a lever was not
  the cause (arc factor, for one). Say so, so nobody chases it.
- **No new register.** Tags live on the source. The index is a dated snapshot of the tag query. A
  principle with no home goes into the matching cluster as one row.

## Three uses

### Reading measured costs

You have profiled or ranked costs and want to know which to pursue.

1. Keep the measured ranking as it is. This skill does not re-rank by cost.
2. For each path, state the two ends and answer the questions that change the cure: relationship,
   friction, plane, unit.
3. Split the list in two: compressible (trustful and verifying or waiting; fused planes; a unit
   conversion) and owed (mechanical). Rank each by the measured cost alone.
4. For each compressible path, write the one question a further measurement should settle, and the
   compression it would justify if the answer goes one way.

Acceptance stays with the habit's own check. Copy figures from the source; never derive one.

The `runtime-performance` skill is the measuring half. It answers where the time goes; this skill
answers whether that cost should exist on that path. Use them as a loop: choose the path and write
the question here, capture and report there, then read the result here. When its report shows a
hot leaf function with no caller attribution, the gradient question says which caller to look for;
it does not replace the missing evidence.

### Declaring an object at design time

Run this beside the `p2p-design-gate` skill, which owns entity classification. For the object, state
plane by plane:

- **Reach** of its bytes, and separately of each reference to it. An intimate reference to commons
  bytes is two reaches, not one.
- **Custody**: the resilience it needs, the holder set that meets it at the least carrying cost, the
  threshold, and how independence of the holders would be observed.
- **Freshness**: how stale a read may be and at what stakes; whether the readable head serves or the
  notarized one is required.
- **Linkability**: what holding, serving or viewing it reveals about the one who does.
- **Cost bearer**: who pays to carry it and who benefits.

Use fields the protocol already has. Where none exists, report a missing node between named atoms
(`chain / between A→C / missing node B: assertion + probe / current state`). Do not invent a schema.

### Labelling sources

For a handful of entries, tag them yourself: read the whole entry, answer the eight questions, write
the tags into the frontmatter and preserve the file's format. Use only tags the content supports;
leave a question unanswered before guessing. A source with no frontmatter is indexed by reference.

For a bulk pass, dispatch a general-purpose reader on a cheaper tier with three things: the list of
sources, this skill's path, and which questions to answer. Have it return one row per source as data
and leave the repository untouched; apply the tags yourself, check that every frontmatter still
parses, and regenerate the index tables from the tags. Ask it how deep it read. A shallow pass is a
first reading: re-read anything you rank near the top.
