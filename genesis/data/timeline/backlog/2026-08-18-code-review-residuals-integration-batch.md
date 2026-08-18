---
id: "backlog-2026-08-18-code-review-residuals-integration-batch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Code-review residuals from the 2026-08-18 integration batch (custody rotation, warm shell, operator gate, attribution read model)"
slug: "code-review-residuals-integration-batch"
written: "2026-08-18"
author: "integration code-review (origin/dev..HEAD, high effort, pre-push)"
status: "open"
priority: "medium"
area: "elohim-storage + doorway-service"
domain: "D2"
jobs: [elohim-edge]
tags: [code-review, custody-rotation, warm-shell, operator-runtime-surface, identity-cross-signed, efficiency]
---

# Code-review residuals — 2026-08-18 integration batch

The pre-push review of the 8-commit batch (`453f00373..8c739a723`) surfaced 10
findings. Four correctness defects were fixed in the batch itself (custody
successor stranding, warm-shell head-relabel race, CORP header drop on escaped
errors, attention.rs shed laundering). These six were verified real but
non-blocking; each names its cheaper/deeper form so the fix is mechanical when
picked up.

## 1. Operator gate: non-POST under /api/v1/operator/* conflates wrong-method with unknown-verb
`doorway/doorway-service/src/server/http.rs` — `op_gate_capability()` returns
`None` for any non-POST before inspecting the path, so the fail-closed branch
403s `GET /api/v1/operator/reconcile` with an "unknown operator verb" log.
Fail-closed is correct; the semantics/log conflation is the defect. Give known
verbs wrong-method 405 semantics, or at least a distinct log line.

## 2. Operator gate: capability string duplicated across crates
Same site hardcodes `"operator-reconcile"`, hand-synced with elohim-storage's
`OPERATOR_RECONCILE_CAPABILITY` (no shared crate, no manifest link). The deeper
mechanism already exists: add a capability field to `Route` in doorway-client so
storage's `build_manifest()` declares the gate and doorway's existing
RouteRegistry fetch drives it — same missing-wiring class as `is_service_path`.
Do this before slice 3 adds a second verb.

## 3. Warm shell: hydrate() runs once at boot; the router refresh never re-hydrates
`doorway/doorway-service/src/main.rs` — a doorway that boots while storage is
down warms 0 shells and never retries even after the
`DOORWAY_EPR_REFRESH_SECS` refresh repopulates mounts — the first request per
app pays the lazy path in exactly the degraded-boot scenario the cache targets.
Re-run hydration (idempotent) after a refresh that adds mounts.

## 4. Warm shell: boot hydration is sequential
`warm_shell.rs::hydrate` awaits each (slug, entry_file) target in a for-loop
before the HTTP server starts; N apps ⇒ ~2N sequential Mongo round-trips.
`join_all`/`buffer_unordered` bounds boot at the slowest single app.

## 5. Warm shell: server-side `stock_warm_shell` has the same resolve-after-fetch shape
`doorway/doorway-service/src/server/http.rs::stock_warm_shell` resolves the
declared head AFTER the proxied body arrives — the same head-relabel race fixed
in `resolve_shell`/`stock_and_return` this batch (resolve at decision time,
thread it through). Apply the same pattern.

## 6. Custody rotation: per-candidate N+1 queries each tick
`elohim/elohim-storage/src/services/custody_rotation.rs::select_rotation_candidates`
issues two single-row queries per active custody-blob pledge every 300s tick
(`current_blob_for` + `successor_state`), sync diesel on the async runtime.
Batch with the same `IN_CHUNK`/`eq_any` pattern the function already uses for
providers (3 queries total), and/or wrap the pass in `spawn_blocking`.

## 7. Attribution cut: `proof_status` is a bare String on the read model
`elohim/elohim-storage/src/db/models.rs` — `NewPeerIdentityBindingRow` carries
the typed `BindingProofStatus`, but `PeerIdentityBindingRow.proof_status` is a
`String`, so the "never string-compare proof_status" rule is compiler-enforced
on writes only; reads are held by `is_cross_signed()` discipline alone. A
diesel `FromSql` impl reusing `BindingProofStatus` extends the write-side
guarantee to reads. Belongs with the C2 series
(`genesis/data/timeline/backlog/agent-peer-binding-signing.md`).
