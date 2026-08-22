---
id: "backlog-p2p-sync-mode-unified-gate-design"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Sync-mode control: one reason-carrying SyncGate unifying operator mode, network class, and the existing internal suppressions — design passed p2p-design-gate, ready to implement"
slug: "p2p-sync-mode-unified-gate-design"
written: "2026-08-22"
author: "orchestrator"
status: "backlog"
priority: "medium"
severity: medium
---

## Why

Five wave1b scenarios red on absent surface: `deployment/sync-control.feature` (POST `/p2p/sync-mode` 404,
`GET /p2p/sync-mode/history` 404) and `deployment/p2p-validation.feature` ("P2P status endpoint exposes
sync_paused state" — `/health` `p2p.syncPaused` undefined). Census class IMPLEMENT-DESIGN. Design is now
done (p2p-design-gate run 2026-08-22); this row carries it for the implementing agent.

**The unifying fact:** elohim-storage ALREADY suppresses sync internally — account-import pause, bulk-write
pause, drain-backlog auto-suppression (the other p2p-validation scenarios assert these). The feature must NOT
add a second pause plane. One `SyncGate` composes every suppression source with a reason, and every surface
reads the same gate.

## P2P Design Gate: sync-mode control

### Entity: SyncModeState (current operator/device mode: `sync` | `paused` | `wifi-only`, plus declared network class `wifi` | `cellular` | `unknown`)
- **Classification**: Ephemeral (C) — operational node-local control state, like rate-limit counters/pool
  metadata. No peer ever validates it; the community does not witness a device's bandwidth posture.
- **Reconstruction strategy**: on loss, defaults to `mode=sync`, `network=unknown` (current behavior).
  Document in code comment.
- **Network Stakes**: all stages; behavior identical at every stage (pure node-local). No floor-protected
  cost — but note CounterEvidence floor is untouched: sync pause defers bulk content sync, it must NOT block
  correction-reach delivery paths if/when those ride the same swarm (guard with a code comment at the gate).
- **Content Address Strategy**: none (singleton per node) — Slug/UUID class justified: operational entity,
  no content to hash, no agent-stance tuple. Route is unparameterized.
- **Source of Truth**: SQLite (operational) — one-row table or kv, `-- Source of truth: local (operational)`.
- **Integrity/Coordinator zome**: none (C class, no DHT touch). DNA-hash-NEUTRAL trivially.
- **Projections**: SQLite only; no Automerge; no dht_anchor_hash.
- **HTTP Routes**: declared in elohim-storage `build_manifest()` (http.rs) — NOT doorway routes:
  - `GET /p2p/sync-mode` → `{mode, network, effective: {syncing: bool, reasons: [..]}}`
  - `POST /p2p/sync-mode` `{mode}` (idempotent; same-mode POST is a no-op, still 200)
  - `POST /p2p/network-class` `{network}` (or fold into the same POST body — implementer's call, scenarios
    establish device network as a precondition)
- **Anti-pattern check**: clean — no UUID, no DHT, no doorway route file, no per-host notarized write.

### Entity: SyncModeTransition (history rows for `/p2p/sync-mode/history`)
- **Classification**: Ephemeral (C) — operator-facing audit log (observability, like logs), bounded
  retention (cap at ~500 rows, prune oldest). Loss is acceptable; not a protocol lie.
- **Fields**: `at`, `from_mode`, `to_mode`, `source` (`operator` | `network-change` | `internal`), `reason`.
- **Route**: `GET /p2p/sync-mode/history` (same manifest declaration).
- Everything else: as above.

### The SyncGate (verdict fn — Step 4 concern canon)

`effective_sync(mode, network, internal_suppressions) -> {syncing: bool, reasons: Vec<SyncSuppressReason>}`

Suppression sources composed (OR): `OperatorPaused` (mode=paused) · `WifiOnlyOnCellular`
(mode=wifi-only ∧ network=cellular) · `ImportInProgress` · `BulkWriteInProgress` · `DrainBacklog`
(existing auto-suppression). Pure function, unit-testable; the existing internal pause call-sites route
through it instead of their private flags.

Concern-canon answers: C0 node-local control plane (answered); C1 n-a; C2 answered — suppression is
monotonic OR, no source can un-suppress another; C3 answered — every suppression source has a release
path (operator POST, network change, operation end, backlog drain) and `reasons[]` names what holds it;
C4 answered — `network=unknown` is honest and does NOT suppress under wifi-only (permissive default =
current behavior; the cellular guard is an opt-in protection, and unknown is surfaced, never guessed);
C5 n-a; C6a answered — gate evaluation is O(sources); C6b answered — POST idempotent; C7 answered —
`/health p2p.syncPaused` + `/p2p/status syncPaused/syncReasons` serve exactly what the gate decides
(one gate, no second breaker-map class); C8 answered — every transition appends a history row with
source+reason; C9 n-a; C10 answered — `syncPaused: bool` added additively to existing views (schema
contract updated, never repurposed); C11 answered — DrainBacklog IS the externally-imposed backpressure
arm, now visible; C12 partial — POST is operator surface: gate behind the same admin auth class as other
/admin-grade levers if one exists on /p2p routes, else note the gap; C13 n-a; C14 n-a.

Register in `elohim/elohim-storage/seam-registry.yaml`: kind `verdict-fn`, contractTests naming the gate's
unit tests (or explicit null + gapNote at first commit).

### Design constraints discovered
- The two-breaker-views lesson (doorway, fixed this session) applies verbatim: `/health`, `/p2p/status`,
  and the gate must read ONE state — never a second privately-updated map.
- Scenario "connectedPeers >= 5" assertions in the same features are alpha-shaped — fixture scoping, not
  part of this implement.
- Wifi/cellular detection is NOT the storage node's job — the device/steward layer declares it via the
  API; the scenarios do the same. No platform network sniffing.

## Done when

`deployment/sync-control.feature` scenarios pass on the local mesh (operator pause/resume, wifi-only ×
cellular/wifi, history audit) and "P2P status endpoint exposes sync_paused state" passes; existing
import/bulk/drain suppression scenarios still pass, now with reasons visible.
