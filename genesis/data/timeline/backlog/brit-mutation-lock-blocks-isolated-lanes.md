---
id: "backlog-brit-mutation-lock-blocks-isolated-lanes"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "One repository-wide Brit lock, held for a whole push, stops every other worktree's Brit writes"
slug: "brit-mutation-lock-blocks-isolated-lanes"
written: "2026-11-12"
author: "claude-opus-5-5 (shem, host-compatibility K6 slice 2, session 3dd97e3d)"
status: "open"
priority: "high"
tags: [brit, daily-driver, worktree, concurrency, push, lock]
cites:
  - elohim/brit/gitoxide-core/src/repository/switch/linux/mod.rs
  - elohim/brit/src/plumbing/mutation.rs
---

# One repository-wide Brit lock, held for a whole push, stops every other worktree's Brit writes

## Observed (2026-11-11/12, shem)

A Codex session (01a1cbeb, Brit feedback sprint) ran
`brit push origin refs/heads/feat/brit-feedback-operation-20261111:refs/heads/dev` from its
worktree from about 23:33Z, and it was still in the pre-push hook at 00:40Z, more than an hour later.
For that whole time every Brit mutation in any other worktree of the same repository refused
at once:

```
Error: another Brit mutation is active
Caused by:
    Resource temporarily unavailable (os error 11)
```

Refused on shem, for a separate worktree on a separate branch: `brit worktree add -b … <path>
<commit>` (retried for three minutes) and `brit add -- <paths>`. Neither touched the other
session's branch, index or worktree. Both were done with Git instead, as declared fallbacks,
and so were the commit and the push of the feature branch, because the same lock was still
held. That loses the Brit safeguards the daily-driver experiment exists to exercise. Retrying
does not help: the refusal names no holder and has no wait.

## Why

`shared_guard` (`gitoxide-core/src/repository/switch/linux/mod.rs`) takes a non-blocking
exclusive `flock` on `<common-dir>/brit-mutation.guard`. `common_dir` is shared by every linked
worktree. `MutationGuards` (`src/plumbing/mutation.rs`) holds that guard until the invoking
command finishes, and for `brit push` that includes the whole pre-push hook, so an hour-long
gate holds every lane's writes.

## What safe isolated lanes need

1. **Scope the lock to what is mutated.** A worktree-local write (index, HEAD, checkout,
   `worktree add` of a new branch) takes a per-worktree lock. Shared state (refs, packed-refs,
   the object store, config, the pending switch/pull journal) takes the shared lock, and only
   for the moment of the shared write.
2. **Do not hold the shared lock across hooks.** `push` takes it for the ref and remote-tracking
   update and send-pack, not while pre-push runs; the hook's gates read a fixed commit, and
   push already sends an explicit object.
3. **Name the holder.** The refusal reports the holding pid, session (when an envelope exists),
   command and start time, read from the guard file, so a waiting agent can judge it.
4. **Offer to wait.** `--wait[=SECONDS]` (or an environment default) blocks on the lock instead
   of refusing, for scripts and agents that would otherwise fall back to Git.

Verify: with a `brit push` paused inside its pre-push hook in worktree A, `brit worktree add`,
`brit add` and `brit commit` in worktree B on another branch succeed; a ref update to the
branch A is pushing still waits or refuses naming A's holder.

## Related gaps seen in the same session (each its own small item)

- `brit fetch origin dev` reported pack negotiation but left `refs/remotes/origin/dev` at the
  old commit; the real head had to be read with `git ls-remote`.
- `brit submodule` offers only `list`, so a fresh worktree's pre-push preflight
  ("not checked out: elohim/brit elohim/rakia sophia") can only be cured with
  `git submodule update --init`.
- No `brit ls-remote`.
