---
title: "Nachalah — the Allotment Epic: every record on the DHT is held by the peers its tier deserves, arcs are earned allotments read from the trust gradient, and the conductor that enforces this is our own artifact shipped over p2p"
id: nachalah-allotment-epic
brand: Nachalah
name: Allotment
status: Active
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
serves:
  - happ-lineage-migration
  - runtime-upgrade-propagation
  - dataplane-convergence
graduation-trigger: graduated Draft→Active 2026-09-05 on the operator's acceptance of §2's tier policy (gold / deeds / paper, relative per level), §2b's fractal, §2c's floor/ceiling/trust sizing and §3's allotment posture (arcs are earned, evidence-backed commitments dynamically negotiated by the elohim; recorded in §7.1). Graduates Active→Canonical when (a) the household mesh runs three DNA seams with three different floors and a `@concern:nachalah-allotment` receipt shows a paper-tier record never leaving its household while a gold-tier record reaches every hub, (b) our 0.7 conductor line runs with sharding on and an arc policy hook driven by the valueflow trust gradient, measured on the mesh, and (c) a conductor binary propagates to alpha through rung 5 with no Jenkins act after the build
created: 2026-09-05
domain: D2
topic: [dht, storage-arc, sharding, allotment, trust-gradient, tiering, reach, dna-seam, block-governance, conductor-fork, conductor-artifact, rung-5, ci-hygiene]
boundary: "Companion to the Holochain Evolution Epic. That epic owns the CROSSING (a hApp version change carried by the network, notarizations intact). This epic owns the HOLDING — who is asked to hold, validate and serve a record, at what floor, decided by what evidence — and the conductor as an artifact of our own. Where a crossing needs a holding rule (a closed chain must be written by nobody, Task 32 there) the rule is minted THERE and cited HERE; where a holding rule needs a crossing (an arc policy changes under running peers) the crossing is minted HERE and rehearsed with THEIR vehicle."
informed-by:
  - genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md (the companion — §4 sunset posture, §11.4 2026-09-05 entries: the sunset partitioned the household; Tasks 30–32)
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md (the seam atlas; the four participation tracks; the inversion)
  - genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md (rung 5 — artifact classes, channel, verify, vehicles; this epic adds the sixth class)
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md (verify-locally-then-serve; the probes that name a red)
  - elohim/holochain/conductor-image (the conductor pipeline that already builds and pins a fork; today wired to the che-devworkspaces submodule, outside the webhook)
cites:
  - "holochain-evolution-epic | Holochain Evolution Epic | sha256:2c06f0a9579446b9 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "runtime-artifacts-elected-content | Runtime Artifacts as Elected Content | sha256:48ff8d7f46d423b9 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - "holochain-evolution-epic-mvp-plan | Holochain Evolution Epic | sha256:467fa823a8d1c32a | path: genesis/docs/superpowers/plans/2026-09-04-holochain-evolution-epic-mvp-plan.md"
  - elohim/holochain-conductor/crates/holochain_p2p/src/local_agent.rs
  - elohim/elohim-storage/src/hc_client.rs
  - elohim/conductor-image/README.md
  - app/elohim-app/scripts/hc-mesh.sh
---

# Nachalah — the Allotment Epic

> *Nachalah*: the allotted portion, the inheritance each tribe was given to hold and steward.
> The DHT address space is not a commons every peer must carry in full. It is allotted: each
> household holds the portion its trust and compute earn, the golden records are held by all,
> the paper is held near.

## 0. Where this came from (2026-09-05, the morning after the sunset partitioned the household)

The Holochain Evolution Epic proved on the household mesh that a hApp version can cross without a
wipe. The same night its Station 6 went red for a reason that was not the crossing: Station 8's
`seal_close` closed two real v1 chains, the next write on them (a `CapGrant` every client mints on
connect) was invalid to every neighbour, and holochain 0.7 blocked both authors' cells forever
with no unblock. One rejected op per author partitioned the space. The mechanism that makes the
DHT trustworthy by construction — every peer validates every action — is the mechanism that makes
it brittle at household scale, because today every peer IS every authority.

The operator's 10,000 ft reading, recorded here as the epic's premise:

- A record does not need gossip among ALL peers to be resilient. It needs a **critical floor**
  derived from the underlying compute and trust — seven household hubs in diverse regions is
  resilient for almost anything we hold.
- Today everything on the DHT is treated as **digital gold**: held by all, validated by all,
  served by all, forever, at the cost of corpus × peers on every axis. A learner's draft note pays
  what the constitution root pays, and a rejected draft note partitions the household as surely
  as a forged commandment would.
- What we need is to **discriminate between paper and golden commandments**, and to let our
  EPR governance and social compute — the leveraged trust between peers — decide who holds what.

Holochain did not hand us an unscalable DHT by design. It handed us a **sharded design with the
shard turned off** (0.7: every cell starts with an empty arc and can only grow to full; the
conductor refuses any arc factor above one) and a **punishment model that assumes strangers**
(a permanent block on the first rejected op, no household act can lift it). The DNA is the only
floor lever that ships; arcs are the lever it left unfinished; block governance is absent.

## 1. Charter

**Every record on the DHT is held by the peers its tier deserves. Arcs are earned allotments — REA
commitments read from the trust gradient, regulated by VSM loops so the mesh reacts like a living
ecology — changed under running peers without a big-bang roll. The conductor that enforces this is
our own artifact, built by CI and delivered over p2p like every other artifact.**

Three slices, in the order they can be measured:

| Slice | Name | What it changes | Lever available |
|---|---|---|---|
| 1 | **Gold and paper, per level (Gevul)** | A tier policy per record class, relative to the holon level, and DNA seams drawn along it | today (DNA + network seed) |
| 2 | **Earned arcs** | Our 0.7 conductor line with sharding on, an arc policy hook driven by the valueflow trust gradient, block governance under Mishpat | needs the fork |
| 3 | **The conductor as an artifact** | The ark adopts a conductor binary through rung 5; the fork joins the primary repo's watch; Jenkins only builds | needs slice 2's fork to have something worth shipping |

## 2. Slice 1 — Gold and paper: the tier policy and the DNA seams

Reach already grades records by audience (household → neighbourhood → commons). It governs who
may READ through a doorway. It must also govern who is asked to HOLD and VALIDATE. Three tiers,
each a DNA seam with its own membership and seed:

- **Commandments (gold).** The values agreement, constitution roots, rosters, lineage
  commitments, release elections. Wide membership, full arcs, seven-hub floors, every peer
  validates. This is where trustworthy-by-construction pays for itself.
- **Deeds.** Contribution events, mastery attestations, economic events. Membership scoped to the
  collective that stewards them; floor of three to seven hubs; reach beyond that through storage
  projection and doorways, never through the DHT.
- **Paper.** Drafts, presence, session state, working notes. A household-seeded DNA only that
  household's devices join, or off the DHT entirely (class B/C in the p2p design gate). Never
  gossiped past the people who wrote it.

Gaps (each becomes a plan task when the operator accepts the tier policy):

- G1.1 A written tier policy: every entry type in every integrity zome carries a tier; the
  p2p-design-gate asks for it as question (6).
- G1.2 The DNA seams: which existing DNAs split, which records move, what the migration is (this
  is a crossing — rehearsed with the Holochain Evolution Epic's vehicle, `carry_from` and the
  lineage window).
- G1.3 The floor as a habit: `@concern:nachalah-allotment` — a paper record never leaves its
  household; a gold record reaches every hub; measured on the household mesh with the fixture's
  three seeds.
- G1.4 Reach and holding agree: the Reach vocabulary spec gains the holding axis so a record's
  audience and its floor are one declaration, not two.

## 2b. Gevul — the boundary of a holon's allotment: the fractal, not a single tier policy

*Gevul*: boundary — the edge of a holon's allotment, where a record's holding stops. Nachalah is
the portion inside it. The two words are one construction.

The physics (operator, 2026-09-05): seven-plus billion people make a global full-arc DHT
physically non-viable, whatever the conductor allows. So §2's tiers are not one policy for one
DHT; they are **relative to a level**, and the levels are the holarchy the protocol already
names — device → household → neighbourhood → collective → commons → the global orchestra. Every
holon has a gevul and, inside it, a nachalah. **What is gold at one level is paper at the level
above.** A household's records are gold to the household (every device holds and validates them)
and paper to the planet (never gossiped past the household). A collective's deeds are gold to the
collective and paper to the commons.

**Values are the same system held at different arcs (operator correction, 2026-09-05).** The
elohim's global values agreement — the covenant that centres human flourishing — is core at
every level. But the scope of what is *agreed* at each level is constructed by the consensus
underneath it: global values, local cultural values, individual values are all value agreements on
the same system, each held at the arc of the holon that agreed it. Immutability is not a flag on
a record; it is an **emergent property of arc width**. A value held by every peer on the planet
is practically immutable because the physics, the compute and the deliberation required to
change it are planetary. A household's value is flexible because three devices can deliberate
by supper. The gradient from flexible to practically immutable IS the gradient from a narrow
arc to a wide one, and the balance between global, local and individual values is not a
policy someone writes — it is where each agreement sits on that gradient, decided by the
consensus that formed it.

A record's reach is therefore a **path up the holarchy** with a floor at each level it crosses:
three devices at the household, seven diverse hubs at the commons, every peer at the orchestra.
Crossing a gevul upward is an act — a witnessed event with a steward, the same shape as the
earned-reach PR-ceremony — never a side effect of gossip. Crossing downward (a commons record
reaching a household) is projection through storage and doorways, not holding.

This is the Reach vocabulary (household → neighbourhood → commons) extended by one axis: reach
already says *who may read*; gevul says *who is asked to hold and validate*; the two are one
declaration on the record. It is also the hyperscaler inversion named on the seam atlas — no
hyperscaler has a social plane that can decide where a record's holding stops.

Gaps added by this reading:

- G2b.1 The level ladder as data: each DNA seam declares the holon level it serves and its floor
  recipe; a record's tier is read relative to that level (gold-here / paper-above), never as an
  absolute.
- G2b.2 The upward crossing as a ceremony: promoting a record across a gevul is a witnessed,
  stewarded event (earned reach), with the receiving level's floor as its precondition.
- G2b.3 The global tier is enumerable: a standing list of what is gold at the orchestra level
  (the covenant, roots, elections), reviewed as a Mishpat matter; anything not on it is paper
  above its own holon by default.
- G2b.4 Fractal measurement: the allotment habit runs at two levels on the household mesh (device
  and household) before any collective level exists, so the relative-tier rule is measured, not
  asserted.

## 2c. The Republic ceiling — carrying capacity is the protocol limit, and we are the IPv4 generation

Holochain will not survive seven billion peers on one DHT, whatever the arcs say. The honest
question is: **what is the maximalist "republic" the physics of one Holochain space can
reasonably support** — the peer count × corpus × validation load at which gossip, arcs and
deliberation still converge — and at the global scale, **how much diversity can be held under
that limit?** Carrying capacity, not membership, is the constraint the top of the fractal is
built against — but it is the CEILING, not the goal.

**Deterministic floor, elohim ceiling (operator correction, 2026-09-05).** The floor of every
allotment is deterministic: the minimum holding and verification the physics guarantees, the
same floor the pre-push gate dogfoods as a reach-earned attestation. The ceiling is the
ecology's carrying capacity — the limitarian cap, Robeyns' upper limit, the top of the donut —
and it is reached only at the full-arc global tier — §2c is where the *maximal diversity the
physics of the DHT allows* lives, and only there. Neither is the everyday. **The everyday goal
is the most performant representation that TRUST allows.** Trust is the resource that lets a
record be held by fewer, validated by fewer, and still be viable: seven hubs that have earned
each other's trust through witnessed contribution carry what a trustless design would need
seven thousand strangers to carry. A trust-full architecture is MORE viable, not less — the
trustless "everyone verifies everything, nobody is trusted" pattern is the crypto anti-pattern
this protocol rejects, and full-arc-everywhere is that anti-pattern wearing a Holochain costume.
So the allotment at each level is: never below the deterministic floor, never above the elohim
ceiling, and sized in between by the trust the peers have actually earned.

This is a network-maturity concern of the kind the IPv4 designers met: an address space that
looked infinite until it was not, and a protocol limit whose real meaning only appeared at
scale. We are that generation for this substrate. The fractal is our answer in advance: the
orchestra level is not one republic of every peer but a **republic of republics** — the
commons-level holons hold the covenant at full arc among themselves, and each commons is itself
a republic of collectives, down to the household. The ceiling bounds the width of any one level;
the depth of the fractal is what carries the diversity.

Gaps:

- G2c.1 Measure the ceiling: on the household mesh and then alpha, the peers × corpus curve at
  which gossip rounds stop converging, validation backlog grows without bound, or a new joiner's
  initial sync exceeds the deliberation window — a measured constant with error bars, re-measured
  per conductor line, recorded here.
- G2c.2 Design the top level against the ceiling: the orchestra as a republic of republics, with
  delegation (stewards, elohim) as the crossing between levels — the sociocratic double-link, not
  a flat membership.
- G2c.3 Trust-sized allotments between floor and ceiling: each level's recipe names the
  deterministic floor (count and diversity the physics requires), the elohim ceiling (the
  ecology's carrying capacity, reached only at the global tier), and the trust evidence that
  sizes the everyday allotment between them — so the mesh runs at the most performant
  representation trust allows, and grows toward the ceiling only as deliberation demands it.

## 3. Slice 2 — Earned arcs: sharding on, arcs from the trust gradient, blocks under Mishpat

The intended sharding picks arcs by address-space geometry, blind to who the peers are. Our
valueflows already know which peers have earned what: witnessed contributions, saga survival,
recovery drills passed. An arc assignment that reads that gradient — seven trusted hubs cover the
commandments in full, lightly trusted phones hold slivers of paper — is the thing neither
Holochain nor any hyperscaler has: **a floor derived from social compute rather than hardware
count.** The crossing work is its rehearsal, because an arc policy, like a DNA lineage, changes
under running peers.

Posture: an arc is an **earned, evidence-backed commitment** (a Mishpat commitment bounded by the
peer's measured reliability), never a geometry accident and never a self-declaration. Arcs are
**dynamically negotiated by the elohim**: a good allotment decision is aware of its natural
limits (the deterministic floor and the elohim ceiling), its externalities (what the holding costs
the peer and the ecology, what it denies others), and its context (the level, the trust in
force, the environment S4 is reporting) — so the negotiation is a standing loop, not a
one-time assignment. A peer that
claims a wider arc than its evidence supports is refused by the same roster check that refuses a
forged lineage.

Gaps:

- G2.1 The fork: `elohim/holochain-0.7` started from the stock 0.7.0 the mesh already runs (a
  re-port of the 0.6.3 cures, not a rebase — kitsune2 replaced the networking layer). Carries
  only: (a) admin `list_blocks` / `unblock` (today reached by opening the encrypted db — Task 30
  of the companion), (b) sharding enabled with an arc policy hook the storage peer drives, (c) the
  household gossip defaults (60 s round deadline, 4 accepted rounds) as defaults, not patches.
- G2.2 The arc policy hook: storage computes each cell's target arc from the tier (§2) and the
  trust gradient (valueflow evidence) and sets it through the hook; changes are commitments with a
  window and a revert, rehearsed with the companion's vehicle.
- G2.3 Block governance: a rejected op on paper is a household matter (lift locally, name the
  author); on a commandment it is a Mishpat matter (a case, a ruling, a bounded block). The
  permanent-forever default is replaced by a tier-graded interval.
- G2.4 The partition probe becomes a habit check: per-space `dumpNetworkMetrics` (arc null +
  no completed round + timeouts rising) reds by name in the passport (companion Task 32 mints the
  probe; this epic binds it to the allotment habit).

## 3b. The ontology — allotment as a living ecology (REA + VSM + EPR), not a faster deterministic network

Allotment alone is the performance slice. The operator's steering (2026-09-05): the REA and VSM
primitives are what let Holochain evolve from a deterministic network into a system that reacts
like a living ecology. So the allotment is not a tuning knob on kitsune2; it is an economic and
cybernetic object, minted in the ontology the substrate already carries (`elohim-epr`, `epr-rea`,
`elohim-compute` — inherit, never duplicate).

**REA reading.** The *resource* is holding capacity: a slice of address space × the validation,
storage and serving compute to keep it. The *agents* are peers (and the households and collectives
they belong to). The *events* are holding, validating, serving, healing — each a witnessed
economic event with a cost the peer bears and a value the commons receives. The *commitment* is
the allotment itself: a Mishpat-bounded promise by a peer to hold a portion at a floor for a
window, claimable, fulfillable, revocable, priced by the trust gradient the peer has earned. An
arc is therefore an REA commitment with a provenance chain, not a self-declared number, and the
critical floor for a tier is a *recipe* (this many hubs, this diverse, this reliable) that the
valueflow projects and measures — the same machinery that projects a developer's commitments from
the repo projects a household's holding commitments from the DHT.

**VSM reading.** The ecology has the five systems Beer names, and each already has a seat here:

| VSM system | What it is in the allotment | Where it lives today |
|---|---|---|
| S1 operations | holding, validating, serving a portion | the conductor's cells and arcs |
| S2 coordination | gossip and reconciliation between overlapping portions | kitsune2 rounds; storage's reconcile controller |
| S3 control | the arc policy: what each peer is asked to hold now | the hook storage drives (§3 G2.2) |
| S3* audit | the partition probe, the block ledger, the passport | companion Task 32; Task 30's tool |
| S4 intelligence | the trust gradient and the environment: who has earned what, what the world is doing to the mesh | valueflow evidence, saga survival, recovery drills |
| S5 policy | the tiers and the refusals; Mishpat rulings on blocks | §2, §5; the Mishpat DNA |

**EPR coupling — the minimal atom that keeps the whole network honest.** REA says what the
economics of holding are and VSM says how the loops regulate; the EPR is what makes any of it
checkable. Every holding claim, holding event, allotment commitment and trust judgement is an
EPR atom: content-addressed, witnessed, cite-sealed, carrying its own provenance — the same atom
the repo's valueflow, the habit register and the lineage witness already use. That is the
smallest unit at which honesty can be verified without trusting the teller, and it is the unit
wisdom deliberates over: the elohim scale trust in balance with the whole by reading atoms, not
narratives — this peer held that portion through that recovery, witnessed by those neighbours,
at that cost. Trust that scales is trust that resolves to atoms; an allotment negotiated over
anything less is a story, and stories are how a network drifts from honest. The coupling is
what lets a trust-full architecture stay viable at width: fewer verifiers, because what they
verify is atomic and content-addressed rather than everything, everywhere.

Ashby's law is the design rule: the variety of the environment (peers joining, leaving, failing,
misbehaving; records of every weight) must be met by variety in the regulator. A single full arc
and a single permanent block are the regulator with no variety; the allotment, graded by tier and
earned by evidence, is the regulator with enough. That is the difference between a network that
is merely deterministic and one that reacts.

Gaps added by this reading:

- G3b.1 The allotment as an REA commitment type, inherited from `epr-rea`, with the recipe for a
  tier's floor projected and measured by `epr flow` (no new ledger, no parallel schema).
- G3b.2 Holding events as witnessed economic events (attention/compute-denominated), so a peer's
  holding history IS its trust evidence — the S4 signal that S3 reads, closing the loop without a
  human in it as terminal authority.
- G3b.4 The EPR coupling: holding claims, events, commitments and trust judgements are EPR
  atoms (content-addressed, witnessed, cite-sealed) in the existing atom home, so an allotment
  negotiation and a Mishpat ruling on a block both resolve to atoms a neighbour can re-verify.
- G3b.3 The five VSM seats named on the seam atlas so a future concern (a new probe, a new policy)
  self-locates: is it S3* (audit) or S5 (policy)?

## 4. Slice 3 — The conductor as an artifact: the sixth rung-5 class

Rung 5 already moves coordinator bundles, hApp bundles, config EPRs, the storage binary and
(with the companion) a whole DNA lineage between peers with no Jenkins act. The conductor is the
one process nothing in the peer can adopt, because only the ark owns its lifecycle and the ark
today only witnesses death. That is why the fork lives on a separate pipe wired to the
che-devworkspaces submodule, outside the webhook, with a manual build tag — the CI hygiene pain.

Gaps:

- G3.1 `conductor-binary` as an artifact class: a release manifest, hash, applies-to declaration,
  packaged by the same ceremony that packages storage.
- G3.2 The ark adopts it: verify, stage, swap the pinned artifact in the runtime manifest,
  restart the child under the same berth; keep the previous artifact and roll back on a failed
  readiness check without an election round trip; the storage token re-mint on restart is the
  hand-over's first step.
- G3.3 The fork joins the primary repo's watch (a real subtree or a watched submodule); a bump
  dispatches the conductor build the way a DNA change dispatches the DNA build; `update = none`
  retires.
- G3.4 Build stays in CI, delivery leaves it: Jenkins compiles and publishes the candidate to the
  DHT from the workspace peer (the workspace-to-fleet story); alpha follows observe → canary →
  apply per peer; revert through the same channel.

## 5. What this epic refuses

- Refuses "hold everything, validate everything" as the default posture for any tier below gold.
- Refuses self-declared arcs: an allotment is earned and bounded, or it is not an allotment.
- Refuses a permanent block that no household act can lift.
- Refuses a conductor that reaches the fleet by any path other than the one every other artifact
  takes.
- Refuses re-keying as a cure for anything in this epic (the companion's rule holds here).

## 6. Sequence

1. ~~Operator accepts §2's tiers and §3's posture (Draft → Active).~~ Done 2026-09-05.
2. Slice 1 plan: tier policy + first seam split + the allotment habit (household mesh, three
   seeds). Cheapest; no fork.
3. Slice 2 plan: the fork with blocks first (it retires Task 30's database surgery), then
   sharding + the hook.
4. Slice 3 plan: the ark adopts the conductor; the first thing shipped through it is slice 2's
   fork.

## 7. Progress (the hub; every follow-up starts here)

### 7.1 Ledger (newest first)

- 2026-09-05 — **graduated Draft → Active** on the operator's acceptance ("ok accepted.. let's graduate that"): §2 tiers, §2b fractal, §2c floor/ceiling/trust, §3 posture, §3b ontology all accepted. Next artifact: the Slice 1 plan (tier policy + first seam split + the allotment habit on the household mesh, per §6 step 2).
- 2026-09-05 — steering (operator): §2c is where the maximal diversity the physics allows lives (the ceiling, only there); §3b gains the EPR coupling — the minimal atom that keeps the whole network honest, what wisdom deliberates over to scale trust in balance with the whole (G3b.4).
- 2026-09-05 — operator: satisfied with the allotment design ("you've got the right idea now"); arcs are dynamically negotiated by the elohim, every allotment decision aware of its limits, externalities and context (§3 posture amended).
- 2026-09-05 — steering (operator): deterministic floor, elohim ceiling — the ceiling (planetary carrying capacity, the limitarian cap) is reached only at the full-arc global tier; the everyday goal is the most performant representation that TRUST allows; a trust-full architecture is more viable; full-arc-everywhere is the trustless crypto anti-pattern in a Holochain costume (§2c corrected, G2c.3 rewritten).
- 2026-09-05 — steering (operator): (a) values are the same system held at different arcs — immutability is an emergent property of arc width, local/individual values are constructed by the consensus underneath the global covenant (§2b corrected); (b) the Republic ceiling — one space's carrying capacity is the real protocol limit, we are the IPv4 generation, the orchestra is a republic of republics (§2c, gaps G2c.1–3).
- 2026-09-05 — steering (operator): the construction is fractal — every holon has a gevul (boundary) and a nachalah inside it; gold at one level is paper at the level above; only the covenant is gold everywhere; a record's reach is a path up the holarchy with a floor at each level — §2b added (gaps G2b.1–4).
- 2026-09-05 — steering (operator): allotment is the performance slice; the REA + VSM primitives are what make the DHT a living ecology — §3b added (allotment = REA commitment; five VSM seats; Ashby as the design rule; gaps G3b.1–3).
- 2026-09-05 — epic planted from the operator's sidebar after the sunset partition finding
  (companion §11.4). Brand *Nachalah*, name *Allotment*. Boundary with the companion recorded in
  frontmatter `boundary:`. Status Draft; nothing decomposed yet — the first plan waits on §6 step 1.
