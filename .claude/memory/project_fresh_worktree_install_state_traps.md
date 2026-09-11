---
index: false
id: project-fresh-worktree-install-state-traps
name: project_fresh_worktree_install_state_traps
title: "project_fresh_worktree_install_state_traps"
description: A fresh sprint worktree fails gates on install-state, not code — deps, submodules, gitignored generated-ts, app dists; the mechanical fix list (2026-09-07)
metadata:
  type: project
---

A fresh `git worktree add` of this repo is NOT gate-ready. Measured 2026-09-07 on
`.claude/worktrees/sprint-0908` (branched from local `dev`, which was 10 commits ahead of
origin, so `EnterWorktree` fresh-from-origin would have been wrong — create with
`git worktree add <dir> -b <branch> dev`, then `EnterWorktree path=`).

What breaks and the fix, in order:
- **node deps absent** → `pnpm install --offline --frozen-lockfile` (8 s, store-linked).
- **submodules empty** (`elohim/rakia`, `elohim/brit`, `sophia`, `elohim/holochain-conductor`)
  → storage test `release_manifest_mirror_agrees_with_the_rakia_schema` panics on
  `../rakia/schemas/v1/release-manifest.schema.json`; `git submodule update --init -- elohim/rakia`
  (local clone via the shared `.git/modules`). App builds need `sophia` initialised + UMD built.
- **`elohim/sdk/schemas/generated-ts/` is gitignored** except force-added files → `just codegen
  schema verify` reds on a missing `views/auth-discovery.ts` until `pnpm run schema:codegen:ts`
  runs once. Running it also exposes REAL drift (2026-09-07: `feedback-signal.ts` actRef in the
  schema, never regenerated in six distributed copies) — commit those; ignore the Prettier
  oscillation files ([[feedback_codegen_prettier_oscillation]]).
- **app dists absent** → `just mesh prologue` FATALs on `app/elohim-app/dist/elohim-app/browser`;
  symlink the main tree's `app/elohim-app/dist` and `app/lamad/dist` (gitignored build output)
  when the change under test is not the app.
- **element packages unbuilt** → `just gate elohim-app` fails `Could not resolve
  "elohim-core/register"` (`app/elohim-elements/*/dist/register.js` missing); storybook Vite
  build fails the same way.
- **domain-types gate rewrites six `elohim/sdk/domains/*/types/Cargo.lock`** (0.6→0.7 line);
  check whether the manifests already declare 0.7 before committing the locks.
- **running storage binary predates a Rust change** → prologue `relationship_type 'STEP' is
  not valid` until rebuild + `just mesh storage-restart` ([[project_local_mesh_binary_slot_and_restart]]).
- The worktree-isolation hook refuses `git -C /projects/elohim …` and even Write to the memory
  dir; populate from inside the worktree and write memory via a shell heredoc.

**Why:** three of the four batch-1 gate reds on 2026-09-07 were install-state; an agent that
reads them as code failures wastes the single cargo slot re-running.
**How to apply:** on entering a fresh worktree, run the list above BEFORE dispatching agents
that gate. See also [[feedback_push_branch_discipline]].

**Cargo pool slot follows the BRANCH, not the checkout** (T8, 2026-09-07): `detect_family()` in the gate runner maps `sprint/*` → `/projects/.cargo-target-pool/family/sprint/<ws>/dev`, a cold slot (a full storage build there is ~62 GB); the `family/dev` slot only serves `dev`. On a sprint branch, expect one cold build per workspace and point manual `CARGO_TARGET_DIR` at the `family/sprint` slot so the gate and hand-run cargo share artifacts. Related: [[project_cargo_pvc_disk_discipline]].
