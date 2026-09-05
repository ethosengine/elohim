---
title: "Nachalah — the Allotment Epic: every record on the DHT is held by the peers its tier deserves, arcs are earned allotments read from the trust gradient, and the conductor that enforces this is our own artifact shipped over p2p"
id: nachalah-allotment-epic
brand: Nachalah
name: Allotment
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
serves:
  - happ-lineage-migration
  - runtime-upgrade-propagation
  - dataplane-convergence
graduation-trigger: Draft→Active when the operator accepts §2's tier policy (gold / deeds / paper) and §3's allotment posture (arcs are earned, evidence-backed commitments, never geometry accidents); Active→Canonical when (a) the household mesh runs three DNA seams with three different floors and a `@concern:nachalah-allotment` receipt shows a paper-tier record never leaving its household while a gold-tier record reaches every hub, (b) our 0.7 conductor line runs with sharding on and an arc policy hook driven by the valueflow trust gradient, measured on the mesh, and (c) a conductor binary propagates to alpha through rung 5 with no Jenkins act after the build
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

**Every record on the DHT is held by the peers its tier deserves. Arcs are earned allotments read
from the trust gradient, changed under running peers without a big-bang roll. The conductor that
enforces this is our own artifact, built by CI and delivered over p2p like every other artifact.**

Three slices, in the order they can be measured:

| Slice | Name | What it changes | Lever available |
|---|---|---|---|
| 1 | **Gold and paper** | A tier policy per record class and DNA seams drawn along it | today (DNA + network seed) |
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

## 3. Slice 2 — Earned arcs: sharding on, arcs from the trust gradient, blocks under Mishpat

The intended sharding picks arcs by address-space geometry, blind to who the peers are. Our
valueflows already know which peers have earned what: witnessed contributions, saga survival,
recovery drills passed. An arc assignment that reads that gradient — seven trusted hubs cover the
commandments in full, lightly trusted phones hold slivers of paper — is the thing neither
Holochain nor any hyperscaler has: **a floor derived from social compute rather than hardware
count.** The crossing work is its rehearsal, because an arc policy, like a DNA lineage, changes
under running peers.

Posture: an arc is an **earned, evidence-backed commitment** (a Mishpat commitment bounded by the
peer's measured reliability), never a geometry accident and never a self-declaration. A peer that
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

1. Operator accepts §2's tiers and §3's posture (Draft → Active).
2. Slice 1 plan: tier policy + first seam split + the allotment habit (household mesh, three
   seeds). Cheapest; no fork.
3. Slice 2 plan: the fork with blocks first (it retires Task 30's database surgery), then
   sharding + the hook.
4. Slice 3 plan: the ark adopts the conductor; the first thing shipped through it is slice 2's
   fork.

## 7. Progress (the hub; every follow-up starts here)

### 7.1 Ledger (newest first)

- 2026-09-05 — epic planted from the operator's sidebar after the sunset partition finding
  (companion §11.4). Brand *Nachalah*, name *Allotment*. Boundary with the companion recorded in
  frontmatter `boundary:`. Status Draft; nothing decomposed yet — the first plan waits on §6 step 1.
