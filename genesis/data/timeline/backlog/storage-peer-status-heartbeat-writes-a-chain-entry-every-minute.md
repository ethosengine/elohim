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

**Cure candidates, none built, none chosen.**

- Write only when the snapshot differs from the last one written, plus a slow keep-alive (hourly, say) so a reader
  can bound staleness. Coordinator-or-storage-only; no DNA hash move. Readers treat "no newer snapshot" as
  unchanged within the keep-alive bound.
- Carry liveness on the substrate connection the peers already hold and notarise only transitions (came up,
  degraded, leaving).
- Independently: have the read ask for the newest link only, or keep one link and replace it, so read cost stops
  growing with history. Old links already written stay on the chain either way.

**Done when.** A resting peer's infrastructure chain grows by a declared number of actions a day that is not a
multiple of its uptime in minutes, measured on the household, and the latest-status read's cost is shown not to
grow with history.
