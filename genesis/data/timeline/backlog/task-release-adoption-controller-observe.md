---
id: "backlog-task-release-adoption-controller-observe"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: adoption controller (observe mode) — watch followed release channels through the own conductor, fetch + verify releases, report typed verdicts on /admin/adoption; NO apply"
slug: "task-release-adoption-controller-observe"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-manifest-schema-packager"
  - "backlog-task-release-apply-vehicles"
tags: [upgrade-propagation, rung5, adoption-controller, reconciliation, elohim-storage, delegable]
---

**Claimable by any implementation agent. Depends on T1's schema (the manifest
currency); T4 (`task-release-apply-vehicles`) builds on this task's module and
MUST NOT start until this lands. The one genuinely new component of the rung-5
spec (§6), in its safe half: observe-only.**

## Why

The spec's adoption semantics — "bytes hash right" is transport, adoption is
consent — need a home: a reconciliation loop (P1: DHT is the manifest, the
controller eagerly reconciles) that can SEE and JUDGE releases before anything
is allowed to act. Observe-mode-first mirrors how every prior arm landed
(dry-run before apply on `/admin/coordinators/sync`).

## P2P design-gate decision

Carried by the spec §5: AdoptionState is Ephemeral (C) — reconstructable,
node-local, surfaced on an admin route, never notarized or gossiped as
authority. The controller adds NO entity, NO route in `build_manifest()` (the
admin surface is node-local exactly like `/admin/coordinators/sync`,
`http.rs:4931`'s exclusion pattern). Concern canon: this task must land C6a
(bounded work per sweep + finite backoff), C6b (idempotent on (channel,
releaseCid)), C8 (typed reason on every arm + per-decision metrics), C4
(honest absence: no earned head → idle, `tier: none`, never a guess) — and
register the verdict predicate in the crate's `seam-registry.yaml` at birth.

## Scope

1. New module `elohim/elohim-storage/src/services/release_adoption/`
   (`mod.rs`, `watch.rs`, `verify.rs`, `state.rs`; `apply.rs` is T4's — leave
   a `pub trait ApplyVehicle { fn apply(&self, v: &VerifiedRelease) ->
   Result<AppliedReceipt, AdoptionRefusal>; }` seam with NO implementations).
2. **Watch**: followed channels come from the rung-4 runtime-config surface —
   add `releaseChannels: [{channelId, mode: "observe"}]` to the watched
   config (only `observe` is legal until T4). Resolve each channel's
   canonical head through THIS node's conductor (I1; reuse the head-resolve
   rails `services/head_adoption.rs` uses — compose, don't re-derive).
3. **Fetch**: manifest content + artifact blobs by CID via the existing blob
   fetch path (`p2p/blob_fetch.rs` evidence-ordered candidates).
4. **Verify** (`verify.rs`, the floor — spec §6.3): schema-validate the
   manifest (T1's schema, vendored or re-exported); blob CID match; envelope
   check against the runtime passport's installed reality
   (`runtime_passport.rs` per-role dna_hash + coordinator_wasm_hashes — the
   same per-role refusal `happ_manager.rs::lineage_mismatch_error` enforces,
   moved to verify time); lineage parent verified against the channel's L2
   version chain, body field a hint that must match; attestation threshold
   per the manifest's `adoptionDiscipline` (read via T5's
   `count_qualifying_attestations` if landed, else a `threshold_unchecked`
   typed verdict — NOT a pass).
5. **Report**: `GET /admin/adoption` (node-local, NOT in `build_manifest()`)
   → per channel: `{channelId, mode, resolvedHead: {cid, tier} | null,
   verdict: {ok} | {refusal: <typed reason>}, lastCheckedAt}`. Metrics:
   `elohim_release_adoption_decisions_total{arm, reason}` following the
   `elohim_content_election_*` pattern (`metrics.rs`).

## Interface contract (consumed by T4, T6)

- `VerifiedRelease { channel_id, release_cid, manifest: ReleaseManifest,
  artifact_paths: Vec<PathBuf> }` and `AdoptionRefusal` (typed enum) are the
  currency T4 implements `ApplyVehicle` against — names normative.
- `/admin/adoption` JSON is T6's receipt input — extend only additively.

## Disjointness contract

- MAY create the module, add the runtime-config key, the admin route, metric
  names, the seam-registry rows, unit/contract tests, and edit this atom.
- MUST NOT implement any apply (no conductor mutation, no config write, no
  exec/slot touch), edit `happ_manager.rs` / `head_adoption.rs` /
  `hc-mesh.sh` / zomes / sibling scripts. Conductor calls are read-only
  resolves, bounded per sweep (the uncancellable-call rule: size work before
  calling).

## DoD + verification

- `cargo test` green for the module (verify-arm contract tests: envelope
  mismatch, lineage-hint mismatch, threshold-unchecked, honest-absence —
  each a distinct typed reason).
- On the mesh with T1+T2 outputs: a followed channel with a staging head
  shows `verdict: ok` (or the precise refusal) on `/admin/adoption` on every
  peer, within a bounded number of sweeps; `mode: observe` provably applies
  nothing (conductor PIDs + coordinator hashes unchanged).
- `seam-registry.yaml` row present; `placement-audit.py --epr-meta` clean for
  the crate.
