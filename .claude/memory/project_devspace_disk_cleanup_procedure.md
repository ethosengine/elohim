---
index: false
id: project-devspace-disk-cleanup-procedure
name: devspace-disk-cleanup-procedure
title: Devspace disk cleanup procedure
description: "Pool families dominate disk pressure; act at 85%+; reclaim ladder ends in operator-gated family prune — never prune the active family mid-push."
metadata: 
  node_type: memory
  type: project
  originSessionId: 6bd0f758-fe18-46cf-b0d0-8848acafeca0
cites:
  - genesis/agentic/pool-policy.json
  - .claude/hooks/cargo-disk-guard.py
---

Recreated 2026-06-04 (the original was graduated to MemPalace 2026-06-02, but live citations remained — `genesis/data/timeline/backlog/prepush-cargo-target-pool.md` and the operator's own muscle memory — so it earned re-canonization, refreshed with verified numbers).

**Thresholds:** `cargo-pool status` flags at 80%; act at **85%+** (preflight headline shows the same warn). The pool (`/projects/.cargo-target-pool`) is typically the dominant occupant (~320G of a 911G volume) — worktree `target/` dirs and node_modules are usually NOT the offenders anymore (stewardship reclaims them; expect 12K stamp dirs in-tree).

**Diagnosis (in order):**
1. `cargo-pool status` — families table + legacy-targets / stale-node_modules indicators
2. `du -sh /projects/elohim/.claude/worktrees/*/target` — usually empty
3. In-tree targets (`doorway/doorway-service/target`, `elohim/elohim-storage/target`) — 12K stamps unless a gate just cold-built

**Reclaim ladder (safe → destructive):**
0. **`cargo-pool enforce --yes`** — since 2026-06-04 the whole ladder below is automated + policy-driven (`genesis/agentic/pool-policy.json`: 150G pool cap, 75% soft / 85% hard watermarks, family dispositions). Runs automatically at SessionStart (async), Stop, and pre-push-under-pressure. Adds the biggest rung the manual ladder lacked: **stale artifact-hash GC** (superseded ~1GB test binaries — 71% of the measured pool, see [[rust-build-footprint-anatomy]]). Guarded: active family, live-PID worktrees, flock'd slots never touched; keep-warm families keep their freshest slot. Heavy cargo is DENIED at the hard ceiling by `.claude/hooks/cargo-disk-guard.py`. Manual rungs below remain for surgical/operator use:
1. `cargo-pool prune --stale-incrementals --yes` — GC incremental hash dirs >3d (often empty if all families warm)
2. `cargo-pool legacy-targets --clean --yes` and `cargo-pool node-modules --clean --yes`
3. **Family prune** — `cargo-pool prune family <name> --yes` for COLD families only. Rules:
   - Identify the ACTIVE family first: branch → family (`shift/*` → `shift`); a running push's gate builds use the active worktree's family — never prune it mid-push
   - A family is cold when its sessions are closed AND merged (e.g. `sprint` after the operator dev-merge)
   - **Which families count as cold is the operator's call — ask per family, don't infer** (2026-06-04: operator kept dev/e2e/design/elohim warm, pruned only sprint)

**Verified 2026-06-04:** pruning `sprint` (92.9G, post-merge) took the volume 86% → 75%.

**Gate caveat — RESOLVED 2026-06-04:** the pre-push hook now redirects ALL native Rust gates (storage/epr/doorway/steward + sweettest) into per-crate pool slots via `gate_pool_slot` (explicit ws_rel constants — the "do NOT use `cargo-pool key` dynamically" rule still holds, it mis-keys storage and offers slots for DNA workspaces that must stay un-redirected). Backlog item `prepush-cargo-target-pool.md` marked resolved. Sweettest is also now an integration-tier gate: default-runs only on pushes targeting dev/main (`RUN_SWEETTEST=1` forces elsewhere).

Related: [[multi-agent-pvc-pacing]], [[cargo-target-dir-for-native-builds]], [[pvc-threshold-and-recovery]]
