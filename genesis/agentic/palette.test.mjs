import { test } from 'node:test';
import assert from 'node:assert/strict';
import { matchesPalette, loadPalette } from './palette.mjs';

const palette = [
  'Bash(pnpm run test)',
  'Bash(pnpm run test:*)',
  'Bash(RUSTFLAGS="" cargo *)',
  'Bash(git add :*)',
  'Bash(git commit -m :*)',
  'Bash(mcp__jenkins__*)',
];

test('exact match hits', () => {
  assert.equal(matchesPalette('pnpm run test', palette), true);
});

test('suffix wildcard hits', () => {
  assert.equal(matchesPalette('pnpm run test:unit', palette), true);
  assert.equal(matchesPalette('pnpm run test:e2e', palette), true);
});

test('RUSTFLAGS cargo wildcard hits all cargo subcommands', () => {
  assert.equal(
    matchesPalette('RUSTFLAGS="" cargo build', palette),
    true,
  );
  assert.equal(
    matchesPalette('RUSTFLAGS="" cargo test --workspace', palette),
    true,
  );
});

test('unrelated command misses', () => {
  assert.equal(matchesPalette('kubectl get pods', palette), false);
});

test('similar-but-different prefix misses', () => {
  assert.equal(matchesPalette('pnpm exec prettier', palette), false);
});

test('mcp tool prefix matches family', () => {
  assert.equal(matchesPalette('mcp__jenkins__getBuild', palette), true);
  assert.equal(matchesPalette('mcp__jenkins__triggerBuild', palette), true);
  assert.equal(matchesPalette('mcp__sonarqube__whatever', palette), false);
});

test('loadPalette reads settings files and extracts Bash entries', async () => {
  const { readFileSync, writeFileSync, mkdirSync, rmSync } = await import('node:fs');
  const { join } = await import('node:path');
  const { tmpdir } = await import('node:os');
  const dir = join(tmpdir(), `palette-test-${Date.now()}`);
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, 'settings.json'),
    JSON.stringify({ permissions: { allow: ['Bash(pnpm *)', 'Bash(git status)'] } }),
  );
  writeFileSync(
    join(dir, 'settings.local.json'),
    JSON.stringify({ permissions: { allow: ['Bash(cargo *)'] } }),
  );
  const loaded = loadPalette({
    durablePath: join(dir, 'settings.json'),
    localPath: join(dir, 'settings.local.json'),
  });
  assert.deepEqual(
    loaded.sort(),
    ['Bash(cargo *)', 'Bash(git status)', 'Bash(pnpm *)'],
  );
  rmSync(dir, { recursive: true, force: true });
});

test('loadPalette tolerates missing local file', async () => {
  const { readFileSync, writeFileSync, mkdirSync, rmSync } = await import('node:fs');
  const { join } = await import('node:path');
  const { tmpdir } = await import('node:os');
  const dir = join(tmpdir(), `palette-test-missing-${Date.now()}`);
  mkdirSync(dir, { recursive: true });
  writeFileSync(
    join(dir, 'settings.json'),
    JSON.stringify({ permissions: { allow: ['Bash(ls)'] } }),
  );
  const loaded = loadPalette({
    durablePath: join(dir, 'settings.json'),
    localPath: join(dir, 'does-not-exist.json'),
  });
  assert.deepEqual(loaded, ['Bash(ls)']);
  rmSync(dir, { recursive: true, force: true });
});
