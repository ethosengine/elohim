---
title: Holons, Spaces, and Holochain — What the Vision Requires Us to Reconsider
id: holons-are-spaces-how-we-use-holochain
date: 2026-09-06
status: reference
author: orchestrator (overnight shift, 2026-09-06; grounded in the 0.7 fleet line at fork 25dd2d0be and kitsune2 0.5.1)
tier: architecture
stewardship-frame: adversary
cites:
  - "elohim-protocol-manifesto | The purpose governing this reassessment: intelligence and coordination that scale to human complexity while restoring dignity and resisting capture. | sha256:c1b65508df47bcaa | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "values-forward | The declared constraints on commons enclosure, accountable trust, concentrated power, and phased AI authority that the technical design must honor. | sha256:80a6f4eeeefa1ffd | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "hardware-spec | The physical participation and inclusion vision connecting everyday devices and household capacity to the protocol promise. | sha256:230d54b7e8ad2df2 | path: genesis/docs/content/elohim-protocol/hardware-spec.md"
  - "hardware-providence-commons | The existing proof obligations for dependable household operation, bounded automated care, practical substitution, and resistance to rent extraction. | sha256:17e52609abf5f92a | path: genesis/docs/content/elohim-protocol/hardware-providence-commons.md"
  - "resilience-protocol-spec | The convenience requirement: ordinary people must receive dependable services through reciprocal infrastructure without becoming system administrators or captive tenants. | sha256:5d5f1f85fe7dcfe2 | path: genesis/docs/content/elohim-protocol/resilience/README.md"
  - "genesis/docs/content/elohim-protocol/architecture/social-reach-nervous-system.md"
  - "trust-as-efficiency-signal | Trust as reciprocal reduction of distribution and verification cost, distinct from a permanent privilege or a global score. | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - "ubiquitous-wisdom-dissolves-chokepoint | Distributed judgment at authoring, relay, and consumption as the anti-capture deployment thesis behind social reach. | sha256:ad2345f8adb56ee9 | path: genesis/docs/content/elohim-protocol/architecture/ubiquitous-wisdom-dissolves-chokepoint.md"
  - "genesis/docs/content/elohim-protocol/social_medium/epic.md"
  - "nachalah-allotment-epic | Nachalah | sha256:855a5cb52df7f201 | path: genesis/docs/superpowers/specs/2026-09-05-nachalah-allotment-epic-design.md"
  - "holochain-evolution-epic | Holochain Evolution Epic | sha256:d821c5f45fd5d2e5 | path: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:fd5ced9f996ff5af | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "conductor-authority-arc-auto-policy | 2026-06-13-conductor-authority-arc-auto-policy | sha256:597157e7bb552d73 | path: genesis/docs/superpowers/specs/2026-06-13-conductor-authority-arc-auto-policy.md"
---

# Holons, Spaces, and Holochain — What the Vision Requires Us to Reconsider

*An architecture explainer reopened for product refounding on 2026-09-06. Start with §0 and §0a:
they distinguish the human purpose from the proposed technical answer. The later measurements and
review history remain evidence to examine, not proof that the proposed experience is right.
Fleet numbers are the original author's dated observations unless marked otherwise.*

---

## 0. Start at the manifesto

The [manifesto](../manifesto.md) asks for digital infrastructure organized around human flourishing:
people able to learn, care, create, remember, and repair their shared lives without having those lives
turned into someone else's instrument of accumulation. Its ambition is intimacy and dignity at the
scale of humanity. The hardware, network, and AI architecture must deliver that ambition in forms
ordinary people can actually use.

It starts from a present crisis: people trying to understand and act together are overwhelmed by
industrialized falsehood, attention extraction, and technologies that concentrate power over their
shared reality. Adversarial uses of AI belong to that operating environment. The manifesto's concern
is the loss of the conditions for shared understanding and agency, including their conversion into
instruments of authoritarian control. The protocol exists to make another way of living credible,
available, and attractive.

The [hardware specification](../hardware-spec.md) makes the promise physical. A browser user, a person
with only a phone, and a household tending a rack should all receive capable infrastructure through
community-grounded participation. The convenience people expect from a hyperscaler, and the power
operators expect from cluster orchestration, must become dependable everyday services: keep our
records safe, run our applications, recover from failure, coordinate available resources, and keep
within our limits. Participation must not require everyone to become a systems administrator.

**The telos is to put that operational power and applied intelligence into people's hands through
infrastructure their communities can steward, and let it compose to planetary scale without creating
a point from which anyone can dominate, enclose, or charge permanent rent on their participation.**

Integrity is constitutive of that promise: people can investigate a claim, receive a correction,
preserve an accountable shared memory, and act together without surrendering their judgment or
dignity. That experience must also be useful, welcoming, and dependable enough to choose in daily
life. It cannot require permanent vigilance or technical expertise from every participant.

Social reach is central to that composition. The protocol already proposes earned distribution,
relationship-grounded responsibility, distributed discernment, and feedback that travels back along
propagation relationships. Those mechanisms are part of how trust becomes a scaling resource and
manipulation becomes accountable. They cannot be reduced to which Holochain space holds a record.

This document originally answered a narrower problem: too much content in too few Holochain spaces,
held at full arc. Spaces remain an important candidate remedy. But the design question is larger:
**how do we aggregate capacity, knowledge, and coordination without aggregating an unaccountable
power over the people who make them possible?** In this review, the immediate question is what
Holochain contributes to that promise, what its use costs the daily experience, and whether we must
change its role substantially—or replace it. The alternative must work within this present reality;
we judge the technical choices by how they make its integrity and usefulness real.

## 0a. Product refounding — what Holochain must earn, and what we must change

### The chain of reasoning we need to restore

Read the argument in this order. Each step supplies a requirement for the next:

1. **Manifesto:** human flourishing, accountable biography, care, and dignity; the people overlooked
   by existing systems are the audience of first resort. Intelligence should scale to human
   complexity. [Values Forward](../values-forward.md), especially Stances I.3 and II.1–II.4, makes
   the constraints explicit: capability must arrive in people's lives, trust must remain accountable,
   and neither capital nor AI acquires final authority over the commons.
2. **Hardware and inclusion:** the [hardware spec](../hardware-spec.md) describes the participation
   gradient. Its status is **Vision envelope**, not a demonstrated device-capability matrix. The
   [Hardware Providence companion](../hardware-providence-commons.md) names the proof: a bounded
   operator carries sensing, action, verification, and recovery while households retain authority
   and can change suppliers. Hardware capacity varies; human standing is not purchased with a rack.
3. **P2P capability:** the [resilience argument](../resilience/README.md) explains why cloud
   convenience must survive changing the custodian. The [seam map](./2026-06-21-elohim-seam-map-concern-routing.md)
   locates the mechanisms across the notary, byte transport, projections, hub orchestration, and
   resource governance. The map is a capability destination, not evidence of delivered cloud parity.
4. **Social reach and trust:** [Social Reach](./social-reach-nervous-system.md) and
   [Trust as an Efficiency Signal](./trust-as-efficiency-signal.md) explain how participation earns
   wider distribution and how accountable relationships reduce repeated work. This is a distinct
   scaling mechanism, alongside partitioning data and adding hardware.
5. **Aggregation with recourse:** larger coordination must preserve the evidence, obligations,
   limits, and paths of correction beneath its summaries. A larger aggregate must not become a
   landlord over its constituent communities or a dossier of their private lives.
6. **Primitives and adversarial proof:** choose spaces, content addressing, witnessed commitments,
   sync, coverage rollups, and deployment arrangements by whether they sustain those promises under
   failure, growth, manipulation, and attempted capture. The primitive earns its place through proof.

This is the level set. The vision is the fixed point; the present mapping from it to implementation
is open for correction. The previous revision began with privacy and consent journeys. Those remain
necessary, but they are tests within this larger purpose, not a substitute for it.

### What hyperscaler convenience and k8s-like power mean in a household

The human request is “keep this dependable within the commitments we made.” The system carries the
operational detail. These are experience requirements mapped to the existing architecture, not new
APIs or a claim that a complete household orchestrator ships today.

| What a person should be able to rely on | Existing architectural home | The authority boundary it must preserve |
|---|---|---|
| Our applications remain available and recover when a device fails | Hub process, cluster, and pod coordination in `steward/node`; runtime supervision | Permission to restart or place a workload does not grant authority to change its purpose or read its records |
| Our records have recoverable copies and follow us to another provider | Content addressing, custody and reconciliation in `elohim-storage`; confidential replication | Holding bytes does not grant readership, canonical-version authority, or a toll on departure |
| A task can use suitable shared capacity within our budget | Capability-bound commitments, operation authorization, resource bounds, and placement | More supplied compute does not buy more governance power; delegated work stays within its mandate |
| We can find and use services without depending on one company | P2P discovery and transport; doorway web projection | A discovery or gateway service must not become the necessary broker of identity, reach, or recovery |
| Updates and repairs complete without losing our continuity | Witnessed release adoption and the Hardware Providence care loop | Publishing an artifact or recommendation is distinct from authorizing its adoption or a destructive action |

The k8s analogy has a precise boundary. Coordination **within a hub** concerns its blades and
processes. Cooperation **between households** concerns independently governed participants and their
commitments. The seam map locates these in separate hub-internal and inter-hub swarms. A planetary
aggregate is not a cluster administrator entitled to schedule arbitrary work on every household.

Current source makes the gap visible: `steward/node/src/cluster/{discovery,leader,membership}.rs`
are TODO stubs. Storage reconciliation and bounded operator verbs provide real pieces, but the
complete observe–authorize–act–verify household loop remains a proof obligation. The failed-SSD
journey in Hardware Providence is a stronger test of convenience than another healthy-service badge.

### What job are we actually asking Holochain to do?

Holochain's candidate role is **agent-authored, tamper-evident records with peer validation under
shared integrity rules**. A source chain records one agent's actions; a cell is that agent's
participation in one DNA space; the DHT distributes and validates the relevant public operations.
There is no single global transaction order. The DNA hash includes integrity code and modifiers,
including the network seed. Coordinator behavior can change without changing that identity.

Those mechanisms can witness authorship, commitments, provenance, and rule-governed changes. They do
not by themselves decide whether a social claim is wise or true, provide a complete human identity
and recovery experience, encrypt off-DHT custody, schedule a workload, or operate a household rack.
“Notarized” must say precisely what was checked. A valid signature is not evidence that an observed
need was real; a validated entry is not proof that a repair happened; a source-chain sequence is not
a global consensus about events.

Our current use is concrete: five hApp roles, cloning disabled in the manifest, permissive content
admission, a large content integrity zome spanning several domains, and the full-or-empty arc surface
described in §6½. Much of the day-to-day product runs through storage projections, CRDT sync, the
blob plane, and doorway access. We need to explain why each consequential act crosses the notary,
what the crossing earns, and what a person can still do while it is unavailable or catching up.

| Technical boundary to explain | What the person experiences | The design question it forces |
|---|---|---|
| Source-chain authorship and eventual DHT validation | “I wrote it” may precede “others can verify or retrieve it” | Which actions need witnessing before their effects are relied on, and how do pending or rejected actions remain understandable? |
| Cell admission and DNA-scoped validation | An invitation must establish a real relationship and usable access | Which community agreements belong in shared integrity rules, and which membership or consent changes must remain possible without a disruptive migration? |
| Off-DHT records, blobs, and serving projections | A page can be available, stale, unwitnessed, or unreadable to a particular person | Which plane is authoritative for each claim, and what prevents a convenient projection from silently becoming that authority? |
| Multiple cells and spaces on bounded hardware | A phone's responsiveness, battery, offline access, and dependence on a hub | What must run locally, what can be delegated, and does the same person retain continuity when that arrangement changes? |
| Integrity changes versus coordinator updates | An upgrade should not unexpectedly divide a community or replace its identity | Which invariants truly deserve a DNA-hash boundary, and what migration evidence is required before changing them? |
| Cross-space references and social reach | A contribution, correction, or commitment travels between communities | How do provenance, currentness, consent, and responsibility survive that crossing without a central joining service becoming indispensable? |

These are the seams to handle honestly. A missing feature in our use of Holochain, a constraint of
our pinned conductor, and a mismatch between Holochain's model and our desired experience are three
different findings. “Holochain supports clones” answers none of them on its own. Conversely, a large
storage crate is not proof that Holochain failed: bytes, fast queries, orchestration, and contextual
judgment were never all the same job as peer validation.

### Serious alternatives belong on the table

The following are alternatives to investigate against the same journeys, not a migration decision:

| Direction | What it would change | What it must establish before we choose it |
|---|---|---|
| Use Holochain more deliberately | Reconsider integrity-domain boundaries, admission, and space placement; measure cloning with the actual workload | The model improves lived continuity and bounded cost without multiplying migrations or confusing membership with social reach |
| Give Holochain a narrower notary role | Reassess which enduring facts need peer validation, reusing private, linked, ephemeral, and derived classifications where appropriate | We can identify the authority behind every consequential claim and avoid both notarizing every transient observation and treating mutable projections as truth |
| Replace Holochain for some or all of that role | Change the witnessing substrate while retaining the purpose and portable record contracts | An alternative actually supplies the required authorship, integrity, validation, currentness, revocation, discovery, and recovery guarantees, with a credible migration and operating cost |

The first two can overlap. The third is a legitimate outcome if evidence warrants it. This document
does not yet compare a concrete replacement implementation, so it cannot responsibly select one.
Content addressing and portable EPR envelopes give us room to investigate; they are not a replacement
for shared validation or currentness. Signatures and P2P byte transport alone do not close those gaps.

**Holochain has to earn its role by making the vision easier to deliver.** Continuing to use it is
not the objective of this review. Nor is removing it. The objective is a coherent account of what
the substrate guarantees, how a person experiences those guarantees, and what we must change when
the two do not line up.

### Social reach is an active scaling mechanism

The social-reach canon explicitly distinguishes reach from geography and visibility. Its contract
must survive any space redesign:

- **Earn at authoring.** A declaration of wide reach is a claim to substantiate through the graph
  of relationships, standing, and responsibility before publishing at that scope. Cheap generation
  must not automatically create a claim on everyone's bandwidth and attention.
- **Pre-authorize reciprocal service.** Standing relationships and stewardship agreements let peers
  take responsibility for distribution, discovery, and validation with less repeated work. They
  provide an efficiency benefit, not a universal requirement for prior membership or reputation
  before receiving anything. Protected new-voice and local floors remain available. The design
  avoids making every receiver absorb an unlimited stream and buy a filtering service.
- **Sense and respond through provenance.** Recognition and correction travel back through the
  relationships that carried the content. Propagation has consequences for authors and relayers;
  reach is not permanently earned once and then immune to subsequent evidence.
- **Quarantine and restitution at the edges.** The intended loop limits further harmful propagation
  and accounts for consequences through shefa's economic events. It distributes judgment and repair
  rather than installing a single moderation authority over the network.
- **Personal preference contributes without becoming an echo chamber.** The canon's guards are
  consciously expressed, expiring, tended, bounded by anti-bubble policy, and contribute collective
  signals. Individual preference, consented readership, and collective earned reach remain distinct.

The [social-medium epic](../social_medium/epic.md) makes the positive experience concrete. Maria's
account of a school-board success reaches the people who can use it, with context added before it
travels. When a correction arrives, there is a path to acknowledge and repair. This is how useful
experience becomes shared intelligence without making maximum virality the objective. The epic
also describes standing as a relationship-dependent pattern of contribution, correction, vouching,
and repair; different communities interpret it through their commitments. A local numeric evaluator
must not quietly become a global reputation authority. Wider influence carries responsibilities;
losing amplification must not erase a person's intimate relationships or standing as a human.

[Ubiquitous Wisdom](./ubiquitous-wisdom-dissolves-chokepoint.md) supplies the deployment thesis:
contextual judgment at authoring, relay, and consumption, near the people concerned. Holochain,
iroh/libp2p, and doorways provide separable infrastructure for that judgment. Deploying the same
model at many endpoints is not sufficient evidence that judgment or authority is independent.

This is **trustful scaling**: trustworthy participation should reduce redundant discovery and
verification work and improve useful distribution, while carrying reciprocal duties and consequences.
That benefit must not become a permanent incumbent privilege. New voices, local relationships,
recovery, and constitutional protections need their declared floors even when standing is low.
Amortized work must preserve the applicable integrity and authorization checks.

The mechanisms have differing maturity. `services/reach_earning.rs` contains a deterministic
standing-and-manifest evaluator and explicit floor classes. `p2p/mod.rs` records predecessors;
`api/epr.rs` wires feedback projection and back-propagation with its runtime context;
`services/back_prop.rs` supports a sealed, immediate-predecessor walk rather than broadcasting a
complete social trail. The forwarding result reports attempted delivery, not a confirmed end-to-end
correction. These are substantive primitives. They do
not establish universal authoring-path coverage or a completed quarantine–restitution loop. The
canonical documents and source comments contain older deferred labels; neither labels nor symbols
alone certify the end-to-end behavior. No such behavioral certification is claimed in this revision.

**Space membership cannot replace this nervous system.** Nor is `reach` one knob that can safely
stand in for audience, propagation entitlement, holding responsibility, and governance standing.
The couple's private-note story remains a useful counterexample to that collapse: James can belong
to Matthew and Jessica's household and help hold backups without being a reader. The relevant
`genesis/a2o/features/lms/intimate-reach-household.feature` scenarios remain `@wip`, with confidential
custody also `@envisioned`. Their unfinished state is a requirement to fulfill, not a reason to
simplify the intended relationships away.

### Scale the aggregates without erasing the people

There are at least four different costs to bound: records held and validated within a space;
bytes and operations moved between peers; social signals evaluated and propagated; and evidence
summarized across communities. Smaller spaces address part of the first cost. They do not alone
bound the other three. Trust-aware routing and reuse, appropriately scoped custody, and recursive
aggregation must compose with them.

The existing `CoverageRollup` in `elohim/elohim-storage/src/recursion.rs` offers a useful primitive:
union child coverage, expose unmet obligations as a deficit, and retain constituent pointers so a
summary can be investigated. The design direction is **summarize upward while preserving authorized
descent to evidence**. A collective can recognize an unmet care obligation and trace its cause
without demanding a complete account of each person's life. Missing evidence must remain distinguishable
from a measured absence of care; privacy is not proof of non-contribution.

The current implementation is narrower than the destination. Its production shefa helper builds a
flat rollup; the rollup constructor initializes witness quorum and source-head metadata empty, and
its hash does not include `required`. Agreement on that hash therefore does not establish agreement
on the obligation itself, independently witnessed coverage, or freshness. A pointer alone also grants
no right to read its target. These are questions to resolve before citing recursive aggregation as
proof of planetary coordination.

A peer roster is not evidence of independent failure domains. A volume of endorsements is not proof
of independent judgment. Compute contribution is not governance authority. Each aggregate needs an
explicit account of what it combines, what evidence warrants that combination, who can challenge it,
and which powers the result actually authorizes. The same question applies to aggregated storage,
care, standing, preference guards, and AI recommendations.

### Integrity is the alternative people must be able to inhabit

The destructive information environment is the starting condition of this work. Manipulation,
manufactured consensus, and the enclosure of relationships are the problems the manifesto addresses;
AI extends the means available to actors already pursuing them. People need the ability to recognize
what warrants trust and coordinate around it without handing that judgment to an unaccountable owner.
An attractive alternative makes useful contribution, trustworthy context, correction, and repair
ordinary experiences. Its success is felt as greater agency, dependable help, and time for living.

This gives integrity several inseparable meanings. **Record integrity** preserves authorship,
provenance, and the evidence of change. **Epistemic integrity** keeps claims investigable and
correctable without equating a signature, popularity, or a model's confidence with truth.
**Relational integrity** connects reach and responsibility while preserving dignity and repair.
**Institutional integrity** keeps power bounded, decisions contestable, and participation free of
an indispensable intermediary's right to impose new terms. These are promises to people whose
mechanisms must compose; they are not four names for what the DHT alone guarantees.

The existing primitives give this purpose technical form: witnessed records, earned social reach,
contextual judgment near participants, feedback through propagation relationships, and aggregates
that preserve paths to evidence. Hardware Providence adds the practical proof of continuity when
a household changes its provider or operator. Each must carry its part without quietly discarding
another: fast propagation cannot erase accountability; aggregation cannot erase the person;
convenient recovery cannot become a permanent claim over their life.

The commitment is to the dignity of life and the architectural properties that protect it. Elohim
must be willing to recognize a peer system that keeps those same commitments; its own adoption and
dominance are not the measure of success. Holochain is one implementation choice to assess against
them. Fidelity to those commitments gives us reason to change its role wherever the evidence calls
for it.

We do not need a completed theory of every future adversary to take these obligations seriously.
We do need to identify what each mechanism actually guarantees, where present seams break the
promised experience, and what substantial changes will make the alternative more trustworthy and
more livable. That is the immediate purpose of this Holochain reassessment.

### Bring this whole chain to the design board

Begin with the manifesto, hardware participation promise, and social-reach mechanisms. Then walk
one connected day: a phone-only participant opens a community-hosted application, writes a useful
account, shares it with appropriate reach, receives a correction, benefits from a collective response,
and keeps working while a household device fails. Include the moment a misleading account reaches
the community and people discover, acknowledge, and repair the error. Integrity and convenience
must be observable within the same ordinary day.

For each step, put four things beside one another: **the person's expectation; the technical act
actually performed; the guarantee and its evidence; the unresolved seam.** Trace that act through
Holochain, storage, sync, reach, and the client. Do not describe an intended mechanism as wired, a
wired mechanism as a passing story, or a passing local story as evidence of planetary scale.

Breakouts examine that same day through three lenses:

- **Experience and hardware:** what the person can rely on across hosted, phone, and hub participation;
  where waiting, failure, recovery, or operational labor breaks the convenience promise.
- **Holochain and adjacent primitives:** what is witnessed and why; which boundaries are required,
  chosen, or accidental; how each of the alternatives above changes cost and continuity.
- **Social reach and aggregates:** how useful contributions, feedback, responsibility, and shared
  capacity compose; which evidence survives aggregation and which authority must never be inferred.

Each group returns a present design assumption to retain or change, the existing primitive it would
reuse or needs to reconsider, its costs at household and aggregate scale, and a probe that could
change the decision. Recombine where the same journey produces incompatible answers. Record the
chosen changes in the existing design, a2o stories, and owning habits, with unfinished proof named.

A fixtures clone remains a bounded experiment in isolation and cell cost. It neither removes old
published fixtures nor proves that every household should be a space. The sequence in §10.6 is prior
technical review history to reassess, not a predetermined outcome. The original “holons are spaces”
thesis is under examination, including the proposed zome split, global roles, and cross-space joins.

**The review clears the way when we can explain Holochain's role in the person's day, name the
seams without pretending they are solved, and commit to the changes that evidence compels.** That
may mean using Holochain differently, using less of it, or establishing a credible replacement. The
vision decides what must be preserved; the current technology does not get to decide the vision.


---

## 0b. Second opinion and sealed decisions (2026-09-06)

A second opinion (Opus fact-check, Sonnet habit mapping, Codex adjudication, plus an outside-developer
read) reviewed this document and its companion, [the reimplementation plan](./2026-09-06-ai-stewarded-commons-reimplementation-plan.md),
which now owns the resulting sequence. Ten decisions (D0–D10) were sealed; in plain words:

- **D0** — classify every act by authority, durable source, validation contract, and rebuild path
  before deciding whether it needs witnessing. No default DHT-or-projection binary.
- **D1** — the connected graph across validation contexts is the product; aggregates are composed
  views, never DNAs.
- **D3** — the membrane gate is peer-side validation of the join proof; the genesis self-check this
  document proposed leading with is a courtesy only.
- **D4** — a cross-plane or cross-space act needs named successor authority and stated partition
  behavior, designed before slice-1 code.
- **D5** — the omnibus split folds into the next integrity crossing already needed; "no live writer"
  does not make deletion safe.
- **D6** — group clone spaces are an operator-selected candidate, gated behind D0, D3, and a measured
  phone/hub budget, not a demonstrated destination.
- **D7** — the fixtures clone is a harness experiment run in parallel with slice 1; it grants no
  downstream authority.
- **D10** — the first slice is "accountable correction converges," under the existing
  `dataplane-convergence` habit: commissioning evidence, not settled placement.

So: (a) the automatic household→DNA mapping described below is retired; group clone spaces remain an
operator-selected candidate with reconsideration criteria. (b) the omnibus split is not a
prerequisite; no live writer does not by itself make deletion safe. (c) the fixtures clone is a
harness experiment with no downstream authority. (d) the first slice is accountable-correction
convergence, not the household-spaces sequence in §10.6.

---

## 1. First principles — what a Holochain network actually is

Forget "blockchain" and forget "database." Picture three things.

**1a. Every person writes their own journal.** Each agent (a keypair on a device) has a *source chain*:
an append-only, signed log of the things that agent did. Nobody else writes in it. There is no global
ledger. There is no consensus about "the order of everything." There is only "Matthew signed this, after
that, at this time."

**1b. Public records contribute operations to a shared shelf.** Public entry data can be held and
validated by other peers; private entry payloads stay off the DHT, although their associated public
actions still participate in validation. Which peers hold a public operation depends on its DHT
location and their declared arcs. Every
entry and every agent has a hash. Both live on the same circle of 2^32 addresses.

A peer holds
the slice of that circle it has declared responsibility for — its **arc**. The shared shelf is the
**DHT**, the distributed hash table. It is a circle sliced into arcs, rather than a copy on every node.

**1c. The rules are the network.** Before a peer holds your entry, it runs the *validation rules*.
Those rules are compiled code — the **integrity zome** — and the hash of that code (plus a few
modifiers) *is the identity of the network*. Matching integrity code alone is not enough: the modifiers must match too. Changing integrity code
or a hashed modifier produces a different DNA hash and therefore a different space. Existing
participants do not migrate to it automatically.

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

The word in the name is a design statement. A **holon** (Koestler's term) is a whole that is also
a part. A cell is whole and is part of an organ. A household is whole and is part of a village.

Holochain made the holon the unit of the network on purpose: one DNA per group that shares rules.
A peer's conductor runs many cells at once, one per space it belongs to. Your device is the place
where your holons meet. There is no "the network." There are the networks you are a member of.

This is why the project's own language fits Holochain so naturally once you see it:

- A **household** is a space. Its rules are household rules. Its members are its peers. Its arc
  budget is "five devices hold everything," because five is small.
- A **collective** (a church, a co-op, a school) is a space. Larger membership, different rules, its own
  holding floor.
- The **commons** (elohim.host content, the protocol's own docs, the public learning paths) is a space.
  Thousands of readers, tens of stewards.

Nachalah calls a household note becoming a collective deed becoming a commons page *promotion*.
In Holochain, this is a **witnessed re-publish into a wider space**. Hard fact B explains why
promotion is a ceremony. The content's identity, its **CID** (content identifier: the hash of the
bytes), does not change, but its *holders* do.

The **elohim** witness that act. This is the protocol's cross-holon plane: doorways, aggregation,
and witnessing, outside every DHT.


---

## 3. How we actually use Holochain today

Let me be precise, because this is the part that was never written down as a decision.

**Our hApp has five roles**: `lamad` (content: every learning node, page, path), `imagodei` (the identity role: humans, devices, recovery),
`mishpat` (the governance role: commitments, grants, revocations), `infrastructure`, and `node_registry`. Each role is one DNA.
On the alpha fleet, each role is **one space** — the fleet's peer store lists exactly five spaces, and
every peer is a full-arc member of every one of them (32 of 35 agent entries carried the full arc
`[0, 4294967295]`). The three exceptions are the OOM-shed leechers. This snapshot came from
`GET /db/p2p/conductor-diagnostics` on doorway-alpha at about 03:15Z on 2026-09-06.
The reported `agentCount` rose to 43 after the workspace peer joined.

**The content role is one space for everything.** Every learning node, every landing page, every
fixture the test suite ever seeded, every household's working note that has been written so far, lives
in the single `lamad` space. On doorway-A's storage peer that is 3,442 content rows this morning.
The sync plane counts 110 live documents in flight.
An **EPR** is an Elohim Protocol Record: the protocol's content-addressed document, the unit a person reads or writes.
The test fixtures alone are several thousand EPRs.

Every full-arc peer validates and holds all of it. When a peer restarts, it re-gossips all of it.
The habit ledger's "restart churn ≈ 20 minutes" and "RAM ∝ corpus at full arc" are direct consequences
of this single layout decision.

**And there are no household spaces yet.** I want to be exact here, because I got this wrong in the
first draft of this document. The only place our hApp manager uses a network seed today is a *lineage*
install — a role moving from an old DNA hash to a new one preserves its seed. The new DNA hash
still denotes a different network; preserving the seed does not remove the lineage crossing.
No code path creates a clone cell for a household or a collective. The "household space" the Nachalah
stories describe is a design, and the "household space partition" of 2026-09-05 was a cell on the local
household *mesh* being blocked, not a clone space. Four of our five integrity zomes implement
`genesis_self_check` as a one-line stub returning *valid* for anyone (content, imagodei, mishpat,
infrastructure); `node_registry_integrity` has no `genesis_self_check` at all. No integrity zome
validates the `AgentValidationPkg` op either — the peer-side membrane gate does not exist in any
form (correction, 2026-09-06; see §9.2). So the primitive is available in the
substrate and unused in the product: one space per role, open to any key, everything at full arc.

**Then we built a second system to cope.** This is the honest part. Because everything sat in one space
at full arc, we grew, in *our own* storage layer:

- an **arc actuator** in our storage layer that flips a whole peer to zero-arc when it OOMs (jessica and james were flipped
  to arc 0 in June for exactly this reason), and an **arc policy** that computes a fractional aim it
  cannot actually set.
- **reach enforcement at read time** in our storage layer — the `reach-enforced-everywhere` habit. A peer that
  *holds* a household's bytes (because it holds the whole space) refuses to *serve* them to the wrong
  person.
- **held views, per-host custody stamps, projection fallbacks** in our storage layer let a doorway keep serving
  when its conductor cannot answer for a corpus that is too big to be timely.
- a **quiesce gate** in our storage layer waits for fleet catch-up before accepting a measurement.
- **fixture hygiene** rituals in our storage layer keep test content from crowding real content.

None of that is wrong code. Most of it is load-bearing today. But look at what it is: it is
**membership and holding, re-derived one layer up, because we did not use the layer that has it.**
An enforced household membrane could exclude nonmembers from that DHT. It would not settle
which members may read a record or secure its off-DHT copies. A separate fixture space could keep
future test writes out of the commons; existing published fixtures would remain.

### The mental model we were carrying

We treated the `lamad` DNA as *the content database* and the DHT as *replication*. That is the
relational-database instinct the `p2p-design-gate` skill exists to catch, and it slipped through at
the coarsest grain: not at the entry level (we got content addressing right, see §5) but at the level
of *what a space is for*. We thought of a space as a **schema**. Holochain thinks of a space as a
**membership**.

---

## 4. The scaling question, with numbers — a decomposition to benchmark, not a measurement

**Correction (2026-09-06):** the arithmetic below is a decomposition, not an extrapolation. Before §4b
can be believed, measure:

1. Per-space bootstrap, peer-store, and relay state, not just document storage.
2. What the 1.2 GiB idle baseline (§9.3) is actually attributable to.
3. The authorship cost of a zero-arc cell (it still joins, bootstraps, and holds peer-store state).
4. The cost of the global roles (identity, infrastructure, node registry) at scale.
5. Cross-scope fanout when a record crosses a holon boundary.
6. Phone-class RAM, thread, file-descriptor, and battery cost, not idle x86 (§9.3).
7. Year-equivalent history growth per holon, not a day-one snapshot.
8. Multi-household failure and recovery behavior, not a single isolated space.
9. Service-side (doorway/hub) bootstrap and relay scaling across many spaces.

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
  notes and your collective's deeds. It never hears about a household in Lima. It cannot, because it is not in
  that space.
- Validation per device ≈ only the rules it has agreed to, for the communities it has joined.

The total work in the world grows with `Σ (members × documents)` **per holon**, which is a sum of
small products, not one product of two huge numbers. Seven billion people is 1.5 billion households of
five, each a space of five peers holding a few thousand documents, plus a long tail of collectives.
Each one is the size of the mesh we already run in this workspace (three peers, one household). We
have been rehearsing the unit of scale for a year without naming it.

### 4c. Identity and discovery across spaces

Two infrastructure concerns must cross holons: **identity** and **discovery**. They are not the
whole cross-holon story: social reach, accountable commitments, and aggregate evidence also cross
relationships. §0a restores those mechanisms to the scaling argument.

- **Identity** crosses because we chose content addressing. An EPR's CID is the hash of its bytes. It is
  the same in every space it is published into. The DHT anchor (the action hash) is per space. The
  *thing* is not. Our earlier choice to make the CID the identity and the DHT anchor a per-space
  attribute is the single decision that makes holonic placement possible at all. **Correction
  (2026-09-06):** "we link across spaces by CID, never by action hash" is wrong as stated. Content
  bodies and blobs are CID-addressed, but two live paths ship action hashes under CID-shaped names —
  `content_store/src/feedback_signal.rs:137,143` formats an `ActionHash` into `target_cid`, and
  `conductor_writes.rs:74,156-179`'s collective "CID" is `collective:{ActionHash}`; head adoption is
  keyed on `head_action_hash` throughout. The mapping between action identity and content identity
  needs an explicit rule before more crosses a space (D4 in §0b).
- **Discovery** — "which peers are in this space, and how do I reach them" — is per space too, and
  that is fine. Kitsune2 bootstraps per space. Our doorways already serve bootstrap and relay per space.
  A new household mints a seed, its five devices bootstrap through any doorway, and no global registry
  is needed. The **elohim** (the aggregation plane, doorways, the future hub cluster) is how a person
  finds a holon they are not yet in — which is a social act, not a DHT act.

### 4d. What the doorway and the blob plane do in this picture

- The **blob plane** (iroh/libp2p in our storage) already lives outside the DHT: big bytes never go into
  a space. Only manifests and heads do. Spaces partition *notarization and small-record gossip*. The blob
  plane follows the same membership (a household's peers hold a household's blobs) but its transport is
  independent. That is why it was the right plane for the **T2 substrate**: the storage layer's own
  peer-to-peer plane for bytes, using iroh/libp2p.
- A **doorway** is a web2 projection of *the spaces its storage peer is a member of*. That is also why a
  doorway "cannot see" a household: it should not be able to, unless a household member runs the doorway
  or the household promoted the content to a space the doorway steward belongs to.

---

## 5. Where we fought the architecture, and where we got it right

Being fair to the last two years:

**We got right:**
1. **Content addressing as identity** (§4c). Without it, holons would be islands.
2. **Blobs off the DHT.** The DHT holds small signed records. Bytes ride the storage substrate.
3. **Heads as declared, elected content.** A page's "current version" is a record peers agree on by
   rules, not a mutable row. Release channels for runtime upgrades are the same idea applied to code, and
   last night the first coordinator release crossed from a workspace to the fleet through that mechanism.
   Edge #1432 was the fleet deploy. The coordinator crossing itself had no pipeline in its path.
   Evidence: `genesis/a2o/reports/workspace-release/2026-09-06/shift-it2.json`, `shift-it6.json`,
   and `shift-it7.json` (5/5 each). The habit records DELTA 2026-09-06 in
   `elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md`.
4. **Coordinator hot-swap.** Behavior changes without changing the network's identity. This is
   Holochain's own gift and we lean on it hard.
5. **Lineage installs keep their seed.** The seed plumbing is exercised, but a new DNA hash still
   means a new network. Continuity depends on the crossing machinery, not seed preservation alone.
6. **The Nachalah tiers on DNA seams.** Gold, deeds, paper: the epic already says the tiers are spaces.
   We had the design but had not applied it to content.

**We fought the architecture by:**
1. **One content space for the world.** Everything else on this list follows from it.
2. **Treating arc as the knob.** Arc actuators, fractional aims, "shed to zero on OOM" — all of it is
   trying to make a peer hold *less of the wrong space* instead of *not being in it*.
3. **Treating read-time reach as the whole confidentiality boundary.** Serve refusal alone cannot
   protect plaintext already held by a peer. But membership alone cannot express the intimate,
   creator-only, and blind-custody distinctions our stories require either. The boundary needs review
   across DHT records, blob custody, and serving paths; §0a reopens the claim that membership retires it.
4. **Per-host custody stamps** (the `serverBlobHash` PATCH the dataplane-convergence habit now flags):
   a doorway telling its storage peer "you serve this bundle" is a per-host imperative because there was
   no space to say "the doorway stewards hold the served commons at full arc" declaratively.
5. **Fixtures in the commons.** Thousands of test EPRs in the same space as elohim.host's landing page,
   gossiped by every fleet peer forever. A `fixtures` clone space nobody follows at full arc is cheap,
   not free (correction, 2026-09-06): a zero-authority clone still authors, bootstraps, and holds
   peer-store state.
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
   wire. Lifting the clamp is the "arc-policy hook" in the Evolution epic §11.3, relevant to large commons spaces.
   Under §10.4, we carry the per-space hint as an upstream PR first and do not implement fractional sharding.
   This changes nothing about §4.
3. **An entry lives in one space; links do not cross spaces.** Cross-holon reference is by CID.
   Cross-holon presence is a witnessed re-publish (promotion).
4. **Validation is by the members of the space, against that space's rules.** A five-device household
   validates household rules. It cannot validate "commons reach" for you — that is what promotion into
   the commons space is for.
5. **A cell is a full participant.** Conductor memory and gossip scale with the *number of cells* a
   device runs. Holons are household- and collective-sized. Never per document, never per person-pair.
6. **A rejected op can block a cell (0.7).** Every space is its own partition risk. The household
   unblock tooling the epic built matters *more* as spaces multiply, not less.
7. **Crossing a lineage break moves data by ceremony, not by magic.** Moving content from the one big
   space into tiers is a crossing per holon — the same machinery the Evolution epic is building for
   version crossings. Nothing is thrown away. It is re-published under witness.
8. **Discovery is per space.** Bootstrap and relay run per DNA hash. The doorway already does this.

---

## 6½. Arcs, precisely — what is configurable today, what the Holochain team's scaling story is, and what the arc work is for

This section exists because the ruling "the unit of arc is the space" can sound like "arcs don't matter."
They matter a great deal. They are just the *second* axis of scale, and the one that is currently missing.

### The Holochain team's own scaling story (both axes)

Read the Holochain design papers and the team's talks and you find **two** independent answers to "how
does this reach billions," and the project's health depends on knowing which one is available today.

**Axis 1 — many networks (holons).** No global consensus, no global network.
Every hApp is its own network, and every clone is its own network.
A person's conductor bridges the networks they belong to. Membranes (membrane proofs) decide who may join.
Total work is a sum over small memberships.

This axis is *architectural*: it is how the team expects most of humanity's data to be organized.
It is fully available in stock Holochain today. §4 explains this axis.

**Axis 2 — sharding inside one network (arcs).** For a network that is genuinely large — a public
commons with millions of readers — no single member can hold everything. So each peer holds a *slice*
of the address circle (its arc), and the network sizes the slices so that every entry is held by roughly
`R` peers (the redundancy target) no matter how big the network gets. Work per peer stays roughly
constant as members join, because arcs shrink as density grows. Validation is by the peers whose arc
covers the entry. This is the "sharded DHT" the team describes, and it is what lets *one* space scale,
as opposed to letting *many* spaces coexist.

**The conductor clamps arcs to full or zero today.** At the 0.7 fleet pin, the code says:
"target arc factor > 1 is not yet allowed until sharding is implemented."
Kitsune2 still carries arc ranges on the wire and in the peer store.
Alpha's peer store showed `[0, 4294967295]` this morning, but the conductor uses only **full or empty**.

**Axis 2 shipped, then went away.** Kitsune, the first networking layer (holochain 0.1–0.4), implemented dynamic arcs.
A peer's arc resized toward the redundancy target using observed peer density.
The rewrite, Kitsune2 (holochain 0.5 onward, the line we run), kept the data model but reduced conductor behavior to a switch.
That history explains a year of our confusion: Axis 2 is *dormant*, present in the substrate but absent in the conductor.

### What is actually configurable today (verified at the fleet pin)

1. **`target_arc_factor` is a single number per conductor.** It lives in the conductor's network config
   and the conductor builder applies the same value to every cell's local agent. Values: `0` (empty arc,
   a "leecher" who holds nothing and reads from the network) or `1` (full arc). Nothing in between.
2. **It is conductor-wide, not per space.** A conductor cannot today be full-arc in its household space
   and zero-arc in the commons. Kitsune2's internal API *does* carry a per-agent, per-space target-arc
   hint — the conductor simply sets that hint to FULL for every cell (`holochain_p2p` actor, on join) and
   derives the factor from the one global knob.
3. **Changing it needs a conductor restart.** It is boot config. Our storage's arc actuator renders a new
   conductor config and performs a staggered restart. That is the "T1 {0,1} switch" tier in the arc policy spec.
   It flipped jessica and james to leechers in June when they OOM'd.

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
  *which spaces a device joins*. At Axis 2 it decides *how wide a slice it holds in a large space*.
- **The storage layer (`arc_policy.rs`, `arc_actuator.rs`).** A pure `derive()` from memory ceiling,
  coverage floor, observed peer count and corpus size. Its executor can only set full or zero arc per
  conductor, with a restart. This is a correct Axis-2 controller waiting for an Axis-2 lever. Its coverage
  invariant ("a leecher must leave the mesh covered") is exactly the redundancy-target reasoning the
  Holochain team describes.
- **The fork hook (Evolution epic §11.3, open).** The minimum change derives each cell's target-arc hint
  from a **per-space policy**, replacing the global factor. An admin interface would let storage set it
  *without a restart*. The original proposal also allowed a range instead of a Boolean, since the peer
  store and gossip already speak ranges. No branch in this repository carries that change yet.
  Under §10.4, we pursue the per-space hint as an upstream PR first. We do not attempt the fractional sharding it originally bundled in.

**What this means for priorities.** Axis 1 (spaces per holon, §7) removes most of today's pain and needs no fork.
The instrumented fixtures clone goes first. Per-space arcs let a phone be a full household member and a light commons reader at once.
That hint is necessary for the phone-to-rack spectrum in the **seam map**, the project's atlas of where each concern lives, from watch to rack.
Phones stay spokes to a household hub until it lands.

Fractional arcs are the separate Axis-2 work a large commons needs. We leave that work to upstream under §10.4.
Both axes are the Holochain team's own story. We had been trying to do Axis 2's job with its dormant lever while leaving Axis 1 unused.

### The scale story, restated with both axes

Seven billion people. Roughly 1.5 billion households of five, each a space held fully by five devices:
Axis 1, available now.

Tens of millions of collectives of 100 to 10,000, each a space held fully by its
stewards and at zero arc by its members: Axis 1, available now.

**Correction (2026-09-06):** "available now" means available for *creating* each of these as a
separate network. It is not available for the advertised mixed-arc phone — full household membership
plus zero-arc commons reading, on one conductor — until the per-space hint above lands. Until then, a
device is full-arc in every space it joins or zero-arc in all of them.

A few thousand commons spaces — the
protocol's own, a language's, a region's, a movement's — each with millions of readers and thousands of
stewards, held in *slices* by those stewards so that no rack needs to hold a whole commons: Axis 2, the
upstream sharding work.

A person's phone would bridge its 10 to 50 spaces once the per-space hint lands.
Until then, it stays a spoke to a household hub. Its work is bounded by its own memberships and allotment,
never by the size of the world. This story combines shipped capabilities, the named per-space hint,
and fractional sharding that we leave to upstream.

### Is sharding the lynchpin? (operator question, answered)

Half right. Sharding is the lynchpin for a **large commons held by many**.
Without it, only nodes that can hold everything can hold the commons: racks and a few stewards.
That is tolerable now but wrong for the phone-to-rack spectrum.

**"shared between holons" is not what sharding gives you.** Cross-holon sharing comes from membership and promotion, available today.
A data commons is a space everyone joins, most at zero arc. Content enters by witnessed re-publish.
Sharding decides how widely the commons is *held* once it is there.

The dependencies are (1) placement, (2) a small per-space arc hook settable hot, and (3) true fractional sharding.
Placement moves fixtures and households out of the commons without a fork. The hook lets a phone hold its household fully and read the commons lightly.
Fractional sharding filters ops by arc, routes gets to out-of-arc authorities, and maintains validation coverage as arcs shrink.
Upstream built (3), deferred it during the rewrite, and intends to restore it in kitsune2.
Following §10.4, we carry the unblock API and per-space arc hint as upstream PRs first, without attempting fractional sharding.

## 7. What changes in our design, concretely

This is the technical hypothesis recorded on the arc-policy code and in the Nachalah hub, spelled out
below; sequencing across it is decided in [the reimplementation plan](./2026-09-06-ai-stewarded-commons-reimplementation-plan.md)
under D1/D5/D6 (§0b).

**The placement rule:** *content declares its space. Its members hold it at the arc the trust gradient allots.* Today, full or zero arc is conductor-wide. The per-space hint needs an upstream PR and a carried patch.
An EPR's reach and holding declarations constrain placement, but social reach also governs earning,
propagation, feedback, and accountability. Space placement is one output; it cannot replace that loop.

**What that means for each plane:**

| Plane | Today | After |
|---|---|---|
| Content (`lamad`) | one space, full arc for all | commons space (stewards full arc, readers zero) + collective spaces + household spaces + a fixtures space |
| Reach | enforced through existing serving paths, with known gaps | membership, consented readership, and confidential custody must each be demonstrated; membership alone does not retire serving authorization |
| Holding floor | a global replica target | per holon: "held by ≥ r of this holon's members" (the Nachalah gold/deeds/paper floors) |
| Promotion | copy + flag | witnessed re-publish into the wider space; CID unchanged, anchor per space |
| Arc policy | shed a peer to zero on OOM | choose which spaces a device joins and at what arc, once the per-space hint lands. Until then, phones stay spokes to a household hub. OOM becomes "too many full-arc memberships," a social/allotment question |
| Upgrade propagation | per role | per **cell**: a coordinator release must reach every clone of a role. The adoption controller's "installed reality" becomes per cell. **Correction (2026-09-06):** `sync_coordinators_for_app_info` (`happ_manager.rs:1310-1314`) filters `CellInfo::Provisioned` and silently drops clones today — a clone would not receive a coordinator hot-swap and would not appear in drift reports. That gap is in scope of the Evolution epic, not yet closed. |
| Fixtures | in the commons | in their own clone space; only the test runner follows it |
| Doorway | projects "the" content | projects the spaces its steward belongs to |

**Foundations to preserve:** content addressing, blobs off the DHT, declared heads, release channels,
and coordinator hot-swap. Their use across new spaces still needs verification. The existing a2o
stories provide requirements, including consent and blind custody, that the proposed topology must
satisfy; the blob and serving planes cannot be assumed unchanged merely because they sit outside the DHT.

**The proposed first experiment (existing clone primitive; effort to be measured):** an instrumented `fixtures` clone of the content role with its own seed.
The seeder targets it, and fleet peers do not follow it at full arc. Measure per-cell cost that same week.
This tests isolation for new fixtures and measures cell cost. It does not remove previously
published EPRs from the commons or validate the household experience.
For the adopted sequence and risks, see §10.6.

**The second slice:** the `list_blocks` / `unblock` admin API, as an upstream PR plus a carried patch.
It must land before any household space. It is administrative: clearing a local block never repairs
an invalid history. It lives in the Holochain Evolution epic §11.3 as an upstream PR.

**The third slice:** the zome split as one lineage crossing.
Remove identity duplicates, economy types, and infrastructure types from content.
Land a real membership check, a lineage record, and a non-null progenitor in the same hash move.

**The fourth slice:** household working notes into household content spaces, using the seeds the mishpat role already mints, with membrane proofs.
The `reach-enforced-everywhere` habit still requires evidence across the serving and custody paths;
rejecting an outsider at DHT admission alone does not prove that habit.

**The fork slice (small, and not optional for phones):** carry the *per-space* arc hint as an upstream PR first.
The conductor already ignores this hint. Plumbing it through lets one device hold its household fully and read the commons at zero arc.
Until the hint lands, a phone is either a full member of every space it joins or a reader of all of them.
Phones therefore stay spokes to a household hub until then, as the seam map already draws.
Fractional arcs inside a big space remain separate, larger work that §10.4 leaves to upstream.


---

## 8. How to explain it to someone else in two minutes

> We want people to have dependable applications, records, recovery, and applied intelligence
> through infrastructure their communities can steward. A person with only a phone should benefit
> without becoming a systems administrator or a captive tenant. Social reach helps useful work
> travel with context and responsibility; corrections and repair are part of that same system.
> This is the alternative we owe people facing manipulation and an overwhelming stream of claims:
> a useful place to learn what deserves trust, contribute, receive help, and act together with dignity.
>
> Holochain can supply signed histories and peer validation under shared rules. Our storage and
> transport supply other parts of the experience. The question is whether we have assigned those
> jobs well: what must be witnessed, who must participate, what happens while a witness is unavailable,
> and whether upgrades and community changes preserve people's continuity.
>
> More spaces might help. A narrower notary role might help. A different witnessing substrate must
> remain an option if it can meet the actual requirements. We need to walk an ordinary person's day
> through those choices, measure their consequences, and change the architecture where it fails the
> vision. The integrity of that lived alternative is the commitment; our use of Holochain must earn
> its place in delivering it.

---

## 9. The five technical questions — original answers under product review

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
memory (see 9.3). A space buys three things: a membership boundary, a holding floor its members can
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

The table's future fractional arcs depend on upstream sharding. Under §10.4, we do not implement that work ourselves.

Affinity and locality become *filters on records inside a rung*, never rungs of their own.
For example, a record in the English commons space can have a denomination affinity.

**And per role:** content and governance go **per holon** (a household's pages and its commitments are
its own business). Identity, infrastructure and node registration stay **global**, and the reason is the
best argument in this whole document: a recovery quorum is deliberately made of people *outside* your
household (the "grandma case" in the identity zome, the backup custodian who may not read the note). A
household space cannot validate its own rescue. Identity must be readable across the membrane.

**So this suggests doing:** raise `clone_limit`, mint a `fixtures` clone of the content role, and let no fleet peer follow it.
This is a bounded test of the placement mechanism using an existing primitive. It prevents future
fixture writes in the commons; it does not remove previously published records.

Two landmines from the field (§11): keep the role *provisioned* and just raise `clone_limit`.
The `clone_only` strategy leaves it unprovisioned, and the 0.7 conductor panics while assembling app info.
The `deferred` flag is also ignored on install.
A provisioned role with a clone limit in the hundreds runs in production elsewhere.

### 9.2 Membrane proofs — who may join a space?

**What we found.** Holochain has the mechanism end to end.
A joiner supplies a *membrane proof* (bytes) when a cell is installed or cloned.
The conductor runs the integrity zome's `genesis_self_check` *before* creating the source chain.
Rejection means the cell is never born. Acceptance writes the proof permanently as the chain's second record, where later validation can inspect it.

**Correction (2026-09-06):** four of our five integrity zomes implement that callback as a one-line
stub returning *valid* for anyone (content, imagodei, mishpat, infrastructure); `node_registry_integrity`
has no `genesis_self_check` at all. And a stronger gap: no integrity zome validates the
`AgentValidationPkg` op — the peer-side membrane gate does not exist in any form, only the
genesis-time courtesy check does. Our hApp manager never passes a proof. The lineage install
explicitly writes `None`.

**What we already have that is proof-shaped.** Two things: the cross-signed binding between an agent
key and its transport identity (the `identity-cross-signed` habit, red today, observe-only), and mishpat's
`verify_credentials`, built for "collective membership CIDs presented by connecting peers," which checks
that a record exists and who authored it.

**If we did it this way, then (reordered per D3, §0b)** joining a household or collective space
requires presenting a signed membership record, and the gate that matters is **peer-side**: every
other member's `validate()` checks the `AgentValidationPkg` op against the space's founding rule
before gossiping anything the joiner sends. `genesis_self_check` reuses that same rule as an early,
local courtesy — it runs before the chain exists and cannot itself protect anyone but the joiner. The
trust root ("signed by a key this household's affirmation names") is fixed in the *target DNA's own
rules*, never self-named by the proof the joiner presents. Concretely: the membership is a mishpat
commitment (the household affirmation the Nachalah stories already describe), signed through the
existing conductor signing path, carried as the membrane proof on the clone call. An honest installer
rejects a malformed or unauthorized invitation early; honest peers running `validate()` refuse a
joiner with no or invalid proof even when that joiner's own `genesis_self_check` was patched to
always pass — the honest peers, not the joiner's binary, are the gate.

**So this suggests doing:** four pieces, in order. First, implement peer-side validation of the
`AgentValidationPkg` op in the content and governance zomes' `validate()` — this is the actual gate.
Second, have `genesis_self_check` reuse the same rule as a courtesy, never a substitute. Third, add
the code path that turns a mishpat membership into proof bytes and pass those bytes on clone creation.
Fourth, no restricted space ships until a Sweettest harness includes a joiner that bypasses its own
self-check and confirms honest peers still refuse it. Changing the integrity code changes its DNA
hash. Keeping the existing commons space would require retaining its existing integrity definition
while introducing the restricted variant separately; otherwise the commons needs a lineage crossing
too. That choice must be explicit.

One field warning (§11): the self check is a courtesy; the real gate is peer-side `validate()`.
Sweettest has no support for membrane proofs at all. Decide the holon test harness *before* building
the door.

### 9.3 Cell count per device — what does a holon cost?

**What we measured.** An isolated stock holochain 0.7.0 conductor, our five-role hApp installed once, then
clones of the content role added in steps with a minute of idle between checkpoints, on loopback only:

| Cells | RSS | Per added cell | Disk | Open files | Threads |
|---:|---:|---:|---:|---:|---:|
| 5 (the base roles) | 1,205 MiB | — | 186 MiB | 118 | 56 |
| 10 | 1,215 MiB | 2.1 MiB | 188 MiB | 208 | 96 |
| 20 | 1,240 MiB | 2.5 MiB | 190 MiB | 390 | 177 |
| 40 | 1,287 MiB | 2.4 MiB | 196 MiB | 754 | 339 |

Clone creation took about 126 ms each. Growth was linear: **about 2.4 MiB of memory, 0.26 MiB of disk,
18 open files and 8 threads per idle cell.** Commands and raw measurements are in
`genesis/a2o/reports/cellcost-2026-09-06/`: `rerun.sh`, `measure.mjs`, and `measurements.jsonl`.
**Note (2026-09-06):** that directory is gitignored (`genesis/a2o/.gitignore:3`) and not in the repo;
the numbers above were verified byte-exact against `measurements.jsonl` during the second-opinion
review.

**How to read it.** Memory is not the constraint people expected. Forty extra holons cost 80 MiB.
The striking number is the *baseline*: 1.2 GiB for five idle cells, before content or gossip.
That is the omnibus integrity zome and its four siblings being loaded and warmed.
It decides whether a phone can run a conductor at all, reinforcing the red team's point in §10.1.

The second constraint is **threads and file descriptors**.
Eight threads per cell means 200 cells (forty holons across five roles) need roughly 1,600 threads and 3,600 open files.
A laptop can accommodate that. A phone's OS cannot.

**If we did it this way, then** the marginal holon is cheap and the fixed cost is not.
A 4 GB phone can plausibly carry ten to twenty holons *if* the base cells get lighter.
A 2 GB phone is tight at any count.
A watch cannot run a conductor and must be a spoke to a household hub, as the seam map has always drawn.

**So this suggests doing:** first slice: measure the same table on one phone-class device during the fixtures-clone week.
This is an x86 idle measurement. Mobile validation with real gossip is next (§10.6, step 1).

Then address two constraints in order. First, the zome split (§10.1) is also the memory fix: the baseline loads everything for everyone.
Second, budget holons by *threads*, not bytes. The proposed phone budget is roughly fifteen spaces, pending the per-space hint (§6½) that lets zero-arc cells be genuinely dormant.
Until that hint lands, phones stay spokes to a household hub.
Treat "a person in forty holons" as a hub-class statement, not a phone-class one.

### 9.4 Migration of the existing commons — does anything have to move?

**What we found.** The content space holds 3,442 rows on doorway-A this morning and there is no
by-kind breakdown route. The a2o fixtures plant thousands of records per run, and nothing ever removes them.
Fixtures may already outnumber real content. The seeder knows nothing about spaces: every seed
path writes to the base cell. And the crossing machinery built for the Evolution epic is *lineage-only*:
it moves one role from an old DNA hash to a new one, same role, same purpose. It is the wrong shape for
"re-publish these records into space B and sunset them in A."

**If we did it this way, then** the current content space simply *is* the commons and stays where it
is. Nothing real needs to move. Two things stop being written there: fixtures (which go to their own
clone from the next seed run onward) and new household notes (which go to household spaces from the day
those exist). Old fixtures already in the commons remain as residue.
A one-time sunset of records authored by the seeder's fixture identities is a cleanup, not a migration.

**So this suggests doing:** give the seeder a cell selector (it has none), make the fixtures clone the
default target of every a2o seed run, and never ask fleet peers to provision it. Household notes need no
crossing machinery at all: new notes go to the household space from now on.

### 9.5 Elohim as the cross-holon plane — what lives between membranes?

**What we found.** Much of our system deliberately lives outside every DHT space.
This includes the steward-peers pool, conductor registry, upstream circuit breakers, and self-healing read model.
It also includes the transport-manifest bootstrap cache, freshness pantry, and coherence fingerprint.

On the storage side, it includes reconcile controllers, arc policy and actuator, head adoption, and the release controller's *follow-set*.
The elected head is notarized. The decision to follow it is node-local.
The federation-failover plan's whole gap list is likewise non-DHT.

This is not debt. The seam map calls it the inversion: the social, governance, trust and recovery plane has no hyperscaler equivalent.
No single space can hold it because it concerns *relationships between spaces*.

**If we did it this way, then** five things follow, each of which resolves something that has felt like
a bug:

1. *A doorway projects exactly the spaces its steward joined.* Membership replaces the hand-edited steward-peers pool, so "the doorway can't see a household" becomes correct behavior.
2. *Discovery is a social graph in identity, not a DHT lookup.* Identity and infrastructure stay global (9.1), because per-holon identity would make holons unreachable.
3. *A recovery quorum is a cross-space commitment in the custodian's governance space.* The custodian must validate it without joining the household, whose note the allotment story says they may not read.
4. *A release channel becomes per cell, not per role.* The adoption controller's "installed reality" must enumerate clones, while following stays node-local: james's canary promotion this morning changed only his row.
5. *The hub cluster is the commons' rack-tier stewards.* Only per-space arc lets a phone hold its household fully and read the commons lightly at once, hence §6½'s fork hook.

**The design rule to examine:** *inside a membrane, the DHT provides the shared record of acts
validated under that space's rules; it does not establish unquestionable social truth.
Across a membrane, the proposed elohim plane carries a witnessed commitment recorded in the receiving holon's governance space.* The
aggregation plane's node-local state is correctly placed. The work is to make it *derive from
membership* rather than from config.

**So this suggests doing:** first slice: derive the doorway's steward-peers pool from space membership once the fixtures clone exists — one config list becomes one derived roster.
Nothing new goes in the DHT. Record recovery quorums and promotions in the receiving holon, and let the release controller enumerate cells.
Each is a small change to existing code.

### 9.6 What this section's hypothesis was, in one breath

This is the technical hypothesis recorded here; sequencing across it is decided in the
reimplementation plan under D1/D5/D6 (§0b).

Spaces are memberships with their own rules, sized household → collective → regional commons → protocol commons.
Content and governance go per holon. Identity and discovery stay global.
Every holon gets a door made from a signed membership record.
Nothing in the commons moves except fixtures and new household notes.
The elohim plane stays where it is, deriving from membership.

First move: raise `clone_limit`, mint the fixtures clone, and measure the difference in gossip.

## 10. The red team — a Holochain reviewer runs us through it

*The operator asked for a hostile review from a Holochain core contributor's point of view before
committing to any pivot. What follows is that review's findings, each followed by the orchestrator's
verdict. Where I disagree I say so. Where I accept, the answer above is amended.*

### 10.1 "You screwed up the layout, not the architecture — except for one thing."

**Finding.** One space, full arc, fixtures in the commons, and a clone limit of 0 are layout choices, reversible in days.
In the reviewer's field experience, every serious hApp team (Acorn, Moss, Neighbourhoods) shipped single-space first before cloning per group.
The three irreversible choices are right: CID as identity, blobs off the DHT, and integrity/coordinator discipline.
They are why a pivot is available.

**But** the content integrity zome is an omnibus: about 4,800 lines, with **75 entry types and 225 link types**.
Holochain allows 256 link types per integrity zome. Content sits at **225 of 256**.
It spans learning, community, identity, economy and infrastructure, and **duplicates identity's own types**.
**Correction (2026-09-06):** eight types overlap between content and identity, not five —
`Agent`, `AgentProgress`, `ContentMastery`, `ContributorPresence`, `Human`, `HumanProgress`,
`HumanRelationship`, and `StringAnchor`. `Relationship` is content-only, not a duplicate; imagodei's
overlapping type is `HumanRelationship`. Only `Human` and `HumanProgress` carry a "legacy" comment.

**Live-writer finding (new, 2026-09-06):** for every one of those eight types, no DNA is the live
writer today — elohim-storage's SQLite is. `POST /db/relationships` writes Diesel with
`dht_anchor_hash` null (`http.rs:9050,9061`). The only DHT writer for Human/Agent is the genesis
seeder into imagodei.

These duplicate declarations create an authority question: which live authoring and reading paths
use which definition? Their presence alone does not establish two active authorities over the same
record. The coupling is real; the authoritative home and the proposed split need that path evidence.
Every DNA's membership check is also a permissive stub, and no role commits to a progenitor.
Every space today is therefore an open network whose hash binds to no root.

**Verdict: accepted as the technical hypothesis recorded here; sequencing is decided in the
reimplementation plan under D1/D5/D6 (§0b).** Cloning the content role as it stands
would clone the omnibus into every household. A split requires a lineage crossing at the affected seam. §10.6 combines the content zome changes into one crossing.
The Evolution epic just proved that crossing machinery on the mesh. So: **the zome split comes before any
household space.** §9.1's role table stands. The content role that goes per-holon is the *split* one.

### 10.2 "Your storage layer is legitimate — with two organs over the line."

**Finding.** A legitimate projection is rebuildable by replay, never originates truth, and fails to staleness rather than divergence.
By that test, the storage layer is a proper peer-hoster. Two organs cross the boundary:
**read-time reach enforcement** and **per-host custody stamps**.

Every full-arc peer *holds* household bytes. Only our code declines to serve them: patch the binary and it can serve everything.
That puts authorization outside the validated substrate, rather than providing caching.
The reviewer would defend everything else to upstream, including the honest full-or-zero arc actuator and the node-local follow-set
("following is consent, and consent is node-local").

The measured size is about 330,612 lines of Rust across 563 files in the storage crate, counted 2026-09-06.
`http.rs` alone has 19,918 lines, against a 4,816-line content integrity zome.
Design gravity has been outside the DHT for two years.
The pivot *increases* it, because cross-space reads by CID become storage-side joins.

**Verdict revised during product refounding, 2026-09-06.** The earlier acceptance said membership
would retire read-time reach. The couple's-note and blind-custody stories contradict that conclusion
(§0a). Storage must still distinguish who holds bytes from who may read them, and demonstrate
confidentiality where the holder is not a reader. Cross-space projection and offline access remain
legitimate work; their authorization contract needs design and evidence.

### 10.3 "The pivot is right. Six things will bite."

1. **The per-cell cost was unmeasured** when this document went out (9.3 was a placeholder). Community
   experience: clone-per-group works around 5–20 cells with *small* zomes and hurts as cells grow heavy.
   Ours is heavy (10.1). *Verdict: measure before promising; 9.3 now carries the numbers.*
2. **A membrane proof cannot do a DHT read** — `genesis_self_check` runs before the chain exists, so the
   proof must be checkable against something in the DNA itself. *Verdict: accepted; 9.2 already says so.*
3. **Anything in DNA properties folds into the hash:** a per-household founding key gives each household a distinct DNA hash, not merely a distinct seed.
   So "per cell" release channels are really "per DNA hash," while today's hApp manager stale-check inspects only role structure.
   *Verdict: accepted, not a blocker: clones share integrity wasm and coordinator hot-swap already applies per cell, but adoption must enumerate cells by role (§7).*

4. **Cross-space reference by CID kills link traversal**; every cross-holon read is a storage-side join.
   *Verdict: accepted (see 10.2).*
5. **Per-space arc is fork-gated, and §7 had contradicted §6½ by calling it "later, separable."** A phone in
   fifteen spaces is full-arc in all or zero in all until the hint lands. *Verdict: accepted; §7 corrected
   above. Phones stay spokes to a household hub until then.*
6. **Blocks scale with spaces.** On our own mesh one rejected write after a seal blocked a cell permanently,
   with no unblock. Multiply that by every household. *Verdict: accepted; this is why the unblock API moves
   ahead of any household space (10.5).*

### 10.4 "On the fork: three patches, each with an open upstream PR — and refuse sharding."

**Finding, per change.** Cross-relay fix: upstream it. Jemalloc: an image choice, not a patch.
Sys-validation backoff: make it a config knob and PR the knob.
A `list_blocks`/`unblock` admin API is **the best contribution we have**: small, genuinely missing upstream, and able to retire our worst risk.
The per-space arc hint is small, PR-able plumbing for a field the conductor already ignores.

**Fractional sharding: refuse.** Upstream built dynamic arcs, removed them in the rewrite, and has restoring them on the roadmap.
Re-implementing op filtering by arc, out-of-arc reads, and validation-coverage accounting on a moving 0.x line is a multi-year commitment.
The target would be rebuilt underneath a small team doing that work.

Carry at most three patches, each under ~200 lines and each with an open PR.
Stop pinning the fork submodule as reference-only so it can be bisected against upstream.

**Verdict: accepted in full.** The earlier draft's "track and contribute" policy has been replaced in §6½.
We do not attempt fractional sharding. We carry the unblock API and the per-space arc hint as PRs first, fork
second.

### 10.5 "What a person feels."

Joining a household must feel like tap-accept on an invitation. If anyone sees proof bytes, we have failed.
**Promotion gets better**: "share to the church" becomes a visible, witnessed, attributable act rather than a flag. Lead with it.

**Reading the commons at zero arc gets worse**: no local authority, tail latency on network reads, and broken offline access.
The projection DB and pin-what-you-read hide this. Pinning is content-level caching and must never be confused with arc.

**A phone in fifteen spaces is the weak point** until the per-space hint lands.
Keep phones as spokes to a household hub, as the seam map already draws.
**Recovery by people outside the household** is the strongest argument in the document. Identity never goes per-holon.

Hide seeds, DNA hashes, arcs, cell counts, proof bytes, and re-publish mechanics entirely.
Show who can see this, who holds a copy, and who witnessed the move.

**Original verdict: accepted as the UX contract for the epic. Reopened in §0a.** Invitation and
sharing labels are only part of the experience. Membership disagreements, selective sharing,
departure, supported agency, and offline continuity must shape the architecture before it is hidden.

### 10.6 The recommendation, resequenced

The reviewer recommends the §9 pivot in a different order than §7 originally proposed.
Household spaces before an unblock API would be a support catastrophe.
Per-holon spaces before the zome split would copy the omnibus into every holon. What follows is the
technical hypothesis recorded here; sequencing is decided in the reimplementation plan under
D1/D5/D6 (§0b):

1. **Fixtures clone, instrumented.** Raise `clone_limit` and give the seeder a cell selector.
   No fleet peer follows the clone. Measure per-cell cost in the same week (9.3).
   Risk: low. The only way to fail is to do it without instruments.
2. **`list_blocks` / `unblock`: upstream PR plus a carried patch.** Risk: medium (fork discipline).
   Nothing downstream, including household spaces, proceeds before it lands.
3. **The omnibus split as one lineage crossing**: remove identity duplicates, economy types, and infrastructure types from the content zome.
   In the *same* hash move, land a real membership check, a lineage record, and a non-null progenitor.
   Risk: high. This moves the fleet's largest space to a new hash, with a reinstall path that still mints keys.
   The Evolution epic exists for this crossing. Doing it after the pivot would mean doing it per household.
4. Only then: household spaces, with membrane proofs.
5. Explicitly not: fractional sharding.

**Original verdict: adopted as the plan.** Household notes, originally §7's "second slice", became
step 4. Product refounding (§0a) reopens the household mapping and the claims about authorization.
This is the recorded technical sequence to reassess against the human journeys, not evidence that
those journeys have been designed or delivered.

### 10.7 The reviewer's verdict, verbatim (historical annex — superseded 2026-09-06)

*Historical review retained for traceability. Its claim that membership retires read-time reach is
challenged by §0a and the revised §10.2; this quotation is not the current product conclusion.*

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

Two days before this document, Sacha Pignot (hAppenings Community) published `holochain-agent-skills` (Apache-2.0).
It has one skill, twenty references, eight workflows, seventeen templates, and a compiling example hApp.
It uses the same HDK and HDI versions as our DNAs and cites crate sources.
Its CI gate fails when a document teaches a removed API.

His own app is single-space with a clone limit of 0.
The cloning and membrane material therefore derives from upstream rather than app experience, but it is careful.

**Adopt as-is (planted as a package with attribution):** membranes, source chain, countersigning, and testing (multi-conductor Sweettest and partitions).
Also adopt the 0.6→0.7 upgrade break list, troubleshooting, cryptography, and scheduling.

**Adapt:** cell cloning and the zome-review checklist.
For cloning, take the manifest mechanics and `clone_only` panic verbatim.
Add our holon vocabulary and the "a cell is a full participant" cost rule.
For zome review, take the API items but *drop* the default path-plus-agent discovery link on every entry.
That query-index link pattern exceeds our link budget: the content zome already sits at 225 of 256 link types.

**Write ourselves, because nothing exists:** per-space arc policy, the head-plane cost model, holon placement, promotion by witnessed re-publish, and cross-space reference by CID.
His networking reference stops at the global knob. It does not know about the clamp or that the factor is conductor-wide.

**Contribute back:** the arc clamp and conductor-wide finding, which his repository explicitly asks for.

*Grounding for this document: the fork conductor at the fleet pin (`elohim/holochain-conductor` 25dd2d0be,
`crates/holochain_p2p/src/local_agent.rs:133`), `kitsune2_api` 0.5.1 `DhtArc`, the alpha peer store via
`GET /db/p2p/conductor-diagnostics` (five spaces, 32/35 full-arc entries), doorway-A `GET /db/stats`
(3,442 content rows), the Nachalah allotment epic (tiers on DNA seams, arcs negotiated by the elohim), the
Holochain Evolution epic §5 (dual-cell bridging) and §11.3 (arc-policy hook), the arc policy and actuator
modules in elohim-storage, and the 2026-09-06 ruling recorded on `services/arc_policy.rs`.*
