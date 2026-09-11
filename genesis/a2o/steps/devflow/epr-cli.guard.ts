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
import { accessSync, constants, statSync } from 'node:fs';
import { delimiter, isAbsolute, join } from 'node:path';

import { Before } from '@cucumber/cucumber';

/** The same spelling every devflow step file uses to name the binary. */
const binary = process.env.EPR_BIN ?? 'epr';

function isExecutableFile(candidate: string): boolean {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

/**
 * Resolve `binary` exactly the way `spawnSync` would: a name carrying a separator is taken as a
 * path, a bare name is searched along PATH. Computed once — PATH does not change mid-run, and a
 * per-scenario stat storm would buy nothing.
 */
function resolveEprBinary(): string | null {
  if (isAbsolute(binary) || binary.includes('/')) {
    return isExecutableFile(binary) ? binary : null;
  }
  for (const entry of (process.env.PATH ?? '').split(delimiter)) {
    if (entry.length === 0) continue;
    const candidate = join(entry, binary);
    if (isExecutableFile(candidate)) return candidate;
  }
  return null;
}

const resolved = resolveEprBinary();
let announced = false;

Before({ tags: '@requires:epr-cli' }, function (scenario) {
  if (resolved !== null) return undefined;
  if (!announced) {
    announced = true;
    // eslint-disable-next-line no-console
    console.log(
      `  ⏭️  HELD (@requires:epr-cli): \`${binary}\` is not on PATH in this image — ` +
        'every scenario carrying this tag is skipped, not failed (not measured, not green).'
    );
  }
  // eslint-disable-next-line no-console
  console.log(`  ⏭️  HELD (@requires:epr-cli): "${scenario.pickle.name}"`);
  return 'skipped';
});
