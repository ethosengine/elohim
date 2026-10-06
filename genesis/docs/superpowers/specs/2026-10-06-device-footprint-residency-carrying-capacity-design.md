---
title: What a device holds, why, and how much — residency, holds and carrying capacity for code and bytes
id: device-footprint-residency-carrying-capacity-design
status: Draft
class: architecture
serves: release-retention
date: 2026-10-06
context-tier: disclosed
steward: human:matthew
graduation-trigger: on the household mesh a peer answers, for every byte in every store it keeps, which named reason holds it; bytes that nothing names are let go without a ledger backfill; a commitment that would exceed its band is refused by name before any shard moves; and the own-node view shows the own band as free space with the set-aside already deducted
cites:
  - "storage-physics-benchmark | the measured fleet footprint and the code-custody rulings (ten-release hub window, current plus one either side on a personal device, nothing running is deleted) this design takes as already decided | sha256:34eb0e7b16efa51a | path: genesis/docs/content/elohim-protocol/architecture/2026-10-05-storage-physics-benchmark.md"
  - "compute-envelope-tevah | the compute envelope whose declared disk bound, graded bands and shed order this design gives numbers and an admission rule; ark is named here as the canonical home of the quota | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "workspace-berth-carrying-capacity-design | the prior statement of carrying capacity and its viable-system placement for a workspace berth, which this design applies to a device holding code and bytes | sha256:f7f53bb9e7686d19 | path: genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md"
  - "mutual-storage-replication-dwelling-hub-design | source of the constitutional donut bands (own, dwelling, collective, commons, with floors and ceilings) that this design turns into day-one reservations enforced at admission | sha256:3ef54b7a3bec9f17 | path: genesis/docs/superpowers/specs/2026-05-28-mutual-storage-replication-dwelling-hub-design.md"
  - "seamless-groupware-ux-addendum | the install ladder (on this device or on my hub, gated by a budget with no numbers yet) that this design supplies the numbers for | sha256:a9698f7152f60430 | path: genesis/docs/superpowers/specs/2026-09-08-seamless-groupware-ux-addendum.md"
  - "epr-app-delivery-verdict-layered-fallback-design | the peer-judged verdict on whether an app version starts, named here as the first module of the hub-side app host in a later slice | sha256:cef986af6515e913 | path: genesis/docs/superpowers/specs/2026-09-05-epr-app-delivery-verdict-layered-fallback-design.md"
  - "elohim-seam-map-concern-routing | the concern-routing atlas this design follows to place each piece: storage for bytes and holds, the hub for serving by key, ark for the quota, the doorway for clients that cannot reach a peer by key | sha256:7ea7563016174974 | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "elohim-hub-boundaries-design | the hub, node and storage boundaries this design keeps: storage stays oblivious to hub composition, and the hub-side app host is a separate slice from the storage-side holds work | sha256:233cb996edd7c366 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md"
  - genesis/plans/2026-04-13-device-archetypes-design.md
  - elohim/elohim-hub/README.md
  - elohim/elohim-storage/.epr-meta/release-retention.habit.md
  - elohim/elohim-storage/src/services/release_adoption/retention.rs
  - elohim/elohim-storage/src/services/peer_capacity_service.rs
  - elohim/elohim-storage/src/services/constitutional_ratio_registry.rs
  - elohim/ark/core/src/manifest.rs
  - genesis/a2o/features/delivery/release-retention.feature
---

# What a device holds, why, and how much

This is a design, not a build order. It states where the footprint of code and bytes on a device is
decided, and the code is expected to move toward it. What the code has not caught up with is listed
in "Missing nodes" and "Slices", never in the rules.

The problem it answers: a peer's disk only grows. Every release of an app adds files to the peers
that serve it; nothing reads a byte limit; and a peer cannot say why it holds most of what it holds.
The larger aim behind it is the diversity-of-peers epic: a phone, a Chromebook and a home rack should
each carry a footprint that fits them, with the household and the community holding what a small
device cannot, and with no part of that depending on a doorway or a DNS name.

Terms this leans on. A *release* is one published entry naming the exact bundles of every app it
carries; a *release channel* is the named feed releases are published on, and its *head* is the
release the channel currently elects (`genesis/a2o/features/delivery/release-retention.feature`
defines these for a reader). A *device pin* is a person's local declaration "keep this on my device"
(`/api/v1/pins`, table `acquisition_pins`). A *commitment* is a notarized promise in the REA sense:
here, a promise to hold or run something for someone else. The *envelope* is the compute-envelope
spec's bound on what a device's processes may use (`ark`, tevah). *Reach* is how far a thing may
travel, from private to commons, and *standing* is a person's earned, relational record of trust;
both are defined in `genesis/docs/content/elohim-protocol/glossary.md`. A *holon* is a collective at
any scale: a household, a community, a network of them. *Hub*, *doorway* and the web seam are as
`elohim/elohim-hub/README.md` states them.

## 1. What is already ruled and is not redesigned here

- **Code custody** (storage-physics benchmark §7–8, operator rulings of 2026-10-05). Hubs hold a
  ten-release window. A personal device keeps the current release and one either side. Nothing a
  peer is running is ever deleted. Which releases a hub holds for others is a brit and rakia concern.
- **The device pin** is built: declarable with no network, satisfied only when bytes arrive. Hub
  backing of a pin is designed and deferred.
- **The install ladder** (groupware UX addendum §3): "on this device" or "on my hub", gated by a
  budget that has no numbers yet. This spec supplies the numbers.
- **The envelope** (tevah): `RuntimeManifest.envelope.bound.disk_bytes`, graded bands, a shed
  order, and `ArtifactRef::Channel | Pinned`. Declared; not enforced.
- **The web seam** (hub README): serving an app's files and rendering by key is hub-side work, the
  doorway is only for clients that cannot address a peer by key, and storage is bytes, heads,
  custody and reach.

## 2. Three acts that are never the same act

On a peer today, following a channel, holding its bytes and promising to hold them for others are
one fused act. The rule is that they are three:

| Act | What it is | Cost |
|---|---|---|
| **Follow** | Know the channel's elected head | Small. This is where community stewardship of code lives |
| **Reside** | Hold bytes on this device | Disk on this device, decided by this device's person |
| **Carry** | A commitment to hold or run for others | Disk or compute set aside in advance, per commitment |

A device may follow a channel and hold none of its bytes. A device may hold bytes it has promised to
nobody. A hub carries a channel's window with **one commitment per hub per channel**, not one per
bundle blob per release.

## 3. Residency is a state on the device pin

No new entity. Residency is device state and is never sent anywhere. A pin is identified by its
agent, the head it names and its closure kind (one item or a cluster); the state lives in the pin's
`status`, which gains *archived* beside the existing active, paused, removed and retired.

| What the person sees | Shape |
|---|---|
| Kept offline | A device pin on the app's head with the release's declared closure. Never let go automatically |
| In use (the default) | Current release and one either side, following adoption |
| Archived | Bytes let go. The composition record and the person's data stay. One act restores it |
| Opened from my hub | No pin and no bytes here. The app runs against a hub's runtime (§7) |
| Published to the web | Not residency. A doorway operator's own projection |

For "kept offline" to mean anything, a release must declare its *closure*: every blob it needs to
start. Sizes are already declared; closure is the addition.

## 4. Holds: every byte has a named reason, and the peer can read its own answer

**The rule: for every byte in every store a peer keeps, the peer can name why it is held. Bytes a
peer brought here by its own act, and that nothing names any more, are its own to let go. Bytes
another peer placed here are that peer's to withdraw. Bytes with no record of how they arrived are
reported and never cleaned.**

The retention pass already answers a narrow form of this. It looks only at app-bundle releases the
release ledger names and that are past the window; it never lets go the newest release of a channel
or one in use; and of each remaining blob it asks three questions: does a kept release name it, does
a content row serve it, does a live `custody-blob` commitment oblige it. Bytes the ledger does not
name are outside it entirely. The rule generalizes that to one resolver over every byte store and
every artifact class, with these reasons:

| Reason | Meaning | May it be let go? |
|---|---|---|
| running | A process on this device is executing it | Never |
| pinned | A device pin's closure names it. Today an item pin holds through the content row it names, so it reads as served | Never automatically |
| served | A content row on this peer names it | Never |
| kept-release | A release inside the window names it | When it leaves the window |
| carried | A live commitment obliges this peer to hold it | Only through the commitment |
| arriving | It is in transfer, or arrived and its record is not yet written | Not until it is named or the transfer is abandoned |
| cache | Rebuildable from bytes held elsewhere on this device | At once, under pressure |
| runtime | The peer's own databases, indexes and keys | Never by this pass |
| placed | Another peer placed it here, and nothing above names it | When the peer that placed it withdraws it |
| own-unnamed | This peer brought it here itself, and nothing above names it | Yes (below) |
| unrecorded | Nothing above names it and no record says how it arrived | Never by this pass. Reported |

**Every store is in the answer.** The content-addressed stores are the blob store with its chunk and
shard files, the iroh store, the extraction cache, release staging and compute payloads; the
resolver names a reason per byte there. The peer's databases, indexes and keys (the content
database and its write-ahead log, the graph and sync stores, the search index, coordinator bundles,
the closed-chain fence, the identity keys) are reported by size under *runtime*. A store the resolver
does not know is itself reported, as unknown, so a new store cannot grow unseen.

**Every arrival is recorded.** "Nothing names it" is only a safe reason to let a byte go when the
peer knows how the byte got here. So each arrival writes a record before or with the bytes: how it
arrived (this peer put it, this peer fetched it, an adoption pulled it, another peer placed it),
who placed it when someone else did, and what it arrived for when that is known (a release, a
commitment). Put, fetch and adoption are the peer's *own acts*: it can always fetch again, so once
nothing names such a byte it is the peer's to let go. A byte another peer placed is held until that
peer withdraws it. A byte with no arrival record is never let go by this pass, however long it has
been unnamed. Eligibility is judged per byte, not per store, so cleaning is automatic from the
first pass for every byte that has a record, with no report-only phase. A blob too large for one
file is held as shards under a manifest; it is judged, and let go, as the one blob it is. Bringing
the same bytes here again is a new arrival: the wait starts over.

Three consequences.

**The peer's own view.** The answer is exposed on the peer itself, own-node only: bytes and counts
per reason and per store, against capacity. It is how a peer manages itself, and the same surface is
how a test fixture peer is understood and asserted on. What a device holds reveals what its person
uses, so this view never leaves the device; a hub sees only what it carries, and a doorway sees only
what it serves. It therefore lives on the node-local surface. The existing capacity read model
(`PeerCapacityView`) is served by peer id with no owner check, and nothing calls it; holds do not go
on that route as it stands.

**Autoclean.** A byte that reads as own-unnamed is let go automatically. The safeguards are that it
must read so on two consecutive passes and be older than 24 hours, the age counted from the arrival
record and not from the file's modification time, and that the peer asks again, immediately before
deleting, whether anything names it. That last look and the delete are closed to arrivals: a put
or fetch of the same bytes either lands before, and stops the delete, or lands after, and stores
them again. A process that has just started lets nothing go until its own second pass, and its
first pass waits a full default interval after boot. Both are declared defaults, reloadable. The pass is periodic (the
retention pass runs every 300 seconds today), and a story shortens the age the way the retention
story shortens the window. Letting go removes the byte from every place the peer keeps it, files
first and rows after, as the retention pass already does, and removes only this peer's own records
of holding it.

**The backlog.** This is what reclaims the bundles that predate the release ledger, with no
backfill. A bundle the peer put or fetched itself carries a record of that (the put path has
written one since 2026-05-10, and every fetch writes one), so it is let go as own-unnamed. An
older bundle with no record stays, reported as unrecorded, until a person names it.

**Withdrawal.** A peer that placed bytes with others tells each holder, directly and as itself,
that it no longer needs them held; the holder accepts that only from the peer its record names.
The custody announcement on the gossip topic is not that message: it is unsigned, receivers ignore
its status, and a peer one build behind would read a release as a fresh claim.

## 5. Carrying capacity

**One byte number per device, derived, with a configurable override.** Bytes are the limiter;
counts are shown because they inform.

**Where the number comes from.** The authority for what a kind of device is, is its archetype atom:
`genesis/data/devices/<id>.md`, the story and the hardware facts together (`devices.json` is
generated from the atoms and is not an authority). Code couples to the atoms by derivation, not by
looking them up at run time: one function takes hardware facts and returns the envelope (capacity,
bands, sync budget). A story or a simulated fleet peer feeds it an archetype's declared facts; a
real device feeds it observed facts. Its home is `ark-core`, which already owns the quota types and
which storage already depends on. The notarized four-value `DeviceArchetype`
(`node | desktop | mobile | steward`) is a different axis, names which shell presents a binding, and
is not touched.

**The denominator** is the device's carrying capacity after the runtime's own floor: what §4
reports as *runtime*, plus headroom. It is not raw disk. The floor's number is not set here (§12);
Slice 1 does not need it.

**Bands are reservations, made on day one.** The constitutional donut is the protocol's existing
division of a device's storage into four bands, with floors and ceilings fixed as DNA constants and
a person's own choice between them. It divides the capacity, and each band's share is set aside for
that purpose from the start:

| Band | Default | Floor / ceiling | What code it holds |
|---|---|---|---|
| Own (shown as "free") | 15% | 5 / 70 | My pinned apps and in-use releases |
| Dwelling | 40% | 10 / 80 | A hub's release window for the household |
| Collective | 25% | whatever the other three leave | The window for a holon the device's steward has standing in |
| Commons | 20% | 10 / 60 | The protocol's own channels |

- The four shares always sum to 100%. A person sets own, dwelling and commons within their floors
  and ceilings, and collective is what remains; a setting that leaves collective below zero is
  refused.
- The personal figure, "X of Y free", is the own band only. What is promised to others is already
  deducted before the person sees a number.
- **Admission at authoring.** A new commitment is checked against its band's remaining bytes before
  any shard moves.
- **Admission at arrival.** An arriving shard is charged to the commitment that names it. A shard no
  commitment names is refused.
- **Idle reserved space** may hold cache and nothing else, so it is reclaimable at once.
- **A full band** refuses new commitments in that band only, by name, and signals the steward. No
  band can evict another. Pledges are renegotiated through the commitment, never dropped.
- **A device that cannot steward** (capability level 0 to 2) gets its whole capacity as own. That is
  capability, not opting out.
- **Under pressure**, in order: shed cache; shrink the in-use window to its floor and offer
  archives; refuse new carrying. Running, pinned and carried bytes are never shed.

In shefa, a person drills from band, to commitment, to promised against actually stored. That is
the lens on what their device supplies to the commons.

## 6. Where bytes come from, in order

This device; the person's other devices; the household hub; hubs of a holon with standing; a pool;
and a doorway last and never required. Each is addressed by key and content address, never by DNS
name. Losing the doorway costs web reach and nothing else.

## 7. Small devices

- **A browser-only device** needs some HTTPS origin for its first load. Any doorway will do and it
  is swappable. After that the service worker (`app/elohim-app/src/apps-sw.ts`), which already
  fetches an archive, verifies it against its address and caches it, takes a **list of delivery
  peers**, and a doorway is one entry on that list. A Chromebook also gets the peer-capable shell,
  which is the lane with no doorway at all.
- **A thin client running an app it does not hold** opens it against a hub's runtime. The hub then
  holds that person's key for that cell, exactly as a doorway does for a hosted human, and the
  person is told so. A device-held key with a remote conductor is a missing node (§10).

## 8. Who owns what

| Piece | Role |
|---|---|
| rakia / brit | What a release is, its closure and sizes, the window and who holds it |
| `elohim-storage` | Carries bytes, answers why each is held, enforces capacity until `ark` does |
| `elohim-hub` | Serves a resident app's files and renders, by key; judges whether a version starts |
| `ark` / berth | Canonical home of the quota. Storage's enforcement retires when `ark`'s is real |
| eprfs | Materializes bytes as files. No pin and no collection of its own, and it should not grow one |
| doorway | Names, TLS, host routing and an edge cache, for clients that cannot reach a peer by key |
| shefa (app) | The person's view: bands, commitments, promised against stored |

In viable-system terms: running apps are System 1; admission with refusal by name is System 2; the
bargain between own, carried and cache is System 3; the holds view is the audit channel (3*); disk
pressure is the algedonic signal; the person and the elected head are Systems 4 and 5. The same
shape recurs at household, holon and network.

## 9. Design gate

No new DHT entry type. No integrity-zome change in Slices 1 and 2; Slice 3 has one open question
that could add one (§12).

| Entity | Class | Identity | Note |
|---|---|---|---|
| Release and channel head | Notarized, existing `Content` | CID; which head applies is declared | Adds a declared closure. One head per channel however many releases a year |
| Residency | Private, `acquisition_pins` | (agent, head, closure kind) | Adds an archived value of `status`. A local table change, not a DHT change; the archived state must be fenced from the path that re-admits pins named by inventory |
| Hub carry | Notarized, existing Commitment | Commitment CID | Per hub per channel. A bare new action string needs no integrity change: the commitment's action list is open. Per-action required fields would be a mishpat integrity change |
| Arrival record | Ephemeral, local, operational | (content address, how it arrived, who placed it) | How a byte got here and what for. Losing it is fail-safe: the byte reads as unrecorded and is kept |
| Withdrawal | A direct peer message, not stored | (content address), from the placing peer | Appended to the shard protocol; a peer one build behind fails to decode it and nothing is deleted |
| Capacity | A manifest declaration; the effective value is Ephemeral | n/a | A peer advertises bands, not exact bytes |
| Holds view, shed events | Ephemeral | n/a | Own-node only. Rebuilt by re-running the resolver |

- **Head-plane cost.** No per-item heads are added. Carry moves from one commitment per blob per
  release to one per hub per channel: tens of commitments at one year, with no change to quiesce.
- **Back-fill check.** No coordinator function or route takes a new hash.
- **Routes.** No peer-facing route. The holds view is node-local; whether it extends the existing
  node-local adoption report or is a sibling of it is the slice's choice.
- **Decision predicates.** This design births three: *hold* (why is this byte held), *admission*
  (may this commitment or shard be accepted) and *shed* (what goes first under pressure). Each owes
  the concern-canon answer (the design gate's fixed list of recurring failure classes, answered one
  by one: `.claude/skills/p2p-design-gate/SKILL.md`, Step 4) and a row in the crate's
  `seam-registry.yaml` in the slice that births it. Those answers are not written yet and are part of each slice's plan, not of this document.
- **What holding reveals.** A device's holds reveal its person's use, so they stay on the device.

## 10. Missing nodes

Each in the chain's own shape: between two named atoms, the assertion and its probe, and the current
state.

- **Pin to retention.** Between a device pin and the retention pass: *a pin is a hold.* Probe: pin a
  release that is past the window and it survives a pass. Absent today; the pass does not read pins.
- **Pin to bytes.** Between a pin and the blobs it holds: *a pin's closure resolves to a set of
  content addresses.* A pin names a head and a closure rule, and no table maps a pin to blobs, so
  the resolver has to expand the closure. Probe: the view lists the blobs a pin holds. Absent.
- **Arrival to record.** Between bytes arriving and the resolver: *every arrival records how it
  got here.* Probe: a shard pushed by a peer shows as placed, with the peer that placed it. Absent
  on several paths: an inbound push stores bytes and writes no row (and stores before it checks
  the hash), a direct shard put writes none, and an adoption pull is recorded only when the whole
  release completes.
- **Placement to withdrawal.** Between a peer letting a blob go and the peers it placed copies
  with: *the placing peer withdraws, and the holder then lets go.* Probe: after the origin lets a
  blob go, a holder it placed a copy with no longer holds it, and a holder of a copy that was not
  withdrawn still does. Absent. Shard distribution selects only the local peer today, so no such
  copies are known to exist; the node is built when the holds view shows placed bytes.
- **Sha256 to iroh copy.** Between a blob and its copy in the iroh store: *the alias between the two
  addresses exists for every blob.* It is written only on the seeder's put path, so a blob that
  arrived by fetch or push has an iroh copy nothing can map back, and letting the blob go leaves that
  copy behind. Probe: after a blob is let go, neither store holds it.
- **Archetype to capacity.** Between an archetype atom and a peer's effective capacity: *the peer's
  capacity is derived from the facts the atom declares.* Probe: a fixture peer cast as an archetype
  reports that archetype's capacity. Absent; `max_storage_bytes` has no reader. Read from code
  earlier in this design and not re-verified: deployed pods fall back to a default because the id is
  stripped before the lookup.
- **Commitment to stored bytes.** Between a carry commitment and an arriving shard: *a shard is
  charged to the commitment that names it.* Probe: a shard no commitment names is refused. Absent;
  an inbound shard is stored after a hash check and nothing else.
- **Thin client to hub runtime.** *A device-held key drives a cell on a remote conductor.* Absent.
- **Browser to hub by key.** *A browser fetches app bytes from a peer with no doorway in the path.*
  Absent, and whether it is feasible is unverified.
- **Conductor volume.** The benchmark puts a fleet peer at 6.0 to 6.6 GB of conductor volume against
  0.3 GB of storage, most of it compiled wasm. Nothing in §4 reaches it: the conductor's databases do
  not shrink when rows are freed. It is the largest runaway and has no bound in this design beyond
  being named in Slice 5.

## 11. Slices, in order

Each is a lane with its own gate and its own proof. Only the first is ready to plan.

1. **Holds and autoclean** (`elohim-storage`). In this order inside the slice: the resolver of §4
   and the own-node view; a record on each arrival path; then letting go of own-unnamed bytes. Serves the red
   `release-retention` habit and is its stated first move: it reclaims what predates the ledger
   without a backfill. Story: a scenario beside `release-retention.feature` under the same
   `@concern:release-retention`, in which a household peer holding bundles no ledger row names
   reports them as unnamed and, once they have been unnamed on two passes and are past the age, no
   longer holds them in any store, while everything it serves is still served and a blob with no
   arrival record is still held. Closes two missing nodes, arrival to record and sha256 to iroh
   copy; an item pin already holds through its content row, and cluster pins are not built.
2. **Capacity and admission.** The derivation function in `ark-core`, bands as reservations,
   admission at authoring and at arrival, and the shed order. Needs its own habit, born with the
   exit condition "when `ark`'s quota verb enforces the envelope".
3. **Carry per channel.** One commitment per hub per channel, replacing the per-blob pledge.
4. **The app host on the hub side.** The piece that creates the `elohim-hub` crate; its first module
   is the deliverability verdict, moved from where the hub README's debt table lists it. Then the
   delivery-peer list in the service worker.
5. **Conductor volumes.** Rewriting the databases so freed wasm shrinks the file, and bounding
   write-ahead logs and snapshots. This is where the gigabytes are.

Slices 1 and 4 do not depend on each other. Slice 1 is first because it moves a red habit with
proof on the household mesh and needs no new crate.

## 12. Not decided here

- **Bundles with no arrival record.** On the publishing peer, bundles put before 2026-05-10 carry
  no mark and stay reported. Whether a person names them once, by hand, is the operator's call
  after the first fleet read.
- **What the unidentified files on quiet fleet peers are.** About forty on each. The code suggests
  each peer fetched them itself when an app record named them, which would make them own-unnamed
  and let go by Slice 1; it does not suggest they were pushed. Neither has been observed. The holds
  view on the first deploy answers it.
- Ruled 2026-10-06: a pushed shard is named by a placement record, and let go on the placing
  peer's withdrawal. For a shard pushed before records existed, whether a withdrawal from a peer
  that claims to have placed it may stand in for the missing record is decided when the withdrawal
  message is built.
- Whether channel-level carry needs per-action required fields. A bare action string does not move a
  DNA hash; required fields would move mishpat's, and Slice 3 must say so if it adds them.
- How much consecutive releases share per file. The same bytes are held up to three times today
  (blob store, iroh store, extraction cache); deduplicating them is worth designing only after that
  is measured.
- The numbers for the runtime floor that comes off raw disk before the bands are cut.
