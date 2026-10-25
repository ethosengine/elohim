/**
 * Gate cycle time — the gate tells you when it has become too slow to live with.
 *
 * `just gate elohim-storage` took ~33 minutes on 2026-09-19 and nobody was told;
 * a person noticed by waiting. The gate runner is the one place every gate runs
 * (humans and pre-push), so it times each project, records the duration as an
 * observation of the declared measure `gate-cycle-seconds@1`, reads the ceiling
 * DECLARED for it in measures.yaml, and — over the hard watermark — files a
 * fingerprinted architecture finding and prints a dispatch directive for a
 * reviewer whose single question is whether this can be modularized.
 *
 * Everything here is fail-open: a gate's verdict never depends on this module.
 *
 * Run:
 *   node --test genesis/orchestrator/gate-cycle.test.mjs
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  GATE_CYCLE_MEASURE,
  observationArgs,
  subjectFor,
  runKind,
  classifyFailure,
  readCeiling,
  judgeCycle,
  fingerprint,
  recordCycle,
} from './gate-cycle.mjs';
import { readCycleRows } from './gate-cost.mjs';

const project = { name: 'elohim-storage', dir: 'elohim/elohim-storage' };

const LENSES = `
measures:
  - id: gate-cycle-seconds
    version: 1
lenses:
  - id: gate-cycle-full-ceiling
    version: 1
    class: measure
    consumes: [gate-cycle-seconds@1]
    env: { run-kind: full-gate, exit: ok }
    soft: 600
    hard: 1200
    concern: dev-system-equilibrium
    dispatch-agent: rust-architect
    dispatch-prompt: "Does this make sense? Can this be modularized?"
  - id: gate-cycle-full-ceiling-doorway
    version: 1
    class: measure
    consumes: [gate-cycle-seconds@1]
    subject: doorway/doorway-service
    env: { run-kind: full-gate, exit: ok }
    soft: 300
    hard: 900
    dispatch-agent: rust-architect
    dispatch-prompt: "Does this make sense? Can this be modularized?"
`;

function sandbox() {
  const dir = mkdtempSync(join(tmpdir(), 'gate-cycle-'));
  const measures = join(dir, 'measures.yaml');
  writeFileSync(measures, LENSES);
  return { dir, measures, ledger: join(dir, 'architecture-findings.jsonl') };
}

const failures = [
  ['leaked toolchain in a WASM gate', 2, { preflight: 'refused', recipeStarted: false }, 'environment'],
  ['missing wasm32 target', 2, { preflight: 'refused', recipeStarted: false }, 'environment'],
  ['missing storage-client dist', 2, { preflight: 'refused', recipeStarted: false }, 'environment'],
  ['session-envelope assertion failure', 1, { recipeStarted: true, assertionFailed: true, testOutput: true }, 'code'],
  ['143 shed before test output', 143, { recipeStarted: true, testOutput: false }, 'environment'],
  ['SIGTERM shed before test output', null, { signal: 'SIGTERM', testOutput: false }, 'environment'],
  ['signal 15 shed before test output', null, { signal: 15, testOutput: false }, 'environment'],
  ['compiler diagnostic', 101, { recipeStarted: true, compilerError: true }, 'code'],
  ['unexplained exit 1', 1, {}, 'unknown'],
  ['unexplained exit 2', 2, {}, 'unknown'],
  ['unexplained cargo exit 101', 101, {}, 'unknown'],
  ['143 with unavailable output', 143, {}, 'unknown'],
  ['143 after test output', 143, { recipeStarted: true, testOutput: true }, 'unknown'],
  ['143 after an assertion failure', 143, { recipeStarted: true, assertionFailed: true, testOutput: true }, 'code'],
  ['failure without a started recipe', 1, { recipeStarted: false, assertionFailed: true }, 'unknown'],
  ['successful run', 0, { preflight: 'refused', assertionFailed: true }, undefined],
];

for (const [name, status, signals, expected] of failures) {
  test(`failure classification: ${name}`, () => {
    const before = structuredClone(signals);
    assert.equal(classifyFailure(status, signals), expected);
    assert.deepEqual(signals, before, 'classification does not mutate its evidence');
  });
}

test('recordCycle carries failure evidence through env fields, omitting it on success', () => {
  const { dir, measures, ledger } = sandbox();
  try {
    for (const [name, status, signals, expected] of failures) {
      const notes = [];
      recordCycle(project, 1, status, {}, {
        root: dir, measuresPath: measures, ledgerPath: ledger,
        runEpr: args => notes.push(args), print: () => {}, now: () => 't',
      }, signals);
      assert.equal(notes.length, 1, name);
      const args = notes[0];
      assert.deepEqual(args.filter(arg => arg.startsWith('failure_class=')),
        expected ? [`failure_class=${expected}`] : [], name);
      if (expected) assert.equal(args[args.indexOf(`failure_class=${expected}`) - 1], '--env', name);
      assert.deepEqual(args.slice(-2), ['--env', 'project=elohim-storage']);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('gate-cost accepts legacy rows and rows with each failure class', () => {
  const record = {
    occurredAt: '2026-10-25T18:00:00Z',
    classifiedAs: ['run:observation', '.', `measure:${GATE_CYCLE_MEASURE}`,
      'value:1', 'env:run-kind=full-gate', 'env:exit=fail', 'env:project=elohim-storage'],
  };
  const data = [undefined, 'environment', 'code', 'unknown'].map(failureClass => JSON.stringify({
    record: { ...record, classifiedAs: [...record.classifiedAs,
      ...(failureClass ? [`env:failure_class=${failureClass}`] : [])] },
  })).join('\n');
  const { rows, skipped } = readCycleRows('fixture', () => data);
  assert.equal(skipped, 0);
  assert.equal(rows.length, 4);
  for (const row of rows) assert.deepEqual(row, rows[0]);
});

test('the observation names the measure, the subject, the run kind, and the project last', () => {
  const args = observationArgs('elohim/elohim-storage/justfile', 1980.4, { kind: 'full-gate', exit: 'ok', jobs: '1', project: project.name });
  assert.deepEqual(args, [
    'flow', 'note', '--kind', 'observation',
    '--measure', GATE_CYCLE_MEASURE,
    '--subject', 'elohim/elohim-storage/justfile',
    '--value', '1980',
    '--unit', 'seconds',
    '--env', 'run-kind=full-gate',
    '--env', 'exit=ok',
    '--env', 'jobs=1',
    '--env', 'project=elohim-storage',
  ]);
});

test('the project env slot is last even when jobs is absent', () => {
  const args = observationArgs('.', 1, { kind: 'pre-push', exit: 'ok', project: 'rakia-validate' });
  assert.deepEqual(args.slice(-4), ['--env', 'exit=ok', '--env', 'project=rakia-validate']);
});

test('successful observations omit even a supplied failure class', () => {
  const args = observationArgs('.', 1, { kind: 'full-gate', exit: 'ok', failureClass: 'code' });
  assert.ok(!args.some(arg => arg.startsWith('failure_class=')));
});

test('the subject is a FILE — the gate recipe itself — because the sidecar addresses a subject by its bytes', () => {
  const dir = mkdtempSync(join(tmpdir(), 'gate-cycle-subject-'));
  try {
    mkdirSync(join(dir, 'a'), { recursive: true });
    mkdirSync(join(dir, 'b'), { recursive: true });
    mkdirSync(join(dir, 'c'), { recursive: true });
    writeFileSync(join(dir, 'a/justfile'), 'gate:\n');
    writeFileSync(join(dir, 'a/Cargo.toml'), '');
    writeFileSync(join(dir, 'b/package.json'), '{}');
    assert.equal(subjectFor({ dir: 'a' }, dir), 'a/justfile', 'the recipe that was timed');
    assert.equal(subjectFor({ dir: 'b' }, dir), 'b/package.json');
    assert.equal(subjectFor({ dir: 'c' }, dir), '.', 'nothing to anchor on: the repository itself');
    assert.equal(subjectFor({ dir: '.' }, dir), '.');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('the run kind is declared by the caller, never inferred from hook side effects', () => {
  assert.equal(runKind({}), 'full-gate');
  assert.equal(runKind({ GATE_RUN_KIND: 'pre-push' }), 'pre-push');
  assert.equal(runKind({ GIT_DIR: '/x/.git' }), 'full-gate', 'a hook pin is not a declaration');
});

test('a subject-scoped ceiling wins over the default for that project and run kind', () => {
  const { dir, measures } = sandbox();
  try {
    const general = readCeiling(measures, project.dir, 'full-gate');
    assert.equal(general.id, 'gate-cycle-full-ceiling@1');
    assert.equal(general.hard, 1200);
    const scoped = readCeiling(measures, 'doorway/doorway-service', 'full-gate');
    assert.equal(scoped.id, 'gate-cycle-full-ceiling-doorway@1');
    assert.equal(scoped.hard, 900);
    assert.equal(readCeiling(measures, project.dir, 'pre-push'), null, 'no ceiling declared for that run kind');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('the verdict comes from the declared watermarks, not from numbers in this file', () => {
  const ceiling = { id: 'x@1', soft: 600, hard: 1200 };
  assert.equal(judgeCycle(1980, ceiling), 'hard');
  assert.equal(judgeCycle(1200, ceiling), 'hard', 'at the watermark counts, as the registry compares at-or-above');
  assert.equal(judgeCycle(700, ceiling), 'soft');
  assert.equal(judgeCycle(90, ceiling), 'ok');
  assert.equal(judgeCycle(9999, null), 'ok', 'no declared ceiling, no opinion');
});

test('over the hard ceiling: one finding, one directive — and silence the second time', () => {
  const { dir, measures, ledger } = sandbox();
  const lines = [];
  const notes = [];
  const deps = {
    measuresPath: measures,
    ledgerPath: ledger,
    root: dir,
    runEpr: args => { notes.push(args); return 0; },
    print: line => lines.push(line),
    now: () => '2026-09-19T18:00:00Z',
  };
  try {
    recordCycle(project, 1980, 0, { CARGO_BUILD_JOBS: '1' }, deps);
    assert.equal(notes.length, 1, 'the duration is recorded as an observation');
    assert.equal(notes[0][notes[0].indexOf('--subject') + 1], '.', 'no anchor file in this sandbox');
    assert.deepEqual(notes[0].slice(-2), ['--env', 'project=elohim-storage']);
    const rows = readFileSync(ledger, 'utf8').trim().split('\n').map(l => JSON.parse(l));
    assert.equal(rows.length, 1);
    assert.equal(rows[0].fp, fingerprint('gate-cycle-full-ceiling@1', project.dir, 'full-gate'));
    assert.equal(rows[0].path, project.dir);
    assert.equal(rows[0].status, 'open');
    assert.equal(rows[0].stock, 1980);
    assert.equal(rows[0].limit, 1200);
    assert.equal(rows[0].concern, 'dev-system-equilibrium');
    const directive = lines.join('\n');
    assert.match(directive, /DISPATCH/);
    assert.match(directive, /rust-architect/);
    assert.match(directive, /Can this be modularized\?/);

    lines.length = 0;
    recordCycle(project, 2050, 0, {}, deps);
    assert.equal(readFileSync(ledger, 'utf8').trim().split('\n').length, 1, 'same fingerprint, no second row');
    assert.doesNotMatch(lines.join('\n'), /DISPATCH/, 'an open finding is not re-dispatched');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a finding closes itself after two consecutive runs back under the ceiling', () => {
  const { dir, measures, ledger } = sandbox();
  const deps = { measuresPath: measures, ledgerPath: ledger, runEpr: () => 0, print: () => {}, now: () => 't' };
  try {
    recordCycle(project, 1980, 0, {}, deps);
    recordCycle(project, 300, 0, {}, deps);
    let row = JSON.parse(readFileSync(ledger, 'utf8').trim());
    assert.equal(row.status, 'open', 'one good run may be a warm cache');
    assert.equal(row.under_streak, 1);
    recordCycle(project, 310, 0, {}, deps);
    row = JSON.parse(readFileSync(ledger, 'utf8').trim());
    assert.equal(row.status, 'closed');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a failed gate is recorded but never judged — it stopped early, its duration means nothing', () => {
  const { dir, measures, ledger } = sandbox();
  const notes = [];
  const deps = { measuresPath: measures, ledgerPath: ledger, runEpr: a => { notes.push(a); return 0; }, print: () => {}, now: () => 't' };
  try {
    recordCycle(project, 5000, 1, {}, deps);
    assert.ok(notes[0].includes('exit=fail'));
    assert.ok(notes[0].includes('failure_class=unknown'));
    assert.deepEqual(notes[0].slice(-2), ['--env', 'project=elohim-storage']);
    assert.equal(existsSync(ledger), false);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('nothing here can fail a gate', () => {
  const { dir, ledger } = sandbox();
  try {
    assert.doesNotThrow(() => recordCycle(project, 1980, 0, {}, {
      measuresPath: join(dir, 'missing.yaml'),
      ledgerPath: ledger,
      runEpr: () => { throw new Error('epr not installed'); },
      print: () => {},
      now: () => 't',
    }));
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});
