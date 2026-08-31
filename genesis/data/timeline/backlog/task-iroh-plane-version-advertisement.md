---
id: "backlog-task-iroh-plane-version-advertisement"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: iroh-plane parity for peer version advertisement — the libp2p leg advertises service/version+commit via identify; the iroh leg advertises nothing"
slug: "task-iroh-plane-version-advertisement"
written: "2026-08-31"
author: "session-2026-08-31-velocity-snowball"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-task-runtime-passport-endpoint"
  - "backlog-upgrade-propagation-p2p-design-arc"
tags: [observability, iroh, transport-parity, mixed-version, delegable, codex-suitable]
---

**Claimable by any agent (Codex-suitable). Requires a short investigation
step before code — budget for it.**

## Why

On the libp2p leg, peers already learn each other's storage build: the
identify protocol carries `elohim-storage/x.y.z+<commit>` as user-agent
(`elohim/elohim-storage/src/p2p/behaviour.rs` ~line 496). On the iroh leg
there is no identify equivalent, so an iroh-only peer (the deployment
direction of travel) is version-anonymous to its peers. The 2026-08-31
frame-cap incident is the concrete cost: an old-reader/new-sender split
had to be diagnosed by log archaeology instead of by asking peers their
versions.

## Scope

1. INVESTIGATE (report findings in the PR/commit body): where the iroh
   plane already exchanges structured peer metadata — the gossip hello /
   peer-manifest exchange in `src/p2p_iroh/` (gossip_receive.rs,
   config.rs, shard_backend.rs) — and pick the LEAST invasive existing
   message to ride. Do not invent a new protocol/ALPN for this.
2. Add an OPTIONAL, additive `userAgent` (or equivalent) field to that
   existing exchange: `#[serde(default, skip_serializing_if = "Option::is_none")]`
   — old peers must decode new messages and new peers must decode old
   ones. Follow the repo's mixed-version wire discipline (additive
   serde(default) + byte-identity pin tests, see
   `elohim/elohim-views/src/shared.rs` wire-compat tests for the
   pattern).
3. Surface what was learned: store the last-seen user-agent per peer
   wherever peer records already live in that module, and expose it on an
   EXISTING peer-listing debug/status surface if one is already served
   (read-only; do not add a new HTTP route — the passport task owns the
   HTTP surface).
4. Wire-compat tests: old-message-decodes / new-message-with-field
   round-trip, byte-identity for the unchanged shape.

## Disjointness contract

- MAY edit: `elohim/elohim-storage/src/p2p_iroh/**` and its tests.
- MUST NOT touch: `src/p2p/view_federation.rs` frame-cap constants,
  `src/http.rs`, happ_manager, reconcile_peers, Jenkinsfiles, manifests,
  hc-mesh.sh.

## DoD + verification

- `CARGO_TARGET_DIR=/projects/.cargo-target-pool/family/dev/elohim__elohim-storage/dev RUSTFLAGS="" cargo test --features "p2p p2p-iroh" p2p_iroh` green (echo `EXIT=$?`).
- Default-features `cargo check` green (module is cfg-gated — the 2026-08-31 cross-ref trap: never reference `crate::p2p_iroh` from always-compiled code).
- Wire-compat tests demonstrate both decode directions.
