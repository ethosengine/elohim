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
claimedBy: "claude-opus-t4"
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

## Implementation notes (2026-09-01)

Landed in `elohim/elohim-storage/src/services/release_adoption/apply.rs` plus
the mode/state/report widening in T3's `mod.rs` / `state.rs` / `watch.rs`, the
metric pre-touch in `metrics.rs`, the boot wiring in `main.rs`, and the seam
registration in `elohim/elohim-storage/seam-registry.yaml`.

**No `happ_manager.rs` extraction was needed.** `sync_coordinators_report(&admin_ws,
&app_id, &happ_path, apply)` was already `pub` and already the shared body behind
`POST /admin/coordinators/sync` — both vehicles call it directly, so the HTTP
route, the boot path and the controller run one implementation.

**`ApplyVehicle` became `async` (a deliberate amendment to T3's declared shape).**
Two of the four vehicles route to that conductor admin call, and a sync fn cannot
reach it honestly on this runtime (`Handle::block_on` panics inside a tokio
worker; `block_in_place` parks a worker for the whole of a multi-second hot-swap).
Argument, return type, typed refusal and the additive `handles()` default are
unchanged; a `name()` accessor was added for the receipt's `vehicle` field.

**Normative slot path** (the §Interface contract this atom owed T6):
`<staging_root>/slot/elohim-storage.next`, with a sidecar
`<staging_root>/slot/elohim-storage.next.json` carrying
`{channelId, releaseCid, sha256, bytes, stagedAtUnix, pendingRestart}`.
Constants: `apply::SLOT_DIR` / `SLOT_BINARY_NAME` / `SLOT_RECEIPT_NAME`, resolved
by `apply::slot_path()`. The staging root is `$ELOHIM_RELEASE_STAGING_ROOT`, else
`<storage_dir>/release-adoption` — which on the local mesh IS
`$MESH_DIR/<peer>/release-adoption` (hc-mesh.sh sets `STORAGE_DIR=$MESH_DIR/$name`),
so the full slot is `$MESH_DIR/<peer>/release-adoption/slot/elohim-storage.next`.
This differs from the shape sketched above (`$MESH_DIR/release-adoption/<peer>`)
on purpose: neither `MESH_DIR` nor a peer name is exported to the storage
process, so deriving from them would have been a guess that silently collapses
every peer onto one slot. Deriving from `storage_dir` is per-peer by
construction. State name is `pendingRestart: true` on the
channel's `/admin/adoption` row, and it is STICKY — never cleared by a later
sweep.

**Report rows extended additively:** `appliedRelease {cid, at, vehicle}` and
`pendingRestart` per channel; `controller.applyVehiclesCompiled` flipped to
`true` and `controller.applyVehicles` added (the classes THIS process is
equipped for — an unequipped class refuses `no_vehicle_for_class` rather than
leaving the reader to infer it). A new `Verdict::Applied` still answers
`ok: true`, so a T6 reader keyed on the atom's `{ok} | {refusal:…}` contract
keeps working when a channel moves from observe to apply.

**Ten new apply-arm `RefusalReason`s**, one per distinct cure:
`no_vehicle_for_class`, `apply_not_permitted`, `deferred_backpressure`,
`apply_failed`, `binary_stakes_not_simulacra`, `bootstrap_out_of_band`,
`config_knob_boot_only`, `runtime_config_unwatched`, `apply_payload_unusable`
— plus the non-refusal `already_current` label. The label-stability test and the
metric pre-touch were widened; T3's `assert_ne!(reason.arm(), Apply)` inverted to
"every arm has at least one reachable refusal", which is what that pin was for.

**happ-bundle routes to the coordinator hot-swap, not `ensure_happ_installed`.**
Verify already refused any release binding a different per-role DNA hash, so a
happ-bundle that reaches apply can differ only in coordinator wasm — and the
install path's stale branch mints a NEW agent key. The vehicle adds one thing
over the coordinator one: a fresh joiner (app not installed) refuses
`bootstrap_out_of_band`; an unreachable conductor is `apply_failed`, never
"not joined".

**C11** is wired on the one signal readable without new plumbing — the conductor
admission lane (`in_flight >= capacity`) — checked before routing. ram-guard /
PVC / quiesce are recorded as remaining-partial in `seam-registry.yaml` rather
than forced.

**T5 seam:** `author_soak_attestation`'s signature was stable, so it is called —
once, from `apply::spawn_soak_observer`, AFTER the channel's own declared
`soakSecs` (capped at 24 h). It is deliberately not called at t=apply: an
attestation minted for a window that never ran is exactly the false evidence
another peer's threshold arm would count toward promotion. A channel left
`pendingRestart` authors a FAILING soak — that window observed the previous
artifact.

### Story-graph node this task discovered (BLOCKED — harness change fenced)

chain: rung-5 mesh receipt (T6) / between `storage-binary apply stages the slot`
→ `just mesh storage-restart <peer> boots the staged binary` / **missing node:**
*the restart arm consumes the slot*. `hc-mesh.sh restart_storage` resolves the
binary as `/proc/<pid>/exe` (live peer) → the recorded `exefile` → `STORAGE_BIN`
(`resolve_exe` / `restore_binary_for`), and the live branch always PREFERS the
running exe — so a staged `elohim-storage.next` is never picked up, at any
`STORAGE_BIN` value. current state: the slot is written, digest-verified,
chmod 0755 and reported `pendingRestart: true`; nothing consumes it. Assertion
the missing node needs: *a peer restarted with a staged slot present boots the
staged binary, and its exe record names the slot.* Probe: `resolve_exe` prefers
`<staging_root>/slot/elohim-storage.next` when it exists (and ideally moves it
aside on success, so a failed boot cannot loop). This atom's disjointness
contract forbids editing `hc-mesh.sh`, so it is reported, not done.
