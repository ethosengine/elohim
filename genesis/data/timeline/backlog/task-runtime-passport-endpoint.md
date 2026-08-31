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
tags: [observability, mixed-version, delegable]
---

**Claimable by any implementation agent. Disjoint from the velocity-rung
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

## P2P design-gate decision

- **Classification:** Ephemeral (C), node-local operational truth. Each response
  is reconstructed from the running process, conductor admin readback, and host;
  it creates no DHT entry/link, durable projection, or head-plane item.
- **Address/source:** the existing `/version` service endpoint is the lookup
  address; no content, agent, or transport identity is minted. The running
  components are authoritative for their own observed versions, while the
  response remains diagnostic evidence rather than network authority.
- **Protocol placement:** no integrity/coordinator zome, post-commit signal,
  SQLite/Automerge projection, or new route is added; the change is
  DNA-hash-neutral and enriches the existing T3 storage/T4 doorway projection.
  The excluded gossip/DHT passport digest would instead be an A2 attribute of
  node registration and requires its own design-gate decision.
- **Concern canon:** C0 is answered by node-local projection placement; C4 by
  explicit `unknown`/error values; C5 by evidence-not-authority semantics; C6a
  by a bounded conductor inventory read; C7 by reporting compiled/active facts;
  C8/C14 by preserving per-block errors; and C10 by additive legacy-shape pins.
  C1, C2, C3, C6b, C9, C11, C12, and C13 are n-a because this read-only report
  performs no election, transition, repeated effect, identity binding,
  admission, authorization, or authority graduation.

## Scope

Widen elohim-storage's `GET /version` into a **runtime passport** (all
read-only, node-local truth only — NO gossip/DHT changes, that half is a
separate p2p-design-gate decision):

1. New module `elohim/elohim-storage/src/runtime_passport.rs` that
   assembles one camelCase JSON document. Keep the existing BuildInfo fields
   flat at the top level (including the existing string-valued `service`);
   place every richer block under a new `passport` object:
   - `passport.service`: the existing `elohim_compute::BuildInfo` block, plus the
     compiled feature set (p2p / p2p-iroh) and the active transport
     backend(s).
   - `passport.conductor`: mode (embedded|external|none), the conductor binary
     version — read it live over the admin websocket if the admin API
     exposes it, else report the `CONDUCTOR_IMAGE_TAG` env when present
     (see companion task `task-conductor-image-tag-runtime-visibility`),
     else `"unknown"`. Resolve the admin ws EXACTLY as
     `GET /db/p2p/conductor-diagnostics` does (embedded `admin_websocket`
     else `hc_registry.any_admin_websocket()`).
   - `passport.happ`: per role — DNA hash, coordinator zome names → wasm
     hashes. Use the public installed-app/admin-websocket readback and, where
     necessary, reproduce only the pure installed-definition projection;
     `happ_manager`'s equivalent helper is private and belongs to the rung lane.
   - `passport.host`: kernel + release + arch (`uname` via libc or std — no new
     heavy deps), container hint if trivially detectable.
   - `passport.flags`: the boot-relevant env flags this node honors and their
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

- The delegated implementation agent (Codex or equivalent) MAY create
  `elohim/elohim-storage/src/runtime_passport.rs` and focused tests.
- It MAY edit only the single `GET /version` match arm in
  `elohim/elohim-storage/src/http.rs`, module registration in `src/lib.rs`,
  and the existing doorway version route for parity.
- It MUST NOT edit any other `http.rs` arm; `happ_manager.rs`; any Jenkinsfile;
  any deployment/orchestrator manifest; `hc-mesh.sh`; `src/main.rs` env
  parsing; `src/p2p/view_federation.rs`; or any other `src/p2p*` surface.
  Those are the rung lane's surfaces this week.

## DoD + verification

- `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test --manifest-path elohim/elohim-storage/Cargo.toml --features "p2p p2p-iroh" runtime_passport; echo "EXIT=$?"` is green (plain cargo, no nextest).
- `just gate elohim-storage` and `just gate doorway` are green.
- Against a running local mesh peer,
  `curl -s localhost:8090/version | jq '.passport, .version, .commit'` shows
  service+conductor+happ+host+flags under `passport`, while `.version` and
  `.commit` still answer at top level (legacy compatibility).
