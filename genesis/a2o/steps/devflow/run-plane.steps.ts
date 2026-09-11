/**
 * The run plane — the write leg (`epr flow note`) and the equilibrium verdict (`epr flow stocks`),
 * driven against a scratch repository root exactly the way a human would drive them.
 *
 * Isolation, in the register `steps/devflow/ceremony-reconciliation.steps.ts` established: every
 * scenario mints its OWN git repository under a temp root, projects it, and removes it afterwards.
 * Nothing here reads or writes the real repository's flow store, so these scenarios are safe to
 * run beside live work.
 *
 * Why a git repository at all: a commitment's mint instant is resolved from the history of the
 * artifact it promises, and `epr flow note` / `epr flow fulfill` date their records off the
 * repository's HEAD commit rather than off the clock. Both are therefore arranged by COMMITTING at
 * a declared date — which is also what makes the windows below nameable, and why a replay at one
 * commit is byte-identical (the idempotence scenario).
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

const binary = process.env.EPR_BIN ?? 'epr';
/** Scratch fixtures live here; a harness that owns a scratchpad points this at it. */
const scratchBase = process.env.A2O_TMPDIR ?? tmpdir();

/** The Background's promise is minted a month before any window below opens. */
const BACKGROUND_DATE = '2026-07-01T00:00:00+00:00';
/** Every "gained a claimed work item" mint falls on this day. */
const MINT_DATE = '2026-08-08T00:00:00+00:00';
/** Every discharge falls on this day — the day after the mint, inside the same declared week. */
const DISCHARGE_DATE = '2026-08-09T12:00:00+00:00';
/** The declared week both of those days fall inside. */
const MEASURED_WINDOW = '2026-08-06..2026-08-13';
/** A declared week the fixture's history never touches — nothing to measure inside it. */
const EMPTY_WINDOW = '2026-01-01..2026-01-08';

const CORRECTION_REASON =
  'Measured the level and called it a drain; measure the two rates over the declared window instead.';
const CORRECTION_SWITCHED_TO = 'run the stock fold over a declared window';
/** Well-formed, and named by nothing this sidecar mints. */
const ABSENT_COMMITMENT = 'bafyreiamadw2ymlpvgrh4iox64njzujojpwqtzn55bahczf6iyixdanzoq';

const FEATURES = 'genesis/a2o/features/dataplane/resiliency-saga';
const RECIPES = '.claude/epr-meta/recipes.yaml';

type JsonObject = Record<string, unknown>;

interface Invocation {
  status: number;
  stdout: string;
  stderr: string;
}

interface Reading {
  /** The rendered human output of `epr flow stocks --check`. */
  text: string;
  /** The same fold, taken again with `--json`. */
  json: JsonObject;
  /** The exit status `--check` produced. */
  status: number;
}

interface RunPlaneFixture {
  root: string;
  env: NodeJS.ProcessEnv;
  window: string;
  backgroundCommitment: string;
  /** Commitments minted INSIDE the declared window, oldest first. */
  windowCommitments: string[];
  reading?: Reading;
  note?: JsonObject;
  refusal?: Invocation;
  corrections?: JsonObject[];
  frontierBefore?: number;
  headBefore?: string;
  storeBefore?: string;
  minted: number;
}

const fixtures = new WeakMap<object, RunPlaneFixture>();

function object(value: unknown): JsonObject {
  assert.ok(value && typeof value === 'object' && !Array.isArray(value), 'a JSON object');
  return value as JsonObject;
}

function fixture(world: object): RunPlaneFixture {
  const found = fixtures.get(world);
  assert.ok(found, 'an isolated run-plane fixture must exist — the Background mints it');
  return found;
}

function run(state: RunPlaneFixture, argv: string[], env: NodeJS.ProcessEnv = {}): Invocation {
  const result = spawnSync(argv[0], argv.slice(1), {
    cwd: state.root,
    env: { ...state.env, ...env },
    encoding: 'utf8',
    timeout: 60_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  return { status: result.status ?? -1, stdout: result.stdout ?? '', stderr: result.stderr ?? '' };
}

function ok(state: RunPlaneFixture, argv: string[], env: NodeJS.ProcessEnv = {}): string {
  const result = run(state, argv, env);
  assert.equal(result.status, 0, `${argv.join(' ')}\n${result.stdout}\n${result.stderr}`);
  return result.stdout;
}

function git(state: RunPlaneFixture, when: string, args: string[]): void {
  ok(state, ['git', '-c', 'core.hooksPath=/dev/null', ...args], {
    GIT_AUTHOR_DATE: when,
    GIT_COMMITTER_DATE: when,
  });
}

/** Stage everything and commit it AT a declared date, so every record dated off HEAD is nameable. */
function commitAt(state: RunPlaneFixture, when: string, message: string): void {
  git(state, when, ['add', '-A']);
  git(state, when, ['commit', '-q', '-m', message]);
}

function write(state: RunPlaneFixture, rel: string, contents: string): void {
  const path = join(state.root, rel);
  mkdirSync(dirname(path), { recursive: true });
  writeFileSync(path, contents);
}

function flow(state: RunPlaneFixture, args: string[]): JsonObject {
  return object(JSON.parse(ok(state, [binary, 'flow', ...args, '--root', state.root, '--json'])));
}

function project(state: RunPlaneFixture): void {
  ok(state, [
    binary,
    'flow',
    'project',
    '--root',
    state.root,
    '--recipes',
    join(state.root, RECIPES),
  ]);
}

function storeBytes(state: RunPlaneFixture): string {
  return readFileSync(join(state.root, '.eprfs/status/flows.jsonl'), 'utf8');
}

/** Every record in the append-only store, newest last. */
function records(state: RunPlaneFixture): JsonObject[] {
  return storeBytes(state)
    .split('\n')
    .filter(line => line.trim().length > 0)
    .map(line => object(object(JSON.parse(line)).record));
}

/** The commitment a projection minted for one artifact path. */
function commitmentFor(state: RunPlaneFixture, path: string): string {
  const bytes = storeBytes(state).split('\n').filter(Boolean);
  for (const line of bytes) {
    const row = object(JSON.parse(line));
    const record = object(row.record);
    if (record.kind !== 'commitment') continue;
    const spec = object(record.resourceSpec);
    if ((spec.classifiedAs as string[]).includes(path)) return String(row.cid);
  }
  throw new Error(`no commitment was minted for ${path}`);
}

/** Every event whose `fulfills` names a promise — the one discharge path the fold counts. */
function discharges(state: RunPlaneFixture): JsonObject[] {
  return records(state).filter(
    record => record.kind === 'event' && ((record.fulfills as unknown[]) ?? []).length > 0
  );
}

function featureText(name: string): string {
  return [
    `Feature: ${name}`,
    '  @concern:resiliency',
    `  Scenario: ${name} holds`,
    '    Given a node dies',
    '    Then the data survives',
    '',
  ].join('\n');
}

/** Add N work items to the plan, commit them inside the window, and project them into promises. */
function gainClaimedWorkItems(state: RunPlaneFixture, count: number): void {
  const paths: string[] = [];
  for (let index = 0; index < count; index += 1) {
    state.minted += 1;
    const rel = `${FEATURES}/${String(state.minted).padStart(2, '0')}-claimed.feature`;
    write(state, rel, featureText(`claimed ${state.minted}`));
    paths.push(rel);
  }
  commitAt(state, MINT_DATE, `${count} claimed work item(s) inside the window`);
  project(state);
  state.window = MEASURED_WINDOW;
  state.windowCommitments = paths.map(path => commitmentFor(state, path));
  assert.equal(state.windowCommitments.length, count);
}

/** Discharge one promise the way the fulfil verb does, dated inside the declared window. */
function dischargeCommitment(state: RunPlaneFixture, commitment: string): void {
  const report = 'genesis/reports/discharge.json';
  write(state, report, `${JSON.stringify({ status: 'DONE', commitment }, null, 2)}\n`);
  commitAt(state, DISCHARGE_DATE, 'evidence for a discharge inside the window');
  flow(state, [
    'fulfill',
    '--on',
    commitment,
    '--report',
    report,
    '--status',
    'DONE',
    '--as',
    'agent:implementer@fixture',
  ]);
}

function writeCorrection(state: RunPlaneFixture, target: string): Invocation {
  return run(state, [
    binary,
    'flow',
    'note',
    '--on',
    target,
    '--kind',
    'correction',
    '--reason',
    CORRECTION_REASON,
    '--switched-to',
    CORRECTION_SWITCHED_TO,
    '--root',
    state.root,
    '--json',
  ]);
}

function frontier(state: RunPlaneFixture): JsonObject {
  return flow(state, ['status']);
}

function stillOwed(status: JsonObject): string[] {
  return (status.top_unfulfilled as JsonObject[]).map(row => String(object(row.commitment).cid));
}

/** The measures block of the stock the reading is about. */
function measures(state: RunPlaneFixture): JsonObject {
  const reading = state.reading;
  assert.ok(reading, 'the equilibrium check must have run');
  const stock = (reading.json.stocks as JsonObject[])[0];
  assert.equal(stock.stock, 'commitments', 'the reading names the stock it is about');
  return object(stock.measures);
}

function verdict(state: RunPlaneFixture): string {
  const reading = state.reading;
  assert.ok(reading, 'the equilibrium check must have run');
  const stock = (reading.json.stocks as JsonObject[])[0];
  return String(object(stock.verdict).verdict);
}

function readingText(state: RunPlaneFixture): string {
  assert.ok(state.reading, 'the equilibrium check must have run');
  return state.reading.text;
}

// ════════════════════════════════════════════════════════════════════════════
// Background
// ════════════════════════════════════════════════════════════════════════════

Given(
  'a repository whose plan carries one work item marked claimed',
  { timeout: 60_000 },
  function () {
    const root = mkdtempSync(join(scratchBase, 'run-plane-'));
    const env = { ...process.env };
    for (const key of ['GIT_DIR', 'GIT_WORK_TREE', 'GIT_INDEX_FILE', 'GIT_COMMON_DIR'])
      delete env[key];
    Object.assign(env, {
      GIT_CONFIG_COUNT: '1',
      GIT_CONFIG_KEY_0: 'safe.directory',
      GIT_CONFIG_VALUE_0: root,
      GIT_AUTHOR_NAME: 'Fixture',
      GIT_AUTHOR_EMAIL: 'fixture@example.test',
      GIT_COMMITTER_NAME: 'Fixture',
      GIT_COMMITTER_EMAIL: 'fixture@example.test',
    });
    const state: RunPlaneFixture = {
      root,
      env,
      window: MEASURED_WINDOW,
      backgroundCommitment: '',
      windowCommitments: [],
      minted: 0,
    };
    fixtures.set(this, state);

    // The plan: one recipe stage whose claimed work items are the scenarios under it.
    write(
      state,
      RECIPES,
      [
        'version: 1',
        'recipes:',
        '  - id: resiliency-saga',
        '    version: 1',
        '    description: the fixture plan',
        '    stages:',
        '      - name: scenario',
        '        artifactKind: "a2o:feature"',
        '        paths:',
        `          - "${FEATURES}/**/*.feature"`,
        '    edges: []',
        '',
      ].join('\n')
    );
    state.minted += 1;
    write(state, `${FEATURES}/01-claimed.feature`, featureText('claimed 1'));
    git(state, BACKGROUND_DATE, ['init', '-q']);
    commitAt(state, BACKGROUND_DATE, 'the background work item, before every window');
  }
);

Given(
  'that repository has been projected, so the flow store holds one open commitment for that work item',
  { timeout: 60_000 },
  function () {
    const state = fixture(this);
    project(state);
    const status = frontier(state);
    assert.equal(status.unfulfilled_total, 1, 'projection minted exactly one open commitment');
    state.backgroundCommitment = commitmentFor(state, `${FEATURES}/01-claimed.feature`);
  }
);

// ════════════════════════════════════════════════════════════════════════════
// 1. The write leg
// ════════════════════════════════════════════════════════════════════════════

Given(
  'a correction is written against the open commitment naming what went wrong and what to do instead',
  function () {
    const state = fixture(this);
    const result = writeCorrection(state, state.backgroundCommitment);
    assert.equal(result.status, 0, result.stderr);
    state.note = object(JSON.parse(result.stdout));
    state.headBefore = ok(state, ['git', 'rev-parse', 'HEAD']).trim();
  }
);

Given('the session that wrote it has ended', function () {
  const state = fixture(this);
  // A session is one process lifetime. Every CLI invocation above has already exited; what is
  // dropped here is everything this harness still remembers ABOUT that session, so the later
  // read below has no route to the text but the flow store itself.
  state.note = undefined;
  state.corrections = undefined;
});

When('a later session reads the flow store with no conversation carried over', function () {
  const state = fixture(this);
  const view = flow(state, ['concerns', '--corrections']);
  const surfaces = object(view.surfaces);
  state.corrections = Object.values(surfaces).flatMap(rows => (rows as JsonObject[]).map(object));
});

Then('that correction is still readable in the flow store, word for word', function () {
  const state = fixture(this);
  assert.ok(state.corrections, 'the later session read the store');
  const found = state.corrections.filter(row => row.reason === CORRECTION_REASON);
  assert.equal(found.length, 1, `the correction text survived verbatim: ${JSON.stringify(found)}`);
});

Then('the correction still names the commitment it was written against', function () {
  const state = fixture(this);
  const found = (state.corrections ?? []).find(row => row.reason === CORRECTION_REASON);
  assert.ok(found, 'the correction is readable');
  assert.equal(found.target, state.backgroundCommitment);
});

Given('the open commitment is among the still-owed commitments in the frontier', function () {
  const state = fixture(this);
  const status = frontier(state);
  assert.ok(stillOwed(status).includes(state.backgroundCommitment));
  state.frontierBefore = Number(status.unfulfilled_total);
});

When('the agent writes a correction against that commitment', function () {
  const state = fixture(this);
  const result = writeCorrection(state, state.backgroundCommitment);
  assert.equal(result.status, 0, result.stderr);
  state.note = object(JSON.parse(result.stdout));
});

Then('that commitment is still among the still-owed commitments in the frontier', function () {
  const state = fixture(this);
  assert.ok(stillOwed(frontier(state)).includes(state.backgroundCommitment));
});

Then('the count of unfulfilled commitments, which counts that same set, is unchanged', function () {
  const state = fixture(this);
  assert.equal(frontier(state).unfulfilled_total, state.frontierBefore);
});

Then('the record written is a note, not a fulfillment', function () {
  const state = fixture(this);
  assert.equal(object(state.note).kind, 'run:correction');
  assert.deepEqual(discharges(state), [], 'nothing in the store discharges a promise');
});

Given('a correction has already been written against the open commitment', function () {
  const state = fixture(this);
  const result = writeCorrection(state, state.backgroundCommitment);
  assert.equal(result.status, 0, result.stderr);
  state.note = object(JSON.parse(result.stdout));
  assert.equal(state.note.appended, true, 'the first write appended a record');
  state.headBefore = ok(state, ['git', 'rev-parse', 'HEAD']).trim();
  state.storeBefore = storeBytes(state);
});

Given(
  'the repository is at the same commit it was at when that correction was written',
  function () {
    const state = fixture(this);
    assert.equal(ok(state, ['git', 'rev-parse', 'HEAD']).trim(), state.headBefore);
  }
);

When('the same correction text is written against that same commitment again', function () {
  const state = fixture(this);
  state.refusal = writeCorrection(state, state.backgroundCommitment);
});

Then('the flow store holds exactly one such correction', function () {
  const state = fixture(this);
  const written = records(state).filter(record =>
    (((record.classifiedAs as string[]) ?? [])[0] ?? '').startsWith('run:correction')
  );
  assert.equal(written.length, 1, 'the replay added no second record');
  assert.equal(storeBytes(state), state.storeBefore, 'the append-only store is unchanged');
});

Then('the second write reports success rather than an error', function () {
  const state = fixture(this);
  const replay = state.refusal;
  assert.ok(replay);
  assert.equal(replay.status, 0, replay.stderr);
  const view = object(JSON.parse(replay.stdout));
  assert.equal(view.appended, false, 'the replay recognised its own record');
  assert.equal(view.record_cid, object(state.note).record_cid, 'same content, same address');
});

Given('a correction names a commitment that does not exist in the flow store', function () {
  const state = fixture(this);
  const minted = storeBytes(state).split('\n').filter(Boolean);
  assert.ok(
    !minted.some(line => line.includes(ABSENT_COMMITMENT)),
    'the named target is well-formed and minted by nothing in this store'
  );
  state.storeBefore = storeBytes(state);
});

When('that correction is written', function () {
  const state = fixture(this);
  state.refusal = writeCorrection(state, ABSENT_COMMITMENT);
});

Then(
  'the write exits with a non-zero status and names the target it could not resolve',
  function () {
    const state = fixture(this);
    const refusal = state.refusal;
    assert.ok(refusal);
    assert.notEqual(refusal.status, 0, 'an orphaned note is refused, not filed loose');
    assert.match(
      `${refusal.stdout}${refusal.stderr}`,
      new RegExp(ABSENT_COMMITMENT),
      'the refusal names the target it could not resolve'
    );
  }
);

Then('the flow store is byte-for-byte what it was before the attempt', function () {
  const state = fixture(this);
  assert.equal(storeBytes(state), state.storeBefore);
});

// ════════════════════════════════════════════════════════════════════════════
// 3. The equilibrium verdict
// ════════════════════════════════════════════════════════════════════════════

Given(
  'a declared window of one week in which the projected repository gained two claimed work items',
  { timeout: 60_000 },
  function () {
    gainClaimedWorkItems(fixture(this), 2);
  }
);

Given(
  'a declared window of one week in which the projected repository gained one claimed work item',
  { timeout: 60_000 },
  function () {
    gainClaimedWorkItems(fixture(this), 1);
  }
);

Given(
  'a declared window of one week in which the projected repository gained no claimed work item',
  function () {
    const state = fixture(this);
    // Nothing is added and nothing is projected; the declared window is simply one the
    // repository's history never reaches into.
    state.window = EMPTY_WINDOW;
  }
);

Given(
  'no fulfillment in that week named either of those commitments or the background commitment',
  function () {
    assert.deepEqual(discharges(fixture(this)), [], 'no promise has been discharged at all');
  }
);

Given(
  'no fulfillment in that week named any commitment — the background commitment predates the window, so nothing falls inside it to measure',
  function () {
    assert.deepEqual(discharges(fixture(this)), [], 'no promise has been discharged at all');
  }
);

Given(
  'one fulfillment in that week named the commitment from the Background, so as many were discharged in the window as were minted in it',
  { timeout: 60_000 },
  function () {
    const state = fixture(this);
    dischargeCommitment(state, state.backgroundCommitment);
    assert.equal(discharges(state).length, 1);
  }
);

Given(
  'a fulfillment in that week named that same commitment, discharging it',
  { timeout: 60_000 },
  function () {
    const state = fixture(this);
    assert.equal(state.windowCommitments.length, 1, 'exactly one promise was minted in the window');
    dischargeCommitment(state, state.windowCommitments[0]);
    assert.equal(discharges(state).length, 1);
  }
);

When(
  'the equilibrium check runs over the commitment stock for that declared window',
  { timeout: 60_000 },
  function () {
    const state = fixture(this);
    const argv = [
      binary,
      'flow',
      'stocks',
      '--window',
      state.window,
      '--per',
      'week',
      '--stock',
      'commitments',
      '--check',
      '--root',
      state.root,
    ];
    // The check is read-only, so the same fold is taken twice: once rendered for a reader, once
    // as JSON. The exit status asserted below is the RENDERED run's — the one an operator sees.
    const rendered = run(state, argv);
    const json = run(state, [...argv, '--json']);
    assert.equal(
      rendered.status,
      json.status,
      'the same fold exits the same way however it is rendered'
    );
    state.reading = {
      text: rendered.stdout,
      json: object(JSON.parse(json.stdout)),
      status: rendered.status,
    };
  }
);

Then('the check exits with a non-zero status', function () {
  const state = fixture(this);
  assert.ok(state.reading);
  assert.notEqual(state.reading.status, 0, `expected a non-zero exit:\n${state.reading.text}`);
});

Then('the check exits with a zero status', function () {
  const state = fixture(this);
  assert.ok(state.reading);
  assert.equal(state.reading.status, 0, `expected a zero exit:\n${state.reading.text}`);
});

Then('the reading names the commitment stock as filling', function () {
  const state = fixture(this);
  assert.equal(verdict(state), 'filling');
  assert.match(readingText(state), /stock: commitments \(unit "commitment"\)/);
  assert.match(readingText(state), /verdict: {2}FILLING/);
});

Then(
  'the reading states inflow and outflow as rates over that window, not as a level against a ceiling',
  function () {
    const state = fixture(this);
    const text = readingText(state);
    // Both rates are printed PER the declared period, beside the window they were taken over.
    assert.match(text, /window: 2026-08-06T00:00:00Z \.\. 2026-08-13T00:00:00Z \(1 Week-periods\)/);
    assert.match(text, /inflow: {3}\d+\.\d{3} per Week/);
    assert.match(text, /outflow: {2}\d+\.\d{3} per Week/);
    assert.match(text, /net: {6}[+-]\d+\.\d{3} per Week/);
    // The level is reported too, but it is not what the verdict is taken on: no band, no limit,
    // no ceiling is named for this stock.
    assert.doesNotMatch(text, /band:|limit \d|ceiling/);
    const m = measures(state);
    assert.equal(typeof m.inflow, 'number');
    assert.equal(typeof m.outflow, 'number');
    assert.match(String(m.inflow_basis), /over 2026-08-06T00:00:00Z \.\. 2026-08-13T00:00:00Z/);
    assert.match(String(m.outflow_basis), /over 2026-08-06T00:00:00Z \.\. 2026-08-13T00:00:00Z/);
    assert.match(String(m.level_basis), /all history through/);
  }
);

Then(
  'the reading names the fulfillment path as the only discharge path it counted as drain',
  function () {
    assert.match(readingText(fixture(this)), /outflow arm: fulfillment only/);
  }
);

Then('the reading names the commitment stock as holding or draining', function () {
  const state = fixture(this);
  assert.equal(verdict(state), 'draining', readingText(state));
  const m = measures(state);
  assert.ok(
    Number(m.outflow) >= Number(m.inflow),
    `drain ${m.outflow} must be at least inflow ${m.inflow}`
  );
});

Then('the reading names the drain as unknown rather than adequate', function () {
  const state = fixture(this);
  assert.equal(verdict(state), 'refused');
  const stock = (state.reading!.json.stocks as JsonObject[])[0];
  assert.equal(object(object(stock.verdict).reason).reason, 'no-observations');
  assert.match(readingText(state), /verdict: {2}REFUSED/);
  assert.match(readingText(state), /absence, not a drained stock/);
  assert.equal(stock.observed_in_window, 0, 'the window showed the check nothing to count');
});

Then('the reading never reports equilibrium for a window it could not measure', function () {
  const state = fixture(this);
  assert.equal(state.reading!.json.equilibrium, false);
  assert.match(readingText(state), /equilibrium \(every stock draining\): no/);
  assert.doesNotMatch(readingText(state), /equilibrium \(every stock draining\): yes/);
});

Then('the reading counts that discharge exactly once as outflow', function () {
  const state = fixture(this);
  const m = measures(state);
  assert.equal(m.outflow, 1, 'one discharge, counted once');
  assert.match(String(m.outflow_basis), /^1 consume events on commitment/);
  const stock = (state.reading!.json.stocks as JsonObject[])[0];
  assert.equal(stock.redundant_discharges, 0, 'no discharge was counted twice');
  assert.equal(stock.discharges_of_unheld, 0, 'the discharge named a promise the stock held');
});

Then('the reading does not count that discharge as inflow', function () {
  const state = fixture(this);
  const m = measures(state);
  // One work item was minted inside the window. If the Produce-shaped fulfillment were folded
  // raw, this would read 2 — the sign inversion arriving through the data.
  assert.equal(m.inflow, 1, 'inflow counts the mint only');
  assert.match(String(m.inflow_basis), /^1 produce events on commitment/);
});

Then(
  'the reading names the commitment stock as holding or draining rather than filling',
  function () {
    const state = fixture(this);
    assert.equal(verdict(state), 'draining', readingText(state));
    assert.doesNotMatch(readingText(state), /verdict: {2}FILLING/);
  }
);

After(function () {
  const state = fixtures.get(this);
  if (state) rmSync(state.root, { recursive: true, force: true });
  fixtures.delete(this);
});
