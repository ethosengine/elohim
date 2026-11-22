import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { affected, impact, main, localSeconds } from './gate-impact.mjs';

const manifests = [{ content: { pipeline: 'p', steps: { a: { depends: [] }, b: { depends: ['a'] }, c: { depends: ['b'] } },
  gate: { projects: { a: { dir: 'a', steps: ['a'] }, b: { dir: 'b', steps: ['b'] }, c: { dir: 'c', steps: ['c'] } } } } }];
const direct = name => ({ qualified_name: `p:${name}`, affected_by: [{ kind: 'changedFile', path: `${name}/file` }] });
const via = (name, upstream) => ({ qualified_name: `p:${name}`, affected_by: [{ kind: 'upstreamNode', upstream: `p:${upstream}` }] });
function query(steps) {
  return affected(['a/file'], { root: tmpdir(), britBin: 'fake-brit', manifests,
    spawn: (bin, args, opts) => {
      assert.equal(bin, 'fake-brit');
      assert.deepEqual(args, ['build', 'affected', '--repo', tmpdir(), '--files', 'a/file']);
      assert.equal(opts.timeout, 15000);
      return { status: 0, stdout: JSON.stringify({ affected: steps }) };
    } });
}
test('shared direct project with seconds snapshot', () => {
  assert.deepEqual(impact(query([direct('a')]), query([direct('a')]), { a: 23 }),
    [{ project: 'a', direction: 'shared', seconds: 23 }]);
});
test('upstream-of-me and complete multi-hop reasons', () => {
  const steps = [direct('a'), via('b', 'a'), via('c', 'b')];
  const mine = query(steps);
  assert.deepEqual(mine.steps, steps);
  assert.deepEqual(impact(mine, query([direct('c')])),
    [{ project: 'c', direction: 'upstream-of-me', seconds: null }]);
});
test('downstream-of-me', () => {
  assert.deepEqual(impact(query([direct('b')]), query([direct('a'), via('b', 'a')])),
    [{ project: 'b', direction: 'downstream-of-me', seconds: null }]);
});
test('disjoint and shared indirect consumers convey no direction', () => {
  assert.deepEqual(impact(query([direct('a')]), query([direct('b')])), []);
  assert.deepEqual(impact(query([via('c', 'a')]), query([via('c', 'b')])), []);
});
for (const [label, result, expected] of [
  ['brit missing', { error: { code: 'ENOENT' } }, 'ENOENT'],
  ['timeout', { error: { code: 'ETIMEDOUT' } }, 'timeout after 15 s'],
  ['nonzero', { status: 1, stderr: 'broken' }, 'exited 1'],
  ['invalid JSON', { status: 0, stdout: 'bad' }, 'path-fallback'],
]) test(`${label} -> path-fallback exit 0`, () => {
  const lines = [];
  assert.equal(main(['--mine', 'a', '--theirs', 'b'], {
    root: tmpdir(), britBin: 'fake', spawn: () => result,
  }, line => lines.push(line)), 0);
  assert.match(lines[0], /^path-fallback: /);
  assert.ok(lines[0].includes(expected));
});
test('missing binary resolution and names-only output remain unknown', () => {
  assert.match(affected(['a'], { britBin: null }).fallback, /no brit/);
  const value = affected(['a'], { britBin: 'fake', spawn: () => ({ status: 0,
    stdout: JSON.stringify({ affected: ['p:a'] }) }) });
  assert.deepEqual(value.steps, ['p:a']);
  assert.match(value.fallback, /reasons unavailable/);
});
test('directory and glob expansion capped at 2000 tracked files', t => {
  const root = mkdtempSync(join(tmpdir(), 'impact-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, 'src'));
  for (const pattern of ['src', 'src/**']) {
    const calls = [];
    const value = affected([pattern], { root, britBin: 'fake', manifests, spawn: (bin, args) => {
      calls.push([bin, args]);
      return { status: 0, stdout: bin === 'git' ? Array.from({ length: 2001 }, (_, i) => `src/${i}`).join('\0') + '\0'
        : '{"affected":[]}' };
    } });
    assert.equal(value.truncated, true);
    assert.deepEqual(calls[0], ['git', ['ls-files', '-z', '--', pattern]]);
    assert.equal(calls[1][1].at(-1).split(',').length, 2000);
  }
});
test('last local gate seconds use existing row parser, missing history is unknown', t => {
  const root = mkdtempSync(join(tmpdir(), 'impact-seconds-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const projects = [{ name: 'a', dir: 'a' }];
  assert.deepEqual(localSeconds(root, projects), {});
  mkdirSync(join(root, '.eprfs/status'), { recursive: true });
  writeFileSync(join(root, '.eprfs/status/flows.jsonl'), [12, 19].map(value => JSON.stringify({ record: {
    occurredAt: '2026-11-22T00:00:00Z', classifiedAs: ['run:observation', 'a/justfile',
      'measure:gate-cycle-seconds@1', `value:${value}`, 'env:run-kind=full-gate', 'env:exit=ok', 'env:project=a'],
  } })).join('\n'));
  assert.deepEqual(localSeconds(root, projects), { a: 19 });
});
test('CLI JSON and empty intersection', () => {
  const options = { root: tmpdir(), britBin: 'fake', manifests,
    spawn: () => ({ status: 0, stdout: '{"affected":[]}' }) };
  const lines = [];
  assert.equal(main(['--mine', 'a', '--theirs', 'b'], options, x => lines.push(x)), 0);
  assert.deepEqual(lines, ['no shared gates']);
  main(['--mine', 'a', '--theirs', 'b', '--json'], options, x => lines.push(x));
  assert.deepEqual(JSON.parse(lines[1]), { rows: [], fallback: null, truncated: false });
});

test('gate-only build-process inputs directly select their project', () => {
  const local = [{ content: { pipeline: 'p', steps: {}, gate: { projects: {
    policy: { dir: '.', inputs: { buildProcess: ['policy.json'] } },
  } } } }];
  const result = affected(['policy.json'], { root: tmpdir(), britBin: 'fake', manifests: local,
    spawn: () => ({ status: 0, stdout: '{"affected":[]}' }) });
  assert.equal(result.projects[0].direct, true);
  assert.deepEqual(impact(result, result), [{ project: 'policy', direction: 'shared', seconds: null }]);
});
