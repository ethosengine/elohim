/**
 * The one `epr` resolver for devflow steps, and the one way a scenario is held when there is none.
 *
 * `epr-cli.guard.ts` holds every `@requires:epr-cli` scenario through `holdWithoutEpr`; a feature
 * whose own `Before` needs the binary (devices-share-one-berth) calls the same function first, so a
 * run with no `epr` skips its scenarios instead of failing them, whichever `Before` cucumber reaches
 * first (it skips every later `Before` once one returns 'skipped').
 */
import { accessSync, constants, statSync } from 'node:fs';
import { delimiter, isAbsolute, join } from 'node:path';

/** The same spelling every devflow step file uses to name the binary. */
export const eprBinary = process.env.EPR_BIN ?? 'epr';

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
 * Resolve `eprBinary` exactly the way `spawnSync` would: a name carrying a separator is taken as a
 * path, a bare name is searched along PATH. Computed once — PATH does not change mid-run, and a
 * per-scenario stat storm would buy nothing.
 */
function resolveEprBinary(): string | null {
  if (isAbsolute(eprBinary) || eprBinary.includes('/')) {
    return isExecutableFile(eprBinary) ? eprBinary : null;
  }
  for (const entry of (process.env.PATH ?? '').split(delimiter)) {
    if (entry.length === 0) continue;
    const candidate = join(entry, eprBinary);
    if (isExecutableFile(candidate)) return candidate;
  }
  return null;
}

/** The executable `epr` this run uses, or null when there is none. */
export const resolvedEpr = resolveEprBinary();

let announced = false;

/**
 * 'skipped' (with the reason printed: once for the run, then once per scenario) when there is no
 * `epr`; undefined when there is one.
 */
export function holdWithoutEpr(scenario: { pickle: { name: string } }): 'skipped' | undefined {
  if (resolvedEpr !== null) return undefined;
  if (!announced) {
    announced = true;
    // eslint-disable-next-line no-console
    console.log(
      `  ⏭️  HELD (@requires:epr-cli): \`${eprBinary}\` is not on PATH in this image — ` +
        'every scenario carrying this tag is skipped, not failed (not measured, not green).'
    );
  }
  // eslint-disable-next-line no-console
  console.log(`  ⏭️  HELD (@requires:epr-cli): "${scenario.pickle.name}"`);
  return 'skipped';
}
