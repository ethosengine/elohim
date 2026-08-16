---
id: "backlog-dev-cli-hygiene-script-census"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Developer CLI hygiene: 362 npm scripts across 25 packages + a drifted root justfile — consolidate onto a manifest-driven verb set to cut cognitive/context load"
slug: "dev-cli-hygiene-script-census"
written: "2026-08-16"
author: "claude-fable"
status: "open"
priority: "medium"
area: "dev-loop"
domain: "process"
relatedNodeIds:
---

# Developer CLI hygiene — script census + consolidation direction

Operator-opened thread (2026-08-16): reduce complexity, context, and cognitive load
of the `npm run *` / CLI surface for the whole project.

## Census (measured 2026-08-16, worktrees/research-repos/sophia excluded)

- **362 script entries across 25 packages.** Four surfaces carry 63% of it:
  `genesis/seeder` (69), `app/elohim-app` (62), root `package.json` (54),
  `genesis/a2o` (42).
- **Name fragmentation:** `build` ×19, `test` ×16, `lint` ×10, `start` ×8,
  `typecheck` ×8, `format` ×6 — the same verb re-spelled per package, each with
  local quirks (RUSTFLAGS variants, path-dependent invocation, env-var soup).
- **Bloat classes** (from the four heavy surfaces): per-entity seed variants
  (`seed:X` + `seed:X:dev` + `seed:X:dry` — the seeder's 69 are mostly one verb ×
  a corpus × an env matrix), per-domain codegen pairs (`X:codegen` +
  `X:codegen:verify` ×9 at root), per-tag test aliases (a2o's 20+ `test:*`),
  lifecycle glue (`pre*/post*`).
- **A root `justfile` already exists** (~26 recipes: status, stack-start,
  storage-build, dna-*, seed…) but is **drifted**: last touched 2026-06-23, has
  ZERO `CARGO_TARGET_DIR`/cargo-pool awareness (the PreToolUse disk-guard DENIES
  native cargo lacking it — so `just storage-build` is either denied or mints a
  rogue legacy `target/`), points at `elohim/elohim-storage/target/release` (the
  exact legacy-target class `cargo-pool legacy-targets` reclaims), and predates
  hc-mesh / quiesce harness / `epr flow` / habits-status.

## Direction (from the parent-thread framing, ratified for capture)

1. **One root entrypoint, ~8 stable verbs** — revive the EXISTING root justfile
   (do not mint a new tool): `gate`, `test`, `dev`, `mesh` (start/stop/status/
   probe/quiesce), `seed`, `look`, `status` (habits/saga), `codegen`.
   `just --list` becomes the discoverable surface.
2. **Manifest-driven `gate`** — resolve the current tree's steps from
   `build-manifest.json` `gate.projects` (via `pipeline-registry.mjs`) so the
   pre-push hook, CI, and humans run ONE code path; kills the documented
   two-detection-path drift class (manifest walker vs grep-fallback `case`).
3. **Bury the gotchas in recipes** — RUSTFLAGS per crate, cargo-pool slot
   (`cargo-pool key`), bin-only-crate handling — encoded once; CLAUDE.md build
   section shrinks toward "run `just gate`" (direct agent-context reduction).
4. **Named profiles over env soup** — generalize the dev-tier pacing profile
   pattern (one declared block, override-able defaults) instead of ambient exports.
5. **Prune by census, not taste** — map each of the 362 scripts to a habit/gate/
   real workflow; unmapped ones get a deprecation echo for a cycle, then deletion;
   matrix-shaped families (seeder) collapse to one parameterized verb.
6. **Trajectory-respecting** — thin scaffolding composing toward lvi / `epr flow`
   / eprfs-rooted tooling, never a rival framework.

## First surgical pass (ran 2026-08-16, this doc's mint session)

Verified-dead vs verified-alive, so the next pass doesn't re-derive it:

- **Fixed:** `app/elohim-app` `build:sophia` pointed at `../sophia` (= `app/sophia`,
  which does not exist; sophia is a repo-root submodule) → corrected to `../../sophia`.
- **Removed:** `app/elohim-app` `sonar:preview` — SonarQube's `analysis.mode=preview`
  was removed upstream years ago; the script could never run against the current server.
- **Verified ALIVE (do not re-flag):** `hc:seed` (target `genesis/seeder/src/seed.ts`
  exists; path drift already fixed 2026-06-10 per `hc-seed-ports-file-path-drift.md`;
  advertised by `hc-start.sh:561`); the 12 elohim-app→seeder pass-through aliases
  (`seed:*`, `stats:*`, `snapshot:*`, `diagnose*` — targets all exist; `snapshot:*`
  referenced by the seed-workflow skill) — these are the DUPLICATION class for the
  justfile consolidation, not dead code; a2o `test:genesis:{epic,layer}`
  (parameterized `$npm_config_name`, `@epic:` tags exist in features); root
  `wire-types:*`, `validate:cargo-coverage`, `agentic:readiness` (targets exist);
  root `agentic:test` (glob matches `genesis/agentic/cargo-pool-doctor.test.mjs`).
- **Gospel drift (note only — managed surface, needs a cite-tooling pass):** root
  CLAUDE.md documents `pnpm run cypress:run` for elohim-app E2E; no such script exists
  (no cypress config in elohim-app; E2E lives in `genesis/a2o`, reachable via the
  `e2e`/`e2e:browser` aliases).
- **Not touched (active sprint write-set):** `genesis/seeder/package.json` — the
  minutes-quiesce sprint's Q5 agent owns seeder edits today; the 69-script matrix
  collapse waits for the justfile slice.

## First slice (when picked up)

Fix + extend the root justfile: pool-aware `gate`/`build` recipes (unbreaks it
against the disk-guard), add `mesh`/`quiesce` verbs wrapping
`app/elohim-app/scripts/hc-mesh{,-quiesce}.sh`, and the manifest-driven `gate`
resolver. Then the seeder matrix collapse (69 → ~10 parameterized).
