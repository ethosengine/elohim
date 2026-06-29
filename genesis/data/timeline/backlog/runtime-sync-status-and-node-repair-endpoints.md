---
id: "backlog-runtime-sync-status-and-node-repair-endpoints"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Runtime per-node ops gaps: GET /api/v1/sync/status (content-sync diagnostic) + POST /admin/node/repair (genesis self-heal) — dev-mode bridge Plane 1"
slug: "runtime-sync-status-and-node-repair-endpoints"
written: "2026-06-29"
author: "overnight deploy shift (content-sync plane) — dev-mode config service grounding"
status: "backlog"
priority: "medium"
jobs: [elohim]
---

## What

Two per-node runtime ops the operator/dev-mode bridge wants but the runtime doesn't expose yet (surfaced while grounding runtime-troubleshooting for the content-sync deploy shift). Neither blocked the deploy; captured so they're build-targets, not dumps.

1. **`GET /api/v1/sync/status`** — a content-sync diagnostic surface. Today verification of the Automerge producer + `/sync` plane on a live node is indirect (`GET /sync/v1/elohim/docs` → doc count > 0). A first-class status endpoint should report: doc count in the `"elohim"` namespace, last sync-round timestamp, producer-listener health, and content-projection lag (content_created vs doc lastModified). Home: `elohim/elohim-storage/src/http.rs` (+ manifest `build_manifest()` so doorway can proxy it). Reads the SyncManager/DocStore + the producer listener state.

2. **`POST /admin/node/repair`** — the dev-mode bridge's Plane-1 Op #1 (spec `genesis/docs/superpowers/specs/2026-06-23-runtime-orchestration-developer-mode-bridge-design.md`). Should call `ConductorManager::clear_conductor_state()` (`elohim/elohim-storage/src/conductor/process_manager.rs:233`) + `restart()`, guarded against unsafe paths, gated by the `GENESIS_SELF_HEAL_IDENTITY` policy (re-seedable nodes only; mints a new agent key). Node-local (NOT in `build_manifest()` — same posture as `/admin/arc-policy/actuate`). Auth: operator/steward capability (future `delegates-repair` commitment).

## Why deferred (not blocking)

The content-sync deploy shift verifies producer+/sync via the existing `/sync/v1/elohim/docs` doorway-proxied surface; node-repair is only needed if a deployed node needs genesis re-heal (re-key/wipe), which is a heavier, policy-gated op. Build either when a shift actually needs the op (the operator's "add the service that lets you do that, push, and do that" mandate).

Domain D6 (runtime topology) / D5 (data-plane diagnostic). Spec: `2026-06-23-runtime-orchestration-developer-mode-bridge-design.md`.
