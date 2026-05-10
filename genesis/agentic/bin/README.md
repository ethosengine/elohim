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
  | branch merged to dev, clean status | `git worktree remove --force` |
  | branch merged to dev, dirty status | log to `orphan-worktrees.tsv` |
  | branch active on origin | left alone |
  | branch not merged + not on origin | left alone (conservative; can't tell `deleted-upstream` from `never-pushed`) |
  | broken `.git` or missing dir | left alone |

  Then it computes the slot path for the current worktree's family and
  emits an `additionalContext` block telling the agent which
  `CARGO_TARGET_DIR` to set for each native cargo workspace.

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
cargo-pool status            # families table — disk, slot count, last touched
cargo-pool steward --dry-run # preview stewardship without applying
cargo-pool steward           # apply stewardship now
cargo-pool key               # print slot path for current PWD
cargo-pool prune family iroh # nuke a family slot tree (interactive)
cargo-pool prune --older-than 7d --yes  # GC slot subdirs untouched > 7 days
cargo-pool log -n 20         # tail steward.log
cargo-pool orphans           # print orphan-worktrees.tsv
```

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
