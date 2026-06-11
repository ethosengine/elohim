---
title: EPR Acquisition Family — Dual Pins, the Async Pull Queue & the Striping Seam — Design
id: epr-acquisition-pull-queue-design
status: Draft
class: protocol-canonical
topic: [acquisition, pull-queue, pin, dual-pin, cluster-sync, closure, multipeer, striping, shard, replicates-commons, provide-commitment, offline, download, affordance-ladder, epr-link, reconcile, desired-set]
domain: D5
informed-by:
  - genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md
  - genesis/docs/content/elohim-protocol/architecture/2026-05-02-blob-custody-reconciliation-design.md
  - genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
refines: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md
cites:
  - epr-route-claims-link-conformance-design | the parent spec this refines — its Appendix E is the verbatim acquisition-family seed (affordance ladder + pull-queue + striping evidence audit); its R1-R3 gradient invariants and §5.3 hint envelope are inherited rails | sha256:30b7cd1baf222922 | path: genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md
  - genesis/data/timeline/backlog/epr-routing-complementary-captures.md
  - genesis/plans/2026-04-06-identity-driven-replication-design.md
  - genesis/plans/2026-03-22-epr-body-plane-design.md
  - genesis/plans/2026-03-30-resilient-html5-app-delivery-design.md
  - blob-custody-reconciliation-design | the FetchBlob per-peer contract (timeout, fail-fast-on-mismatch, serve-only-verified) the pull queue inherits, plus the candidate-list/no-Kademlia-for-blob-hashes resolver seam (R-I) | sha256:b5a567ba337539a2 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-blob-custody-reconciliation-design.md
  - tiered-quilt-stewardship-design | the D5 custody seed — RS(N,K) erasure quilt the striping seam reconstructs against; quilt diversity accounting the dwelling tier participates in; source of the BLAKE3-vs-sha256 divergence flagged in §14 | sha256:9f9c6a1c391712b3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-tiered-quilt-stewardship-design.md
  - records-lifecycle-design | the typed relation vocabulary and lifecycle the cluster-closure walks (reach boundary + closed-state termini), and custody-quilt Commitments as accepted-at-authoring REA — the pin commitment shape | sha256:2b5f54d20108bcf0 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-24-records-lifecycle-design.md
  - genesis/docs/architecture/rea-compute-commitment-primitive.md
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - genesis/docs/superpowers/specs/2026-04-30-trust-compute-gradient-brainstorm.md
  - elohim-protocol-specification | protocol-specification | sha256:659b0d47078b298f | path: genesis/docs/content/elohim-protocol/protocol-specification.md
  - genesis/docs/content/elohim-protocol/architecture/cluster-topology.md
  - doorway/CLAUDE.md
  - rea-compute-substrate-native-roadmap | 2026-05-28-rea-compute-substrate-native-roadmap | sha256:64e5ffe3b8756e6e | path: genesis/docs/superpowers/plans/2026-05-28-rea-compute-substrate-native-roadmap.md
  - compute-commitment-substrate-floor-design | Substrate Floor / Elohim Ceiling | sha256:614e30134ee0d7ab | path: genesis/docs/content/elohim-protocol/architecture/2026-05-04-compute-commitment-substrate-floor-design.md
  - sprint2-bounds-validator-standing-aggregator | 2026-05-28-sprint2-bounds-validator-standing-aggregator | sha256:8923ad357ea4ee80 | path: genesis/docs/superpowers/plans/2026-05-28-sprint2-bounds-validator-standing-aggregator.md
---

# EPR Acquisition Family — Dual Pins, the Async Pull Queue & the Striping Seam

**One coherent feature seen from three altitudes:** what a link lets a person DO beyond browsing
(the affordance ladder), the substrate primitive that executes the want (the async pull queue),
and the bandwidth underneath (the multipeer striping seam). Elevated from Appendix E of the
route-claims/link-conformance spec (the verbatim evidence audit) and the BRAINSTORM SEED capture
in `epr-routing-complementary-captures.md`.

**Normative depth cut (operator-adjudicated):** the pull queue and ladder rungs 2–4 are spec'd to
implementable depth; cluster-sync (rung 5) is normative at the closure-contract level; multipeer
striping is **seamed, not built** — the composition seam is named normatively so the queue's
per-item resolver contract is unambiguous, and a follow-on spec implements it.

> **Verification topology constraint** (cluster-topology canon): any leg claiming P2P resilience
> is accepted only in the multi-node topology, never a single-box simulation. Every normative leg
> in this spec is testable on `household-nodes` today; the held legs are tagged inline
> (`@requires:<cap>`), so this doc declares **no doc-level `requires_env`** (mixed-plan
> convention).

---

## §1 The dual-pin model (normative)

A pin is **two objects with one UX**.

### §1.1 The device pin (the airplane-mode floor)

The primary, durable object is the **device pin**: a local want declaration
`(agent, head, kind, closure_rule, priority)` that is **fully functional offline** — declarable,
browsable, executing against whatever bytes are already local — with **zero hub and zero DHT
write**. This is the hub-optional floor (one device, no hub, full participant) made concrete in
an affordance. When connectivity returns, the pin *syncs back*: it resumes its inward pull AND
notarizes its provide half (§6).

- A device pin MUST be creatable and effective with no conductor round-trip and no network.
- Device pins are agent-scoped private (Category B): peers never validate them; no HTTP surface
  serves them to other agents.
- v1 working store: a local table in elohim-storage (`acquisition_pins`,
  `-- Source of truth: local (agent-scoped device pin; roams via export, not gossip)`). The
  private source-chain entry is the canonical Category-B roaming home this graduates to when
  pin roaming across devices is wanted; v1 keeps pins per-device deliberately (a phone and a
  hub legitimately want different pin sets).

### §1.2 The provide tier (the notarized shadow)

At sync-back, a commons-reach pin writes its **ProvideCommitment**: an REA Commitment on the
EXISTING Mishpat entry type authored with the **canonical `replicates-commons` action** — the
commons tier of the storage graduation gradient (dwelling-hub design §2.1: free → dwelling →
collective → **commons**). A pin authors the **content-scoped payload variant** of
`replicates-commons` (a specific head_ref/closure provided at reach=commons), as distinct from
the *capacity-pledge* variant (a device's bulk commons donut contribution); ONE commons action,
two payload shapes — composed, not forked (see §6.1). The rea-compute-commitment 5-step recipe
applies; no new DHT entry types; Mishpat headroom untouched (actions are discriminators, not
entry types). Downloading BECOMES provisioning: the read→host loop the trust-compute gradient
expects.

**Graduation = on `ProvideAnnounce` (D4 decided).** A self-directed commons commitment has no
counterparty to sign acceptance, so instance-1's (`replicates-dwelling`) bilateral-by-reference
graduation does NOT transfer. Taking *intent-first, observed-state-second* to its end: the
**act of providing IS the acceptance** — the commitment graduates `proposed → active` when its
first `ProvideAnnounce` EconomicEvent observably fires (§6.3), never auto-active at author time
(which would mint standing a node hasn't yet backed with bytes). This also sidesteps the
**`proposed`-inert trap**: a commitment left at `proposed` scores nothing in the prioritizer and
lights no provide-rows — graduation is the gate the whole loop waits on.

### §1.3 The dwelling tier (co-stewardship, a dial — designed here, implemented later)

Backing a personal pin up to dwelling-level custody is a **distinct escalation with real two-way
consent**: a co-stewardship commitment at the data layer, `in_scope_of: household_id`,
**adjacent to but never conflated with** `replicates-dwelling` (a dwelling = a person's home
content; a pinned album is not that). "Hubbiness is a role with a dial": the dwelling tier is an
*enhancement* the household opts into per-pin or per-policy — it never gates the device pin, and
every rung below it works with no hub at all. The escalation UX (who consents, how the dial
reads, what the household sees) is protocol surface in its own right — designed as this contract,
implemented in a follow-on slice when the consent surface exists.

### §1.4 The capability gradient (honest affordance availability)

The ladder is also a device-capability gradient:

| Rung | Affordance | Requires | v1 reach policy |
|---|---|---|---|
| 1 | Browse | any | per claims/dispatch spec |
| 2 | Open in {pillar} | any | per claims table |
| 3 | Download / offline | any (browser = SW lane) | any reach the person can read |
| 4 | Pin as peer | peer-capable device (storage trio / Tauri-direct) | **commons only (v1)** |
| 5 | Sync a cluster | peer-capable device | **commons only (v1)** |

**Commons-only pinning (v1, operator-adjudicated):** rung 3 serves nobody, so it works at any
readable reach. Rungs 4–5 make your replica *serve*, which collides with the captured
**capability-by-hash open decision** (`GET /blob/{hash}` is reach-ungated today; holding a gated
EPR's blob hash is holding the bytes). This spec QUARANTINES that decision: pinning is
`reach == commons` only; the gated-pinning extension point is declared (§14) and depends
explicitly on capability-by-hash being either canonized or replaced with blob-reach enforcement.
Nothing in this spec forecloses either resolution.

---

## §2 Inherited invariants (normative rails)

These are restated as MUSTs of this family; each is evidence-anchored, none is new.

- **R-A — Byte-arrival, never inventory-arrival.** Inventory/gossip exchange is metadata-only.
  A pin is satisfied by **verified bytes on disk**, asserted as such; `caughtUp` from a
  fresh-empty node before bytes arrive is the known false-success shape. The parity-sweep
  assertion (gossiped-set == filesystem-set) is the regression defense to reuse.
- **R-B — Fan-IN at the requester, never fan-out at the doorway.** Striping = N parallel
  per-shard requests issued FROM the requesting elohim-storage peer (the shape `race_fetch`
  already has). The doorway forwards each request to a single storage target and never iterates
  peers (`doorway/CLAUDE.md` No Blob Fan-Out); it gains no shard/bitswap behaviour from this
  family. Swap test: any sibling doorway must serve identical content.
- **R-C — The verify split.** Blob/shard verification is **sha256-of-bytes** (the implemented
  path: `blob_fetch.rs:258-267`, `blob_store.rs:496-503`, `loader.ts:105-125`, format
  `sha256-<hex>`); EPR atom verification is CBOR-canonical CID recompute. Never conflate them.
  Verification is always caller-side (the codec never re-hashes). The tiered-quilt design's
  BLAKE3-for-RS-restitution intent is an unreconciled divergence: flagged §14, not silently
  unified.
- **R-D — Hash mismatch is hard-fail, not failover.** Corrupt data is corrupt regardless of
  transport. Transport errors fail over to the next candidate; a hash mismatch rejects the item
  (per-shard: reject that shard, refetch *that shard* from another peer — never serve, never
  whole-blob restart).
- **R-E — Paced drain, accountable to stewarded compute.** Pacing is a rail, not a knob:
  slot-backpressure (inherit `MAX_REPLICATION_INFLIGHT`-style caps), **retry-on-next-cycle**
  (never immediate requeue — the freeze-at-partial battle-scar), peer-gated dispatch (no peers ⇒
  no progress and no phantom counts), AIMD batch pacing scaled to the device's available compute.
  **One controller pattern governs all reconcile streams** (§4).
- **R-F — Standing modulates ordering, never retrievability.** Trust-weighted peer scoring may
  rank candidates and tune timeouts; CID-targeted lookup stays unconditional and low-standing
  fallback is mandatory. Live `Standing` is `Unknown` until Phase 3.5 — scoring MUST degrade
  gracefully to trust-agnostic availability scoring today. `@requires:alpha-cluster-6peer` for
  the trust-weighted leg.
- **R-G — Acquisition is not tending.** Tending is post-arrival, peer-private discernment; a
  pull/prefetch decision is pre-arrival acquisition. A tending agent may veto a *prefetch-hint*
  lane item (the epr-summary-hint remains the pre-fetch decision envelope, R3 of the conformance
  spec); tending never touches an explicit pin, and the desired-set does not reuse
  AttentionTending machinery.
- **R-H — The commons fast path is untouched.** All acquisition compute (closure resolution,
  scoring, verification) runs on the *requesting* peer or off the request path entirely; nothing
  in this family adds per-request governance compute to commons dispatch (conformance spec
  R1/R2).
- **R-I — No discovery-first Kademlia for blobs.** `KadStartProviding` stays narrow to EPR atom
  CIDs. Blob location resolves via the inventory/candidate-list contract — the queue's per-item
  resolver calls the single "give me candidates for this blob hash" fetch-helper seam and
  inherits its graduation path.
- **R-J — Blob bytes never travel via Automerge sync.** Automerge moves metadata/progress
  documents (delta sync); bytes move on the blob/shard protocols. Distinct fetch units, distinct
  verification.

---

## §3 Entities (P2P design gate record — adjudicated 2026-06-07)

| Entity | Class | Identity | Truth | Projection |
|---|---|---|---|---|
| DevicePin | **B** (agent-scoped private) | composite `(agent, head, kind)` | local store | `acquisition_pins` (no dht_anchor) |
| ProvideCommitment | **A** (existing Commitment entry type, action=`replicates-commons` / content-scoped payload) | commitment-body CID | Holochain DHT | `rea_commitments` (dht_anchor: yes) |
| DwellingPin | **A** (same entry type, `in_scope_of: household_id`) | commitment-body CID | Holochain DHT | `rea_commitments` (dht_anchor: yes) |
| ProvideAnnounce | **A** (existing EconomicEvent, `bounded_by: <commitment CID>`) | event CID | Holochain DHT | REA projection |
| ClusterClosure | **A2 + C** (rule rides the pin body; membership is recomputed projection) | n/a (derived) | derived over typed relation links | none stored |
| PullQueueState | **C** (operational; in-memory + derived counts) | n/a | recomputed (active pins × local inventory) | wire counts only (§4.4) |
| ShardRangeRequest | wire message (no persistence) | n/a | n/a | n/a |

**No new DHT entry types anywhere in this family.** Anti-pattern checks passed: no UUID PKs
(commitment CIDs / composite keys); HTTP designed last (§4.4, §6.3); every new table carries a
source-of-truth comment; one canonical address per entity; `ContentToSync` not used as a
query-only membership marker (it becomes a real producer/consumer pair instead, §5.3).

---

## §4 The async pull queue (the acquisition stream)

### §4.1 Controller architecture (Approach B — sibling stream, shared rails)

A new **acquisition reconcile stream** (`acquisition.rs` in elohim-storage) implements the same
controller pattern as the running replication loop — and the "one pattern" rail is honored **in
code**: the proven rails (slot-backpressure, retry-on-next-cycle, peer-gated dispatch, AIMD
pacing, tri-state completion) are extracted into a shared `reconcile_rails` module consumed by
BOTH streams. The replication loop's semantics are untouched (it remains the node-level
whole-inventory policy stream); the acquisition stream is the *selective, user-declared* stream.
This is the P1 reconciliation-controller pattern pointed at the local node: the desired-set is
the manifest; the stream is the controller.

### §4.2 The reconcile cycle

1. **Wants** = active DevicePins, closure-expanded (§5) to concrete item sets.
2. **Gap** = wants × local inventory diff → priority-laned queue:
   `explicit pin > closure member > prefetch hint`. Lanes are strict-priority with aging (a
   starving lane ages items upward; the explicit lane is never preempted).
3. **Dispatch** per item through the candidate-list contract (R-I): ≤16MB whole-blob via the
   existing scored parallel `race_fetch`; >16MB bands via the striping seam (§9) once built —
   until then the >64MB RS band is honestly **unfetchable** by this queue (the 64MiB codec cap
   is below the RS trigger; striping is the only transport for that band, not an optimization)
   and surfaces as `failed` with a named reason, never silently pending.
4. **Done-signal** per item: sha256-verified bytes durably stored (R-A, R-C). Failures are
   visible in counts; retry on next cycle (R-E).

### §4.3 Completion semantics (the tri-state contract)

Per-pin status: `{total, fetched, pending, failed, caughtUp}` — the **unified vocabulary**
(deliberately neither shipped queue's: drain is `{total, published, pending}`, replication is
`{pending, completed, failed, caughtUp}`; this family unifies so a future wait-for-* shares one
contract).

- `total: null` ⇒ closure not yet resolved ⇒ **keep waiting** (never caughtUp).
- `total: 0` (resolved-empty) is a DISTINCT state from unresolved-null and surfaces as a
  warning-grade outcome — a zero-link closure must never false-complete silently.
- `caughtUp` ⇔ **byte-arrival complete**: `total > 0 ∧ fetched == total` (every wanted item's
  sha256-verified bytes durably stored) ∧ `total ≥ expectedMin`. This is `pending == 0` AND
  `failed == 0` by construction, but stated as `fetched == total` deliberately: a **failed**
  item is removed from `pending` until the next reconcile re-queues it, so a naive `pending == 0`
  test would false-complete in that transient window with `fetched < total` — violating R-A.
  caughtUp tracks *bytes that arrived*, never *queue momentarily empty*. For a cluster pin
  expectedMin = the resolved closure count; for a single-item pin, 1. **`wait-for-*` terminates
  on `caughtUp`** (not `pending == 0`): content that can never be fetched (retries exhausted)
  never reports caughtUp and correctly TIMES OUT — the honest outcome, not a false success.
- Restart recomputes everything (Category C): no persisted queue, no resumption bug class.

### §4.4 Wire surface (designed last)

- `P2PStatusInfo.pull` — global rollup counts (ts-rs exported, schema-contract-tested, sibling
  of `.drain` / `.replication`).
- `GET /api/v1/pins` + per-pin scoped counts (own node only) — enough for a "7/12" progress
  affordance. Item-level detail stays device-local machinery; the wire is counts-only like its
  siblings.
- `wait-for-pull` (seeder/tooling): same tri-state poll loop as `wait-for-drain`, against per-pin
  or rollup counts.

---

## §5 Cluster closure (rung 5's contract)

### §5.1 The closure rule (typed relation set + depth cap)

A cluster pin names a **head EPR** and a closure rule carried IN the pin body:

```jsonc
{
  "head": "<epr-id>",
  "relations": ["contains", "part-of"],   // structural set; "prerequisite" opt-in per pin
  "depthCap": 4,                           // explicit, operator/persona-bounded
  "sizeBudget": "2GiB"                     // refuse-to-start if exceeded at resolve time
}
```

The walk traverses the typed structural relation family (album→track, course→module→lesson),
with cycle detection. Substrate-enforced termini come free: the **reach boundary** (a closure
cannot escape the head's reach without a federation hop) and **closed-state termini** (closed
EPRs reject subordinate traversal). 1-hop lookups ride the Diesel edge tables; multi-hop walks
are a graph-views concern — the resolver is the first real consumer of that bifurcation.

### §5.2 Resolution precedes fetching

The resolver MUST produce a **concrete membership count** before any closure item enters the gap
queue (`total: null → N`). Resolution is itself paced/off-request-path compute (R-H) and re-runs
when the head's graph changes (a closure pin is a *standing* want: new tracks added to a pinned
album enter the queue on re-resolve — the same eager-reconcile posture as the controller
principle).

### §5.3 `contentToSync` gets both ends

The existing-but-unwired `GateHintRelation::ContentToSync` becomes real:

- **Producer**: the projection emits a `contentToSync` gate hint on a head's epr-summary-hint
  when the head has closure-eligible typed relations (i.e., "this is a syncable cluster").
- **Consumer**: the link surface (§8) uses the hint to decide rung-5 affordance visibility
  *before* any fetch — the same pre-fetch decision envelope discipline as the rest of the hint
  system.

---

## §6 Sync-back & the provide tier

### §6.1 The action: content-scoped `replicates-commons` (5-step recipe)

**Composed, not forked** (compose-lens reconciliation 2026-06-08): the commons tier of the
dwelling-hub graduation gradient already reserves the **`replicates-commons`** action
("universal constitutional contribution", dwelling-hub design §2.1 + the §4 floor-check
follow-up). A pin authors a **content-scoped** `replicates-commons` commitment — the SAME action
as the *capacity-pledge* variant (bulk commons donut contribution), distinguished only by its
payload shape. **This Slice 2 IS the commons-tier follow-up sprint** the dwelling-hub design
named, and it MUST close that design's flagged gap: replace floor-via-declaration
(`ratio_attestation.commons_pct` accepted as intent) with a backing-pledge requirement (commons
declarations un-backed by an active `replicates-commons` commitment fail `bounds_validator`).

Per the rea-compute-commitment recipe: (1) action discriminator `replicates-commons` added to
the Mishpat coordinator + integrity validators (mirror `replicates-dwelling` at
`mishpat/src/commitments.rs`); (2) JSON schema with a payload **union** —
`{ variant: "content", head | closure_rule, reach: "commons", bounds }` and
`{ variant: "capacity", commons_bytes, bounds }` — at
`elohim/sdk/schemas/v1/commitments/replicates-commons.schema.json`; (3) bounds-validator reuse
(every subsequent provide EconomicEvent carries `bounded_by: <commitment CID>`; the 7-check
`bounds_validator::validate` walks the back-ref and refuses out-of-bounds/revoked events);
(4) coordinator handler; (5) the §5 canon row. This **closes the Epic-B gap properly**:
production commons provide-rows finally exist outside `test_util` (which used the shorthand
`action="provide"` — reconcile to `replicates-commons`).

> **Verify the instance-1 rail, don't assume it** (semantic-lens caveat): `replicates-dwelling`
> is design-complete but verification-incomplete in code (the dwelling-hub plan's 77 unchecked
> steps + named `commitmentBackedReplication`/`replication_commitments` stubs). Slice 2's first
> implementation task confirms the conductor-path commitment write + validator actually fire on
> `household-nodes` BEFORE building the content-scoped variant on top.

### §6.2 The scorer arm

`replication_prioritizer::active_commitments_for_provider` filters `action == "replicates-dwelling"`
today; the arm widens it to also load active `replicates-commons` rows and parse the
content/capacity payload. `score_advertised_blob` is otherwise action-agnostic, but the matching
axis differs: a content-scoped commons commitment matches the advertised blob by **content
identity** (head_ref / closure membership), not by recipient-hub — and the scorer's reserved
**`Medium` tier** (shipped as a reservation in instance-1, High/Skip only) is the commons-tier
priority slot to implement here. The dwelling tier (§1.3) keeps its own `replicates-dwelling`
arm; the three commons/dwelling vocabularies stay distinct in scoring and in mutuality/capacity
audits.

### §6.3 Announce, reciprocity, revocation

- Sync-back emits the **ProvideAnnounce EconomicEvent** (`bounded_by` the commitment) — the
  notarized announce-as-provider step; inventory advertisement then carries the new holdings on
  the existing metadata gossip (which is and remains metadata-only, R-A).
- **Un-pin is real revocation**: the substrate refuses subsequent provide events the moment the
  commitment is revoked; prior accepted events stand (no retroactive invalidation mid-walk).
  Local bytes MAY be garbage-collected after revocation per device storage policy.
- Reciprocity is observable, not enforced-unilateral: completing a pull at commons naturally
  flips the node to providing; the gradient economics reward the loop rather than gate it.

### §6.4 Substrate-prerequisite split — Slice 2a (rails) then Slice 2b (the action) [planning finding 2026-06-08]

Planning §6 surfaced (with file:line proof) that §6 assumed REA rails that are **storage-side
unwired**, even though the DNA side is ready. The substrate map:

| Rail | DNA (elohim content_store) | Storage side | Slice 2a work |
|---|---|---|---|
| EconomicEvent create | ✅ `create_rea_economic_event` (`lib.rs:12124`) + `ReaEconomicEventCommitted` signal (`:10892`) | ❌ no conductor-emit wrapper (only diesel-direct `record_event`, state `recorded`, no anchor) | add `call_create_rea_economic_event` wrapper + a conductor-path emit service |
| Commitment state transition | — | ✅ `call_update_rea_commitment_state` wrapper EXISTS | wire graduation: on first `bounded_by` event projecting, flip the proposed commitment → active |
| `bounded_by` | enforced in `create_rea_economic_event` metadata validation (`lib.rs:12146`); column on `economic_events` (`+bounded_by` migration) | projection populates from the entry | run `bounds_validator::validate` (7-check) on the emit path so revoked/out-of-bounds events are refused |
| Commitment→content scorer data | — | ❌ STUB: `DistributionDetails.replication_commitments` = `[]`, `commitment_backed_replication` = zeros (instance-1 deferred the rea_commitments-by-content query) | finish the query so the scorer can see commitment backing |

These rails are **general-purpose** (they also unblock the dwelling-hub instance-1 stubs), so they
are factored into a foundational **Slice 2a** (REA economic-event emit + commitment graduation +
bounds-on-emit + scorer-data) that lands and is verified on `household-nodes` FIRST. **Slice 2b**
then builds the `replicates-commons` content-scoped action + the pin sync-back + the scorer arm +
rung-4 UI *on top of the proven 2a rail* — honoring the "verify instance-1, don't assume it"
caveat (§6.1). Plan: `2026-06-08-epr-acquisition-slice2a-rea-rails-plan.md`.

### §6.5 Two-commitment-system reconciliation [brainstorm finding 2026-06-08 — execution-halted-at-T3, then composed]

Executing Slice 2a (T1–T3 landed) surfaced — and a /brainstorm pre-step resolved — that the rail
table above under-modeled the commitment substrate. **There are two commitment writers, and the
split is canonical (per `compute-commitment-substrate-floor-design` + `rea-compute-substrate-native-roadmap`),
not a bug to unify:**

- **`Mishpat::Commitment`** (mishpat DNA) is THE compute-commitment **substrate primitive** — the
  **policy envelope**: `payload_json` bounds, `valid_from/until`, `revoked_at`, scope; validated by
  the single substrate-side `bounds_validator`. `replicates-dwelling` rides it; **`replicates-commons`
  must too** (Slice 2b mints it here, mirroring the dwelling-hub schema). The roadmap's thesis:
  *"one substrate primitive (`Mishpat::Commitment` + `bounded_by` event back-reference + single
  bounds validator) … all events carry `bounded_by`."*
- **content_store `Commitment` / `EconomicEvent`** (elohim DNA) is the **REA/ValueFlows economic
  fact** — provider/receiver/resource/quantity; the event's `fulfills` link points here.

**One event references BOTH, orthogonally:** `bounded_by` (in `metadata_json` → the
`economic_events.bounded_by` column) → the **Mishpat** policy commitment (what the bounds-gate
checks); `fulfills` → the **content_store** REA commitment (accounting; may be empty for a pure
provide). The historical failure to design around (`CoordinationEnvelope` bypass): a `ProvideAnnounce`
must `fulfills`/`bounded_by` a **real notarized** commitment, never a projection-only ghost — the
unbridged state IS the bug.

**The genuine Slice-2a work is therefore the `rea-compute-substrate-native-roadmap`'s unfinished
Sprint-1 stubs, composed (not forked):**

| Corrected rail | What to build (compose onto the roadmap) |
|---|---|
| **Mishpat-commitment projection** | a projection table carrying `dht_anchor_hash` + `payload_json` bounds + `valid_from/until` + `revoked_at` + `state`, populated from the Mishpat `create_commitment` post-commit signal (the canonical DHT-first → projection wiring, `notary-anchors-sdk-boundary-design`). `rea_commitments` lacks these columns. |
| **`ProjectionCommitmentFetcher`** | replace the `ConductorCommitmentFetcher` stub (`ConductorUnreachable`) — bounds-checks READ the projection (P1 reconciliation-controller; `depin_contracts_are_policy` "operational loops read bounds, never create contract entities"), **guarded by `dht_anchor_hash`**: refuse a bounds-pass on a null-anchor (un-notarized) row. |
| **Graduation (`proposed → active/accepted`)** | NOT a mutable status column as truth. Holochain immutability ⇒ state transitions author **new link entries on `CommitmentByState` anchors** (`records-lifecycle-design` §A.5/§5, `COMMITMENT_STATES` 6-state machine); the SQL `state` column is the **projection**, the link/event is **truth**. For a self-directed commons commitment, **the first `ProvideAnnounce` EconomicEvent IS the acceptance** (§6.1 confirmed) — it authors the state-link; the projection reflects `active`. |

T1–T3 (probe, `call_create_rea_economic_event` wrapper, bounds-validated `economic_event_emit_service`)
remain valid and sit on top of this. The emit service's `bounded_by` annotation already targets the
Mishpat CID; what's missing is the projection + fetcher + the `CommitmentByState` graduation, plus
minting `replicates-commons` as a Mishpat action (Slice 2b). **Slice 2a is re-scoped to finish the
roadmap's Mishpat-commitment projection + `ProjectionCommitmentFetcher` + `CommitmentByState`
graduation** (verify the live `ConductorCommitmentFetcher`/trait against
`bounds_validator.rs` first — palace is behind 2026-06-02). The "why both commitment writers exist"
decision is **history-record-worthy** (currently pattern-only in the corpus). Compose-targets:
`refines:` `2026-05-28-sprint2-bounds-validator-standing-aggregator` (the `CommitmentFetcher` seam);
`cites:` `rea-compute-substrate-native-roadmap`, `records-lifecycle-design` (CommitmentByState),
`compute-commitment-substrate-floor-design`.

---

## §7 The dwelling escalation (designed contract, deferred implementation)

The device→dwelling graduation is a consented, two-party flow: the pinner proposes
("back this up at home"), the dwelling steward accepts (the tier with REAL
proposed→accepted semantics — unlike self-pins), producing a household-scoped commitment
(`in_scope_of: household_id`) whose custody participates in quilt diversity accounting. The
**dial**: a household policy may pre-consent classes of escalation (auto-accept family pins
under N GiB), making hubbiness a graduated role, not a switch. Nothing at this tier is required
for any lower rung (hub-optional floor). Implementation waits for the consent surface; the
contract here is normative so the lower tiers don't paint over it.

---

## §8 The link surface (the ladder rendered)

- Rungs ride the **existing** `contextMenuItems` injection on `<elohim-epr-link>` (host-injectable
  items re-emitted as `epr-menu-select` — the hook the code comments already call "Epic E"). The
  default menu stays `{Open, About, Copy}`; the acquisition items are host-composed.
- Menu composition is capability- and reach-aware (§1.4): *Open in {pillar}* from the claims
  table (conformance §7.5); *Download* whenever readable; *Pin as peer* / *Sync cluster* only
  when peer-capable ∧ commons (v1); rung-5 visibility from the `contentToSync` hint (§5.3).
- **Progress** renders in the popover/viewer chrome fed by per-pin counts ("7/12") — NOT bolted
  onto `ContextMenuItem` (which stays `{id, label, disabled?}`); a small
  `<elohim-pin-progress>`-class affordance is the first consumer of `.pull` counts.
- Downloads (rung 3) reuse the Loader's CID-verify-on-load pattern (default-on, hard-fail) as
  the per-item fetch+verify. Browser-only users get the SW offline lane (cache persistence
  against already-projected content) — no DevicePin object exists in the browser; the pin
  primitives require a peer-capable device per §1.4, and the SW lane is rung 3's honest browser
  rendering, not a second pin store.

---

## §9 The striping seam (named normatively, built in a follow-on spec)

**The seam:** a new `ShardRequest` variant addressing `{blob_hash, shard_index}` on the existing
`/elohim/shard/1.0.0` protocol × `race_fetch`'s scored-parallel loop **re-keyed per shard hash**
× RS(N,K) `reconstruct` (already implemented and unit-tested in `sharding.rs`, needing only K of
N — redundant fan-in from a scored peer subset) × **per-shard sha256 verify** with
reject-one-refetch-one (R-D) × final whole-blob verify after reassembly.

Facts that make this seam load-bearing rather than optional:

- The 64MiB codec cap (`HARD_MAX_RESPONSE_SIZE`) is below the >64MB RS trigger — RS-band blobs
  cannot transit the blob protocol whole. The 16–64MB chunked band has the same gap (no
  chunked-fetch variant exists).
- The epr-body-plane design already names this extension point ("decoupled for future
  multi-peer resolution… the contract stays stable") — this seam lands inside it.
- The resilient-delivery 6-tier peer-preference rubric (proximity, delivery capability, recency,
  tier, warm-hash-match) is the scoring to reuse, trust-weighting deferred per R-F.
- **bitswap is explicitly NOT the seam**: the port is mounted serve-only (zero get/sync callers;
  progress events dropped) and addresses whole CID-blocks over pre-chunked DAGs it would first
  need authored. It remains a complementary DAG-block transport candidate, captured §14 — the
  acquisition driver this spec needs is shard-shaped, not block-shaped.

The doorway never participates (R-B). Until the seam is built, §4.2's honest-failure rule
applies to the bands that need it.

---

## §10 Error handling & edge cases

| Case | Behavior |
|---|---|
| Closure resolves empty | `total: 0` distinct state; warn, never silently caughtUp (§4.3) |
| Head graph grows after caughtUp | re-resolve flips pin back to pending (standing want, §5.2) |
| Peer serves corrupt bytes | hard-fail item/shard from that peer; refetch elsewhere; never failover-on-mismatch (R-D) |
| All candidates exhausted | item `failed` with reason; retry next cycle; pin shows failed count (no silent drop) |
| RS-band item, seam unbuilt | `failed: transport-unavailable` — visible, honest (§4.2) |
| Device offline mid-pull | queue pauses (peer-gated, no phantom counts); resumes on connectivity; pin state intact (airplane-mode test) |
| Un-pin mid-pull | queue drains the pin's lanes; revocation per §6.3; bytes GC per device policy |
| Storage pressure | size-budget refuse-at-resolve (§5.1); device pin creation MAY warn against free space |
| Gated content pin attempt | UI never offers it (commons-only v1); substrate validator refuses a `replicates-commons` payload with reach≠commons as defense-in-depth |
| Fresh-empty node claims caughtUp | structurally impossible: `total` requires resolution AND wants-diff requires inventory exchange first (R-A) |

---

## §11 Testing & a2o scenarios (story-first; tags inline)

Land with implementation, per slice:

1. **Pin on airplane mode** — declare offline, browse local, resume + notarize online (the
   hub-optional floor as a regression test).
2. **Byte-arrival parity** — pin caughtUp ⇒ filesystem set == resolved closure set (the
   inventory-vs-bytes lesson, pinned forever).
3. **Zero-link closure never false-completes** — `total: 0` distinct from caughtUp.
4. **Un-pin revokes** — subsequent provide event refused by bounds-validator.
5. **Corrupt shard refetches elsewhere, never serves** — R-D at shard granularity
   (with the striping follow-on).
6. **Dual-pin escalation consent** — dwelling steward accept/decline (with §7 implementation).
7. **K-of-N redundant fan-in** — ≥4 peers holding distinct shards, fetch with one peer down
   (striping follow-on; multi-node on `household-nodes`).
8. **Trust-weighted ordering** — `@requires:alpha-cluster-6peer` (live Standing signal).
9. **WAN cold-fetch across namespaces** — `@requires:alpha-cluster-6peer`.
10. **Beyond-alpha inventory scale** — `@requires:shem`.

Sweettest covers the `replicates-commons` mint + scorer arm (zome-sweettest-sync); `reconcile_rails`
gets unit coverage mirroring replication's; the queue's wait-for contract is exercised by
`wait-for-pull` in seeder tooling.

---

## §12 P2P design gate record

Gate run 2026-06-07 (mandatory, pre-design). Classifications as §3. Key adjudications:

- Pin = **operational-then-committed**; the notarized step is the announce EconomicEvent
  `bounded_by` the commitment (mirrors mark-published-vs-notarize independence).
- Desired-set = **the set of active pins** (no standalone desired-set entity); progress
  per-device operational; durable halves are Commitments.
- Closure = **derived (A2)**, rule-in-pin-body; membership never stored.
- Pull-queue state = **operational C** (evidence-determined across every surface).
- No new DHT entry types; Mishpat 11/~100 untouched; `ContentToSync` passes the projection
  triage only by gaining a real producer/consumer pair (§5.3).

---

## §13 Slicing

| Slice | Contents | Env |
|---|---|---|
| 1 | `reconcile_rails` extraction + acquisition stream + DevicePin + rungs 2–3 + `.pull` wire + `wait-for-pull` | household-nodes |
| 2 | content-scoped `replicates-commons` mint (closes dwelling-hub floor-check gap) + scorer arm (Medium tier) + sync-back (graduate-on-ProvideAnnounce) + rung 4 + revocation | household-nodes |
| 3 | closure resolver + rung 5 + `contentToSync` producer/consumer | household-nodes |
| follow-on spec | striping implementation on the §9 seam (`ShardRangeRequest`, per-shard verify, K-of-N) | household-nodes (≥4 peers) |
| follow-on slice | dwelling escalation UX (§7) on the consent surface | household-nodes |

Slice 1 is independently valuable (downloads + queue observability); each slice lands with its
scenarios (§11).

---

## §14 Captured follow-ups (NOT this spec)

- **Capability-by-hash vs blob-reach enforcement** — quarantined; gated pinning waits on it
  (§1.4). Tracking home: `epr-routing-complementary-captures.md`.
- **BLAKE3-vs-sha256 divergence** — tiered-quilt's RS-restitution intends BLAKE3; the
  implemented blob path is sha256 throughout. Reconcile in the striping follow-on before RS
  restitution lands (R-C).
- **bitswap acquisition driver** — serve-only today; if DAG-chunked content arrives, author the
  want-list driver as a complementary transport. Also the infrastructure-crates CODE-NO-DOC gap
  (MAP Gap Ledger) — `elohim-bitswap` needs its README/CLAUDE.md regardless.
- **Pin roaming via source-chain** — the Category-B canonical home for device pins, when
  cross-device pin sets are wanted (§1.1).
- **Prefetch-hint lane policy** — which hints auto-enter the lowest lane (tending-mediated,
  R-G) — needs the AttentionTending surface to exist first.
- **apps-sw striping consumer** — once the seam is built, the SW's sequential per-file walk can
  consume striped delivery for the extracted branch; requires the per-file manifest the SW
  currently lacks (deterministic-zip / Sprint-2 cache debt is the prerequisite).

---

## Appendix A — Decision log (operator-adjudicated, 2026-06-07)

1. **Depth cut**: queue + ladder normative; striping seamed (vision axis: household-coherence
   up, network breadth down; striping E2E is multi-node-gated anyway).
2. **Reach × pin**: commons-only pinning v1; download-for-self at any readable reach;
   capability-by-hash quarantined with a declared extension point.
3. **Pin action**: mint a commitment (5-step recipe) — AND the dual-pin reframe: device
   pin is the durable airplane-mode object; dwelling co-stewardship is a distinct consented
   tier; hubbiness is a dial. (Operator dimension that restructured D1/D2/D4.)
   **Reconciled 2026-06-08 (compose-lens):** the action is the canonical commons-tier
   `replicates-commons` (content-scoped payload variant), NOT a new `provide-content` verb — the
   dwelling-hub graduation gradient already reserves it; this spec's Slice 2 is that commons-tier
   follow-up. Graduation `proposed → active` happens on the first `ProvideAnnounce` EconomicEvent
   (D4 decided: act-of-providing = acceptance; a self-directed commons commitment has no
   counterparty). See §1.2, §6.1.
4. **Closure rule**: typed relation set + depth cap, bounds in pin body, concrete-count guard.
5. **Striping seam**: `ShardRangeRequest` on shard_protocol; bitswap complementary-later.
6. **Queue wire**: per-pin counts + rollup; unified vocab `{total, fetched, pending, failed,
   caughtUp}`.
7. **Controller**: Approach B — sibling acquisition stream with rails extracted to a shared
   module consumed by both streams.

## Appendix B — Evidence index (ground truth, verified 2026-06-07)

- Drain queue: `DrainStatusInfo{total,published,pending}` is a derived projection over
  `content.p2p_published_at` (`p2p/mod.rs:663-673`, `content_diesel.rs:870,894-909`); AIMD-paced
  `drain_publish_queue` (`mod.rs:3147-3286`); `wait-for-drain.ts:13-15,107-109` (tri-state +
  expectedMinTotal).
- Replication stream: `ReplicationState{pending,completed,failed,caught_up}`
  (`replication.rs:12-28`), retry-on-next-cycle battle-scar (`:135-141`),
  `MAX_REPLICATION_INFLIGHT=50`, round-robin GetContent (`mod.rs:6497-6538`); in-memory,
  rebuilt on restart; whole-inventory (`reach_filter: None`, `mod.rs:6470`).
- Scored parallel fan-in exists for whole blobs: `race_fetch` (`blob_fetch.rs:64-135`), sha256
  verify (`:258-267`).
- Shard primitives uncomposed: RS(N,K) encode/reconstruct (`sharding.rs:172-385`),
  `ShardManifest.shard_hashes` (`:30-71`), bands ≤16MB/16-64MB/>64MB (`:18-133`); wire is
  whole-content only (`blob_protocol.rs:56-63`, `shard_protocol.rs:24-44`);
  `HARD_MAX_RESPONSE_SIZE` 64MiB (`blob_protocol.rs:37`).
- Scorer gap: `replication_prioritizer.rs:63,103` keys exclusively on `replicates-dwelling`;
  provide rows only in `test_util.rs:111-123`; POST commitments default `state='proposed'`
  (`rea_commitments.rs:308`), active-set excludes proposed.
- Link surface: `contextMenuItems` injection + `epr-menu-select` (`epr-link.ts:99-112,256-289`);
  `ContextMenuItem{id,label,disabled?}` (`context-menu.ts:4-8`); popover footer hardcoded
  (`epr-popover.ts:420-442`); Loader verify default-on hard-fail (`loader.ts:74-76,92-97`).
- `GateHintRelation::ContentToSync` exists, zero producers/consumers (`projection.rs:104-106`,
  generated TS, seeder type-union).
- bitswap serve-only: zero get/sync callers in steward/node; progress events dropped
  (`coordinator.rs:137-138`).
