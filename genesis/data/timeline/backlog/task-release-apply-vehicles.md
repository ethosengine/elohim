---
id: "backlog-task-release-apply-vehicles"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: apply vehicles — turn a VerifiedRelease into a running change: coordinator bundle via the sync_coordinators path, config EPR via runtime-config reload, storage binary via exe-slot staging (mesh trust context)"
slug: "task-release-apply-vehicles"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-adoption-controller-observe"
  - "backlog-task-release-soak-attestation-rail"
tags: [upgrade-propagation, rung5, apply, coordinator-hotswap, exe-slot, elohim-storage, delegable]
---

**Claimable by any implementation agent — ONLY after
`task-release-adoption-controller-observe` (T3) lands; this task implements
T3's `ApplyVehicle` trait inside T3's module. Also consumes T5's attestation
rail public fn if landed (a single call site); otherwise leaves a marked
seam.**

## Why

Every apply vehicle already exists (rung 1: coordinator hot-swap; rung 4:
config reload; the mesh exe-slot swap for binaries). This task is pure
composition: verified release in → existing vehicle out — which is exactly
why apply must NOT invent mechanics, only route to them.

## P2P design-gate decision

No new entity, route (beyond `?mode=apply` semantics on existing node-local
surfaces), or DHT write. Concern canon: C6b idempotence — applying the
already-applied head is a no-op with a typed `already_current` reason; C2 —
apply NEVER moves a head (the ceremony does); C11 — apply defers under
ram-guard/PVC/quiesce pressure with a typed `deferred_backpressure` reason
(lag-within-window, spec §4.3).

## Scope

1. `elohim/elohim-storage/src/services/release_adoption/apply.rs` —
   implement `ApplyVehicle` per artifact class:
   - `coordinator-bundle` → the in-process `happ_manager::sync_coordinators`
     apply path (the exact machinery behind `POST /admin/coordinators/sync`,
     `http.rs:4909-5070` — call the shared fn, do not shell out to own HTTP);
     respects `ALLOW_COORDINATOR_UPDATE`.
   - `config-epr` → write the release's config payload to the watched
     runtime-config file path + let the rung-4 watcher pick it up (same-PID,
     seconds); boot-only knobs refuse with the existing typed reasons.
   - `storage-binary` → **staging only, never self-exec**: verify, write the
     binary to a well-known slot (`$MESH_DIR/release-adoption/<peer>/
     elohim-storage.next` locally; path from config), mark AdoptionState
     `pending-restart`, and let the harness restart arm
     (`hc-mesh.sh restart_storage`, which already handles exe records, env
     capture, and loud failure) consume the slot. Fleet binaries remain OUT
     (spec §9) — refuse `storage-binary` apply unless the declared network
     stakes are Simulacra (the mesh/dev trust context).
   - `happ-bundle` → joined/re-installing peers only (spec §6.4 bootstrap
     caveat): route to the existing install path; fresh joiners refuse with
     `bootstrap_out_of_band`.
2. Wire the controller's mode gate: `mode: "apply"` in `releaseChannels`
   config becomes legal; observe stays the default. Post-apply, call T5's
   `author_soak_attestation` if the rail is present (one call site; feature-
   gate or marked TODO seam if T5 hasn't landed).
3. Extend `/admin/adoption` rows additively: `appliedRelease {cid, at,
   vehicle}` and `pendingRestart: bool`.

## Interface contract (consumed by T6)

- The staged-binary slot path + `pending-restart` state name is normative —
  the mesh harness/receipt reads them; coordinate the exact path string in
  this atom when implemented.

## Disjointness contract

- MAY edit `services/release_adoption/` (T3's module), add the shared-fn
  extraction in `happ_manager.rs` IF the sync path is not already callable
  in-process (smallest possible refactor, no behavior change), extend the
  admin route additively, add tests, edit this atom.
- MUST NOT edit `hc-mesh.sh` (the harness verb already exists — if slot
  consumption needs a harness change, STOP and report the missing station as
  a story-graph node), zomes, doorway, Jenkinsfiles, or manifests.

## DoD + verification

- Mesh receipt (composes T1+T2): publish coordinator release → promote →
  a `mode: apply` peer converges (coordinator wasm hashes on `/version`
  change to the release's; conductor PID unchanged; ~2 min class) →
  `revert` re-election converges it back. Config release: flag flip lands
  same-PID in seconds. Binary release: slot written + `pending-restart`
  reported + a manual `just mesh storage-restart <peer>` boots the staged
  binary (exe record proves it).
- Idempotence: re-sweep on a current head is `already_current`, zero
  conductor calls beyond the resolve.
