#!/usr/bin/env node
/**
 * Verifies that `dist/custom-elements.json` is fresh relative to source TS
 * for each elohim-elements package that has a CEM config.
 *
 * Mirrors the schema:codegen --verify pattern: regenerates the manifest to a
 * temp directory and diffs against the committed file. Fails if they differ.
 *
 * Usage:
 *   node tools/check-cem-fresh.mjs           # regenerate (runs full pnpm build per pkg)
 *   node tools/check-cem-fresh.mjs --verify  # diff-only; nonzero exit on drift
 */
import { execSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { mkdir, readdir, readFile, rm } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const REPO_ROOT = resolve(__dirname, '..');
const VERIFY = process.argv.includes('--verify');

const ELEMENTS_DIR = join(REPO_ROOT, 'app/elohim-elements');

if (!existsSync(ELEMENTS_DIR)) {
  console.log('[cem-fresh] No app/elohim-elements directory yet — nothing to check.');
  process.exit(0);
}

// Discover packages: every directory under app/elohim-elements/ that has a
// custom-elements-manifest.config.mjs file.
const dirs = (await readdir(ELEMENTS_DIR, { withFileTypes: true }))
  .filter((d) => d.isDirectory())
  .map((d) => join(ELEMENTS_DIR, d.name))
  .filter((d) => existsSync(join(d, 'custom-elements-manifest.config.mjs')));

if (dirs.length === 0) {
  console.log(
    '[cem-fresh] No elohim-elements packages with manifest configs yet — nothing to check.',
  );
  process.exit(0);
}

let failed = false;

for (const dir of dirs) {
  const name = dir.split('/').pop();
  const committed = join(dir, 'dist/custom-elements.json');

  if (VERIFY) {
    if (!existsSync(committed)) {
      console.error(
        `[cem-fresh] ${name}: dist/custom-elements.json missing. ` +
          `Run \`pnpm run elements:codegen\` and commit.`,
      );
      failed = true;
      continue;
    }
    // The cem CLI joins cwd with the --outdir flag (path.join(cwd, outdir)),
    // so absolute paths in /tmp end up rooted under the package directory
    // anyway. Use a unique subdir under the package's own tmp/ (gitignored)
    // and pass it as a relative path to keep cem's path math sane.
    const tmpRel = `tmp/cem-verify-${process.pid}-${Date.now()}`;
    const tmp = join(dir, tmpRel);
    await mkdir(tmp, { recursive: true });
    try {
      // Run the analyzer fresh against current source, output to tmp dir.
      // The package's analyze script uses the local config; we override
      // outdir on the command line so we don't clobber the committed file.
      execSync(
        `pnpm --filter ${name} exec cem analyze --config custom-elements-manifest.config.mjs --outdir ${tmpRel}`,
        { cwd: REPO_ROOT, stdio: 'pipe' },
      );
      const fresh = join(tmp, 'custom-elements.json');
      if (!existsSync(fresh)) {
        console.error(`[cem-fresh] ${name}: cem analyze did not emit custom-elements.json to tmp.`);
        failed = true;
        continue;
      }
      // Normalize: parse + re-stringify to ignore whitespace-only diffs.
      const a = await readFile(committed, 'utf8');
      const b = await readFile(fresh, 'utf8');
      const aNorm = JSON.stringify(JSON.parse(a), null, 2);
      const bNorm = JSON.stringify(JSON.parse(b), null, 2);
      if (aNorm !== bNorm) {
        console.error(
          `[cem-fresh] ${name}: dist/custom-elements.json is STALE. ` +
            `Run \`pnpm run elements:codegen\` and commit the regenerated file.`,
        );
        failed = true;
      } else {
        console.log(`[cem-fresh] ${name}: fresh.`);
      }
    } finally {
      await rm(tmp, { recursive: true, force: true });
    }
  } else {
    // Regen mode — full build (vite + cem).
    console.log(`[cem-fresh] ${name}: regenerating...`);
    execSync(`pnpm --filter ${name} run build`, { cwd: REPO_ROOT, stdio: 'inherit' });
  }
}

process.exit(failed ? 1 : 0);
