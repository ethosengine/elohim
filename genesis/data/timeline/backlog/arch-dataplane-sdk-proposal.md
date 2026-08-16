---
id: "backlog-arch-dataplane-sdk-proposal"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Dataplane SDK — .dataplane() facade LANDED with CI substrate-verify as first consumer; residue: codegen-consumer wiring, http.rs input validation, legacy snake_case types.ts, bash-script retirement"
slug: "arch-dataplane-sdk-proposal"
written: "2026-06-11"
author: "agentic-developer (dataplane architecture review, operator-requested)"
status: "backlog"
priority: "medium"
ci_status: verified
tags: [architecture, dataplane, sdk, api-surface, design-gate, brainstorm]
cites:
  - elohim/sdk/storage-client-ts/src/client.ts
  - genesis/scripts/ci/substrate-verify.sh
---

# Dataplane SDK proposal — LANDED 2026-08-16 (shift dataplane-facade-first-consumer)

**Current decision / state:** the facade shipped and is daily-proven. The three
design questions were answered on standing evidence (operator skip-the-gates
preference) and are recorded in DATAPLANE-SDK-PATH.md critical path #3: (1)
multi-peer addressing via `DataplaneFleet` (PEER_STORAGE_URLS CSV), doorway as
single-target `DoorwayClient`; (2) healing stays a node primitive — the
proposal's `healBlobConcurrent` client fan-out was deliberately NOT built; (3)
`schemas/v1/inputs/` extended as the convention (rea-commitment-query +
economic-event-query schemas). CI evidence: genesis #1477/#1478 — all 7
substrate-verify subcommands run `genesis/a2o/scripts/substrate-verify.ts` on
`@elohim/storage-client`, artifacts stamp `runner:"facade"`, 5/7 failed=0
(projection/federation env-fails match the bash baseline byte-for-byte).

**Residual rows (the open work; each bounded):**

1. **Wire storage-client-ts as a schema-codegen consumer** — the two inputs/
   schemas exist but INTERFACE_FILES doesn't emit into storage-client-ts, so
   `ReaCommitmentQuery`/`EconomicEventQuery` are hand-declared in
   `src/api/dataplane.ts` with schema pointers. Wire the consumer, replace the
   hand declarations with generated re-exports.
2. **`http.rs` input validation** — `handle_db_rea_commitments` (and the
   economic-events list handler) still loose-parse their query params; wire
   the new input schemas into request validation per the views/ discipline.
3. **Legacy snake_case surface in storage-client-ts `src/types.ts`** — the
   sync/blob response types (`doc_id`, `blob_hash`, …) predate the ts-rs View
   convention and violate "snake_case never leaves the Rust boundary"; needs
   Rust-side View types + generated replacements (breaking change for
   consumers of those fields — coordinate with elohim-app usage).
4. **Retire `genesis/scripts/ci/substrate-verify.sh`** — un-invoked since the
   Jenkinsfile swap (c1ee43081); kept one cycle as the revert lever. Remove
   after the facade runner survives a week of genesis runs (or the next
   substrate-verify feature change, whichever first).

Original proposal (op surface, staged adoption, the three questions as asked)
preserved below for provenance.

---

Operator intent: "an API/SDK surface that helps cure our capabilities over
this dataplane substrate" — let developers and agents PLAY with custody,
healing, pinning, posture, adjacency, federation, not just prove them in
bash+jq.

# ARTIFACT 2 — DATAPLANE SDK PROPOSAL (for /brainstorm)

**Problem.** The dataplane's HTTP surface exists (~35 routes in `elohim-storage/src/http.rs`: `/p2p/status`, `/p2p/peers`, `/api/v1/pins`, `/api/v1/commitments`, `/api/v1/status/projector`, `/api/v1/peers/delivery`, `/api/v1/diagnostics/inventory-parity`, blob/sync/federation) and 445 ts-rs-generated types exist in `elohim/sdk/storage-client-ts/src/generated/` — but `StorageClient` (`src/client.ts:40-371`) exposes no typed operations over them. Every consumer hand-rolls URLs: `genesis/scripts/ci/substrate-verify.sh` is 594 lines of curl+jq (`?action=custody-blob&limit=200` at lines 314, 381); a2o steps use undici + manual JSON (`genesis/a2o/steps/delivery.steps.ts:30-42`).

**Package shape.** Extend the existing `@elohim/storage-client` with a `.dataplane()` typed facade — no new package, no new HTTP routes. Doorway stays transparent (single base URL; projection cache is a layer-2 optimization, not API ownership).

**Op surface (illustrative, not final):**
- `custody.queryCommitments(q: CommitmentQuery) → ReaCommitmentView[]` · `custody.queryCustodyManifest(blobHash)`
- `acquisition.listPins() → PinView[]` · `createPin(headRef, closureRule)` · `deletePinById(id)`
- `posture.getNetworkPosture() → NetworkPostureView` · `posture.getPeerStatus() → P2PStatusInfo`
- `peers.listConnectedPeers() → DeliveryPeer[]` · `getPeerCapabilities(peerId)`
- `sync.waitForCaught(peerId, opts: SyncWaitOptions) → P2PStatusInfo` — polls `/p2p/status` over the existing counters (`reconcile_passes_total`, `kicks_fired_total`, `placement_gaps_emitted_total`) and the tri-state `caughtUp` fields
- `healing.healBlobConcurrent(blobHash, opts) → HealResult` — client-side fan-out over `/p2p/peers` + `GET /blob/{hash}` (heal-on-read is already implicit server-side)
- `federation.listDoorways()`

**Where types come from.** Output types: already generated via ts-rs (PinView, ReaCommitmentView, P2PStatusInfo, NetworkPostureView…). Input types: **new** — add `inputs/` schemas to `elohim/sdk/schemas/v1/` (first: `rea-commitment-query.schema.json`), wire validation into `http.rs` (`handle_db_rea_commitments` currently loose-parses), codegen TS per the existing view-schema pipeline.

**Staged adoption — CI as first consumer.**
1. *Weeks 1-2:* `.dataplane()` facade + CommitmentQuery schema; migrate substrate-verify's mesh/propagation/projection assertions to a TS `DataplaneAssertions` class in `genesis/a2o`. The CI suite proves the SDK surface daily.
2. *Weeks 3-4:* `waitForCaught` polling helper; finish CI migration (delivery, federation, resilience); a2o BDD steps reuse the client.
3. *Weeks 5-6:* `healBlobConcurrent` fan-out; federation ops; peer-loss failover hooks (Workstream C).
4. *Later:* an aggregated `observe()` endpoint — the **only** item adding HTTP surface, deferred, and it must pass the p2p-design-gate before design.

**Three design questions for the operator/brainstorm:**
1. **Addressing model:** does the SDK stay single-base-URL (doorway-transparent) or learn multi-peer addressing now to serve healing fan-out and peer-loss failover natively? (Alpha note: doorway proxies `/api/v1` reads to matthew only — single-URL hides real peer diversity.)
2. **Healing orchestration locus:** client-side fan-out (`healBlobConcurrent` in TS, P2P-native, no new routes) vs a node-side `POST /api/v1/heal/{blobHash}` primitive? This decides whether orchestration intelligence lives in consumers or in the node — gate-worthy.
3. **Typed-inputs scope:** is `CommitmentQuery` a one-off, or does `schemas/v1/inputs/` become the convention for *all* list/filter/pagination endpoints (with `http.rs` validation as standard) — and what is the pinned contract for tri-state sync fields (`null` = not computable vs `false` = behind) that `waitForCaught` depends on?

**Non-goals:** no new HTTP routes in Phases 1-3; no doorway TS client (single-target proxy needs none yet); no over-specified method signatures before the brainstorm answers Q1-Q3.
