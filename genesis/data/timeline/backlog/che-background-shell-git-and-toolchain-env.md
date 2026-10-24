---
id: "backlog-che-background-shell-git-and-toolchain-env"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gates go unmeasured or unrun from background shells and worktrees: missing safe.directory, RUSTUP_HOME and a relative hooksPath"
slug: "che-background-shell-git-and-toolchain-env"
written: "2026-10-23"
author: "claude-opus-5-5"
status: "done"
priority: "high"
area: "devspace / pre-push / gates"
domain: "D6"
tags: [devspace, pre-push, worktree, toolchain, git, gates, foundations]
---

# Gates silently weaken outside the main interactive shell

Three defects were observed on 2026-10-23. Each one makes a gate refuse, report UNMEASURED, or not
run at all, depending on where the command started:

1. **Git ownership.** In this container git refuses `/projects/elohim` as "dubious ownership". The
   rust-style checker then reports `UNMEASURED` (exit 2). Habit discovery falls back to rglob, which
   was fixed in `epr_habits.py` the same day. The `index:` headline line reads `skipped`.
   Workaround: export `GIT_CONFIG_COUNT=1 GIT_CONFIG_KEY_0=safe.directory GIT_CONFIG_VALUE_0='*'`.
   The cure is a devfile-level git config, or matching UIDs.
2. **Toolchain environment.** Background shells lack `RUSTUP_HOME` and the `/opt/rust/cargo/bin`
   PATH, so the gate refuses the pinned 1.96.1 toolchain as not installed.
3. **Worktree pushes skip pre-push silently.** `core.hooksPath` is the relative `.husky/_`, which is
   not generated in a fresh worktree. A push from a worktree under `.worktrees/` therefore ran no
   gate and printed nothing (push of `a47bb95e0`). The cure: an absolute hooksPath, or a check that
   refuses a push when the hook directory is missing.

These matter more once two workspaces land continuously: each one is a way for a landing to look
gated when it was not.

## Resolution (2026-10-24)

- **Git ownership:** no current `.claude` SessionStart hook restores `safe.directory`.
  `devfile.base.yaml` now runs idempotent `setup-git` first in postStart, adding the global
  wildcard only when absent. `genesis/orchestrator/rust-style.sh` checks Git before pool
  discovery or Cargo and names `git config --global --add safe.directory '*'` and `setup-git`
  on ownership refusal. Proof: `devfile.test.mjs` runs setup twice against an empty global
  config; `rust-style/hook_test.py` forces a real dubious-ownership failure in both normal
  and `--hook` modes, asserting exit 2 and the cure without invoking Cargo.
- **Background Rust environment:** already declared on origin/dev: `b86b8c29c` introduced
  the shared base with container-level `RUSTUP_HOME=/opt/rust/rustup`, `CARGO_HOME` and
  `/opt/rust/cargo/bin` on PATH. Both `devfile.yaml` and `devfile_x86_v2.yaml` inherit it.
  The pinned che-devworkspaces image sources also declare these (`containers/rust-dev/Dockerfile`
  lines 68–70 and `containers/udi-plus-mem-rust-nix/Dockerfile` lines 71–73); no duplicate
  shell profile is needed. Proof: the devfile test passes the declared environment into a
  non-interactive background Bash with profiles disabled; the read-only
  `env RUSTUP_HOME=/opt/rust/rustup PATH=/opt/rust/cargo/bin:/usr/local/bin:/usr/bin:/bin rustup toolchain list`
  probe exits 0 and lists `1.96.1-x86_64-unknown-linux-gnu`.
- **Worktree hooks:** origin/dev already contains `a1da50f0d` (after `74e142fb7`), whose
  `package.json` prepare runs `husky && node .husky/hooks-path.mjs`; `9e461bfa1` documents
  the main-checkout hook serving worktree pushes. The remaining gap was accepting a
  directory with no executable shim, or skipping prepare entirely. `setup-git` now runs
  the same helper before install. The helper validates the main pre-push entrypoint and
  implementation, repairs incomplete shims from a prepared worktree, and otherwise selects
  the absolute tracked `.husky` directory. Missing hook code refuses loudly. Proof:
  `.husky/hooks-path.test.mjs` exercises real `git hook run pre-push` in linked worktrees,
  including absent/partial shims, absolute-but-broken paths and missing/non-executable hooks.

Verification: `node --test --test-isolation=none .husky/hooks-path.test.mjs genesis/orchestrator/devfile.test.mjs`
exits 0 (12 tests); `python3 -m unittest discover -s genesis/orchestrator/rust-style -p hook_test.py`
exits 0 (8 tests); `python3 -c 'import yaml,sys;yaml.safe_load(open(sys.argv[1]))' devfile.base.yaml`
exits 0. Node's default process-isolated runner exits 1 in this sandbox because child Node
console output through pipes is lost; file-backed fixture capture and isolation `none` retain
the same assertions. No Cargo builds, commits or pushes were performed.

Existing Che workspaces retain their flattened devfile: recreate/update from the shared base
to receive postStart changes. This patch does not mutate the running workspace's shared Git
configuration. The separately maintained che-devworkspaces devfiles are not changed.
