/**
 * Every cucumber profile that can select a `@requires:cargo` feature must load the cargo guard.
 * Broad profiles pick guards up through the `steps/<dir>/*.ts` glob; narrow profiles list their
 * step files explicitly and must name the guard too.
 */
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { test } from 'node:test';
import { fileURLToPath, pathToFileURL } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '../../..');
const GUARD = 'steps/devflow/cargo-toolchain.guard.ts';
const DIR_GLOB = 'steps/' + '*/' + '*.ts';

void test('every profile selecting rust-style.feature loads the cargo guard', async () => {
  const mod = await import(pathToFileURL(resolve(root, 'cucumber.mjs')).href);
  const profiles = mod.default() as Record<string, { require: string[]; paths: string[] }>;
  const feature = 'features/devflow/rust-style.feature';
  assert.match(readFileSync(resolve(root, feature), 'utf8'), /@requires:cargo/);
  assert.ok(profiles['rust-style'].require.includes(GUARD));
  for (const [name, p] of Object.entries(profiles)) {
    if (!p.paths.includes(feature)) continue;
    assert.ok(
      p.require.includes(DIR_GLOB) || p.require.includes(GUARD),
      `profile ${name} selects ${feature} without the cargo guard`
    );
  }
});
