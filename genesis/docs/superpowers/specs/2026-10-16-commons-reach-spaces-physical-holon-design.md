---
title: Commons reach spaces on the physical holon
id: commons-reach-spaces-physical-holon-design
status: Draft
class: architecture
date: 2026-10-16
context-tier: disclosed
steward: human:matthew
author: "Codex, drafted for the orchestrator (Claude) at the operator's direction"
tags: [commons, holons, reach, custody, membrane, sharding, composition, local-first]
graduation-trigger: >-
  Matthew rules on the open choices; the reach-enforced-everywhere and dataplane-convergence
  habits carry evidence for admission, projection isolation, cross-space succession and
  phone/hub budgets, including the nachalah-allotment outsider scenario and completed probes.
cites:
  - "commons-spaces-probe-a-report | Probe A (clone spaces) and probe B (fractional arc) results and raw-evidence references (2026-10-16) | sha256:6cfcd8332e25a77d | path: genesis/docs/superpowers/sprints/2026-10-16-commons-spaces-probe-a-report.md"
  - "holons-are-spaces-how-we-use-holochain | Holons, Spaces, and Holochain — What the Vision Requires Us to Reconsider | sha256:8307626aa810f8a2 | path: genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md"
  - genesis/data/timeline/backlog/arch-authority-in-integrity-backlog.md
  - "commons-pool-as-collective-party-design | A commons pool is a collective that is party to its members' commitments — the missing node under shared data, shared inference, edge serving and edge compute | sha256:095276e5ccec79b3 | path: genesis/docs/superpowers/specs/2026-10-04-commons-pool-as-collective-party-design.md"
  - genesis/research/2026-10-15-covenant-commons-working-mou.md
  - genesis/research/commons-facility-at-scale-2026-10-15.md
  - genesis/research/commons-composition-at-scale-2026-10-15.md
  - "memory-search-scale-three-seams-design | Memory, search and scale — an index is a measure, reach at replication, pool folds on arcs (the three seams) | sha256:5397da36e2c29635 | path: genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md"
  - "reliance-as-cite-shared-provenance-primitives-design | Reliance as cite — the first implementation of the shared provenance primitives, for code, the co-op grocer and the community-garden network | sha256:ed648f3378dba82a | path: genesis/docs/superpowers/specs/2026-10-15-reliance-as-cite-shared-provenance-primitives-design.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map — A Concern-Routing Atlas | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - .claude/skills/elohim-commons-design/SKILL.md
  - .claude/skills/p2p-design-gate/SKILL.md
  - .claude/skills/gradient-reading/SKILL.md
  - .claude/skills/local-first/SKILL.md
---

# Commons reach spaces on the physical holon

## 1. Context — why the bottom layer first

The README support button must rest on a foundation that is honest and scales. A supporter
must be able to tell where their pledge is held, who accepts it, what survives a lost device,
and what remains merely proposed. This draft proposes a hybrid model; it does not authorize
new spaces, an admission policy, a release of the project's Holochain fork (our modified
conductor), or money handling.

**BUILT** means inspected implementation or the dated experiment cited, not a fresh fleet
verification. **SPEC** means a proposed contract. **ABSENT** means no demonstrated complete
path in the cited evidence. The operator-approved plan of 2026-10-15 (session evidence,
not tracked), §4, supplies this scope. The 2026-10-15 lowest-layer reading (session evidence,
not tracked) guided inspection; its factual claims below cite repository sources directly.
No runtime was started or measured for this document. **Receipt** denotes the sprint report
`genesis/docs/superpowers/sprints/2026-10-16-commons-spaces-probe-a-report.md`; every Receipt
reference below resolves to that report. **Holons**, **Authority**, **Pool**, **MOU**, **Facility**,
**Composition**, **Three-seams**, and **Reliance** resolve to the corresponding paths in `cites`.
Line numbers name repository source evidence; Receipt section references name the sprint report.

A **DNA** is a Holochain application's validation rules and modifiers, identified by their
hash. A **network seed** changes those modifiers and therefore the network identity. A
**DHT** (distributed hash table) is the peers' shared validation and holding network. A
**physical holon** is that concrete space, not a social label. A **cell** pairs a DNA with
an agent key on a **conductor**, the process running Holochain. A **clone** is another cell
using modified DNA rules or modifiers. An **arc** is the portion of the DHT's hash range a
peer holds; **gossip** is peers exchanging records to converge.

**Reach** means the permitted audience and propagation of a record. A **capability** is an
explicit grant to perform an operation or obtain bytes. A **membrane** is an enforced
admission boundary. A **Collective** and **Membership** are existing community and membership
records; a pool is such a collective acting as a party to members' commitments. A **rollup**
is a summary with references to its contributing records. A **head** is the declared version
of an addressed object; the **head plane** is the recurring work of finding and reconciling
those versions. **Quiesce** is the time for the network to settle to a consistent quiet
state after changes; every new head adds to that work. A **CID** identifies content by its
bytes. A **zome** is a DNA module:
integrity zomes validate records; coordinator zomes author and read them. A **source chain**
is an agent's signed history. An **EntryHash** addresses record contents; an **ActionHash**
addresses a signed action. An **EPR** is an addressed protocol record; **REA** is
Resource–Event–Agent accounting. A **presence** represents a contributor held in trust until
claimed. A **lens** is a declared, versioned rule for interpreting records.

D0–D6 refer to the operator's own sealed decisions summarized in **Holons §0b**
(`genesis/docs/content/elohim-protocol/architecture/2026-09-06-holons-are-spaces-how-we-use-holochain.md:358–385`),
not decisions made by this draft. Sealed constraints remain in force: classify acts before
witnessing (D0); aggregates are
views, not DNAs (D1); peer validation is the admission gate (D3); name successor authority
and partition behavior before cross-space code (D4); group clones remain a gated candidate
(D6). D5 folds the omnibus split into the next necessary integrity crossing; absence of a
live writer does not make deletion safe (Holons §0b:372–373). D2 is not enumerated in that
summary, but its companion plan records it: everyday relationships, humans, presence and
comments already write to storage without DNA witnessing, so do not claim a “smaller
Holochain job” as a new achievement
(`genesis/docs/content/elohim-protocol/architecture/2026-09-06-ai-stewarded-commons-reimplementation-plan.md:121`).
D2 bears on this design's classification and projection claims and is retained here.
Automatic household-to-DNA mapping is retired (Holons:358–385). The older Holons §10.6 sequence,
unblock API → omnibus split → household spaces, is a recorded dependency discussion, not
a new authorization: Holons §0b explicitly supersedes its first-slice ordering. Fractional sharding
remains unshipped. Authority:284–291 forbids founder-key admission.

The operator's wording is controlling: “The one non-symbolic holon is physical,” “what the
holon can seat,” and “the commons is one degree removed from the human hand” (MOU, “Already
ruled”). The same ruling says the council is “plural and representative,” humans perform
its role “in the network's infancy,” and human sortition councils “audit and govern any
contestable result.” Sortition means selection by lot. This draft does not choose a council
size, an end to infancy, or a substitute for the human floor. Facility capacity remains
members' produce; commons-share custody belongs to that plural council (Pool:73–85).

## 2. What the physical holon is today

**BUILT:** the deployed bundle declares lamad, infrastructure, imagodei, mishpat and
node_registry as shared role networks, with cloning disabled in each role
(`elohim/holochain/dna/elohim/workdir/happ.yaml:24–92`). A Collective
created in imagodei does not create a cell
(`elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs:58–72`).
Reach-aware storage fanout changes distribution
but not DHT membership (`elohim/elohim-storage/src/p2p/fanout.rs:64`).
Capability-scoped confidential replication is **SPEC**, not a property to infer from fanout.

**BUILT, experiment only:** Receipt §Results demonstrates isolated clone content and member
propagation at the first sample **5.7 seconds** after writing. A seed-knowing outsider joined;
the entry was first observed on the outsider about **77 s** after its install (poll sample six).
No probe content appeared in the base cell;
base inventories did change through outsider genesis and initialization. Thus isolation means
separate content spaces, not byte-identical inventories or exclusion of unauthorized people.
The seed is the only demonstrated separator today; it is not an authorization credential.

Receipt §Results reports the marginal **10→20-cell** interval as approximately **6.2 MiB RSS,
3.2 MiB disk, 3.0% of a core and 383 B/s per clone**, averaged over the member conductors at
development cadence. RSS means resident process memory. The earlier interval includes a
one-time **150–210 MiB RSS / about 140 MiB disk per conductor** step; it must not disappear
from a startup budget. The receipt calls the sampled growth linear, not an extrapolation to
arbitrary clone counts. Its **20 cells include 15 clones**, not twenty clones.

**ABSENT:** steady-state production-cadence costs and phone measurements. The post-restart
production sample (**2.97% CPU, 321 MiB RSS**) is
a non-steady-state observation; steady-state cost unknown. Only **0–2 of 50 agent-arcs**
were full. Restart also reduced memory at development cadence. These are Receipt §Results
figures, not evidence that changing cadence alone saves that memory. The experiment used
empty conductors without storage, direct peer connections, and shared bootstrap/relay
services; it did not measure the complete application or independent failure domains.
Receipt §Notes leaves `blockedIncoming` unexplained and the startup step unattributed.

**BUILT constraint:** validated deployment arcs are full or zero
(`genesis/seeder/src/validate-deployments.ts:182`). The tracked reconciliation records
fractional arcs compiled out on fork `2b334df7973d`
(`genesis/data/timeline/backlog/dht-scale-envelope-and-web2-projection-at-planetary-scale.md:102`).
That is a dated source reading. For the pinned fork `96bcd2bee` Probe B verified it directly:
the pinned binary refuses `target_arc_factor: 0.5` at config parse (§4, Receipt B §P0).

## 3. The space model per tier

These are semantic tiers, not new reach enum values. Their physical placement is a **SPEC
recommendation for Matthew**, not a claim that the hybrid exists end to end.

| Tier | Proposed physical model | Boundary and budget consequence |
|---|---|---|
| Household | Hybrid candidate: private device working state first; shared-role public facts; capability-scoped private replication when built; optional restricted clone after the test below | No automatic cell per household. Sensitive roster/activity stays local until both metadata and serving boundaries pass. A hub is additional capacity, not a prerequisite for preparing an act. |
| Collective / pool | Shared imagodei Collective + Membership for public-purpose pools, commitments in their existing role; capability-scoped artifact custody; clone option for a separately justified validation community | A pool is a party, not a new entry type. A content clone alone does not move its membership or pledge authority. |
| Language commons | Hybrid: public shared-role declarations, bounded rollup shards held by a steward collective; dedicated space only after the same admission and budget gates | Readers do not each need a steward cell. The steward collective can have a physical space; the language-wide aggregate remains a view. |
| Planetary | Composed views of child rollups; public roots on existing role infrastructure initially; separately admitted steward spaces only where justified | No planetary aggregate DNA and no single holder of all source histories. No measured planetary capacity claim. |

The **written D6 reconsideration test** is conjunctive. A requesting commons must:

- Classify each act and identify its durable authority, validation and reconstruction path.
  Explain which resilience or validation failure shared-role records plus scoped replication
  cannot solve. Membership count, language and prestige alone do not establish need.
- State its failure model, recovery deadline, required independent holders, and communication
  needs of both acting and auditing councils. Show observed independence of power, network,
  location and administration; a set of processes on the same host is not this evidence.
- Present a device inventory and operator-approved budgets for steady-state and transition
  memory, disk, CPU, bandwidth, battery, startup, catch-up and head reconciliation. Measure
  the whole phone/hub workload with arcs regrown, intended cadence, stored content, hosted
  identities and the proposed number of role cells. Receipt's marginal costs are screening
  evidence only; do not multiply them into a phone capacity assertion.
- Pass peer-side unauthorized-join rejection and unauthorized-read tests, including storage,
  discovery, projection and cache paths. Prove correction, upgrade and recovery across all
  participating cells, not just clone creation.
- Name crossing and dissolution successors as in §6; demonstrate partition and restore.
  Matthew then explicitly selects the candidate and its budget. A breached budget reopens
  placement; it does not silently weaken admission or increase limits.

No tier currently passes that entire test. Remaining on shared-role records is acceptable
for public purposes; it is not an interim confidentiality promise for a private household.

## 4. Custody and arcs

**SPEC:** keep selected small-space holders at full arc and account for every such cell.
Readers at zero arc do not supply the space's recovery floor. Arc configuration is presently
conductor-wide in the cited design; selecting different arcs per space needs a separate
**fork change** (a new patch to that fork)
(Holons §7). Do not describe phone participation as independent full custody without its
measurement. Replica breadth must buy independent recovery, not merely headcount.

**PROBE-B: measured 2026-10-16 — a partial arc holds where gossip delivers, and leaks where
publication delivers.** Receipt B: `genesis/a2o/reports/recovery/commons-spaces-probe-b-2026-10-16/RECEIPT.md`
on the measuring device (raw evidence beside it; summary in the tracked sprint report). The pinned
fork cannot hold a fractional arc at all: `target_arc_factor: 0.5` is refused at config parse
(exit 42, "expected u32"). A **scratch probe binary** — the pinned fork plus a local, unpushed
commit that compiles kitsune2's partial-arc growth in and adds a `target_arc_span` setting — ran on
**one fresh joiner** at exactly half the ring. James was not used: no layer at this fork deletes
operations outside an agent's arc, so narrowing an established peer's arc cannot show a reduction.
The household's three conductors stayed on the pinned binary.

| Measurement | Result (Receipt B) |
|---|---|
| Arc percentage | **PASS.** Storage arc stayed Empty for about 10 min, then settled exactly at the half target on each of the four spaces whose DNA matched the household's (about 56 min on the 197,467-operation infrastructure space). It never jumped to full. A full-arc control joiner on the pinned binary was still Empty on its two largest spaces at 24 min, so a full-arc joiner can start as slowly; one control does not establish the cause. |
| Operation count ratio | **Split by space.** infrastructure: 84,344 integrated against Matthew's 198,717 (42.4 %), plus 27,993 still in validation limbo (56.5 % with them). node_registry: 96.7 %. mishpat: 98.9 %. |
| Held by basis, integrated operations | Counts only, not operation identities. For operations whose basis is readable from the action (agent activity, links): infrastructure's integrated operations include 36,156 in-arc against Matthew's 36,237 (99.8 %) and **6** out-of-arc against his **64,727** — consistent with a correct shard; its 27,993 limbo operations were not classified. node_registry's include as many in-arc as Matthew's (8,072) **and 3,388 of his 3,445 out-of-arc (98 %)** — a leak. |
| Out-of-arc read correctness | **PASS.** 120 of 120 records (64 outside the arc) read correctly through the joiner; no misses or errors. Whether each was served locally or by another peer was not recorded, and the leak makes local service likely, so this does not yet prove reads served by other peers under a correct shard. |
| Gossip completion | **PASS.** Rounds complete with every household peer on the four matching spaces; `hc-mesh.sh blocks` lists no block on Matthew, Jessica or James; no warrant in the joiner's log. |
| Savings | Only integrated-operation counts were compared. infrastructure: −57.6 % integrated, −43.5 % counting limbo. node_registry −3.3 % and mishpat −1.1 %, which is not a sharding saving. No matched disk, bandwidth, memory, CPU or throttling comparison was taken. |

**The approved "50 % ±10 % of Matthew" criterion was the wrong yardstick.** Operations are not
uniform on the ring: 70 % of Matthew's located node_registry operations and 36 % of his
infrastructure operations lie in this half. The test that means custody is "holds the in-arc set
and not the out-of-arc set", measured per operation (by identity, which this probe counted only in aggregate).

**Why it leaks** (source reading, key path verified; Receipt B §Mechanism). Gossip is correct: it
syncs only the intersection of the two sides' *target* arcs, and full-arc peers need no sharding
support. Publication is not: `holochain_p2p` publishes each operation to every responsive peer
near its basis with no check that the basis lies inside the receiver's arc
(`crates/holochain_p2p/src/spawn/actor.rs:1981` at fork `96bcd2bee`), and the receiving conductor
admits incoming operations with no arc gate. Why node_registry and mishpat are still being
published to a newcomer while infrastructure is not is PLAUSIBLE only: an operation is
republished until it holds five validation receipts, which a three-conductor household may never
supply. Receipt counts were not measured.

So sharding could reduce replicated operations, disk and gossip within a space — only the
integrated-operation count was measured, on one space — but at this fork it does not hold under
publication. It need not remove per-cell
runtime memory, and it does not implement a membership boundary. An empty arc can look cheap while
holding nothing; measure recovery and availability alongside savings.

Shipping is a later **operator-gated fork change → reviewed pin move → fleet roll**. A scratch
probe binary on one peer is not that release. A **pin move** changes the committed submodule
commit the fleet builds from; a **fleet roll** redeploys every running peer. Before shipping,
extend configuration validators,
prove coverage/redundancy and out-of-arc reads under loss, declare compatible rollback and
regrowth behavior, and measure the rollout. This document changes no pin or deployed policy.
Probe B adds two items that fork change must contain before any of that: publication filtered to
receivers whose arc contains the basis, and an arc gate on incoming operations keyed to the
receiver's *target* arc (a storage-arc gate would starve a joiner whose arc is still Empty), with
warrants and self-authored operations handled explicitly and operations already held outside the
arc cleaned up. Repeat Probe B's per-operation held-set test on every space after the change.

## 5. Membrane

**SPEC, needs Matthew:** replace Holons §9.2's household-named admission key with a
peer-vouched proof. Authority D5's concrete node-registry model accepts a vouch from an
already-registered node, validated using `must_get_valid_record` (a deterministic dependency
on an existing valid record). It does not authorize turning every role into an invite-only
network. Restricted group admission and any measured sybil pressure must be justified
separately. A sybil is an attacker multiplying identities; counting keys does not defeat it.

The proposed proof carries the target DNA identity, joiner's agent key, charter/policy version,
intended membership scope, voucher signatures over those exact fields, voucher registration
or membership action references, and admission epoch plus any permitted validity bound.
An epoch is a declared policy period. It also carries the proof of how that period was
recognized. It cannot select its own validating rule or nominate an ultimate admission key.
The minimum voucher rule for a restricted group is open; D5's already-registered-node example
must not silently become a threshold or a council veto invented here.

Honest peers validate the join proof in the `AgentValidationPkg`, the source-chain record
carrying admission evidence. They verify target-space and agent binding, signatures, voucher
eligibility under the pinned policy, and dependency validity; unresolved dependencies remain
pending, not accepted. Eligibility must be checked at a defined historical action/epoch,
not by a nondeterministic “latest membership” query. Revocation after admission requires a
separate current-operation/serving check; it cannot retroactively erase a valid genesis.

`genesis_self_check` can check structure, signatures and available pinned material locally.
It cannot read the DHT to establish voucher eligibility. It is a courtesy; a patched joiner
can bypass it. Peer validation uses same-space valid dependencies where available. A foreign
imagodei or mishpat ActionHash cannot simply be fetched as a local valid record: the
cross-space authority bridge must first be specified and verified. The first members also
cannot circularly vouch for each other without an independently justified founding policy.
**Unresolved:** a founder-free bootstrap and deterministic cross-space recognition chain
for the first cohort. No confidential clone is commissioned before that is resolved.

The hostile-joiner probe repeats Receipt's same-seed outsider using a locally permissive
self-check. Test missing, forged, wrong-space, wrong-agent, withdrawn-voucher and stale-epoch
proofs, plus an authorized control. Inspect peer validation and accepted records as well as
agent-info membership. Then request existing content, roster, inventory, storage projection
and doorway caches directly. Passing write rejection alone does not prove read exclusion:
whether the conductor's gossip/read paths enforce admission is an explicit missing node.
No sensitive plaintext should be entrusted to a clone until the complete probe passes;
capability checks and encryption, where implemented, remain separate confidentiality work.

Peer admission validation changes integrity code and therefore the DNA hash. It separates
old and new spaces even with the same seed. Price every affected role/cell, migration and
identity continuity, overlap custody, and historical evidence verification. Coordinator-only
proof plumbing is hash-neutral; it cannot supply the missing integrity gate. Avoid a blind
reinstall or presumed key continuity. Migration is a witnessed crossing with the successor
and partition behavior below, not a configuration toggle.

## 6. Composition

**BUILT:** `CoverageRollup::rollup` combines child coverage and preserves constituent hashes.
It sets `witness_quorum` to **0** and leaves `as_of_heads` empty
(`elohim/elohim-storage/src/recursion.rs:216–248`). **ABSENT:** a complete authenticated,
witnessed publication path across spaces (Composition MN-6; `elohim/elohim-storage/src/recursion.rs:247`).

**SPEC:** a published rollup names the child roots, applicable source heads, period, recipe
CID, consenting coverage, excluded/unreadable remainder, translation rules and witnesses.
A recipe is the pinned calculation; a Merkle root is a hash committing to a tree of chunks.
Recompute locally as a view first. Publish a root only with a charter-selected witness quorum
**greater than zero** (operator-approved plan of 2026-10-15, §4; session evidence, not tracked), eligible signers and observed independence. The exact quorum
is a ruling/budget input, not supplied by arithmetic. Mere signatures or a nonzero field are
not recognition. Consumer validation must check the policy, evidence and permitted reach.

| Cross-space act | Successor authority | Partition behavior (SPEC) |
|---|---|---|
| Child rollup to parent | Child council authorizes the disclosed outcome; receiving council accepts it under its own recognized charter, answerable to its human audit floor | Retain last accepted root with age and missing-child coverage. Pending/rejected inputs do not become zero contributions or new authority. |
| Pledge offered to another pool/space | Supporter signs the offer; recipient council accepts the exact terms. A source council cannot accept for the recipient | Keep signed offer locally pending. No debit, transfer or accepted status inferred from delivery or retries. |
| Reach expansion / republication | Author's permitted disclosure plus destination's admission and acceptance authority | Keep old scope; wider publication stays pending. Identical body CID does not make old authority valid in the new space. |
| Space upgrade or pool dissolution | Old charter's authorized closure names a successor; successor council recognizes retained duties; human floor can contest | Preserve old evidence and label successor acceptance pending. No receipt means no completed handover; no automatic takeover by the relay. |

Each crossing binds source space, destination space, source action/root and policy versions.
Use those identities to deduplicate retries; preserve corrections and revocations. No
cross-network atomic transaction is claimed. Monetary settlement remains outside this spec.

**Shard key:** use presence-key ranges for the reverse-index/rollup payload (Facility F6),
committed under the parent root. A hot presence can need chunking of its child contributions
within that root; its source accounting must remain complete and deduplicated. DHT arcs
partition custody of notarized operations by their hashes. They are a different keyspace.
Do not equate a presence range with an agent's DHT arc. Three-seams §5's co-location can be an
optional placement optimization after a measured mapping exists, not today's invariant.

**Fan-in per tier:** define charter bounds `b_household`, `b_pool`, `b_language`, and
`b_planetary` on direct children. F7 requires at most one forwarded row per child **per
presence per period** (Facility:164–169,547–550). At each tier that bounds rows for a hot
presence by its `b`; it does not bound total rows across presences. Require separate measured
budgets for total bytes, proof checking, number of shards, queue size, catch-up and witnesses.
Exceeding any budget defers publication with a visible deficit or prompts a governed split;
it cannot discard contributions silently. These bounds remain **UNSET**, so planetary
readiness is **ABSENT**. Federation frames must fit the actual **1 MiB** bound including
proofs/envelope (`elohim/elohim-storage/src/p2p/view_federation.rs:172`), not the research's
payload-only estimate.

## 7. Head-plane budget per record kind

**SPEC counting budgets, not throughput forecasts.** Let `W` be workspaces, `R` releases per
workspace in the year, `P` supporters, `H` physical holons and `T` publication periods in the
year. Use sums where cadences differ. Seed counts are `W_seed × R_seed`, `P_seed`, and
`H_seed × T_seed`; these inputs are **unmeasured**, not zero by assumption. Annual counts are
`W × R`, `P` for initial pledges plus explicit amendments/revocations, and `H × T`.
The formulas are conditional counts (Facility:169;
`.claude/skills/p2p-design-gate/SKILL.md:157–159`); they do not forecast users.

| Kind | Classification and selected bundling shape | Recurring work and unresolved cost |
|---|---|---|
| Summary per workspace per release | Notarized (A): reuse EconomicEvent action `cite` as a composite-root declaration; lockfile details and dependency edges beneath the root | Declare the workspace's applicable release, retain history without electing every old edge. `W × R` retained declarations are not automatically only `W` heads; prove the projection indexes only the intended live roots. |
| Pledge per supporter | Notarized (A): reuse Commitment; pool recipient and acceptance are explicit. Composite-root pledge corpus per pool/period with inclusion evidence, not a new independently reconciled head for every installment | `P` signed offers remain real authority records. A corpus digest can compare them, but does not erase their validation or revocation costs. Direct bounded lookup by pledge hash remains required. |
| Rollup per holon per period | Computation is Ephemeral (C); publication is Notarized (A), reusing EconomicEvent as a root candidate, subject to a validated witness contract | `H × T` declarations, with presence shards beneath one composite root. Do not notarize each shard as another moving head. Witness validation and shard availability remain paid. |

**Illustrative worked example — not a forecast:** suppose `W = 10`, `R = 10`, `P = 50`,
`H = 5`, and `T = 10`. The year retains 100 release declarations + 50 initial offers +
50 rollup declarations = **200 records before acceptance, amendments, revocations and
membership history**. With one applicable release root per workspace, one pledge corpus
root for a single pool, and one current rollup root per holon, the intended comparison set
would be 10 + 1 + 5 = **16 live roots**, only if indexing implements that shape. The 200
records still need validation and historical lookup; neither number predicts quiesce time.

These choices justify bundling; they do not prove that current reconcilers implement it.
The gate's **approximately 500-item** review threshold is from
`.claude/skills/p2p-design-gate/SKILL.md:159`, not a safe capacity ceiling. Unknown annual
inputs trigger bundling and load tests, not permission to proceed without a budget. If the
implementation retains per-item heads above that threshold, Matthew must explicitly sign
off on measured quiesce cost or require a different shape before shipping.

New independent heads join sweeps, availability advertisement, election and adjudication.
The historical **3,469 heads / about 2.5 hours** observation is an order-of-magnitude warning
(`.claude/skills/p2p-design-gate/SKILL.md:45`). Current batching has already
changed conductor call costs (`elohim/elohim-storage/src/p2p/projection_reconcile.rs:1148–1178`).
Do not divide that historical wall-clock by heads and multiply into this design. Measure
retained history, live heads and release/restart recovery separately. No new entry type is
proposed; reuse does not make new validation semantics free.

## 8. Where the README pledge lives

**SPEC chosen baseline:** a public-purpose pool's identity and roster live in the existing
imagodei network. The supporter's signed pledge is authored on their own source chain in
the existing mishpat cell, using Commitment, and witnessed there. It names the exact
Collective action address and terms; the receiving council's acceptance must be separately
verifiable. The DHT holds the declaration; local storage is its query projection, not the
council's independent ledger. The supporter's draft stays on their own device.

**BUILT substrate only:** `mishpat::create_commitment` returns both entry and action hashes
(`elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs:25–64`). **ABSENT:** the monetary
pledge application, collective recipient enforcement, council acceptance and cross-role
recognition needed here (Pool:127–163; Facility:433; `elohim/elohim-storage/src/mishpat_projection.rs:816`). A content clone does not make
that pledge local to the pool's physical holon. If Matthew selects a pool governance clone,
the author must join its approved mishpat space and author there, or use a witnessed migration;
the physical destination cannot be changed by relabeling a shared-role record.

The honest minimum button is **“Prepare a pledge”**. With a persistent personal workspace,
it retains signed or unsigned terms locally and says **“Saved on this device; not sent or
accepted. No money moved.”** A visitor without a key can prepare terms, but sees **“Not kept
after this visit; export to keep a copy.”** Preparing does not require a key; signing does.
Export must preserve the terms and address. Submission becomes available only when a real
recipient and acceptance path exist. If persistence fails, do not display the saved message.
This is a proposed minimum, not a claim that today's README implements it.

It may explain the intended give-and-join destination, show the public proposed pool and its
charter, and distinguish a prepared offer from a witnessed and accepted commitment. It may
not claim payment, membership, council custody, a private roster, independent recovery or a
physically isolated pool merely because a click occurred. MOU's ruling is “The support button
is give and join”; this draft does not weaken that intended destination into an already-built
pledge-only product. A money-taking button requires the separate acceptance and settlement
work and the operator/counsel decisions recorded in MOU §G.

## 9. Leaving, dissolution and metadata reach

**BUILT record primitive:** Membership withdrawal is recorded with
`withdrawn_at_block_height`; current-member evaluation follows updates
(`elohim/holochain/dna/imagodei/zomes/imagodei/src/qahal_coordinator.rs:573`; Pool:96–115).
**SPEC integrated behavior:** leaving stops new draws under current membership and revokes
or expires the member's commitments under their terms. Preserve the person's records,
received evidence and ability to contest. Disabling a cell does not erase copies others
already hold. Epoch/key rotation can constrain future access only after its enforcement is
built and tested; expulsion cannot promise deletion of prior plaintext.

Dissolution requires a charter-authorized closing record, named successor or explicit
unassigned remainder, consented re-parenting of constituents and successor acceptance
(Facility F10; Composition MN-6). A tombstone is the retained record that the pool ended.
It preserves descent, contestable obligations and the human appeal path. During partition
closure remains pending as in §6; no doorway or founder inherits custody by default.

**BUILT exposure:** the present public Membership entries and list functions reveal rosters;
compute requests also reveal requesters (Pool:207–233). Public pools must disclose this.
Seed separation has not proved confidential discovery: Receipt used a shared bootstrap,
and transport observers can learn endpoints, timing and participation. Inside a future
restricted space peers still see one another; a hosted person's doorway sees their traffic
and can sign as them.

**SPEC boundary:** publish a space's existence or roster outside it only by a separately
scoped disclosure act. Keep discovery, inventory, logs, projections and caches keyed and
authorized by space. Public summaries must not expose a private reference to otherwise public
bytes. The recorded shared-projection escape
(`genesis/data/timeline/backlog/clone-content-escapes-via-shared-projection.md`) remains
a release blocker for sensitive clones. A malicious authorized member can copy what they read; the membrane is
not a prevention claim against that member. Private households stay local until metadata
and byte paths pass the outsider probe. Do not publish their roster into imagodei as a
“temporary” implementation of confidentiality.

## 10. Gate outputs inline

These readings constrain the proposals above. They are not runtime acceptance. A **declared,
unwired** question has an intended function but no runnable probe for this specific pool;
no live charter or completed council is implied.

### Elohim commons design report

The eleven questions, in the skill's order, ask of this pool:

- **Q1:** Do appropriation and provision rules fit its local people, resource and purpose?
- **Q2:** Is what a member may draw proportional to provision over a declared window?
- **Q3:** Can members see the resource's condition without a steward's help?
- **Q4:** Is there an agreed graduated ladder, notice first, with return always open?
- **Q5:** Can members inspect who drew what, at the declared reach?
- **Q6:** Who belongs, how do they join, and how do they leave?
- **Q7:** What resources does the pool hold, and what lies outside its boundaries?
- **Q8:** How can members propose, decide and close a rule change?
- **Q9:** Is there a fast, cheap local conflict arena and an upward appeal?
- **Q10:** Which parent recognizes the pool's rules at its reach, and what does it reserve?
- **Q11:** Is the deciding body small enough to communicate, or does it nest?

```text
Pool: proposed public support commons  reach: commons (semantic; wire vocabulary unresolved)  parts: facility=members' storage/compute/bandwidth/labor artifacts=release summaries, pledge evidence, rollup roots
Present (declaration + probe): none            0/11
Declared, unwired (no probe):   Q3 Q5 Q6 Q7 Q11
Absent:                         Q1 Q2 Q4 Q8 Q9 Q10
Failure-predicting absences:    Q1 Q2 Q4
Labor misplaced on the elohim:  none proposed; first notice only for a member's own agent, no agent vote or final verdict
Plurality: lineages=UNDECLARED  closure-across=UNDECLARED
Rights on artifacts held by members: access contribution extraction removal management exclusion
Alienation: held by no one
Gaps, in mintable shape: commons charter / between public pool identity→recognized commons / missing node: locally agreed eleven declarations + member-run probes / current state: proposed, no instantiated charter
```

The count is a gate result under the skill's eleven-question form, not a score of the
protocol. Q3 proposes member-readable fill/freshness/independence evidence; Q5 permits members
to inspect draws at declared reach; Q6 names join/leave behavior; Q7 bounds holdings; Q11
requires a measured child and membership ceiling. Each still lacks this pool's runnable
probe. Q1 needs its local lens, Q2 the contribution/draw rule and window, Q4 the agreed notice-first
ladder, Q8 the amendment closure path, Q9 a cheap local challenge and appeal path, Q10 an
actual recognizing parent's attestation. Q3's unwired resource monitor is also a
failure-predicting gap, despite being declared. Rights are proposed charter rights, not
permission to remove evidence held under another participant's retention obligation.
The member's elohim senses, reads, drafts and carries; people admit, vote, impose consequences
past notice and settle. The plural council's commons custody remains subject to the human
floor. Representation and independent lineages remain open (MOU §D4).

### Local-first reading: commons reach spaces and pledge

- **Alone, key on the device:** read held published bytes with address verification; prepare,
  persist, address and sign the person's offer locally. Publication, acceptance and fresh
  membership evidence wait for peers. Signing alone does not mean witnessed or accepted.
- **Alone, key held by a doorway:** read cached addressed bytes and retain an unsigned working
  version locally; account signing waits. Show “not saved to your account.” The doorway can
  read and sign for the person. This is hosted convenience, not a claim about incapacity.
- **Alone, no key:** read cached public material and prepare transient terms; disclose that
  nothing persists past the visit unless the person explicitly exports it.
- **Working version:** author — the supporter alone edits their draft; council proposals use
  separately permissioned working documents. Publication fixes and signs an addressed version;
  a subsequent change is another submission with explicit supersession, not a merged verdict.
- **Each kind of state:** selection is shown here; presence/cursors are passing and unstored;
  terms are a private working document; signed offers and accepted roots are published state.
  Raw attention, private notes and counsel context stay off the governed shared plane.
- **Arrivals:** another owned device adds a consented backup/signing location and learns the
  shared draft; losing it leaves the original. Another person adds acceptance or witnessing
  and learns only the offered material; losing them leaves a pending offer and old evidence.
  An always-on holder adds availability and learns timing/sizes (plaintext unless protected);
  losing it leaves locally held records. A doorway adds hosting and learns account activity;
  losing it preserves local drafts/readable copies, while hosted signing waits for recovery.
  A pool adds capacity/council acceptance and learns the declared offer/membership; losing it
  leaves local copies and unresolved obligations. None replaces the only personal copy.
- **Shown to the person:** on-device only; each copy caught up/behind/unknown; offer pending,
  accepted or rejected; publication pending/current/not current; stale evidence and the
  particular authority whose response is awaited.
- **Refusals:** a mandatory server-first pledge and a private household on public Membership
  records are refused. Local preparation precedes delivery; confidential shared publication
  waits for the admission, serving and metadata contracts.
- **Stands as is:** content-addressed local reading, private draft ownership and existing signed
  commitment primitives have appropriate homes. Acceptance belongs to the receiving council;
  the missing space/authority adapter must run on the person's own node as well as a host.

**Back-fill check:** delete every doorway: the person's node still prepares, stores and signs
an offer and later submits through its own conductor. Close the laptop mid-change: the last
persisted draft returns without a network. The first unavoidable wait is another party's
membership recognition/witness or acceptance; the person's entire offer is prepared alone.
These are **SPEC acceptance criteria**, not an assertion that the browser currently does this.

### P2P Design Gate: commons reach spaces

The **SDO/RWA test** asks what a dominant few, supported by obedient or synthetic followers,
could observe, join or compel (social dominance orientation / right-wing authoritarianism
in the skill’s psychological sense). It tests the worst holder, not the best operator.

The following source inventory was read before proposing entry reuse:
`imagodei_integrity`'s entry enum contains Collective, Membership and CollabAgreement
(`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/lib.rs:945–949`);
`mishpat_integrity` contains Commitment (`elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs:309–320`);
`content_store_integrity` contains EconomicEvent and validates it
(`elohim/holochain/dna/elohim/zomes/content_store_integrity/src/lib.rs:3726,4317–4320,4398`).
Their actual zome names were checked in each directory's `dna.yaml`. No capacity tally
is used as a reason to add or reuse a type.

### Entity: Collective and Membership

- **Classification**: Notarized (A), existing types; not a newly invented pool entity.
- **Justification**: membership and charter identity cannot silently change; the current public
  records are unsuitable for confidential membership.
- **Head-Plane Cost Budget**: seed `C_seed + M_seed`, annual `C_year + M_year` plus explicit
  changes, all unmeasured. `C` counts collectives and `M` memberships. Corpus digest comparison
  per pool is proposed; authority records remain individually checkable. Actual live-head
  indexing and large-roster quiesce need measurement before deployment (§7).
- **Network Stakes**: Simulacra (declared simulation), Bootstrap, Coordinated and Enforced
  (live enforcement). Cached matching-data reads can cheapen; consent, membership authority,
  constitutional evidence and corrections remain floor-protected at every stage.
- **Content Address Strategy**: content-derived records with existing ActionHash identity for
  the Collective; membership is scoped to its participant and collective.
- **Address Justification**: `imagodei::create_collective` actually returns ActionHash and
  uses `collective:<ActionHash>` in references, not an EntryHash or raw-content CID
  (`qahal_coordinator.rs:54–75,536`). Preserve that existing contract explicitly.
- **Transport Affinity**: not applicable to DHT records; related addressed artifact bytes use
  `auto` (available transport selection), subject to the same reach checks on either transport.
- **Source of Truth**: Holochain DHT; local draft values have no membership authority.
- **Integrity Zome + DNA-hash class**: `imagodei_integrity`; reuse is DNA-hash-NEUTRAL;
  adding admission validation or changing record validation is DNA-HASH-MOVING.
- **Coordinator Zome**: `imagodei::create_collective -> ActionHash`; creates the initial
  Membership too. **BUILT:** `list_memberships_for_collective` fetches linked original records
  without resolving withdrawals (`qahal_coordinator.rs:128–145`). The separate internal
  `membership_is_current` helper checks withdrawal for authority gates; the stale-list warning
  is at :579–588. Its walk covers only direct, same-author updates of the Create, not
  updates-of-updates (:624–628). This is not a generally current roster API. Restricted
  join/admission has no completed coordinator contract here.
- **Projections**: SQLite collective/membership caches, exact pool projection wiring unresolved;
  `dht_anchor_hash` required on witnessed records. Automerge (the merging document store): no
  roster broadcast proposed; reach tier `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none added. Any later storage manifest route must accept the actual
  ActionHash-based collective identity, not silently reinterpret it as EntryHash.
- **Anti-Pattern Check**: no clone per aggregate; no founder privilege promoted from the current
  founder-membership bootstrap into a membrane; no public roster called confidential.
- **SDO/RWA Test**: a dominant holder backed by compliant agents can enumerate today's public
  roster. Restricted membership is refused on that store; no private participation placed there.

### Entity: release summary and published rollup root

- **Classification**: Notarized (A) publication; Ephemeral (C) recomputable calculation below it.
- **Justification**: the arithmetic is reconstructable; the declaration that a council accepts
  a particular result is an accountable act. A root alone does not prove that acceptance.
- **Head-Plane Cost Budget**: seed and annual summary/rollup formulas and composite-root
  justification are §7; independent shard heads are excluded by this proposed design.
- **Network Stakes**: all declared stages; batch/cached recomputation can be stage-priced;
  witness eligibility, constitutional scope, consent and counter-evidence stay protected.
- **Content Address Strategy**: Content-Derived (CID) for root bytes, EntryHash for the reused
  EconomicEvent, ActionHash for its notarizing action.
- **Address Justification**: the declaring workspace pins its release; the authorized council
  pins a rollup's period and policy. Timestamp recency does not elect either.
- **Transport Affinity**: `auto` for root/shard bytes; availability is measured independently
  from DHT notarization. No byte transport itself grants readership.
- **Source of Truth**: Holochain DHT for the declaration; addressed constituent bytes for
  recomputation; SQLite caches are reconstructable.
- **Integrity Zome + DNA-hash class**: `content_store_integrity`; existing EconomicEvent reuse
  is DNA-hash-NEUTRAL. A new peer-enforced witness validation rule would be DNA-HASH-MOVING;
  general EconomicEvent validation does not already prove the rollup quorum.
- **Coordinator Zome**: `content_store::create_rea_economic_event -> ReaEconomicEventOutput`
  containing EntryHash and ActionHash (`content_store/src/lib.rs:17356,17493`). Summary
  graduation still needs the version/window and flow-store adapter (MOU F2); the rollup
  witness verifier and publisher are ABSENT. No hypothetical function is presented as built.
- **Projections**: existing `ReaEconomicEventCommitted` signal (`content_store/src/lib.rs:15904`)
  is the reuse seam; exact summary/root projection and hash-safe lookup remain unwired.
  SQLite requires an anchor once witnessed; no Automerge broadcast of private constituents.
  Reach tier `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none added; future root lookup accepts a content CID, event lookup an
  EntryHash, with the action anchor separate. Registry implementation waits for the verifier.
- **Anti-Pattern Check**: no standalone CoverageRollupAttestation type; no per-dependency head;
  zero-quorum computation is not labeled witnessed; transport limits include proof overhead.
- **SDO/RWA Test**: a hostile aggregate holder learns only authorized outcomes and disclosed
  descent, not household activity or private dependency references. If source disclosure
  would expose those, publication is refused or narrowed before witnesses sign.

### Entity: pledge offer and council acceptance

- **Classification**: Notarized (A), existing Commitment for the offer; acceptance is a
  separately signed authority act whose exact reuse binding remains a gate, not a new type.
- **Justification**: a supporter's offered obligation and the recipient's acceptance must be
  distinguishable. A private draft is not either act.
- **Head-Plane Cost Budget**: §7's `P` plus amendment/revocation records; composite pledge
  corpus with inclusion evidence. Acceptance records add work and must be counted in the
  measured corpus; they are not free attributes inferred from a root.
- **Network Stakes**: all declared stages; simulation can cheapen operational checks, not
  participant consent, recipient authority, revocation or constitutional review.
- **Content Address Strategy**: Content-Derived EntryHash for the fixed Commitment; the
  supporter/recipient relationship scopes duplicate detection, not a substitute identity.
- **Address Justification**: `CommitmentOutput.entry_hash` identifies terms;
  `action_hash` anchors authoring. Recipient uses the actual Collective identity above.
- **Transport Affinity**: `auto` for separately stored terms/evidence; DHT records use conductor
  transport. Audience checks are required before sending either.
- **Source of Truth**: supporter's source chain and Holochain DHT for published offer;
  recipient's witnessed acceptance for acceptance, not a doorway database flag.
- **Integrity Zome + DNA-hash class**: `mishpat_integrity`; reusing Commitment is
  DNA-hash-NEUTRAL. Restricted-space admission is separately DNA-HASH-MOVING.
- **Coordinator Zome**: `mishpat::create_commitment -> CommitmentOutput { entry_hash,
  action_hash }` (`commitments.rs:25–64`). Public-pool recipient validation and council
  acceptance remain ABSENT; current capacity verbs must not be relabeled monetary pledges.
- **Projections**: `mishpat_commitments`, `CommitmentCommitted` on the authoring path
  (`commitments.rs:90–98`); witnessed anchor required. Cross-peer acceptance/revocation
  projection is an explicit gap. Automerge: no published pledge broadcast proposed;
  reach tier `unresolved — reach vocabulary in declared drift`.
- **HTTP Route**: none added. Future manifest routes must preserve the returned EntryHash for
  offer lookup and keep ActionHash as proof; no independent doorway pledge ledger.
- **Anti-Pattern Check**: no money moved by an offer; no acceptance inferred from membership;
  no economic attribution from unsigned transport/agent identity bindings.
- **SDO/RWA Test**: public offers reveal supporter relationships. Private giver preferences
  and counsel notes are excluded; a person must explicitly consent before publication.

### Entity: join evidence, drafts and calculated views

- **Classification**: a vouch is Linked (A2) in meaning to existing membership/registration,
  carried in the built-in admission package, not a proposed standalone entry; private signed
  personal notes are Private (B); working drafts remain non-authoritative documents;
  recomputable rollup/index views are Ephemeral (C). No new Attested-Private (B2) raw-data
  collection is proposed.
- **Justification**: admission asserts an existing relationship under target rules; private
  preparation and calculated views are not published authority.
- **Head-Plane Cost Budget**: join proofs scale with admitted cells and proof history; seed and
  annual admissions are unmeasured. No extra application head proposed for the proof itself;
  referenced Membership/registration records still count. Views/drafts add no published heads.
- **Network Stakes**: admission and private reach checks remain protected in all stages;
  view recomputation can be cheaper under declared lower stakes.
- **Content Address Strategy**: Agent-Scoped Composite for joiner/target/policy vouch, signed
  over fixed evidence; content-derived addresses for fixed drafts and rebuildable roots.
- **Address Justification**: proof must bind the joining agent and exact target; it cannot
  choose its own authoritative rule by a free-form identifier.
- **Transport Affinity**: admission proof travels with installation; `auto` only for permitted
  evidence bytes. Personal drafts stay local unless sharing is explicitly authorized.
- **Source of Truth**: valid admission package plus referenced DHT authority for the vouch;
  private source chain for signed private records; document store for working values;
  SQLite operational cache for reconstructable views.
- **Integrity Zome + DNA-hash class**: target role's existing integrity zome, e.g.
  `mishpat_integrity` or `content_store_integrity`; admission validation is DNA-HASH-MOVING.
- **Coordinator Zome**: no new create-link extern asserted; this A2 classification does not
  pretend a Holochain Link is already written. Admission proof delivery belongs to cell
  installation, with peer validation in integrity. Bootstrap evidence binding is unresolved.
- **Projections**: local-only drafts; no shared proof/roster cache outside the space. Views
  rebuilt from authorized roots; no authority anchor fabricated for a calculated view.
- **HTTP Route**: none. No shared draft or private-note endpoint is proposed.
- **Anti-Pattern Check**: no new entry type, no founding admission key, no current remote lookup
  disguised as deterministic self-check, no authored view mistaken for authority.
- **SDO/RWA Test**: vouchers and insiders can correlate membership; disclosure outside the
  space is refused until scoped discovery and serving prove the participation boundary.

### Design Constraints Discovered

**Gate outcome: incomplete for implementation, explicit enough for operator review.** No
schema, migration or route is authorized by this draft. The unresolved acceptance contract,
founder-free bootstrap, witness verifier, projection isolation and measured head indexing
block the corresponding implementation slices. The existing collective ActionHash contract
is a documented exception to the template's generic EntryHash expectation, not “fixed” by
silently changing it. Agent identity uses the canonical agent key; transport IDs require
verified binding before economic attribution.

**P2P back-fill check:** coordinator return values above were read from code before specifying
any route; no route is designed here. Integrity names come from `dna.yaml`, and admission
changes move the DNA hash while coordinator-only reuse does not. Annual counts are §7's
symbolic budgets with unmeasured inputs; they increase retained validation work even when
bundling reduces live-head work. Quiesce therefore remains a required measurement, not a
computed promise. These answers reveal missing contracts rather than claim a completed gate.

**Concern-canon declaration:** for future admission and rollup publication decisions, the
C0–C14 registry answers below are all `partial`: this draft names contracts, but supplies
no new contract tests. Vocabulary follows `.claude/epr-meta/concerns.yaml` and `policies.yaml`.

| Code | Meaning for this design | Status |
|---|---|---|
| C0 | Separate custody, reach and heads | partial |
| C1 | Require independent eligibility, not self-election | partial |
| C2 | Preserve source-pinned authority through supersession | partial |
| C3 | Keep preparation live while acceptance waits | partial |
| C4 | Label absent/stale evidence | partial |
| C5 | Treat signatures as evidence, not automatic authority | partial |
| C6a | Bound validation, child fan-in and retries | partial |
| C6b | Deduplicate by source/destination/action | partial |
| C7 | Test advertised admission and actual reads | partial |
| C8 | Retain reasoned decisions | partial |
| C9 | Name lineage migration | partial |
| C10 | Make policy/version compatibility explicit | partial |
| C11 | Defer work at receiver limits | partial |
| C12 | Check disclosure and recipient consent | partial |
| C13 | Separate human floors from delegated action | partial |
| C14 | Witness deficits and unmapped remainder | partial |

The code owner must register each actual decision point and its contract tests at birth; this doc-only
pass creates no speculative crate registry rows.

### Gradient-reading per-plane declaration

| Plane | Reach of bytes and references | Custody, threshold and independence | Freshness and stakes | Linkability; cost bearer |
|---|---|---|---|---|
| Private working values | Author-only by default, including references to public works | Own device; optional consented owned-device backup; no pool recovery claim | Local working version usable offline, no published authority | Holder learns the draft; author pays; no network export by inference |
| Artifact bytes | Declared audience; referencing a public artifact can remain intimate | Least holder set meeting charter loss model; recovery threshold UNSET; measure power/network/admin diversity | Cached fixed bytes valid by address; absence visible separately from stale head | Holder sees size/timing and plaintext unless protected; contributing holders pay carrying cost |
| Declared heads and admission | Public baseline only; restricted metadata waits for §5/§9 | Full-arc selected holders while small; authorization quorum and data recovery threshold are separate and UNSET | Exact pinned policy and witnessed head required for acceptance; old evidence usable only as labeled history | Membership and activity visible to holders; validators and hosted operators pay checking/retention |
| Crossing references and rollup roots | Child-authorized disclosed outcomes only; no private source-reference leakage | Child evidence retained locally, root carried to named parent; witness quorum positive but value UNSET; independent witnesses observed | Period and source heads explicit; missing child stays deficit; no stale root authorizes a new transfer | Root can reveal contributors and relationships; child/parent holders pay proof and forwarding costs |
| Derived index/view | At most the audience of authorized inputs, references independently scoped | Rebuildable holder subset by presence range; no authority assigned to cache owner | Label fold lag and unmeasured coverage; recompute on applicable head change | Queries expose interests to contacted custodians; requester keeps its receipt, providers bear declared compute |

For the member→holder path, the intended relationship is `trustful-declared`; proof checking
is `friction-verify`, serving is `friction-mechanical`. For an unknown joiner it is `trustless`;
verification is owed. Clone runtime cost is mechanical per cell and per cadence, including
transition regrowth; it cannot be removed by trusting the peer. Summary publication is per
release, rollup publication per period and child, not per click. Cached verified roots can
compress repeated resolution; consent and revocation checks remain. Background gossip and
catch-up must not occupy the person's draft-editing path.

Source evidence qualifies the skill's blanket trust-blind wording: fanout already responds
to reach (`elohim/elohim-storage/src/p2p/fanout.rs:64`), but grounded relationship-derived
negotiation is missing (`elohim/elohim-storage/src/p2p/mod.rs:7033`;
`elohim/elohim-storage/src/trust_verification.rs:176`). This proposal depends
on that missing signal and does not claim an earned-trust performance path is built.
Additional spaces, confidential discovery, witnessed roots and independent custody add cost;
§2 measures only part of it. Separation must be measured, not described as free.

## 11. Contradictions and proposed resolutions

| Contradiction | Proposed resolution | Operator decision required? |
|---|---|---|
| Holons §9.2 household-named key versus Authority D5 | D5 rules out a founder gate. Use eligible-peer vouches; first-cohort authority, epoch rules and read enforcement remain explicit design gates (§5). | Yes for the concrete membrane; no reopening of the founder-key prohibition. |
| Three-seams §5 arc index versus Facility F6 presence ranges | Presence keys partition rollup payloads; DHT hash arcs partition operation custody. Optional co-location needs a measured mapping (§6). | Endorse this reconciliation before revising the older source; sharding funding is a separate ruling. |
| Pool in imagodei versus its own space | Keep public identity/membership in imagodei now; dedicated operational/governance spaces are a separately selected placement, not a different kind of pool (§3/§8). Restricted metadata cannot use the public baseline. | Yes, per tier and role, after the D6 test. |
| Tier as view, collective, or holon of stewards | Aggregate tier is a view (D1); its governing membership can be a Collective; its stewards may occupy a separately admitted physical space. None implies the others. | No new ruling needed for D1; yes for any steward-space commissioning. |
| Fractional arc assumptions versus full/zero arcs | Price full custody today. Probe B (§4) shows a scratch binary can hold an exact half arc, and its integrated operations on a gossip-delivered space are consistent with a correct shard, while publication leaks out-of-arc operations into the same peer; fractional benefits stay unclaimed until a fork change closes the leak and the pin/fleet release path is taken. Do not inherit old capacity assertions. | Yes for funding and shipping; no to claiming an absent capability as built. |

## 12. Rulings for the operator

- Which peer-vouched membrane shape is acceptable for restricted groups: eligible voucher rule,
  founder-free first cohort, recognized policy epochs, and behavior when voucher evidence is
  unavailable? Does any public role have measured sybil pressure warranting admission changes?
- Probe B measured a partial arc that holds under gossip and leaks under publication (§4).
  Should a production sharding fork change be funded — the probe's arc setting plus a
  publication filter and an incoming-operation arc gate — and under what release evidence may
  its pin move and fleet roll occur? Does per-space arc selection get separate funding?
- At household, pool, language and planetary steward tiers, which roles stay in shared
  imagodei/mishpat/content spaces, and which may become dedicated spaces after D6 passes?
- What phone/hub resource and recovery budgets, independent-holder requirements, child fan-in
  bounds, publication periods and witness quorums may those spaces consume?
- What determines council representation without invasive demographic correlation? What ends
  infancy, and what happens where the physical holon cannot seat separate acting/auditing bodies?
  §12.1 offers candidates, not a selection.
- Is the proposed presence-key/arc distinction the accepted correction to Three-seams §5?
- May the README initially expose local pledge preparation with the limitations in §8?
  Who is the recognized recipient, and what acceptance and settlement decisions must precede
  give-and-join? This second half is deferred to counsel under **MOU §G**: jurisdiction-specific
  securities, charitable-giving, money-transmission and KYC treatment of pledge, custody without
  a beneficiary and pass-through; whether a maker's declared funding channel can be paid
  without further consent; and copyright estates and forced-heirship portions for children.
  The operator's product ruling does not settle those counsel questions.
- If measured head indexing cannot implement the selected bundling, is any larger retained
  per-item head budget acceptable, with which measured quiesce limit?

### 12.1 Council seating — CANDIDATE models for ruling 5

**BUILT primitives, not a seating contract:** `Collective` carries a charter; `Membership`
connects a Person, Collective or ElohimAgent to it with a Steward, Contributor or Observer
role (`elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs:8–47`). There is no
CouncilSeat role, council key or minimum council headcount in those types. Pool:82–84 and
MOU §D3–D4 prescribe plural representation and human audit, but leave seating mechanics open.
A Steward membership alone does not prove a representative mandate or independent audit.

**SPEC meaning for these candidates:** to “seat” a council is to recognize named participants
under a charter, with evidence of selection, represented constituency or lineage, mandate,
term, current eligibility, decision threshold, recusals and a reachable human challenge body.
Membership is a candidate carrier for participation; the binding from those records to a
council mandate and signed decisions is **ABSENT** here. Keys identify signers, not seats;
plurality requires distinct participants, not several keys controlled by one actor. Minimum
headcount, quorum and the exact record binding remain operator choices, with retained
membership/mandate changes counted in §10's head budget, not free metadata.

Every candidate below preserves a **plural, representative elohim council governing each
pool**, with humans performing that role during infancy. A **human sortition council audits
and governs every contestable result**, with power to overturn it; no elohim has final
say. Human votes, admission and the other reserved human acts in §1 remain human acts.
Counts below describe pool membership, not an approved number of seats. Lack of enough
independent auditors defers contested closure to an agreed human floor; it never waives it.

| CANDIDATE model | Pool of 5 | Pool of 50 | Pool of 500 |
|---|---|---|---|
| Rotating representative seats, with shared human audit at the parent reach | A plural acting council represents the small pool; draw independent human auditors from an agreed parent pool when local recusals exhaust the roster. Low local burden, but recognition and access to that parent become prerequisites. | Rotate representation across voluntary constituencies; use local human sortition when independent, parent audit otherwise. Rotation spreads work but needs continuity and conflict records. | Nest constituencies and rotate their representatives; maintain local and parent human sortition paths. Reduces council size but risks distant representation and more mandate bookkeeping. |
| Distinct local acting and human auditing bodies, with reciprocal audit between peer pools | Local separation may be impossible; an agreed peer pool supplies independent human sortition auditors. Preserves a plural local acting council but risks reciprocal favoritism, requiring recusals and upward appeal. | Reserve disjoint acting and audit cohorts, with auditors selected by lot; peer support covers conflicts. More local accountability, more human attention required. | Nested representative acting councils and human sortition panels drawn to each decision's reach; peer support covers captured or conflicted panels. Better distribution of work, greater selection and appeal complexity. |

**CANDIDATE infancy exit tests:** the rotating model could require demonstrated independent
lineages, verified representative mandates and successful human overturn/recovery exercises
before humans hand acting seats to elohim. The separate-body model could require those tests
per constituent pool plus sustained capacity to draw independent human auditors. Neither pool
size nor elapsed time ends infancy by itself in these candidates. Test duration, thresholds,
representation without demographic inference and who recognizes exit remain ruling 5;
neither candidate supplies its own authorization. Until a ruling, humans continue the acting
role and the authoritative human floor remains in place.

## 13. Missing nodes in mintable shape

These belong to existing commons/scale/reach chains; no new register is created. The first
pair preserves Receipt's discoveries. Quoted measurements below come from Receipt §Results.

- `chain / commons-space membership / between agent learns seed→agent holds space data / missing node B: peer-validated admission and serving refusal; probe same-seed hostile install, agentInfo, accepted ops and content poll / current state: ABSENT — outsider admitted to base and clone; entry first observed on the outsider about 77 s after its install (poll sample six; Receipt)`
- `chain / per-clone cost measurement / between conductor restart→steady-state cost window / missing node B: arcs regrown before sampling; probe every expected agent's non-null/full storage_arc in agents-N.json / current state: ABSENT for production cadence — arcs null after 2 min; 39/50 full after 9 min on the development control (Receipt)`
- `chain / reach-enforced-everywhere / between isolated clone→confidential household / missing node B: space-scoped discovery, inventory, projections, caches and serving; probe outsider cannot enumerate roster or obtain bytes through any path / current state: ABSENT; shared-projection escape known`
- `chain / commons admission / between existing membership→first valid clone cohort / missing node B: founder-free bootstrap and deterministic cross-space recognition; probe forged roots and circular vouches fail while authorized founding members converge / current state: SPEC, operator ruling required`
- `chain / D6 commissioning / between clone cost receipt→phone/hub space selection / missing node B: measured full application budgets and independent recovery; probe intended workload through loss, regrowth, upgrades and restore / current state: ABSENT; conductor-only development receipt`
- `chain / commons pledge / between signed offer→council-held obligation / missing node B: collective binding and independently verifiable acceptance; probe receiver resolves exact terms and refuses unauthorized recipient or replay / current state: SPEC; commitment writer exists`
- `chain / commons composition MN-6 / between child result→parent authority / missing node B: witnessed cross-space root with successor, translation and residual; probe partition, replay and independent recomputation / current state: ABSENT; arithmetic quorum zero`
- `chain / commons scale F6/F7 / between rollup roots→bounded planetary view / missing node B: presence-range chunks and per-child forwarding under total work bounds; probe hot presence, oversized proof, catch-up and child loss / current state: SPEC; no capacity envelope`
- `chain / head-plane budget / between signed corpus→bounded reconciliation / missing node B: live-root indexing with bounded historical lookup; probe seed and annual retained-history workloads through restart / current state: SPEC; no marginal quiesce forecast`
- `chain / custody sharding / between partial target arc→reduced custody cost / missing node B: publication and ingest honour the receiver's arc; probe per-operation held set (in-arc complete, out-of-arc absent) on every space of a fresh partial-arc joiner / current state: ABSENT at fork 96bcd2bee — infrastructure's integrated operations include 6 out-of-arc against Matthew's 64,727 (27,993 in limbo unclassified), node_registry's 98 % of them (Receipt B); no pin authorization`
- `chain / household receipt / between mesh is running→a late joiner enters the same spaces / missing node B: the installed bundle's DNA hashes are recorded with the mesh; probe compare listApps cell hashes with the on-disk bundle at mesh status / current state: ABSENT — the on-disk bundle's lamad hash differs from the running household's, and a joiner installing it silently lands in a different lamad network (not a private one: any other joiner with that bundle meets it there) (Receipt B)`
- `chain / commons governance / between declared pool→recognized local rule / missing node B: charter clauses, independent council and human challenge closure; probe each commons gate question from a member device / current state: unwired or absent as §10 reports`
- `chain / pool lifecycle F10 / between dissolved pool→surviving obligations / missing node B: consented re-parenting and successor acceptance under a tombstone; probe unreachable successor and recovery without source deletion / current state: SPEC`

The incomplete gates are part of this draft's result. Production steady state, phone budgets,
membrane bootstrap/read behavior, council parameters, acceptance wiring and bundling behavior
remain unverified; Probe B is measured (§4) but sharding is not shippable at this fork. The
tracked sprint report (Receipt) carries the probe A and probe B results; raw evidence stays in
`genesis/a2o/reports/recovery/commons-spaces-2026-10-16/` and
`genesis/a2o/reports/recovery/commons-spaces-probe-b-2026-10-16/` on the measuring device. No source document is silently amended by these proposed reconciliations.
