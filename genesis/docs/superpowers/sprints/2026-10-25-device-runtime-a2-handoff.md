---
title: "Handoff — the device runtime after slices A and A2"
id: device-runtime-a2-handoff-20261025
date: 2026-10-25
status: handoff
author: "claude-opus-5-5 (Che workspace, ethosengine), at the operator's direction"
habits: [dataplane-convergence, workspace-device-arrival]
cites:
  - "household-follows-latest-design | The household follows latest | sha256:7a60d62e64a44ee5 | path: genesis/docs/superpowers/specs/2026-10-23-household-follows-latest-design.md"
  - genesis/data/timeline/backlog/prepush-head-landing-never-ingests.md
---

# Handoff — the device runtime after slices A and A2

**What this work is.** The device runtime makes each workspace one of its human's devices, and
makes every device follow the latest build: a green gate on `dev` reaches every agent session on
every device without a hand install, while a session already running keeps the build it started
on. `epr` is the device's native command-line runtime; its builds live in a content-addressed bin
store, and a launcher on `PATH` execs the build a session is pinned to. Slices:
A (native session identity, bin store, arrival), A2 (the session envelope: one record keys every
lease, claim, permission and recall a session holds), **M** (the device's declared target: one
manifest says which channels and tools the device follows and runs, and every hand-edited copy is
rendered from it), then B (heads and bytes travel between the steward's devices), C (supervision),
D and E. The plan `/projects/.claude-config/plans/shiny-booping-map.md` is the authority; the
design is the spec cited above, §10 (A) and §11 (A2).

**Start here.** The operator pastes this into a NEW Claude Code session:

> Carry on with the device-runtime plan at /projects/.claude-config/plans/shiny-booping-map.md;
> read genesis/docs/superpowers/sprints/2026-10-25-device-runtime-a2-handoff.md first. Fix the
> pre-push head landing, then begin slice M.

It must be a new session. The session that wrote this stays pinned to an `epr` built before
envelopes, so its capability tier reads `unknown` (destructive git such as rebase is refused) and
its recall degrades. A fresh session pins the current head and opens an envelope.

**First, verify the device** (from a fresh session, plain `just status device` is enough):
`epr bin status --tool epr --json` must report `head` =
`bafkreiftoz245j2pbgy2gqngcczrk7y764ccm5r4p4hpjbjwrwlgurh5ju`, and `just status device` must exit
0. If the head differs, read `/projects/.claude-config/tools/bin-store/epr/current.json` (it names
the commit) before landing anything.

## What landed on `dev`

| Commit | What |
|---|---|
| `8bc6f0d0e` | slice A: native session identity, bin store and launcher (CIDv1), arrival, gate receipts |
| `cbdb065b4` | `workspace-device-arrival` green; the habit flip check judges rulings on the author clock |
| `fe893b990` | the main checkout's one unlanded commit (self-heal, `reanchorDeadHeldNotAuthor`), with its household serving receipt (7/7 stations) |
| `b542c1a73`, `33ab3925c` | slice A2: spec §11, then the session envelope |
| `9bcb85973` | backlog: pre-push's head landing never finds a candidate |

Each commit message carries its evidence and review-loop cost.

## Live state that has no other home

- **This device:** head `bafkreiftoz245j2pbgy2gqngcczrk7y764ccm5r4p4hpjbjwrwlgurh5ju` = `33ab3925c`
  (previous `bafkreigoah43…` = `8bc6f0d0e`), landed by hand from the pre-push eprfs build. The
  launcher `/opt/rust/cargo/bin/epr` → `/projects/.claude-config/tools/bin-store/bin/bin-launch` is
  that build's. A fresh session's `just status device` exits 0 (ok: epr, key, roster, model, trust,
  peer daemon; degraded: brit, fold lag, berth mooring). Diagnostic only — to read the device from
  inside an OLD session as a fresh one would, hide the runtime ancestry, because a declared session
  id no longer crosses a registered agent runtime (spec §10 A.1):
  `env -u CLAUDE_CODE_SESSION_ID EPR_PROC_ROOT=<empty dir> ELOHIM_SESSION_ID=probe-$(date +%s) just status device`.
- **The main checkout** `/projects/elohim` is on `dev` at `9bcb85973`. It holds an earlier
  session's uncommitted work (195 staged, ~20 unstaged paths: memory edits, `.eprfs/status` flow
  records, backlog notes). **Do not commit, reset or stash it**; it is that session's to finish.
  Before it was moved to `dev` it was backed up in `genesis/local-dev/main-checkout-2026-10-25/`
  (patches, untracked tarball, the five set-aside drafts that `dev` now tracks in newer versions)
  and the staged index at `refs/backup/main-checkout-2026-10-25-index`. **Do keep it on `dev`**
  (`git merge --ff-only origin/dev` after each landing, once you have checked no dirty path
  overlaps the incoming change): every Claude session's hooks run from this checkout, and so does
  a worktree's pre-push (`core.hooksPath` is absolute, operator ruling 2026-10-23). When it sat
  135 commits behind, those hooks ran old code that executed `epr` from a cargo build directory,
  so the recall bootstrap saw its executor change under every session and refused, and pre-push
  ran a version with no head-landing step at all.
- **brit:** `epr bin bootstrap --tool brit` still has not run here (status degraded). PR
  ethosengine/brit#3 still awaits review.
- **Housekeeping due:** the `cleanup:` headline is past its watermark (136 ≥ 120) and the memory
  index is over budget → `memory-stasis-loop`; the recall standing reader is due.

## Next, in order

1. **Pre-push head landing** (`genesis/data/timeline/backlog/prepush-head-landing-never-ingests.md`,
   high): after a green eprfs gate, ingest that gate's build at the pushed SHA, land it, and
   install the launcher when `bin-launch` changed; regression test through the hook. Without it
   every landing needs the hand steps above.
2. **Slice M**, the declared target. ark's `RuntimeManifest` (`elohim/ark/core/src/manifest.rs`)
   gains `follows` (which release channels the device follows, and in which mode) and `tools`
   (today's hand-edited `genesis/agentic/bin-tools.json`); one writer, `epr device follow | run |
   show | apply`, renders the `ELOHIM_RELEASE_CHANNELS` line, `deployments.json` follow entries and
   `bin-tools.json` from it, and the follow route and `ConfigEprVehicle` call that writer.
   - Write spec §12, the design note, and commit it before any code: the record's truth class,
     the writer's offline path, which habit the slice's scenario serves, and what the
     follow-set census (15 today) falls to.
   - Run the implementation leg, which the brief
     `genesis/local-dev/device-runtime/briefs/m-manifest.md` describes in full, in a prepared
     worktree (the brief says how), with the Codex CLI:
     `CODEX_HOME=/projects/.codex-config TMPDIR=/var/tmp/m-run codex exec -m gpt-6-astra -s workspace-write -C /projects/elohim/.worktrees/device-runtime-m --add-dir /var/tmp/eprfs-m-target --add-dir /var/tmp/m-run -c sandbox_workspace_write.exclude_slash_tmp=true -c sandbox_workspace_write.exclude_tmpdir_env_var=true -c model_reasoning_effort="high" --color never -o <result.md> - < <brief>`
     (create both `/var/tmp` dirs first; afterwards `ls -d /tmp/.git` and `rmdir` it if present).
     `genesis/local-dev/device-runtime/runs/a2-leg.sh` is the script this run used.
   - Review with a fresh read-only astra reviewer per round (`codex exec -s read-only --ephemeral`),
     fix, and run every gate yourself outside the sandbox before pushing.

## Traps this run paid for

- **Prepare a worktree before its first push** (submodules, its own `pnpm install`, element and
  client builds). Skipping it cost a 65-minute pre-push with 15 refused legs.
- **A green Codex sandbox is not a green device.** The Codex CLI runs an implementation leg in a
  sandbox with no Claude Code process above it, no live device key and no live bin store, so code
  that depends on the session, the device or the pin is never exercised there as pre-push
  exercises it. A2 passed four sandboxed full runs and failed four real pushes. Fixtures
  must isolate `EPR_PROC_ROOT`, `ELOHIM_DEVICE_KEY_FILE`, `ELOHIM_SESSION_ID` and the store.
- **Sessions keep their pins across a landing.** New tooling must treat a session whose `epr`
  predates a verb as having none of that verb's state (berth's envelope guard does). The tier gate
  is deliberately fail-closed for such sessions.
- **Never push with `git -c safe.directory=*`:** `GIT_CONFIG_PARAMETERS` carries it into every
  test, and ownership-refusal tests then see trust.
- **A serving-path change needs a household receipt** (`just test mesh
  features/dataplane/epr-app-deliverability.feature`); a fresh three-peer `MESH_DIR` mesh built
  from the worktree passed in ~32 minutes including builds.
- **`schema-codegen` can flag a generated, untracked seeder file** on alternating runs (the known
  Prettier oscillation); it passes on rerun.
- **Hook ledgers** (`.claude/data/*.jsonl`) are local append-only findings the hooks write on
  every tool call, so the tree is always dirty with them and they are never committed; a
  rebase refuses over them, so save their diff to `genesis/local-dev/` and restore the files
  first.

## Leftovers safe to remove

- `.worktrees/device-runtime-a`, `.worktrees/device-runtime-a2`, `.worktrees/land-self-heal`
  (its gitignored receipt `genesis/a2o/reports/sprint-report-household-20261025T022102Z-fe893b99.json`
  has served its push) and their branches — all landed.
- The 2026-10-24 handoff this session started from was never committed; it is an untracked
  file in the main checkout (`genesis/docs/superpowers/sprints/2026-10-24-household-follows-latest-handoff.md`).
  This handoff supersedes it. Its leftovers (`tmp/epr-pin-probe`, `.worktrees/household-follows-latest`,
  `.worktrees/hfl-landing`, `.worktrees/brit-follows`) are still present.
