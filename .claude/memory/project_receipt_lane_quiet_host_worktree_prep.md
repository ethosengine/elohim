---
name: project_receipt_lane_quiet_host_worktree_prep
description: "Household receipt lanes miss under another session's cargo (2 dated misses 2026-09-23); fresh worktrees need element/client builds + rakia; mint after every commit"
id: project-receipt-lane-quiet-host-worktree-prep
metadata:
  node_type: memory
  title: Receipt lanes need a quiet host; worktree gate prep
  type: project
  originSessionId: 3a11e1a8-7c12-4116-80f9-fcf59b56702c
  modified: 2026-09-24T12:45:22.230Z
cites:
  - genesis/orchestrator/scripts/serving-receipt.mjs
  - app/elohim-app/scripts/hc-mesh.sh
---

**Receipt lanes need a quiet host.** On 2026-09-23 two consecutive `epr-app-deliverability` lanes on a fresh `/tmp/hh` household missed one step each while another session ran `cargo test` (24 cores, load average 10 then 27–30): first the doorway-B row waited past 75s on a DHT election that never settled; then three concurrent `storage-restart` calls outlived their caller's budget. The third lane on an idle host (load 3.5) passed 102/102 in 3m44s. Check `uptime` and `pgrep -x cargo` before casting; a lane started under load costs 30 minutes and proves nothing.

**Fresh-worktree gate prep** (a `git worktree add` from origin/dev is not gate-ready): `pnpm install --frozen-lockfile --prefer-offline` (own install, never a symlink); `pnpm --filter "./app/elohim-elements/**" --if-present run build` before the storybook gate; `pnpm --filter @elohim/storage-client build` before the seeder gate and a2o typecheck; `git submodule update --init -- elohim/rakia elohim/brit sophia` before the storage suite and the orchestrator suite (one storage test reads the rakia schema; since 2026-09-24 dev's gate registry loads brit/rakia/sophia as attested components from build-manifests INSIDE those submodules — an absent checkout throws `Cannot read properties of undefined (reading 'run')`). Then `git checkout -- app/elohim-elements` to drop the regenerated manifests. Also: the private flow plane (`.eprfs/status/{flows.jsonl,actors.jsonl,labels.json,index/,lenses/}`, gitignored) must be copied in or the memory-index parity test projects an empty index; the recall/ session records need not come along.

**Receipt identity is per component tree.** `serving-receipt.mjs` validates `sutParts` (tree hashes of `elohim/elohim-storage`, `doorway/doorway-service`, `genesis/a2o`, `app/elohim-app/scripts/hc-mesh.sh`, …) against the checkout you push from. Any commit touching those trees after minting — including a habit-atom delta under `elohim/elohim-storage/.epr-meta/` or `doorway/doorway-service/.epr-meta/` — invalidates it. Order: all commits, then mint, then push from that worktree. Use private `STORAGE_BIN`/`DOORWAY_BIN` copies so the shared pool slots are never the mesh's source of truth.

**Another session's mesh is not mine to stop.** The auto-mode classifier denies killing another session's prologue or stopping the household it started, even when it has hung for an hour; the operator runs `! just mesh stop` (and `pkill` of the hung tree) from their prompt, or grants it via /permissions.

**Why:** the receipt is the pre-push pawl for serving-path changes; wasted lanes and invalidated receipts were the whole cost of the 2026-09-23 doorbell/RTT integration.
**How to apply:** before `receipt-cast`, confirm the host is idle and every commit is in; when a lane misses, read the household logs for the step's real mechanism before rerunning. See [[project_cargo_pvc_disk_discipline]], [[feedback_pnpm_symlinked_node_modules_write_through]], [[feedback_push_branch_discipline]].
