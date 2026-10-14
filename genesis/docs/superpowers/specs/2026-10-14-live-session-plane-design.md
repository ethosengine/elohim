---
title: The live session — where passing state travels, who hosts it, and why two hosts
id: live-session-plane-design
status: Draft
class: architecture
serves: dataplane-convergence
date: 2026-10-14
context-tier: disclosed
steward: human:matthew
graduation-trigger: >
  Two people on separately enrolled devices see each other's presence and cursor in one shared
  working version on the household mesh; killing one of the two hosts mid-session leaves no gap;
  killing both leaves editing working; and a peer that offers the host role with no session open
  reads zero added calls on the household-at-rest lane. A document or Gherkin check alone cannot
  graduate it.
cites:
  - "working-version-sdk-standard-design | defines passing state and leaves its transport unchosen; this spec fills that slot and reuses its circle, verbs and doorway refusal | sha256:66ba00931d6dd97e | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
  - "shared-working-versions-social-coding-design | says connected editors exchange changes live and that presence and cursors are passing state; names the always-on holder this spec gives a host role | sha256:19a6bb3d12f01c95 | path: genesis/docs/superpowers/specs/2026-10-13-shared-working-versions-social-coding-design.md"
  - "plane-separation-design | rules one trust act per relationship, no unread record, and that node liveness is not a notary fact; this spec follows all three and does not reopen the heartbeat ruling | sha256:657eb48f9c7a4db0 | path: genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md"
  - "compute-envelope-tevah | defines the node envelope, guest sub-envelopes with quota and a bounding commitment, and placement without a scheduler; the host role runs as such a guest | sha256:d427a367f3b01a6e | path: genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md"
  - "swarm-curve-and-blind-custody-design | the unbuilt design for sealing bytes to a circle; until it lands a host must stand inside the circle | sha256:ef23b30ec9b8145c | path: genesis/docs/superpowers/specs/2026-08-23-swarm-curve-and-blind-custody-design.md"
  - "sync-state-contract | owns the caught-up, behind and unknown words; presence that cannot be known is shown as unknown in the same sense | sha256:cea95b17140d2fd6 | path: genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md"
  - "elohim-hub-boundaries-design | defines a hub as a role that serves nearby peers and is not a doorway; the session host is hub work | sha256:233cb996edd7c366 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-02-elohim-hub-boundaries-design.md"
  - "elohim-seam-map-concern-routing | names the four participation tracks and the role seams used here to say where a session travels and where its host lives | sha256:3ce0f9bd564ca1bf | path: genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md"
  - "iroh-libp2p-complementarity | describes the thin-client connection to a hub whose wire format is unsettled and the undesigned browser-direct path, both listed here as missing nodes | sha256:da518f4d0861aff8 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-08-iroh-libp2p-complementarity.md"
  - "observation-event-layer-design | the only path by which a count from a session could ever become a record, and only with a reader | sha256:002fc7ec733dd10a | path: genesis/docs/content/elohim-protocol/architecture/2026-05-11-observation-event-layer-design.md"
  - "rea-compute-commitment-primitive | the delegates-compute commitment a host works under; this spec asks for one new scope name on it | sha256:3ea123e3a9796449 | path: genesis/docs/architecture/rea-compute-commitment-primitive.md"
  - "private-thought-governed-fruit | the six hard boundaries; boundary 6, that taking part is itself sensitive, bounds what a host may learn | sha256:2945eb608257be1e | path: genesis/docs/architecture/private-thought-governed-fruit.md"
  - genesis/data/timeline/CONVENTIONS.md
  - elohim/elohim-hub/README.md
  - elohim/sdk/working-version.seam.json
  - elohim/elohim-storage/src/reconcile/placement.rs
  - elohim/elohim-storage/src/services/peer_selection.rs
  - elohim/elohim-storage/src/api/compute_grants.rs
  - doorway/doorway-service/src/signal/mod.rs
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

# The live session

This is a design, not a build order. Nothing in it is built and nothing in it is measured. It
fills one slot that an earlier standard left empty on purpose.

The working-version standard defines **passing state**: who is present in a thing that is being
changed, where their cursor is, a drag in progress. It says passing state travels on a separate
channel, is authenticated to its sender, is sent without guarantee, expires on a heartbeat and is
never stored. It then says: "The transport for passing state is not chosen here." This document
chooses it.

Terms used here:

- **Merging layer**: the part of a storage peer that keeps Automerge documents and merges
  concurrent changes to them. It carries values and no authority.
- **Working version**: a changeable copy of a thing, kept on the merging layer, that has not been
  published. Defined by the working-version standard.
- **Circle**: the people who may edit one working version. Who is in it is a signed record.
- **Reach**: how far a thing may travel, as a named rung from `private` out to `commons`. A
  circle is never wider than the reach of the thing it edits.
- **Passing state**: the in-the-moment state above. It is not a record and has no history.
- **Live session**: the channel that carries passing state for one shared working version, for as
  long as someone has it open.
- **Host**: a peer that relays a live session's messages between the people in it.
- **Hub**: a role a peer takes when it serves nearby peers. It is not a kind of machine and it is
  not a doorway (the hub boundaries design).
- **Doorway**: a node run as a service so that browsers can reach the network by a web address.
- **Substrate**: the peer-to-peer connections storage peers hold with each other, as distinct
  from the Holochain network that notarises records.
- **Notary**: the Holochain network, used here only to record that something was declared, by
  whom and when. A **head** is the version of a thing the notary currently names as current.
- **Plane**: one of the separate concerns a single thing has, each with its own cost: its bytes,
  its head, who has authority over it, who holds it, who is attending to it. The names used in §5
  are the ones the timeline conventions list.
- **Posture**: a standing declaration a peer makes about what it offers, such as "I accept
  shards" or, here, "I host live sessions". It is written when it changes and not on a timer.
- **Compute envelope**: the supervisor process that is the parent of everything a node runs, and
  that can run an admitted guest with its own resource limit (the compute envelope design, which
  calls it a tevah).

## 1. Why this is being written now

Matrix, the federated chat protocol, has an online-status feature that almost every large
deployment turns off. Reading its issue history gives eight separate causes. Each one is a design
choice this document has to make differently.

| What went wrong there | The rule it gives us |
|---|---|
| The audience was every server sharing any room with the person | The audience is the people who joined this session |
| More than half of all updates carried no change, only a timestamp | Nothing is sent when nothing changed, apart from one renewal while the thing is open |
| One phone moving between foreground and background could saturate a small server | Updates are coalesced and capped where they are sent and again at the host |
| A server could not tell senders to stop | Nothing is pushed to a party that did not join; leaving stops it |
| Every update woke every reader | A session's traffic reaches only that session |
| Status was written to a database | Nothing is stored, anywhere |
| Joining a large room caused a burst | A session has a declared size; above it people get a count |
| Timestamps leaked activity to the millisecond | States are coarse and carry no time |

The figures behind the left column are in the Matrix issue tracker and the analysis at
`ispresencefixedyet.com`; they were read through a summariser and are not reproduced here as fact.

## 2. The rule

**Passing state is carried by a live session. A live session exists only while someone is in it,
holds nothing that cannot be resent, writes nothing down, and is relayed by two hosts at once.
Nothing that matters is ever decided from it.**

Four consequences follow and are used throughout:

1. **A session is not a record.** No chain write, no database row and no log line is made for a
   join, a leave, a cursor or a session.
2. **A host holds nothing authoritative.** Everything a host knows was sent to it in the last few
   seconds by the people in the session and will be sent again.
3. **Presence exists only inside a session a person joined.** There is no standing answer to "is
   this person online" outside one. This is a decision, taken by the operator on 2026-10-14.
4. **Rest costs nothing.** A person with nothing open sends nothing. A peer that offers to host
   and has no session holds no timer and makes no call.

## 3. Local-first reading

- **Alone, key on the device:** no session exists. Opening, editing, fixing and republishing a
  working version work exactly as the standard says. Nothing waits on a host.
- **Alone, key held by a doorway:** the same. The browser keeps the working version; no session
  is opened because nobody else is there.
- **Alone, no key:** the same. A visitor has no circle to join.
- **Working version:** shared, by definition. A live session is offered only for a type whose
  manifest declares `kept: circle` and a non-empty `passing` list.
- **Each kind of state:** what this device shows (selection, scroll) never leaves it. Passing
  state travels in the session and is never stored. The working version receives one change when
  a gesture ends. Published versions are untouched.
- **Arrivals:**

  | Arrival | Adds | Learns | What survives its loss |
  |---|---|---|---|
  | Another of the person's devices | A second place to work; it can also be a host | The person's own passing state | Everything; the first device works as before |
  | Another person in the circle | Their presence and cursor | The person's presence and cursor in this one working version | Editing and merging continue through ordinary sync |
  | A host | Reach between people who cannot connect directly, and a count | Who is in the session and when they send; the content of frames, until frames can be sealed | Editing, saving and republishing. Presence reads "unknown" |
  | A second host | Continuity when the first is lost | The same as the first | Nothing changes while the other host stands |
  | A doorway | A way in for a browser that cannot reach a peer by key | Everything it relays, in clear | Native devices are unaffected; a hosted person keeps editing in the browser |

- **Shown to the person:** who else is here; "unknown" when no host is reachable; never "absent"
  on the strength of silence alone (§7).
- **Refusals:** an earlier draft of this design started every session at a host. That fails the
  local-first rule, so §6 adds a direct path and two of a person's own devices on one network
  need no host. The earlier draft also kept a join time per person; §8 removes it.
- **Stands as is:** the working-version standard's verbs, its sync routes and its publication
  paths. The session is added beside them and changes none of them.

**Back-fill check.** Delete every doorway: a person's node still opens sessions with peers it can
reach by key. Close the laptop mid-change and reopen it with no network: the working version is
there, and presence reads "unknown". The first thing that waits on someone else is seeing another
person's cursor, which cannot be prepared alone because it is theirs.

## 4. Design gate

### Entity: passing-state frame

- **Classification:** not an entity. The local-first reading classes passing state as "not an
  entity". Of the gate's five classes the nearest is Ephemeral (C): delete it and it is rebuilt by
  the sender's next message. It has no table, no column and no projection.
- **Head-plane cost:** none. No head, no provider record, no conductor call.
- **Network stakes:** the same under all four declared network stages. The protocol names three
  kinds of record whose checking is never made cheaper (constitutional records, counter-evidence,
  and local-relationship records). A session carries none of them.
- **Address:** none. A frame is named by `(sender, session, count)` for the life of one
  connection (§5).
- **Transport affinity:** direct streams between nodes on the substrate; a stateful connection
  from a thin client to a hub. Not Holochain signals and not wide gossip (§6).
- **Source of truth:** the sending device.
- **Integrity zome and DNA-hash class:** no change. DNA-hash-neutral.
- **Coordinator, projections, routes:** none.
- **Count at one year:** bounded by what people have open at one moment. It costs quiesce nothing.

### Entity: session admission

- **Classification:** not new. Admission to a session is the circle's signed admission record,
  which the working-version standard names as a missing node. This document adds no second record.
- **Everything else:** as that standard decides when the record is designed.

### Entity: host commitment

- **Classification:** not new. A host's work is a `delegates-compute` commitment, an existing
  Notarized (A) kind. This document asks for one new scope name, `live-session`, in the closed
  scope list. One commitment covers a host's standing offer to a circle; nothing is written per
  session.
- **Head-plane cost:** one commitment per host per circle it serves, written when the offer is
  made and when it is withdrawn.

**SDO and RWA test.** The design gate asks this of every store and service. The letters stand
for social dominance orientation and right-wing authoritarianism: the few who seek to dominate,
and the many disposed to follow them. Asked of the host: if that pairing held it, what could they
see, join and compel? They could see who is in each session it relays and when each person is
active, and until frames are sealed, where each cursor is. They could not join a session without
the circle's admission. They could not compel anything, because nothing is decided from passing
state. Boundary 6 (taking part is itself sensitive) is what bounds this: at narrow reach, a host
must stand inside the circle (§8). A host that keeps what it relays breaks this document's rule
and cannot be prevented by design, only by who is chosen.

**Back-fill detector.** The gate asks three questions that cannot be answered by working
backwards from a route. No coordinator function is added, so none returns a hash. No integrity
zome changes. The count at one year is zero stored items.

### Concern canon, for the two new decisions

The design gate keeps a list of sixteen failure classes that have recurred in production, and
asks every new decision to answer each one. The decisions here are "may this agent join this
session" and "which peers host it". Neither exists in code, so every row is `unbound` until
built. The intended answer is recorded so it is not rediscovered.

| Class | Intended answer |
|---|---|
| C0 plane location | Attention plane only; §5 table |
| C1 anti-self-election | A host never chooses itself: the ranked list comes from the circle's commitments (§7) |
| C2 monotonic authority | n-a: a session grants no authority |
| C3 liveness | Renewal and expiry (§5); a host's liveness is the substrate's, per plane separation §5 |
| C4 honest absence | Silence reads "unknown", never "absent" (§7) |
| C5 evidence, not authority | A frame is a hint. "A heartbeat hints; the record decides" |
| C6a bounded work | Size bound and send cap, enforced at the host (§5) |
| C6b idempotent effect | A repeated frame is dropped by its count |
| C7 advertise and serve symmetry | A peer that declares the host posture must accept an admitted join, or withdraw the posture |
| C8 observability per decision | A refused join names its reason to the joiner |
| C9 identity-lineage continuity | A join names the agent and binds a one-session key; transport ids resolve to the agent, never the reverse |
| C10 contract-evolution honesty | A frame carries a format version; an unknown version is dropped, not guessed |
| C11 externally imposed backpressure | The host's cap; a slow receiver gets the latest state, not a queue |
| C12 consent and authorization | The circle's admission record, verified by each host and each receiver |
| C13 graduated authority | n-a |
| C14 witnessed residual | n-a: nothing remains to witness |

## 5. Reading through the planes

The plane names are the ones in the timeline conventions. Each row says what a live session does
on that plane and what it is forbidden to do.

| Plane | What the session does | What it never does |
|---|---|---|
| Authority | Each join presents the circle's admission record once. Each host and each receiver verifies it locally. This is plane separation's "one trust act per relationship, carried by a proof" | A host never admits anyone. Being connected grants nothing |
| Notary | Nothing | No write per join, gesture or session |
| Attention | Carries presence and gestures. This is the plane the session exists for | Nothing here is kept, pooled or summarised per person |
| Bytes and head | Changes to the working version travel on the standard's sync routes. The session may also relay a change as soon as it is made, so that others see it sooner. A relayed change that is lost is recovered by ordinary sync | The session is never the only path for a change, and never touches a published head |
| Custody | None. The two hosts are for continuity, not for keeping | A host is never counted as a holder of the working version |
| Projection | A host may tell the session how many people are in it | No list of people leaves the session. No summary is written unless something reads it (plane separation §5, ruling 3) |
| Value | A host's work is accounted under its commitment | No per-message accounting |

**Per-plane shape of a frame** (the gradient reading's five questions):

- **Reach:** the circle. Never wider than the working version's own circle.
- **Custody:** none needed. The resilience wanted is continuity of a live channel, met by two
  relays.
- **Freshness:** a few seconds stale is acceptable; the stakes are nil.
- **Linkability:** covered in §8.
- **Cost bearer:** the people in the session pay to send; the hosts pay to relay, under a
  commitment.

**Session id.** Nobody creates a session. Its id is a pure function of the shared working
version's document id, so everyone who opens that working version computes the same id. A host
learns that a session exists when the first join for that id arrives, and forgets it when the
last person's state expires.

**Two kinds of frame.** A session relays two things, and they are kept apart:

- A **presence frame** carries one person's passing values. Each person in a session owns one
  small set of named values, the ones the app declared in its manifest's `passing` list. A
  presence frame is that whole set, never a difference from the last frame.
- A **change frame** carries one change to the working version, the same bytes the sync routes
  carry. It is an accelerator only (§5 table, bytes and head).

Every frame of either kind carries the sender's id for this session, the session's id, a count
that only goes up, and a signature. A frame has a size limit, which the host enforces. The limit
is unmeasured; one kilobyte for a presence frame is where the measurement should start. How a
frame is encoded on the wire is not chosen here (§10).

**Liveness.** A person's state is renewed at half its expiry and dropped at its expiry. A clean
exit sends an explicit leave so that others do not wait. Starting values to measure: renew at
15 seconds, expire at 30. These are the values the Yjs awareness protocol uses.

**Rate.** Gestures are coalesced so that only the latest position is sent, the final position is
always sent, and the sender holds to a cap. The host enforces the same cap and drops the excess.
Starting value to measure: between 10 and 30 frames a second per person.

**Size.** A type declares how many people a session shows in full. Above that number each person
receives the count and the states of a bounded few. The number is unmeasured; 50 is where the
measurement should start.

**Authenticity.** A join is signed by the person's agent key and names a key made for this
session alone. Frames are signed with the session key. A receiver checks each frame against the
join, so a host cannot invent a person's presence.

## 6. Where it travels

The seam map, the repository's guide to where a concern belongs, names four ways a node takes
part in the network and calls them tracks: the notary, the substrate between peers, a thin
client's connection to a hub, and a doorway's projection to the web. A live session uses the
middle two.

| Path | Used for | State today |
|---|---|---|
| Direct stream between two nodes that know each other's keys (track 2) | A person's own devices; household members; any two native peers. No host is needed | Streams exist for other purposes; none carries passing state |
| A stateful connection from a thin client to a hub (track 3) | A device that cannot hold direct streams to everyone, and any session larger than a handful | The connection's wire format is not settled |
| Through a doorway | A browser, which cannot reach a peer by key | Allowed only where the working-version standard already lets that doorway read the thing: `public`, `commons`, and a `community` it serves |
| The notary (track 1) | Never | n-a |

Two things are ruled out, with the reason:

- **Holochain remote signals.** Holochain's documentation describes them as send-and-forget with
  no error on failure, tells authors to use them sparingly, and calls heartbeats over them very
  costly. No DNA here uses them.
- **Wide gossip.** The reach-scoped gossip topics carry announcements to everyone subscribed to a
  reach. A cursor sent there would have the audience this document exists to avoid.

**Which paths a person uses.** The rule needs no detection and no negotiation:

1. A person opening a shared working version reads two signed facts their own node already
   holds: who is in the circle, and which peers hold a hosting commitment to it (§7).
2. If the circle has no hosts, the session is direct only. Each person opens a stream to each
   circle member whose key their node can resolve to a network address, using the binding between
   an agent and its transport identity that storage already keeps. A household of two or three
   devices works this way and never needs a host.
3. If the circle has hosts, each person connects to two of them (§7) and sends every frame to
   both. A person may also keep direct streams to members they can reach. A frame that arrives by
   more than one path is dropped after the first copy, by the same key.

So a host is something a circle chooses to have. It is never something a session falls back to
on its own, and a session never has to notice that direct streams have stopped being enough.

Two people who are in one circle and have never connected find each other's network addresses the
way any two peers do today, through the notarised binding between an agent and its transport
identity. Where that cannot be resolved and the circle has no host, each shows the other as
"unknown".

## 7. Two hosts

**The host is a role.** It is the hub's kind of work: serving nearby peers who reach it by key.
Its home is the hub crate, run by the same node process that runs on every peer-capable device.
It is not doorway code and not storage code. Under the compute envelope design it runs as a guest
of the node's own envelope, with a quota carved from the parent and bounded by its commitment, so
a runaway session cannot take the node down with it.

**Why two.** One host is a single point of failure for a thing people are looking at. Two hosts
with nothing shared between them remove it, by the oldest method there is: send on both, take
whichever arrives first.

- Each person connects to both hosts and sends every frame to both.
- A receiver takes the first copy of a frame and drops the second, by `(sender, session, count)`.
  The same key stops a frame looping between relays.
- The hosts do not talk to each other. There is no leader, no election and nothing replicated.
  If the two hosts cannot see each other, nothing goes wrong, because neither holds anything the
  other needs.
- A host that restarts knows nothing and is told everything again by the next renewals.
- Both hosts carry traffic all the time, so a dead host is noticed at once and not at the moment
  it is needed.

The cost is that every frame is sent twice. The send cap in §5 bounds it.

**Who may be a host.** Three conditions, all required:

1. The circle's own agreement admits it. Capacity alone never makes a peer a host. This is the
   compute envelope design's rule that placement has no scheduler: every party is a party to a
   commitment.
2. It holds a `live-session` commitment to that circle.
3. Until frames can be sealed to the circle, it is inside the circle as a reader. The
   working-version standard refuses a doorway on the same ground, and calls it a refusal and not
   a mitigation to schedule.

**How the two are chosen.** The circle's commitments give a standing list of peers that may host.
Every person in the session ranks that list the same way: by a deterministic function of the
session id, preferring a second host in a different household from the first, then a different
kind of device, then a different node. This is the rule storage already uses to place shards
(`reconcile/placement.rs`, `services/peer_selection.rs`). Because the list is a signed fact and
the ranking is a pure function, everyone computes the same order without asking anyone.

Each person holds connections to the two highest-ranked hosts that answer them, and keeps trying
any higher-ranked host they are not on for as long as they have the thing open. A person who
cannot reach the first uses the second and third.

This converges whenever people can reach the same hosts. If the top two are down for everyone,
everyone lands on the third and fourth. People end up on different hosts only when reachability
differs between them, which is the second limit below.

**What the pair does not promise.** Three limits, stated so nobody assumes otherwise:

- **A household's own session is hosted inside the household.** Condition 3 means the two hosts
  of a household working version are two of that household's devices. That survives a device
  failing. It does not survive the house losing power or its network. Before frames can be
  sealed there is one way round it, and it is the household's own choice: admit a peer in another
  household to the circle as a reader, knowing it can then read the working version. After
  sealing, any committed host will do.
- **Two people may each be present and not see each other.** If one person can reach only the
  first two hosts and another only the third and fourth, they share no host. Each sees the other
  as "unknown". The retry above repairs it when reachability returns. This is why silence is
  never shown as absence.
- **Both hosts can be lost.** Presence then reads "unknown" for everyone who has no direct
  stream, and editing continues through ordinary sync. The last few seconds of passing state are
  gone, which is what passing state is. A change frame in flight is not lost: the same change
  reaches the others by sync, later.

## 8. What a host learns

- **Who and when.** A host sees which agents joined and when each sends. That cannot be hidden
  from a relay.
- **What.** Until frames are sealed to the circle, a host can read them. This is why condition 3
  in §7 exists. Sealing depends on the blind custody design, which is not built.
- **Outside the circle, nothing.** A session's existence, its people and its traffic are not
  announced anywhere. A person's sender id is made fresh for each join, so two sessions cannot be
  matched by an outside observer of traffic.
- **No time.** A state carries no timestamp. Receivers know only that it has not expired.
- **Coarse states.** An app declares its own passing names, and the standard words for presence
  are few: here, and away. Nothing finer is offered by the SDK.
- **A watcher is seen.** A person who joins a session is visible to the others in it. There is no
  way to watch a session unseen, and none is to be added.
- **Declining and being absent look the same.** A person who is not in a session, a person who
  left, and a person whose device is off are indistinguishable to everyone else.

For a hosted person the doorway holds their key and relays their session. It can read what
passes through it and it can sign as them. That is said to the person where they sign in; it is
not described as unable to read.

## 9. What every app inherits

One session object in the SDK, beside the working-version verbs, in `@elohim/storage-client` and
mirrored in the Rust SDK. Its contract is declared in a seam file beside
`elohim/sdk/working-version.seam.json`.

| The app does | The SDK does |
|---|---|
| Names its passing values in the manifest's `passing` list | Refuses a name that is not declared |
| Sets a value when something changes on screen | Coalesces, caps, signs and sends to the direct peers or the two hosts |
| Reads who is here and their values | Verifies each frame, drops duplicates and expired states, and reports "unknown" when it cannot know |
| Nothing | Renews while the thing is open and in the foreground; stops when it is closed or hidden |

Three things an app must not do, and the SDK gives it no way to do: open its own channel for
presence, run its own timer for it, or store what it receives.

The same rule covers a person's own attention: what a device senses about focus and activity
stays on that device. The session sends only the declared passing values.

## 10. What this depends on that does not exist

Each is a missing node, named and not designed here.

| Gap | Where | Consequence until it lands |
|---|---|---|
| No signed record admits a person to a circle | working-version standard §9 | no session can check who may join; this design cannot be built before it |
| The sync routes ask neither who is calling nor what reach admits them | same | a relayed change is no safer than the route it shadows |
| No session host exists | `elohim/elohim-hub/` holds local projections only | nothing to connect to |
| No session object in either SDK | `storage-client-ts`, `crates/elohim-sdk` | apps cannot inherit the pattern |
| `passing` is not in the app-manifest schema | `elohim/sdk/schemas/v1/manifest/` | a type cannot declare passing values |
| No posture says a peer offers the host role | peer status flags | the standing list in §7 has no source; the posture must be written on change, never on a timer |
| `live-session` is not a grant scope | `api/compute_grants.rs` | a host's work cannot be bounded or accounted |
| Frames cannot be sealed to a circle | blind custody design | hosts must be inside the circle; a household's two hosts share a roof |
| The thin-client connection has no settled wire format | iroh and libp2p complementarity design | track 3 cannot be specified further here |
| The encoding of a frame is not chosen | none | two implementations could not talk to each other |
| A browser cannot connect directly to another browser | same design, listed as undesigned | two hosted people always go through a doorway |
| Node liveness still rides a chain write each minute | plane separation §5, ruling 5 | the ranked list cannot be filtered by substrate liveness until that ruling's order is carried out. Not reopened here |

## 11. Measurements that could overturn this

None has been taken.

1. **Rest.** A peer that declares the host posture and has no session adds zero calls and zero
   timers on the household-at-rest lane. If it does not, the design breaks `idle-is-free` and the
   host role must be started on demand.
2. **Delay.** The time from one person moving a cursor to another seeing it, on the household
   mesh, direct and through a host. A bound is to be set from the first reading, not before.
3. **One host lost.** Kill one of two hosts mid-session. No person's presence or cursor is
   interrupted.
4. **Both hosts lost.** Kill both. Presence reads "unknown"; an edit made by one person reaches
   the other through ordinary sync.
5. **Doubling.** Traffic with two hosts against one, at the declared size. If the cap does not
   hold it inside a stated budget, the second host becomes a standby that receives renewals only.
6. **Signing.** The cost of signing and checking every frame at the cap. If it is felt, frames
   between renewals carry a cheaper check tied to the signed join.
7. **Split view.** How often two people share no host on a real household and a real collective.
   If it is common, the ranking rule is wrong.

## 12. What this does not cover

- **Voice and video.** Different traffic, different hosts.
- **Whether a person is online, outside a session.** Ruled out in §2. If it is ever wanted, it is
  a separate design with its own audience rule, and it starts from a consented relationship, not
  from reach.
- **Whether a node is up.** Ruled in plane separation §5.
- **Merging.** The working-version standard and Automerge.
- **A person's presence across several of their devices.** Each device is its own participant in
  a session. Showing them as one person is the app's choice.

## 13. The story it is measured by

`genesis/a2o/features/federation/local-first-shared-sheet.feature` is the vision feature for
shared working versions, and every step in it is undefined. This design is accepted when that
story has a scenario in which two people see each other in the sheet and one host is lost, and
when `genesis/a2o/features/dataplane/household-at-rest.feature` still passes with the host
posture declared. Neither scenario is written here: a story needs its own blind-reader loop and a
host to run against.

## Sources outside this repository

Read by a research pass on 2026-10-13 from primary sources unless marked.

- Yjs awareness protocol: `github.com/yjs/y-protocols`, `docs.yjs.dev/api/about-awareness`. Whole
  state per update, a clock per client, renewal and expiry.
- automerge-repo ephemeral messages: `automerge.org/docs/reference/repositories/ephemeral/`.
  Sender, session id and count on every message; duplicates dropped by count.
- Phoenix Presence: `phoenix.hexdocs.pm/presence.html`. Replicas with no leader.
- SMPTE ST 2022-7 and the Parallel Redundancy Protocol (IEC 62439-3): send on two paths, take the
  first copy, identified by a sequence number. Read from summaries, not the standards themselves.
- Liveblocks client reference: presence kept apart from stored data; a default send interval of
  100 milliseconds.
- Holochain signals: `developer.holochain.org/build/signals/`.
- Not confirmed from a primary source: how Google Docs or Figma carry cursors.
