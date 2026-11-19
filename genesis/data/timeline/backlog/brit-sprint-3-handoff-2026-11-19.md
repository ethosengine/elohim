---
id: "backlog-brit-sprint-3-handoff-2026-11-19"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Brit sprint 3 handoff: where the lanes are, what is decided next, what needs the steward, and the integration runbook"
slug: "brit-sprint-3-handoff-2026-11-19"
written: "2026-11-19"
author: "claude-opus-5-5 (brit sprint 3, session 0e0924d5, steward of record human:matthew)"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [brit, handoff, daily-driver, integration, shem]
cites:
  - CLAUDE.md
  - genesis/data/timeline/backlog/brit-push-fails-after-green-hook-root-cause.md
  - genesis/data/timeline/backlog/brit-rebase-cost-gitlinks-and-stranding.md
  - genesis/data/timeline/backlog/brit-declared-trust-and-reduced-trust-pack-cap.md
  - genesis/data/timeline/backlog/brit-porcelain-gaps-census-2026-11.md
  - genesis/data/timeline/backlog/brit-ci-and-workspace-limits-found-in-sprint-3.md
  - genesis/data/timeline/backlog/brit-mutation-lock-blocks-isolated-lanes.md
  - genesis/data/timeline/backlog/brit-covenant-session-segment-aliasing.md
  - genesis/data/timeline/backlog/brit-daily-driver-handoff-2026-10-27.md
shift_objective: |
  Continue brit sprint 3 in the decided order: recovery round 5 (tighten the admission gates
  until tests/cli-journey/tests/memory_replay.rs passes unchanged), its independent review and the
  non-root leg, then push 10 of the integration branch with Phase 4g; then the recovery lane's
  queued follow-up round (every journal record published whole, truthful --status) under its own
  independent review; then the redesign, after the steward's format decision; then the push-policy
  lane, the epr-cli lane, worktree-side diff with gitlinks, and merge porcelain.
---

# Brit sprint 3 handoff (2026-11-19)

Read this first. Sprint session `0e0924d5`; steward of record human:matthew. The other device is shem
(session `b49f7c46`).

The brit repository is the submodule `elohim/brit`. Sprint base: brit 0.2.6 =
`1db7a955197b10bc7672c9aa544768a329bb9008`. Nothing has landed on brit `main`, and the monorepo pin
has not moved.

## Where the work is

**Integration branch** `sprint3/integrate` in the brit repo, worktree
`/projects/elohim/.worktrees/brit-s3-integrate`. Pushed to the brit remote as
`run-ci/sprint3-integrate` nine times, through `brit push` and brit's reach gate (`epr ready` +
`scripts/ci/rust-style.sh`). The first three pushes took about 6 s each; the fourth exited 0. Pushed
head: `080f3057973e6c5c19e38a1b78a1330917d3851d` (push 9). Local head:
`948a939f16e416d6357448e49eb2a4ac318e2f9d` (Phase 4g, the recovery merge), NOT pushed and not to be
pushed as it stands (recovery entry in the lane table).

| Pushed head | CI run |
|---|---|
| `251d10e52a` | 44469917351 |
| `da5e4a0504` | 44477093766 |
| `7479ce4539` | 44481834878: finished. Every Linux and macOS job green. The two cross-platform compile fixes worked (Windows built and ran about 5,670 tests). Red: four tests on both Windows runners, all from the build-inputs lane, plus cargo-deny-advisories (red on main too) and therefore `Tests pass` |
| `bd28ecd8cf6c9593c77825d734e7fb38e1923534` | 44494603484: every required job green on Linux, macOS and both Windows runners; only cargo-deny-advisories red (as on main) |
| `9945331577` | 44528984349: green on Linux and macOS. On both Windows runners ONE unit test red, `gitoxide_core::repository::git_spellings::rev_parse::tests::relative_paths_match_git`: `relative_path` rendered a collected PathBuf with `display()`, so Windows joined with `\` where the test asserts git's `/`. The journeys for the same cause had been gated before the push; this unit test was missed |
| `e2b94f2da7dc11597222bd7074ebc657deb7cb28` | 44544779620: finished. Every Windows job green on both runners (first Windows run of the trust lane's tests; `relative_paths_match_git` passes unchanged). Non-root Linux red on ONE test, `brit-cli::repository_slots::an_unreadable_alternate_pack_directory_does_not_fail_the_open`: the test was unsound, not the product (see `brit-declared-trust-and-reduced-trust-pack-cap.md`); test-only fix `bd18b171` on `sprint3/trust`, to be merged with the diff/show merge, not yet pushed when written. macOS cancelled part-way, so unverified for that head. The aggregate "Tests pass" concluded `success` on this run although `test-fast` legs had failed (see the steward section) |
| `8e5072ffd35cd31d7cfaf8d31fa6b28ae53dbda1` | 44567300304: finished (last job 17:19:14Z). 29 jobs succeeded, among them every Linux and Windows test job, both `test-fixtures-windows` jobs and `test`. Not green: cargo-deny-advisories (failure, as on main) and `test-fast (macos-latest)` (cancelled 16:47:51Z with no steps: it never got a runner, the leg the diagnostic step printed as `abandoned`). The pre-fix `Tests pass` concluded `success` at 17:09:53Z all the same (see the steward section) |
| `1bf96648cbec09baf9a957477ea36d975aa7c1bf` | 44575030094: GREEN, the first run with the aggregate fix. Finished 17:58:03Z (`Tests pass` 17:55:51Z). 30 jobs succeeded; the only non-green job is cargo-deny-advisories. `test-fast (macos-latest)` got a runner and passed (17:26:05Z), the first macOS result since `9945331577`; `test` passed; `Tests pass` succeeded, its diagnostic step printing 17 needs, every one `success`, and the allow-list step silent. Same product tree as `8e5072ffd3` plus the allow-list step |
| `080f3057973e6c5c19e38a1b78a1330917d3851d` | 44583007601 (pushed 17:59Z; the read-only merge). Attempt 1, finished 18:46:21Z: 27 succeeded; `test-journey` failed at 18:00:52Z in its `extractions/setup-just` step ("Server Error"), no journey ran; `test-fast (macos-latest)` cancelled 18:14:29Z without a runner; cargo-deny-advisories failed as on main; `Tests pass` FAILED (18:36:53Z), the allow-list refusing the abandoned-macOS shape the old aggregate passed on run 44567300304 (push 7). Attempt 2 (failed jobs re-run, 18:47:00Z–19:22:13Z): `test-journey` succeeded (all four bash variants and the journeys, memory step included); macOS cancelled again (19:02:04Z); `Tests pass` failed (19:22:12Z). Attempt 3 started 19:26:08Z; read at 19:34Z: `test-fast (macos-latest)` queued for a runner, cargo-deny-advisories failed again, no `Tests pass` yet. So this head is green on everything that ran and unverified on macOS; `1bf96648cb` remains the only commit with a complete green aggregate |

A second integrator took over from the runbook note below and did the phase from `7479ce4539` to
`bd28ecd8cf`. Besides the lane merges, that phase made two integration commits: `4f1d05a93a` (one
fixture test gated to Unix) and `bd28ecd8cf` (`FingerprintError` marked non-exhaustive).

A third integrator took over from the same note for the refs-shem and trust merges; the note has a
"Phase 4 (second integrator)" section. Merged in that phase:

- **refs-shem:** merge `b9b4a294fc` of `806ea8114b`. Integration commits: `b2cc6435fc` (census and
  guard-scope journey for the spellings and `-C`), `b6451a07be` (changelog), `9945331577` (gating of
  `rev_parse.rs`, `change_dir.rs` and `ls_remote_matches_git` to Unix).
- **trust:** merge `38427b921b` of lane tip `bb0dd28837`, no textual conflict. Integration commits:
  `d3452c7816` (a failure reported to a closed stderr keeps its code and is recorded:
  `brit log no-such-rev` and `brit branch delete no-such-branch` exited 101 before, 1 after);
  `5e4327182c` (the read-only list's doc sentence); `1764e989ee` (changelogs); `c55525d236`
  (`rev-parse`'s relative answers print `/` on every platform: `--show-toplevel`, `--git-dir`,
  `--git-common-dir` after `--path-format=relative`, and `--git-common-dir` without a format when it
  answers relative); `e2b94f2da7` (eight ssh-refusal snapshots updated for the new error layout,
  which the lane had left stale).

Merged after that:

- **diff-show:** merge `19ac68ad59` of `5a422ceb51`. Integration commits: `c2d017f72f` (census and
  guard journeys for show and diff, and `-C` keeping diff/show's `--`); `1a0aef4526` (changelog);
  `8e5072ffd3`, the merge of `c1e3580088`, which corrected two stale expectations in
  `tests/cli-journey/tests/native_authoring.rs` that the lane had never run (`diff -- <untracked>` now
  exits 0 like git; `diff --cached` over a staged gitlink move now prints git's `Subproject commit`
  patch).
- **trust follow-up:** `69aa5275b3`, merge of `bd18b171cb`.
- **aggregate fix:** `4dceac3557` and `1bf96648cb` (see the steward section).
- **read-only extension (shem):** merge `93287e5f19256ca281c4dca348e2afdabb2b554b` of
  `6f5a82f130180d34293c99c2901663d207026bac` (parents `1bf96648cb` and `6f5a82f130`), by a fresh
  integrator, no conflict and no integration fix; changelog commit `080f305797` is the head. What it
  delivers, how it was verified and what it leaves: "What the merged lanes deliver", read-only entry.

- **recovery (Phase 4g, local only, NOT pushed):** merge `b1010c4540c791a6a81d9cd573797d714e25ff2f`
  of `sprint3/recovery` `c63ef6bfa4` into `080f305797`, no conflicts (`brit-cli/CHANGELOG.md` and
  `tests/cli-journey/tests/cli_surface.rs` auto-merged; the lane's base is `9945331577`), then
  `b0770fdeadcaae2d8d39d2a5172fcbcc332b691d` (docs/rebase.md section "States this version can still strand" and its changelog
  entry) and `948a939f16` (the lane's changelog entries). Blocked from push 10 by `memory_replay.rs`
  (recovery entry in `brit-rebase-cost-gitlinks-and-stranding.md`).

Eleven lanes are merged and pushed on the integration branch: covenant-escape, log-rev-path,
branch-tag, fetch-tracking, lock, build-inputs, push-transport, refs-shem, trust, diff-show,
readonly-shem. The twelfth, recovery, is merged locally only.

Verification of the ten-lane head on this host (before the read-only merge; the merge's own legs are
in the read-only entry): full cli-journey 496 passed on the default build, 357 on
the max-pure journey list; a non-root leg over 123 test binaries (1028 ok; the six differences from
root are host artifacts: this host's `/etc/gitconfig` is unreadable to an unprivileged user, and three
tests write a lock into a read-only worktree); the reviewer's three diff/show comparison scripts re-run
on the MERGED binary show only the documented refusals.

**Lanes** (each in its own worktree under `/projects/elohim/.worktrees/`):

| Lane branch | Tip | State | Worktree |
|---|---|---|---|
| `sprint3/covenant-escape` | `4ef74edb69` | merged into `7479ce45` | `brit-s3-covenant-escape` |
| `sprint3/log-rev-path` | `71bbb86aa3` | merged | `brit-s3-log-rev-path` |
| `sprint3/branch-tag` | `99caa43f` | merged | `brit-s3-branch-tag` |
| `sprint3/fetch-tracking` | `06616a06` | merged | `brit-s3-fetch-tracking` |
| `sprint3/lock` | `63173728` | merged | `brit-s3-lock` |
| `sprint3/build-inputs` | `0feb451a`; Windows follow-up tip `a3a874d4` (commits `7810a60e`, `4f1dd2c8`, `a3a874d4`) | merged; follow-up merged in `bd28ecd8cf` | `brit-s3-build-inputs` |
| `sprint3/trust` | `bb0dd28837`; follow-up `bd18b171cb` | merged (`38427b921b`); follow-up merged (`69aa5275b3`) | `brit-s3-trust` |
| `sprint3/push-transport` | `4a60a61d` | merged in `bd28ecd8cf` (three implementation rounds, two independent reviews, a doc-link fix) | `brit-s3-push-transport` |
| `sprint3/diff-show` | `5a422ceb51` | merged (`19ac68ad59`) after two review passes and a delta review | `brit-s3-diff-show` |
| `sprint3/refs-shem` | `806ea8114b` (shem's `b8264afc4`) | merged (`b9b4a294fc`) | `brit-s3-refs-shem` |
| `sprint3/readonly-shem` | `6f5a82f130` (five patches from shem's `origin self/nt-review-receipts` `ee7d60ecc65a`, applied with `git am` on `8e5072ffd3`) | merged (`93287e5f19`) | `brit-s3-readonly-shem` |
| `sprint3/recovery` | round 1 up to `0d23297e22`; round 2 `9f3e6373`, `073cbfb0`, `5b107d72`; round 3 `b4183e93b9`, `e47ac0bbcd`, `a9fd9cc80a`; round 4 `a9fd9cc80a..c63ef6bfa4fa520364ebf4f1d8318aa307db68d9` (seven commits) | four independent reviews; merged LOCALLY as Phase 4g (`948a939f16`), NOT pushed: the lane's admission gates make `memory_replay.rs` (untouched by either side) go from 8 passed / 1 ignored to 2 passed / 6 failed. Round 5 (tighten the gates) started 19:35Z in the integration worktree; then its own review and the non-root leg before push 10. The follow-up round publishing every journal record whole is queued, not started (see `brit-rebase-cost-gitlinks-and-stranding.md`, Phase 4g and round 5) | `brit-s3-recovery` |

**Monorepo side:** branch `feat/ark-declared-trust` (worktree `brit-s3-ark-trust`), first written as
three commits on origin/dev `01e4de7f1`, then four (tip `d14e7357f`), reviewed. dev landed at
`71069b1f551f` (the other ethosengine session's one push: L2b's eight commits, then L1's twenty-five)
and moved on to `0f48e02fbf43`.

- **Rebased and verified, still local.** `feat/ark-declared-trust` = `285e8513b` on dev
  `0f48e02fbf43` (commits `0c989cf91`, `4bdc2b7ca`, `f41477076`, `285e8513b`); backup branch
  `backup/ark-declared-trust-pre-rebase-2` at `d14e7357f`. No conflicts; `git patch-id --stable`
  identical for all four; the nine files' content identical to the backup (tree diff empty, checked by
  the coordinator); the last commit message now says 47 cases. The vector file is unchanged by a
  re-mint (the mint run exits 101 by design).
- **Verification.** epr-cli 1714 passed / 1 ignored (1692 before the rebase); the ignored
  foreign-group test passes as root. ark 67 passed / 2 ignored; the two root-only tests pass. clippy
  and fmt clean; `[rust-style] PASS`. The fleet-roll guard, run as pre-push runs it
  (`GATE_PUSH_TARGETS=refs/heads/dev GATE_PUSH_REFS="refs/heads/dev <HEAD> <base>" node
  genesis/orchestrator/gate-preflight.mjs --fleet-roll`), prints no row and exits 0 for the range,
  and refuses dev's `cde35de1b` as a control. Non-root (uid 65534, both HOME modes): every test of the
  branch passes; ark 67 ok in all three legs. Failures found that are not this branch's:
  `brit-ci-and-workspace-limits-found-in-sprint-3.md`, item (q).
- **Before landing.** origin dev has since moved to `c0b1ec5e3` (gate code, pool policy and docs
  only), so one more trivial rebase precedes any landing. The other session on this device
  (`a222db7c`, landing L2a in `elohim/eprfs`) confirmed over berth (notice 18:02Z, reply 18:04Z,
  answer 18:33Z) that L2a touches none of the branch's files and depends only on
  `git::trust_env(&Path)` and the read helpers keeping their signatures; they do. Asked of the other
  device at about 19:30Z and answered by it at about 19:33Z (reported by the other device):
  - Custody: its device key directory is mode 2700 and the declaration `trusted-checkouts` mode 600,
    both uid 0 gid 1234, a 17-byte regular file, and its process holds that group. By the branch's
    predicate it passes, so `trust_env` keeps applying the declared trust there. Its repository holds
    32 pack indexes, exactly the reduced-trust cap, so a withheld trust would fail its next read.
  - Linked worktrees: its declaration names only the main checkout and every landing there is made
    from a linked worktree. Read at `285e8513b` against its base: `git::checkout()` is not changed by
    the branch (the nearest ancestor with a `.git` entry, so a linked worktree is its own root) and
    matching is still "that root equals one whole line of the declaration". An undeclared linked
    worktree gets no declared trust before or after the branch; it works on both devices because git
    accepts it by ownership. Whether declaring a main checkout should cover its linked worktrees is
    the open steward item, not decided by this branch.
  - Lane: the branch waits behind the other device's landing L2a. If L2a has not been handed back to
    that device by 21:30Z, this branch lands first, with one line to it and to `a222db7c` on berth
    ahead of the push. Either way: rebase onto dev as it then stands, run the fleet-roll check on the
    range, land by fast-forward through the gated path, leave the peer daemon serving. That landing
    does not move the brit pin or touch brit `main`.
- The queued epr-cli `device_store(…)` lane stacks on this branch.

**This record** is on monorepo branch `docs/brit-backlog-sprint3` (worktree `brit-s3-backlog`).

### Review state of the lanes merged in this phase

- **refs-shem** (merged): shem delivered two fix rounds after two independent review passes on its side. Its
  reviewer found, among others, a change that could send a mutating command to an outer repository;
  it was removed in patch 0013. Sixteen patches are applied as `sprint3/refs-shem` `806ea8114b`
  (shem's `b8264afc4`), merged as `b9b4a294fc`. A deliberate difference
  from git is kept: `rev-parse --verify missing` with a closed stderr exits 128 where git dies with
  141.
- **trust** (merged as `38427b921b`, its test-only fix `bd18b171cb` as `69aa5275b3`; green on CI
  runs 44567300304 and 44575030094; final review state in
  `brit-declared-trust-and-reduced-trust-pack-cap.md`): three independent review passes, no
  security defect; the trust decision matches git 2.52 in all seventeen cases of the final table. The
  multi-pack-index regression and the env-scope `includeIf` grant the first pass found are fixed
  (trust entry).
- **diff-show** (merged; final state in `brit-porcelain-gaps-census-2026-11.md`): the
  first, partial review read: `brit diff`: 1,155 of 1,230 comparisons byte-identical to
  git, the other 75 clean refusals from one file carrying a `diff=<driver>` attribute; worktree forms
  100% identical except refusals for intent-to-add entries and a conflicted index; shallow merge-base
  264/264. `brit show` is verified on ordinary commits only (122 of 132 identical, 10 driver
  refusals); merges, tags and encodings are still under review. Limits recorded in
  `brit-porcelain-gaps-census-2026-11.md`.

## What the merged lanes deliver

- **covenant-escape:** see `brit-covenant-session-segment-aliasing.md` (status 2026-11-19), including
  its residuals.
- **lock:** see `brit-mutation-lock-blocks-isolated-lanes.md` (status 2026-11-19).
- **push-transport:** see `brit-push-fails-after-green-hook-root-cause.md`.
- **log-rev-path:** `brit log [REVISION]... [--oneline] [-n N] [-- PATH...]` with `A`, `^A`, `A..B`,
  `A...B`, `A^@`, `A^!`. With paths it follows git's default history simplification, and a trailing
  slash means directory only. Bare `brit log` now stops at a shallow boundary (monorepo: 12,839
  commits, equal to git; 0.2.6 listed 12,851). Without paths the set is exact reachability; it differs
  from git only where clock skew makes git's limiter stop early. Tie order for identical timestamps is
  unspecified without paths. Also `diff tree --name-only`, `--merge-base`, and one-argument
  `A..B`/`A...B` (ported from shem's patch `f35d1b753`).
- **branch-tag:** `brit branch delete|-d [-f]`, `-D`, `move|-m`; `brit tag [-a] [-m] [-f] <name>
  [<commit-ish>]`, `tag delete|-d`. Tag mutations previously bypassed the mutation guard. Differences
  from git: a rename of `a`→`a/b` when only the renamed ref is in the way is refused (rename through a
  temporary name); no `-M`, no `-r`; multi-name deletes are all-or-nothing; a rename writes no HEAD
  reflog entry; `brit tag -a x -m ""` is refused.
- **fetch-tracking:** `brit fetch <remote> <branch>`: the first positional that is a configured remote,
  URL or repository path is the remote when `-r` is absent. A destination-less refspec also updates the
  configured remote-tracking ref. A single-ref refspec the remote does not advertise is an error. Any
  rejected ref update exits non-zero. `brit pull --ff-only` works in a shallow repository when the
  decision is provable above the shallow boundary, and otherwise refuses, naming the boundary commit
  and `brit fetch --deepen/--unshallow`. Refusals that change nothing leave no pending journal. Rebase
  still refuses on shallow repositories.
- **build-inputs:** `brit build fingerprint` keys a submodule gitlink (path + pinned commit) when a
  step glob matches it or could match beneath it, and a symlink (its own object id) on the same rule.
  Gitlink-free, symlink-free steps keep byte-identical fingerprints. When a brit carrying this is
  pinned, 16 monorepo steps re-key once (measured at monorepo `5d53c7ac8` across 25 manifests):
  cargo-build-storage (input_count 3168→3170: gitlink `elohim/rakia` + symlink
  `elohim/sdk/schemas/current`), build-edge-image, build-conductor-image, app build-angular, genesis
  schema-codegen / schema-validate / validate-seeds / constants-sync, orchestrator lint-jenkinsfiles,
  dna schema-dna / build-dna-wasm / build-happ / dna-extern-lints / manifest-hygiene / sweettest-check.
  Not keyed: the content of a symlink's target, unless a glob also matches the target. The same hole
  remains in SELECTION in rakia-core 0.1.1 `constellation::plan_from_changes` (a pin-only or link-only
  change selects nothing); that is in `elohim/rakia`, in no write-set. Also in this lane: `cc`
  1.2.65→1.11.1 in brit's Cargo.lock, so C dependencies compile at `-march=x86-64` when no
  `-C target-cpu` is given (sqlite3.o went from 13,131 VEX/BMI instructions to 0). With
  `-C target-cpu=x86-64-v2` (release and bin-store builds) cc passes no `-march`, so those builds still
  need their CFLAGS, and brit-cli's build probe refuses without them.

  **Windows follow-up** (merged in `bd28ecd8cf`). CI run 3 showed four Windows failures, from two
  causes: (1) a test helper formatted a Windows path into a command string that `gix_testtools::git`
  splits, and the splitting ate the backslashes; (2) `globset`'s default makes a backslash an escape on
  Unix and a path separator on Windows, so the same manifest glob compiled to a different matcher per
  platform. Cure:
  - Every step glob compiles through one function, `compile_step_glob`, with all five globset options
    set explicitly to the Unix defaults. Linux/macOS keys are byte-identical: 25 monorepo manifests
    identical, cargo-build-storage still 3170 inputs, CID
    `bafyreiflnjxisflzqdrpy2seqmj4y6sy63wcjyznxboodamtq76i226jca`.
  - A pattern with a backslash directly beside `**` is refused on every platform with `InvalidGlob`,
    naming pattern and position (globset's `parse_star` consults the host separator rule there
    whatever the options say).
  - A tree path containing a backslash is refused on non-Unix hosts with
    `FingerprintError::UnportableTreePath` (there globset rewrites `\` to `/` in candidates). On such a
    host, one such path anywhere in the repository refuses every step's fingerprint.
  - Fixtures are index-only.

  Evidence: an independent reviewer emulated Windows with a patched globset copy (the old code
  reproduces exactly the two misdecided rows and the 0-of-7 count CI showed) and fuzzed 118,522,235
  patterns: zero accepted-and-not-refused patterns compile differently; 78,434 were refused, of which
  49,343 would in fact compile identically (over-refusal; no manifest in the monorepo contains a
  backslash, 224 checked). Not run on real Windows before the push.

  Still open: rakia-core 0.1.1 `constellation.rs` `build_glob_set` compiles the same globs with plain
  `Glob::new`, so step SELECTION by `brit build plan`/`affected` keeps the per-platform meaning and has
  no gitlink/symlink reach (`elohim/rakia`, in no write-set).
- **readonly-shem** (the other device's lane, merged as `93287e5f19`). Source: five patches written
  against `8e5072ffd3`, in `genesis/docs/superpowers/sprints/2026-11-17-native-throughput/receipts/brit-s3-readonly/`
  on `origin self/nt-review-receipts` `ee7d60ecc65a`, with a README, a before/after table and the
  lane's own independent review receipt. Commits: `4aa01ebcd34d3d0afb98f7866e87ecd0d9abf37a` (build
  output goes through the watched writer), `be1cee434309205af12f189389fb2343e6a1dadf` (admit the
  checked read-only commands), `a313829d8f96b986d59f8d4c558fdbd5c0800d35` (`tree info` resolves its
  revision first), `274ffa04f6e986648878866cde336b1fef76ccf5` (narrow the list; flush stdout before the status is decided),
  `6f5a82f130180d34293c99c2901663d207026bac` (admit `remote refs` and `remote ref-map` alike with `ls-remote`).
  - **What it does.** The closed-reader rule (a read-only command that otherwise succeeded exits 141
    when its reader closed; a command's own failure always wins) now covers 46 exact command paths, 28
    of them new: show, tag list, remote url, remote refs, remote ref-map, revision previous-branches,
    verify, fsck, tree entries, tree info, commit-graph list, commit-graph verify, mailmap entries,
    mailmap check, attributes query, exclude query, bare submodule, config, config show, config list,
    index entries, dirwalk; and the build forms graph discover, graph show, affected, plan, fingerprint,
    baseline read. Left out, each with its reason beside the list (`READ_ONLY_COMMANDS` in
    `brit-cli/src/main.rs`): bare tag, commit verify, commit describe, revision list, attributes
    validate-baseline, submodule list, is-clean, is-changed, config fmt, build run, build baseline
    write and migrate. The caveat that `ls-remote`, `remote refs` and `remote ref-map` may run a
    credential helper, which stores or erases a credential, is written in `brit-cli/CHANGELOG.md` at
    `080f305797` (line 149).
  - **Measured** on the merged binaries; default and max-pure gave the same result in every row.

    | Case | Before | After |
    |---|---|---|
    | `show HEAD`, stdout closed | 1, "Error: Broken pipe" | 141, empty stderr |
    | `build fingerprint`, stdout closed | 101 (panic) | 141 |
    | `index entries`, stdout closed | 0 | 141 |
    | `show HEAD:<short blob without a newline>`, stdout closed | 0 | 141 |
    | `tree info no-such-revision`, stderr closed | 1 | 1 (the base did not list `tree info`; the other device's table showed 141 from its own intermediate commit, and it has corrected its record) |
    | `build baseline write`, stdout closed | 101 (panic) | 1, "error: io error: Broken pipe", the ref already written |
    | `verify`, stderr closed | 0 | 0 |

    Genuine failures keep their own status with a closed reader (1 both live and closed):
    `-f json config list`, `remote -n nosuch refs`, `build baseline read --repo no-such-dir`,
    `mailmap check` with no contacts. With a live reader, output is byte-identical before and after on
    both builds (the `generatedAt` field masked) for build fingerprint, graph show (JSON and DOT), plan
    (`--files` and `--since`), graph discover, affected and baseline read.
  - **Read by the integrator.** The flush is at `brit-cli/src/main.rs:571-575`, after `run()` and
    before `exit_status()`, on the read-only branch only, its result ignored. `exit_status`
    (`:416-431`) gives 141 only for success, a plumbing exit of 0, or an error that is itself a typed
    BrokenPipe write. The closed-stderr ordering from `d3452c7816` is intact. Path strings are built
    from clap's canonical names (`read_only_command`, `:366`), so aliases resolve to the listed path.
    Six admitted commands re-read for "writes nothing, prints only through the watched writer": remote
    url, config list/show, index entries, verify, build fingerprint, build plan. None writes and none
    takes a guard.
  - **Verification on this host** (every leg EXIT 0): brit-cli 110 passed / 5 ignored; cli-journey
    full 499 / 0 failed / 73 ignored; brit-epr + brit-build-ref 177; gitoxide-core 199 / 5 ignored;
    brit-graph 54; power_loss 4; core faults 181 / 1 ignored; journey faults 113; max-pure brit-cli
    110 / 5 ignored; max-pure journeys 372 / 6 ignored; clippy for the four workspace legs plus
    `-p brit-cli --all-targets` under small, lean-async and max-pure; both doc legs with
    `-D warnings`; fmt. Non-root leg (uid 65534; plain HOME and HOME with `[safe] directory = *`;
    default and max-pure): 117 ok, 0 failed, 5 ignored in all six runs, no per-test difference from
    root. Reach-gate probes against `1bf96648cb`: rust-style PASS, `epr ready` READY.
  - **Not established.** The bash journeys for the pure, async and max variants ran here only up to
    their first `git://` section: this host has no git-daemon, and the local-only skip earlier
    integrators applied in `tests/helpers.sh` (runbook §4) was refused by the session's automatic
    permission check this time and was not worked around. The small variant ran fully (62
    assertions). CI covers this: on run 44583007601 attempt 2, `test-journey` ran all four variants
    green, and Windows and 32-bit were green; macOS is unverified for this head (head/push table).
    The new journey file `read_only_closed_reader.rs` is `#![cfg(target_os = "linux")]` and
    runs in `test-fast` only (whether to add it to `test-journey`: "Decided but not started", item 3).
  - **Follow-ups the other device accepted as its lane's, none blocking.** A completed `build baseline
    write` or `migrate` exits 1 on a closed stdout after its ref changed; both devices agree this
    belongs with the push-policy lane's rule "a delivered operation reports success" and is decided
    there together with push, not patched in the list. The deprecated `rakia` alias returns before the
    read-only flag is set (`brit-cli/src/main.rs:440-451`), so `rakia fingerprint | head` should still
    end 1 (read, not measured). `remote -n <unknown> url` exits 0 printing the name as a URL (predates
    the lane). With the Windows separators for rev-parse's absolute answers, the other device says
    that is the whole of what its lane still owes brit.

## The sprint's backlog entries

- New: `brit-push-fails-after-green-hook-root-cause.md`, `brit-rebase-cost-gitlinks-and-stranding.md`,
  `brit-declared-trust-and-reduced-trust-pack-cap.md`, `brit-porcelain-gaps-census-2026-11.md`,
  `brit-ci-and-workspace-limits-found-in-sprint-3.md`.
- Updated with a 2026-11-19 status section: `brit-mutation-lock-blocks-isolated-lanes.md`,
  `brit-covenant-session-segment-aliasing.md`, `brit-daily-driver-handoff-2026-10-27.md`.
- Found already fixed before this sprint (status section added):
  `brit-build-fingerprint-cannot-resolve-any-commit-in-monorepo.md`,
  `ci-brit-worktree-restore-cancellation-test-races.md`, `brit-reach-gate-requires-packages-root.md`.

## Decided but not started, in order

1. The rebase recovery/stranding lane, in the order decided after the cost report in
   `brit-rebase-cost-gitlinks-and-stranding.md`: (a) the stranding fixes (report §7 a–d) plus a
   preflight that refuses before any spooling when completion is impossible; (b) the redesign (report
   §6), after the steward's decision on the retained-manifest format below. Rebase stays a declared
   git fallback on both devices. Step (a) is lane `sprint3/recovery`: reviewed four times, merged
   locally as Phase 4g, not pushed; round 5 (tightening the admission gates) is running (lane table).
   Its queued follow-up round (every journal record published whole, truthful `--status` for
   unreadable records) is part of step (a) and needs its own independent review.
2. Eleven reviewed lanes are merged and pushed on the integration branch (listed under "Where the
   work is"); recovery is merged locally only.
3. A push-policy lane (`push.rs`): one handshake when no hook ran; the notes/tag fast-forward rule;
   refuse an ungated push when hooks are configured but missing or not executable; render the delivery
   note in the CLI. See `brit-push-fails-after-green-hook-root-cause.md`. Also for this lane: push's
   status lines use `writeln!(stderr, …)?` (`gitoxide-core/src/repository/push.rs`), so a DELIVERED
   push with a closed stderr exits non-zero. The same rule, "a delivered operation reports success",
   is to decide `build baseline write` and `migrate`, which exit 1 on a closed stdout after their ref
   changed (read-only entry under "What the merged lanes deliver").
   - Closed-pipe leftovers: the read-only merge routed brit-cli's build output through the watched
     writer, so `brit build fingerprint` no longer panics (101 → 141) and `build graph discover` is on
     the list. Not established as changed by it: the `println!` sites counted earlier in
     brit-build-ref (21), brit-verify (7) and brit-cli (3); no brit binary installs a panic hook. `brit log` prints a commit message with an
     `encoding` header as raw bytes (`show` is fixed on the diff-show lane; `log` is separate code);
     `show::to_utf8` is private, so this needs it made `pub(crate)`. `-C` refusals name the clap id
     `change_dir`.
   - CI's `test-journey` job runs its named list against a standalone default-feature binary, not the
     shipped max-pure one. Eleven journey files added this sprint are not in that list (broken_pipe,
     change_dir, check_ref_format, diff_revisions, for_each_ref, ls_remote, merge_base_is_ancestor,
     rev_list, rev_parse, show, read_only_closed_reader), though test-fast runs them; whether to add
     them is the steward's call.
   - 35 first-party test functions open a repository in-process without isolation from host git
     config (brit-graph repo_fingerprint 21, gitoxide-core replay 11, others): the class that produced
     the non-root CI failure. Listed in the integration note, not fixed.
4. The epr-cli lane: a `device_store(…)` constructor that always attaches `DeviceRunner`, and a
   source-scan test, stacked on `feat/ark-declared-trust` after shem's L2b lands; `bin.rs` waits for
   the landed sha. See `brit-declared-trust-and-reduced-trust-pack-cap.md` (epr-cli spawn audit).
5. Worktree-side diff with gitlinks.
6. Merge porcelain.

## What needs the steward

- **A brit pin move does not select an Edge build in CI today (observed).**
  - Observed: the last brit pin move was monorepo commit `1602399eead0`, "chore(brit): pin brit 0.2.6
    (1db7a955)" (0.2.0 to 0.2.6, committed 2026-11-17T21:01:08Z, one path: `elohim/brit`); the one
    before it is `e239f339369b`, "chore(brit): pin brit 0.2.0 (e70fc8ad)", 2026-11-15. The id
    c64967822 quoted by the other device for this commit is not on the remote; the id here was read
    from the path's history on dev. The orchestrator run that covered it is Jenkins
    `elohim-orchestrator/dev` #2359, whose checkout is `bea497110028` (2026-11-17T21:12:11Z, eleven
    minutes after the pin commit). Its changed-files tree lists `elohim/brit`; it prints "3 external dep(s) to unmanaged
    pipelines (Sprint 2): elohim-brit:brit-ci, elohim-rakia:rakia-ci, elohim-sophia:build-sophia-umd";
    its plan table shows the `[elohim-edge]` block with `build-edge-image │ SKIP │ no changes` (shem
    reports every step of that block SKIP); and its only dispatch line is `Triggering elohim-eprfs/dev`.
    Found by shem; the coordinator re-read lines 753, 828, 1079, 1082 and 1308 of that build log
    through the Jenkins MCP on 2026-11-19, and they say what is quoted.
  - So a brit pin move is not a fleet roll; it does dispatch the eprfs pipeline. Still unobserved, same
    mechanism by the code: rakia and sophia pin moves. What DOES select Edge: a holochain-conductor pin
    move (its manifest is in the superproject; `fleet-roll-guard.mjs` refuses it without
    `[edge:roll]`).
  - How to check any future pin move: search the orchestrator build log for the `║ [elohim-edge]` block
    and the `Triggering` lines.
  - The remaining preconditions of a brit pin move are the steward's word and an attestation that means
    something (next item).
  - How the question arose: `build-edge-image` in `elohim/holochain/build-manifest.json` declares `depends` on
    `cargo-build-doorway`, `cargo-build-storage`, `elohim-holochain:build-happ`, `elohim-brit:brit-ci`
    and `elohim-rakia:rakia-ci` (read on this branch). CI's `build-graph.groovy` propagates staleness
    through `depends`; the JS walker `genesis/orchestrator/graph-walker.mjs` does not (and ignores
    dotfiles), so a by-hand walker check shows nothing for a pin move. The ark branch was re-checked
    against `depends`: Edge depends on nothing from elohim-eprfs or elohim-ark.
  - But brit's manifest (`elohim/brit/build-manifest.json`) lives inside the submodule. Two
    independent readers on shem concluded from the Jenkinsfile and `build-graph.groovy` that CI's
    checkout does not see a submodule's manifest, so CI does not carry a brit or rakia pin move into
    Edge today.
  - That reading had not been checked against a real CI run, and it contradicted the standing note that
    a pin move rolls the fleet. The observation above settles it for brit.
  - Since dev `0f48e02fbf43`, a pre-push check `genesis/orchestrator/fleet-roll-guard.mjs` (on dev at
    `0f48e02fbf43`; not on this branch, which is older) refuses a push to dev or main whose commits
    would make CI select an Edge build or deploy step, unless a pushed commit message carries
    `[edge:roll]`. It REFUSES a holochain-conductor pin move (manifest in the superproject) and only
    WARNs for a brit or rakia pin. It reads git objects only, so on a shallow clone a range reaching the
    shallow boundary with no declaration is refused as undecidable (cure: `git fetch --deepen=<n>
    origin`). A `[skip ci]` or `[edge:validate-only]` tip over inputs a pipeline builds gets a WARN that
    CI dispatches nothing for that push.
- **A brit pin must name a commit whose own CI run included the aggregate fix.**
  - Setup: brit's `build-manifest.json` declares its gate as `run.kind: attested`, provider
    `github-checks`, repo `ethosengine/brit`, check "Tests pass". `genesis/orchestrator/gate-attest.mjs`
    takes the most recently started check run of that name at the pinned commit and passes on
    `success`, with no local recipe.
  - What was seen: on brit CI run 44544779620 (commit `e2b94f2da7`) the one check run named "Tests
    pass" (id 135658475517, started 2026-11-19T15:12:32Z) concluded `success`, although in the same run
    and single attempt `test-fast (ubuntu-latest)` and `test-fast (ubuntu-24.04-arm)` had concluded
    `failure` (14:49–14:50Z) and `test-fast (macos-latest)` `cancelled` (14:48Z). The aggregate's first
    step is `if: contains(needs.*.result, 'failure') || … 'cancelled' || … 'skipped'` → `exit 1`, and it
    did not run.
  - CAUSE: when one leg of a matrix job never gets a runner, GitHub reports that whole job to
    dependants as `needs.<job>.result == "abandoned"`, a value that is none of `failure`, `cancelled`,
    `skipped`. brit's aggregate `Tests pass` failed only on those three, so it passed.
  - Evidence: on run 44567300304 (head `8e5072ffd3`) a diagnostic step printed every need as `success`
    except `"test-fast": { "result": "abandoned" }`. `test-fast (macos-latest)` shows in the jobs API as
    `cancelled` 15 minutes after start (16:32:49Z → 16:47:51Z) with no steps and no log, exactly as on
    run 44544779620 (14:32:05Z → 14:48:12Z), where the same abandoned leg hid two FAILED ubuntu legs of
    the same matrix job. The earlier "last leg to finish" pattern was a coincidence and is withdrawn.
    On run 44567300304 the pre-fix `Tests pass` again concluded `success` (17:09:53Z) over the
    abandoned macOS leg.
  - FIX on the integration branch: commit `4dceac3557` (the diagnostic step `Show dependency results`)
    and commit `1bf96648cb` (a step `Fail unless EVERY dependency succeeded`: an allow-list that fails
    on any result other than `success` and on an empty result list). The older deny-list step is kept
    because `scripts/ci/test_qualify_readiness.py` reads its `'skipped'` clause.
  - CONSEQUENCE (in the other device's words, agreed): the monorepo's gate already passes a pin only
    when the named check's conclusion is exactly `success` (`gate-attest.mjs`), so the hole was wholly
    in what the aggregate reports. Therefore a brit pin should name a commit whose OWN CI run included
    the allow-list step. A `Tests pass` from any commit built before it cannot be trusted either way,
    even where its legs were in fact green; that covers every brit commit on main today, including the
    pinned 0.2.6.
  - FIRST COMMIT THAT MEETS IT: `1bf96648cbec09baf9a957477ea36d975aa7c1bf`. Its own CI run
    44575030094 included the allow-list step and is green on Linux, macOS and Windows with ten lanes
    merged (head/push table). It is on `run-ci/sprint3-integrate`, not on brit `main`. The other device
    has recorded the same commit and run on its side. Landing on main and moving the pin remain the
    steward's decisions; nothing has been done toward either. The later head `080f305797` (eleven
    lanes) is green on everything that ran and unverified on macOS (run 44583007601, three attempts;
    head/push table), so `1bf96648cb` remains the only commit with a complete green aggregate.
  - Still proposals for the steward, not done: add the job `test` (the unit-tests recipe) to the
    aggregate's `needs` (it also failed on run 44544779620 and is not in `needs` at all; it passed on
    44575030094, whose aggregate printed 17 needs without it); on the monorepo side, read every
    workflow run's conclusion at the commit as a second check (queued by shem behind its gate lanes, to
    land together with the manifest's attestation field). Read on GitHub: the run conclusion of
    44575030094 is `failure`, as is 44567300304's, solely because cargo-deny-advisories is red, so that
    second check as worded would refuse every brit commit while the advisory job stays red.
  - macOS: abandoned on runs 44544779620 and 44567300304 (from about 14:30Z), it got a runner and passed
    on 44575030094 (17:26:05Z), then was cancelled without a runner on attempts 1 and 2 of 44583007601.
    The fix makes an abandoned leg FAIL the aggregate, as it did on both those attempts.
- Landing on brit `main` and the pin move. Candidate commit: previous item. 16 monorepo steps re-key once (build-inputs above); one
  supervised real landing is needed to retire the push exception in `CLAUDE.md`. Its notes step is the
  device's own notes publish, fast-forward to `refs/notes/brit/devices/<did>/build/*`, which brit must
  be able to perform (or the publish script keeps using git, stated as a fallback); see
  `brit-push-fails-after-green-hook-root-cause.md`. The landing procedure changed during this sprint:
  re-read it from `CLAUDE.md` on origin/dev at landing time.
- Decision: in the rebase redesign, the retained-manifest format change (v2 with a floor) would apply
  to EVERY new copy, not only shallow ones as scoped earlier, so an older brit fails closed on any copy
  a newer one wrote. This reverses the approved scope; see
  `brit-rebase-cost-gitlinks-and-stranding.md`.
- Landing `feat/ark-declared-trust` on dev: READY (rebased onto `0f48e02fbf43` and verified, local
  only; one trivial rebase onto origin dev `c0b1ec5e3` first; the landing-lane and custody-check
  questions to the other device are open; see "Monorepo side" above). Not landed.
- `git repack -a -d` in the monorepo.
- The upstream-owned gix changes in `sprint3/trust` (now merged; `safe.rs` is byte-identical to
  upstream again).
- Trust for linked worktrees.
- The local-only commit `5d53c7ac8`.
- brit-graph (0.2.1) gained an error variant and `#[non_exhaustive]` on
  `brit_graph::fingerprint::FingerprintError`, so it needs a version bump at release.
- brit's crate releases are recorded in `brit-epr/CHANGELOG.md`; `brit-cli/CHANGELOG.md` does not
  record brit-graph API changes.
- gix gained `REDUCED_TRUST_MAX_PACK_INDEXES` and gix-odb `Store::count_index_slots`, so both need
  version bumps at release; brit-epr gained a public `diagnostic` module and a gix-error dependency.
- A write-set for `elohim/rakia`, for one of two items: the rakia-core selection hole needs one;
  nothing is needed there for the slot wall.
- The spawn audit in `brit-declared-trust-and-reduced-trust-pack-cap.md` was run on a monorepo
  checkout at `5d53c7ac8`, 142 commits behind origin/dev. Its sites are now named by function and were
  re-located on this branch; re-locate them again before acting on them.
- The six new standalone brit entries from this sprint are not yet listed in
  `genesis/data/timeline/backlog/CLUSTERS.md`. No brit cluster exists, and the census entry
  (`brit-porcelain-gaps-census-2026-11.md`) in particular is research-derived and belongs in a cluster
  by the convention. To be done when this branch lands, or by whoever owns the clusters file.

## Coordination with shem

- Hand-over ref: `origin self/nt-review-receipts`. shem's two refs-lane fix rounds have arrived
  (shem's `b8264afc4`, applied as `sprint3/refs-shem` `806ea8114b`).
- The refs lane (merged) brought `merge-base --is-ancestor` and `rev-list --count`.
- shem's read-only extension, together with the build commands' output through the watched writer,
  arrived as five patches on `self/nt-review-receipts` `ee7d60ecc65a` and is merged (`93287e5f19`).
- Still shem's, for sprint 4: Windows `/` separators for rev-parse's ABSOLUTE answers (helper
  `gix_path::to_unix_separators_on_windows`), and the read-only lane's three accepted follow-ups
  (read-only entry under "What the merged lanes deliver"). shem says that is the whole of what its lane
  still owes brit.

## Integration runbook

The text below is the integration worktree's handoff note, copied in full from the session
scratchpad. Its `/tmp/claude-0/…/scratchpad/integrate/` paths (the `$S` scripts, summaries, logs and
`bin-keep` binaries) are session scratch and will not survive a workspace restart. The scratch build
directory `dev-nocflags` it mentions has been deleted. A second integrator took over from this note
and did the phase that merged push-transport and the build-inputs Windows follow-up; later integrators
merged refs-shem, trust, diff-show and readonly-shem from it, and recovery locally as Phase 4g (not
pushed). The procedure stays valid; the facts in §1, §3 and §9 that changed are corrected in place.
Phase 4g's full cli-journey leg (§3) found the `memory_replay.rs` red that the recovery lane's four
reviews, none of which ran CI's memory step (ci.yml:480), had missed.

# Handoff: brit sprint 3 integration

You are taking over the integration branch for brit sprint 3. Brit is a gitoxide fork ("covenantal git"), written in Rust.

This note covers:
- where the branch stands;
- how to build and verify it on this host;
- the traps that already cost a CI run each;
- the open watch items.

Unless stated otherwise, every path below is absolute, and every script lives in
`/tmp/claude-0/-projects-elohim/0e0924d5-52e0-4709-b329-ea2463064148/scratchpad/integrate/`, called `$S` below.

```bash
S=/tmp/claude-0/-projects-elohim/0e0924d5-52e0-4709-b329-ea2463064148/scratchpad/integrate
```

## 1. Where things stand

**Location**
- Worktree: `/projects/elohim/.worktrees/brit-s3-integrate`. Work only here.
- It is a linked worktree of the brit repo, whose common dir is `/projects/elohim/.git/modules/elohim/brit`.
- Never edit lane worktrees (`/projects/elohim/.worktrees/brit-s3-*`), `/projects/elohim/elohim/brit` or any lane branch.

**Branch and commits**
- Branch: `sprint3/integrate`.
- Base: `1db7a955197b10bc7672c9aa544768a329bb9008` (brit 0.2.6).
- Head: `080f3057973e6c5c19e38a1b78a1330917d3851d` (eleven lanes), pushed to
  `origin run-ci/sprint3-integrate` as push 9. Every pushed head and its CI run: the head/push table at
  the top of this entry.
- `branch.sprint3/integrate.britReachBase` is set (by the coordinator).

**Repo state**
- `core.hooksPath` is `.githooks`, set by the coordinator's `epr setup`; never run `epr setup` yourself.
- Every commit prints `Brit attribution UNMEASURED: EPR lookup failed or timed out (requires session commit-trailers --json)` from `prepare-commit-msg`. It does not block the commit. End every commit message with `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`.
- `origin/main` is `fd6c86b363`, five commits past base. A dry merge onto it is clean (`git merge-tree --write-tree HEAD origin/main` exits 0). Do not rebase onto main or merge main into this branch.

First-parent history, oldest first (2nd parent = lane tip):

| commit | what |
|---|---|
| 06e832d57f | Merge covenant-escape (88230fe4c3): one ref component per covenant session |
| 35baff29c6 | Merge log-rev-path (ba8d2b94fd): `brit log [REV] [--oneline] [-- PATH…]` |
| f972e96b5f | Merge branch-tag (99caa43f87): branch -d/-D/-m, tag create/delete. Conflict: the `#[expect(unsafe_code)]` reason in src/plumbing/main.rs |
| 4e0ee29475 | Merge fetch-tracking (d97a765db9): tracking updates, positional remote, shallow pull |
| afc4646c41 | docs: changelog + parity note (root CHANGELOG part later reverted) |
| 0e94c0641a | Merge lock (920c3c5ddd): GuardScope, --wait, holder naming |
| a6b78d9c89 | fix: branch rename takes each rewritten worktree's guard (+ mutation_lock rename journey) |
| 476e5ab845 | test: rename moves every `[branch "old"]` section like git |
| 0550e554ee | fix(fetch): refuse to attribute ref updates when counts disagree |
| 1b9d277d68 | feat(brit-cli): CLI census records subcommand short/long flag spellings + hidden |
| 866a525fd4 | docs: root CHANGELOG reverted to upstream bytes; brit-cli/CHANGELOG.md created |
| eeb84e3853 | fix: `validate_local_history_above_shallow_boundary` gated on blocking-client (small clippy) |
| 34bdd83a08 | docs(brit-epr): no private intra-doc links in put_covenant_clause (`cargo doc -D warnings`) |
| 251d10e52a | docs(gitoxide-core): Held From-impl doc link (later superseded by lock round 3) |
| 34d2115bcf | Merge covenant-escape follow-up (4ef74edb69). Conflict: refs.rs doc, kept both |
| af912007ac | Merge fetch-tracking follow-up (06616a066d): URL/path remotes, cleaner refusals |
| 42c0f733ac | Merge log-rev-path follow-up (71bbb86aa3): revision ranges, diff tree --name-only |
| 7d40d5ae78 | chore: untrack and ignore `.claude/data/epr-meta-advice.json` |
| 49ce98e281 | ci: add diff_name_only, mutation_lock to test-journey's --test list |
| 0b4a9b75e0 | docs: changelog for log ranges, diff tree, fetch/pull follow-ups |
| da5e4a0504 | test(journey): `[OPTIONS]` in brit free pack small-build usage snapshot |
| 1fc60f559d | Merge lock round 3 (63173728bf). Conflict: guard.rs, took lane side |
| 0cdf5dae90 | Merge build-inputs (0feb451a35): gitlink/symlink-aware fingerprints, cc 1.11 |
| 14a9c239a2 | docs(brit-cli): round-3 guard model and build-input keys in changelog |
| 483ad2f992 | test(brit-cli): counting upload-pack wrapper `#[cfg(unix)]` (Windows compile) |
| 7479ce4539 | fix(gitoxide-core): export `reacquire_shared` from the non-Linux stub |

**Lanes not yet pushed.** `sprint3/recovery` is merged locally as Phase 4g (local head `948a939f16`)
and must not be pushed until round 5, its independent review and the non-root leg pass (recovery
entry in `brit-rebase-cost-gitlinks-and-stranding.md`). Its queued follow-up round is not started and
merges only after its own independent review. Merge nothing else before the coordinator says so;
expect more follow-ups on existing lanes.

## 2. Cargo wrapper and build slot

The wrapper is `$S/cargo.sh <log> <cargo args…>`. It:
- cds into the worktree;
- sets `CARGO_TARGET_DIR` (default below, or `TD=<dir>` to override);
- sets `CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS:-4}` and `RUSTFLAGS=""`;
- sets `CFLAGS=-march=x86-64-v2` (or `NOCFLAGS=1` to unset it);
- waits until no `rustc` or `rustdoc` process is running;
- runs cargo with all output to `<log>`, appends `EXIT=<rc>` to the log and prints `EXIT=<rc>`.

Rules:
- **Judge only by `EXIT=`**, never by piped output.
- A hook denies cargo without `CARGO_TARGET_DIR`.
- `CFLAGS` is required for normal builds, or brit-cli's CPU-baseline probe fails on this host. A cold build *without* CFLAGS and without `-C target-cpu` also passes now (build-inputs lane); I verified that on 0cdf5dae.
- Toolchain 1.100.0 (`rust-toolchain.toml`).
- No nextest on this host.
- The environment has `RUSTFLAGS=--cfg getrandom_backend="custom"` and `RUST_BACKTRACE=1`. The wrapper blanks RUSTFLAGS. Unset RUST_BACKTRACE for journey and bash runs (§4).

**The build slot.** The workspace rule is one build at a time. Other sessions often run cargo concurrently and don't serialize.
- An earlier wrapper that waited for "no cargo AND no rustc" starved for over an hour.
- The current one waits only for compilation (`rustc`/`rustdoc`); other sessions' test binaries may keep running.
- Use `CARGO_BUILD_JOBS=2` for doc legs if memory is tight.
- If the RAM guard kills a build (signal 15), retry with `CARGO_BUILD_JOBS=1`.

**Target dirs** (shared pool, capped at 150 GB; LRU eviction is active):
- `/projects/.cargo-target-pool/family/sprint3/projects__elohim__.worktrees__brit-s3-integrate/dev`: default build.
- `…/dev-maxpure`: max-pure build.
- `…/dev-nocflags`: the no-CFLAGS check.

When the pool evicts a dir, the next build is cold. That's fine, just slower: a cold brit-cli build takes about 1–3 min at 4 jobs, and a cold full workspace test build a lot longer. You can seed a new dir with `cp -a …/dev …/dev-maxpure`.

**Keep binaries you need across steps outside the pool**, e.g. `$S/bin-keep/maxpure/brit`. The copy MUST keep the basename `brit` (see §3).

Other scripts in `$S`:
- `journey.sh`, `journey-loop.sh`: bash journeys (§4).
- `rust-style.sh`: rust-style check (§5).
- `p3b.sh`: the full verification, most recent version.
- `p3b-extra.sh`: brit-graph tests and the CPU-baseline probe test.
- `smoke.sh`: live smoke test in scratch repos.

## 3. Full verification

Run `bash $S/p3b.sh`. It writes `$S/p3b-summary.txt`, one line per leg (`<name> EXIT=<rc>`), ending in `DONE`; each leg's log is `$S/p3b-<name>.log`. It runs in the background; watch the summary with a Monitor, not by polling.

Wall-clock time is about 60–90 min with contention on the slot. Approximate durations when cached:
- brit-cli tests: about 3 min.
- cli-journey: 15–25 min. memory* targets take up to a minute each; commit_attribution and worktree_bounded take about 18 s.
- gitoxide-core: about 5 min; its lib tests take about 55 s.
- Each clippy leg: 1–5 min.
- Docs: 5–10 min.
- max-pure test + build: 5–15 min if cold.

The legs, in order. Every build, test, clippy and doc invocation uses `--locked`.

```bash
NOCFLAGS=1 TD=…/dev-nocflags cargo build --locked -p brit-cli --bins      # CPU probe without CFLAGS
cargo build --locked -p brit-cli --bins
cargo test  --locked --no-fail-fast -p brit-cli
cargo test  --locked --no-fail-fast -p cli-journey                        # includes memory_replay (CI's memory step)
cargo test  --locked --no-fail-fast -p brit-epr -p brit-build-ref
cargo test  --locked --no-fail-fast -p gitoxide-core
cargo test  --locked --no-fail-fast -p gitoxide --lib
cargo clippy --locked --workspace --all-targets -- -D warnings -A unknown-lints --no-deps
cargo clippy --locked --workspace --no-default-features --features small      -- -D warnings -A unknown-lints --no-deps
cargo clippy --locked --workspace --no-default-features --features max-pure   -- -D warnings -A unknown-lints --no-deps
cargo clippy --locked --workspace --no-default-features --features lean-async --tests -- -D warnings -A unknown-lints --no-deps
RUSTDOCFLAGS='-D warnings' cargo doc --locked --workspace --no-deps
RUSTDOCFLAGS='-D warnings' cargo doc --locked --features=max,lean,small --workspace --no-deps
cargo fmt --all -- --check
# switch-test-faults legs (CI test-journey):
cargo test --locked -p gitoxide-core --features blocking-client,gix/sha1 --lib power_loss
cargo test --locked -p gitoxide-core --features blocking-client,gix/sha1,switch-test-faults --test replay --test replay_validation --test replay_retained --test replay_qualification --test replay_io --test replay_invocation --test conflict_authoring --test replay_interactive
cargo build --locked -p brit-cli --bin brit --bin brit-rebase-retained --features switch-test-faults
cargo test  --locked -p cli-journey --features switch-test-faults --test rebase --test switch --test pull --test commit_merge --test diff_stat
cargo build --locked -p brit-cli --bin brit --bin brit-rebase-retained      # restore the normal binary
# Shipped build: max-pure (reqwest transport), separate target dir TD=…/dev-maxpure
TD=…/dev-maxpure cargo test  --locked --no-fail-fast -p brit-cli --no-default-features --features max-pure
TD=…/dev-maxpure cargo build --locked -p brit-cli --bin brit --bin brit-rebase-retained --no-default-features --features max-pure
mkdir -p $S/bin-keep/maxpure && cp …/dev-maxpure/debug/brit $S/bin-keep/maxpure/brit && cp …/dev-maxpure/debug/brit-rebase-retained $S/bin-keep/maxpure/brit-rebase-retained
BRIT_BIN=$S/bin-keep/maxpure/brit BRIT_REBASE_RETAINED_BIN=$S/bin-keep/maxpure/brit-rebase-retained \
  cargo test --locked --no-fail-fast -p cli-journey --test rebase --test git_rebase_characterization --test brit --test inherited_git_environment --test shared_tree --test authoring --test commit_attribution --test commit_merge --test diff_stat --test worktree_bounded --test pre_push --test explicit_leases --test commit_cleanup --test commit_amend --test worktree_restore --test switch --test pull --test broader_staging --test revision_ranges --test daily_driver --test native_authoring --test mutation_lock --test diff_name_only --test cli_surface --test context --test mock_remote
```

How the binaries are selected:
- cli-journey selects the binary through `BRIT_BIN` and `BRIT_REBASE_RETAINED_BIN`. Unset, it uses `<target>/debug/brit`.
- brit-cli's own tests use `CARGO_BIN_EXE_brit`, which is why they run built max-pure in `dev-maxpure`.

**Trap: the copied binary must be named `brit`.** Holder descriptions in guard refusals resolve the Brit subcommand path only when the executable's basename is `brit`. A copy named `brit-maxpure` made 7 mutation_lock tests fail, because "names push" assertions saw "`brit-maxpure`" with no subcommand. `p3b.sh` is fixed to copy as `bin-keep/maxpure/brit`.

**Extra legs** (`bash $S/p3b-extra.sh`, summary in `$S/p3b-extra-summary.txt`):
- `cargo test --locked --no-fail-fast -p brit-graph --features repo`
- `scripts/ci/test-cpu-baseline.sh` with `RUSTFLAGS='-C target-cpu=x86-64-v2' CFLAGS='-march=x86-64-v2' CARGO_BUILD_JOBS=1`. It expects `cpu-baseline=passed …`.

Last results (head 7479ce4539, or 0cdf5dae/14a9c239 for legs not re-run after the cfg-only fixes): every leg 0 except cli-journey's known `rakia` red. max-pure gave identical counts to the default build: mutation_lock 20, pull 32, pre_push 14 (2 ignored), brit 16, explicit_leases 10, daily_driver 3, revision_ranges 3, diff_name_only 2. No journey uses an HTTP remote yet, so the reqwest transport itself is untested until push-transport lands.

## 4. Bash journeys (`tests/journey.sh`; CI test-journey's `just ci-journey-tests`)

CI runs `journey-tests-pure`, `journey-tests-small`, `journey-tests-async`, then `journey-tests` (max). Each builds `ein`, `brit` and `jtt` with that feature set into the default target dir; this overwrites `dev/debug/brit`, so rebuild the normal binary afterwards if later steps need it.

Run one variant: `$S/journey.sh <recipe> <log>`. It:
- exports the registry index (`CARGO_REGISTRIES_ELOHIM_INDEX=sparse+https://nexus.ethosengine.com/repository/cargo-internal/`), which the gix-tempfile section needs because it runs cargo from outside the repo;
- unsets `RUST_BACKTRACE`, which otherwise breaks the panic-behaviour snapshot;
- waits for the slot.

Iterate a variant until it passes: `$S/journey-loop.sh <recipe> <tag>`.
- On each snapshot divergence it writes the actual output into the snapshot (through the journey's `ON_ERROR` hook) and records the path in `$S/updated-<tag>.txt`, so the next round gets further.
- **Afterwards, review every changed snapshot with `brit diff`.**
- Commit only changes a lane intended. So far that is one: `tests/snapshots/plumbing/no-repo/pack/receive/"pack receive-no-networking-in-small-failure"` gained `[OPTIONS]` because of the lock lane's global `--wait`; committed in da5e4a0504.

Local-only workarounds. **Never commit these:**
1. **git:// sections.** This host has no git-daemon (`$(git --exec-path)/git-daemon` is missing), and `jtt git-daemon` needs it. Before max, pure and async, add this to `launch-git-daemon()` in `tests/helpers.sh` right after `local i git_daemon_url_file`:
   ```bash
   # LOCAL-ONLY (integration host lacks git-daemon): skip this git:// subshell. Never commit.
   if ! test -x "$(git --exec-path)/git-daemon"; then echo 1>&2 "SKIPPED git:// section: no git-daemon"; exit 0; fi
   ```
   This skips two sections per variant: `remote refs` over git:// and `free pack receive` over git://.
   For the readonly-shem merge the session's automatic permission check refused this edit, so pure,
   async and max ran only up to their first git:// section; a verification that depends on a
   local-only edit is not repeatable across agents (`brit-ci-and-workspace-limits-found-in-sprint-3.md`,
   item (u)).
2. **Host-specific pack checksums.** In pure (and max), seven snapshots differ only in pack/index checksums produced by this host's stock git 2.52 upload-pack (index `c02d9bdd…`, pack `9380796d…`). CI's git produces the committed ones, and the installed base brit (1db7a955) reproduces the host values here, so they are not ours:
   `tests/snapshots/plumbing/no-repo/pack/receive/{file-v-any-no-output,file-v-any-no-output-json,file-v-any-no-output-json-p2,file-v-any-no-output-p2,file-v-any-with-output,file-v-any-with-output-p2,ls-in-output-dir}`.

Restore all of this after the run:
```bash
cd /projects/elohim/.worktrees/brit-s3-integrate && P=tests/snapshots/plumbing/no-repo/pack/receive
brit restore --worktree -- tests/helpers.sh $P/file-v-any-no-output $P/file-v-any-no-output-json $P/file-v-any-no-output-json-p2 $P/file-v-any-no-output-p2 $P/file-v-any-with-output $P/file-v-any-with-output-p2 $P/ls-in-output-dir
```

Last results (14a9c239a2): small 61, pure 130, async 77, max 143 assertions, all EXIT 0.

## 5. Reach gate and push protocol

The coordinator pushes, never you. The gate is `.githooks/pre-push`; it needs one branch update from the checked-out branch, a fully clean tree, and the two checks below.

```bash
cd /projects/elohim/.worktrees/brit-s3-integrate
H=$(brit rev parse HEAD | tail -1)
bash scripts/ci/rust-style.sh --base 1db7a955197b10bc7672c9aa544768a329bb9008 --head $H     # expect PASS
GITHUB_SHA=$H EVENT_BASE=7479ce453981f69832792113fe917bcf72b62dde bash scripts/ci/rust-style-ci.sh  # CI's push form; EVENT_BASE = previous pushed head
epr --repo . ready --target 7479ce453981f69832792113fe917bcf72b62dde                          # expect READY
# Brit fallback: the gate's own probes
git status --porcelain=v1 --untracked-files=all | wc -l    # must be 0
git ls-files -v | grep -c '^[a-zS]'                        # must be 0
```

How the rust-style checks behave:
- The checker is installed in the ignored `.eprfs/tools/rust-style`; `$S/rust-style.sh` installs it if missing. If the install dies on a vanished incremental dir, retry with `CARGO_INCREMENTAL=0`.
- brit's policy enrols only brit-build-ref (14 files, 1 crate, debt 0 → 0).
- A pull request against main is judged on GitHub's PR merge commit, so base `fd6c86b363` is then an ancestor. Emulated with `merge-tree` + `commit-tree`, it passed.
- An all-zeros `before` falls back to `origin/main`, which is not an ancestor, so it reports UNMEASURED (that happened on run 1).

**Protocol:** when you send the head with SendMessage, make NO commit until the coordinator replies "pushed". The gate pushes only the checked-out HEAD.

The file `.claude/data/epr-meta-advice.json` is rewritten by the monorepo's epr-meta hook after edits. It is now untracked and ignored (`.gitignore:36`), so it no longer dirties the probe.

## 6. Merge procedure (one real merge commit per lane, preserving lane history)

```bash
# Brit fallback: three-way merge into worktree with MERGE_HEAD — brit 0.2.6 has no porcelain merge
git merge --no-ff --no-commit sprint3/<lane>
git diff --name-only --diff-filter=U           # conflicts; resolve by reading both sides fully
git diff --cached HEAD -- <files>              # review what the merge brings
# Brit fallback: concluding a merge — brit commit has no parent/merge-head option
git add <resolved files> && git commit -q -F <message-file>
```

Preview first, read-only: `git merge-tree --write-tree --name-only HEAD sprint3/<lane>`. `brit merge commit --in-memory` only says "possibly resolved conflicts" and exits 0 either way.

The merge message should record:
- every conflict and its resolution;
- each cross-lane check made;
- an append-only attribution correction for any lane commit lacking the Co-Authored-By trailer (934deae473 and 7a6beccc1a were recorded that way).

Write messages to files; never put backticks inside double-quoted shell strings.

For non-merge commits use brit: `brit add <paths> && brit commit -F <file>`.

Git fallbacks, each declared with a leading `# Brit fallback: <reason>` comment (the hook enforces it):

| git command | missing brit capability |
|---|---|
| `git merge`, `git commit` on MERGE_HEAD | no merge porcelain, no two-parent commit |
| `git log A..B`, trailers, bodies, `--first-parent` | installed `brit log` has none of these; the new log lane adds revisions but no format/body |
| `git diff A B -- path`, `--stat`, `--name-only`, `--diff-filter` | `brit diff` has no commit-range form |
| `git show <rev>:<path>` | none (`brit cat <rev>:<path>` works for blobs) |
| `git rm --cached` | brit has no untrack command |
| `git ls-files -s/-v`, `git check-ignore`, `git status --porcelain` | none; the reach gate's own probes |
| `git merge-tree`, `git commit-tree` | dry merges and PR-merge emulation |

`brit restore --source` accepts only HEAD; use `brit cat <rev>:<path> > file` for other revisions.

## 7. Mutation-guard scope table (`src/plumbing/main.rs` `mutation_guard_scope`)

The full model is in `docs/mutation-guards.md`.

| command | scope at dispatch |
|---|---|
| `add` | Worktree (worktree guard + repository shared) |
| `commit` (no subcommand) | None: takes its own worktree guard; repository shared only at start and to publish; nothing during post-commit |
| `restore`, `merge tree --update-head`, `clean --execute` | RepositoryAndWorktree |
| `push` | None: own worktree guard for the whole run; repository shared for resolution and send-pack, never across pre-push |
| `switch`, `pull`, `rebase` | None at dispatch; take worktree + exclusive repository themselves (`shared_guard`) |
| `log`, `diff` (incl. `diff tree`), `branch list`, bare `tag`/`tag list`, `remote` (refs, ref-map), `status`, and the rest of the `requires_mutation_guard` false list | None |
| `branch create/delete/-D/move`, `tag NAME [COMMIT]`/`tag -d`, `fetch`, `config` writes, `worktree add/remove`, everything else | Repository (exclusive), the `requires_mutation_guard` default |
| `branch -m` additionally | `switch::other_worktree_guard` (non-waiting) on every worktree whose HEAD is on the old branch, this one included, before the ref transaction (a6b78d9c89) |
| `worktree remove` additionally | the target worktree's guard, non-waiting |

**Classifying a new command:**
- Read-only: add it to the `requires_mutation_guard` false list, so it is None.
- Writes shared state (refs, config, registrations, journals): leave the default, so it is Repository.
- Writes only its own worktree's index, HEAD or files plus objects and compare-and-swap refs: Worktree.
- Rewrites its own worktree and shared state: RepositoryAndWorktree.
- Rewrites *another* worktree: take `other_worktree_guard` for it while holding the repository guard; never wait for a worktree guard while holding the repository guard.
- Runs hooks: decide what it holds across them; see docs/mutation-guards.md.

Then add a journey to `tests/cli-journey/tests/mutation_lock.rs` (file-level linux cfg).

## 8. Windows/macOS audit (CI builds Windows, macOS, i386 and arm; this host can't cross-compile)

Only `x86_64-unknown-linux-gnu` is installed. Do not install targets or linkers on this shared device, so the audit is by reading.

For every test or support file a lane adds or changes, grep the added lines and check each hit's cfg context:
```bash
# Brit fallback: added lines over a commit range
git diff -U0 <old-head> HEAD -- '*.rs' | awk '/^\+\+\+ b\//{f=substr($0,7)} /^@@/{split($3,a,","); ln=substr(a[1],2)} /^\+[^+]/{print f":"ln": "substr($0,2); ln++}' \
 | grep -E 'os::unix|PermissionsExt|from_mode|OsStrExt|OsStringExt|MetadataExt|libc::|rustix|nix::|/proc|#!/bin/sh|"sh"|flock|"/tmp|kill\(|SIG|chown|push\(":"\)|set_permissions|symlink'
```

Also look for:
- hooks or scripts written to disk;
- `:`-separated PATH manipulation;
- executable-bit assumptions;
- hard-coded `/` paths.

Local conventions:
- per-test `#[cfg(unix)]`, as in commit_amend, authoring and log_rev_path;
- file-level `#![cfg(target_os = "linux")]`, as in mutation_lock, pull and rebase;
- `#[cfg(target_os = "linux")]` for flock tests.

`file://{path.display()}` URLs passed on Windows in run 1.

**Non-test code.** Every `switch::` symbol used outside `switch/linux` must be exported by BOTH `pub use` lists in `gitoxide-core/src/repository/switch/mod.rs`: the linux list and the `unavailable` list. Compare them each merge:
```bash
grep -rn 'switch::[a-zA-Z_]*' --include=*.rs gitoxide-core/src src brit-cli/src | grep -v 'repository/switch/' | grep -oE 'switch::[a-zA-Z_]+' | sort -u
```
`reacquire_shared` was missing (7479ce4539). Note that rustfmt also formats cfg'd-out code.

Per-file conclusions so far:

| file | conclusion |
|---|---|
| brit-cli/tests/remote_tracking.rs | upload-pack wrapper + its test gated `#[cfg(unix)]` (483ad2f992) |
| brit-cli/tests/log_rev_path.rs | chmod already split `cfg(unix)` / `cfg(not(unix))`; compiles |
| brit-cli/tests/branch_delete.rs, tag_mutation.rs | flock tests `cfg(target_os = "linux")`; rest is git + reads |
| brit-cli/tests/cli_smoke.rs | symlinks and gitlinks via hash-object + update-index; portable |
| brit-cli/tests/feedback.rs, repository_slots.rs, tests/cli-journey/tests/cli_surface.rs | no platform APIs |
| tests/cli-journey/tests/mutation_lock.rs, pull.rs, rebase.rs | file-level linux cfg |
| tests/cli-journey/tests/revision_ranges.rs, diff_name_only.rs | brit_bin + git only |
| tests/cli-journey/tests/memory_build.rs | fixture paths moved; its existing linux-only test keeps its cfg |
| tests/cli-journey/src/support/**, explicit_leases.rs | unchanged by the lanes |
| brit-epr/tests/ref_management.rs, brit-build-ref/tests/verified_reads.rs | no platform APIs |
| brit-graph/tests/repo_fingerprint.rs | `cfg(feature = "repo")`; links via index; `GIT_LITERAL_PATHSPECS`; portable |
| non-test: guard code | under `switch/linux` |
| non-test: CPU probe | C check only when `CARGO_CFG_TARGET_OS == linux` |
| non-test: guard_label.rs, mutation.rs, branch.rs | no Unix APIs; mutation.rs tests are `cfg(all(test, target_os = "linux"))` |

Still to audit when they arrive: the trust, push-transport and diff-show lanes' new files.

## 9. Known reds that are not ours, and what CI requires

**Known reds:**
- **cli-journey `rakia` `fingerprint_emits_64_char_blake3_hex`** failed on this host with "object database slots exhausted (current: 32, needed: 33)" against the live `/projects/elohim` monorepo, identically with the installed base brit 0.2.6 (CI skips it: no monorepo root). Cleared by the trust lane: the full cli-journey leg was 496 passed / 0 failed at `8e5072ffd3` and 499 / 0 at `080f305797`. No longer a known red.
- **cargo-deny-advisories** reports `RUSTSEC-2026-0543` (chrono unmaintained) and the libz-sys 1.1.31 heap-overflow advisory (the coordinator's RUSTSEC-2026-0544). It is red on main too (runs 44200786326, 44144465688) and is listed in `check-blocking`'s `EXPECTED_NONBLOCKING_JOBS`, so `Tests pass` does not need it.

**`Tests pass` needs:** brit-rust-style, brit-cpu-compatibility, brit-release-package-contract, brit-readiness-contract, brit-feature-census, msrv, msrv-badge, pure-rust-build, auditable, test-doc, test-journey, test-fast, test-32bit-windows-size-doc, lint, cargo-deny, check-no-persist-credentials, check-blocking.

Coverage notes:
- test-fast runs on ubuntu x86_64, ubuntu arm, macOS and Windows.
- Windows test-fixtures jobs run too.
- Not reproducible here: cargo-deny, machete and diet (not installed); MSRV toolchain builds; Windows, macOS and 32-bit compiles; `etc/parity/enumerate.sh --check` (needs a vendor/git checkout); git:// journey sections.
- Cheap and runnable here:
  - `env PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s etc/parity -p 'test_*.py'`
  - `bash etc/parity/shortcomings.sh --check`
  - `cargo run -p internal-tools -- check-mode`

**CI history:**
- run 1 (251d10e5): three reds. test-journey (the small snapshot), brit-rust-style (all-zeros base), advisories.
- run 2 (da5e4a05): only the four Windows jobs, at the remote_tracking compile.
- run 3 (7479ce45): Linux and macOS green; four Windows tests red (build-inputs lane).
- Later runs: the head/push table at the top of this entry.

## 10. Open watch items (raised, not acted on)

1. **fetch `names_a_repository`** (fetch lane 06616a06, gitoxide-core/src/repository/fetch.rs): it resolves a bare relative name against the *process* working directory, not the repository's. With `brit -r <elsewhere> fetch foo`, a repository at `./foo` in the caller's cwd would be taken as the remote.
2. **`diff tree --name-only`** (log lane 3954fcdd24, gitoxide-core/src/repository/diff.rs `tree_name_only`): it flattens both trees into in-memory indexes (`index_from_tree`). That is correct, but memory grows with tree size; relevant for the bounded-memory daily-driver work.
3. **Holder naming:** refusals name the holding Brit subcommand only when the executable's basename is `brit`. An installed `brit-0.2.x`, the deprecated `rakia` alias, or any renamed copy shows only the program name (`redacted_command`/`set_command_resolver` in switch/mod.rs and src/plumbing/guard_label.rs).
4. **CI test lists:**
   - test-journey's explicit `--test` list in ci.yml omits older cli-journey targets that run only via test-fast: brit, commit_signing, filter_scope, inherited_git_environment, mock_remote, native_source, normalize, rakia, runner, test_repo. mutation_lock and diff_name_only were added (49ce98e281).
   - The release workflow's (brit-release.yml) cli-journey list also omits mutation_lock and diff_name_only.
5. **Hidden `branch force-delete`:** `-D` needs a subcommand name, so `brit branch force-delete X` is typeable; git has no such spelling. clap can't avoid it; left as is.
6. **Fetch double handshake:** a short name matching several advertised refs makes fetch connect twice.
7. **`branch::rename` atomicity:** the reflog carry-over and config section move happen after the ref transaction, and errors there say so. A crash in between leaves refs renamed with the old config and reflog.
8. **Cosmetic:** a main worktree's path renders as `…/repo/.` in rename and delete refusals (`main_repo().workdir()`).
9. **CLI census:** `--cli-surface-json` now records `shortFlag`/`longFlag`/`hidden` (schemaVersion still 1). Any consumer that pins the schema should know the keys are additive.
10. **Changelog placement:** brit has no running executable changelog besides the new `brit-cli/CHANGELOG.md`; `docs/releases/brit-0.2.0.md` holds the only release narrative. The release author should fold "Unreleased" into `docs/releases/brit-<next>.md`. The root CHANGELOG.md is upstream's generated file; keep it byte-identical.
11. **Untracked advisory file:** `.claude/data/epr-meta-advice.json` is untracked (7d40d5ae78). Other brit worktrees still have it as a tracked, modified file until they merge this.