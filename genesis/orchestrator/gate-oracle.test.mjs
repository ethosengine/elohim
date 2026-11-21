import { describe, test } from 'node:test';
import { strict as assert } from 'node:assert';
import { chmodSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { britAffected, depthOne, ORACLE_TIMEOUT_MS, resolveBritBin } from './gate-oracle.mjs';

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

describe('britAffected — the binary is optional and its failure is a fallback, never a crash', () => {
  const root = '/repo';
  const reasons = (spawn, britBin = '/bin/brit', files = ['sophia']) => {
    const seen = [];
    const result = britAffected(root, files, { britBin, spawn, onUnavailable: r => seen.push(r) });
    return { result, seen };
  };

  test('returns null with the no-executable reason when no binary resolves', () => {
    const { result, seen } = reasons(() => { throw new Error('must not spawn'); }, null);
    assert.equal(result, null);
    assert.deepEqual(seen, ['no brit executable (BRIT_BIN or PATH)']);
  });

  test('a non-zero exit names the status and the first non-empty stderr line', () => {
    const { result, seen } = reasons(() => ({ status: 2, stdout: '', stderr: '\n  manifest error: bad step \nsecond line\n' }));
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected exited 2: manifest error: bad step']);
  });

  test('a long stderr line is trimmed to 160 characters', () => {
    const { seen } = reasons(() => ({ status: 1, stdout: '', stderr: 'x'.repeat(500) }));
    assert.equal(seen[0], `brit build affected exited 1: ${'x'.repeat(160)}`);
  });

  test('a timeout (ETIMEDOUT error) reports the timeout', () => {
    const { result, seen } = reasons(() => ({ status: null, stdout: '', stderr: '', error: Object.assign(new Error('spawnSync ETIMEDOUT'), { code: 'ETIMEDOUT' }) }));
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected timed out after 60 s']);
  });

  test('a child ended by a signal with no timeout names the signal, not a timeout', () => {
    // The workspace RAM guard sheds with SIGTERM; spawnSync reports a timeout as an ETIMEDOUT error.
    const { result, seen } = reasons(() => ({ status: null, signal: 'SIGTERM', stdout: '', stderr: '' }));
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected was ended by SIGTERM']);
  });

  test('a spawn that never ran names its error code', () => {
    const { result, seen } = reasons(() => ({ status: null, stdout: '', stderr: '', error: Object.assign(new Error('spawnSync brit ENOENT'), { code: 'ENOENT' }) }));
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected could not run: ENOENT']);
  });

  test('a spawn that returns nothing reports exited without a status', () => {
    const { result, seen } = reasons(() => undefined);
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected exited without a status']);
  });

  test('an error with no code reports its message', () => {
    const { result, seen } = reasons(() => ({ status: null, stdout: '', stderr: '', error: new Error('spawn blew up') }));
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected could not run: spawn blew up']);
  });

  test('stdout that is not JSON reports no affected list', () => {
    const { result, seen } = reasons(() => ({ status: 0, stdout: 'not json', stderr: '' }));
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected printed no affected list']);
  });

  test('JSON without an affected array reports no affected list', () => {
    const { result, seen } = reasons(() => ({ status: 0, stdout: '{"changed_paths":[]}', stderr: '' }));
    assert.equal(result, null);
    assert.deepEqual(seen, ['brit build affected printed no affected list']);
  });

  test('a zero exit with a warning on stderr still succeeds and reports nothing', () => {
    const { result, seen } = reasons(() => ({ status: 0, stdout: '{"affected":[]}', stderr: 'warning: something\n' }));
    assert.equal(result.size, 0);
    assert.deepEqual(seen, []);
  });

  test('onUnavailable is optional', () => {
    assert.equal(britAffected(root, ['sophia'], { britBin: null }), null);
  });

  test('passes build affected --repo --files as one comma-joined argument, and parses the affected list', () => {
    let seen;
    const spawn = (bin, args) => {
      seen = [bin, args];
      return { status: 0, stdout: JSON.stringify({ changed_paths: ['sophia'], affected: [direct('elohim-sophia:build-sophia-umd', 'sophia'), viaUp('elohim:build-angular', 'elohim-sophia:build-sophia-umd')] }), stderr: '' };
    };
    const kept = britAffected(root, ['sophia', 'a b'], { britBin: '/bin/brit', spawn });
    assert.deepEqual(seen, ['/bin/brit', ['build', 'affected', '--repo', root, '--files', 'sophia,a b']]);
    assert.deepEqual([...kept.keys()], ['elohim-sophia:build-sophia-umd', 'elohim:build-angular']);
  });

  test('an empty changed list never spawns and returns an empty map', () => {
    const kept = britAffected(root, [], { britBin: '/bin/brit', spawn: () => { throw new Error('must not spawn'); } });
    assert.equal(kept.size, 0);
  });

  test('the spawn carries the timeout', () => {
    let opts;
    britAffected(root, ['sophia'], { britBin: '/bin/brit', spawn: (b, a, o) => { opts = o; return { status: 0, stdout: '{"affected":[]}' }; } });
    assert.equal(opts.timeout, ORACLE_TIMEOUT_MS);
    assert.equal(ORACLE_TIMEOUT_MS, 60_000);
  });
});

describe('resolveBritBin', () => {
  const fakeBin = (t, name) => {
    const dir = mkdtempSync(join(tmpdir(), 'gate-oracle-'));
    t.after(() => rmSync(dir, { recursive: true, force: true }));
    writeFileSync(join(dir, name), '#!/bin/sh\nexit 0\n');
    chmodSync(join(dir, name), 0o755);
    return dir;
  };
  test('prefers BRIT_BIN when it points at an executable file', () => {
    assert.equal(resolveBritBin({ BRIT_BIN: process.execPath }), process.execPath);
  });
  test('returns null when BRIT_BIN is set but not executable, even with a brit on PATH', t => {
    const dir = fakeBin(t, 'brit');
    assert.equal(resolveBritBin({ BRIT_BIN: '/nonexistent/brit', PATH: dir }), null);
  });
  test('finds the first executable brit on PATH', t => {
    const dir = fakeBin(t, 'brit');
    assert.equal(resolveBritBin({ PATH: `/nonexistent${':'}${dir}` }), join(dir, 'brit'));
  });
  test('a rakia on PATH is not an oracle', t => {
    const dir = fakeBin(t, 'rakia');
    assert.equal(resolveBritBin({ PATH: dir }), null);
  });
  test('the spawn receives exactly the brit argv when only a fake brit is on PATH', t => {
    const dir = fakeBin(t, 'brit');
    const britBin = resolveBritBin({ PATH: dir });
    let args;
    britAffected('/r', ['a', 'b'], { britBin, spawn: (bin, a) => { args = a; return { status: 0, stdout: '{"affected":[]}' }; } });
    assert.deepEqual(args, ['build', 'affected', '--repo', '/r', '--files', 'a,b']);
  });
});
