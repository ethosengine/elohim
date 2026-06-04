---
title: "Qahal EPR — The Household Living-Core Lattice"
id: qahal-epr-household-lattice-design
status: Draft
class: protocol-canonical
domain: D7
topic: [qahal, household, collective, dwelling, hub, formation, reciprocity, reach, presence, lattice, epr]
cites:
  - qahal-architecture-vision | gospel-tier qahal doctrine (one primitive, rubric-configured; Qahal as Layer-0 identity-anchor EPR) — this lattice REFINES it with the household-scale story and the two membership-acquisition flows | sha256:6a519b464b586832
  - experience-story-epr-design | the canonical EPR-as-graph-node pattern — defines what an EPR is so §2 can define the Qahal EPR as a composition, not a new primitive | sha256:b1dc5838ffab2e5d
  - genesis/docs/superpowers/specs/2026-05-23-multi-collective-collaboration-epr-design.md
  - elohim-hub-boundaries-design | hub trait + DwellingHub/CollectiveHub split + open questions Q1/Q2 (hub identity, multi-blade peer) — the place-axis hub role this lattice positions as compute-at-place | sha256:d7ffa707a34d126f
  - mutual-storage-replication-dwelling-hub-design | first REA compute-commitment instance; intent-first/observed-second pattern that §4 lifts from storage to reach; hub-is-a-role posture the dwelling thesis must reckon with | sha256:5596799dbb456bc2
  - d1-through-d5-node-and-household-canon | settled canon: D2 household = Collective(kind household), place-as-first-class deferred to v2 — the lattice keeps D2 and names the dwelling thesis as that v2 | sha256:5ee9472bbefad806
  - genesis/docs/plans/2026-03-15-qahal-community-directory-design.md
  - recovery-protocol-phase-2-revised-design | graduated recovery authority + StewardshipGrant primitives + the IntimateQuorum/Dissolution structural separation the capability-arc thesis honors | sha256:9d1844484ed64de4
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - tiered-quilt-stewardship-design | donut tiers (free/dwelling/collective/commons) + §4 quiltPolicy classes — the substrate the steward-finding flywheel shards onto | sha256:9f9c6a1c391712b3
  - genesis/data/timeline/backlog/qahal-household-collective-first-class.md
refines: genesis/docs/superpowers/specs/2026-05-21-qahal-architecture-vision.md
informed-by:
  - genesis/docs/superpowers/specs/2026-05-23-multi-collective-collaboration-epr-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
---

# Qahal EPR — The Household Living-Core Lattice

> The household is not one worked example among many — it is the living core: the
> foundation, the seed, the driver. The protocol becomes embodied at the dwelling,
> where care is given and received, where presence is shown. — manifesto

This seed gathers the qahal work into one frame. It is deliberately **thin**: it names
the lattice, the doctrines, and the spec family, and points at the deep specs. It is
the canonical home the architecture/ tree lacked for household-collective formation
(MAP Gap: no D# seed owned it; D7 is the nearest domain and this seed claims that slot
for the household-scale story, leaving cross-collective coordination to
multi-collective-collaboration-epr).

## 1. The lattice — two axes, each with structure and time

| | **Structure (what is)** | **Time (what changes)** |
|---|---|---|
| **People axis** (imagodei · qahal · mishpat) | Household formation + the default reciprocity bundle | The capability arc: aging-in (sponsored kids), aging-out, decline, medical stewardship, planned death — and **unexpected death as a separately-modeled intervention** |
| **Place axis** (dwelling · hub) | Dwelling as place-grounded entity; presence duality; the directory; hub = compute-at-place | Mobility seams: relocation, member moves, dwelling-class transitions (apartment → ADU → facility) |

Four entity **roles** populate the lattice. Only the first two have substrate today;
the second two are held theses (§6):

| Role | Grounds | Presence duality | Substrate today |
|---|---|---|---|
| **imagodei** (person) | identity | human ↔ contributor-presence (UNCLAIMED→STEWARDED→CLAIMED) | canon, shipped |
| **qahal household** (people-collective) | relationship | member ↔ invited | `Collective` + `Membership` (imagodei DNA); formation spec lands the ceremony |
| **dwelling** (place) | land / location / real world | on-protocol dwelling ↔ dwelling-presence (global directory) | vocabulary token (`replicates-dwelling`), HubKind value, donut tier — **not an entity** (D2 deferred place-as-first-class to v2) |
| **hub** (compute-at-place) | the dwelling's compute organ | present ↔ absent (hub-optional floor) | role dialed by capability, never notarized (hub-boundaries Q1/Q2 open) |

The **seams** are the edges between roles over time: household×dwelling is
many-to-many across a life (a family moves; a kid leaves for a dorm while staying in
the household; a grandparent moves into the ADU). Each **dwelling class** —
apartment, dorm, ADU, state-ward, retirement community, nursing home, temporary
housing — is an *authority-regime template* on those edges, not a new entity.
Capability decline, medical stewardship (power-of-attorney-shaped delegation), and
death are **person-axis stories, not dwelling stories**: they follow the person
wherever they dwell; the dwelling class only modulates context.

**The two ends of life share one mechanism.** The `sponsor_cid` + StewardshipGrant
shape that bounds a child's entry into the household is the same shape that unwinds
for an aging parent — graduated authority, never reset (the Jasmine principle),
instantiating the rea-compute-commitment §5 guardianship row at both ends.

## 2. Qahal EPR, defined

An EPR is a content-addressed graph node whose meaning is realized through typed link
couplings (experience-story-epr). Qahal is already canon as "a Layer-0
identity-anchor EPR … a category of EPR, not a new primitive"
(qahal-architecture-vision §3.1). The **Qahal EPR for a household** is therefore a
*composition, not an entity*:

- **node**: the `Collective` entry (household character declared in its charter/rubric
  at init — see formation spec; D2: household reuses `collectives`, no new types)
- **edges**: `Membership` entries (each member's own agent authoring their belonging),
  the default reciprocity bundle (`custody-blob` ambient, `delegates-compute`
  explicit, `replicates-dwelling` cross-dwelling), StewardshipGrants for minors
- **rubric**: the configuration that makes this collective a *household* — including
  which membership-acquisition flow it runs (§3)

Identity is the collective's action hash (`collective:{action_hash}`, as
hub_resolver already resolves); slugs (`family-dowell`) are display aliases resolved
at the edge. `reach:"household"` is **not** a Reach literal — household-ness lives at
the governance-layer projection (`governanceLayer:'family'`); the Rust `Reach` enum
has no Household variant and designs must not bake one (reach-enum reconciliation is
a standing precondition, roadmap #13).

## 3. Two membership-acquisition flows, rubric-selected

One `Membership` entry type; two ways of coming to belong; the collective's rubric
selects which:

1. **Graduated onboarding** (`request_membership` → standing/attestations →
   `attest_membership`): petition, then earned recognition. The general qahal path —
   study groups, coops, congregations — already designed
   (qahal-collective-membership-dht).
2. **Recognition of the given** (`affirm_membership`, new): the relationship
   pre-exists the protocol — spouse, parent-child, intense proximal intimacy from day
   one. Nobody applies to be family. The ceremony **affirms a belonging that is
   already real**; the substrate witnesses, it does not gate.

The household rubric selects recognition-of-given. Dwelling-class authority regimes
later slot into the same rubric surface: the ADU grandparent is an *affirmation*
(relationship given); a new roommate is *graduated* (standing accrues); an
institutional dwelling (nursing home, state wardship) configures bounded-authority
parties into the rubric. One primitive, many configurations — exactly the
qahal-architecture-vision posture.

## 4. Reach doctrine — intent-declared, observed-earned, witness-without-authority

Creation is never gated; **reach is graduated**. Every record is born with an
*intended* reach declared; its **effective reach = min(intended, earned)**. Records
whose earned reach lags their intent batch privately on their creator's device
(agent-scoped) and **drain** as validation stories accumulate, elevating toward the
earned/warranted stasis. This is the dwelling-hub spec's intent-first /
observed-state-second REA pattern, lifted from storage to reach itself.

- **The household is self-validating**: intended-intimate reach is fully earned by
  the affirmation set itself — member affirmations ARE the observed validation. A
  household never waits in anyone's outbox. (Minting a household-class collective is
  in principle a gated act on the graduated-capability surface, but the bar is
  deliberately near-floor — a Human entry and a device suffice; declared intent is
  the qualification. The Grandma Standard holds at creation, not just recovery.)
- **Witness-without-authority**: anyone may create a record on behalf of another (a
  collective, a dwelling entry, a contributor presence — the Google-Maps-community-
  entry shape). The witness earns a stewardship contribution credit (an REA event),
  and the record's validity is *strengthened* by the witness's authority disclaimer:
  "I created this; I claim no authority over it." This is the ContributorPresence
  STEWARDED state generalized from people to any record. Recognition flows first;
  the rightful party claims later; accumulated standing transfers by right
  (`claim_recognition_transferred_*` fields are reserved; the transfer executor is a
  named gap — resilience epic Part V).
- **The steward-finding flywheel**: a witnessed collective is inert until it finds
  its stewards. Each steward connection elevates earned reach → the record gossips
  at that reach → its blobs shard onto that reach-level of the quilted substrate
  (dwelling → collective → commons donut tiers) → individual hosting burden offloads
  to the quilt and commons → stewarding gets cheaper → the next steward arrives.
  Burden distribution is the reward loop.
- **Claim incentives**: value accumulating through an unclaimed collective is what
  makes claiming worth it, and the transferred standing should offset the sharding
  cost stewards bore pre-claim. Incentive-stability analysis is explicitly future
  work (see backlog: witnessed-records-reach-flywheel).
- The household is the flywheel's **first turn, degenerate by design**: stewards
  found instantly (the family), reach self-supplied, blobs sharding at household
  tier from minute one — "stewarded bytes for people you love" is the flywheel at
  N=3.

## 5. Drive doctrine — deterministic floor, elohim ceiling

- **Floor (now)**: formation is a deterministic choreography over coordinators,
  woven into device setup/onboarding, fully operable on the household LAN with no
  doorway and no hub (hub-optional honored at ceremony level). Automatable by the
  seeder as the personas' real agents.
- **Ceiling (graduates in)**: the elohim agent facilitates — narrates, sequences,
  attests completion — and **never gates**. Complexity is subsumed upward, the floor
  remains.
- **Realism ladder policy** (from the 2026-06-04 seed-realism audit): every seed
  module declares its rung and why. Corpus body stays rung 0 forever (replication
  carries bytes; anchoring a 50-minute corpus is a non-goal). Everything that is an
  **inter-party agreement, consent, or identity-binding climbs to rung 3**
  (authored by each persona's real conductor agent) — fabricated-key reciprocity is
  the centralized attestation the protocol exists to prevent (the terrance-drift
  RCA is the standing proof). Content authorship/attestation splits off the corpus
  and climbs.
- **k8s is not the architecture**: deployments.json / cluster-state model
  compute, hardware, and network for now; at maturity they are subsumed into
  peer-native modeling under EPR compute contracts (brit/rakia direction). Design
  lands at the peer-native home; k8s artifacts are derived projections.

## 6. The spec family

**Active spine:**
- `genesis/docs/superpowers/specs/2026-06-04-household-formation-ceremony-design.md`
  — the formation ceremony, default reciprocity bundle, drive stages, fixtures +
  retirement gate, household quiltPolicy class, scenario architecture.

**Held theses (each born-linked here; one backlog item each):**
- **Dwelling as first-class place-grounded entity + presence duality + global
  directory** — D2's deferred "place-as-first-class v2". Less greenfield than it
  sounds: the shipped Place/SpatialContext/H3/CarryingCapacity subsystem is the
  substrate; ContributorPresence is the duality donor; the witness mechanic (§4) is
  the directory's activation engine. Contradicts the current "hub is a role, not an
  entity" posture — needs its own p2p-design-gate.
  → backlog: `dwelling-first-class-entity.md`
- **Mobility seams + institutional authority regimes** — household split/merge,
  relocation, multi-membership precedence, dwelling-class templates (dorm,
  state-ward, retirement, nursing home, shelter). Finding: the primitives exist;
  the **lifecycle choreography** is missing.
  → backlog: `household-mobility-seams.md`
- **Capability arc + death** — the steward↔stewardee gradient across decline
  (remote-independent grandparent → ADU co-located → facility), medical
  stewardship, planned death; **unexpected death modeled separately** as an
  intervention (the recovery spec already structurally separates IntimateQuorum
  from NetworkWitness::Dissolution — honor that separation).
  → backlog: `capability-arc-stewardship-gradient.md`
- **Witnessed records + reach flywheel** — the §4 mechanics in full: drain queue,
  witness credits, authority disclaimer shape, steward-finding, claim-incentive
  economics and stability.
  → backlog: `witnessed-records-reach-flywheel.md`

## 7. Roadmap posture

This lattice keeps **single-household coherence primary** (vision-readiness roadmap:
Sprint 1 care-loop and Sprint 2 grandma-recovery are #1/#2; collective/network-scale
diffusion is held at #6 *by design* — "ranking network-scale above the
single-household seed would invert the gospel"). This seed does **not** resurrect:
network-scale collective coordination, the alpha-cluster cross-node recovery
rehearsal (env-blocked), bulk-seeded reciprocity as an answer (the fork is settled:
emergent, with marked interim fixtures), or new DHT entry types for
household/dwelling (the p2p-design-gate verdict stands: zero new types).
