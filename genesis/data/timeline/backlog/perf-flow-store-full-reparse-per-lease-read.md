---
id: "backlog-perf-flow-store-full-reparse-per-lease-read"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Every steward lease read, channel projection and fold re-parses the whole flow store"
slug: "perf-flow-store-full-reparse-per-lease-read"
written: "2026-10-24"
author: "claude-opus-5-5"
status: "backlog"
priority: "medium"
area: "eprfs-peer / epr-cli / berth"
relatedNodeIds:
  - "habit:devices-share-one-berth"
  - "backlog-ci-as-steward-channel-participant"
tags: [performance, perf-io, perf-latency, perf-scale, eprfs, berth, steward-channel, trustful-self, friction-mechanical, risk]
shift_objective: |
  Make a steward lease read cost proportional to what changed since the last read, not to the
  flow store's whole history: `epr peer leases` (and the daemon's projection and fold) read the
  flow store incrementally from a recorded position, or the daemon serves the lease view it already
  holds, and the channel tree carries a window (live leases and moorings) instead of every crossing
  event ever written. Done when a claim's settle on a store of 20,000+ events costs a small,
  measured fraction of today's ~0.2 s per read, and the devices-share-one-berth story still passes
  10/10.
---

# Every steward lease read re-parses the whole flow store

## What it costs

Three readers on the steward channel each call `store.events()` over the entire
`.eprfs/status/flows.jsonl`:

- `project_channel` (`elohim/eprfs/eprfs-peer/src/channel.rs`), which the daemon runs after every
  flow write;
- `lease_events` (`elohim/eprfs/eprfs-peer/src/lease_view.rs`), which `epr peer leases` runs on every
  call;
- `folded_from` (`elohim/eprfs/eprfs-peer/src/fold.rs`), which each fold runs.

Measured in the final review of `feat/workspaces-cooperate-as-devices` (2026-10-24): the main
checkout's flow store held 19,983 lines (13.1 MB), and a worktree's held 14,422 lines (18.6 MB). One
`epr peer leases` took about 0.2 s. Every berth claim on a steward-scoped resource reads the view
until it leaves pending, for up to `confirm_wait_s` (10 s). A claim therefore parsed the store a few
dozen times while berth polled every 250 ms.

The channel tree also carries every crossing event ever written. A mature device's first fold lists
thousands of leaves, and it completes only across successive heads (1,024 per head, about 30 s
apart).

None of this breaks safety. Every partial state fails closed. The cost grows with history, though,
and the flow store is append-only, so it grows without bound.

## Bounded now (2026-10-24)

- berth polls the view once a second, so a claim costs at most about 11 reads (`POLL_S`,
  `genesis/agentic/bin/berth`).
- While a projection keeps failing on one state of the store, the daemon retries after 1 s,
  doubling to 30 s. A change to the store resets the wait. It no longer re-parses the store on
  every 250 ms tick (`ProjectionRetry`, `daemon.rs`).

These steps cap the rate. Each read still costs the whole store.

## Options

1. **Incremental read.** `HeadTable::own_log` already proves which store a position counts, by
   naming the CID of the last event it counted. A reader that keeps its own position (byte offset
   plus last CID) can resume from there. A mismatch means the store was reset or replaced; then it
   reads from the start, as it does today.
2. **The daemon serves the view.** The running daemon already holds the head table and sees every
   flow write. It could keep the lease replay in memory and answer `epr peer leases` over a local
   socket. A reader without a daemon falls back to the file read. That fallback is also the case
   that I2 now names (`epr peer serve` not running).
3. **A window on the channel tree (rotation).** The lease view needs only live leases and
   moorings, plus enough history to order a device's records (`dseq`). A tree that carries a window
   keeps first folds small. This is the rotation question that spec §4 left open, and it needs a
   rule for when a record may leave the window (a lease past its ttl, a released lease that every
   admitted device has acknowledged).
4. **A lease-record index.** A sidecar keeps only the `dev-berth-events@1` records, with their
   store positions. This is cheaper than option 1 but is one more derived file to keep coherent.

Options 1 and 2 compose. Option 3 changes what crosses, so it goes through the p2p-design-gate
before any code.

## Trigger

Re-rank this entry when either of these happens:

- one `epr peer leases` takes more than 1 s on a workspace;
- a claim's settle reports `unconfirmed … did not acknowledge within` because its own reads ran
  out the window.
