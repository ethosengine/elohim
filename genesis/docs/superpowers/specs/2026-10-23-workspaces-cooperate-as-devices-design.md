---
title: Workspaces cooperate as devices — one steward's berth, shared across the steward's devices on the peer runtime
id: workspaces-cooperate-as-devices-design
status: Draft
class: architecture
serves: [devices-share-one-berth, ledgers-fold-once]
date: 2026-10-23
context-tier: disclosed
steward: human:matthew
author: "Claude (orchestrator, Opus 5.5), at the operator's direction"
tags: [berth, devices, roster, eprfs, iroh, channel, integration, leases, ledgers-fold-once, brit, rakia]
graduation-trigger: >-
  Two workspaces bound to the same human claim the steward-scoped resource dev-integration within the
  same second; exactly one holds it, the other's berth refuses naming the holder's device and session,
  and each side's `berth status` lists the other's moorings and leases read from its own fold. With
  the other device stopped, a claim reads `unconfirmed` naming that device and its last-seen age,
  never `free`. Nothing in either flow is carried by git, a doorway or the operator.
cites:
  - genesis/data/timeline/backlog/native-peer-dev-channel-and-collective-memory-seed.md
  - genesis/data/timeline/backlog/integration-sweep-2026-10-23.md
  - "what-every-app-inherits-placement-contract-design | eprfs tier and EprfsStorage this link serves; §12 Scion/GitLab transfer; artifacts plan reuses its view walk | sha256:fe10f9dc966c2e9a | path: genesis/docs/superpowers/specs/2026-10-20-what-every-app-inherits-placement-contract-design.md"
  - "workspace-berth-carrying-capacity-design | berth moor/claim/say records that become the channel vocabulary; container-scoped resources unchanged | sha256:f7f53bb9e7686d19 | path: genesis/docs/superpowers/specs/2026-09-03-workspace-berth-carrying-capacity-design.md"
  - "workspace-stewarded-device-peer | the workspace as a device of its human; the roster binding the PeerEndpoint row extends | sha256:b358395ef1e9f335 | path: genesis/docs/superpowers/specs/2026-08-30-workspace-stewarded-device-peer-design.md"
  - "runtime-artifacts-elected-content | build outputs as content; the second plan on this link | sha256:eaa2716381075140 | path: genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md"
  - .epr-meta/ledgers-fold-once.habit.md
  - elohim/eprfs/.epr-meta/materialize-what-is-read.habit.md
  - "elohim-rust-style-guide | RS-GOV-02/04 paydown and strict-from-birth for eprfs-peer (Global Constraints) | sha256:30c95d2e27e9087f | path: genesis/docs/rust-style-guide.md"
---

# Workspaces cooperate as devices

## 1. The problem, as the 2026-10-23 integration showed it

Development runs in two Che workspaces: one on ethosengine and one on shem. Codex and Claude
sessions work in both. Today the only channel between them is a push to `dev`, with Jenkins
reacting as a side effect. One fast-forward on 2026-10-23 showed every cost of that:

- **Work is trapped.** The Rust style guide, whole finished lanes and about 50 worktrees sat in one
  workspace, invisible to the other, until a bulk landing.
- **Divergence is found at merge time.** About 250 of the files one workspace meant to land were
  already on `dev` in a different form. The landing hit 13 conflicts and the pull on the other side
  hit 8 more, every one in a git-tracked JSONL ledger, and each was settled by hand.
- **Landing is a high-blast-radius event.** The only reconciliation surface was a bulk merge with the
  pre-push gate and CI switched off. Two defects reached `dev` as a result. The orchestrator's new
  Rust Style Gate could never find its baseline, so every dev build refused before dispatch. And an
  unrouted seam registry made every gated push fail, which invites the next bypass.
- **Coordination happens by inference.** To learn what Codex still held, a session read Codex's
  session logs and the process list. Nothing declares who holds which lane.
- **Shared resources collide.** Both workspaces share `dev`, Jenkins dispatch and the alpha fleet,
  with no shared view of who is using them. Any `[build:edge]` push from either side deploys whatever
  is on `dev`. That is why hosted provisioning had to stay held.

The operator's direction: integration should happen continuously, over a channel on our own peer
runtime that both workspaces coordinate through. The workspaces take part as devices bound to their
human, with the operator as steward of record. Done that way, two workspaces add berth capacity
instead of piling up divergence.

## 2. What this composes; it invents no transport and no record kind

| Piece | State today | Role here |
|---|---|---|
| **berth** (`genesis/agentic/bin/berth`) | Moor, claim, release, say, ledger. Every record is already in the field shape of its protocol home (lease = REA `use` commitment, note = witnessed interaction). Local to one container. | The vocabulary of the channel. Its records are what crosses. |
| **ledgers-fold-once** (red) | Its first move is named: berth emits every record kind as a FlowEvent on the host's SidecarFlowStore. | It makes berth's records content-addressed events that one device can hand another. |
| **Roster** (`.eprfs/status/participants/<handle>.jsonl`, `ParticipantRow`) | Genesis, Binding and Contest rows, signed by device did:keys, carried by git. | Who is admitted. One new row kind binds a device to its peer endpoint. |
| **eprfs tier** (`eprfs-serve::TierEndpoint`, `eprfs-core::ReachGate`) | Built and tested, with no transport route. | Serves a device's channel tree to admitted peers, so a fold is a tier read. |
| **iroh** (0.92 line, already pinned by elohim-storage) | Endpoint, ALPN routing and gossip in use for the storage dataplane. | The link between devices, with QUIC encryption and authenticated node ids. |

This answers question 5 of the seed (`native-peer-dev-channel-and-collective-memory-seed`). The seed
named convergence of a root the operator authors as a rudimentary channel. Its limits were these: not
private, about 75 s per update, and every message a head. This design keeps the shape (a signed root
per device, converged by its peers) and drops the Holochain head entirely. Only the steward's admitted
devices can connect, an update is one gossip announce plus one tier read, and no DHT head is minted.

## 3. Design

### 3.1 Admission: a device's peer endpoint is a roster row

A new `ParticipantRow` variant:

```text
PeerEndpoint { handle, chain_root, device, node_id, nonce, signature }
```

`device` signs it with its did:key. The row is valid when `device` is the handle's Genesis
`chain_root` or the `controller` of a valid Binding row, and no valid Contest names that binding. The
newest valid PeerEndpoint row for a device supersedes older ones, which is how a key rotates. A Contest
on the device's binding revokes the device: its node id stops being admitted and its heads are ignored
from the next fold onward. Every lease and lane claim already folded from that device also drops
out of the steward lease view once the Contest is folded, so a revoked device cannot keep a shared
resource pinned. Each device reads admission from the roster file in its own checkout. A Contest
signed on device A and written there takes effect on A at once, with no pull. It reaches the other
devices with A's next push, and the revoked device's lease has already left A's view, so that push is
not blocked. The admitted devices drop the revoked device's gossip and tier connections. It therefore
sees them as silent, and its own steward-scoped claims come back `unconfirmed`. Revocation withdraws a device's standing in coordination; it does not revoke the device's repository credentials, which are withdrawn separately. Berth cannot stop a revoked device that chooses to override. The row travels by git, as Binding rows already do.

The device generates its iroh secret key and keeps it at `$XDG_CONFIG_HOME/epr/peer/iroh.key`
(mode 0600, on the persistent `/nix/xdg/config`). The key is never committed, never copied and never
sent. A rebuilt workspace mints a new key and writes a new PeerEndpoint row.

### 3.2 The device peer

By operator ruling (2026-10-23), the peer lives in its **own crate**, `eprfs-peer`, in the eprfs Cargo
workspace. elohim-storage does not depend on eprfs-serve. `epr-cli` consumes the crate through two verbs:

- `epr peer serve` is the long-running daemon. The devfile declares it as an on-demand command, never
  in postStart.
- `epr peer status` prints one line per admitted device: last head, its age, and how many events were
  folded.

The daemon opens one iroh endpoint with two protocols:

1. **ALPN `/elohim/eprfs-tier/1.0.0`** serves a `TierEndpoint` over the device's own local eprfs store.
   Its gate, `StewardDevicesGate: ReachGate`, admits a `Requester` only when the requester's
   authenticated node id maps to a valid PeerEndpoint row for a device of the same `chain_root`.
   Everyone else gets the endpoint's existing uniform refusal: a stranger cannot tell a held root from
   an unknown one.
2. **Gossip topic `blake3("elohim/steward-channel/1/" ‖ chain_root)`** carries `ChannelHead`
   announcements. Only admitted node ids are taken as gossip neighbours, and a head whose signature
   does not verify against its device's did:key is dropped and counted.

Discovery comes from the roster (node ids) plus iroh's address discovery through the configured relay
(`EPR_PEER_RELAY_URL`, with the same semantics as storage's `ELOHIM_IROH_RELAY_URL`). Che workspaces
are pods in one cluster, so direct addresses usually connect and the relay is the fallback.

### 3.3 The channel root and the fold

Each device keeps one eprfs tree: `channel/events/<event-cid>`, where each leaf is a FlowEvent's
canonical dag-cbor bytes. Only events whose kind is on the **crossing allow-list** are placed in it.
The allow-list is default-deny and declared in `genesis/agentic/pool-policy.json` under
`berth.channel.crosses`:

- **Crosses:** `dev-berth-events@1`, meaning berth's moor, unmoor, claim, renew, release, override,
  say and shed records.
- **Never crosses:** recall sessions and their receipts, private memory, session transcripts, any
  secret or environment value, and anything not on the list. Thoughts are private; only what is said
  or done crosses. A `berth say` note is said on the record by definition, so it may cross.

After each change (debounced to 1 s), and every 30 s regardless, the device signs and announces:

```text
ChannelHead { device, root, seq, at, folded, signature }   # seq strictly increasing per device
                                                           # folded: {device → highest seq folded}
```

`folded` is the device's acknowledgement vector: for each other admitted device, the highest `seq`
it has folded from that device. A device that folds a newer head re-announces its own head
(debounced to 1 s), so an acknowledgement reaches the sender within a couple of seconds while both
devices are up. No decision in this design compares two devices' clocks.

To fold, a device that receives a newer head from an admitted device asks that device's tier for the
diff between the last folded root and the new one. It verifies each event leaf against its CID and
appends each event to its own SidecarFlowStore with provenance `via: <device>`. Appending is
idempotent by event CID, so a replay or a duplicate announce is a no-op. A head with a lower `seq` than
the last folded one for that device is refused as a rollback and counted. One verb does the fold:
`epr flow fold --from <device> --root <cid>`. Today no import path exists for flow events, and this
is it.

### 3.4 Steward-scoped berth resources

`pool-policy.json berth.resources` entries gain a `scope`: `container` (the default) or `steward`.

- **Container-scoped** resources (`mesh`, `cargo`, `disk-heavy`) are unchanged and never consult
  peers. Each workspace's slots stay its own. This is where the capacity gain comes from: two
  workspaces are two sets of slots, and only what is genuinely shared coordinates.
- **Steward-scoped** resources are `dev-integration`, `jenkins-dispatch`, `alpha-fleet`, and lane
  claims named `lane:<id>`.

A claim on a steward-scoped resource has a lifecycle:

1. **Pending.** The claim writes its lease as today. The lease is a crossing event, so the device's
   next head (sequence `s`) carries it to the other devices.
2. **Confirming.** The claim command blocks for up to `berth.channel.confirm_wait_s` (10 s) until
   every other admitted device has announced a head whose `folded` entry for this device is at
   least `s`. That is proof that the device has seen the claim.
3. **Settled.** The claim is **effective**, and the command exits 0, when every other device has
   acknowledged `s` and no folded lease held by another holder overlaps it, unless the tie-break
   below gives this claim precedence. Otherwise it exits 3 and releases its pending lease:
   `held by <device>/<session> since …` when an overlapping lease wins, and
   `unconfirmed: <device> last seen <age> ago` when a device did not acknowledge in time.

Two claims made before either device had folded the other's are concurrent. A deterministic
tie-break settles them: the earlier `claimed_at` wins, then the lexically lower device did:key. Both
devices compare the same recorded values, each claim's own `claimed_at` as its device stamped it, so
they reach the same verdict. A device whose lease loses after it was already effective, because a
concurrent claim arrived late, reports the refusal on that fold and releases.
`berth.channel.freshness_s` (90 s) only sets when `berth status` reports a device as silent.

A refused or unconfirmed claim exits 3 and names its reason, `held by <device>/<session> since …` or
`unconfirmed: <device> last seen <age> ago`. An absent peer is never read as a free resource.
`--offline-ok` takes the lease anyway and records a `kind: override` row. It overrides only an
`unconfirmed` refusal, never `held by`. The overriding lease is an ordinary lease: it binds until it is
released or its ttl expires. Two overrides taken while the devices could not reach each other are
concurrent claims, and the tie-break settles them. An agent session may pass `--offline-ok` on its
own judgment. The safeguard is the record, not a gate: the override row crosses on
reconnect, and the other device's berth then refuses its own sessions on that resource, naming the
overriding device and session.

When only one device is admitted, the set of other devices is empty and every steward-scoped claim is
effective at once. That is local-first: the design works on one device and adds peers without making
them required.

### 3.5 Integration over the channel

- The pre-push hook claims `dev-integration` (class `verify`, ttl 1800 s) for a push to `dev` or
  `main`, and releases it when the push ends. A refusal stops the push and names the holder.
- A commit carrying `[build:edge]` also needs `alpha-fleet`. A `[build:*]` tag also needs
  `jenkins-dispatch`. The hook holds every claim only for the duration of the push. A session that
  watches a build or deploy through to the end takes its own explicit `berth claim` on that resource
  for as long as it watches. A build or deploy that no session watches is protected only while its push lasts; making CI itself a participant is `ci-as-steward-channel-participant`. The hook never overrides on its own. It honours `BERTH_OFFLINE_OK=1`, set by the session that pushes, as an explicit `--offline-ok` on each of its claims, with the same `unconfirmed`-only scope and the same override record.
- `berth claim lane:<id> --note '<intent>'` declares who holds a lane before the work starts, so no one
  has to infer it from process lists afterwards.
- `berth status` adds a section for the other devices: their live moorings, their steward-scoped
  leases and their lane claims, all read from the local fold.

This turns landing into a small, continuous act with the gates on. The pre-push hook already runs
the gates. What it lacked was knowing whether another device was mid-landing.

### 3.6 Out of scope here

- **Artifacts.** That covers sharing build outputs through the same tier ALPN, the rakia input key,
  and brit build records. It is the second plan on this link and is recorded in §8.
- **Jenkins as a channel participant.** Build verdicts would arrive as events and the orchestrator
  would honour `jenkins-dispatch`. Backlog: `ci-as-steward-channel-participant.md`.
- **Jobs across workspaces.** This is the peer-executed stage over the channel instead of the DHT.
- **Sparse source and overlays.** These belong to the placement contract (§12.2).

## 4. P2P design gate

| Entity | Class | Source of truth | Identity | Head-plane cost |
|---|---|---|---|---|
| Berth record as FlowEvent (moor, claim, say, …) | **Private (B)**: the steward's own record, validated by no peer outside the steward's devices | the authoring device's SidecarFlowStore | CID of canonical dag-cbor (`bafyrei…`) | none: no DHT entry, no head |
| Device channel tree | **Ephemeral (C)**: a projection of the device's own crossing events | rebuilt from the SidecarFlowStore and the allow-list | eprfs root CID | none |
| `ChannelHead` announce | **Ephemeral (C)**: gossip, superseded by the next one | the announcing device | (device, seq) plus a signature over the root CID | none |
| `PeerEndpoint` roster row | **Attested-Private (B2)**: one human's device fact, verifiable by any of the steward's devices from its signature | the git-tracked roster | agent-composite (device did:key, node id) | none |
| Steward-scoped lease view | **Ephemeral (C)**: derived from folded lease events | rebuilt from the fold | n/a | none |

No Holochain entry type is added and no DNA hash moves. Item counts are at most hundreds of berth
records a day per device, and they never enter the head plane. They are bounded by the fold store's
rotation, which is the open question in `witness-ledger-rotation-is-an-unsolved-design-question.md`;
this design adds no new rotation duty. Stage: dev-system only, so the network stakes stage does not
apply. Floor-protected costs: none are cheapened. A forged head or a contested device is refused at
every stage.

Concern canon: **C0 (reach)** is answered by `StewardDevicesGate` and the tier's uniform refusal.
**C2 (head order)** is answered by per-device `seq` with rollback refused. **C4 (membership)** reuses
eprfs-core's `Membership::of` through the tier unchanged.

## 5. Review focus: what tests must pin

1. **Offline peer.** A claim while the other device is down reads `unconfirmed` and names it. It never
   reads as free.
2. **Simultaneous claim.** Both devices claim within one gossip interval. Exactly one stays effective
   after both folds, by the tie-break.
3. **Revoked device.** After a Contest on its binding, its heads are ignored and its tier connections
   refused, with no restart of the other device.
4. **Replay and rollback.** A replayed head, or a lower `seq`, folds nothing and is counted.
5. **Allow-list leak.** An event of a kind not on the list never appears in the channel tree, even if
   it is in the SidecarFlowStore.

## 6. Habit and graduation

A new habit, **`devices-share-one-berth`**, is declared in `genesis/agentic/.epr-meta/` and born red:

> A steward-scoped berth claim made on one of the steward's devices is effective only after every
> other admitted device's current head is folded. Otherwise it is refused naming the holder, or
> `unconfirmed` naming the silent device. A push to `dev` from one device is refused while another
> device holds `dev-integration`.

- **checks:** `a2o @concern:devices-share-one-berth`
  (`genesis/a2o/features/devflow/devices-share-one-berth.feature`, two devices in one container), and
  the cross-workspace scenario tagged `@requires:shem`.
- **retire-when:** the lease view and the fold are served by the native peer runtime (lvi or storage)
  with no berth-specific code.

This design also discharges `ledgers-fold-once`'s first move. That habit stays red until the
harvesters fold too.

## 7. Global constraints

- **Rust style guide** (`genesis/docs/rust-style-guide.md`). The new crate `eprfs-peer` enrolls as
  `rust-style-strict@1` from its first commit. Every `allow` or `expect` carries a reason (RS-GOV-04).
  A touched crate with debt pays down one finding (RS-GOV-02): `epr-cli`, `epr-rea`, `eprfs-serve`.
  Expected failures are explicit `Result`s (RS-API-06), and no `unwrap` appears outside proven
  invariants (RS-IMP-03).
- **Toolchain** is pinned at 1.96.1. `iroh` and `iroh-gossip` use the same `=0.92` pins as
  elohim-storage, so the tree holds one iroh line.
- **Keys never cross.** No key, keystore or credential is copied between devices, by any verb.
- **Thoughts private.** The allow-list is default-deny, and recall and session stores are never
  placed in the channel tree.
- **No `kubectl`.** Cluster networking is observed, not configured, from a workspace.

## 8. The second plan on this link: artifacts

The second plan reuses §3.1 and §3.2 unchanged. A build output becomes a small eprfs tree (its
closure) placed in the device's store and served by the same ALPN. Its rakia input key is the CID of
`{project, recipe, toolchain, rustflags, files: path→bafkrei…}`, read from the gate project's
`inputs.sources` and `Cargo.lock`. Its record carries the output CID, the builder device and session,
and the cargo-bridge Process. The record is a crossing event on the same allow-list. Fetching an
artifact is a view walk of the peer's tree, which is scenario 2 of `materialize-what-is-read`.

By operator ruling (2026-10-23), agents may commit the key and record types to the rakia submodule and
move its pin under the pin-attestation gate. That plan's graduation: shem runs `just mesh preflight`
on binaries built on ethosengine with no cargo invocation.

## 9. Rulings recorded (operator, 2026-10-23)

- The peer is a separate crate (`eprfs-peer`). elohim-storage keeps eprfs-serve as a
  dev-dependency only (rail R-E3 unchanged).
- Agents may commit to rakia and move its pin, subject to the pin-attestation gate.
- The channel comes before artifacts. The operator's account of the 2026-10-23 integration makes
  coordination the payoff.
