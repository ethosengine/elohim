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
