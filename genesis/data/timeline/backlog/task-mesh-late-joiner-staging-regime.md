---
id: "backlog-task-mesh-late-joiner-staging-regime"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: the household mesh stages an organic late joiner — a peer joining a RUNNING mesh is a harness verb, closing the late-joiner discovery receipt at process level"
slug: "task-mesh-late-joiner-staging-regime"
written: "2026-09-01"
author: "session-2026-09-01-integrator"
status: "open"
priority: "high"
jobs: [elohim-app]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-late-joiner-peer-discovery-refresh"
  - "backlog-late-joiner-peer-discovery-boot-only-board"
  - "backlog-mesh-fixture-fidelity-regimes"
  - "habit:dataplane-convergence"
tags: [mesh, fixtures, late-joiner, discovery, receipt, delegable]
claimedBy: "codex"
---

**Claimable by any implementation agent. Harness work — the Rust cure
(recurring board refresh in `doorway_bootstrap.rs`) is landed and unit-proven;
this task builds the staging capability its fleet-shaped receipt needs.**

## Why

`task-late-joiner-peer-discovery-refresh` closed with a unit test and this
explicit residue: "fleet evidence is not claimed because the household fixture
cannot yet model organic late join." `mesh-fixture-fidelity-regimes` names the
same gap as regime 3: "mesh peers boot together; a peer joining a running mesh
between rolls is not a scenario the harness can stage." The doctrine is the
fleet CONFIRMS, never discovers — so the receipt for the late-joiner cure must
be mintable on the mesh, at real-process level, on demand.

## P2P design-gate decision

- **Classification:** Ephemeral (C) — dev-harness orchestration. No entity,
  DHT entry, coordinator function, projection, or HTTP route; DNA-hash-neutral.
- **Identity/address:** the late joiner is an ordinary mesh peer with the
  roster identity the harness already mints; no new identity namespace.
- **Concern canon:** C0 = harness placement (`hc-mesh.sh` verb + a2o probe);
  C4 = "discovered" must be a positive read (the running peers' books gained
  the joiner's NodeId) — never inferred from the joiner booting cleanly;
  C6b = the verb is idempotent per peer name (re-staging an existing peer is
  a refusal, not a duplicate). Remaining concerns n/a — no authority, wire
  type, or persistence is added.

## Scope

1. `app/elohim-app/scripts/hc-mesh.sh`: an additive verb (suggested name
   `join-peer <name>`) that brings up ONE additional storage peer against an
   ALREADY-RUNNING mesh — same binary slot, profile overlay, and env
   derivation as the boot-time roster (reuse the existing per-peer start
   path; do not fork a second peer-launch code path). It must not restart or
   reconfigure any running peer. Wire it through the `just mesh` verb map
   only if that requires no justfile surgery beyond the existing passthrough.
2. New probe `genesis/a2o/scripts/late-joiner-receipt.ts` (tsx): with the
   mesh running and warm (peers hold non-empty books), stage the late joiner,
   then poll the RUNNING peers' observable surface (`/p2p/status` iroh fields
   / metrics) until each reports the joiner known — WITHOUT any restart. Exit
   0 with time-to-discovery per peer; exit 2 naming which peer never learned
   the joiner within a bounded deadline (the announce cadence × 3 is a
   reasonable bound). The old boot-only-board behavior must fail this probe —
   state in the transcript why the pass is attributable to the recurring
   refresh (books warm before staging).
3. Close the loop: append the receipt to
   `late-joiner-peer-discovery-boot-only-board.md` (and mark the regime-3 row
   of `mesh-fixture-fidelity-regimes.md` staged), append a one-line DELTA (NO
   status flip) to
   `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md`, and run
   `.claude/scripts/habits-project.py`.

## Disjointness contract

- MAY edit `app/elohim-app/scripts/hc-mesh.sh` (additive verb only), create
  `genesis/a2o/scripts/late-joiner-receipt.ts`, and edit the named backlog
  atoms + append to the habit ledger.
- MUST NOT edit any Rust source (the cure is landed), any other
  `hc-mesh-*.sh` sibling, `hc-start.sh`, any Jenkinsfile, or
  deployment/orchestrator manifests. The GSO-receipt task owns
  `gso-burst-receipt.ts`; do not touch it.
- Do not change boot-time roster behavior: `just mesh start` byte-for-byte
  identical flow for existing peers is part of the DoD.

## DoD + verification

- On a warm mesh: `join-peer` stages a fourth peer; `late-joiner-receipt.ts`
  exits 0 with per-peer discovery times, 3 consecutive runs (fresh joiner
  name each run, or clean teardown between).
- `just mesh start` + `just test mesh` on the untouched roster remain green
  (no regression from the harness edit).
- Backlog atoms updated; habit DELTA appended; `habits-project.py --check`
  clean.
- Fleet-shaped confirmation (an organic late join on alpha observed without
  a fleet restart) remains the integrator's watch — do not claim it here.
