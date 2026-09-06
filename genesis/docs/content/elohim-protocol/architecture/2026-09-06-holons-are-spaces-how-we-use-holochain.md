---
title: Holons Are Spaces — How We Actually Use Holochain, and How It Scales
id: holons-are-spaces-how-we-use-holochain
date: 2026-09-06
status: reference
author: orchestrator (overnight shift, 2026-09-06; grounded in the 0.7 fleet line at fork 25dd2d0be and kitsune2 0.5.1)
tier: architecture
cites:
  - "nachalah-allotment-epic | Nachalah | sha256:855a5cb52df7f201 | path: genesis/docs/superpowers/specs/2026-09-05-nachalah-allotment-epic-design.md"
  - "holochain-evolution-epic | Holochain Evolution Epic | sha256:d821c5f45fd5d2e5 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "conductor-authority-arc-auto-policy | 2026-06-13-conductor-authority-arc-auto-policy | sha256:597157e7bb552d73 | path: genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md"
---

# Holons Are Spaces — How We Actually Use Holochain, and How It Scales

*An explainer you can teach from. Start at the top; every section builds on the one before. Numbers are
from the live alpha fleet on 2026-09-06 unless marked otherwise.*

---

## 0. The one-paragraph version

Holochain does not scale by making one big database bigger. It scales by making many small networks,
one per group of people who share rules, and letting each person's device join only the networks it
belongs to. Those networks are called **spaces** (a DNA plus a network seed). A space is Holochain's
**holon**: a whole in itself, part of a larger whole, with a membrane. We built the Elohim Protocol on
Holochain but, for our first two years, we put almost everything into **one** space and asked every
peer to hold **all** of it. Then we built machinery in our own storage layer to shed the load that
layout created. The cure is not a fork and not a new feature: it is to use spaces the way they were
designed, with the tiers the Nachalah epic already names, and to let content declare *which space it
lives in* rather than *how much of a space each peer should hold*. Fractional arcs inside a single
space are a real, separable optimization that does need a fork. They are not the architecture.

---

## 1. First principles — what a Holochain network actually is

Forget "blockchain" and forget "database." Picture three things.

**1a. Every person writes their own journal.** Each agent (a keypair on a device) has a *source chain*:
an append-only, signed log of the things that agent did. Nobody else writes in it. There is no global
ledger. There is no consensus about "the order of everything." There is only "Matthew signed this, after
that, at this time."

**1b. Journals are gossiped into a shared shelf.** When you write an entry, it is also handed to a set of
other peers to hold and validate. Which peers? The ones whose *address* is near the entry's *hash*. Every
entry has a hash; every agent has a hash; both live on the same circle of 2^32 addresses. A peer holds
the slice of that circle it has declared responsibility for — its **arc**. The shared shelf is the
**DHT**, the distributed hash table. It is not a copy on every node; it is a circle sliced into arcs.

**1c. The rules are the network.** Before a peer holds your entry, it runs the *validation rules*.
Those rules are compiled code — the **integrity zome** — and the hash of that code (plus a few
modifiers) *is the identity of the network*. Two peers are on the same network if and only if they run
the byte-identical rules. Change one byte of the rules and you have created a different network with
nobody in it.

That third point is the one to hold onto. In Holochain, **"which network am I on" and "which rules do
I run" are the same fact**, and it is content-addressed: the network's name is the hash of its rules.

### The vocabulary, precisely

| Word | What it is | What it is *not* |
|---|---|---|
| **DNA** | A bundle of integrity zomes (rules) + coordinator zomes (behavior) + modifiers | An app; a database |
| **DNA hash** | Hash of the integrity zomes and the modifiers. The network's identity. | Something you can change in place |
| **Modifiers** | The network seed and "properties" baked into the DNA hash | Runtime config |
| **Space** | One DNA hash = one DHT = one set of peers gossiping. *A holon.* | A table or a folder |
| **Cell** | One agent's participation in one space (its source chain there) | A server |
| **Clone cell** | A new space from the *same* rules with a different network seed — no code, no reinstall | A copy of the data |
| **Arc** | The slice of the address circle a peer holds *in one space* | A choice of which entries to hold |
| **Coordinator zome** | Behavior code; hot-swappable; does NOT change the DNA hash | The rules |
| **Integrity zome** | The rules; changing them changes the DNA hash → new network | Behavior |

Two hard facts follow, and the whole essay turns on them:

- **Hard fact A — an arc is geometry, not choice.** Inside a space you hold "addresses 0x1A000000 to
  0x4C000000." You cannot hold *this* document and not *that* one. Per-entry replication policy does
  not exist inside a space, and cannot, because the address of an entry is its hash and the point of
  the design is that nobody chooses where things land.
- **Hard fact B — an entry lives in one space.** Links, validation, gossip and arcs are all per space.
  There is no cross-space link. To put a document "in" two spaces you publish it twice, once in each.

---

## 2. What "holonic" was always supposed to mean

The word in the name is a design statement. A **holon** (Koestler's term) is a whole that is also a
part: a cell is whole and is part of an organ; a household is whole and is part of a village. Holochain
made the holon the unit of the network on purpose: one DNA per group-that-shares-rules, and a peer's
conductor runs many cells at once, one per space it belongs to. Your device is the place where your
holons meet. There is no "the network." There are the networks you are a member of.

This is why the project's own language fits Holochain so naturally once you see it:

- A **household** is a space. Its rules are household rules. Its members are its peers. Its arc
  budget is "five devices hold everything," because five is small.
- A **collective** (a church, a co-op, a school) is a space. Larger membership, different rules, its own
  holding floor.
- The **commons** (elohim.host content, the protocol's own docs, the public learning paths) is a space.
  Thousands of readers, tens of stewards.

And the thing the Nachalah epic calls *promotion* — a household note becoming a collective deed
becoming a commons page — is, in Holochain terms, a **witnessed re-publish into a wider space**. Hard
fact B is not an obstacle to that; it is *why promotion is a ceremony*: the content's identity (its CID)
does not change, but its *holders* do, and that is exactly the act the elohim witness.

---

## 3. How we actually use Holochain today

Let me be precise, because this is the part that was never written down as a decision.

**Our hApp has five roles**: `lamad` (content: every learning node, page, path), `imagodei` (identity),
`mishpat` (governance, commitments, grants), `infrastructure`, and `node_registry`. Each role is one DNA.
On the alpha fleet, each role is **one space** — the fleet's peer store lists exactly five spaces, and
every peer is a full-arc member of every one of them (32 of 35 agent entries carried the full arc
`[0, 4294967295]` this morning; the three exceptions are the OOM-shed leechers).

**The content role is one space for everything.** Every learning node, every landing page, every
fixture the test suite ever seeded, every household's working note that has been written so far, lives
in the single `lamad` space. On doorway-A's storage peer that is 3,442 content rows this morning; the
sync plane counts 110 live documents in flight; the test fixtures alone are several thousand EPRs.
Every full-arc peer validates and holds all of it. When a peer restarts, it re-gossips all of it.
The habit ledger's "restart churn ≈ 20 minutes" and "RAM ∝ corpus at full arc" are direct consequences
of this single layout decision.

**And there are no household spaces yet.** I want to be exact here, because I got this wrong in the
first draft of this document. The only place our hApp manager uses a network seed today is a *lineage*
install — a role moving from an old DNA hash to a new one keeps its seed so it stays on the same network.
No code path creates a clone cell for a household or a collective; the "household space" the Nachalah
stories describe is a design, and the "household space partition" of 2026-09-05 was a cell on the local
household *mesh* being blocked, not a clone space. Every integrity zome's membership check
(`genesis_self_check`) is a stub that returns *valid* for anyone. So the primitive is available in the
substrate and unused in the product: one space per role, open to any key, everything at full arc.

**Then we built a second system to cope.** This is the honest part. Because everything sat in one space
at full arc, we grew, in *our own* storage layer:

- an **arc actuator** that flips a whole peer to zero-arc when it OOMs (jessica and james were flipped
  to arc 0 in June for exactly this reason), and an **arc policy** that computes a fractional aim it
  cannot actually set;
- **reach enforcement at read time** — the `reach-enforced-everywhere` habit — so that a peer that
  *holds* a household's bytes (because it holds the whole space) refuses to *serve* them to the wrong
  person;
- **held views, per-host custody stamps, projection fallbacks** — ways for a doorway to keep serving
  when its conductor cannot answer for a corpus that is too big to be timely;
- a **quiesce gate** that waits for the fleet to finish catching up before it will believe a measurement;
- **fixture hygiene** rituals to keep test content from crowding real content.

None of that is wrong code. Most of it is load-bearing today. But look at what it is: it is
**membership and holding, re-derived one layer up, because we did not use the layer that has it.**
A household space with five members never needs read-time reach enforcement against outsiders —
outsiders are not in the space. A fixture space nobody stewards costs nobody gossip.

### The mental model we were carrying

We treated the `lamad` DNA as *the content database* and the DHT as *replication*. That is the
relational-database instinct the `p2p-design-gate` skill exists to catch, and it slipped through at
the coarsest grain: not at the entry level (we got content addressing right, see §5) but at the level
of *what a space is for*. We thought of a space as a **schema**. Holochain thinks of a space as a
**membership**.

---

## 4. The scaling question, with numbers

Let me do the arithmetic the way I'd do it on a whiteboard.

### 4a. The way we are doing it now (one space per role)

Let `N` be the number of peers, `D` the number of documents, `a` the arc fraction each peer holds.

- Storage per peer ≈ `a · D` documents. At full arc, every peer holds `D`.
- Gossip per peer per round ≈ proportional to what it holds and how many neighbors it reconciles with.
  At full arc, every peer reconciles the whole corpus with every other peer.
- Validation work per peer ≈ `a · (writes per second)`. At full arc, every write is validated by
  everyone.

Today `N = 7`, `D ≈ 3,442 + fixtures`, `a = 1`. That already costs 20 minutes of churn per restart.
Now set `N = 7,000,000,000` and `D` = every household's notes on Earth. With `a = 1` the model is
absurd — nobody can hold humanity's journal — and with fractional `a` (which we cannot set today
anyway) each peer would hold a random 1/N slice of *strangers'* documents, validating rules for a
community it does not belong to. That is not a privacy model anyone wants, and it is not a
resiliency model either: your household's photos would be held by whoever's address happens to be
near their hash, in Jakarta and Lagos and Lima, at a replication factor set by a global constant.

### 4b. The way Holochain is built to do it (one space per holon)

Now let a space be a membership. Let a person belong to `k` holons: their household (say 5 devices),
two or three collectives (150 to 5,000 members each), and the commons (millions of readers, thousands of
stewards). Typical `k` is maybe 10 to 50 over a lifetime, most of them small.

- Storage per device ≈ Σ over its spaces of (that space's documents × that device's arc there).
  The household space is tiny and held fully by all 5 devices. The collective space is medium and held
  fully by its stewards, at zero arc by its readers. The commons is large and held by its stewards' rack
  nodes at full arc, by everyone else at zero.
- Gossip per device ≈ Σ over its spaces of (that space's churn). Your phone gossips your household's
  notes and your collective's deeds. It never hears about a household in Lima. It cannot; it is not in
  that space.
- Validation per device ≈ only the rules it has agreed to, for the communities it has joined.

The total work in the world grows with `Σ (members × documents)` **per holon**, which is a sum of
small products, not one product of two huge numbers. Seven billion people is 1.5 billion households of
five, each a space of five peers holding a few thousand documents, plus a long tail of collectives.
Each one is the size of the mesh we already run in this workspace (three peers, one household). We
have been rehearsing the unit of scale for a year without naming it.

### 4c. The two things that do scale globally, and how

Two things must cross holons: **identity** and **discovery**.

- **Identity** crosses because we chose content addressing. An EPR's CID is the hash of its bytes; it is
  the same in every space it is published into. The DHT anchor (the action hash) is per space; the
  *thing* is not. Our earlier choice to make the CID the identity and the DHT anchor a per-space
  attribute is the single decision that makes holonic placement possible at all. Hard fact B (no
  cross-space links) does not bite because we never linked by action hash across spaces; we link by CID.
- **Discovery** — "which peers are in this space, and how do I reach them" — is per space too, and
  that is fine. Kitsune2 bootstraps per space; our doorways already serve bootstrap and relay per space.
  A new household mints a seed, its five devices bootstrap through any doorway, and no global registry
  is needed. The **elohim** (the aggregation plane, doorways, the future hub cluster) is how a person
  finds a holon they are not yet in — which is a social act, not a DHT act.

### 4d. What the doorway and the blob plane do in this picture

- The **blob plane** (iroh/libp2p in our storage) already lives outside the DHT: big bytes never go into
  a space; only manifests and heads do. Spaces partition *notarization and small-record gossip*. The blob
  plane follows the same membership (a household's peers hold a household's blobs) but its transport is
  independent, which is why it was the right plane for the T2 substrate.
- A **doorway** is a web2 projection of *the spaces its storage peer is a member of*. That is also why a
  doorway "cannot see" a household: it should not be able to, unless a household member runs the doorway
  or the household promoted the content to a space the doorway steward belongs to.

---

## 5. Where we fought the architecture, and where we got it right

Being fair to the last two years:

**We got right:**
1. **Content addressing as identity** (§4c). Without it, holons would be islands.
2. **Blobs off the DHT.** The DHT holds small signed records; bytes ride the storage substrate.
3. **Heads as declared, elected content.** A page's "current version" is a record peers agree on by
   rules, not a mutable row. Release channels for runtime upgrades are the same idea applied to code, and
   last night the first coordinator release crossed from a workspace to the fleet by exactly that
   mechanism, no pipeline in the path.
4. **Coordinator hot-swap.** Behavior changes without changing the network's identity. This is
   Holochain's own gift and we lean on it hard.
5. **Lineage installs keep their seed.** A role crossing to a new DNA hash stays on its network — the
   seed plumbing the holon model needs is already exercised, just not for holons.
6. **The Nachalah tiers on DNA seams.** Gold, deeds, paper: the epic already says the tiers are spaces.
   We had the design; we had not applied it to content.

**We fought the architecture by:**
1. **One content space for the world.** Everything else on this list follows from it.
2. **Treating arc as the knob.** Arc actuators, fractional aims, "shed to zero on OOM" — all of it is
   trying to make a peer hold *less of the wrong space* instead of *not being in it*.
3. **Reach enforced at read time in storage.** Necessary today, but it is a membrane re-implemented at
   the wrong layer. Holochain's membrane is *space membership + membrane proof*. If outsiders are not
   members, the bytes never reach them; there is nothing to refuse at read time.
4. **Per-host custody stamps** (the `serverBlobHash` PATCH the dataplane-convergence habit now flags):
   a doorway telling its storage peer "you serve this bundle" is a per-host imperative because there was
   no space to say "the doorway stewards hold the served commons at full arc" declaratively.
5. **Fixtures in the commons.** Thousands of test EPRs in the same space as elohim.host's landing page,
   gossiped by every fleet peer forever. A `fixtures` clone space nobody follows at full arc costs nothing.
6. **Full-arc-everywhere as the trust posture.** The Nachalah spec already names this: full-arc
   everywhere is the *trustless* anti-pattern. A holon holds what its members trust each other to hold.

---

## 6. The hard constraints, stated once

These are the walls. Design inside them.

1. **The DNA hash is the network.** Integrity zomes + modifiers, hashed. Change either, you have a new
   network. Therefore: never put *policy* (arc, reach, tuning) in DNA properties. Policy lives in
   runtime config, in coordinator behavior, or in *which space* something is placed.
2. **Arc is a keyspace slice per agent per space.** On the 0.7 line the conductor still clamps it to
   full or zero ("not yet allowed until sharding is implemented"). Kitsune2 already carries ranges on the
   wire, so lifting the clamp is a *fork* item — the "arc-policy hook" in the Evolution epic §11.3. It is
   worth doing for large commons spaces. It changes nothing about §4.
3. **An entry lives in one space; links do not cross spaces.** Cross-holon reference is by CID;
   cross-holon presence is a re-publish (promotion), witnessed.
4. **Validation is by the members of the space, against that space's rules.** A five-device household
   validates household rules. It cannot validate "commons reach" for you — that is what promotion into
   the commons space is for.
5. **A cell is a full participant.** Conductor memory and gossip scale with the *number of cells* a
   device runs. Holons are household- and collective-sized. Never per document, never per person-pair.
6. **A rejected op can block a cell (0.7).** Every space is its own partition risk; the household
   unblock tooling the epic built matters *more* as spaces multiply, not less.
7. **Crossing a lineage break moves data by ceremony, not by magic.** Moving content from the one big
   space into tiers is a crossing per holon — the same machinery the Evolution epic is building for
   version crossings. Nothing is thrown away; it is re-published under witness.
8. **Discovery is per space.** Bootstrap and relay per DNA hash; the doorway already does this.

---

## 6½. Arcs, precisely — what is configurable today, what the Holochain team's scaling story is, and what the arc work is for

This section exists because the ruling "the unit of arc is the space" can sound like "arcs don't matter."
They matter a great deal. They are just the *second* axis of scale, and the one that is currently missing.

### The Holochain team's own scaling story (both axes)

Read the Holochain design papers and the team's talks and you find **two** independent answers to "how
does this reach billions," and the project's health depends on knowing which one is available today.

**Axis 1 — many networks (holons).** No global consensus, no global network. Every hApp is its own
network; every clone is its own network; a person's conductor bridges the networks they belong to;
membranes (membrane proofs) decide who may join. Total work in the world is a sum over small memberships.
This axis is *architectural*: it is how the team expects most of humanity's data to be organized, and it
is fully available in stock Holochain today. §4 of this document is this axis.

**Axis 2 — sharding inside one network (arcs).** For a network that is genuinely large — a public
commons with millions of readers — no single member can hold everything. So each peer holds a *slice*
of the address circle (its arc), and the network sizes the slices so that every entry is held by roughly
`R` peers (the redundancy target) no matter how big the network gets. Work per peer stays roughly
constant as members join, because arcs shrink as density grows. Validation is by the peers whose arc
covers the entry. This is the "sharded DHT" the team describes, and it is what lets *one* space scale,
as opposed to letting *many* spaces coexist.

Here is the historical fact that explains a year of our confusion: **Axis 2 shipped, then went away.**
The first networking layer (Kitsune, holochain 0.1–0.4) implemented dynamic arcs — a peer's arc resized
itself from observed peer density toward the redundancy target. The networking layer was then rewritten
(Kitsune2, holochain 0.5 onward, the line we run). Kitsune2 kept the *data model* — arcs are ranges on the
wire and in the peer store; we can see `[0, 4294967295]` in alpha's peer store this morning — but the
conductor's use of it was reduced to a switch: **full or empty**. The 0.7 conductor at the fleet pin still
says so in code: "target arc factor > 1 is not yet allowed until sharding is implemented." So on the line
we run, Axis 2 is *dormant*: present in the substrate, absent in the conductor.

### What is actually configurable today (verified at the fleet pin)

1. **`target_arc_factor` is a single number per conductor.** It lives in the conductor's network config
   and the conductor builder applies the same value to every cell's local agent. Values: `0` (empty arc,
   a "leecher" who holds nothing and reads from the network) or `1` (full arc). Nothing in between.
2. **It is conductor-wide, not per space.** A conductor cannot today be full-arc in its household space
   and zero-arc in the commons. Kitsune2's internal API *does* carry a per-agent, per-space target-arc
   hint — the conductor simply sets that hint to FULL for every cell (`holochain_p2p` actor, on join) and
   derives the factor from the one global knob.
3. **Changing it needs a conductor restart.** It is boot config. Our storage's arc actuator renders a new
   conductor config and performs a staggered restart; that is the "T1 {0,1} switch" tier the arc policy
   spec describes, and it is what flipped jessica and james to leechers in June when they OOM'd.

So the honest description of our arc surface is: *one Boolean per device, applied to every holon it is in,
changed by restart.* That is not a policy surface. It is a fuse.

### What the arc work is for, then

If arcs cannot be adjusted per space or fractionally, what has the "EPR-native arc configuration" work been
doing? Reading the tree honestly, three things, and they are not wasted:

- **The design layer (Nachalah, September).** "Values as one system held at different arcs; immutability
  is emergent from arc width; arcs are dynamically negotiated by the elohim under limits, externalities and
  context; the everyday allotment is the most performant representation trust allows; full-arc-everywhere
  is the trustless anti-pattern." This is the *policy* half: who should hold what, how widely, on whose
  say. It is correct and it is space-agnostic — it applies at both axes. What it lacked was the mapping to
  the substrate's actual knobs, which is what this document supplies: at Axis 1 the allotment decides
  *which spaces a device joins*; at Axis 2 it decides *how wide a slice it holds in a large space*.
- **The storage layer (`arc_policy.rs`, `arc_actuator.rs`).** A pure `derive()` from memory ceiling,
  coverage floor, observed peer count and corpus size, and an executor that can only express `{0,1}` per
  conductor with a restart. This is a correct Axis-2 controller waiting for an Axis-2 lever. Its coverage
  invariant ("a leecher must leave the mesh covered") is exactly the redundancy-target reasoning the
  Holochain team describes.
- **The fork hook (Evolution epic §11.3, open).** The minimum conductor change that turns the fuse into a
  policy surface is small and local: derive the per-cell target-arc hint from a **per-space policy** instead
  of the global factor, expose it on the admin interface so storage can set it *without a restart*, and let
  the value be a range rather than a Boolean. That is "sharding" from the conductor's side; the peer store
  and gossip already speak ranges. No branch in this repository carries that change yet; it remains the
  named fork item.

**What this means for priorities.** The Axis-1 work (spaces per holon, §7) removes most of today's pain
and needs no fork, so it goes first. The Axis-2 work (per-space and fractional arcs) is what a large commons
needs, and what lets one phone be a full household member and a light commons reader *at the same time* —
so it is not optional for the phone-to-rack spectrum the seam map promises; it is the first thing the fresh
0.7 fork should carry. Both axes are the Holochain team's own story. We had been trying to do Axis 2's job
with Axis 2's dormant lever and not doing Axis 1's job at all.

### The scale story, restated with both axes

Seven billion people. Roughly 1.5 billion households of five, each a space held fully by five devices:
Axis 1, available now. Tens of millions of collectives of 100 to 10,000, each a space held fully by its
stewards and at zero arc by its members: Axis 1, available now. A few thousand commons spaces — the
protocol's own, a language's, a region's, a movement's — each with millions of readers and thousands of
stewards, held in *slices* by those stewards so that no rack needs to hold a whole commons: Axis 2, the
fork item. A person's phone bridges its 10 to 50 spaces; its work is bounded by its own memberships and its
own allotment, never by the size of the world. That is the whole story, and every part of it is either
shipped or a bounded, named change.

### Is sharding the lynchpin? (operator question, answered)

Half right. Sharding is the lynchpin for a **large commons held by many** — without it a commons is held only
by nodes that can hold all of it (racks, a few stewards), which is tolerable now and wrong for the phone-to-rack
spectrum. But **"shared between holons" is not what sharding gives you.** Cross-holon sharing comes from
membership and promotion, available today: a data commons is a space everyone joins (most at zero arc) and
content enters it by witnessed re-publish. Sharding only decides how widely the commons is *held* once it is
there. Hence the order: (1) placement — fixtures and households out of the commons, no fork; (2) the small fork
hook — per-space arc, settable hot, so one phone is a full household member and a light commons reader; (3) true
fractional sharding — the conductor filtering ops by arc, routing gets to out-of-arc authorities, keeping
validation coverage honest as arcs shrink. Upstream built (3) once and deferred it in the rewrite; their roadmap
intends to restore it in kitsune2. Track and contribute rather than rewrite alone; (1) and (2) are what unblock
the pain we feel now.

## 7. What changes in our design, concretely

This is the decision the overnight ruling recorded on the arc-policy code and in the Nachalah hub, spelled
out.

**The placement rule:** *content declares its space; a space's members hold it at the arc the trust
gradient allots; arc inside a space is a per-space {full, zero} decision today and a fork optimization
tomorrow.* An EPR's reach/holding declaration is the **input**; space placement is the **output**.

**What that means for each plane:**

| Plane | Today | After |
|---|---|---|
| Content (`lamad`) | one space, full arc for all | commons space (stewards full arc, readers zero) + collective spaces + household spaces + a fixtures space |
| Reach | enforced at read time in storage | enforced by membership; read-time check remains as defense-in-depth for the commons |
| Holding floor | a global replica target | per holon: "held by ≥ r of this holon's members" (the Nachalah gold/deeds/paper floors) |
| Promotion | copy + flag | witnessed re-publish into the wider space; CID unchanged, anchor per space |
| Arc policy | shed a peer to zero on OOM | choose which spaces a device joins and at what arc; OOM becomes "too many full-arc memberships," a social/allotment question |
| Upgrade propagation | per role | per **cell**: a coordinator release must reach every clone of a role. The adoption controller's "installed reality" becomes per cell. One adjustment, already in scope of the Evolution epic. |
| Fixtures | in the commons | in their own clone space; only the test runner follows it |
| Doorway | projects "the" content | projects the spaces its steward belongs to |

**What does NOT need to change:** identity (CID), the blob plane, heads and elections, release channels,
the coordinator hot-swap, the doorway as projection, the a2o stories (they already speak in households and
collectives). The stories were ahead of the substrate.

**The first slice (no fork, days not months):** a `fixtures` clone of the content role with its own seed;
the seeder targets it; fleet peers do not follow it at full arc. That alone removes thousands of EPRs
from every peer's gossip and is the cleanest possible measurement that the model is right.

**The second slice:** household working notes into household content spaces, using the seeds the mishpat
role already mints. The `reach-enforced-everywhere` habit's outsider scenario becomes true *by construction*.

**The fork slice (small, and not optional for phones):** plumb the *per-space* arc hint the conductor
already ignores, so one device can be full-arc at home and zero-arc in the commons. Until that lands, a
phone is a full member of every space it joins or a reader of all of them — so phones stay spokes to a
household hub, which is what the seam map already draws. Fractional arcs inside a big space are a
separate, larger item (see §10).

---

## 8. How to explain it to someone else in two minutes

> Holochain isn't one big shared database. It's a way for a *group* to keep a shared, tamper-evident
> shelf of records that every member helps hold and check. Each group is its own little network, called a
> space, and the rules the group agreed to *are* the network's name. Your phone can be in lots of spaces
> at once: your family's, your church's, the public commons. It only holds and gossips the ones it's in.
>
> We built Elohim on that, but for the first stretch we put everything into one space and asked every
> node to hold all of it, then wrote a lot of code to cope. The fix is to do what the design wanted:
> households get a space, collectives get a space, the commons gets a space, and a document's "reach" is
> really "which of those spaces it's been published into." Moving a note from your family to your church
> is a witnessed act, not a flag. That's what makes it scale to everyone: each space is family-sized or
> church-sized, and nobody ever holds a stranger's journal.
>
> The one thing we still want from a fork is letting many small holders share a *big* public space by
> each holding a slice. Nice to have. Not the architecture.

---

## 9. The five open questions, answered

Each of these was open when the first draft of this document went out. Each was then grounded by a
reader against the code and the corpus (file references are in the commit that added this section), and
each now has an answer in the shape *"if holons are spaces, then ___, so this suggests doing ___."*
Where an answer is a theory rather than a measured fact, it says so.

### 9.1 Holon granularity — what should be a space?

**What we found.** The deployed hApp manifest sets `clone_limit: 0` on every role, with a comment that
says "default-deny; revisit when per-household infrastructure…" So there are no clone spaces because we
told the conductor to refuse them. Meanwhile the *vocabulary* for holons already exists, three times over:
a locality axis (private → bioregional → commons), an affinity axis (personal, household, congregation,
denomination, professional, interest group, open), and a projection scope that already names
`commons | qahal:<id> | household:<id>`. A `Collective` is already the one governance-context record,
with "household" as its special case. We had the words and refused the spaces.

**The reasoning.** A space costs one cell per device per role, and cells are the conductor's unit of
memory (see 9.3). A space buys three things: privacy by membership, a holding floor its members can
actually meet, and validation by people who agreed to the rules. So a space should be *the smallest group
that can validate its own writes and afford its own recovery*. Per document is refused outright: it
buys nothing and multiplies cells. Per affinity is refused as a rung: a denomination can be a million
people across every region, which is a commons, not a middle layer.

**If we did it this way, then** a person carries roughly five kinds of space and ten to twenty in total:

| Rung | Size | Who holds it fully | Floor |
|---|---|---|---|
| Device | no space; private records stay on the source chain | the device | — |
| Household | 2–10 devices | every device | survives any one device loss |
| Collective | 30–10,000 members | 3–7 *independent households* as holding domains; readers at zero arc | three evidenced domains |
| Commons per language/region | millions of readers | tens to hundreds of stewards; the fork's fractional arcs later | seven diverse hubs |
| The protocol commons | everyone | rack-tier stewards | seven diverse hubs |

Affinity and locality become *filters on records inside a rung* (`{space: commons-en, affinity:
denomination}`), never rungs of their own.

**And per role:** content and governance go **per holon** (a household's pages and its commitments are
its own business). Identity, infrastructure and node registration stay **global**, and the reason is the
best argument in this whole document: a recovery quorum is deliberately made of people *outside* your
household (the "grandma case" in the identity zome, the backup custodian who may not read the note). A
household space cannot validate its own rescue. Identity must be readable across the membrane.

**So this suggests doing:** raise `clone_limit`, mint a `fixtures` clone of the content role, and let no
fleet peer follow it. It is the cheapest possible falsification of the whole model, it needs no fork,
and it removes thousands of records from every peer's gossip on the day it lands. Two landmines from the
field (§11): keep the role *provisioned* and just raise `clone_limit` — the `clone_only` strategy leaves
the role unprovisioned and the 0.7 conductor panics assembling app info; and the `deferred` flag is
ignored on install. A provisioned role with a clone limit in the hundreds runs in production elsewhere.

### 9.2 Membrane proofs — who may join a space?

**What we found.** Holochain has the mechanism end to end. A joiner supplies a *membrane proof* (bytes)
when a cell is installed or cloned; the conductor runs the integrity zome's `genesis_self_check` on it
*before* the source chain is created; a rejection means the cell is never born; an acceptance writes the
proof permanently as the second record of the chain, where any later validation can inspect it. And
every one of our integrity zomes implements that callback as a stub that returns *valid* for anyone. Our
hApp manager never passes a proof; the lineage install writes `None` explicitly.

**What we already have that is proof-shaped.** Two things: the cross-signed binding between an agent
key and its transport identity (the `identity-cross-signed` habit, red today, observe-only), and mishpat's
`verify_credentials`, built for "collective membership CIDs presented by connecting peers," which checks
that a record exists and who authored it.

**If we did it this way, then** joining a household or collective space would require presenting a
signed membership record that the space's own rules can check without asking anyone. Concretely: the
membership is a mishpat commitment (the household affirmation the Nachalah stories already describe),
signed through the existing conductor signing path; those bytes travel as the membrane proof on the
clone call; the space's `genesis_self_check` deserializes them and checks the signature against the
space's founding rule ("signed by a key this household's affirmation names"). Outsiders who know the
network seed are refused at genesis, which is exactly the "outsider knows the seed" scenario in the
allotment story.

**So this suggests doing:** three small pieces, in order. Replace the stub `genesis_self_check` in the
content and governance zomes with a real check of a membership record; add the one code path that turns
a mishpat membership into proof bytes; pass it on clone creation. None of it moves the DNA hash of the
*commons* space (the commons stays open); it only gives holons a door. One field warning (§11): the self
check is a courtesy; the only real gate is validating the proof record on the chain, and Sweettest has no
support for membrane proofs at all — so the test harness for holons must be decided *before* the door is
built, not retrofitted.

### 9.3 Cell count per device — what does a holon cost?

*(Measured on 2026-09-06 with an isolated stock holochain 0.7.0 sandbox; numbers below are from that run.
See the table and commands in the commit that added this section.)*

MEASUREMENT_PLACEHOLDER

### 9.4 Migration of the existing commons — does anything have to move?

**What we found.** The content space holds 3,442 rows on doorway-A this morning and there is no
by-kind breakdown route; the a2o fixtures plant thousands of records per run and nothing ever removes
them, so fixtures may already outnumber real content. The seeder knows nothing about spaces: every seed
path writes to the base cell. And the crossing machinery built for the Evolution epic is *lineage-only*:
it moves one role from an old DNA hash to a new one, same role, same purpose. It is the wrong shape for
"re-publish these records into space B and sunset them in A."

**If we did it this way, then** the current content space simply *is* the commons and stays where it
is. Nothing real needs to move. Two things stop being written there: fixtures (which go to their own
clone from the next seed run onward) and new household notes (which go to household spaces from the day
those exist). Old fixtures already in the commons are an honest residue; a one-time sunset of records
authored by the seeder's fixture identities is a cleanup, not a migration.

**So this suggests doing:** give the seeder a cell selector (it has none), make the fixtures clone the
default target of every a2o seed run, and never ask fleet peers to provision it. Household notes need no
crossing machinery at all: new notes go to the household space from now on.

### 9.5 Elohim as the cross-holon plane — what lives between membranes?

**What we found.** A great deal of our system already lives outside every DHT space, on purpose: the
steward-peers pool, the conductor registry, the upstream circuit breakers, the self-healing read model,
the transport-manifest bootstrap cache, the freshness pantry, the coherence fingerprint; on the storage
side the reconcile controllers, the arc policy and actuator, head adoption, and the *follow-set* of the
release controller (the elected head is notarized; the decision to follow it is node-local). The
federation-failover plan's whole gap list is likewise non-DHT. This is not debt. It is the inversion
the seam map names: the social, governance, trust and recovery plane has no hyperscaler equivalent, and
no single space can hold it because it is *about the relationships between spaces*.

**If we did it this way, then** five things follow, each of which resolves something that has felt like
a bug:

1. *A doorway projects exactly the spaces its steward joined.* The steward-peers pool stops being a
   hand-edited list and becomes a derived membership roster; "the doorway can't see a household" becomes
   correct behavior.
2. *Discovery is a social graph in identity, not a DHT lookup.* This is why identity and infrastructure
   stay global (9.1): a per-holon identity space would make holons unreachable.
3. *A recovery quorum is a cross-space commitment recorded in the custodian's governance space, not the
   household's.* The custodian must validate it without joining the household, and the allotment story
   says the custodian may not read the note.
4. *A release channel becomes per cell, not per role.* Following is already a node-local act (james's
   promotion to canary this morning was a data change on his row alone); the adoption controller's
   "installed reality" must enumerate clones.
5. *The hub cluster is the commons' rack-tier stewards*, and per-space arc is the only way a phone is a
   full household member and a light commons reader at once, which is the fork hook of §6½.

**The design rule that falls out:** *inside a membrane, the DHT is the truth; across a membrane, the
elohim plane carries a witnessed commitment recorded in the receiving holon's governance space.* The
aggregation plane's node-local state is correctly placed. The work is to make it *derive from
membership* rather than from config.

**So this suggests doing:** nothing new in the DHT. Derive the steward-peers pool from space
membership once spaces exist, record recovery quorums and promotions in the receiving holon, and let
the release controller enumerate cells. Each is a small change in code that already exists.

### 9.6 What this section decides, in one breath

Spaces are memberships with their own rules, sized household → collective → regional commons → protocol
commons; content and governance per holon, identity and discovery global; a door on every holon made
from a signed membership record; nothing in the commons moves except fixtures and new household notes;
and the elohim plane stays where it is, deriving from membership. First move on all of it: raise
`clone_limit`, mint the fixtures clone, measure the difference in gossip.

## 10. The red team — a Holochain reviewer runs us through it

*The operator asked for a hostile review from a Holochain core contributor's point of view before
committing to any pivot. What follows is that review's findings, each followed by the orchestrator's
verdict. Where I disagree I say so; where I accept, the answer above is amended.*

### 10.1 "You screwed up the layout, not the architecture — except for one thing."

**Finding.** One space, full arc, fixtures in the commons, `clone_limit: 0`: all layout, all reversible in
days, and every serious hApp team (Acorn, Moss, Neighbourhoods) shipped single-space first before cloning per
group. The three irreversible choices — CID as identity, blobs off the DHT, integrity/coordinator discipline —
are right, and they are why a pivot is available at all. **But** the content integrity zome is an omnibus:
about 4,800 lines, **75 entry types and 225 link types**, spanning learning, community, identity, economy
and infrastructure, and it **duplicates identity's own types** (Human, Agent, Relationship, ContentMastery,
ContributorPresence appear in both DNAs, the content copies marked "legacy"). Two networks are authoritative
for the same fact, so neither is. That is an architecture error, not layout. Alongside it: every DNA's
membership check is a permissive stub and no role commits to a progenitor, so every space today is an open
network whose hash binds to no root.

**Verdict: accepted, and it changes the order of everything in §7.** Cloning the content role as it stands
would clone the omnibus into every household. The split is one lineage crossing per seam, and the crossing
machinery is exactly what the Evolution epic just proved on the mesh. So: **the zome split comes before any
household space.** §9.1's role table stands; the content role that goes per-holon is the *split* one.

### 10.2 "Your storage layer is legitimate — with two organs over the line."

**Finding.** A projection is legitimate when it is rebuildable by replay, never originates truth, and fails
to staleness rather than divergence. By that test the storage layer is a proper peer-hoster. Two organs are
over the line: **read-time reach enforcement** (every full-arc peer *holds* the household bytes; only our
code declines to serve them; patch the binary and serve everything — that is authorization outside the
validated substrate, not caching) and **per-host custody stamps**. Everything else the reviewer would defend
to upstream, including the honest `{0,1}` arc actuator and the node-local follow-set ("following is consent,
and consent is node-local"). The number to worry about: roughly 330,000 lines of storage against a 4,800-line
integrity zome. Design gravity has been outside the DHT for two years, and the pivot *increases* it, because
cross-space reads by CID become storage-side joins.

**Verdict: accepted, with one framing change.** §5 called read-time reach "necessary today"; it is more
honest to call it *an authorization boundary that membership will retire*. And the reviewer is right that the
pivot does not shrink storage — it moves storage from *shedding load* to *joining across membranes*, which
is the job the elohim plane was always going to have (§9.5). The projection DB stays, and it is what keeps the
commons usable offline.

### 10.3 "The pivot is right. Six things will bite."

1. **The per-cell cost was unmeasured** when this document went out (9.3 was a placeholder). Community
   experience: clone-per-group works around 5–20 cells with *small* zomes and hurts as cells grow heavy.
   Ours is heavy (10.1). *Verdict: measure before promising; 9.3 now carries the numbers.*
2. **A membrane proof cannot do a DHT read** — `genesis_self_check` runs before the chain exists, so the
   proof must be checkable against something in the DNA itself. *Verdict: accepted; 9.2 already says so.*
3. **Anything in DNA properties folds into the hash.** A per-household founding key means each household is a
   distinct DNA hash, not merely a distinct seed; "per cell" release channels are really "per DNA hash," and
   the hApp manager's stale-check is role-structure-only today. *Verdict: accepted as a consequence, not a
   blocker — clones share integrity wasm, the coordinator hot-swap already applies per cell, and the adoption
   controller must enumerate cells by role. It is a real change and it is now in §7's table.*
4. **Cross-space reference by CID kills link traversal**; every cross-holon read is a storage-side join.
   *Verdict: accepted (see 10.2).*
5. **Per-space arc is fork-gated, and §7 had contradicted §6½ by calling it "later, separable."** A phone in
   fifteen spaces is full-arc in all or zero in all until the hint lands. *Verdict: accepted; §7 corrected
   above. Phones stay spokes to a household hub until then.*
6. **Blocks scale with spaces.** On our own mesh one rejected write after a seal blocked a cell permanently,
   with no unblock. Multiply that by every household. *Verdict: accepted; this is why the unblock API moves
   ahead of any household space (10.5).*

### 10.4 "On the fork: three patches, each with an open upstream PR — and refuse sharding."

**Finding, per change.** Cross-relay fix: upstream it. Jemalloc: an image choice, not a patch. Sys-validation
backoff: should be a config knob, PR the knob. A `list_blocks`/`unblock` admin API: **the best contribution we
have**, small, genuinely missing upstream, retires our worst risk. Per-space arc hint: small plumbing of a
field the conductor already ignores, PR-able. **Fractional sharding: refuse.** Upstream built dynamic arcs
once, removed them in the rewrite, and has restoring them on the roadmap; a small team re-implementing op
filtering by arc, out-of-arc reads and validation-coverage accounting on a moving 0.x line is a multi-year
commitment against a target that will be rebuilt under it. Discipline: at most three carried patches, each
under ~200 lines, each with an open PR; and stop pinning the fork submodule as reference-only so it can be
bisected against upstream.

**Verdict: accepted in full.** This overrides the softer "track and contribute" wording earlier in §6½: we do
not attempt fractional sharding. We carry the unblock API and the per-space arc hint as PRs first, fork
second.

### 10.5 "What a person feels."

Joining a household must feel like tap-accept on an invitation; if anyone ever sees proof bytes we have
failed. **Promotion gets better**: "share to the church" becomes a visible, witnessed, attributable act rather
than a flag — lead with it. **Reading the commons at zero arc gets worse**: no local authority, tail latency on
network reads, offline broken — the projection DB and pin-what-you-read are what hide this, and pinning is
content-level caching, never to be confused with arc. **A phone in fifteen spaces is the weak point** until
the per-space hint lands: keep phones as spokes to a household hub, as the seam map already draws. **Recovery
by people outside the household** is the strongest argument in the document; identity never goes per-holon.
Hide entirely: seeds, DNA hashes, arcs, cell counts, proof bytes, re-publish mechanics. Show: who can see
this, who holds a copy, who witnessed the move.

**Verdict: accepted as the UX contract for the epic.**

### 10.6 The recommendation, resequenced

The reviewer's option: the §9 pivot, in a different order than §7 first proposed. Household spaces before an
unblock API is a support catastrophe; per-holon spaces before the zome split clones the mess. So:

1. **Fixtures clone, instrumented.** Raise `clone_limit`, give the seeder a cell selector, no fleet peer
   follows the clone — and measure per-cell cost the same week (9.3). Risk: low; the only way to fail is to do
   it without instruments.
2. **`list_blocks` / `unblock`: upstream PR plus a carried patch.** Risk: medium (fork discipline). Nothing
   downstream — no household space — before it lands.
3. **The omnibus split as one lineage crossing**: shed the identity duplicates, the economy and the
   infrastructure types from the content zome, and in the *same* hash move land a real membership check, a
   lineage record and a non-null progenitor. Risk: high — a hash move on the fleet's largest space, with a
   reinstall path that still mints keys — but it is what the Evolution epic exists for, and doing it after the
   pivot means doing it per household.
4. Only then: household spaces, with membrane proofs.
5. Explicitly not: fractional sharding.

**Verdict: adopted as the plan.** §7's "second slice" (household notes) moves to step 4.

### 10.7 The reviewer's verdict, verbatim

> You did not screw up the architecture — you screwed up the layout, and then you built 330,000 lines of
> storage to make the layout survivable, which is what made it look like architecture. The three choices that
> would have been fatal to get wrong (CID identity, blobs off the DHT, integrity/coordinator hygiene) you got
> right, and that is why the pivot is available to you at all. The one genuine architecture error is the
> 75-entry-type omnibus integrity zome that duplicates your own identity DNA — fix that *with* the crossing
> machinery you just proved, *before* you multiply spaces, or you will clone it into every household on Earth.
> The pivot is correct and its first slice is cheap, but do not create a single household space until the
> conductor can unblock a cell, do not sell the phone-in-fifteen-spaces story until the per-space arc hint
> lands, and do not attempt fractional sharding at all. Ship the fixtures clone this week with instrumentation,
> PR the unblock API, split the zome, and leave the shadow-database question alone — the two organs that are
> actually over the line are read-time reach enforcement and per-host custody stamps, and membership will
> retire the first one for you.

## 11. Field guides we adopt, adapt, and still have to write

Two days before this document, Sacha Pignot (hAppenings Community) published `holochain-agent-skills`
(Apache-2.0): one skill with twenty references, eight workflows, seventeen templates and a compiling example
hApp, pinned to the same HDK and HDI versions our DNAs use, with citations into the crate sources and a CI
gate that fails when a document teaches a removed API. His own app is single-space with `clone_limit: 0`, so
the cloning and membrane material is upstream-derived rather than app-proven, but it is careful.

**Adopt as-is (planted as a package with attribution):** membranes, source chain, countersigning, testing
(multi-conductor Sweettest and partitions), the 0.6→0.7 upgrade break list, troubleshooting, cryptography,
scheduling. **Adapt:** cell cloning (take the manifest mechanics and the `clone_only` panic verbatim; add our
holon vocabulary and the "a cell is a full participant" cost rule), and the zome-review checklist (take the
API items; *drop* its default of a path-plus-agent discovery link on every entry, which is the query-index
link pattern our link budget refuses — the content zome already sits at 225 of 256 link types). **Write
ourselves, because nothing exists:** per-space arc policy (his networking reference stops at the global knob
and does not know about the clamp or that the factor is conductor-wide), the head-plane cost model, holon
placement and promotion by witnessed re-publish, and cross-space reference by CID. **Contribute back:** the
arc clamp and the conductor-wide finding, which his repository explicitly asks for.

*Grounding for this document: the fork conductor at the fleet pin (`elohim/holochain-conductor` 25dd2d0be,
`crates/holochain_p2p/src/local_agent.rs:133`), `kitsune2_api` 0.5.1 `DhtArc`, the alpha peer store via
`GET /db/p2p/conductor-diagnostics` (five spaces, 32/35 full-arc entries), doorway-A `GET /db/stats`
(3,442 content rows), the Nachalah allotment epic (tiers on DNA seams, arcs negotiated by the elohim), the
Holochain Evolution epic §5 (dual-cell bridging) and §11.3 (arc-policy hook), the arc policy and actuator
modules in elohim-storage, and the 2026-09-06 ruling recorded on `services/arc_policy.rs`.*
