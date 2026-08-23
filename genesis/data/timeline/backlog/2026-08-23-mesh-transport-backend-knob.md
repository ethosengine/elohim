---
id: "backlog-mesh-transport-backend-knob"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Local mesh runs the dual-plane dataplane: build the mesh storage binary with p2p-iroh, add MESH_TRANSPORT_BACKEND={libp2p,dual,iroh} to hc-mesh.sh / just test mesh, and stamp the mode into the sprint report"
slug: "mesh-transport-backend-knob"
written: "2026-08-23"
author: "fable-5 session 2026-08-23 (operator steering: prove the dual-plane transport rock-solid in either mode)"
status: "refined"
priority: "high"
area: "a2o/mesh-prologue"
domain: "protocol"
jobs: [elohim-genesis, elohim]
relatedNodeIds:
  - "habit:dataplane-convergence"
  - "habit:blob-durability"
cites:
  - genesis/data/timeline/backlog/mesh-prologue-cast-and-env-gaps.md
  - genesis/docs/superpowers/plans/2026-08-04-holochain-iroh-convergence-upgrade-campaign.md
tags: [a2o, mesh, iroh, libp2p, dual-plane, transport, scripts, codex-claimable, agent-agnostic]
---

# Mesh transport-backend knob

**Why this exists.** Alpha runs `ELOHIM_TRANSPORT_BACKEND=dual`
(`genesis/orchestrator/manifests/edgenode/alpha.yaml:290`), but the household
mesh sets neither the env var nor the `p2p-iroh` cargo feature — its storage
binary is default-feature-only, so `just test mesh` has NEVER exercised the
iroh dataplane. "Rock-solid in either mode" cannot be measured locally until
the mesh can boot in each mode. (The conductor's own iroh/QUIC transport —
kitsune2/tx5 — is a different layer; this knob is elohim-storage's Track-2
dataplane only. Do not conflate them in the script comments.)

## Scope (scripts + justfile only — no Rust changes)

1. `app/elohim-app/scripts/hc-mesh.sh`: honor `MESH_TRANSPORT_BACKEND`
   (default `libp2p` = today). Export `ELOHIM_TRANSPORT_BACKEND` to every
   storage peer's environment at start AND through `storage-restart` (the
   restart re-execs from the captured environ — verify the captured environ
   carries it; `MESH_RESTART_APPLY_PROFILE=1` must not drop it).
2. Binary: when the mode is `dual` or `iroh`, `start` refuses a storage binary
   built without `p2p-iroh` (probe: `elohim-storage --help` or a
   `--print-features` flag if one exists; else check the symbol via
   `strings | grep p2p_iroh` — document which). Print the build command for the
   pool slot: `CARGO_TARGET_DIR=<slot> RUSTFLAGS='--cfg getrandom_backend="custom"'
   cargo build --features "p2p p2p-iroh" --bin elohim-storage` (the
   `elohim/elohim-storage/justfile:28` shape).
3. `just test mesh`: the sprint report (`sprint-report-household-*.md/json`)
   gains a `transport: <mode>` line in its header so two runs are comparable;
   `status` prints the mode per peer (read `/p2p/status` — confirm a field
   exposes it, else read the captured environ).
4. `hc-mesh-prologue.sh`: no change expected; verify the prologue legs pass in
   `dual` (blob staging, stamp-server-projection-peers).

## DoD / verification

- `MESH_TRANSPORT_BACKEND=dual just mesh start && just mesh prologue` boots
  three peers whose logs show the iroh endpoint up (`p2p_iroh` target lines)
  and the libp2p swarm; `just mesh status` shows `transport=dual`.
- `MESH_TRANSPORT_BACKEND=iroh just mesh start` boots, and the report header
  says so. (Expect blob heal-on-read reds in iroh-only mode — that is the
  measurement this knob exists to make; record them, do not hide them.)
- `just test mesh genesis/a2o/features/dataplane/content-sync.feature` green
  in `libp2p` and `dual`.

## Disjointness

Scripts + justfile only. The Rust work it unblocks (iroh peer blob-fetch in
heal-on-read, custody push over iroh) is Opus-tier and tracked in the roadmap
plan, not here.
