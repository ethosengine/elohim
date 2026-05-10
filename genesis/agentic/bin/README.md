# cargo target pool — operator notes

This directory holds the cargo target pool's hooks and operator CLI. Per-
worktree `target/` dirs each cost ~18GB of disk (storage 5.8G + doorway
7.7G + sweettest 4.7G). With 4 parallel worktrees on the same family
that's ~72GB; six abandoned worktrees on 2026-05-10 hit 100% disk.

The pool keeps a single shared `target/` per (family, workspace, profile),
so parallel agents on the iroh family share one `elohim-storage/target`,
one `doorway-service/target`, etc. Worktree stewardship cleans up
worktrees whose branches have already merged to dev.

**Design:** `genesis/docs/plans/cargo-target-pool-design.md`.

## How it works

- **Pre-flight (SessionStart hook).** `pool-preflight.sh` runs at every
  session start. It walks `.claude/worktrees/*` and classifies each
  worktree by branch state:

  | Classification | Action |
  |---|---|
  | **process CWD inside** (any classification) | `active-subagent` → left alone (race-safe override) |
  | branch merged to dev, clean status | `git worktree remove --force` |
  | branch merged to dev, dirty status | log to `orphan-worktrees.tsv` (deduped by wt+branch+class) |
  | branch active on origin | left alone |
  | branch not merged + not on origin | left alone (conservative; can't tell `deleted-upstream` from `never-pushed`) |
  | broken `.git` or missing dir | left alone |

  Then it computes the slot path for the current worktree's family and
  emits an `additionalContext` block telling the agent which
  `CARGO_TARGET_DIR` to set for each native cargo workspace.

  The preflight block also surfaces:

  | Trigger | Banner |
  |---|---|
  | volume ≥90% used | `!! CRITICAL DISK PRESSURE` with one-command cleanups |
  | volume ≥80% used | `!  Disk pressure` with prune suggestion |
  | legacy `target/` outside pool > 256MB recoverable | `!  Legacy target/` with `legacy-targets --clean --yes` |
  | stale incremental hash dirs >1GB | `!  Stale incremental` with `prune --stale-incrementals --yes` |
  | newly-created worktree since last preflight + free disk < estimate × 1.5 | `!! BUDGET SHORT` (or `!  Budget tight`) with pre-dispatch cleanup |
  | stale `node_modules` + old `.angular` caches > 2GB combined | `!  JS-cache pressure` with per-class clean commands |

- **Post-flight (Stop hook).** `pool-postflight.sh` re-runs stewardship
  to catch worktrees that finished during the session (typical case:
  agent merged its branch right before stopping). Idempotent; always
  safe.

- **Cargo concurrency.** Two agents on the same family share the same
  `target/`. Cargo's intra-target advisory lock (`.cargo-lock`)
  serializes their `rustc` invocations naturally — and that
  serialization is *desired*, since A's incremental fingerprints warm
  B's build. There is no per-cargo-invocation holder file in this
  design; the pool is hooks-only.

- **DNA / WASM workspaces are exempt.** `hc dna pack` canonicalizes
  `./target` and breaks if the artifact lives elsewhere (this regressed
  in shift `2026-05-09T16-30-orchestrator-clean-cascade`). Don't redirect
  WASM target dirs.

## Family detection

The hook derives the family for a worktree as:

1. `CARGO_TARGET_POOL_FAMILY` env var (override).
2. `<worktree-root>/.family` file (per-worktree pin).
3. Branch name prefix split on `-` or `/`, with `feat/`, `feature/`,
   `fix/`, `chore/`, `worktree-` prefixes stripped first.
   - `iroh-pkarr` → `iroh`
   - `feat/iroh-phase12-manifest` → `iroh`
   - `worktree-iroh-parallel-stack` → `iroh`
   - `epr-phase-2b-batch-c` → `epr`
   - `dev` → `dev`
4. Sanitized worktree directory name (last-resort fallback).

## Operator commands

The `cargo-pool` script is on PATH (via devfile PATH addition).

```sh
cargo-pool status            # families table — disk + pressure indicators
cargo-pool steward --dry-run # preview stewardship; respects active-subagent override
cargo-pool steward           # apply stewardship now
cargo-pool key               # print slot path for current PWD; shows HWM if observed
cargo-pool estimate [wt]     # GB cost of next cargo in this worktree (uses HWM × 1.2)
cargo-pool watermark         # print observed peak size per slot
cargo-pool watermark update  # re-sample now (postflight does this automatically)
cargo-pool legacy-targets    # find target/ dirs outside pool; classifies native/wasm/typesrc/unknown
cargo-pool legacy-targets --clean --yes  # remove the `native` class
cargo-pool node-modules      # find node_modules; classifies stale/fresh/active/unknown
cargo-pool node-modules --clean --yes        # remove stale (default); --all for everything non-active
cargo-pool angular-cache     # find .angular caches; by age
cargo-pool angular-cache --clean --older-than-days 7 --yes
cargo-pool prune family iroh # nuke a family slot tree (interactive)
cargo-pool prune --older-than 7d --yes        # GC slot subdirs untouched > 7 days
cargo-pool prune --stale-incrementals --yes   # GC per-crate incremental hash dirs >3d old
cargo-pool prune --stale-incrementals --older-than-days N --yes
cargo-pool log -n 20         # tail steward.log
cargo-pool orphans           # deduped merged-dirty history + live dirty-no-process scan
```

### What each pressure command actually does

- **`prune --stale-incrementals`** walks `family/*/*/*/{debug,release}/incremental/*-<hash>/`
  and removes per-crate fingerprint dirs whose mtime is older than N days
  (default 3). These are the cargo-incremental remnants of source diffs from
  branches that have been merged and removed — Cargo never GCs them.
  Naive (mtime-only) — branch-aware variant deferred until naive proves
  insufficient.

- **`legacy-targets`** finds `target/` directories outside the pool root and
  classifies each:
  - `native`: under a known cargo workspace (`elohim/elohim-storage`,
    `doorway/doorway-service`, `steward/node`, `elohim/holochain/tests/sweettest`,
    `crates`). Safe to remove — pool will rebuild on next cargo invocation.
  - `wasm`: under `elohim/holochain/dna/*` or sibling of `dna.yaml`/`happ.yaml`
    or has `crate-type = ["cdylib"]`. **Never removed by `--clean`** — `hc dna
    pack` canonicalizes `./target` and would break.
  - `typesrc`: under `elohim/sdk/domains/*/types/*`. Small TS-codegen artifacts;
    not on the disk-pressure path; left alone.
  - `unknown`: anything else (e.g., workspace-root invocations bypassing
    `CARGO_TARGET_DIR`). Reported, not auto-cleaned.

- **`estimate`** sums per-workspace cost for every `NATIVE_WORKSPACES` entry
  present in the worktree. Prefers observed peaks (`<slot>/.peak-size`) ×
  1.2 safety margin; falls back to a sibling family's HWM for cold slots;
  falls back to hardcoded 10G cold / 3G warm when nothing is observed.
  Per-workspace breakdown labels each number with its source. Verdict is
  `ok` / `tight` / `short` based on free disk vs estimate × 1.5.

- **`watermark`** prints (or updates) the per-slot HWM file
  `<slot>/.peak-size`. Postflight (Stop hook) automatically samples after
  every session, so HWMs rise monotonically as builds get bigger. After ~5
  sessions of real activity the estimator is grounded in this workspace's
  actual cargo footprint rather than guessed constants.

- **`node-modules`** finds `node_modules/` trees outside the pool root and
  outside the sophia submodule. Classifications:
  - `stale`: pnpm-lock.yaml (or package-lock.json/yarn.lock) is newer than
    `.modules.yaml`. The install has drifted from the lockfile — reinstall
    needed anyway, safe to remove.
  - `fresh`: lockfile matches the install marker.
  - `active`: a process has CWD inside the *project directory* (parent of
    the node_modules, not the whole worktree — narrower than the cargo
    steward check, because a shell at the repo root would otherwise mark
    every sub-project node_modules as active).
  - `unknown`: no lockfile found nearby. Left alone.

  `--clean --stale` (default) removes only stale class. `--clean --all`
  nukes everything non-active. Either way, `pnpm install` is required in
  each cleaned project before the next build.

- **`angular-cache`** finds `.angular/` cache directories with size + age in
  days. `--clean --older-than-days N` removes those exceeding N days
  (default 7). Always safe — Angular CLI rebuilds on next start/build.

- **`orphans`** combines the dedup view of `orphan-worktrees.tsv` (merged-dirty
  history, one row per `wt+branch+class` tuple) with a live `git status`
  scan of every worktree, filtering out any whose CWD is held by a live
  subagent process. The live scan surfaces crash-recovery candidates without
  storing them.

## Env vars

| Variable | Default | Purpose |
|---|---|---|
| `CARGO_TARGET_POOL_ROOT` | `/projects/.cargo-target-pool` | Where slots live |
| `CARGO_TARGET_POOL_FAMILY` | (heuristic) | Override family detection |
| `POOL_PARENT_REPO` | `$CLAUDE_PROJECT_DIR` or `/projects/elohim` | The main repo |
| `POOL_WORKTREES_DIR` | `$POOL_PARENT_REPO/.claude/worktrees` | Where worktrees live |
| `CARGO_TARGET_POOL_PREFLIGHT_DRY` | unset | If `1`, preflight runs stewardship in dry-run mode |
| `CARGO_TARGET_POOL_POSTFLIGHT_DRY` | unset | Same for postflight |
| `CARGO_TARGET_POOL_POSTFLIGHT_GC_DAYS` | unset | If set, postflight prunes slot dirs older than N days |

## Devfile re-roll required

Adding `CARGO_TARGET_POOL_ROOT` and the PATH change to `devfile.yaml`
requires a workspace **restart-from-local-devfile** (commit + push
first; plain restart won't pick up env changes). Until then the hooks
will use the default pool root and the wrappers won't be on PATH —
operator can run them via absolute path:

```sh
bash genesis/agentic/bin/cargo-pool status
```

## Safety notes

- Preflight stewardship will REMOVE worktrees whose branches have
  merged to dev. This is destructive but mostly recoverable
  (`git worktree add` reconstitutes any worktree from history).
- "Cleaned-dirty" worktrees (merged + uncommitted) are NOT removed;
  they get logged to orphan-worktrees.tsv for operator review.
- Branches not merged + not on origin are LEFT ALONE. Conservative.
- To inhibit stewardship for one session, set
  `CARGO_TARGET_POOL_PREFLIGHT_DRY=1` in your shell before launching
  Claude Code.

## Files

- `pool-lib.sh` — shared functions (worktree/family detection, classify,
  steward).
- `pool-preflight.sh` — SessionStart hook.
- `pool-postflight.sh` — Stop hook.
- `cargo-pool` — operator CLI.

## Related

- Sccache substrate: `project_garage_sccache_substrate_2026_05_09.md` in
  agent memory. Sccache caches *compile outputs* across worktrees;
  the target pool shrinks the *materialized* `target/` footprint.
  Orthogonal mechanisms; both wanted.
- Worktree creation: `superpowers:using-git-worktrees` skill.
