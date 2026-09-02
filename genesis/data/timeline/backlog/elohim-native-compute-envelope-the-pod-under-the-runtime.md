---
id: "backlog-elohim-native-compute-envelope-the-pod-under-the-runtime"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "The elohim-native compute envelope — one primitive under the runtime that runs, quotas, accounts for, listens to, and witnesses the processes a peer is made of (conductor, storage, doorway, devspace sandboxes), the same on a watch, a household box, a rack blade, and inside a k8s pod — so the low-level powers we borrow from kubelet today (supervision, cgroups, logs, describe, restart policy) are the peer's own; the enabling story for the death witness"
slug: "elohim-native-compute-envelope-the-pod-under-the-runtime"
written: "2026-09-02"
author: "shift 2026-09-02T02-20-land-rung5-batch (operator-directed)"
status: "refined"
priority: "high"
domain: "D-runtime-operations"
roadmap_rung: "seam map 3.2 OS/packaging + 3.3 runtime/footprint — the primitive both seams assume and neither owns; generational shift (device spectrum smartwatch → rack)"
relatedNodeIds: []
tags: [compute-envelope, pod, supervisor, kubelet-parity, cgroups, resource-quota, compute-rea, death-witness, process-manager, lvi, steward-node, device-spectrum, generational-shift, self-healing]
cites:
  - genesis/docs/content/elohim-protocol/architecture/2026-06-21-elohim-seam-map-concern-routing.md
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - steward/node/src/pod/mod.rs
  - steward/node/src/pod/compute_rea.rs
  - elohim/lvi/CLAUDE.md
  - genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml
  - genesis/data/timeline/backlog/death-witness-runtime-harvests-a-dying-conductors-last-words.md
  - genesis/data/timeline/backlog/alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes.md
---

## Why (operator, 2026-09-02)

The death-witness item goes low: pipes, exit statuses, `/proc`, ring buffers. Before it is
built against whatever happens to hold the conductor's pipes today, name the primitive it
should sit on. Presumably we move to something pod-like that owns the low-level compute
primitives — supervision, quota, logging, reporting — and the stack beneath us has to be fit for
the generational shift the protocol is making (the device spectrum, not a cluster). If a
capability is visible to `kubectl describe`, our native peers should have it themselves.

## What exists today — three partial envelopes, none of them the primitive

| Where | What it owns | What it lacks |
|---|---|---|
| `elohim/elohim-storage/src/conductor/process_manager.rs` | spawns the conductor, owns its pipes, readiness poll, nice; after 2026-09-02: `try_wait`, exit status + tail | one child only; no quota; no accounting; no witness persistence; lives inside storage |
| `steward/node/src/pod/` | a **cluster operator** (monitor → analyze → decide → execute) with `compute_rea.rs` (compute as REA events), admission, capacity, consensus for a rack's blades | is not a process supervisor; hub-internal; assumes the processes already run |
| `elohim/lvi` (`lvi-actuator`) | process supervision + sandbox containment for a devspace: hard cgroup quota (`--memory --cpus --pids-limit`, disk, `--network=none`), TTL, co-resident safety | devspace-only by charter; must not become the peer's runtime |
| k8s (`_edgenode-consolidated.template.yaml`, conductor StatefulSets) | the real envelope on alpha: restart policy, probes, cgroups, `describe`, `logs --previous` | belongs to the hyperscaler plane; the household has none of it; and it is where the 2026-09-02 diagnosis had to be done |

The seam map (3.2 OS/packaging, 3.3 runtime/footprint) routes concerns *through* an envelope
it never names. That is the gap.

## The primitive (envisioned — to be brainstormed, not designed here)

An `elohim-pod` (name provisional) is the unit a peer is made of: a declared set of
**processes** (conductor, storage, doorway, sidecars, devspace sandboxes) under one supervisor
that owns, for each:

- **lifecycle** — spawn, readiness, liveness, restart policy with witnessed decisions (a dead
  child is never mistaken for a slow one; a give-up is a recorded verdict, not an exit code);
- **quota** — the cgroup/ulimit envelope lvi already knows how to draw, applied to the peer's
  own processes and to admitted guests alike (co-resident safety is the same invariant);
- **accounting** — compute as REA (`pod/compute_rea.rs` shape): CPU/memory/disk/bandwidth
  consumed per process per interval, so stewardship, limitarian governors, and delegated
  compute commitments read one ledger;
- **listening** — ownership of every child's stdout/stderr, ring-buffered, structured-parsed
  where the child speaks structure (DB-pool saturation today), harvested into the
  **death witness** at exit — the witness is this primitive's first-class output, not a
  storage feature;
- **describe** — a passport for the whole envelope (image/binary hashes, hApp hashes, quotas,
  uptimes, restart counts, last verdicts) served at one address and offered along the peer's
  custody plane, so `kubectl describe` becomes `GET /epr/{pod-cid}`.

One implementation across the device spectrum: bare processes on a household box (what
`hc-mesh.sh` hand-rolls today), a rack blade under steward/node's cluster operator, a devspace
sandbox under lvi, and **inside a k8s pod on alpha** — where k8s becomes one packaging of the
envelope rather than its owner, and the conductor and node containers can no longer drift to
different builds unnoticed (they did: `4a81a749` vs `7513654f`, 2026-09-02).

## Sequencing

1. Death witness (its own item) is the first output and the forcing function — build it on
   `process_manager` only as far as the ring buffer + exit classification (landed 2026-09-02),
   then lift it into the envelope rather than growing storage's supervisor further.
2. Brainstorm the envelope proper: which crate (a `crates/elohim-pod` under the runtime seam,
   consumed by elohim-storage's binary, steward/node, and lvi-actuator), what the declared
   process set looks like as a manifest (SDK seam: compose inward), and how the REA accounting
   joins the existing `compute_rea` shape. p2p-design-gate on the passport and the accounting
   events before any route.
3. The alpha edgenode template then runs the envelope as PID 1 of the pod, with the conductor as
   one supervised child — closing the split-image drift and the "only k8s can see it" gap in one
   move.

## Done when (for this item — the envelope's first slice)

The household mesh's three peers run under the envelope (no `hc-mesh.sh` process juggling),
`GET /epr/{pod-cid}` on any of them describes its process set with quotas, uptimes, and last
verdicts, a forced conductor death produces a death witness through the envelope, and the same
binary boots as PID 1 inside the alpha edgenode pod — measured by an a2o scenario in
`features/recovery/` and a habit under the runtime seam.

## Design canonized (2026-09-02) — see the spec

`genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md` seals the primitive
(decision register §12, adversarial-review disposition §16). Corrections to this atom, from the
grounding (21 briefs, adversarially verified) and three reviews:

- **Name:** `elohim-pod` retired — `pod` already names `steward/node/src/pod`, the cluster
  *operator* (the opposite layer). The runtime is **tevah**; the declaration is a **`RuntimeSeed`**.
- **The "three partial envelopes" table is wrong in every row but one.** `process_manager`'s
  try_wait + tail DID land (`264ce8ce4`, `dcf9a16c3`) — the ring is 200 lines, the give-up path
  carries no tail, nobody `try_wait`s after readiness, and the tail is logged, never persisted.
  `lvi` has zero `.rs` files (spec-only; its "hard cgroup quota" is podman flags in a doc).
  `steward/node::pod` supervises nothing (service health is a constant, consensus a simulation,
  `RestartService` fabricates success, `compute_rea.rs` is an inference-token record with random ids
  that is minted and dropped). The conductor's k8s envelope is `_edgenode-conductor.template.yaml`
  (rung 2), where the storage image runs in embedded mode with every feature off — that pod IS the
  envelope in disguise and the cheapest extraction path.
- **Accounting shape:** not `compute_rea.rs`; the ledger is `economic_events.bounded_by` ↔
  `mishpat_commitments.cid` with `substrate_signal`, which exists with no producer. Interval samples
  stay Category-C in a separate table; breach events and daily composites are the heads.
- **Sequencing changed:** S0 is the envelope as its own launchable unit on the household mesh
  (`hc-mesh.sh` direct mode → `tevah run`), NOT a witness grown inside `process_manager` — the only
  pipe-owning supervisor ships in one alpha container behind a frozen image tag, so growing it there
  ships the witness only where `kubectl logs --previous` already exists. Alpha confirms by shipping
  tevah inside the storage image with a deliberate conductor roll.
- **Done-when corrected:** `GET /epr/{pod-cid}` resolves by `Content.id` = the atom CID; the habit is
  `runtime-death-witnessed` (born unwired in `elohim/elohim-storage/.epr-meta/`) with the envelope's
  own habit born in `elohim/tevah/.epr-meta/` when the crate exists; the scenario lives in
  `features/resilience/death-witness.feature`, not `features/recovery/`.

Status: envisioned → **refined** (the timeline vocabulary has no "designed"; refined = a sealed spec exists and the atom is its provenance).
