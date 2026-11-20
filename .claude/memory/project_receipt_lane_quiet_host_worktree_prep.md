---
index: false
name: project_receipt_lane_quiet_host_worktree_prep
title: "project_receipt_lane_quiet_host_worktree_prep"
description: "Receipt lanes miss under another session's cargo (2 misses 2026-09-23); fresh worktrees need pnpm install, element/client builds and rakia/brit/sophia BEFORE the first push; mint after every commit."
id: project-receipt-lane-quiet-host-worktree-prep
metadata:
  node_type: memory
  title: Receipt lanes need a quiet host; worktree gate prep
  type: project
  originSessionId: 3a11e1a8-7c12-4116-80f9-fcf59b56702c
  modified: 2026-10-25T00:26:42.716Z
cites:
  - genesis/orchestrator/scripts/serving-receipt.mjs
  - app/elohim-app/scripts/hc-mesh.sh
---

**Receipt lanes need a quiet host.** On 2026-09-23 two consecutive `epr-app-deliverability` lanes on a fresh `/tmp/hh` household missed one step each while another session ran `cargo test` (24 cores, load average 10 then 27–30): first the doorway-B row waited past 75s on a DHT election that never settled; then three concurrent `storage-restart` calls outlived their caller's budget. The third lane on an idle host (load 3.5) passed 102/102 in 3m44s. Check `uptime` and `pgrep -x cargo` before casting; a lane started under load costs 30 minutes and proves nothing.

**Fresh-worktree gate prep** (a `git worktree add` from origin/dev is not gate-ready): `pnpm install --frozen-lockfile --prefer-offline` (own install, never a symlink); `pnpm --filter "./app/elohim-elements/**" --if-present run build` before the storybook gate; `pnpm --filter @elohim/storage-client build` before the seeder gate and a2o typecheck; `git submodule update --init -- elohim/rakia elohim/brit sophia` before the storage suite and the orchestrator suite (one storage test reads the rakia schema; since 2026-09-24 dev's gate registry loads brit/rakia/sophia as attested components from build-manifests INSIDE those submodules — an absent checkout throws `Cannot read properties of undefined (reading 'run')`). Then `git checkout -- app/elohim-elements` to drop the regenerated manifests. Also: the private flow plane (`.eprfs/status/{flows.jsonl,actors.jsonl,labels.json,index/,lenses/}`, gitignored) must be copied in or the memory-index parity test projects an empty index; the recall/ session records need not come along. Do all of it BEFORE the first push: on 2026-10-25 skipping it cost a 65-minute pre-push in which 15 legs refused, 11 of them for prep alone. Never push with `git -c safe.directory=*`: git hands `-c` to every child through `GIT_CONFIG_PARAMETERS`, so tests that assert an ownership refusal see trust and fail.

**Receipt identity is per component tree.** `serving-receipt.mjs` validates `sutParts` (tree hashes of `elohim/elohim-storage`, `doorway/doorway-service`, `genesis/a2o`, `app/elohim-app/scripts/hc-mesh.sh`, …) against the checkout you push from. Any commit touching those trees after minting — including a habit-atom delta under `elohim/elohim-storage/.epr-meta/` or `doorway/doorway-service/.epr-meta/` — invalidates it. Order: all commits, then mint, then push from that worktree. Use private `STORAGE_BIN`/`DOORWAY_BIN` copies so the shared pool slots are never the mesh's source of truth.

**A degraded canonical household is not required for the receipt (2026-10-02, campaign 1.4).** The rule binds source trees, household lane and five passed stations — not a `MESH_DIR`, peer count or chain length. Codex spent a day tuning observers against the 45,000-action Dowell household (34-minute runs, 0–1 of 5); a fresh three-peer `MESH_DIR=genesis/local-dev/<new>` mesh on the same binaries passed 102/102 in 4m53s. Stop the canonical household first (fixed ports), resume it after. Fresh-mesh prep that cost a pass each: build dists with `just dev package app/elohim-app` and `app/lamad` (a bare `ng build` skips `prebuild`, so the gitignored fonts are absent and the prologue refuses the bundle; revert the two tracked files it rewrites); build the hApp from the worktree (`just build` per DNA, then pack) rather than reuse an assembled bundle; and run `just gate` with the mesh STOPPED — the prologue's hosted cast puts ~12 GB on the doorway-A conductor (~700 MB per hosted agent) and the storage gate's compile is then shed by the RAM guard.

**Another session's mesh is not mine to stop.** The auto-mode classifier denies killing another session's prologue or stopping the household it started, even when it has hung for an hour; the operator runs `! just mesh stop` (and `pkill` of the hung tree) from their prompt, or grants it via /permissions.

**Why:** the receipt is the pre-push pawl for serving-path changes; wasted lanes and invalidated receipts were the whole cost of the 2026-09-23 doorbell/RTT integration.
**How to apply:** before `receipt-cast`, confirm the host is idle and every commit is in; when a lane misses, read the household logs for the step's real mechanism before rerunning. See [[project_cargo_pvc_disk_discipline]], [[feedback_pnpm_symlinked_node_modules_write_through]], [[feedback_push_branch_discipline]].
