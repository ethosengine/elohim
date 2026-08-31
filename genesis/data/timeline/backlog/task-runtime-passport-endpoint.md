---
id: "backlog-task-runtime-passport-endpoint"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: runtime passport — widen /version into a full node-local runtime report (service build, conductor version, DNA/wasm inventory, kernel/OS, transports, flags)"
slug: "task-runtime-passport-endpoint"
written: "2026-08-31"
author: "session-2026-08-31-velocity-snowball"
status: "open"
priority: "high"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [observability, mixed-version, delegable, codex-suitable]
---

**Claimable by any agent (Codex-suitable). Disjoint from the velocity-rung
lane by contract — see Disjointness below.**

## Why

Peers are about to be genuinely diverse (mixed coordinator versions by
design — the upgrade-propagation arc's steady state; household racks and
desktops after that). Debugging a cross-peer issue starts with "what is
actually running over there," and today only the storage binary answers
(`GET /version` → BuildInfo). The conductor version is invisible, the
installed DNA/wasm inventory is only reachable through the coordswap
dry-run, and kernel/OS/arch are reported nowhere. Live example: the
iroh-quinn GSO assert (backlog `iroh-quinn-gso-assert-crashes-storage`) is
kernel-x-library specific and the network cannot be asked who is exposed.

## Scope

Widen elohim-storage's `GET /version` into a **runtime passport** (all
read-only, node-local truth only — NO gossip/DHT changes, that half is a
separate p2p-design-gate decision):

1. New module `elohim/elohim-storage/src/runtime_passport.rs` that
   assembles one camelCase JSON document:
   - `service`: the existing `elohim_compute::BuildInfo` block, plus the
     compiled feature set (p2p / p2p-iroh) and the active transport
     backend(s).
   - `conductor`: mode (embedded|external|none), the conductor binary
     version — read it live over the admin websocket if the admin API
     exposes it, else report the `CONDUCTOR_IMAGE_TAG` env when present
     (see companion task `task-conductor-image-tag-runtime-visibility`),
     else `"unknown"`. Resolve the admin ws EXACTLY as
     `GET /db/p2p/conductor-diagnostics` does (embedded `admin_websocket`
     else `hc_registry.any_admin_websocket()`).
   - `happ`: per role — DNA hash, coordinator zome names → wasm hashes
     (same readback the coordswap report uses; reuse
     `happ_manager` helpers read-only, do not modify their signatures).
   - `host`: kernel + release + arch (`uname` via libc or std — no new
     heavy deps), container hint if trivially detectable.
   - `flags`: the boot-relevant env flags this node honors and their
     effective values (ALLOW_COORDINATOR_UPDATE,
     ELOHIM_OBEY_CARRIED_ELECTION, ELOHIM_ADOPT_BEFORE_AUTHOR, transport
     selection) — values only, never secrets.
2. `GET /version` keeps its current top-level BuildInfo fields
   byte-compatible (additive only — old consumers must not break) and
   gains the new blocks.
3. Doorway parity: `doorway-service`'s version route reports its own
   BuildInfo the same additive way (host + service blocks; it has no
   conductor/happ).
4. Unit tests: passport serialization (camelCase), additive-compat test
   asserting the legacy fields survive at the top level.

## Disjointness contract

- MAY create: `src/runtime_passport.rs`, tests.
- MAY edit: the single `GET /version` match arm in
  `elohim/elohim-storage/src/http.rs` (keep the diff to that arm), the
  doorway version route, module registration in `lib.rs`.
- MUST NOT touch: `happ_manager.rs` beyond read-only calls,
  Jenkinsfiles, `genesis/orchestrator/manifests/**`, `hc-mesh.sh`,
  `src/main.rs` env parsing, anything under `src/p2p*` — those are the
  active velocity-rung surfaces.

## DoD + verification

- `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS="" cargo test --features "p2p p2p-iroh" runtime_passport` green (echo `EXIT=$?`; plain cargo, no nextest).
- Default-features `cargo check` also green.
- Against a running local mesh peer: `curl -s localhost:8090/version | jq .` shows service+conductor+happ+host+flags blocks, and `jq .version,.commit` still answer (legacy compat).
