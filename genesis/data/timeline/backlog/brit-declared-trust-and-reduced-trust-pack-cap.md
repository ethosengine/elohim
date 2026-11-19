---
id: "backlog-brit-declared-trust-and-reduced-trust-pack-cap"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "brit ignores command-scope safe.directory, so it runs at reduced trust and cannot open a repository with more than 32 pack indexes"
slug: "brit-declared-trust-and-reduced-trust-pack-cap"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [brit, gix, trust, safe-directory, daily-driver, gate-evidence, epr-cli]
cites:
  - genesis/data/timeline/backlog/brit-build-fingerprint-cannot-resolve-any-commit-in-monorepo.md
  - genesis/orchestrator/gate-runner.mjs
  - genesis/orchestrator/gate-attest.mjs
  - genesis/orchestrator/gate-evidence.mjs
  - .claude/hooks/pre-flight-check.py
  - .claude/scripts/_lib/epr_meta_git.py
  - elohim/eprfs/epr-cli/src/artifact.rs
  - elohim/eprfs/epr-cli/src/integrate.rs
  - elohim/eprfs/epr-cli/src/device/arrival_work.rs
  - elohim/eprfs/epr-cli/src/git.rs
  - elohim/eprfs/device-runtime/src/layout.rs
shift_objective: |
  Lane sprint3/trust is merged into sprint3/integrate (38427b921b). What remains is landing it on
  brit main and moving the pin (steward; see brit-sprint-3-handoff-2026-11-19.md).
---

# brit at reduced trust: command-scope safe.directory and the 32-slot pack cap

## Why brit stays at reduced trust

gix accepts `safe.directory` only from the system and global files. git also honours the command
scope (`-c`, `GIT_CONFIG_COUNT`/`KEY`/`VALUE`). So the trust the elohim tools pass to children reaches
brit-build-ref's git children but not brit itself, which stays at reduced trust: 32 pack-index slots,
a 16 MiB allocation limit, hooks refused. gix does not read `GIT_CONFIG_PARAMETERS`.

## What fails

With more than 32 pack indexes, brit 0.2.6 cannot open the repository at reduced trust:

- `brit log` → "The object database has too few index slots";
- `brit build fingerprint` → "could not resolve commit 'HEAD': The rev-spec is malformed … object
  database slots exhausted".

The monorepo on ethosengine had 33 pack indexes on 2026-11-19 and 36 later the same day (177 MB of
packs); shem had 28. Every `brit fetch` adds a pack and nothing consolidates. It works only while the
global `safe.directory = *` is present; after a workspace restart every brit read fails until setup
restores it. Cure today: `git repack -a -d`.

This is the reduced-trust leftover of
`brit-build-fingerprint-cannot-resolve-any-commit-in-monorepo.md`, whose full-trust case is fixed in
brit 0.2.6.

## Lane `sprint3/trust` (first reviewed at `d96ff65b2d`; merged at tip `bb0dd28837`)

- gix honours command-scope `safe.directory`.
- At reduced trust, slots are sized from the pack indexes present, up to a ceiling of 512, with one
  named error naming both cures.
- brit-epr's git children report real failures.
- One error printer, with no backtrace unless `BRIT_BACKTRACE=1`. `RUST_BACKTRACE` is set on these
  devices, so the last stderr line of a failed brit command was a stack frame, and every caller that
  reports the last line showed that instead of the cause.

It changes upstream-owned gix files (`gix/src/open/*`, `gix/src/config/tree/sections/safe.rs`), against
the letter of brit's CLAUDE.md ("Upstream-owned gix-* code stays byte-identical to upstream"). Earlier
`fix(gix)` commits such as `fe6da99ffb` touch the same files. The steward has been told. These are
candidates to offer upstream.

rakia-brit 0.1.1 (gix 0.81) has no 32-slot wall.

### Independent review (partial, 2026-11-19)

- No security defect in the trust decision. It agrees with git 2.52 across exact root,
  parent/child/sibling, wildcards, symlinks, linked worktrees, bare, submodules, repo-local config and
  includes, and hooksPath.
- One regression, being fixed: a multi-pack-index over more than 512 packs is refused at reduced trust
  where 0.2.6 opened it. The count must follow gix-odb's midx-aware accounting.
- Env-scope `includeIf` grants where git refuses (low).
- The error-printer commit and most tests are not yet reviewed.

Differences from git the review measured that pre-date the lane (not introduced by it):

- a `safe.directory` value `<R>/*` grants R itself in gix, where git requires strictly below;
- `GIT_CONFIG_PARAMETERS` is not read;
- with `GIT_DIR` set, git skips the ownership check and brit does not;
- `-c include.path=…` is not followed by gix.

What a grant unlocks: hooks, including a repository-chosen `core.hooksPath`; the editor in replay;
repo-local ssh and credential config. NOT external filters, which brit refuses.

### Merged on the integration branch, final review state (2026-11-19)

Merged into `sprint3/integrate` (merge `38427b921b` of lane tip `bb0dd28837`). Not on brit `main`, not
pinned.

- Three independent review passes; no security defect.
- The trust decision matches git 2.52 in all seventeen cases of the final table, including one 0.2.6
  got wrong: a global `*` followed by an empty value from the environment, where git and the new brit
  refuse and 0.2.6 granted.
- The multi-pack-index regression is fixed by counting through gix-odb's own collection
  (`Store::count_index_slots`).
- An env-scope `includeIf` no longer grants trust, and the re-read honours the caller's includes
  permission.
- `safe.rs` is byte-identical to upstream again.
- The last stderr line is the deepest real message, with gix-error's classification folded in as a
  suffix. No reporting path panics on a closed stderr.

Known differences, documented in brit at `docs/plans/2026-11-15-bounded-memory-backlog.md`: a
conditional include (`gitdir:`, `onbranch:`, `hasconfig:`) in a GLOBAL or SYSTEM file is evaluated
against the repository in gix, so the repository's own branch name or remotes can switch on an include
the user made conditional, and with it a `safe.directory` entry git would not honour. Same on 0.2.6. A
fix following the lane's environment pattern is about 40 lines in `gix/src/open/repository.rs` plus
tests, and is a candidate to offer upstream. Advice meanwhile: do not keep `safe.directory` behind a
conditional include.

**Non-root CI failure after the merge (run 44544779620, commit `e2b94f2da7`).** One test,
`brit-cli::repository_slots::an_unreadable_alternate_pack_directory_does_not_fail_the_open`, failed on
non-root Linux. Cause, established by reproducing as uid 65534: the TEST was unsound, not the product.
It opened its repository without process isolation, so a host-wide `safe.directory = *` (CI runners
appear to have one; inferred from the exact reproduction) raised it to full trust, where the object
store lists every pack directory at open (upstream behaviour, unchanged by the lane, same on 0.2.6)
and fails on the unreadable one. At reduced trust the open succeeds and the first lookup fails naming
the directory; 0.2.6 measured the same. Fix: test-only commit `bd18b171` on `sprint3/trust` (test
renamed `an_unreadable_alternate_pack_directory_fails_the_first_lookup_not_a_reduced_trust_open`, runs
isolated, asserts both trust levels), to be merged with the diff/show merge; not yet pushed when
written.

Pre-existing difference it surfaced: at FULL trust an unreadable alternate pack directory fails the
open in gix (0.2.6 too), where git prints an error and continues.

Not measured: the rev-parse exit-code table for brit-epr's `resolve_rev`; an unreadable pack directory
as a non-root user (the test skips as root; first exercised on non-root CI); the foreign-ownership
tests are root-only or `#[ignore]` and never run in CI.

Minimum git for brit-epr: 2.30 (`rev-parse --end-of-options`).

Release items: gix gained `REDUCED_TRUST_MAX_PACK_INDEXES` and gix-odb `Store::count_index_slots`,
so both need version bumps at release; brit-epr gained a public `diagnostic` module and a gix-error
dependency.

## Audit base

The gate-script and epr-cli audit below was run on a monorepo checkout at `5d53c7ac8`, 142 commits
behind origin/dev, so its original file:line references may have shifted. Each site is named here by
function, re-located on this branch (based on origin/dev); re-locate them again before acting on them.

## Inside a gate brit has no command-scope trust

The monorepo's gate-runner deliberately deletes `GIT_CONFIG_PARAMETERS` and
`GIT_CONFIG_COUNT`/`KEY`/`VALUE` from its children (`genesis/orchestrator/gate-runner.mjs`
`gateChildEnv`), because `git -c core.hooksPath=… push` leaked into fixture repositories. So inside a
gate brit runs with no command-scope trust. Reported to the session that owns those files, together
with these silent-degrade sites:

- `genesis/orchestrator/gate-attest.mjs` `worktreeDirty`: it returns
  `result?.status === 0 && (result.stdout || '').trim().length > 0`, so when git refuses (non-zero
  status) it returns false, "not dirty": a refusal reads as clean;
- `genesis/orchestrator/gate-evidence.mjs` `gitRead` and `signerIdentity`;
- `genesis/orchestrator/gate-runner.mjs` `reuseSelector`;
- `.claude/hooks/pre-flight-check.py` `check_git_status`;
- `.claude/scripts/_lib/epr_meta_git.py` `git`.

## epr-cli spawn audit

- `elohim/eprfs/epr-cli/src/artifact.rs` `brit()` spawns brit-build-ref (found by `brit_bin()`) with
  no trust env.
- `integrate.rs` `quality_gate` gives the gate child no trust env.
- `device/arrival_work.rs` `each_tool` builds a bin-store `Store` that keeps the plain runner, and its
  failure goes only to `arrival.log`.
- `git.rs` `discover_root` reports "could not find a git worktree" for a dubious-ownership refusal.

## Open for the steward

- Declared trust applies per exact checkout root, and trusted-checkouts on ethosengine lists
  `/projects/elohim` and one worktree, so it does not apply in other linked worktrees. This is a design
  question for the steward.
- `elohim/eprfs/device-runtime/src/layout.rs` `read_saved` reads `layout.json` (bin-store, state,
  cargo-pool, project-root paths) with no custody check. It is on shem's deliberate-roll list.
- The custody predicate checks writable-by-others, not owned-by-others.
