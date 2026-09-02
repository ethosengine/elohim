---
id: "backlog-dataplane-peer-fallback-and-blob-replication"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dataplane: blob byte-replication peer-to-peer + doorway/storage peer-fallback on blob-miss (EPR-head-aware syncing status, not 'App not found')"
slug: "dataplane-peer-fallback-and-blob-replication"
written: "2026-06-29"
author: "p2p-dataplane validation-suite planning (the gap the suite makes CI-visible)"
status: "superseded-in-code"
priority: "high"
supersession_note: "2026-07-31: items 2 (heal-on-read via race_fetch in the on-demand path — get_blob_or_heal in elohim/elohim-storage/src/http.rs) and 3 (EPR-head-aware syncing status, not hard 404) have LANDED; remaining scope is item 1 (blobHash pointer propagation via content-sync) and item 4 (retire the per-host stageSpaBlob crutch). 2026-09-02: item 1 (blobHash pointer propagation) has LANDED and is confirmed live — both alpha-A and elohim.host now serve non-null blobHash for elohim-host-landing (sha256-04ae4310… / sha256-f0f0e637…, same dhtAnchorHash uhCkkvfsT…), so the App-not-found gap this atom describes is closed; bytes are seeded once per deploy, authorHeadOnce + DECLARE_ONLY fan-out carries the head, and API_KEY_SEED is deployed on both doorway manifests. Item 4 (retire stageSpaBlob) remains open, now blocked on federation-deploy scenario 3 (staging-authority precondition, @wip) rather than on propagation. The framing this atom was written against — bytes/pointer never arriving — is superseded; the open item is head ELECTION under restart-arc churn (both doorways report caughtUp:false, converged:false, divergentAnchor 2131/1011), tracked in sovereign-peer-network-read-no-authorities.md, not here."
jobs: [elohim]
---

## The gap (live failure, 2026-06-29)

elohim.host (alpha-B) serves `{"error":"App not found: elohim-host-landing"}` on `/` while alpha-A is fine.

**REFINED root cause (validation-suite live probe, Task 4 2026-06-29 — corrects the earlier "blob never replicated" reading):** the blob **BYTES ARE present** on elohim.host (`GET /blob/sha256-1c34…` → HTTP 200) — byte custody DID replicate. What's missing is the **`blobHash` POINTER**: the EPR content-node row on elohim.host has `blobHash: null` (`/db/content/elohim-host-landing`), so the EprRouter has no hash to resolve and 404s "App not found" even though the data is locally available. So this is a **metadata-pointer propagation gap, NOT a byte-replication gap.** The CI per-host `stageSpaBlob` PATCH (which writes `blobHash` per-host) is the crutch — it succeeded on alpha-A and failed on elohim.host (the UNSTABLE leg), leaving the pointer null while the bytes arrived by other means.

Canonical (for months): **peers are the store; doorways are projections.** A cold doorway projection should **fetch the EPR's blob from a server-capable peer** (adam/matthew) — the `race_fetch` primitive already exists (`elohim-storage` `blob_fetch::race_fetch`, used only in `custody_sweep`, NOT in the on-demand resolution path) — and degrade gracefully with **EPR-head-aware status** (k8s PodInitializing→Running analog: `head known · blob syncing N/M bytes · ready`), never a hard `App not found` (emitted at `elohim/elohim-storage/src/http.rs:5589/5601`).

## The fix (this is the IMPLEMENTATION; acceptance gate = the validation-suite plan)

1. **blobHash POINTER propagation (the actual gap)**: the EPR content-node's `blobHash` must reach elohim.host with the content (via content-sync / EPR projection — "peers are the store"), NOT via the per-host `stageSpaBlob` PATCH. Today the bytes arrive but the pointer doesn't. (Byte-replication already works — the bytes are present on elohim.host; do NOT spend effort there.)
2. **Heal-on-read in the EprRouter / resolution path**: since the blob bytes ARE local (`/blob` 200) but `blobHash` is null, the resolver should heal — on a known EPR-head with a resolvable/local blob, set/use the hash and serve, rather than 404. If genuinely blob-missing, peer-fallback via `race_fetch` (already exists in `custody_sweep`, NOT in the on-demand path at `http.rs:5589/5601`) — pull from a server-capable peer, serve + persist. Doorway stays a dumb projection (heal/read-through in storage).
3. **EPR-head-aware syncing status**: replace the hard 404 with a status body (`{ eprHead, blob: { state: syncing|ready, bytes, of } }`) — reuse the `WarmupState`/"warming" concept (`self_healing.rs`/`admin_cache.rs`). Likely the home of the backlogged `GET /api/v1/sync/status`.
4. **Retire the per-host stageSpaBlob crutch** once replication+fallback land (the `federation-deploy` concern asserts all-peer resolution).

## Acceptance gate (do NOT re-spec — this plan owns the proofs)
`genesis/docs/superpowers/plans/2026-06-29-p2p-dataplane-validation-suite-plan.md` — concerns `@concern:blob-replication`, `@concern:epr-projection-fallback`, `@concern:federation-deploy` are authored RED-FIRST against live peers. This fix turns them green.

## Enforcement (so it can't silently regress)
- `.epr-meta` code-gate on `elohim/elohim-storage/src/` (validation-suite plan Task 6): yells at edit time if resolution returns hard-fail without a peer-fallback branch.
- The validation suite's per-concern CI surface (red until fixed).

Domain D5 (data plane). Composes `race_fetch`, inventory gossip, `WarmupState`. Verifiable on household-nodes + shem (both available).
