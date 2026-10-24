/**
 * The `@requires:epr-cli` gate — shared by every devflow feature that drives the `epr` binary.
 *
 * WHY IT EXISTS. `@requires:<cap>` has two arms in this repo, and `epr-cli` reaches NEITHER of
 * them: the scope reconciler only knows capabilities declared in
 * `genesis/manifests/cluster-state.yaml`, and the runtime `Before` gate in `steps/common.steps.ts`
 * reads that same manifest. `epr-cli` is a FIXTURE precondition — a binary on PATH, not a
 * substrate dependency point — so both arms ignore it and the tag documented a precondition
 * nothing enforced. On an image without the binary (the genesis CI a2o run executes `@act:host`
 * devflow features and has no `epr` on PATH) every scenario died mid-step on
 * `spawnSync epr ENOENT` and was counted as a RED.
 *
 * A missing binary is NOT a failed assertion. It is a scenario that was never measured, and the
 * honest disposition is the one a substrate-held or `@wip` scenario already gets here: SKIPPED,
 * with the reason printed once so the absence is visible rather than inferred from a silent pass.
 *
 * WHAT IT DOES NOT DO. It never fails a run, it never installs anything, and it never green-washes:
 * a skipped scenario is absent from the measured set, which is exactly what a reader of
 * `declared.notMeasured` needs it to be. Present-but-broken is out of scope by design — this
 * resolves the binary the same way `spawnSync` will, and nothing more.
 */
import { Before } from '@cucumber/cucumber';

import { holdWithoutEpr } from './epr-binary.js';

// The resolver and the held line live in epr-binary.ts, so a feature whose own `Before` needs the
// binary resolves it the same way and holds the same way.
Before({ tags: '@requires:epr-cli' }, function (scenario) {
  return holdWithoutEpr(scenario);
});
