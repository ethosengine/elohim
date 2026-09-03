import { test, describe } from 'node:test';
import { strict as assert } from 'node:assert';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';
import {
  parseCommitTags,
  parseSkipCi,
  parseConductorOptions,
  CONDUCTOR_FEATURE_PRESETS,
} from './commit-tag-parser.mjs';
import { loadPipelineRegistry } from './pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');
const registry = loadPipelineRegistry(ROOT);

describe('parseCommitTags', () => {
  test('[build:app] returns elohim', () => {
    assert.deepStrictEqual(parseCommitTags('ci: retrigger [build:app]', registry), ['elohim']);
  });

  test('[build:edge,app] returns both', () => {
    assert.deepStrictEqual(
      parseCommitTags('[build:edge,app] fix things', registry).sort(),
      ['elohim', 'elohim-edge'].sort()
    );
  });

  test('[build:all] returns all non-manual pipelines', () => {
    const result = parseCommitTags('[build:all]', registry);
    assert.ok(!result.includes('elohim-steward'), 'should exclude steward (manualOnly)');
    assert.ok(result.includes('elohim'), 'should include elohim');
    assert.ok(result.length >= 6, `expected ≥6 pipelines, got ${result.length}`);
  });

  test('unknown tag silently dropped', () => {
    assert.deepStrictEqual(parseCommitTags('[build:nonsense] foo', registry), []);
  });

  test('no tag returns empty', () => {
    assert.deepStrictEqual(parseCommitTags('regular commit', registry), []);
  });
});

describe('parseConductorOptions', () => {
  test('absent tag reports not present, no overrides', () => {
    const o = parseConductorOptions('feat: unrelated work');
    assert.strictEqual(o.present, false);
    assert.strictEqual(o.canary, false);
    assert.strictEqual(o.skipPush, false);
    assert.strictEqual(o.features, undefined);
  });

  test('bare tag is present but overrides nothing — job defaults stand', () => {
    const o = parseConductorOptions('build(conductor): rebuild [conductor:]');
    assert.strictEqual(o.features, undefined, 'production feature set must come from the job, not a second copy');
  });

  test('iroh preset is a 0.7 compatibility no-op (iroh is the only transport, no cargo feature)', () => {
    const o = parseConductorOptions('[conductor:iroh]');
    assert.strictEqual(o.features, CONDUCTOR_FEATURE_PRESETS.iroh);
    assert.ok(!o.features.includes('transport-iroh'));
    assert.ok(o.features.includes('jemalloc'));
    assert.ok(!o.features.includes('jemalloc-prof'));
  });

  test('prof preset selects the profiling feature set', () => {
    const o = parseConductorOptions('[conductor:prof]');
    assert.strictEqual(o.features, CONDUCTOR_FEATURE_PRESETS.prof);
    assert.ok(o.features.includes('jemalloc-prof'));
  });

  test('every preset keeps jemalloc — the bare glibc set is the one that leaked', () => {
    for (const [name, features] of Object.entries(CONDUCTOR_FEATURE_PRESETS)) {
      assert.ok(features.includes('jemalloc'), `${name} preset dropped jemalloc`);
    }
  });

  test('flags combine within one tag', () => {
    const o = parseConductorOptions('build: canary the iroh conductor [conductor:iroh,canary]');
    assert.strictEqual(o.canary, true);
    assert.strictEqual(o.features, CONDUCTOR_FEATURE_PRESETS.iroh);
  });

  test('no-push and dry both set skipPush', () => {
    assert.strictEqual(parseConductorOptions('[conductor:no-push]').skipPush, true);
    assert.strictEqual(parseConductorOptions('[conductor:dry]').skipPush, true);
  });

  test('branch overrides parse', () => {
    const o = parseConductorOptions('[conductor:hc=elohim-0.6.4,tx5=my-fix]');
    assert.strictEqual(o.hcBranch, 'elohim-0.6.4');
    assert.strictEqual(o.tx5Branch, 'my-fix');
  });

  test('features= uses + as the separator and wins over a preset', () => {
    const o = parseConductorOptions('[conductor:iroh,features=sqlite-encrypted+wasmer_sys+jemalloc]');
    assert.strictEqual(o.features, 'sqlite-encrypted,wasmer_sys,jemalloc');
  });

  test('repeated tags merge', () => {
    const o = parseConductorOptions('[conductor:canary] and [conductor:hc=elohim-0.6.4]');
    assert.strictEqual(o.canary, true);
    assert.strictEqual(o.hcBranch, 'elohim-0.6.4');
  });

  test('unknown items are collected, not silently swallowed', () => {
    const o = parseConductorOptions('[conductor:canary,bogus,nope=]');
    assert.strictEqual(o.canary, true);
    assert.deepStrictEqual(o.unknown, ['bogus', 'nope=']);
  });

  test('a build tag alone does not imply conductor options', () => {
    assert.strictEqual(parseConductorOptions('[build:conductor]').present, false);
  });
});

describe('parseSkipCi', () => {
  test('[skip ci] returns true', () => {
    assert.strictEqual(parseSkipCi('chore: docs [skip ci]'), true);
  });

  test('[ci skip] returns true', () => {
    assert.strictEqual(parseSkipCi('chore: docs [ci skip]'), true);
  });

  test('no tag returns false', () => {
    assert.strictEqual(parseSkipCi('regular commit'), false);
  });
});
