---
title: "Plane separation — who writes on a person's chain, and where everything else goes"
id: plane-separation-design
status: Draft
class: architecture
serves: zome-call-cost-bounded
date: 2026-10-03
context-tier: disclosed
steward: agent:integrator@claude-opus-5-5
graduation-trigger: the operator rules on the five decisions in §8; each accepted phase becomes a plan under plans/ and this note retires to history with the one-line lesson per plane
cites:
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/data/timeline/backlog/performance-concern-index.md
  - genesis/data/timeline/backlog/conductor-cap-grant-scan-per-zome-call.md
  - genesis/data/timeline/backlog/conductor-residual-cpu-full-chain-read-and-perpetual-republish.md
  - "shem-performance-deep-dive-report-20261002 | 2026-10-02-shem-performance-deep-dive-report | sha256:df11f4c346d960e2 | path: genesis/docs/superpowers/sprints/2026-10-02-shem-performance-deep-dive-report.md"
  - "campaign-1-4-leg2-device-authorization-20261003 | Campaign 1.4 leg 2 | sha256:a8b6df7ced5151b6 | path: genesis/docs/superpowers/sprints/2026-10-03-campaign-1.4-leg2-device-authorization.md"
  - "trust-as-efficiency-signal | Trust is an Efficiency Signal | sha256:40b8e3d166c935a7 | path: genesis/docs/content/elohim-protocol/architecture/trust-as-efficiency-signal.md"
  - "observation-event-layer-design | Observation/Event Layer | sha256:2b57787e60a0ddc6 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - elohim/holochain/.epr-meta/zome-call-cost-bounded.habit.md
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

# Plane separation — who writes on a person's chain, and where everything else goes

A design proposal for the operator to rule on. Nothing here is implemented, and no figure is new:
every number is copied from the deep-dive report, the backlog entries or the writer trace
(`genesis/local-dev/perf-deep-dive/background-writers-and-planes.md` on shem), all measured on a
disposable three-peer mesh whose actors are test fixtures. Nothing is measured on the campaign 1.4
household or on the fleet.

## 1. The one rule

**A person's source chain records what that person, or a delegate under their scoped grant, stands
behind. Everything a machine observes, repairs or nominates on its own account has a different
home.**

Today storage and the doorway sign as the steward's own agent on the steward's own cells, so
machine bookkeeping and human acts share one chain. Three measured costs follow from that one
fusion:

- **Growth.** An idle three-peer, three-doorway mesh adds 72 actions an hour to each peer's lamad
  chain and 120 an hour to its infrastructure chain with nobody using it.
- **Price.** At the old pin, authorising a call walked the caller's whole chain: 249–270 ms per
  lookup at 19,081 actions. Fork `ab31ecf2c` removes those two walks and is now on the fleet; the
  fleet reading is still owed.
- **Collision.** The chain-growth probe lost 14 of 4,000 writes to the doorway's health
  attestation, and the doorway lost four of its own to the probe. Every commit is strict-ordered,
  so a collision discards a whole zome call.

The price leg has a fix on the fleet. This design is about the other two, and about keeping the
price from coming back by another route.

## 2. The planes, each with its own shape

Vocabulary: `genesis/data/timeline/CONVENTIONS.md` §Plane. For each plane: what it carries, the
party that is honestly bound by a write, where it lives, how fresh a read must be, and what it
multiplies by.

| Plane | Carries | Party bound | Home | Freshness | Unit today → target |
|---|---|---|---|---|---|
| **Person's notary** | publish, version, head declaration, delegation, ceremony attestations | the person | the person's lamad chain | notarized | per human act (unchanged) |
| **Observation** | "X saw Y up/degraded at T"; storage lifecycle heartbeat | the observing node | storage's observation substrate, no chain entry | latest wins, minutes | per round per peer → per status change |
| **Witness summary** | a sampled, signed digest of the observation plane, for when someone must prove a node lied | the observing node | node-registry cell (`HealthAttestation` already exists) | daily, or on dispute | new; one per node per day at most |
| **Authority** | "this key may call these functions on this cell" | the person, granting to a process or device | the cell it authorises (Holochain requires it) | every call | per connect → per relationship |
| **Head, machine side** | "this node nominates head H for id X" | the node | lamad DHT link (must stay there to be a candidate); author moves from the person to a delegated node agent | current | per (id, head) per process → per (id, head) ever |
| **Notary machinery** | publish, receipts, gossip | the conductor | conductor | background | per authored history → per new op (option E, built) |
| **Projection** | SQL rows, reconcile, render | the node | SQLite | ephemeral | owed: mechanical |
| **Bytes** | blobs, shards | nobody; self-certifying | blob store | on arrival | must never wait on head or notary |

Three of these change. The rest are listed so the boundary is visible.

## 3. Observation plane — the largest idle writer leaves the chain

**Paths.** Doorway peer-health probe: every 300 s, per live sibling, `record_health_attestation`
bridges to `issue_attestation` on the lamad cell (1 Create + 2 links). Storage heartbeat: every
60 s, `record_peer_status` on the infrastructure cell (1 Create + 1 link).

**Gradient reading.** A doorway and its own node are one steward (`trustful-self`); sibling
doorways of one household are in a standing declared relationship (`trustful-declared`). Both are
paying the notary's full price (`plane-custody` charged on `plane-notary`, `fused-planes`,
`unit-history`, `lane-borrowed`) for a fact that is superseded five minutes later. Nothing in the
code was found reading the device-health history; `get_doorway_attestations` returns an empty
list today. The trust that already holds can carry this: the compression is one statement per
*change* in place of one per *round*.

**Design, in three steps that each stand alone:**

1. **Write on transition only.** The doorway keeps its last recorded status per sibling and calls
   the extern only when the status differs. Same for the storage heartbeat: lifecycle changes and
   pool-flag changes, not a timer. Doorway-only and storage-only; no DNA change. Expected idle
   rate on both cells: zero.
2. **Bound the silence.** A missing record must not read as "healthy forever". Each observer
   writes one summary per subject per day whatever happened. This is the honest-absence answer
   (concern C4) and it is what makes step 1 a compression and not a skipped ceremony.
3. **Move the live reading to the observation substrate.** The retired doorway self-heartbeat
   already took this route (observation-event-layer spec, stages 4 and 7, not yet wired in the
   doorway). The daily summary then moves to node-registry's existing `HealthAttestation`, by
   retargeting the infrastructure bridge. That is coordinator-only **if** the existing shape is
   accepted as it stands; changing the shape moves the node-registry DNA hash.

**What is given up.** An immutable peer-signed record of every five-minute observation. What is
kept: every status change and one daily summary per observer per subject.

**What separation costs.** The release soak attestation deliberately rides the device-health kind
(`release_attestation.rs`, `RIDDEN_ATTESTATION_KIND`) and is evidence other nodes act on. It must
keep working at step 1 and be re-homed with care at step 3; it is rare, so it is not a cost driver.
Node-registry's integrity zome has no validation yet for `HealthAttestation` (a TODO in the code),
so a witness placed there today is weaker than one on the lamad DNA. Step 3 should not land before
that validation exists.

**Who could misuse it.** A public, immutable log of which doorway saw which, every five minutes,
is participation metadata an outside aggregator can join. Taking the per-round record off the DHT
shrinks that surface; the daily summary is the residue and should carry status only.

## 4. Authority plane — one credential per relationship

**Paths.** A process and its own conductor: storage, doorway, the hosting chaperone, a2o scripts.

**Reading.** `trustful-self` paying `friction-verify` on every call. Two separate costs have been
measured and should not be conflated. The lookup's price by chain length is the fork change now on
the fleet. The *count* is still open: the leg-2 handoff reports about 58,000 grants on Adam's
conductor-0 (11,776 on the lamad cell), a `list_capability_grants` that takes about 8 s, and a
chaperone that issues a grant per role cell per connect. Each grant is also a chain action, so the
count feeds growth.

**Design.**

- One live grant per (grantee signing key, cell, function set). A connect looks for a live grant
  before minting; storage already does this while its closed-chain fence is armed.
- The two storage paths that still mint directly when the fence is absent (`signing.rs`,
  `hc_client.rs`) go through the same lookup, or the fence becomes unconditional.
- The chaperone's `grant_memory` is meant to make this idempotent. The first task is to find out
  why the pile grows anyway; that is a reading task, not a design one.
- Rotation is by expiry with revoke-on-supersede, so the live count is bounded by relationships,
  not by connects. Revocation and scope checks are unchanged: this compresses issuance, it does
  not weaken what a grant permits.

Holochain keeps grants on the cell they authorise, so placement cannot change. The cure is count.

## 5. Head plane, machine side — the node nominates under its own name

**Paths.** Storage's sweep and sync-triggered adoption write `declare_canonical_content_head` (one
link) on the person's lamad chain when heads disagree.

**Reading.** The link must stay in the lamad DHT to be an election candidate, so it cannot move to
another cell. But the party nominating is the node. The campaign 1.4 ruling already names the
shape: the operator's runtime authors roots, and devices act under scoped delegation. A node
nominating a head is that same relationship (`trustful-earned`: a delegate under a scoped grant).

**Design.**

- **Now, storage-only:** persist the per-process candidacy ledger so a restart does not re-mint
  one link per divergent id. The trace reads the declare as an unconditional `create_link`; that
  needs confirming before this is sized.
- **Now, storage-only:** in the adoption trigger, a sync-apply offer for a person's publication
  does not queue behind retained attestation hints. This is the report's leading (unproven) cause
  of the one missed 75-second deadline.
- **Later, with the native-channel work:** the node's nominations are authored by a node agent
  under a steward delegation, using the delegation path `declare_earned_canonical_head` already
  has. Whether link validation pins the author is not verified; if it does, this is an integrity
  change and moves the lamad DNA hash.

Missing node, in mintable shape: `chain: machine head nomination / between "storage decides to
nominate" → "election counts the nomination" / missing node: a node agent, distinct from the
steward's agent, holding a scoped delegation / state: not traced whether one exists in the
installed hApp.`

## 6. What is owed whatever the relationship

These are mechanical. Trust does not make them cheaper and they should not wait for it.

- **Collision handling.** Storage's head-moved retry has a flat 2 s budget, which becomes a single
  attempt once a write costs about a second. The doorway does not retry at all, and a bridge
  crosses cells so storage's write gate cannot serialise it. Sections 3–5 remove most colliders;
  the budget should still follow the observed round-trip time.
- **Writer attribution.** No action carries a writer marker, so a census can split a chain by
  shape only and cannot tell a human publish from a machine adoption. A per-writer commit counter
  exported by storage and the doorway is enough; it does not belong on the chain.
- **Publish loop.** Option E (`cd568819e`, publish only new ops between sweeps) is built and
  approved, and pins separately now that the grant change is on the fleet.
- **Storage `database is locked`.** The `BEGIN IMMEDIATE` design exists and is unrouted.
- **Conductor memory per hosted agent** (about 700 MB). Composition unknown; a plan exists on shem.

## 7. Measurements, in the order they pay off

Each is one bounded reading that could disprove the step it gates. Capture and report use the
`runtime-performance` tooling; nothing here authorises a capture on the fleet.

| # | Question | Reading | Gates |
|---|---|---|---|
| 1 | What share of a long real chain did a machine write? | The census in the writer trace §5, on a **stopped** copy of the campaign household store | whether §3 is the main lever there, or head adoption is |
| 2 | Did the fork change cut per-call cost on the fleet? | No-op call latency on a long-chain fleet agent, compared with the same reading at the old pin if one exists | the `zome-call-cost-bounded` habit; nothing in this design |
| 3 | Does transition-only writing reach zero? | Actions per hour per cell on an idle household, before and after §3 step 1 | `idle-is-free`; expected 72 → 0 and 120 → 0 |
| 4 | Do collisions stop? | Head-moved lines in doorway and storage logs over one seeded household run | §6 retry work: needed or not |
| 5 | Is the grant count bounded? | Grants per cell after N connects, before and after §4 | §4 |
| 6 | What is the remaining idle wasm CPU? | Per-function counters or a wasm symbol map; current profiles cannot separate validation from coordinator calls | whether the heartbeat is also a validation cost on every peer |

Reading 1 comes first because it is read-only, needs no build, and decides the order of
everything else.

## 8. Decisions that are the operator's

1. Whether device-health observations leave the person's chain, and how far: transition-only
   (step 1), plus a daily summary (step 2), plus the move to the observation substrate (step 3).
   **Recommended: steps 1 and 2 now, step 3 after node-registry validation exists.**
2. Whether the per-round witness history may be given up for change records and a daily summary.
3. Whether the storage heartbeat follows the same rule.
4. Whether the adoption trigger gives a person's publication priority over retained attestation
   hints.
5. Whether a node agent under steward delegation is the target for machine nominations, to be
   designed with the native-channel campaign.

## 9. Design-gate record

| Entity | Class | Type | DNA-hash | Head-plane cost | Address |
|---|---|---|---|---|---|
| Device-health observation | Ephemeral | none; observation substrate | neutral | none | none; latest per (observer, subject) |
| Health witness summary | Attested-private: the observations stay local, the summary is notarized | reuse node-registry `HealthAttestation` | neutral if the shape is kept | one per observer per subject per day; a household of three is single digits a day | agent-scoped composite (observer, subject, day) |
| Storage peer status | Ephemeral between transitions | existing `PeerStatus`, written on change | neutral | falls with the write rate | existing |
| Signing credential | chain entry by construction | CapGrant | neutral | none | conductor-minted |
| Machine head nomination | Linked | existing tagged `IdToContent` link | neutral now; possibly moving if the author changes | one link per (id, head), no new head | action hash of the link |

Stakes: observations and summaries are priceable by declared stage. Delegations, constitutional
attestations and counter-evidence are floor-protected and nothing here touches how they are
verified. No new HTTP route is proposed. Identity framing: a node agent acts under the steward's
standing through a scoped grant; it is not a more autonomous tier.

Not verified, and each could change a step: whether anything reads device-health attestations
from the DHT (a grep-level negative only); whether `declare_canonical_head_inner` deduplicates;
whether link validation pins the nominating author; whether a node agent exists in the hApp; the
campaign household's real writer mix.
