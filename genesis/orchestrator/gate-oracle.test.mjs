import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { depthOne, rakiaAffected, resolveRakiaBin } from './gate-oracle.mjs';

const direct = (name, path) => ({ qualified_name: name, affected_by: [{ kind: 'changedFile', path }] });
const viaUp = (name, upstream) => ({ qualified_name: name, affected_by: [{ kind: 'upstreamNode', upstream }] });

describe('depthOne — the local gate stops one hop from a direct change', () => {
  test('keeps direct steps and their immediate dependents, drops the second hop', () => {
    const plan = [
      direct('elohim-sophia:build-sophia-umd', 'sophia'),
      viaUp('elohim:build-angular', 'elohim-sophia:build-sophia-umd'),
      viaUp('elohim:build-site-image', 'elohim:build-angular'),
      viaUp('elohim-genesis:seed-content', 'elohim:build-site-image'),
    ];
    const kept = depthOne(plan);
    assert.deepEqual([...kept.keys()], ['elohim-sophia:build-sophia-umd', 'elohim:build-angular']);
    assert.deepEqual(kept.get('elohim-sophia:build-sophia-umd'), ['source: sophia']);
    assert.deepEqual(kept.get('elohim:build-angular'), ['upstream: elohim-sophia:build-sophia-umd']);
  });

  test('a step reached both directly and via upstream carries both reasons, once each', () => {
    const plan = [
      direct('a:one', 'x/file'),
      { qualified_name: 'a:two', affected_by: [{ kind: 'changedFile', path: 'y/file' }, { kind: 'upstreamNode', upstream: 'a:one' }] },
    ];
    const kept = depthOne(plan);
    assert.deepEqual(kept.get('a:two'), ['source: y/file', 'upstream: a:one']);
  });

  test('an empty plan is an empty map', () => {
    assert.equal(depthOne([]).size, 0);
  });
});

describe('rakiaAffected — the binary is optional and its failure is a fallback, never a crash', () => {
  const root = '/repo';
  test('returns null when no binary resolves', () => {
    assert.equal(rakiaAffected(root, ['sophia'], { rakiaBin: null, spawn: () => { throw new Error('must not spawn'); } }), null);
  });

  test('returns null on a non-zero exit', () => {
    const spawn = () => ({ status: 2, stdout: '', stderr: 'manifest error' });
    assert.equal(rakiaAffected(root, ['sophia'], { rakiaBin: '/bin/rakia', spawn }), null);
  });

  test('passes --repo, --files as one comma-joined argument, and parses the affected list', () => {
    let seen;
    const spawn = (bin, args) => {
      seen = [bin, args];
      return { status: 0, stdout: JSON.stringify({ changed_paths: ['sophia'], affected: [direct('elohim-sophia:build-sophia-umd', 'sophia'), viaUp('elohim:build-angular', 'elohim-sophia:build-sophia-umd')] }), stderr: '' };
    };
    const kept = rakiaAffected(root, ['sophia', 'a b'], { rakiaBin: '/bin/rakia', spawn });
    assert.deepEqual(seen, ['/bin/rakia', ['affected', '--repo', root, '--files', 'sophia,a b']]);
    assert.deepEqual([...kept.keys()], ['elohim-sophia:build-sophia-umd', 'elohim:build-angular']);
  });

  test('an empty changed list never spawns and returns an empty map', () => {
    const kept = rakiaAffected(root, [], { rakiaBin: '/bin/rakia', spawn: () => { throw new Error('must not spawn'); } });
    assert.equal(kept.size, 0);
  });
});

describe('resolveRakiaBin', () => {
  test('prefers RAKIA_BIN when it points at an executable file', () => {
    assert.equal(resolveRakiaBin({ RAKIA_BIN: process.execPath }), process.execPath);
  });
  test('returns null when RAKIA_BIN is set but not executable and PATH has no rakia', () => {
    assert.equal(resolveRakiaBin({ RAKIA_BIN: '/nonexistent/rakia', PATH: '/nonexistent' }), null);
  });
});

describe('rakiaAffected — bounded by a timeout', () => {
  test('the spawn carries a timeout and a timed-out oracle falls back', () => {
    let opts;
    const spawn = (bin, args, o) => { opts = o; return { status: null, stdout: '', stderr: '', error: Object.assign(new Error('ETIMEDOUT'), { code: 'ETIMEDOUT' }) }; };
    assert.equal(rakiaAffected('/repo', ['sophia'], { rakiaBin: '/bin/rakia', spawn }), null);
    assert.ok(opts.timeout >= 30000, 'rakia spawn carries a timeout');
  });
});
