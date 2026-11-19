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
  Continue brit sprint 3 in the decided order: an independent integrator merges sprint3/trust,
  sprint3/push-transport, sprint3/diff-show and sprint3/refs-shem into sprint3/integrate as each
  clears review; then the push-policy lane, the rebase recovery lane (steps 1-2), the epr-cli
  lane, worktree-side diff with gitlinks, and merge porcelain.
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
`run-ci/sprint3-integrate` three times, through `brit push` and brit's reach gate (`epr ready` +
`scripts/ci/rust-style.sh`), each push taking about 6 s:

| Pushed head | CI run |
|---|---|
| `251d10e52a` | 44469917351 |
| `da5e4a0504` | 44477093766 |
| `7479ce453981f69832792113fe917bcf72b62dde` | third run in progress when this was written |

**Lanes** (each in its own worktree under `/projects/elohim/.worktrees/`):

| Lane branch | Tip | State | Worktree |
|---|---|---|---|
| `sprint3/covenant-escape` | `4ef74edb69` | merged into `7479ce45` | `brit-s3-covenant-escape` |
| `sprint3/log-rev-path` | `71bbb86aa3` | merged | `brit-s3-log-rev-path` |
| `sprint3/branch-tag` | `99caa43f` | merged | `brit-s3-branch-tag` |
| `sprint3/fetch-tracking` | `06616a06` | merged | `brit-s3-fetch-tracking` |
| `sprint3/lock` | `63173728` | merged | `brit-s3-lock` |
| `sprint3/build-inputs` | `0feb451a` | merged | `brit-s3-build-inputs` |
| `sprint3/trust` | `d96ff65b2d` | still in review rounds, not merged | `brit-s3-trust` |
| `sprint3/push-transport` | `0ca127c2` | still in review rounds (second review), not merged | `brit-s3-push-transport` |
| `sprint3/diff-show` | `3d10b379cf` | in review, not merged | `brit-s3-diff-show` |
| `sprint3/refs-shem` | `4a39e9dffa` | six patches from shem applied locally; held until shem's fix round | `brit-s3-refs-shem` |

**Monorepo side:** branch `feat/ark-declared-trust` (worktree `brit-s3-ark-trust`), first written as
three commits on origin/dev `01e4de7f1`, is now four commits (tip `d14e7357f`), reviewed, awaiting
rebase onto the landed dev.

**This record** is on monorepo branch `docs/brit-backlog-sprint3` (worktree `brit-s3-backlog`).

## What the merged lanes deliver

- **covenant-escape:** see `brit-covenant-session-segment-aliasing.md` (status 2026-11-19), including
  its residuals.
- **lock:** see `brit-mutation-lock-blocks-isolated-lanes.md` (status 2026-11-19).
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

1. An independent integrator merges `sprint3/trust`, `sprint3/push-transport`, `sprint3/diff-show` and
   `sprint3/refs-shem` as each clears review.
2. A push-policy lane (`push.rs`): one handshake when no hook ran; the notes/tag fast-forward rule;
   refuse an ungated push when hooks are configured but missing or not executable; render the delivery
   note in the CLI. See `brit-push-fails-after-green-hook-root-cause.md`.
3. The recovery lane: steps 1–2 of `brit-rebase-cost-gitlinks-and-stranding.md`.
4. The epr-cli lane: a `device_store(…)` constructor that always attaches `DeviceRunner`, and a
   source-scan test, stacked on `feat/ark-declared-trust` after shem's L2b lands; `bin.rs` waits for
   the landed sha. See `brit-declared-trust-and-reduced-trust-pack-cap.md` (epr-cli spawn audit).
5. Worktree-side diff with gitlinks.
6. Merge porcelain.

## What needs the steward

- Landing on brit `main` and the pin move. 16 monorepo steps re-key once (build-inputs above); one
  supervised real landing is needed to retire the push exception in `CLAUDE.md`. Its notes step is the
  device's own notes publish, fast-forward to `refs/notes/brit/devices/<did>/build/*`, which brit must
  be able to perform (or the publish script keeps using git, stated as a fallback); see
  `brit-push-fails-after-green-hook-root-cause.md`. The landing procedure changed during this sprint:
  re-read it from `CLAUDE.md` on origin/dev at landing time.
- Landing `feat/ark-declared-trust` on dev behind shem's L1/L2a/L2b, with the one-line notice.
- `git repack -a -d` in the monorepo.
- The upstream-owned gix changes in `sprint3/trust`.
- Trust for linked worktrees.
- The local-only commit `5d53c7ac8`.
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

- Hand-over ref: `origin self/nt-review-receipts`.
- shem's lanes in flight: the refs lane's fix round; `merge-base --is-ancestor` and
  `rev-list --count`, which shem also takes; and L1/L2a/L2b, which `feat/ark-declared-trust` lands
  behind.

## Integration runbook

The text below is the integration worktree's handoff note, copied in full from the session
scratchpad. Its `/tmp/claude-0/…/scratchpad/integrate/` paths (the `$S` scripts, summaries, logs and
`bin-keep` binaries) are session scratch and will not survive a workspace restart.

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
- Head: `7479ce453981f69832792113fe917bcf72b62dde`.
  - The coordinator pushed it to `origin run-ci/sprint3-integrate`, and CI run 3 is running.
  - The previous pushed heads were `251d10e52a` (run 1) and `da5e4a0504` (run 2).
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

**Lanes not yet merged.** Merge none of them before the coordinator says so:
- sprint3/trust and sprint3/diff-show (tip 3d10b379cf, on top of log 4ec68b1669) are in review.
  - diff-show makes `merge_bases`, `Literal` and its fields and `new`, and `worktree_path` in `gitoxide-core/src/repository/log.rs` `pub(crate)`. It also re-splits one signature with rustfmt.
  - That touches the same lines as log 71bbb86aa3, which is already merged, so expect a conflict.
- sprint3/push-transport is in its second review.
- sprint3/refs-shem (the sibling device's six-patch lane) is waiting on its fix round.
- Expect more follow-ups on existing lanes.

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
cargo test  --locked --no-fail-fast -p cli-journey                        # rakia 7/8 is a known red, see §9
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
- **cli-journey `rakia` `fingerprint_emits_64_char_blake3_hex`** fails on this host with "object database slots exhausted (current: 32, needed: 33)" against the live `/projects/elohim` monorepo. It fails identically with the installed base brit 0.2.6, and CI skips it (no monorepo root). The trust lane's slot sizing should fix it.
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
- run 3 (7479ce45): in progress.

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

Monorepo-level item: the `brit-gix-slots` worktree / trust lane is expected to clear the rakia slots red.
