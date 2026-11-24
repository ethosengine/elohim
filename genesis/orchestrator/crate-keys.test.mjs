import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { crateKeys, EPRFS_RECIPE_STEP_COVERAGE } from './crate-keys.mjs';

function metadata(order = ['leaf', 'mid', 'top', 'unrelated']) {
  const deps = { leaf: [], mid: ['leaf'], top: ['mid'], unrelated: [] };
  return {
    packages: order.map(name => ({ id: name, name, manifest_path: `/repo/ws/${name}/Cargo.toml` })),
    workspace_members: [...order],
    resolve: {
      nodes: order.map(name => ({
        id: name,
        features: name === 'mid' ? ['zeta', 'alpha'] : [],
        deps: deps[name].map(pkg => ({ pkg, dep_kinds: [{ kind: null, target: null }] })),
      })),
    },
  };
}

function keys(overrides = {}) {
  const trees = { leaf: 'leaf-a', mid: 'mid-a', top: 'top-a', unrelated: 'unrelated-a', ...overrides.trees };
  return crateKeys({
    metadata: overrides.metadata || metadata(),
    treeOf: dir => trees[dir.split('/').at(-1)],
    toolchain: overrides.toolchain || 'rustc fixture\nhost: fixture',
    lockDigest: overrides.lockDigest || 'lock-a',
    outsideReadsDigest: 'reads-a',
    cargoEnv: { RUSTFLAGS: '', CARGO_BUILD_JOBS: '6' },
    step: overrides.step || 'eprfs:test',
  });
}

describe('per-crate verdict keys', () => {
  test('the eprfs recipe coverage attributes only workspace-wide steps', () => {
    assert.deepEqual(EPRFS_RECIPE_STEP_COVERAGE, {
      fmt: 'workspace', precheck: null, lint: 'workspace', bins: null, ark: null, test: 'workspace',
    });
    assert.equal(EPRFS_RECIPE_STEP_COVERAGE.unknown, undefined);
  });

  test('changing a leaf moves it and its reverse dependencies, but not an unrelated crate', () => {
    const before = keys();
    const after = keys({ trees: { leaf: 'leaf-b' } });
    for (const name of ['leaf', 'mid', 'top']) assert.notEqual(after[name], before[name], name);
    assert.equal(after.unrelated, before.unrelated);
  });

  test('changing a top crate moves only the top crate', () => {
    const before = keys();
    const after = keys({ trees: { top: 'top-b' } });
    assert.notEqual(after.top, before.top);
    for (const name of ['leaf', 'mid', 'unrelated']) assert.equal(after[name], before[name], name);
  });

  test('lock, toolchain and step changes each move every crate', () => {
    const before = keys();
    for (const after of [
      keys({ lockDigest: 'lock-b' }),
      keys({ toolchain: 'rustc other' }),
      keys({ step: 'eprfs:lint' }),
    ]) {
      for (const name of Object.keys(before)) assert.notEqual(after[name], before[name], name);
    }
  });

  test('keys are stable across runs and independent of member order', () => {
    const before = keys();
    assert.deepEqual(keys(), before);
    const reversed = ['unrelated', 'top', 'mid', 'leaf'];
    const shuffled = metadata(reversed);
    shuffled.resolve.nodes.reverse();
    for (const node of shuffled.resolve.nodes) node.deps.reverse();
    assert.deepEqual(keys({ metadata: shuffled }), before);
  });
});
