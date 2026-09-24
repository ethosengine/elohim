/**
 * The `@requires:rakia-cli` gate — the rakia planner is a FIXTURE precondition (a binary on
 * PATH or RAKIA_BIN), not a substrate capability, so the cluster-state arms ignore it. A
 * scenario that needs it is SKIPPED with the reason printed once when the binary is absent;
 * it is never failed and nothing is installed. Mirrors steps/devflow/epr-cli.guard.ts.
 */
import { accessSync, constants, statSync } from 'node:fs';
import { delimiter, isAbsolute, join } from 'node:path';

import { Before } from '@cucumber/cucumber';

const binary = process.env.RAKIA_BIN ?? 'rakia';

function isExecutableFile(candidate: string): boolean {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function resolveBinary(): string | null {
  if (isAbsolute(binary) || binary.includes('/')) return isExecutableFile(binary) ? binary : null;
  for (const dir of (process.env.PATH ?? '').split(delimiter).filter(Boolean)) {
    const candidate = join(dir, binary);
    if (isExecutableFile(candidate)) return candidate;
  }
  return null;
}

let announced = false;

Before({ tags: '@requires:rakia-cli' }, function () {
  if (resolveBinary()) return undefined;
  if (!announced) {
    announced = true;
    // eslint-disable-next-line no-console
    console.log(
      `  [rakia-cli] '${binary}' is not on PATH — @requires:rakia-cli scenarios SKIPPED, not measured`
    );
  }
  return 'skipped';
});
