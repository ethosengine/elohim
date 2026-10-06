---
title: A commons pool is a collective that is party to its members' commitments — the missing node under shared data, shared inference, edge serving and edge compute
id: commons-pool-as-collective-party-design
status: Draft
class: architecture
serves: recall-reaches-authority
date: 2026-10-04
context-tier: disclosed
steward: human:matthew
graduation-trigger: one pool exists on the household mesh as a Collective with a pool resource, two members have contributed storage or compute to it through commitments that name the collective, a third member has drawn from it, and removing every pool holder leaves each member's own things readable
cites:
  - genesis/research/commons-data-pools-hot-path-and-external-tooling-2026-09-11.md
  - "memory-search-scale-three-seams-design | Memory, search and scale | sha256:c119695543a2854f | path: genesis/docs/superpowers/specs/2026-09-12-memory-search-scale-three-seams-design.md"
  - "elohim-seam-map-concern-routing | The Elohim Seam Map | sha256:7ea7563016174974 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "hardware-providence-commons | Hardware Providence | sha256:17e52609abf5f92a | path: genesis/docs/content/elohim-protocol/hardware-providence-commons.md"
  - "elohim-protocol-manifesto | manifesto | sha256:66c8e3b40b858b92 | path: genesis/docs/content/elohim-protocol/manifesto.md"
  - "values-forward | values-forward | sha256:f4e7524522d3b811 | path: genesis/docs/content/elohim-protocol/values-forward.md"
  - "private-thought-governed-fruit | private-thought-governed-fruit | sha256:5b6f5cdb858277e4 | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - "swarm-curve-and-blind-custody-design | The swarm curve and blind custody | sha256:ef23b30ec9b8145c | path: genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md"
  - "compute-envelope-tevah | Tevah | sha256:006fd66d23e8f6c2 | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "eprfs-witnessed-interaction-primitive | The eprfs Witnessed-Interaction Primitive | sha256:6a24773ffd7b83f4 | path: genesis/docs/superpowers/specs/2026-07-15-eprfs-witnessed-interaction-primitive-design.md"
  - "working-version-sdk-standard-design | The working-version standard | sha256:b323e209c972831d | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - elohim/holochain/dna/imagodei/zomes/imagodei_integrity/src/qahal.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
  - elohim/elohim-storage/src/api/compute_grants.rs
  - elohim/elohim-storage/src/services/constitutional_ratio_registry.rs
  - elohim/elohim-storage/src/recursion.rs
---

# A commons pool is a collective that is party to its members' commitments

This is a design, not a build order. It graduates the research capture
`commons-data-pools-hot-path-and-external-tooling-2026-09-11.md`, which already inventories what
exists, lists what must be built, names eleven risks and surveys the field. This document does not
repeat that. It decides the one primitive the capture recommends, runs the design gate on it, and
adds the contract a pool owes to the people whose devices carry it.

Terms this leans on, and where each is defined: *reach* (how far a thing may travel, `private` to
`commons`) and *standing* in `glossary.md`; the entity classes (Notarized, Linked, Private,
Attested-Private, Ephemeral), the head-plane cost line and the numbered hard boundaries in the
`p2p-design-gate` skill and `private-thought-governed-fruit.md` §4; the concern classes (C0–C14)
in `.claude/epr-meta/concerns.yaml` and `policies.yaml`.

## 1. The one missing node

The services people expect from a large provider — shared storage no household holds alone, shared
inference, serving from an edge near the reader, compute on machines nearer the work — each
already have their parts in the tree. Every one of them stops at the same place: **no collective
can be a party to a commitment.**

- A compute grant is one agent's consent to one other agent. The recipient must be a Holochain
  agent key (`elohim-storage/src/api/compute_grants.rs:94-98`) and the provider is the serving
  node itself.
- A custody commitment is per agent, per item.
- A `Collective` exists as a notarized entry with members who may be people, other collectives or
  elohim agents (`imagodei_integrity/src/qahal.rs:24-48`). Nothing uses it as the counterparty of a
  grant, the holder of a resource, or the thing a member contributes to.

So a pool today could only be built as every member granting to every other member. That is the
node to add. It is one missing idea and several changes to how commitments are enforced; §3 lists
them, and they are not small.

- *Missing node:* chain `commons capacity` / between `a member's commitment to give storage or
  compute` → `another member's draw on it` / missing node: the collective as the commitment's
  counterparty, with the draw admitted by membership and reach; probe: a member who was never
  named in any grant draws on capacity another member committed to the pool / current state:
  absent; grants are bilateral.

## 2. The primitive

Taking the capture's recommendation (its §8 question 3):

- **A pool is a `Collective`** with a charter that says what the pool is for.
- **It holds a resource** classified `commons-pool:<kind>`. The kind is an open label that says
  what the pool is for (storage, index, inference, serving and compute are the ones §4 uses); it
  is not a closed list. The resource is the thing governed. Any treasury that funds its care is a separate
  resource; the two are never confused (`hardware-providence-commons.md` §8).
- **A member contributes by a commitment that names the collective**, using the actions that
  exist: `replicates-content` for storage, `delegates-compute` for compute. The collective's
  address goes in the commitment's `recipient` field. The bounds (`epr_scope`, `reach_ceiling`, a
  rate, a rotation) keep their present meaning; `epr_scope` names task addresses or `*` today and
  cannot name a pool.
- **A member's device gives no more than the constitutional share allows.** The commons band and
  the `collective_pct` field already bound it (`constitutional_ratio_registry.rs`).
- **A draw is admitted by membership and reach**, not by being named. The check reads: is the
  caller a current member of the collective this commitment names, within the bounds, at a reach
  the thing drawn admits.
- **A contribution can be withdrawn.** `revokes-commitment` exists; a `Membership` is withdrawn
  by an update that sets a withdrawal height, after which nothing accrues.

Nobody holds a pool's key, because a pool has none. A collective is founded by a person's signed
act (`founder_agent_cid`) and every later change to it — a membership, a role, a withdrawal — is
some member's own signed entry. Standing in a pool belongs to its members and is exercised through
their own keys.

**Lifecycle.** Founding: a person creates the collective with its charter and the pool resource.
Joining: a membership entry, sponsored where the role requires it. Contributing: a commitment
naming the collective. Drawing: admitted by the check below. Leaving: a withdrawal height on the
membership and a revocation of the commitments. Dissolving a pool is a governance act of its
stewards and is not designed here.

**The draw check, in order.** (1) The commitment exists, is not revoked and is inside its window
(the existing bounds checks). (2) Its recipient is a collective. (3) The caller's membership in
that collective is current. A withdrawal is an update to the membership, so reading the original
membership entry and finding no withdrawal height is the stale-state mistake the existing code
warns against (`qahal_coordinator.rs:572`); the check must use the existing current-membership
test, which follows updates. (4) The request is inside the commitment's scope, rate and reach
ceiling. (5) The thing drawn admits the caller at its own reach, by the existing read gate
(`api/content_reach_gate.rs`), on every path a pool serves through (§6). A failure at any step is
a refusal that names the step.

**Existing grants are unaffected.** A commitment whose counterparty is an agent key is checked as
it is today. The collective case is an added arm, not a replacement.

## 3. Design gate

### Entity: the pool

- **Classification:** Notarized (A), reusing the existing `Collective` entry and the existing REA
  resource. No new entry type.
- **Head-plane cost:** one collective and one resource per pool. Pools number in the tens to
  hundreds at one year, not thousands.
- **Address:** the collective's existing address.
- **Integrity zome and DNA-hash class:** no change. DNA-hash-neutral.

### Entity: a contribution

- **Classification:** Notarized (A), reusing the existing `Commitment` entry
  (`{action, payload_json, signed_at}`). The parties are fields inside the payload. The integrity
  zome requires `provider` and `recipient` to be present, non-blank strings and the bound fields
  to exist (`mishpat_integrity/src/lib.rs:859`); it does not require an agent key. The
  coordinator checks the rest (`commitments.rs:684-689, 1406`). A collective's address as the
  recipient string therefore passes integrity validation as it stands. DNA-hash-neutral, on the
  condition that the integrity requirements are left as they are; changing how a party is
  represented would need its own analysis.
- **Head-plane cost:** one commitment per member per kind of capacity per pool, never one per
  item contributed. At a thousand members and five kinds that is five thousand commitments for a
  large pool. That is above the count (about five hundred) at which the gate requires a bundling
  justification or the operator's sign-off. The justification is **composite root**: a member's
  commitment covers a range (an arc of the pool's holdings, a rate of compute), and what the pool
  holds is answered by one digest over the whole set, not by per-item records. The count grows
  with members, never with items, which is the growth the gate exists to catch. The operator's
  sign-off is still owed before a pool of that size is built. This is an
  order-of-magnitude reading from the gate's single measured anchor, not a computed number.
- **Address:** agent-scoped composite: the member, the pool, the kind.
- **Coordinator:** the existing commitment create, with `validate_delegates_compute` and the
  `replicates-*` validators accepting a collective address in `recipient`.
- **Enforcement changes in storage.** Admitting a collective is more than lifting one check:
  - grant creation requires the recipient to be an agent key (`api/compute_grants.rs:97`);
  - a task is authorized only when the grant's recipient equals the task's requester
    (`api/compute_tasks.rs:80`); for a collective this becomes "the requester is a current member
    of the recipient";
  - a capacity commitment for replication has no counterparty at all, and a content replication
    commitment uses the content's head as both recipient and scope
    (`mishpat_projection.rs:636`); a commitment of a range to a pool needs its own projection and
    its own enforcement;
  - membership must be resolved through its updates (step 3 of the draw check).
- **Routes:** none new. Compute records stay local-SDK-facing and travel between peers on the DHT,
  as today.

### Entity: a draw

- **Classification:** existing records. A compute request is an REA commitment
  (`rakia-compute-request-v1`, `content_store/src/compute_task.rs:253`); the holder's completion
  is an economic event (`:334`); serving bytes emits a `serve-blob` event. The requester-side `compute-fulfilled` event is a
  local projection today and is not notarized; that is a gap this design inherits, not one it adds.
- **A draw is recorded and names the one who drew.** A compute draw is a notarized request and
  completion, readable network-wide. A byte draw is seen by the holder that serves it. Reading a
  range of a large archive hides what the reader was looking for inside it; it does not hide the
  fetch. See the SDO and RWA test (named
  for social dominance and authoritarian following: what could the few who seek to dominate see,
  join and compel).

### Network stakes

A pool behaves under all four declared network stages (Simulacra, Bootstrap, Coordinated, Enforced;
`elohim-storage/src/trust/stage.rs`). The membership check and the reach check are
floor-protected: they never cheapen. What may be priced by stage is how deeply a member's
contributed bytes are re-verified.

### Per-plane shape

- **Reach:** a contribution is made at a declared reach; a derived artifact (an index, a model)
  can be no wider than the narrowest thing it was derived from.
- **Custody:** many holders chosen for independent failure, not for number. The threshold is a
  separate dial from the membership.
- **Freshness:** a read from a pool carries the head it was read at.
- **Linkability:** belonging to a pool is public. Drawing compute from it is a public record.
  Drawing bytes reveals to the serving holder that this member fetched them.
- **Cost bearer:** the members whose devices carry it, recorded as REA events.

### SDO and RWA test

If the largest holder in a pool, or a majority of its stewards, were captured: what could they
see, join and compel?

- **See:** more than a pool's own members can. As the code stands there is no membrane around a
  pool:
  - *Membership is public.* `Membership` is an ordinary public entry
    (`imagodei_integrity/src/lib.rs:947`) and listing a collective's members asks nothing of the
    caller (`qahal_coordinator.rs:128`). Anyone in the network can read who belongs to a pool.
  - *A compute draw is a public record.* The request names its requester and provider and carries
    its envelope (`content_store/src/compute_task.rs:252`), reading and listing requests is
    unguarded (`:150`), and the holder notarizes completion (`:334`).
  - *A byte draw is seen by the holder that serves it,* which knows who fetched which bytes and
    emits a serving event.

  So a pool built on these parts is one whose membership and whose draws on compute are visible to
  the whole network. That is acceptable for a pool whose purpose is public: a commons index, a
  public map, shared build compute. It fails boundary 6 (taking part is itself sensitive) for
  anything narrower, and boundary 2 (an activity ledger is held by the holon it describes). The
  accounting this design asks for in §5 and the privacy it would like are in direct tension: to
  record who carried a draw is to record the draw. The reconciliation that is specified and not
  built is the consumer-blinded census, which records the carrying without naming the reader.
- **Join:** a person's memberships in two pools carry the same member address, and membership
  lists are public, so anyone can join them. Boundary 5 (correlating identities across namespaces
  is an act of consent) is not met.
- **Not designed here, and required before any pool narrower than public:** membership that is
  readable only inside the collective; a member address that differs per pool; a draw record that
  does not name the reader; a draw that does not identify the member to the holder (admission
  proved to one party and bytes served by another, or a proof of membership that names no one).
  Until these exist this is a refusal: this design builds public-purpose pools only, and nothing
  sensitive to who belongs or who reads goes in one.
- **Compel:** nothing from a member's device beyond what that member committed, and that
  commitment can be revoked. A pool cannot require anything that must never leave a device.
- **What a pool may never hold:** a person's attention, revealed preferences, private notes and
  reflections (`manifesto.md:438`, boundary 1). A pool that would need them is refused. Compute
  travels to the data on the person's own device; the data does not travel to the pool.
- **Counted identities:** identities are free to mint, so any rule that counts them can be met by
  minting more. Admission to a pool therefore never rests on how many members vouch. A member's
  weight in a pool comes from what they have carried over time under a steward's
  standing.

A pool design that fails any line above is a refusal, not a mitigation to schedule.

### Concern canon, for the one new decision

The new decision is "may this caller draw on capacity committed to this collective". It answers
C12 (consent and authorization: the member consented to the collective, within bounds), C13
(graduated authority: reach admits, standing governs), C6a (bounded work: rate and window from the
bounds), C11 (backpressure: the device's constitutional share is imposed from outside the pool),
and C4 (honest absence: a pool with no reachable holder says so). It is registered in the crate's
seam registry when it is built.

## 4. The five services as compositions

Each row is built from parts that exist plus the one missing node, and each has a gap of its own.

| What a person wants | Built from | Conventional analog (atlas §8) | Its own gap |
|---|---|---|---|
| Shared data no household holds alone | `replicates-content` commitments to the pool; Reed-Solomon shards (`rs-4-7`); inventory exchange | object storage | holders can read what they hold; blind custody is specified, not built |
| Shared inference | `delegates-compute` to the pool with an inference scope; the agent-service backend seam | compute, ML | no model address, no real backend on a node, no delegation path; `canInfer` is a declared flag |
| Serving near the reader | the doorway's reach-keyed cache; `serve-blob` events; addressed archives read by range | CDN, edge cache | serving by a patron's own device has no story; a doorway re-asks standing at serve time only as a red habit |
| Compute near the work | `delegates-compute`; the compute request and its events; the tevah envelope for the guest | compute | no offer of a berth and no matching; fulfilment is not notarized |
| A view across many holders | flat coverage rollup; the pool's index as a measure its members compute over what they hold | analytics | no carrier for an aggregate across DHTs; no witness quorum; no bound on how small a counted group may be |

For a first pool that shares storage of public things, none of these gaps blocks: its holders
may read what they hold, and its membership and draws being visible is acceptable for a public
purpose (§3). Blind custody blocks any pool whose holders should not read. The inference, compute and cross-holder rows each need their
own gap closed before that service exists at all.

Two things in this table have no conventional analog and are where the design is stronger than
what it resembles: admission by earned reach in place of a price, and recovery by people who know
the member in place of a provider's support desk (atlas §8, the inversion).

## 5. What a pool owes the devices that carry it

This is what the capture does not say, and it is the local-first reading of a pool.

1. **A pool is never a precondition.** Whatever a person could do with their own device and their
   own peers, they can still do when every pool is unreachable. A pool adds capacity. It does not
   become the place a person's own things live.
2. **A pool that vanishes loses capacity, not anyone's things.** Each member's contributions are
   their own addressed things, held on their own device as well. What the pool derived from them
   is gone until rebuilt.
3. **Everything a pool derives can be rebuilt and is addressed.** An index, a model, a tile
   archive: each is made from the contributions by a recipe that is itself an addressed thing.
   Losing the derived thing costs time, not truth.
4. **Compute goes to the data.** A recipe travels to a member's device and runs there over what
   that member holds. Only what the member agreed to contribute leaves. What arrives is bounded
   too: a recipe is an addressed thing the member's own commitment admits by scope, it runs inside
   the device's compute envelope with a quota carved from what the member gave, and a recipe the
   commitment does not name does not run.
5. **A draw is honest about who can see it.** Today a compute draw is a public record naming the
   requester, a byte draw is seen by its holder, and membership is public (§3). A pool says so to
   the people who join it. A pool that needs less visibility than this waits on the work §3 lists
   and is not built before it.
6. **A read says how fresh it is.** It carries the head it was read at; a stale answer is marked
   stale.
7. **Giving is bounded from outside the pool.** The share of a device a pool may use is set by the
   constitutional band and by the person, never by the pool.
8. **Leaving is clean.** A member who withdraws takes their things, stops carrying, and stops
   accruing, from a stated moment.
9. **Who carries it is recorded.** Carrying storage or compute for a pool is an economic event.
   No pool treats its holders as free.

## 6. Admission, and what it waits on

Reach admits, standing governs, and carrying is recorded. Two things this depends on are not
ready, and the design says so plainly:

- **A reader's reach is checked on some paths, not all.** Content reads over HTTP pass a gate that
  asks what this reader may see (`api/content_reach_gate.rs`). The document-sync routes and the
  peer-to-peer document handler ask nothing, and the capture records the blob pantry re-serving
  without a re-check. There is no freshness stamp on a read. A pool may serve only through paths
  that carry the check; the capture's row 1 is narrower than it reads there, and still open.
- **Standing has no settled record.** The design for standing records failed review and is being
  redrafted. Who may steward a pool, and who may speak for it, waits on that. A pool's membership
  and roles (`Steward`, `Contributor`, `Observer`) exist and are enough for a first public-purpose pool among
  people who already know each other.

## 7. Open questions for the operator

Carried from the capture's §8 with its recommendations. These are decisions, not gaps.

1. **The first pool.** Recommended there: a street-imagery pool, because it exercises contribution,
   provenance, derivation at the sensor and serving in one corpus. A smaller first step is a pool
   whose only capacity is storage of public things among the household mesh's three peers, which
   exercises the missing node and nothing else. Both are public-purpose pools, which is all this
   design can build (§3).
2. **The revenue cap.** A pool that funds itself past a declared share of its members'
   contribution is captured. Recommended there: a locked ceiling per kind of pool.
3. **Who may train on a pool.** Recommended there: contribution terms carry reciprocity,
   recognition and sustainability as fields; a model trained on a pool is a pool asset with an
   address; enforcement is ours.

## 8. What this absorbs

- The capture's §3 row 2 (pool as a collective) and §8 question 3 (where the primitive lives) are
  decided here.
- Its §3 rows 1, 3–14, its risks and its survey stand as they are and are cited, not restated.
- Its §9 outputs that this decides are minted as backlog rows in their existing files.
