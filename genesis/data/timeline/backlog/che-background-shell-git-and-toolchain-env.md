---
id: "backlog-che-background-shell-git-and-toolchain-env"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Gates go unmeasured or unrun from background shells and worktrees: missing safe.directory, RUSTUP_HOME and a relative hooksPath"
slug: "che-background-shell-git-and-toolchain-env"
written: "2026-10-23"
author: "claude-opus-5-5"
status: "open"
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
