---
name: project-sync-state-contract
title: Sync-state contract — epoch, position, caught-up
description: The sync-state contract (2026-08-29) — every replication stream declares epoch/position/declared; caught-up is a comparison, None while unknown; station 1 = epoch in inventory sequence high bits
metadata:
  type: project
---

Spec: `genesis/docs/superpowers/specs/2026-08-29-sync-state-contract-design.md`; code: `elohim-storage/src/p2p/sync_state.rs` (registry row `SyncStreamState::caught_up`). Born from four 2026-08-29 defects that were one defect (implicit position, no epoch, guessed caught-up): inventory 9 % view, tail loss, publisher-restart collapse, half rows.

**Rules:** epoch before position (newer epoch supersedes, older is replay) · position monotone per epoch, arrival order not a contract (hold early pages, bounded) · caught-up = `position >= declared`, `None` while declared unknown — a rollup publishes null, never true · every state is a signal.

**Station 1 landed:** inventory `sequence` = `(boot_epoch << 32) | counter` (`boot_epoch` = secs since 2026-01-01; 68 y headroom). No wire field — mixed-version safe (old receivers see a forward jump). `PUBLISHER_RESTART_GAP` (200) stays as fallback for pre-epoch publishers. **Next:** per-publisher `SyncStreamState` on `/p2p/status`, `docsBehind`, `pull.epoch`, an answered `SnapshotRequest`. See [[project_content_sync_plane]].
