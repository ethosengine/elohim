---
name: project_devspace_recovery
title: Devspace/container recovery (umbrella)
description: "Devspace recovery: container restarts kill mesh + background agents and wipe /tmp and ~/bin (gh must be reinstalled); ethosengine I/O wedges are hard-NFS deadlocks, not node failure."
metadata:
  node_type: memory
  type: project
---

# Devspace/container recovery (umbrella)

Folds the devspace/container failure + recovery-drill cluster. Members:

- [[project_container_restart_recovery_drill]] — The devworkspace container restarted twice on 2026-08-21 (~21:38, ~00:45 UTC), killing mesh + all background agents; /tmp and /home/user/bin wiped. Drill below.
- [[project_ethosengine_wedge_nfs_hardmount]] — ethosengine wedge root cause = hard NFS4 mounts to in-cluster ClusterIP (server pod on SAME node); bites when diagnosing node hangs or rebooting ethosengine
- [[reference_gh_cli_install]] — gh vanishes with the ephemeral container; restore the Go binary to /home/user/bin; GH_TOKEN (EthosengineBot, repo+admin:org) auths it; curl REST needs no install

**2026-08-25 restart (uptime reset mid-chain):** `/tmp` wiped (scratchpad, `$MESH_DIR`, background-task
logs), the running background chain died silently (task notification `stopped`, no marker), and git
answered `fatal: detected dubious ownership` until `git config --global --add safe.directory '*'`.
Everything under /projects (worktree edits, cargo-pool slot binaries) survived. Devfile postStart
re-runs `cargo install --path elohim/eprfs` (load ~20 for the first minutes).
- **Stale cargo sparse-index after restart (2026-08-25):** `CARGO_HOME=/opt/rust/cargo` is image-baked, so a
  restart resets `registry/index/index.crates.io-*/.cache` to the bake (Aug 23). Any pin newer than the bake
  (doorway `holochain_conductor_api =0.7.0-dev.23`) then fails `cargo fetch/clippy` with "failed to select a
  version … candidates 0.3.0-beta-dev.32" even though "Updating crates.io index" prints and crates.io has it —
  cargo trusts the stale per-crate cache files. Cure: `mv registry/index/index.crates.io-*/.cache
  .cache.stale-<ts>` and re-run `cargo fetch --locked` (65 MB, rebuilt in a minute). Crates already in
  `registry/cache` (storage's 0.6 family) are unaffected, which is why the storage gate passed and doorway's
  did not.
