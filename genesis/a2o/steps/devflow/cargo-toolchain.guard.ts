/**
 * The `@requires:cargo` gate -- a Rust toolchain on PATH is a FIXTURE precondition, not a
 * substrate capability, so the cluster-state arms ignore it. The genesis CI runner has no cargo
 * (rust-style.sh dies with "cargo: command not found"), so a scenario that builds Rust fixtures is
 * SKIPPED there with the reason printed once, never failed, and nothing is installed. It still runs
 * wherever cargo resolves. Mirrors steps/devflow/rakia-cli.guard.ts.
 */
import { accessSync, constants, statSync } from 'node:fs';
import { delimiter, join } from 'node:path';

import { Before } from '@cucumber/cucumber';

function isExecutableFile(candidate: string): boolean {
  try {
    if (!statSync(candidate).isFile()) return false;
    accessSync(candidate, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

function cargoOnPath(): boolean {
  return (process.env.PATH ?? '')
    .split(delimiter)
    .filter(Boolean)
    .some(dir => isExecutableFile(join(dir, 'cargo')));
}

let announced = false;

Before({ tags: '@requires:cargo' }, function () {
  if (cargoOnPath()) return undefined;
  if (!announced) {
    announced = true;
    // eslint-disable-next-line no-console
    console.log(
      '  [cargo] `cargo` is not on PATH -- @requires:cargo scenarios SKIPPED, not measured'
    );
  }
  return 'skipped';
});
