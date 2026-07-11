---
title: "History: The community-compute founding vision arc (March 2026)"
id: community-compute-founding-vision-arc
type: history-gotcha
status: noted
tier: history
created: 2026-06-11
topic: [community-compute, founding-vision, family-node, replication, stewardship, economics, explainability, design-arc]
# Provenance breadcrumb: the two retiring island docs this record distills.
# (elohim/holochain/docs/README.md and ARCHITECTURE.md were context for this record;
#  README is being reconciled, not retired, and is cited below as a live path.)
derived_from:
  - elohim/holochain/docs/ARCHITECTURE-GAP.md   # retired to git 2026-06-11 (holochain docs island recompose; authored 2026-03-10)
  - elohim/holochain/docs/COMMUNITY-COMPUTE.md  # retired to git 2026-06-11 (holochain docs island recompose; authored 2026-03-10)
canonical:
  - genesis/docs/content/elohim-protocol/history/2026-06-01-dht-is-a-notary-not-a-byte-store.md
  - genesis/docs/content/elohim-protocol/resilience/README.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
cites:
  - storage-dual-plane-design-arc | same-day sibling arc that owns the ContentLocation/replication-sketch never-shipped verdicts this record points at instead of re-deriving | sha256:2315c84345a2ef3c | path: genesis/docs/content/elohim-protocol/history/2026-06-11-storage-dual-plane-design-arc.md
  - d1-through-d5-node-and-household-canon | the canon that superseded the Stage 1-4 ladder vocabulary | sha256:5ee9472bbefad806 | path: genesis/docs/content/elohim-protocol/history/2026-04-19-d1-through-d5-node-and-household-canon.md
  - doorway-two-axis-scaling | graduation flywheel + axis-2-shrinks — where doorway-becomes-one-node-among-many and the agency ladder shape now live | sha256:36fb15e24ceaf8b2 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-11-doorway-two-axis-scaling.md
  - elohim-hub-boundaries-design | the hub design the hub-optional floor constrains — Stage 1/2 humans first-class, count humans carried not nodes | sha256:d7ffa707a34d126f | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md
  - wave3-valueflows-hrea-interop-design | the designed-pre-implementation VF/hREA interop surface the vision's hREA bet still awaits | sha256:c8d903ad73f0284d | path: genesis/docs/content/elohim-protocol/architecture/2026-05-20-wave3-valueflows-hrea-interop-design.md
  - requests-offers-application-design | §D.20 Layered Commons + friction-gradient limitarianism — where the commons fund evolved | sha256:321ac092b956fe8e | path: genesis/docs/content/elohim-protocol/architecture/applications/requests-offers-application-design.md
  - governance-epic | epic | sha256:be850529ab645a30 | path: genesis/docs/content/elohim-protocol/governance/epic.md
  - genesis/docs/content/elohim-protocol/social_medium/epic.md
  - genesis/docs/content/elohim-protocol/lamad.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - per-substrate-limitarian-governor-design | rates-as-governed-EPR through propose→vote→tally — where constitutional rates evolved | sha256:5d10a556e2ec7a14 | path: genesis/docs/superpowers/specs/2026-06-09-per-substrate-limitarian-governor-design.md
  - trust-compute-gradient-brainstorm | 2026-04-30-trust-compute-gradient-brainstorm | sha256:89c493c73ff6b06b | path: genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
  - genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md
  - genesis/data/timeline/backlog/storage-island-harvest-residue.md
  - elohim/holochain/docs/README.md
  - elohim/epr/src/reach.rs
  - elohim-cache-core-gospel | the extracted crate gospel — live home of the holochain-cache-core component the vision named | sha256:359677d53fb0dcd7 | path: elohim/elohim-cache-core/CLAUDE.md
  - elohim/elohim-storage/src/api/compute.rs
  - app/elohim-app/src/app/shefa/components/shefa-dashboard/shefa-dashboard.component.html
memory_anchors:
  - project_hub_optional_floor
  - project_rea_compute_commitment_primitive
  - project_inventory_exchange_not_byte_replication
---

# History: The community-compute founding vision arc (March 2026)

> **Hot-context pointer (the one sentence to remember):**
> March 2026 named the fourth compute model — community-scaled, embodied investment,
> replication-follows-relationship — and every load-bearing *idea* in it became canon
> somewhere stronger (notary split, mutual-aid stewardship, patron-CDN, REA compute
> economics, Dunbar-by-design), while every concrete *mechanism* it sketched
> (ContentLocation entry, 4-layer GB allocation, 6-value Reach, dollar-equivalent
> cost meter) shipped differently or never — cite the canon, let git keep the sketches.

## What the vision argued

ARCHITECTURE-GAP.md (2026-03-10) diagnosed both inherited compute models as failures for
the protocol's purpose — client-server scales but is captured; pure agent-centric DHT is
sovereign but "chokes at 3000 entries" with no query path (ARCHITECTURE-GAP.md:96-99,
git) — and named the missing piece *relational scaling*: compute scales with investment,
responsibility distributes by relationship, cost is visible, doorway is "a necessary
compromise, not the solution" (:129-135, :177). COMMUNITY-COMPUTE.md built the model:
sovereignty is ephemeral and made real by community (replication / backup / recovery /
amplification / safe harbor, :20-30); every participant runs a "family node" with four
allocation layers (SOVEREIGN / RECIPROCAL / INVESTED / GIFT, with GB budgets, :150-194);
listeners choose *who* to support and the system pins intelligently (:199-232); creators
see network health, not replicas (:234-268); cost is visible but not monetized (:300-335);
hREA tracks value flows around compute under constitutional rates and a commons fund
(:475-522); outcomes are explainable as a matter of justice (:524-573); and values align
*to* layers by negotiation (:575-630). All page/line refs are to the retired bodies in git.

## The argument that became canon

The trust-layer/data-layer split — "The DHT should store COORDINATION data, not CONTENT"
(ARCHITECTURE-GAP.md:215-227, git) — is the earliest substrate articulation of what is now
the notary canon. April's storage dual-plane doc carried it into design (sibling record:
`2026-06-11-storage-dual-plane-design-arc.md`), and the canonical homes are
`2026-06-01-dht-is-a-notary-not-a-byte-store.md` plus the tiered-quilt design's three truth
layers (`2026-05-11-tiered-quilt-stewardship-design.md` §2). One reframe matters when
reading the original: the founding empirical claim was a hard ceiling ("chokes at 3000
entries"); the canon restates it as *cost layering* — the DHT is the most expensive layer,
and putting bytes or operational who-has-what there is paying notary cost for operational
data. Cite the cost framing, not the magic number.

## Ideas that won — where each lives now

- **Community-makes-sovereignty-real** (the :20-30 table). Canon: the resilience epic —
  mutual aid as protocol primitive, recovery as the testbed
  (`genesis/docs/content/elohim-protocol/resilience/README.md` Parts II-IV), with the
  social-recovery mechanism live as the `CustodianCommitment` entry type
  (resilience README:365-369 cites `content_store_integrity/lib.rs:3289`).
- **Replication follows relationship.** Canon: resilience Part V — the three stewardship
  classes (encrypted / social / commons) are *queries over the existing REA Commitment
  ledger* partitioned by reach and `resource_classified_as` (README:310-316), plus the
  tiered-quilt custody-quilt commitments. The vision's 4-layer family node survives almost
  isomorphically: SOVEREIGN→"used (your own things)", RECIPROCAL→encrypted custody,
  INVESTED→social custody, GIFT→commons custody — but as ledger partitions, never as the
  sketched `sovereign/ reciprocal/ community/ commons/` directory layout (which has no
  trace in `elohim/elohim-storage/src/`).
- **Listener's view** ("you don't choose WHAT to store, you choose WHO to support; the
  network uses your capacity intelligently"). Idea won, mechanism evolved: the system-side
  half is the tiered-quilt `TierController` + archetype-tuned `HeuristicClassifier`
  (tiered-quilt §2, :169-187) riding on Plan 1 diverse auto-distribute; the human-side
  half is Part V's stewardship bar (README:244-259). The pledge-allocation idea continues
  in the pledge-tier/donut-clamp mechanism (tiered-quilt frontmatter cite to
  `2026-05-28-mutual-storage-replication-dwelling-hub-design.md`).
- **Creator's view** (supporters, replication health, resilience score). Idea won as
  Part VI's patron-CDN ("Aunt Carol's recycled laptop ... is the CDN edge",
  README:371-377); the dashboard itself is still named-but-unfinished — the
  storage-stewardship summary route's "per-creator patron-CDN composition" drill-down is
  an explicit open item (README:415). The sketched `ReplicationHealth` TS interface
  (`sdk/src/replication-health.ts`, COMMUNITY-COMPUTE.md:719-729 git) never existed —
  no such file at any commit; the interface name appears nowhere outside the retiring doc.
- **Visible cost without financialization.** Canon: Part V's bar ("legible without
  quantifying it into a market", README:324) and the tiered-quilt grandma tile ("compute
  contribution +1.6 GB-hours, served 23 draws", tiered-quilt :129). See the philosophy
  check below for the part of this that *inverted*.
- **Dunbar as natural constraint** (:329-335 git). Canon: "Dunbar by design" in the
  social-medium epic (`social_medium/epic.md:111`), the trust-compute-gradient §2.3
  (`2026-04-30-trust-compute-gradient-brainstorm.md:53-55`), and manifesto Dunbar-violation
  detection (`manifesto.md:333`).
- **Doorway becomes one node among many** (ARCHITECTURE-GAP.md:268-271 git). Canon:
  doorway two-axis scaling (axis 2 *shrinks* as doorway succeeds;
  `2026-06-11-doorway-two-axis-scaling.md`) and the delivery feature suite — peer-mesh's
  "Doorway becomes one source among many" (resilience README:375).
- **The Stage 1-4 progressive-agency ladder** (carried by the island README, which is
  being reconciled, not retired — `elohim/holochain/docs/README.md`). Superseded
  twice: vocabulary by the D1-D5 node/household canon
  (`2026-04-19-d1-through-d5-node-and-household-canon.md`), shape by the graduation
  flywheel "visitor → hosted human → app user → node steward"
  (`2026-06-11-doorway-two-axis-scaling.md` §graduation flywheel).
- **hREA for value flows around compute** (:475-493 git). The vision's sample event —
  "Agent X provided 50GB storage for Community Y" — is exactly today's
  storage-stewardship Commitments + `deliver-service` EconomicEvents (resilience
  README:302). The bounded-delegation half became the gospel-tier REA compute-commitment
  primitive: Mishpat `Commitment` with `action="delegates-compute"`, the anti-X-API-Key
  shape (`2026-05-25-stagespablob-substrate-correct-deploy.md:36,66,86`). The VF/hREA
  interop surface proper is designed, pre-implementation
  (`2026-05-20-wave3-valueflows-hrea-interop-design.md` — "until there is a
  `/api/v1/vf-graphql` endpoint ... hREA alignment is unstarted").
- **Constitutional rates + commons fund** (:494-522 git). Evolved, two homes: rates-as-
  negotiated-governance became community-ratifiable limits carried as a *governed EPR*
  (a Mishpat `Commitment` through the propose→vote→tally pipeline —
  `2026-06-09-per-substrate-limitarian-governor-design.md` §0); the commons fund became
  Layered Commons — Bridge Commons + Global Commons `fee_splits` with friction-gradient
  limitarianism making accumulation mechanically expensive
  (`applications/requests-offers-application-design.md` §D.20, :154-158). The solidarity
  direction (wealthy overage → commons → edge) survives as friction-gradient ratcheting
  to the Global Commons.
- **Explainability as justice** (:524-573 git). Canon: the governance epic — "The
  Explainability Moment" (`governance/epic.md:308`), the explainability-requirement
  proposal (:1000), and the appellant role's mandatory plain-language agent explanation
  (`governance/appellant/README.md:24`). The decision-trace-you-can-verify maps onto the
  DHT attestation substrate generally.

## Mechanisms that never shipped

- **`ContentLocation` DHT entry** and its sibling **`CommunityMembership`/
  `StorageAllocation`** (COMMUNITY-COMPUTE.md:352-364,704-713 git): never built — zero
  hits in `elohim/holochain/dna/` and repo-wide (the only `StorageAllocation` string is
  the unrelated `earningRateStorageAllocation` field in a generated shefa view). This is
  phase-0 verified and already recorded with the live replacement (Kademlia provider
  records + metadata-only inventory gossip) in the dual-plane arc — see
  `2026-06-11-storage-dual-plane-design-arc.md` "The paths not taken"; not re-derived here.
  Same verdict for the announce/request-shards/update-holder-list replication sketch
  (:381-399 git): the live shape is the reconciliation controller (P1) + provider records,
  per the notary record and the dual-plane arc.
- **The 6-value `Reach` sketch** (`Private/Invited/Local/Neighborhood/Municipal/Commons`,
  claiming `elohim-storage/src/reach.rs` — a file that never existed). It is a subset
  sketch of the geographic family whose drift across five-plus vocabularies is tracked in
  `genesis/data/timeline/backlog/reach-vocabulary-frontend-strand.md`. The live epr enum
  is the *relational* 8 (`elohim/epr/src/reach.rs:18-37` —
  private/self/intimate/trusted/familiar/community/public/commons). No reach vocabulary is
  canonized by this record; reconciliation is resilience-epic roadmap item 13.
- **The 4-layer GB allocation budgets** (~1-10 / ~10-50 / ~50-200 / ~5-20 GB): never
  implemented as allocation tiers. The quilt's temperature classes
  (drawn/stocked-warm/stocked/shelved) are *stewardship-intent* tiers, not relationship
  tiers; the relationship axis lives in reach + commitment-ledger partitions (above).
- **The creator dashboard** — still the one ❌ in ARCHITECTURE-GAP's status table that is
  honestly still open (as Part VI's named-but-unfinished drill-down), which is also the
  table-rot caveat: the same table's "Community replication ❌ Not built / Family nodes ❌"
  rows are dead wrong against today's tree (36 modules under `elohim/elohim-storage/src/p2p/`,
  `steward/node` runtime, household canon). Status-tables-rot-silently is canonized in the
  sibling record `2026-06-11-elohim-pillar-architecture-founding-arc.md`; not re-derived.
- **Stale liveness pointers for the curious**: the bootstrap path "doorway/src/routes/signal.rs"
  is now the live module `doorway/doorway-service/src/signal/`; `holochain-cache-core` is
  `elohim/elohim-cache-core/` (own gospel); "Automerge 3.0" is pinned at `automerge = "0.5"`
  in both `elohim/elohim-storage/Cargo.toml:146` and `steward/node/Cargo.toml:13` (the sync
  engine's disposition belongs to the SYNC-ENGINE.md record, not this one); ARCHITECTURE.md's
  "elohim-node lives at the project root" is stale — the P2P runtime is `steward/node`.

## Philosophy checks

- **Family-node-centric floor — inverted.** COMMUNITY-COMPUTE's model has *every
  participant run a family node* (:148 git), and README's ladder makes the Family Node the
  destination. The modern canon relocates the floor: **one device, no hub, is full
  participation** — hubs are graduations that add convenience and scale, never gates
  (memory anchor `project_hub_optional_floor`, constraining
  `2026-05-02-elohim-hub-boundaries-design.md`; that design's :69 also insists Stage 1/2
  humans are first-class and the unit of count is humans carried, not nodes). The ladder
  itself survives as the graduation flywheel; the *requirement* did not. Any future doc
  that quotes the vision's "every participant runs a family node" is quoting the
  pre-inversion floor.
- **Anti-financialization sharpened — with one live vocabulary tension.** The vision said
  "NOT a cryptocurrency or token system" (:319 git) yet also surfaced "Estimated value:
  ~$0.47 equivalent" (:308 git). Canon resolved toward the first instinct: Part V
  explicitly refuses leaderboards and market quantification, and the elohim agent is
  "counsel for the relationship" (README:324). But the second instinct shipped as
  scaffolding: the live shefa dashboard renders an **infrastructure-token balance with a
  ≈USD estimate and demurrage decay**
  (`app/elohim-app/src/app/shefa/components/shefa-dashboard/shefa-dashboard.component.html:450-451,489`),
  fed by a mostly zero-filled view ("token balance from produce events"; every rate/value
  field 0.0 — `elohim/elohim-storage/src/api/compute.rs:206-230`). Recorded, not blessed:
  the token/USD vocabulary on that surface predates-or-bypasses the Part V framing, and
  reconciling it is UX-philosophy work nobody has claimed.
- **BitTorrent/IPFS contrast deepened.** The vision's "meaning as incentive" (:281 git)
  became something stronger: attribution travels with the bytes — ContributorPresence +
  recognition flows are what make this "a civic substrate rather than a piracy substrate"
  (resilience README:300-306). The founding contrast underestimated its own best argument.

## The values-aligned-to-layers stack — diffused, no single home

The six-layer negotiation diagram (Data=Sovereignty, Compute=Solidarity,
Economic=Constitutional Justice, Trust=Transparency, Application=Agency,
Governance=Participation; "alignment emerges from negotiation, not imposition",
:575-630 git) never landed as a named artifact. Its rows each found a stronger home —
sovereignty in the hub-optional floor and stewardship-over-ownership; solidarity in the
resilience epic; constitutional justice in the limitarian governor + Layered Commons;
transparency in the DHT-notary canon; agency in the graduation flywheel; participation in
the governance epic — but the explicit *adjacent-layer negotiation* framing exists nowhere
canonical. Recorded, not blessed. Name-collision warning: the live
`architecture/governance-layers-architecture.md` is about specialist subagents, **not**
this stack — do not "rediscover" it there.

## Vision-remainder (the genuinely unhomed)

- **Education over compliance — decision-to-learning-path generation.** The vision's most
  distinctive composition: when a participant doesn't understand a governance/economic
  decision, an elohim generates a *personalized learning path scoped to that specific
  decision* (:546-573 git). The governance epic covers plain-language explanation +
  appeal (epic.md:308); lamad covers generate-path-from-content (`lamad.md:352`); the
  *composition* — explanation rendered as a learning path, "Explainability + Education +
  Participation = Legitimacy" — has no home in canon, code, or backlog. Candidate backlog
  item (lamad × governance).
- **Edge-hardware subsidy as a named flow.** The vision's commons fund explicitly
  subsidized "hardware for communities that can't afford it" (:507 git). Global Commons
  spending on protocol-wide public goods (D.20) covers it in principle; no named
  mechanism, event type, or scenario exercises hardware subsidy specifically.
  OPEN QUESTION: does the Layered Commons spend-side need a named edge-subsidy flow, or
  is it deliberately left to commons governance?
- COMMUNITY-COMPUTE's seven open questions (:743-751 git) mostly resolved into homes:
  sybil/free-rider/moderation → reach-earning gate + receiver pre-authorization + standing
  (resilience README:385-387, trust-compute-gradient); constitutional process → governance
  epic; cross-community economics → `2026-05-23-multi-collective-collaboration-epr-design.md`
  + D.20; offline resilience → household-nodes floor + tiered quilt; incentive
  bootstrapping → ContributorPresence accumulation (claim-and-transfer executor still a
  named gap, resilience README:304). None of these re-open here.

Everything else still-true in the two retired docs is already homed (notary record,
dual-plane arc, tiered quilt, resilience Parts V-VI, two-axis scaling, D1-D5, the reach
backlog strand, `storage-island-harvest-residue.md`). This record adds no new mechanism
claims — it is the map from the founding vision to where each piece now lives.
