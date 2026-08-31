---
name: project_workspace_stewarded_device_peer_batch
title: "Workspace as matthew's device — native-sync batch"
description: "Native content sync from a workspace device peer to both alpha doorways PROVEN 2026-08-30 (no seed, no Jenkins); recipe + the three fleet enablers; identity binding (station 3) still open."
metadata: 
  node_type: memory
  title: "Workspace as matthew's device — native-sync batch"
  type: project
  originSessionId: bd085a03-fcae-4c1c-b245-fda9ca07a257
  modified: 2026-08-30T18:33:51.360Z
---

**Proven 2026-08-30 17:04Z/17:35Z:** a content node authored, DHT-anchored and head-declared by the
workspace device agent W (fresh key, never matthew's) was served by doorway-alpha (`trust: notarized`,
W's anchor, bytes) and elohim.host (`published`, bytes) with no doorway seed and no Jenkins in the
content path. Workspace storage: `irohPeersKnown 7`, replication caught up, 2.6k inventory pages applied.

**Recipe (localdev → alpha):** `just dev conductor alpha` with `HOLOCHAIN_BIN=<fork pair dir>` (pool debug
slot `family/dev/elohim__holochain-conductor/dev/debug`, built at the fleet's pinned fork commit) →
elohim-storage built with `--features "p2p p2p-iroh"` and run with `ELOHIM_TRANSPORT_BACKEND=iroh
ELOHIM_IROH_RELAY_URL=https://relay.alpha.elohim.host ELOHIM_DOORWAY_URL=https://doorway-alpha.elohim.host
--enable-p2p --agent-pubkey W --admin-url ws://localhost:<admin> --app-url ws://localhost:4485` →
write via the storage's own routes: `PUT /blob/sha256-<hex>` (X-Blob-Cid) → `POST /db/content/bulk` →
`PATCH /db/content/{id}` **carrying a notarized field** (`blobHash`/`reach`; a description-only PATCH is
diesel-direct and never reaches the conductor) → `POST /db/content/{id}/head` with `X-Agent-Cid: W`.
Expect the HTTP port to stall for minutes on first fleet contact (catch-up flood).

**The three fleet enablers that opened the plane (all env, all on dev):** storage `ELOHIM_IROH_RELAY_URL=
RELAY_URL_PLACEHOLDER` (7234b6ff0), doorway-alpha `DOORWAY_MANIFEST_BOARD_ENABLED=true` (Sybil trade-off
accepted for the dev doorway; apex off), storage `ELOHIM_DOORWAY_URL=DOORWAY_ORIGIN_PLACEHOLDER`
(08654c016; rendered `bootstrapUrl - '/bootstrap'` — `deployHumanManifest` sits at the 8000-byte CPS
ceiling, split it before adding another line). Pod-to-pod is namespace-isolated; relay is the only path.

**Still open:** station 3 identity (`bind_identity` / signed AgentPeerBinding — spec
`2026-08-30-workspace-stewarded-device-peer-design.md`, feature `auth/stewarded-device-sync.feature`);
the reach-change join (`acknowledges-reach-change` → row reach; manifesto authored `commons`, stored
`community`); bulk (whole content set) not yet run from the peer. Related: [[project_sovereign_peer_t3_rung_traps]],
[[project_content_sync_plane]]; the conductor-pin finding lives in backlog `conductor-pin-ships-base-binary.md`.

**How to apply:** don't re-derive the recipe or re-run the design gate; start from the running recipe,
ship `bind_identity`, then the reach-change join. Serves habit `dataplane-convergence`.
