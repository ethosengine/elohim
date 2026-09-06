---
id: "backlog-runtime-workspace-stack-idempotent-live-conductor"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "T3 workspace stack (hc-start.sh) is not idempotent over a live conductor and erases its own ports record on failure"
slug: "runtime-workspace-stack-idempotent-live-conductor"
written: "2026-09-06"
author: "story-harvest"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "genesis/a2o/features/delivery/workspace-to-fleet-release.feature"
tags: [runtime-upgrade-propagation, workspace-peer, t3-rung, hc-start, tooling]
cites:
  - app/elohim-app/scripts/hc-start.sh
  - elohim/holochain/local-dev/.hc_ports
  - genesis/a2o/steps/delivery/workspace-to-fleet-release.steps.ts
---

## Chain

`runtime-upgrade-propagation` / between "developer runs `just dev conductor alpha`
(`app/elohim-app/scripts/hc-start.sh NETWORK_PROFILE=join-alpha`)" → "Station 1 mints a
candidate against the workspace peer's own conductor" / **missing node: the T3 workspace
stack is idempotent over an already-live conductor and never destroys its own structural
signal on a failed re-run.**

## Assertion + probe

Run `hc-start.sh` twice in a row against the same live join-alpha conductor. Assertion: the
second run reuses the live conductor (no cold build, no port conflict, no deletion) and
`elohim/holochain/local-dev/.hc_ports` survives both runs unchanged. Probe: `cat
elohim/holochain/local-dev/.hc_ports` before and after the second run — bytes must match;
`workspace-to-fleet-release.steps.ts`'s Background step ("the workspace peer's conductor is
joined to the fleet's network") reads exactly this file and must resolve, not PENDING.

## Current state — two apparatus defects, found live 2026-09-06

Discovered during the first workspace→fleet rung-5 crossing (Objective
`workspace-to-fleet-first-crossing`, iteration 1). Both defects fired in the same session:

**Defect A — cold release build when the fresh binary is right there.** `hc-start.sh`
(`app/elohim-app/scripts/hc-start.sh:482`) hardcodes `STORAGE_BIN="$STORAGE_TARGET_DIR/release/elohim-storage"`
and, when that release binary doesn't exist, cold-builds it
(`app/elohim-app/scripts/hc-start.sh:489`, `RUSTFLAGS='--cfg getrandom_backend="custom"' cargo
build --release`). On the night this was hit, disk was at 91% and a fresh DEBUG binary that
the mesh and `just gate` had already produced sat unused — the script has no path that
reuses a debug binary or a pool-managed release slot; it only knows the one hardcoded release
path. Bridged same-night with non-repo `release/elohim-storage → dev/debug/elohim-storage`
symlinks in the storage and doorway release slots (reversible, not a real fix).

**Defect B — a conductor-start failure deletes the live conductor's structural record.** A
second `hc-start.sh` run against an already-live join-alpha conductor fails at "Could not
start conductor" (`app/elohim-app/scripts/hc-start.sh:445`) because app port 4485 is already
held by the first run's conductor — and that failure path deletes
`elohim/holochain/local-dev/.hc_ports`, the *only* structural signal
`workspace-to-fleet-release.steps.ts`'s `readConductorPorts()` reads. The live conductor is
untouched and healthy; only the record of it is destroyed, which reads as PENDING to the a2o
story rather than "already running." Recovered same-night by hand-reconstructing
`.hc_ports` from the live conductor's actual admin/app ports.

A third, related observation from the same run: `get_admin_port()` (or its caller) recognizes
a live conductor exists but the join-alpha path still attempts to start a new sandbox on the
pinned port 4485 rather than short-circuiting to "already running, reuse it" — this is the
root of Defect B, not a separate bug.

## Also fold in — join-alpha's stock-conductor refusal cites a stale measurement

`app/elohim-app/scripts/hc-start.sh:166-172,232-239` refuses a stock (non-fork) conductor on
`NETWORK_PROFILE=join-alpha` unless `ALLOW_STOCK_JOIN=1`, with the comment citing a
2026-08-28 tx5 measurement ("the difference between the two is the difference between a full
DHT participant and a listed-but-unconnected peer... measured 2026-08-28"). tx5 is gone on
Holochain 0.7 (superseded by kitsune2/iroh); the honest 0.7-era predicate for the refusal is
**conductor-lineage parity with the fleet pin** — `git rev-parse
HEAD:elohim/holochain-conductor` must match what the fleet runs — because alpha is
**two-relay** and the fork carries the cross-relay preflight fix that stock kitsune2 0.7
lacks. The refusal is still correct to keep; its justification comment is stale and should
be re-worded to the lineage-parity + two-relay-preflight reason, not the retired tx5 measure.
The fleet-parity binary is obtainable without a 45-minute fork build: extract layer 25/26
from harbor `elohim-edgenode:conductor-<hc12>` (extracting *all* layers lets the base
layer's stock 0.6 binaries silently overwrite the fork — extract that layer alone).

## Habit served

`runtime-upgrade-propagation` (`elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md`)
— Station 1 of the a2o concern depends on the workspace peer joining cleanly; an apparatus
defect here is friction on every future T3-rung crossing measurement, not just this shift's.

## TODO (integrator)

This entry's `cites:` is a plain path list per the existing convention in this directory
(no fingerprint invented); if `semantic-links`/cite-gen is later required for backlog rows,
run cite-gen over this file rather than hand-writing a fingerprint.
