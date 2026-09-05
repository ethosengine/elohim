---
name: project_devspace_recovery
title: Devspace/container recovery (umbrella)
description: "Devspace recovery: container restarts kill mesh + background agents and wipe /tmp and ~/bin (gh must be reinstalled); ethosengine I/O wedges are hard-NFS deadlocks, not node failure; a local-mesh WRITE STORM on the shared NVMe swaps the pod (controller lease lost) and drops every secret — stop/start heals."
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
- **Pod SWAP without secrets (2026-09-03) — CORRECTED the same evening:** the secretless ReplicaSet
  (`…-6f5fdb6c4f`, 7 volumes instead of 9) was rendered at 07:21Z while k8s-dqlite SEGV-crash-looped 7× and all
  three voter links (.110/.111/.112, the ThinkPads — ethosengine is a dqlite STANDBY) timed out at once; the
  devworkspace controller's lease PUT hit the 5 s apiserver timeout, lost election, and re-rendered cold without
  the labeled secrets. The 10:20Z swap (`…-njfz5` → `…-9rgzg`, `restartCount: 0`) was the Deployment flipping onto
  that stale RS; the "11 generations since Aug 16" are that ping-pong. The earlier "130–270 MB/s write storm"
  story was WRONG: sysstat had 8–12 MB/s / <1 ms await at 07:07 and 67 MB/s at 10:21 (day peak 115 MB/s); the
  in-container io.stat numbers were 3× inflated (md2 + both mirror members summed). Symptom inside: /tmp + ~/bin
  wiped AND `ANTHROPIC_API_KEY`/`GH_TOKEN`/`NPM_TOKEN`/sccache/Grafana/SonarQube/Jenkins env + `/etc/ssh/dwo_ssh_key`
  MISSING — gh/sccache/MCPs/ssh push fail "mysteriously". Cure: workspace stop/start (or delete the stale RS).
  Real I/O signal on this host is LATENCY (10:13Z await 81 ms, queue 52 at 38 MB/s; 2026-08-29 hard resets) on
  QLC NVMes past TBW — read host PSI, not MB/s. Separate 13:32Z event = power button on the shared APC. Read [[project_local_mesh_binary_slot_and_restart]]
  for the mesh start recipe; the 0.7 line adds a required local iroh-relay (`MESH_RELAY_BIN`).
- **HTTPS `git push` hangs after a pod swap (2026-09-03):** `/etc/gitconfig`'s credential helper reads
  `/.git-credentials/credentials`, which the re-rendered pod did not mount, so `git push` waits on a hidden prompt
  until the tool timeout. Cure without reinstalling gh: `GIT_ASKPASS=<script echoing x-access-token / $GH_TOKEN>
  GIT_TERMINAL_PROMPT=0 git push …` (pushed a78904105 that way).
- **After a pod swap, HTTPS `git push` hangs silently (2026-09-03, elohim-8f):** the image's git credential helper
  points at `/.git-credentials/credentials`, which the swap did not restore. Cure: `GIT_ASKPASS` returning `GH_TOKEN`
  (or push over SSH once `/etc/ssh/dwo_ssh_key` is back).

- **Git HTTPS auth after restart (2026-09-05):** `GH_TOKEN` survives but git ignores it — Che's /etc/gitconfig routes lookups to the empty mounted store and `gh` is gone. Durable fix: Che dashboard → User Preferences → Personal Access Tokens → add github.com token; the mounted secret syncs LIVE (~1 min, no restart) and Che's system store helper then serves every repo (done 2026-09-05, user=EthosengineBot). Stopgap if the store is empty: `git config --local credential.https://github.com.helper '!f() { echo "username=x-access-token"; echo "password=$GH_TOKEN"; }; f'` per repo.
- **VS Code "GitHub wants to sign in" prompt (2026-09-05):** same empty Che credential store — the built-in Git extension asks the GitHub auth provider on every fetch/push. Workspace `.vscode/settings.json` sets `github.gitAuthentication: false` + `git.terminalAuthentication: false`. The Che-side cure is User Preferences → Personal Access Tokens (github.com) or Git Services OAuth, which fills `/.git-credentials/credentials` after a workspace restart. gh 2.100.0 is baked into udi-plus from che-devworkspaces 30b5c86 (nightly cron 02:00 rebuild cascades to udi-plus-mem-rust-nix).
