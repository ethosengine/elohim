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

**Where the household spaces exist, they are the exception.** The mishpat role *does* mint household
spaces with their own network seeds (that is the `network seed` the Nachalah acceptance stories
reference, and the "household space partition" incident of 2026-09-05 was a household space getting its
cell blocked). So the primitive is in use — but only for governance records, not for content, and not
as the general placement rule.

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
5. **Household spaces for governance.** The mishpat role already mints per-household spaces with seeds.
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

**The fork slice (later, separable):** lift the arc clamp and land the arc-policy hook on the fresh 0.7
fork, so a large commons can be held at fractional arcs by many small stewards instead of full arcs by a
few racks.

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

## 9. Open questions the epic now owns

1. **Holon granularity.** Household = 1 space per family; collective = 1 per organization; commons = 1
   global, or several by language/region? The seam map's device spectrum (watch → rack) suggests the
   commons needs the fork's fractional arcs sooner than the others.
2. **Membrane proofs.** Holochain supports a proof-of-membership at join. We have imagodei attestations
   and household affirmations; wiring them as membrane proofs is the natural "who may join this space."
3. **Cell count per device.** A person in 40 holons runs 40 cells per role. Measure conductor memory per
   idle cell before promising watches and phones anything.
4. **Migration of the existing commons.** The current `lamad` space *is* the commons; nothing moves out of
   it except fixtures and household notes. Good: the big space stays where it is.
5. **Elohim as the cross-holon plane.** Aggregation, routing care, recovery quorums — all of it spans
   spaces by design and none of it is in the DHT. That was always the intended split; it now has a
   reason written down.

*Grounding for this document: the fork conductor at the fleet pin (`elohim/holochain-conductor` 25dd2d0be,
`crates/holochain_p2p/src/local_agent.rs:133`), `kitsune2_api` 0.5.1 `DhtArc`, the alpha peer store via
`GET /db/p2p/conductor-diagnostics` (five spaces, 32/35 full-arc entries), doorway-A `GET /db/stats`
(3,442 content rows), the Nachalah allotment epic (tiers on DNA seams, arcs negotiated by the elohim), the
Holochain Evolution epic §5 (dual-cell bridging) and §11.3 (arc-policy hook), the arc policy and actuator
modules in elohim-storage, and the 2026-09-06 ruling recorded on `services/arc_policy.rs`.*
