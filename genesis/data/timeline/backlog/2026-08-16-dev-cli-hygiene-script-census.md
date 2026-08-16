---
id: "backlog-dev-cli-hygiene-script-census"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Developer CLI hygiene: 362→322 npm scripts across 25 packages; eight public verbs and one manifest-driven gate path"
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
   (explicit workspace mapping through `pool-lib.sh`; never the mis-keying
   `cargo-pool key` inference), bin-only-crate handling — encoded once; CLAUDE.md build
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
- **Verified ALIVE at census time:** `hc:seed` (target `genesis/seeder/src/seed.ts`
  exists; path drift already fixed 2026-06-10 per `hc-seed-ports-file-path-drift.md`;
  advertised by `hc-start.sh`); the elohim-app→seeder pass-through aliases were
  live duplication, not dead code, and were retired only after the root `seed`
  verb replaced them; a2o `test:genesis:{epic,layer}`
  (parameterized `$npm_config_name`, `@epic:` tags exist in features); root
  `wire-types:*`, `validate:cargo-coverage`, `agentic:readiness` (targets exist);
  root `agentic:test` (glob matches `genesis/agentic/cargo-pool-doctor.test.mjs`).
- **Gospel drift fixed package-first:** the nonexistent elohim-app
  `pnpm run cypress:run` command was replaced by the real `genesis/a2o` E2E
  commands in the authoritative root gospel package and regenerated Claude/Codex
  projections.
- **Seeder safety correction:** `seed:validate` and `seed:dry-run` were not
  implemented modes; `seed.ts` ignored both flags and could write. Both aliases
  are gone, and the root `just seed validate` invokes the non-writing schema
  validator directly.

## Consolidation slice landed 2026-08-16

- **Public surface:** `just --list` exposes exactly eight verbs: `gate`, `test`,
  `dev`, `mesh`, `seed`, `look`, `status`, `codegen`. Safe defaults inspect or
  validate; they do not start services, generate files, or seed content.
- **One gate authority:** 32 local gate projects across 13 manifests now carry a
  typed `run` contract. `just gate` and pre-push share
  `genesis/orchestrator/gate-runner.mjs`; the grep detector and 700+ line shell
  project switch are gone. Gate-only checks use manifest `inputs` rather than a
  second name map.
- **Cargo pool:** native gates and the single-peer stack resolve explicit
  workspace slots with `pool-lib.sh`; storage keeps the custom getrandom flag,
  doorway/node/EPR clear it, eprfs keeps its `/tmp` exception, and DNA/WASM stays
  unredirected. Release artifact lookup follows the resolved pool slot, so no
  recipe depends on legacy in-tree `target/release` paths.
- **Runtime cleanup:** obsolete `storage-start.sh` and `hc-build.sh` were retired;
  `hc-start.sh` is the single-peer lifecycle owner. Snapshot/stats paths now point
  at the canonical `elohim/holochain/local-dev` and `genesis/seeder` locations.
- **Guidance cleanup:** the dead Che `start-doorway` command and its unpooled
  native build are gone; live Holochain development, roadmap acceptance, a2o
  diagnostics, and snapshot messages now route through the root verbs.
- **Script count:** the committed census population is **362 → 322** (-40).

  | Surface | Before | After | Decision |
  |---|---:|---:|---|
  | `genesis/seeder` | 69 | 53 | removed 2 unsafe pseudo-modes + 14 `*:dev` endpoint copies; root `seed action [profile] [scope] [limit]` owns `local|alpha` profiles; distinct validators/seeders/snapshot operations stay specialist commands |
  | `app/elohim-app` | 62 | 38 | removed 23 cross-package/stale lifecycle aliases; retained Angular and canonical `hc:*` compatibility commands |
  | root | 54 | 54 | retained as CI/codegen implementation surface behind the eight root verbs |
  | `genesis/a2o` | 42 | 41 | removed exact duplicate `test:practical`; tag-specific suites are intentional named test profiles |

The literal package-script count is no longer the public cognitive surface.
Remaining scripts are either CI implementation commands, verified specialist
operations, or compatibility names with live consumers. They do not get deleted
by count alone; the next removal requires a consumer migration or deadness proof.

## Evidence

- `validate-manifests.mjs`: 13 manifests, 33 build steps, 0 errors.
- orchestrator suite: 101/101 tests green, including typed execution, path
  resolution, gate-only inputs, seeding safety, seam-contracts, and cargo
  workspace mappings.
- seeder suite: 487 passed, 9 skipped; a2o unit suite: 180/180 passed.
- epr-meta evaluator/resolver: 20 + 29 assertions green; app manifest `check_meta=[]`.
- both edited READMEs passed fresh-context blind-reader review.
- package-first projections for the root gospel, hc-dev-orchestrator, and
  seed-workflow regenerate cleanly; the full verifier has one pre-existing,
  unrelated stale runtime projection (`agentic-developer`).

- 2026-08-16: `just mesh monitor` added — unified mesh system monitor (`app/elohim-app/scripts/hc-mesh-monitor.py`, stdlib python, default :4210 = devfile `mesh-monitor` endpoint, `MESH_MONITOR_PORT` override). Reader over existing probe surfaces; gate-leg panel mirrors fleet-quiesce-gate.sh.
