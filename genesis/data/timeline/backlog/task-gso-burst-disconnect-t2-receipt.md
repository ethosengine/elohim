---
id: "backlog-task-gso-burst-disconnect-t2-receipt"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: household T2 burst/disconnect receipt for the vendored iroh-quinn GSO fix — a storage receiving a near-floor view-federation burst survives the remote peer disappearing"
slug: "task-gso-burst-disconnect-t2-receipt"
written: "2026-09-01"
author: "session-2026-09-01-integrator"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-iroh-quinn-gso-assert-crashes-storage"
  - "backlog-mesh-fixture-fidelity-regimes"
  - "habit:dataplane-convergence"
tags: [iroh, quinn, crash, availability, mesh, fixtures, receipt, delegable]
claimedBy: "codex"
---

**Claimable by any implementation agent. Harness/probe work only — the Rust
cure is already landed and locally green; this task mints its receipt.**

## Why

The vendored `iroh-quinn-proto 0.13.0` backport (Quinn #2167 / `434c3586`)
landed with `just test-iroh` and `just gate elohim-storage` green, but the
parent atom `iroh-quinn-gso-assert-crashes-storage` stays `wip` on its own
DoD: "a household T2 soak/receipt exercises the original burst regime before
this backlog atom becomes `done`." The measured crash shape (2026-08-31
18:26Z): ~891 evidence/head-record view-federation exchanges from a
first-contact peer, then the remote disappearing, aborted the whole storage
process (`untracked_bytes <= segment_size` assert → poisoned-mutex destructor
→ non-unwinding abort). The story-graph node is already minted: *probe =
process liveness plus a successful post-disconnect request*. This is also the
first scenario of the burst regime family in `mesh-fixture-fidelity-regimes`.

## P2P design-gate decision

- **Classification:** Ephemeral (C) — a test-harness probe. No entity, DHT
  entry/link, coordinator function, SQLite/Automerge projection, or HTTP
  route is created; DNA-hash-neutral; zero head-plane cost.
- **Identity/address:** none introduced. The probe addresses running mesh
  peers by the harness's existing roster names/ports.
- **Concern canon:** C0 = harness placement (genesis/a2o scripts beside
  `carried-election-mesh-proof.ts`); C4 = a peer that never crashes AND never
  syncs must read as FAIL, not vacuous PASS (assert the burst actually
  happened — exchange counters moved — before asserting survival); C14 = the
  probe prints the failing leg (no-burst vs crashed vs post-disconnect-dead).
  Remaining concerns n/a: no state machine, authority, ingress policy, or
  persistence is added.

## Scope

1. New script `genesis/a2o/scripts/gso-burst-receipt.ts` (tsx, patterned on
   `carried-election-mesh-proof.ts` conventions) against a running iroh or
   dual household mesh (`just mesh start`, full-corpus seed so view-federation
   has real volume):
   - Drive a burst: restart one peer's storage (`hc-mesh.sh storage-restart`)
     or otherwise trigger a reconcile catch-up so the survivor exchanges a
     large view-federation volume with a specific remote; read the exchange
     counters (view-fed / sync-round metrics on `/metrics` or `/p2p/status`)
     to PROVE volume moved — record the count, target ≥ the hundreds regime.
   - Mid-burst or immediately after, hard-kill the remote (`kill -9` its
     storage PID) — the disconnect leg of the measured crash shape.
   - Assert the survivor: (a) process alive, (b) answers a fresh HTTP request
     (`/db/content` page or `/p2p/status`) with 200 after the disconnect,
     (c) no panic/abort line in its log since the burst started.
   - Exit 0 only when burst-proven AND survived; exit 2 with the failing leg
     named otherwise. `--json` optional.
2. Run it ≥3 consecutive times on the mesh; record counts + outcomes.
3. Close the loop: append the receipt (run id, exchange counts, 3/3) to
   `iroh-quinn-gso-assert-crashes-storage.md` and flip that atom to `done`;
   append a one-line DELTA (NO status flip) to
   `elohim/elohim-storage/.epr-meta/dataplane-convergence.habit.md` and run
   `.claude/scripts/habits-project.py`.

## Disjointness contract

- MAY create `genesis/a2o/scripts/gso-burst-receipt.ts`; MAY edit the parent
  backlog atom, this atom, and append (never rewrite) the habit atom's ledger.
- MUST NOT edit any Rust source, `vendor/iroh-quinn-proto-0.13.0/**`,
  `Cargo.toml`/`Cargo.lock`, `src/p2p/view_federation.rs` or any
  MAX_PAYLOAD/frame-cap constant, `app/elohim-app/scripts/hc-mesh.sh` (the
  late-joiner staging task owns harness edits), any Jenkinsfile, or
  deployment/orchestrator manifests.
- If the burst cannot be provoked from outside the process (counters never
  reach the regime), STOP and report the missing fixture hook as a
  story-graph node (chain / between mesh-harness→burst-regime / missing node)
  instead of adding a Rust hook yourself.

## DoD + verification

- Against a running mesh: `cd genesis/a2o && pnpm exec tsx scripts/gso-burst-receipt.ts` exits 0 with a printed exchange count in the hundreds regime, 3 consecutive runs.
- The kill-leg is real: the transcript shows the remote PID gone and the survivor's post-disconnect 200.
- Parent atom `done` with the receipt pasted; habit DELTA appended; `habits-project.py --check` clean.
- Fleet-shaped confirmation (the next organic fleet burst with zero pod aborts) remains the integrator's watch — do not claim it here.
