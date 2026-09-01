---
id: "backlog-task-release-channel-ceremony-driver"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Task: release-channel ceremony driver — author a channel, publish a release version, declare staging, promote to earned, and revert by re-election, as matthew's authorized runtime/device"
slug: "task-release-channel-ceremony-driver"
written: "2026-09-01"
author: "session-2026-09-01-rung5-design"
status: "open"
priority: "high"
jobs: [elohim-genesis]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-upgrade-propagation-p2p-design-arc"
  - "spec:runtime-artifacts-elected-content"
  - "backlog-task-release-manifest-schema-packager"
  - "backlog-task-runtime-upgrade-a2o-receipt"
tags: [upgrade-propagation, rung5, ceremony, canonical-head, election, delegable]
---

**Claimable by any implementation agent. Depends on T1
(`task-release-manifest-schema-packager`) for the manifest currency; consumes
its output file. The election machinery itself is landed — this task DRIVES
it for runtime channels, exactly as the developer/device ceremony already
drives it for epr-content.**

## Why

The spec's whole thesis is that "which release is canonical" is the SAME
ceremony as content head election. This task proves it: a workspace device
authorized as matthew's runtime (the rail proven 2026-08-30 for native content
sync) authors a runtime channel, publishes releases as versions, and moves the
head — staging declare, earned promotion, revert-by-re-election — with zero
new zome code.

## P2P design-gate decision

Carried by the spec §5: the channel content + declarations reuse
`content_store` authoring and `declare_canonical_content_head` /
`declare_earned_canonical_head` (three-arm authority; the MVP leans on the
bootstrap-steward/progenitor + `HeadDelegation` arms —
`elohim/holochain/dna/elohim/zomes/content_store/src/lib.rs:5431-5477`).
Nothing new is notarized beyond ordinary content + head links; Ephemeral (C)
script state only.

## Scope

1. `genesis/a2o/scripts/release-ceremony.ts` (tsx), composing the zome-call
   rails of `carried-election-mesh-proof.ts` rather than re-deriving them.
   Verbs:
   - `channel create <channelId> --reach <tier> --discipline <json>` —
     author the channel root content (metadata_json `kind:
     "release-channel"`).
   - `publish <manifest.json>` — author the manifest as a content version on
     its channel id (body = T1 manifest; metadata_json `kind:
     "release-manifest"`), then `declare` staging.
   - `promote <channelId> <releaseCid>` — earned declaration via the
     authorized arm.
   - `revert <channelId> <priorReleaseCid>` — earned re-declaration of the
     prior release; PRINT the arbiter's answer from a second peer to prove
     convergence, never assert from the declaring peer alone.
   - `status <channelId>` — resolve the canonical head from EVERY reachable
     peer's conductor and render the tier + cid table (partition honesty:
     unreachable ≠ absent).
2. Respect adopt-before-author: `publish` on a channel with an existing
   declared head must adopt/resolve first (the four-arm pre-flight in
   `services/head_adoption.rs` is the reference semantics — the script must
   not crown its own commit).

## Interface contract (consumed by T3, T6)

- Channel root + release versions are ordinary content ids — T3's controller
  needs ONLY the channelId string to watch a channel.
- `status` output is machine-readable JSON (one row per peer) — T6's receipt
  script consumes it.

## Disjointness contract

- MAY create `release-ceremony.ts`, edit this atom.
- MUST NOT edit Rust source, zomes, `hc-mesh.sh`,
  `carried-election-mesh-proof.ts` (frozen oracle), or sibling scripts.

## DoD + verification

- On a 3-peer mesh: create channel → publish two releases → `status` shows
  staging head on all 3 → promote → all 3 show earned on release-2 → revert →
  all 3 show earned on release-1. Two consecutive runs, fresh channel ids.
- A declare attempted WITHOUT the authorized arm (plain agent) is REFUSED and
  the refusal is printed — the negative control proving the gate, not luck.
