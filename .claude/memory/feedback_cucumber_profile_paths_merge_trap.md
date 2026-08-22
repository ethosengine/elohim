---
name: feedback_cucumber_profile_paths_merge_trap
title: Cucumber profile paths MERGE with positionals
description: "a2o trap: `cucumber-js -p local <files>` runs the WHOLE suite — profile `paths` MERGE with positionals; scope via `--config <empty>` (repo-root-relative) or `--name`."
metadata:
  type: feedback
---

Three agents and the orchestrator each burned 15+ min on 2026-08-21 running "two feature files" that were
actually ~950 scenarios against the shared mesh. `genesis/a2o/cucumber.mjs` sets `paths: ['features/**/*.feature']`
on `default` AND `local`; cucumber-js merges config paths with CLI positionals (its own comment records this
as the 2026-08-16 `saga`-profile finding). Omitting `-p local` does not help — the `default` profile applies.

**How to apply:** scope with an empty config: `npx cucumber-js --config ../../../../tmp/<scratch>/empty.cucumber.mjs
--require-module tsx --require 'steps/**/*.ts' <files> --format summary` (file contains `export default {};`;
`--config` is resolved relative to the repo root `/projects/elohim`, not cwd), or `-p local --name '^Scenario name$'`.
Never give an agent a bare `-p local <files>` re-run command. Related: [[project_local_pair_failover_validation_rail]].
