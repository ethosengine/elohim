---
id: "backlog-dataplane-peer-fallback-and-blob-replication"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dataplane: blob byte-replication peer-to-peer + doorway/storage peer-fallback on blob-miss (EPR-head-aware syncing status, not 'App not found')"
slug: "dataplane-peer-fallback-and-blob-replication"
written: "2026-06-29"
author: "p2p-dataplane validation-suite planning (the gap the suite makes CI-visible)"
status: "backlog"
priority: "high"
jobs: [elohim]
---

## The gap (live failure, 2026-06-29)

elohim.host (alpha-B) serves `{"error":"App not found: elohim-host-landing"}` on `/` while alpha-A is fine. Root cause: the EPR **head** is present on elohim.host (`/db/content/elohim-host-landing` returns the atom) but **`blobHash: null`** — the blob bytes never replicated peer-to-peer. The Automerge content-sync plane converged the `node:e2e-*` docs to both hosts, but the **blob custody plane did not replicate the SPA bundle blob**. The CI per-host `stageSpaBlob` PUT is a crutch papering over this.

Canonical (for months): **peers are the store; doorways are projections.** A cold doorway projection should **fetch the EPR's blob from a server-capable peer** (adam/matthew) — the `race_fetch` primitive already exists (`elohim-storage` `blob_fetch::race_fetch`, used only in `custody_sweep`, NOT in the on-demand resolution path) — and degrade gracefully with **EPR-head-aware status** (k8s PodInitializing→Running analog: `head known · blob syncing N/M bytes · ready`), never a hard `App not found` (emitted at `elohim/elohim-storage/src/http.rs:5589/5601`).

## The fix (this is the IMPLEMENTATION; acceptance gate = the validation-suite plan)

1. **Blob byte-replication peer-to-peer**: ensure an authored EPR/blob replicates to custody peers (inventory-guided fetch / distribute_shards reaching elohim.host) — not just metadata gossip.
2. **Peer-fallback read-through**: wire `race_fetch` into the storage app/blob resolution path (`http.rs:5589/5601`) — on a known-EPR-head local blob-miss, pull from a server-capable peer, serve + persist. Doorway stays a dumb projection (read-through happens in storage).
3. **EPR-head-aware syncing status**: replace the hard 404 with a status body (`{ eprHead, blob: { state: syncing|ready, bytes, of } }`) — reuse the `WarmupState`/"warming" concept (`self_healing.rs`/`admin_cache.rs`). Likely the home of the backlogged `GET /api/v1/sync/status`.
4. **Retire the per-host stageSpaBlob crutch** once replication+fallback land (the `federation-deploy` concern asserts all-peer resolution).

## Acceptance gate (do NOT re-spec — this plan owns the proofs)
`genesis/docs/superpowers/plans/2026-06-29-p2p-dataplane-validation-suite-plan.md` — concerns `@concern:blob-replication`, `@concern:epr-projection-fallback`, `@concern:federation-deploy` are authored RED-FIRST against live peers. This fix turns them green.

## Enforcement (so it can't silently regress)
- `.epr-meta` code-gate on `elohim/elohim-storage/src/` (validation-suite plan Task 6): yells at edit time if resolution returns hard-fail without a peer-fallback branch.
- The validation suite's per-concern CI surface (red until fixed).

Domain D5 (data plane). Composes `race_fetch`, inventory gossip, `WarmupState`. Verifiable on household-nodes + shem (both available).
