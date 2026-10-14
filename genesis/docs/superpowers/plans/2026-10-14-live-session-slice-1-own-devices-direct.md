---
title: Live session, slice 1 — two of one person's machines, direct, presence only
id: live-session-slice-1-own-devices-direct
status: Draft
class: process-meta
context-tier: disclosed
steward: human:matthew
date: 2026-10-14
serves: dataplane-convergence
graduation-trigger: >
  The story genesis/a2o/features/dataplane/live-session-own-devices.feature passes on the household
  mesh with its @wip tag removed, and household-at-rest reads the same conductor request counts with
  the live-session protocol registered as it does without it.
cites:
  - "live-session-plane-design | the design this slice builds the first part of, as committed at ebcaa0847; section 3 lists where the slice departs from it | sha256:d1adbf86e46fd487 | path: genesis/docs/superpowers/specs/2026-10-14-live-session-plane-design.md"
  - "working-version-sdk-standard-design | owns the working version, its circle and the unlanded manifest block this slice deliberately does not add to | sha256:66ba00931d6dd97e | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - "plane-separation-design | the rule that a trust act is made once per relationship and carried, which the per-machine grant follows | sha256:657eb48f9c7a4db0 | path: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md"
  - "rea-compute-commitment-primitive | defines the delegates-compute commitment the slice uses as its admission grant | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md"
  - genesis/a2o/features/dataplane/live-session-own-devices.feature
  - genesis/a2o/features/dataplane/household-at-rest.feature
  - genesis/a2o/features/auth/device-consent-grant.feature
  - genesis/a2o/features/federation/local-first-shared-sheet.feature
  - elohim/elohim-hub/README.md
  - elohim/elohim-storage/src/main.rs
  - elohim/elohim-storage/src/p2p_iroh/node.rs
  - elohim/elohim-storage/src/p2p_iroh/codec.rs
  - elohim/elohim-storage/src/services/operation_authorization.rs
  - elohim/elohim-storage/src/db/mishpat_commitments.rs
  - elohim/elohim-storage/src/api/compute_grants.rs
  - elohim/elohim-storage/src/services/device_carrier.rs
  - elohim/elohim-storage/src/http.rs
  - crates/consent-grant/src/carrier.rs
  - elohim/sdk/working-version.seam.json
  - app/elohim-app/scripts/hc-mesh.sh
---

# Live session, slice 1

This plan builds the smallest part of the live-session design that proves its seams. The design is
`genesis/docs/superpowers/specs/2026-10-14-live-session-plane-design.md` as committed at `ebcaa0847`.
Read its §2 (the rule) and §5 to §7 first; this plan uses its terms without redefining them.

Nothing here is built. The story is written and tagged `@wip`. No code starts before the operator
has read this plan, and no storage code starts before the storage changes already in flight land.

Words the design defines, repeated here in one line each so this plan can be read alone:

- **Working version**: a changeable, unpublished copy of a thing, kept as an Automerge document
  on each holder's storage node.
- **Circle**: the people who may edit one working version. The signed record that would say who
  is in it does not exist yet.
- **Passing state**: who is here, and small in-the-moment values such as a selection. It is sent
  between those present, never stored, and never decided from. "Presence" below means the same.
- **Attention plane**: the design's name for the concern passing state belongs to, as distinct
  from a thing's bytes, its current version, or who holds it.

Words used below that the design does not define:

- **Grant**: a signed record in which one agent gives another leave to do a named kind of thing.
  Here it is the existing `delegates-compute` commitment, which is notarised once and then read
  from a local table.
- **Operation gate**: the existing function that answers "does this agent hold a live grant for
  this kind of thing", `authorize_operation` in `services/operation_authorization.rs`.
- **Conductor**: the process beside each storage node that signs and records what a person does.
  A request to it is the cost the household counts.
- **Protocol**: one named kind of stream between two nodes. Storage registers each at startup.
- **Stage pricer**: the storage code that decides how thoroughly a record is checked, by the
  network's declared stage. Some kinds of record are always checked in full.
- **SDO and RWA test**: a question the design gate asks of anything that holds information: if
  the few who seek to dominate, and the many disposed to follow them, held this, what could they
  see, join and compel?
- **The three storage changes in flight**: three local branches of the storage crate, each still
  in independent review and none pushed: `perf/consent-read-on-demand`, `fix/provide-revoke-arm`
  and `fix/retired-pin-readmit-backoff`.

## 1. The story

`genesis/a2o/features/dataplane/live-session-own-devices.feature`, tagged `@wip` and
`@concern:live-session-direct`. It is the first runnable seam of the vision story
`features/federation/local-first-shared-sheet.feature`, scenario "They see each other's edits and
cursors as they happen".

Matthew has one shared sheet open on two machines he looks after: his always-on node and a laptop.
The story's seven scenarios say:

| Scenario | What it fixes |
|---|---|
| Each machine shows the other is here | within 5 seconds, both ways |
| A value set on one machine appears on the other | within 5 seconds |
| A machine that closes the sheet is shown as gone at once | within 5 seconds |
| A machine that vanishes is shown as unknown, never as having left | "unknown" within 45 seconds (a 30 second expiry plus margin) |
| A node Matthew has not granted leave to is refused and told why | the reason is named; neither side shows the other |
| With no sheet open, a node that could take part does nothing for it | no live-session message; conductor requests unchanged over 5 minutes |
| A session leaves nothing behind | no row, no chain action, no change to the sheet |

The 5 second bound is a starting value, copied from the sibling story `hub-carries-edit`. It is
replaced by a bound set from the first reading (§5).

## 2. The seven seams, settled against the code

Each row says what the plan file proposed before the code was read, what the code says, and what
the plan now proposes. Four rows changed.

### Seam 1. Where the session logic lives — unchanged

- **Proposed:** a new crate, `elohim/elohim-live-session`, pure and without I/O.
- **Read:** `elohim/elohim-hub/README.md` says the hub is the home for serving nearby peers and
  that storage must never depend on it. Its debt table says the node process that would compose
  the two is "published by no pipeline" and "deployed by no manifest". The crate holds only
  `local_views.rs`. The household mesh runs the bare storage binary.
- **Now:** the same. The crate holds the per-person register, dedup by count, renewal and expiry,
  the send cap, the size limit and the session id. Storage uses it now; the node process uses it
  unchanged when it ships. A row is added to the hub README's debt table saying so.

### Seam 2. Transport and registration — unchanged, with one limit found

- **Proposed:** storage registers one iroh protocol.
- **Read:** `main.rs:3956-3969` builds the list of protocols (`sync`, `epr`, `epr-atom`, `shard`,
  `view-federation`, `identity-handshake`, `trust`) and passes it to
  `IrohNode::start_with_protocols` (`p2p_iroh/node.rs:61-80`). The household's default transport is
  `dual` (`hc-mesh.sh:277`), and peer `matthew`'s status shows working iroh paths to the other two
  peers, with a round trip near 32 milliseconds.
- **Now:** one more entry in that list, `/elohim/live-session/1.0.0`, whose handler is a thin
  adapter over seam 1. **Limit:** a peer started in libp2p-only mode has no iroh node and takes no
  part. It says so; it does not pretend. libp2p parity is slice 2 or later.

### Seam 3. Frame encoding — unchanged

- **Proposed:** the existing shared wire format.
- **Read:** `p2p_iroh/codec.rs` states that every custom protocol on the iroh path shares one
  format: a MessagePack body with a length prefix.
- **Now:** the same, with a format version field first in every frame. An unknown version is
  dropped.

### Seam 4. Admission — changed

- **Proposed:** accept a join when both nodes "speak for the same person", read from the device
  carrier's in-memory evidence.
- **Read:**
  - The operation gate already answers this kind of question. `authorize_operation` looks up an
    active `delegates-compute` grant for a named agent and a named scope in the local projection
    of commitments, requires that the grant is anchored on the notary, runs seven bounds checks and
    returns the grant's address (`services/operation_authorization.rs:76-110`,
    `db/mishpat_commitments.rs:204-219`). It makes no conductor request.
  - The gate does not check who issued the grant: "`provider` is NOT filtered in Slice 1"
    (`db/mishpat_commitments.rs:199-201`).
  - Scopes are a closed list of three names, extended by adding a name
    (`api/compute_grants.rs:40-46`). No entry type changes and the DNA hash does not move.
  - The carrier's evidence (`Speaks`, `crates/consent-grant/src/pending.rs:139`) answers "may this
    node approve devices for a person". It is refreshed on a timer today, and the change under
    review makes it on demand.
  - The carrier already makes, once per process, a proof of the node's agent key over its
    transport id (`CarrierProof`, `crates/consent-grant/src/carrier.rs:74`).
- **Now:** a join is admitted when all three hold:
  1. the joining node holds an active grant with scope `live-session` (the operation gate);
  2. that grant was issued by the same agent who issued the receiving node's own `live-session`
     grant (a check this slice adds, because the gate does not make it);
  3. the join carries a proof of the joining node's agent key over the transport id of the
     connection it arrived on, made once per process and checked locally.

  A refusal names which of the three failed.
- **Who issues the grants.** One agent, the issuing agent: the agent key of the node Matthew
  issues from, which on the household is node `matthew`. It issues a grant to each other machine,
  and holds one to itself. A grant whose issuer and holder are the same agent is the existing
  self-contract shape the gate already expects ("the self-contract has provider==recipient",
  `db/mishpat_commitments.rs:199-201`). So check 2 reads the same on every node: the joiner's
  grant and my grant name the same issuing agent. A node that issues itself a grant from its own
  key gains nothing, because no other node's grant names that key.
- **Limit of this slice:** one issuing agent per set of machines. Each of a person's devices
  writes under its own agent key, so a grant issued from a second device would not match. The
  circle record is what removes this limit.
- **Not yet verified, and the checks depend on it:** that a node's local projection holds another
  node's grant, and that the grant route issues both a grant to another agent and a self-contract
  under the new scope. Task 0 answers both before task 3 starts.
- **Why it changed:** the operator asked that the slice lay down the authorisation and
  resource-accounting seams so that production is easy later, and noted that the household's
  machines are ones he stewards. A grant per machine says exactly that, is one act of trust per
  machine, and is already the record a host will hold in slice 2. It also removes the join path's
  dependence on the carrier's evidence, which is the part under review.
- **What it defers:** the circle's own admission record. The three checks sit behind one function
  so that record replaces them without touching seams 1 to 3.

### Seam 5. The app's way in — unchanged, predicate named

- **Proposed:** one this-machine-only route on the local node.
- **Read:** `caller_is_local` (`http.rs:1244-1258`) is the existing predicate: a loopback caller
  that is not a trusted proxy and carries no forwarding header. Storage already has a
  server-to-client event stream (`sse.rs`).
- **Now:** two routes declared in storage's route manifest, both refused unless `caller_is_local`:
  one to set this node's passing values in a session, one event stream of who is here. The node
  binds them to this machine itself. That matters because the open finding
  `security-doorway-auth-required-unenforced` shows a declaration alone is not enforced.

### Seam 6. SDK contract — changed

- **Proposed:** a seam file, a session object in both SDKs, and `passing` added to the
  app-manifest schema.
- **Read:** the app-manifest schema has no `workingVersion` block at all; `passing` would be a
  field of a block the working-version standard owns and has not landed. The Rust SDK has `client`,
  `sync`, `cache`, `reach` and `traits` modules. The seam file format is
  `elohim/sdk/working-version.seam.json`.
- **Now:** the seam file and the two session objects, as proposed. The manifest schema is not
  touched. An app gives its passing names when it opens a session, and the SDK refuses any other
  name.
- **Why it changed:** adding half of another standard's block would be designing that standard
  from the side.

### Seam 7. Session id — unchanged

- **Now:** a domain-separated hash of the working version's document id, computed in seam 1. It is
  a lookup key and never an address.

### Fixture — changed

- **Proposed:** stage a fourth peer and make it Matthew's second device.
- **Read:** this fixture already exists as a pattern. `just mesh join-peer workspace` is the arm
  for staging a fourth node, and the story `auth/device-consent-grant.feature` is written to enrol
  it as Matthew's device. Whether the two run cleanly together today was not tried (task 0).
- **Now:** nodes `matthew` and `workspace`, each holding Matthew's `live-session` grant. Node
  `james` holds no grant and is the refused node. Because admission is by grant, the same slice
  runs unchanged between any two household nodes Matthew grants, whether or not they are enrolled
  as his devices. The operator has said he is unsure the second-device shape is the one he wants;
  see "Decisions the operator has not ruled on".

## 3. Where this slice departs from the design

These are deliberate, and each is a stand-in the design's own shape replaces later.

| Design says | Slice does | Why |
|---|---|---|
| §4: admission is the circle's signed record | a `live-session` grant from one issuer | the circle record does not exist |
| §4 and §7: `live-session` is the scope a host holds | the same scope name admits a participant | one name now; slice 2 decides whether hosting needs its own |
| §5: every frame is signed with a one-session key | frames carry sender, session and count but no signature | the two ends of a direct stream are already authenticated to each other; a signature matters when a host relays |
| §9: the SDK refuses a name the manifest does not declare | the SDK refuses a name the app did not give at open | the manifest block is not this slice's to add |

One contradiction is inside the design itself, found while writing the story. §5 has a clean exit
send an explicit leave "so that others do not wait". §8 says a person who left and a person whose
device is off "are indistinguishable to everyone else". Both cannot hold inside a session. The
story takes §5: inside a session a signalled leave is shown as left, and silence is shown as
unknown. §8 needs one sentence changed to say "to everyone outside the session". The design is not
edited by this plan.

## 4. Local-first reading and design gate

### Local-first reading: live session, slice 1

- **Alone, key on the device:** nothing changes. The sheet opens, is edited and saved as before.
  No session exists, and the node sends nothing.
- **Alone, key held by a doorway:** does not apply. The slice has no doorway and no browser path;
  a hosted person is unaffected and gains nothing yet. Named slice for it: none before slice 2.
- **Alone, no key:** does not apply. A node with no grant is refused and shown nobody.
- **Working version:** shared. The slice changes nothing about who may edit or how it republishes.
- **Each kind of state:** shown-here state stays on the device. Passing state travels on the
  direct stream and is held in memory only. The working version receives nothing from presence.
- **Arrivals:**

  | Arrival | Adds | Learns | Survives its loss |
  |---|---|---|---|
  | Another of the person's machines, with a grant | its presence and values | the person's presence and values in this one sheet | everything; the first machine works as before and shows "unknown" |
  | A machine without a grant | nothing | that a refusal was given, and the reason | everything |

  Could one of the person's other devices do what the design asks a host to do? In this slice
  there is no host; the devices are the whole of it.
- **Shown to the person:** here, left, or unknown, for each other machine in the sheet.
- **Refusals:** none of the red flags apply. The first step is the device's own; no store exists;
  nothing is required from the person that must stay on their device.
- **Stands as is:** the working-version verbs and sync.

**Back-fill check.** Delete every doorway: the slice is untouched, since it never used one. Close
the laptop mid-change and reopen with no network: the sheet is there and the other machine reads
"unknown". The first thing that waits on someone else is seeing the other machine, which is theirs
to send.

### P2P design gate: live session, slice 1

#### Entity: presence frame

- **Classification:** not an entity (the local-first reading's "passing"). Nearest gate class:
  Ephemeral (C), rebuilt by the sender's next renewal.
- **Network stakes:** the same at all four stages; carries no floor-protected record.
- **Address:** none. `(sender, session, count)` for the life of a connection.
- **Transport affinity:** iroh direct stream.
- **Source of truth:** the sending device's memory.
- **Integrity zome and DNA-hash class:** none. DNA-hash-neutral.
- **Coordinator, projections:** none. No table, no Automerge projection.
- **Anti-pattern check:** identities are kept apart. The join names an agent key; the connection
  has a transport id; the two are joined only by a signed proof, never compared as strings.
- **SDO/RWA test:** the other machine in a session sees the person's presence and values there.
  Nothing outside the session sees anything. Boundary 6 holds: no outside observer learns that
  the session exists.

#### Entity: `live-session` grant

- **Classification:** Notarized (A), an existing kind. The commitment is the existing
  `delegates-compute` entry in the mishpat DNA; the slice adds a scope name to a closed list.
- **Head-plane cost:** one commitment per machine granted. At the household, three or four. At
  one year it is bounded by machines a person looks after, far below the level that needs
  bundling. Nothing is written per session.
- **Network stakes:** all four stages. A grant is a delegation, which the stage pricer treats as
  constitutional: its checking is never made cheaper.
- **Address:** content-derived; the commitment's entry hash, which is what the gate returns.
- **Source of truth:** the notary. The local table is a projection, read only when anchored.
- **Integrity zome and DNA-hash class:** unchanged. DNA-hash-neutral, coordinator untouched.
- **Coordinator:** the existing commitment creation path used by the grant route.
- **Route:** the existing grant route; one new accepted scope.
- **SDO/RWA test:** who holds a `live-session` grant from whom is a notarised record, readable by
  whoever can read commitments. It reveals that a person lets a machine take part in live
  sessions, not which sessions or when.

#### Entity: local session routes

- **Classification:** not an entity. A this-machine-only surface over memory.
- **Route:** declared in storage's route manifest; refused unless `caller_is_local`.

#### Design constraints discovered

- The gate's back-fill questions: no coordinator function is added; no integrity zome changes;
  the count at one year is a handful of grants and zero stored session items.
- Whether a node's local projection holds another node's grant is not yet verified (task 0). If
  it does not, check 1 of seam 4 needs the joiner to present its grant for checking.

#### Concern canon, for the join decision

Recorded as intent. Every row is `unbound` until the decision exists in code and is registered in
storage's seam registry with a contract test. The canon's own wording was read for C4 and C5 only.

| Class | Intended answer |
|---|---|
| C0 plane location | attention plane; never the notary |
| C1 anti-self-election | a node is never admitted on its own word: its grant must name the same issuing agent as the receiving node's grant. Only the issuing agent's own node holds a self-issued grant |
| C2 monotonic authority | a revoked grant refuses the next join; an open stream is closed at its next renewal |
| C3 liveness | renewal at half the expiry; expiry drops the state |
| C4 honest absence | refused, left, unknown and here are four different answers |
| C5 evidence, not authority | presence is never read as proof of anything |
| C6a bounded work | send cap and size limit enforced on receipt |
| C6b idempotent effect | a repeated frame is dropped by count |
| C7 advertise and serve symmetry | a node that registers the protocol answers every join, with admission or a reason |
| C8 observability per decision | each refusal names the failed check |
| C9 identity-lineage continuity | agent key and transport id joined only by the signed proof |
| C10 contract-evolution honesty | format version first in the frame; unknown versions dropped |
| C11 externally imposed backpressure | a slow receiver gets the latest state, not a queue |
| C12 consent and authorization | the three checks of seam 4 |
| C13 graduated authority | n-a: one kind of participant |
| C14 witnessed residual | n-a: nothing remains |

## 5. Tasks, in build order

Storage is one crate with one build slot. Tasks 2 to 4 go to **one** agent, in order, so that no
more than one storage build from this slice ever runs beside another agent's. Task 1 needs no
storage build and may run beside it. Task 5 needs no storage build either; it is written against
the two route shapes fixed in task 4, so it starts when task 4's routes are agreed, which may be
before task 4 is finished.

Task 0 produces a dated section appended to this plan, "Task 0 findings", with one answer per
question and the file and line it was read from. Task 3 does not start without it.

| # | Task | Files | Test | Gate | Precondition |
|---|---|---|---|---|---|
| 0 | Verify, read-only: a node's projection holds another node's grant; the grant route issues a grant to another agent and a self-contract under a new scope; how the carrier obtains its key proof and what it costs; that `join-peer workspace` followed by the device-joining story yields an enrolled fourth node today | this plan ("Task 0 findings" section) | none | none | none |
| 1 | The pure crate: register, dedup, renewal and expiry, cap, size limit, session id, frame type | `elohim/elohim-live-session/` (new: `Cargo.toml`, `src/lib.rs`, `src/register.rs`, `src/frame.rs`, `src/session_id.rs`, `build-manifest.json`); `elohim/Cargo.toml` (member) | unit tests in the crate, including a property test that a repeated or older count never changes the register | `just gate elohim/elohim-live-session` | none |
| 2 | Accept the `live-session` scope | `elohim/elohim-storage/src/api/compute_grants.rs` | existing grant tests plus one for the new scope | `just gate elohim-storage` | the three storage changes in flight have landed |
| 3 | Admission function and protocol handler; register the protocol | `elohim/elohim-storage/src/services/live_session_admission.rs` (new), `src/p2p_iroh/live_session.rs` (new), `src/p2p_iroh/mod.rs`, `src/main.rs`, `seam-registry.yaml` | two in-process iroh nodes through the existing parity harness: admitted join, each of the three refusals, expiry, clean leave | `just gate elohim-storage` | task 2; task 0's answer on the projection |
| 4 | The two local routes | `elohim/elohim-storage/src/api/live_session.rs` (new), `src/http.rs` (route manifest and dispatch) | a non-local caller is refused; a local caller sets and streams | `just gate elohim-storage` | task 3; **the consent-read-on-demand change has landed**, because it edits `http.rs` and `device_carrier.rs` and is at review round 5 |
| 5 | SDK session object and seam file | `crates/elohim-sdk/src/live_session.rs` (new), `crates/elohim-sdk/src/lib.rs`, `elohim/sdk/storage-client-ts/src/live-session.ts` (new) and its index, `elohim/sdk/live-session.seam.json` (new) | unit tests against a stub of the two routes | `just gate crates/elohim-sdk` and `just gate elohim/sdk/storage-client-ts` | the two route shapes of task 4 agreed in writing |
| 6 | Story steps and fixture | `genesis/a2o/steps/dataplane.steps.ts`, the feature file (remove `@wip`) | `just test mesh features/dataplane/live-session-own-devices.feature` | the same | tasks 3 to 5 on the household; **the device-joining ceremony is stable**, because it is being changed this sprint and the fixture depends on it |
| 7 | Independent review of tasks 2 to 4, then the measurements in §6 | none | none | none | task 6 |
| 8 | Record: hub README debt row; the design's §10 rows that moved; the backlog cluster row; a one-line delta in the habit atom the concern is listed under | docs | none | none | task 7 |

## 6. The measurements that decide it

1. **Rest.** `household-at-rest.feature` reads the same conductor request counts per node with the
   live-session protocol registered and no sheet open as it reads without it, and the node's
   live-session message counter stays at zero. If either moves, the slice has broken
   `idle-is-free` and the protocol must be registered on first use instead of at startup.
2. **Delay.** The time from a value being set on one machine to its being shown on the other, over
   at least a hundred sets. The stated bound to beat is 5 seconds. The bound the story keeps is
   set from this reading.
3. **Expiry.** A machine stopped without warning reads "unknown" on the other between 30 and 45
   seconds later, and never reads "left".
4. **Refusal.** A node without a grant is refused with the named reason and is shown nobody.
5. **Residue.** After a session, no table has a new row and no chain has a new action.

## 7. What slice 2 adds

Named only: two hosts on the household and the kill-one-host scenario; the host posture; per-frame
signatures; the ranking function; relay mode in the handler; whether hosting takes its own grant
scope; libp2p parity.

## Decisions the operator has not ruled on

Each is written into this plan as a proposal. None has been ruled.

| # | Proposal in this plan | Alternative |
|---|---|---|
| 1 | Session logic in a new pure crate, `elohim-live-session` | In `elohim-hub`, with nothing runnable on the household until the node process ships |
| 2 | Fixture: nodes `matthew` and `workspace`, the second enrolled as Matthew's device | Two of the three existing household nodes, each given Matthew's grant as machines he stewards, with no fourth node. Admission by grant makes either work without code changes; the story's wording and background would change |
| 3 | The concern `live-session-direct` listed under the habit `dataplane-convergence` | Under `idle-is-free`, whose invariant the rest measurement serves more directly; or under no habit until slice 2 |
| 4 | Documents and story now; code after the storage changes in flight land | Wait for the storage queue to clear before any of it |
| 5 | Admission by a `live-session` grant from one issuer | The first proposal: both nodes speak for the same person, read from the carrier |
| 6 | The same scope name for participants now and hosts later | Two scope names from the start |

## Evidence and its limits

- Read directly for this plan: `main.rs:3950-3972`, `p2p_iroh/node.rs:28-80`, the header of
  `p2p_iroh/auth.rs`, `services/operation_authorization.rs:1-110`,
  `db/mishpat_commitments.rs:198-219`, `api/compute_grants.rs:1-50`, the header of
  `services/device_carrier.rs`, `crates/consent-grant/src/pending.rs:40-160` and
  `carrier.rs:60-90`, `http.rs:1240-1262`, the hub README's seam rules and debt table,
  `hc-mesh.sh:55-66,277`, and the header of `auth/device-consent-grant.feature`. Peer `matthew`'s
  local status was read once for its transport paths.
- Not verified, and listed as task 0: whether one node's projection holds another node's grant;
  what the carrier's key proof costs to make; whether the fourth-node fixture enrols cleanly today.
- Not read: the grant route's handler below line 50, the Rust SDK's modules beyond their names,
  the storage route manifest's declaration format.
- Nothing is measured.
