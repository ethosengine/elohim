---
name: project_pure_iroh_parity_and_mesh_traps
title: Pure-iroh parity + mesh ops traps
description: "Homo-iroh warm recovery PASS 258 s (2026-08-28); export MESH_PEER_TRANSPORTS for hc-mesh.sh; never start mesh processes inside a background tool task."
metadata:
  type: project
---

Pure-iroh mode (`MESH_PEER_TRANSPORTS=matthew=iroh,jessica=iroh,james=iroh`) reached libp2p parity on the pull leg on 2026-08-28 (`b9c9ad477`): `p2p_iroh::pull_core` hosts replication gaps + pin acquisition + `/p2p/status.pull` without a `P2PNode`; heal-on-read races the iroh book; the shard responder serves iroh-staged blobs by alias; `GetManifest`/`Manifest` give the iroh leg the composite pivot. Warm recovery jessica←matthew: 901 s FAIL (P0 only) → 258 s PASS P0–P4 the same evening; durable record in `genesis/a2o/reports/recovery/recovery-timeline.jsonl`.

**Why:** the whole loop lived in the libp2p node; iroh synced docs but never projected rows (`contentCount 0`). Each of four defects hid behind the previous one.
**How to apply:** `export MESH_PEER_TRANSPORTS=…` before `just mesh storage-restart` (the `VAR=… just …` prefix did NOT reach hc-mesh.sh's per-peer overlay — peers came back dual). Never start mesh processes inside a background tool command: its children are reaped when the task completes (storages went `down` twice). Read jessica's `/p2p/status`/metrics INSIDE the same command as the run, or restart in the foreground first. Still open: cold shape, N=3, write-time custody push over iroh, converging the two pull drivers. Related: [[project_local_mesh_binary_slot_and_restart]], [[project_sovereign_peer_t3_rung_traps]].

**2026-08-29 — the reconcile loop was libp2p-gated.** `projection_reconcile` (REA/content/collectives/
participations discovery + the anchor heal from the OWN conductor + adopt-before-author head fetch) took
`&P2PHandle`; `main.rs` gated the whole loop on `p2p_handle`, so pure iroh had NO discovery and NO heal —
`caughtUp` could read true while a row stayed divergent from the conductor's truth (homo-iroh warm P1 red
for 469 s; homo-libp2p healed the same row in 58 s via `HEALED content anchor … (peer discovery)`).
Cure: `p2p::reconcile_peers::ReconcilePeers` (agent_pubkey / list_peers / view_federate by peer-id string),
impl'd by `P2PHandle` and `p2p_iroh::IrohReconcilePeers` (peer book + view-fed ALPN). After it: homo-iroh
warm 61 s / 60 s = libp2p parity. **How to apply:** any arm that "asks peers" must take `&dyn ReconcilePeers`,
never `P2PHandle`; when a pure-iroh leg reads green but a row is stale, grep the peer's log for
`projection-reconcile: no libp2p node — discovery + heal ride the iroh plane` first.
**Residual (plane-neutral):** the converged content DOC can carry `sha256-…` while the author's row is
`bafkrei…` (same digest); the doc reverse-projection copies the doc form into amber rows every round and the
anchor heal restores it — backlog `content-doc-blobhash-representation-drift`. The reverse heal is
idempotent + logs `elohim_storage::sync_heal` from/to since this cut.
**`just gate <project>` must run from `/projects/elohim`** (bit me 3× on 2026-08-29): a Bash call whose
cwd is `elohim/elohim-storage` resolves the CRATE's justfile → `justfile does not contain recipe
elohim-storage`, exit 1 in ~1 s. Any chain that `cd`s into the crate for `cargo` must `cd /projects/elohim`
again before `just`. Also: the root gate's clippy runs DEFAULT features (no `p2p-iroh`) — check
`cargo clippy -- -D warnings` without features before assuming a feature-built clippy pass covers it.

**2026-08-29 M4 — transport self-awareness landed** (`p2p::transport_paths`: ring + `select_path`, registry row;
`elohim_transport_route_total{transport,op_class,reason}`, `elohim_transport_path_rtt_ms`, `/p2p/status.transportPaths`).
Small ops race both planes (the races ARE the probe); Bulk selects — pull leg `plan_acquisition_targets_selected`,
custody push `push_shard_routed` (iroh leg + one fallback). Measured LAN: iroh Small 15 ms vs libp2p 816 ms
degraded; probe shards pushed `transport=iroh`; recovery on/off/on-v2 all ~60 s (noise — the fleet is where it
shows). Traps: (1) Unknown must be the IMMEDIATE Bulk pick (a tick-only floor never fires in an 11-pull recovery);
(2) one blip ≠ Degraded — MIN_SAMPLES_FOR_DEGRADED=3; (3) count route decisions only when there is work to
dispatch (73 phantom `prior_iroh`); (4) a warm-restarted peer drains pulls before its iroh book warms (backlog
`pull-leg-drains-before-iroh-book-warms`); (5) `cut` at the end of a Monitor pipeline block-buffers — no events.
