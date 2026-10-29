/**
 * Gate cost — expected duration and local failures come only from full runs.
 *
 * Run:
 *   node --test genesis/orchestrator/gate-cost.test.mjs genesis/orchestrator/gate-cycle.test.mjs
 */
import { test } from 'node:test';
import { mkdtempSync, mkdirSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
import assert from 'node:assert/strict';
import {
  CI_BRANCH_PATTERN,
  readCycleRows,
  readCycleUnion,
  costOf,
  formatExpectation,
  deferDecision,
} from './gate-cost.mjs';

const project = { name: 'eprfs', dir: 'elohim/eprfs' };
const subject = 'elohim/eprfs/justfile';
const scope = { project, subjectFallback: subject };

function row(index, overrides = {}) {
  return {
    value: index * 60,
    kind: 'full-gate',
    exit: 'ok',
    project: project.name,
    subject,
    occurredAt: new Date(Date.UTC(2026, 9, 1, 0, index)).toISOString(),
    ...overrides,
  };
}

test('mean, nearest-rank p90 and max use the latest ten of twelve successful runs', () => {
  const rows = Array.from({ length: 12 }, (_, i) => row(i + 1));
  const before = structuredClone(rows);
  assert.deepEqual(costOf(rows, scope), { runs: 10, mean: 450, p90: 660, max: 720, fails: 0, window: 12 });
  assert.deepEqual(rows, before, 'folding does not reorder the caller rows');
});

test('reused, deferred and other run kinds affect neither duration nor failure windows', () => {
  const rows = [row(1), row(2, { kind: 'pre-push' })];
  for (let i = 3; i <= 30; i++) {
    rows.push(row(i, { kind: ['reused', 'deferred', 'focused-test'][i % 3], exit: i % 2 ? 'ok' : 'fail' }));
  }
  assert.deepEqual(costOf(rows, scope), { runs: 2, mean: 90, p90: 120, max: 120, fails: 0, window: 2 });
});

test('local failures use the last twenty full runs, including failures and both counted run kinds', () => {
  const rows = Array.from({ length: 25 }, (_, i) => row(i + 1, {
    kind: i % 2 ? 'full-gate' : 'pre-push',
    exit: i < 5 || i === 9 || i === 24 ? 'fail' : 'ok',
    value: i < 5 || i === 9 || i === 24 ? 9999 : 60,
  }));
  assert.deepEqual(costOf(rows, scope), { runs: 10, mean: 60, p90: 60, max: 60, fails: 2, window: 20 });
});

for (const [name, minute] of [
  ['descending', i => 25 - i],
  ['repeated', i => i % 3],
]) {
  test(`duration and failure windows follow append order with ${name} author dates`, () => {
    const rows = Array.from({ length: 25 }, (_, i) => row(i + 1, {
      kind: i % 2 ? 'full-gate' : 'pre-push',
      exit: i < 5 || i === 9 || i === 24 ? 'fail' : 'ok',
      occurredAt: new Date(Date.UTC(2026, 9, 1, 0, minute(i))).toISOString(),
    }));
    const before = structuredClone(rows);
    assert.deepEqual(costOf(rows, scope), { runs: 10, mean: 1170, p90: 1380, max: 1440, fails: 2, window: 20 });
    assert.deepEqual(rows, before, 'folding does not reorder the caller rows');
  });
}

test('the project env slot is authoritative even when subjects differ or collide', () => {
  const rows = [
    row(1, { subject: 'some/old/recipe' }),
    row(2, { project: 'different-project' }),
    row(3, { project: undefined, subject: 'another/recipe' }),
  ];
  assert.deepEqual(costOf(rows, scope), { runs: 1, mean: 60, p90: 60, max: 60, fails: 0, window: 1 });
});

test('legacy rows count when their subject matches an unshared fallback', () => {
  const rows = [row(1, { project: undefined }), row(2), row(3, { project: 'other' })];
  assert.deepEqual(costOf(rows, scope), { runs: 2, mean: 90, p90: 120, max: 120, fails: 0, window: 2 });
});

test('a null fallback refuses ambiguous legacy rows for shared recipes and the root subject', () => {
  for (const shared of ['elohim/elohim-storage/justfile', '.']) {
    const rows = [row(1, { project: undefined, subject: shared }), row(2, { subject: shared })];
    assert.deepEqual(costOf(rows, { project, subjectFallback: null }),
      { runs: 1, mean: 120, p90: 120, max: 120, fails: 0, window: 1 });
  }
});

test('no successful runs leaves duration statistics null while retaining observed failures', () => {
  assert.deepEqual(costOf([], scope), { runs: 0, mean: null, p90: null, max: null, fails: 0, window: 0 });
  assert.deepEqual(costOf([row(1, { exit: 'fail' })], scope),
    { runs: 0, mean: null, p90: null, max: null, fails: 1, window: 1 });
});

test('expectations reuse the gate cycle minute formatting', () => {
  assert.equal(formatExpectation('eprfs', { runs: 10, mean: 840, p90: 1320, max: 1860, fails: 0, window: 20 }),
    '[gate] eprfs: expect ~14 min (p90 22 min, max 31 min; 10 runs) · failed locally 0 of last 20');
  assert.equal(formatExpectation('eprfs', costOf([], scope)), '[gate] eprfs: no recorded runs yet');
});

test('readCycleRows reads two real-shaped rows and counts one malformed line', () => {
  const records = [
    {
      cid: 'first-observation',
      record: {
        kind: 'event', action: 'cite', occurredAt: '2026-10-03T11:37:54Z',
        quantity: { Count: { value: 1, unit: 'run-note' } },
        classifiedAs: ['run:observation', 'genesis/a2o/package.json',
          'reason:gate-cycle-seconds@1 = 191 seconds', 'measure:gate-cycle-seconds@1',
          'value:191', 'unit:seconds', 'env:exit=fail', 'env:jobs=4', 'env:run-kind=pre-push'],
      },
    },
    {
      cid: 'second-observation',
      record: {
        kind: 'event', action: 'cite', occurredAt: '2026-10-03T11:38:54Z',
        quantity: { Count: { value: 1, unit: 'run-note' } },
        classifiedAs: ['run:observation', 'genesis/seeder/justfile',
          'reason:gate-cycle-seconds@1 = 35 seconds', 'measure:gate-cycle-seconds@1',
          'value:35', 'unit:seconds', 'env:exit=ok', 'env:jobs=4', 'env:run-kind=full-gate', 'env:project=seeder'],
      },
    },
  ];
  const data = `${JSON.stringify(records[0])}\nnot json\n${JSON.stringify(records[1])}\n`;
  const result = readCycleRows('/fixture/flows.jsonl', (path, encoding) => {
    assert.equal(path, '/fixture/flows.jsonl');
    assert.equal(encoding, 'utf8');
    return data;
  });
  assert.deepEqual(result, {
    rows: [
      { value: 191, kind: 'pre-push', exit: 'fail', project: undefined,
        subject: 'genesis/a2o/package.json', occurredAt: '2026-10-03T11:37:54Z' },
      { value: 35, kind: 'full-gate', exit: 'ok', project: 'seeder',
        subject: 'genesis/seeder/justfile', occurredAt: '2026-10-03T11:38:54Z' },
    ],
    skipped: 1,
  });
});

test('malformed cycle fields are counted; unrelated measures and blank lines are ignored', () => {
  const record = {
    occurredAt: '2026-10-03T11:37:54Z',
    classifiedAs: ['run:observation', subject, 'measure:gate-cycle-seconds@1',
      'value:0', 'env:run-kind=full-gate', 'env:exit=ok'],
  };
  const badValues = ['', 'NaN', 'Infinity', '-1'].map(value => ({
    ...record, classifiedAs: record.classifiedAs.map(tag => tag === 'value:0' ? `value:${value}` : tag),
  }));
  const records = [
    record,
    ...badValues,
    { ...record, occurredAt: 'invalid-date' },
    { ...record, classifiedAs: record.classifiedAs.filter(tag => tag !== 'env:exit=ok') },
    { ...record, classifiedAs: [...record.classifiedAs, 'env:project='] },
    { classifiedAs: ['measure:another-measure@1'] },
    { classifiedAs: ['measure:gate-cycle-seconds@10'] },
  ];
  const data = records.map(record => JSON.stringify({ record })).join('\n') + '\n \nnull\n';
  const result = readCycleRows('fixture', () => data);
  assert.equal(result.skipped, 8);
  assert.equal(result.rows.length, 1);
  assert.equal(result.rows[0].value, 0, 'a zero-second run is valid');
});

test('the CI pattern matches the full branch names declared in Jenkins', () => {
  for (const branch of ['dev', 'dev-gate-reuse', 'feat-cost', 'claude/gate-cost']) {
    assert.equal(CI_BRANCH_PATTERN.test(branch), true, branch);
  }
  for (const branch of ['main', 'feature/cost', 'dev-', 'feat-', 'claude/', 'other-feat-cost', 'refs/heads/dev-cost']) {
    assert.equal(CI_BRANCH_PATTERN.test(branch), false, branch);
  }
});

const eligible = {
  cost: { runs: 10, mean: 840, p90: 1320, max: 1860, fails: 0, window: 20 },
  targets: ['refs/heads/feat-cost'],
  branches: ['feat-cost'],
  ciBranchPattern: CI_BRANCH_PATTERN,
  env: {},
};

for (const [name, overrides, why] of [
  ['dev target', { targets: ['refs/heads/feat-cost', 'refs/heads/dev'] }, /refs\/heads\/dev/],
  ['main target', { targets: ['refs/heads/feat-cost', 'refs/heads/main'] }, /refs\/heads\/main/],
  ['non-CI branch', { branches: ['feat-cost', 'topic/cost'] }, /topic\/cost.*CI branch pattern/],
  ['fewer than ten successful runs', { cost: { ...eligible.cost, runs: 9 } }, /9 successful runs; need 10/],
  ['p90 below the soft ceiling', { cost: { ...eligible.cost, p90: 599 } }, /p90 599 s.*600 s soft ceiling/],
  ['a recent failure', { cost: { ...eligible.cost, fails: 1 } }, /failed locally 1 of last 20/],
  ['GATE_DEFER=off', { env: { GATE_DEFER: 'off' } }, /GATE_DEFER=off/],
  ['FORCE_HEAVY_GATES=1', { env: { FORCE_HEAVY_GATES: '1' } }, /FORCE_HEAVY_GATES=1/],
]) {
  test(`deferral refuses ${name}`, () => {
    const decision = deferDecision({ ...eligible, ...overrides });
    assert.equal(decision.defer, false);
    assert.match(decision.why, why);
  });
}

test('a sufficiently sampled, slow gate with no local failures can defer to feature-branch CI', () => {
  const before = structuredClone(eligible);
  const decision = deferDecision(eligible);
  assert.equal(decision.defer, true);
  assert.match(decision.why, /feature-branch CI.*p90 22 min.*10 successful runs.*0 of last 20/);
  assert.deepEqual(eligible, before, 'the decision does not mutate its inputs');
  assert.equal(deferDecision({ ...eligible, cost: { ...eligible.cost, p90: 600 } }).defer, true,
    'at the soft ceiling counts');
});

test('refusal reports the first failing condition', () => {
  const input = { ...eligible, env: { GATE_DEFER: 'off', FORCE_HEAVY_GATES: '1' },
    targets: ['refs/heads/dev'], branches: ['topic/cost'], cost: { ...eligible.cost, runs: 0, p90: null, fails: 2 } };
  assert.equal(deferDecision(input).why, 'GATE_DEFER=off');
  assert.equal(deferDecision({ ...input, env: { FORCE_HEAVY_GATES: '1' } }).why, 'FORCE_HEAVY_GATES=1');
  assert.match(deferDecision({ ...input, env: {} }).why, /refs\/heads\/dev/);
  assert.match(deferDecision({ ...input, env: {}, targets: eligible.targets }).why, /topic\/cost/);
  assert.match(deferDecision({ ...input, env: {}, targets: eligible.targets, branches: eligible.branches }).why, /need 10/);
  assert.match(deferDecision({ ...eligible, cost: { ...eligible.cost, p90: 1, fails: 2 } }).why, /soft ceiling/);
});

test('the caller can supply the soft ceiling and minimum sample count', () => {
  const input = { ...eligible, cost: { ...eligible.cost, runs: 3, p90: 120 }, softSeconds: 120, minRuns: 3 };
  assert.equal(deferDecision(input).defer, true);
  assert.equal(deferDecision({ ...input, minRuns: 4 }).defer, false);
  assert.equal(deferDecision({ ...input, softSeconds: 121 }).defer, false);
});

test('branch matching is pure even for a caller-supplied global expression', () => {
  const ciBranchPattern = /^topic-.+$/g;
  ciBranchPattern.lastIndex = 7;
  const input = { ...eligible, targets: ['refs/heads/topic-one', 'refs/heads/topic-two'],
    branches: ['topic-one', 'topic-two'], ciBranchPattern };
  assert.equal(deferDecision(input).defer, true);
  assert.equal(deferDecision(input).defer, true);
  assert.equal(ciBranchPattern.lastIndex, 7);
});

test('no branch names is insufficient evidence of CI coverage', () => {
  assert.deepEqual(deferDecision({ ...eligible, branches: [] }), { defer: false, why: 'no CI branch supplied' });
});


test('union includes every sibling, deduplicates row CIDs and keeps append rather than author order', t => {
  const root = mkdtempSync(join(tmpdir(), 'gate-union-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const write = (path, text) => { mkdirSync(dirname(path), { recursive: true }); writeFileSync(path, text); };
  const event = (cid, value, occurredAt) => ({ cid, record: { occurredAt,
    classifiedAs: ['run:observation', subject, 'measure:gate-cycle-seconds@1',
      `value:${value}`, 'env:run-kind=full-gate', 'env:exit=ok', 'env:project=eprfs'] } });
  const first = event('first', 10, '2026-10-29T00:00:00Z');
  const second = event('second', 20, '2026-10-28T00:00:00Z');
  const third = event('third', 30, '2026-10-27T00:00:00Z');
  const fourth = event('fourth', 40, '2026-10-26T00:00:00Z');
  const ledger = (at, rows) => write(join(at, '.eprfs/status/flows.jsonl'), rows.map(JSON.stringify).join('\n'));
  ledger(root, [first]);
  ledger(join(root, '.worktrees/alpha'), [first, second]);
  ledger(join(root, '.worktrees/beta'), [third]);
  mkdirSync(join(root, '.worktrees/empty'), { recursive: true });
  const current = join(root, '.worktrees/current');
  ledger(current, [first, fourth]);
  write(join(current, '.git'), 'gitdir: ../../.git/worktrees/current\n');
  write(join(root, '.git/worktrees/current/commondir'), '../..\n');
  const rows = readCycleUnion(current).rows;
  assert.deepEqual(rows.map(row => row.value), [10, 20, 30, 40]);
  assert.deepEqual(readCycleUnion(root).rows, rows);
  assert.equal(readCycleUnion(current).skipped, 0);
  assert.equal(readCycleUnion(current).readableLedgers, 4);
  rmSync(join(root, '.eprfs'), { recursive: true });
  assert.deepEqual(readCycleUnion(current).rows.map(row => row.value), [10, 20, 30, 40]);
  assert.equal(readCycleUnion(current).readableLedgers, 3);
});
