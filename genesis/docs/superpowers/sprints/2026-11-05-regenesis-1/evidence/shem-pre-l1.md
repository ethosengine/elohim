---
id: regenesis-1-shem-pre-l1-evidence
---

# shem pre-L1 evidence — regenesis-1

Device: shem (`did:key:z6Mkm5de…bowj`). Session `617b9272-adc3-4226-9bd4-cec1ac027767`
(claude-opus-5-5), moored `--task regenesis-1-shem`. Brief: 184beb01 note, 17:36Z 2026-11-05.

## Worktrees

All three worktrees are at `a31d94403` (`origin/regenesis/integrate`, rebased on `b663aa65f`) with
`.family=sprint`. Each has the hub symlink `genesis/local-dev/regenesis-1 -> ../docs/superpowers/sprints/2026-11-05-regenesis-1`.

| worktree | branch | submodules |
|---|---|---|
| `.worktrees/rg1-dna` | `regenesis/dna` | brit, rakia, sophia (the gate preflight needs them, although the T1 brief says none are needed) |
| `.worktrees/rg1-storage` | `regenesis/storage` | all |
| `.worktrees/rg1-integrate` | `regenesis/integrate` | none |

`/.family` and `/genesis/local-dev/regenesis-1` are listed in the shared `info/exclude`. Until they
were, every rg1 gate counted as dirty and recorded no evidence.

## Gate warm-ups

| gate | worktree | start (UTC) | wall-clock | EXIT | evidence |
|---|---|---|---|---|---|
| `just gate sweettest-check` (cold) | rg1-dna | 17:39:57 | 1779 s | 1 | none. The compile was clean; the red was the preflight check for missing submodules |
| `just gate sweettest-check` (warm) | rg1-dna | 18:10:02 | 52 s | 0 | not recorded (worktree dirty: `.family`, hub symlink) |
| `just gate elohim-storage` (`CARGO_BUILD_JOBS=1`, mesh down) | rg1-storage | 18:19:34 | 6014 s (100 min) | 0 | recorded `bafyreidexrg6b4uopprwxif2cewp3scgy56mdouvcjgzpy57uj26bl5vge` |

All elohim-storage test binaries reported `test result: ok` with 0 failures. The ignored tests
include the standing-red `sync_scale_honesty` spine node. Both gates are over the
`gate-cycle-full-ceiling@1` (1200 s) ceiling: storage finding `984ccc2c18d8`, cold sweettest
finding `af499151f07e`.

Gate-live runs: `20261105T181934Z-a31d944038d4-c4d681` (storage) and
`20261105T181003Z-a31d944038d4-aa3ee6` (warm sweettest). The full logs are untracked on shem:
`genesis/local-dev/regenesis-1/gate-T2-elohim-storage.log`, `gate-T1-sweettest.log` and
`gate-T1-sweettest-cold.log`.

## Launcher-name-invalid blocker (19:02Z), root-caused on shem

`genesis.agentic.shift_lease_test.NativeHostSession.test_host_uses_native_envelope_without_taking_callers_session`:
- With `EPR_BIN` unset, the test fails: `REFUSED launcher-name-invalid: "bin-launch" resolves to 0 tools` (exit 127).
- With `EPR_BIN` set to a store binary (`epr bin resolve --tool epr --current`), it passes: `OK`.

Cause: `shift_lease_test.py:682/685` calls `Path(shutil.which('epr')).resolve()`. That follows the
bin-store symlink to `bin-launch`, so `session_envelope_test.py:98` runs the launcher under its own
name. The range `426b0bcb7..b663aa65f` did not touch name resolution. 184beb01 is carrying the fix on
`regenesis/kernel`.

## Brit gaps and Git fallbacks

| operation | observation |
|---|---|
| `brit fetch origin` | fails with "None of the refspec(s) origin matched"; `brit fetch -r origin` works |
| `brit submodule update --init` | missing; `brit submodule` has only `list`. Git fallback |
| `brit -C <dir>` | not accepted; use a subshell `cd` |
| `brit diff <path>`, `brit restore <path>` | need `--` before paths; `restore` needed `--worktree --` |
| `brit log <path>` | "File-based lookup isn't yet implemented" |
| creating a commit | `brit commit` covers only verify, sign and describe. Git fallback to commit this file |

## Not yet run

- T1 authoring is held until 184beb01 says L1 is on origin/dev.
- The H1–H5 household recast waits until L2b lands (ruling at 19:16Z) and needs Matthew's yes.
  The household mesh is stopped with its identities preserved.
