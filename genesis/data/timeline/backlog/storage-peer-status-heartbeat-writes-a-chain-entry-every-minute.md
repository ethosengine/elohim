---
id: "backlog-storage-peer-status-heartbeat-writes-a-chain-entry-every-minute"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every storage peer writes a new PeerStatus entry and link to its infrastructure chain every 60 seconds whether or not anything changed, and the latest-status read fetches every link the peer ever wrote"
slug: "storage-peer-status-heartbeat-writes-a-chain-entry-every-minute"
written: "2026-10-13"
author: "claude (performance sprint, operator-directed)"
status: "open"
priority: "medium"
severity: medium
nodes: [household, alpha]
relatedNodeIds:
  - "habit:idle-is-free"
tags: [elohim-storage, infrastructure-dna, peer-status, heartbeat, chain-growth, performance, perf-cpu, perf-storage, idle-is-free, plane-custody, unit-peer, phase-steady, lane-background]
cites:
  - elohim/elohim-storage/src/heartbeat.rs
  - elohim/holochain/dna/infrastructure/zomes/infrastructure/src/peer_status.rs
  - genesis/data/timeline/backlog/arch-dataplane-borrows-backlog.md
  - genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md
  - elohim/elohim-storage/.epr-meta/idle-is-free.habit.md
---

**What (code read 2026-10-13; one call count measured).** The storage heartbeat ticks every 60 s
(`heartbeat.rs:174`) and calls `infrastructure::record_peer_status` on each tick (`heartbeat.rs:310`). The
coordinator commits a new `PeerStatus` entry and a new `AgentToPeerStatus` link every time, with no comparison
against the previous snapshot (`peer_status.rs:24-37`). Nothing deletes either. That is 2,880 chain actions a day
per peer at rest, published and validated by every holder.

The read, `get_latest_peer_status_for_agent`, fetches all `AgentToPeerStatus` links from the agent, sorts them by
timestamp and takes the last (`peer_status.rs:47-58`). Its cost grows with the peer's whole heartbeat history.

**Measured.** In a settled 300 s household window the write is 5 of each peer's 23 to 27 conductor calls, and the
read is 2 to 4 (`genesis/a2o/reports/recovery/idle-call-floor-20261013/shift-measure-consent-answering-1.txt`).
The readers were slowed this sprint (`8bf53be67`); the publisher was not touched.

**Not measured.** The write's share of idle conductor CPU; how many links a long-lived peer's read returns; how
long that read takes on alpha. The 2026-10-03 note in `arch-dataplane-borrows-backlog.md` (Observation 1, the
heartbeat leg) names holders validating these ops as the leading candidate for the idle wasm cost and could not
confirm it.

**Why it is the wrong shape.** A liveness snapshot is latest-wins between a node and whoever asks about it. Written
to a chain on a timer it becomes a permanent record nobody reads, and every one of them makes the next
authorisation lookup on that chain slower (`conductor-cap-grant-scan-per-zome-call`).

**Constraint on any cure.** The tick's successful zome call also vouches for the conductor connection inside a 90 s
window (`HEARTBEAT_VOUCH_WINDOW`, `heartbeat.rs:28`). A tick that writes nothing must still vouch, or fall through
to the ping the supervisor already has.

**Already ruled, and the order is binding (corrected 2026-10-14).** This entry first listed "write only on change"
as a cure to take first. That is wrong. `genesis/docs/superpowers/specs/2026-10-03-plane-separation-design.md` §5
ruled on this on 2026-10-03:

- Ruling 5: the heartbeat splits in two. Posture (lifecycle, pool flags, archetype) is a standing declaration
  written on change. Liveness moves to the substrate presence that already exists (`peer_liveness`, 35 s). The
  order: arm that presence on iroh and define the unarmed fallback; move shard placement, the resilience card and
  network posture to read posture and substrate liveness; only then change the writer. Changing the writer first
  drops every healthy peer out of placement after 900 s.
- Ruling 6: the latest-status read is bounded regardless of the above. Coordinator-only.

Its fleet reading (§14, 2026-10-04) puts the cost in validating and reading back, not in writing:
`get_latest_peer_status_for_agent` was the top zome function by time fleet-wide (3.3 calls a minute at a mean of
2.6 s), and `record_peer_status` itself was 25 ms.

**State on 2026-10-14.** Neither ruling is built. The read still fetches and sorts every link
(`peer_status.rs:47-58`); the writer still ticks every 60 s.

**Next slice, chosen by the operator 2026-10-14 as the sprint's next story:** ruling 6, the bounded read. It is a
coordinator hot-swap with no DNA hash move. The writer waits for ruling 5's first two steps.

**Ruling 6 cannot be met coordinator-only on the current fork (Codex design read, 2026-10-14, code only, nothing
measured; `genesis/local-dev/idle-call-floor-2/codex-design-bounded-latest-status-read.md`).**

- HDK 0.7.0's `LinkQuery` has tag-prefix, `after`/`before` and author filters but no limit or "latest one".
- On the checked-in conductor fork those filters are applied in Rust after the conductor has selected and decoded
  every live link action under the base (`holochain_data/src/dht/inner/action.rs:663`,
  `holochain_state/src/dht_store/reads.rs:1185`); the authority path likewise loads all creates by base first
  (`holochain_cascade/src/authority.rs:152`). `count_links` uses the same machinery. So a timestamp window or a
  bucket tag shrinks what the zome receives, not what the conductor reads: the work stays proportional to history.
- A bucket-specific base (a different base hash per hour or day) does bound the conductor's work, since the query
  constrains `base_hash` on an index, but a new reader then cannot see an old writer's links without falling back
  to the agent base, which brings the unbounded scan back.
- A window also changes the meaning of `None` from "never published" to "nothing in the window". Fan-in keeps
  last-known snapshots regardless of age, and the 900 s staleness rule lives in the consumers and is configurable
  (`p2p/mod.rs:2280`, `api/resilience.rs:76`), so a fixed window in the coordinator would change their behaviour.
- The DNA's peer-status conductor test is an ignored placeholder that panics; there is no executable round trip.

So the three requirements (bounded work, old writers readable during rollout, an arbitrarily old last status still
returned) cannot all hold with a coordinator-only change. The missing primitive is an indexed latest-live-link
lookup, or a maintained latest-state index, inside the conductor: a fork change, not a coordinator hot-swap. That is
a decision for the operator, and this story is not buildable as "coordinator-only" until it is made. A narrower
timestamp-window read is specified in the design file as an option if a "recent status" meaning is chosen
explicitly; it would not complete ruling 6.

**Done when.** The latest-status read's cost is shown on the household not to grow with the peer's history
(ruling 6); and, later, a resting peer's infrastructure chain grows by a declared number of actions a day that is
not a multiple of its uptime in minutes (ruling 5).
