import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

type RecordValue = Record<string, unknown>;
interface Fixture {
  root: string;
  env: NodeJS.ProcessEnv;
  view: RecordValue;
  opened?: RecordValue;
  unavailable?: RecordValue;
  receipt?: string;
  inspectedSource?: string;
}

const fixtures = new WeakMap<object, Fixture>();
const repository = resolve('../..');
const binary = process.env.EPR_BIN ?? 'epr';
// The footprint measurement is native to `epr flow memory recall`; `--lens <script>` remains only
// as an explicit override for an external lens and is never needed here.
const upstream = 'genesis/source.md';
const repaired = 'genesis/repair.md';
const contested = 'genesis/conflict.md';
const intent = 'Help the next reader distinguish the supported repair from a contested promise.';
const question =
  'Which independent reader should the owner appoint to judge whole-workflow acceptance?';
const nextAction = 'Identify the source owner and ask them to arrange independent evaluation';

function record(value: unknown): RecordValue {
  assert.ok(value && typeof value === 'object' && !Array.isArray(value));
  return value as RecordValue;
}

function rows(value: unknown): RecordValue[] {
  assert.ok(Array.isArray(value));
  return value.map(record);
}

function fixture(world: object): Fixture {
  const found = fixtures.get(world);
  assert.ok(found, 'an isolated ceremony fixture must exist');
  return found;
}

function write(state: Fixture, path: string, text: string): void {
  mkdirSync(dirname(join(state.root, path)), { recursive: true });
  writeFileSync(join(state.root, path), text);
}

function invoke(state: Fixture, argv: string[], expected = 0): string {
  const result = spawnSync(argv[0], argv.slice(1), {
    cwd: state.root,
    env: state.env,
    encoding: 'utf8',
    timeout: 30_000,
    maxBuffer: 1024 * 1024,
  });
  assert.equal(result.status, expected, `${argv.join(' ')}\n${result.stdout}\n${result.stderr}`);
  return result.stdout;
}

function native(state: Fixture, args: string[]): RecordValue {
  return record(
    JSON.parse(invoke(state, [binary, 'flow', ...args, '--root', state.root, '--json']))
  );
}

function journey(
  state: Fixture,
  operation: string,
  args: string[] = [],
  expected = 0
): RecordValue {
  state.view = record(
    JSON.parse(
      invoke(
        state,
        [
          binary,
          'flow',
          'memory',
          'recall',
          operation,
          '--root',
          state.root,
          '--contract',
          join(state.root, 'contract.json'),
          '--session',
          'reader',
          '--json',
          ...args,
        ],
        expected
      )
    )
  );
  return state.view;
}

function follow(state: Fixture, action: RecordValue): RecordValue {
  assert.ok(Array.isArray(action.argv));
  // A linked action names the verb `epr`; this run pins the built binary under test.
  const argv = [...(action.argv as string[])];
  argv[0] = binary;
  state.view = record(JSON.parse(invoke(state, [...argv, '--json'])));
  return state.view;
}

function choose(state: Fixture, path: string): void {
  const opened = journey(state, 'open', ['--intent', intent]);
  state.opened = opened;
  const choice = rows(opened.actions).find(item => String(item.label).includes(`${path} →`));
  assert.ok(choice, `the entry offers navigation to ${path}`);
  follow(state, choice);
}

function inspect(state: Fixture, classification = 'unreviewed'): void {
  const view = journey(state, 'read', [
    '--path',
    upstream,
    '--lines',
    '4:6',
    '--need',
    'Compare the promise to the changed source',
  ]);
  const receipt = rows(record(view.evidence).sources)[0];
  state.receipt = `${String(receipt.path)}:${String(receipt.lines)}`;
  state.inspectedSource = readFileSync(join(state.root, upstream), 'utf8');
  journey(state, 'remember', [
    '--finding',
    classification === 'evidence-ready'
      ? 'The report records that the trial reader opened a cited passage.'
      : 'The report states whole-workflow acceptance remains undecided.',
    '--question',
    question,
    '--next-action',
    nextAction,
    '--evidence',
    state.receipt,
    '--classification',
    classification,
  ]);
}

function createFixture(world: object): Fixture {
  const root = mkdtempSync(join(tmpdir(), 'ceremony-story-'));
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
  const state: Fixture = { root, env, view: {} };
  fixtures.set(world, state);
  write(
    state,
    upstream,
    '---\ntitle: Source promise\n---\n# Promise\nA trial reader opened a cited passage.\nWhole-journey acceptance remains undecided.\n'
  );
  write(state, repaired, '# Supported assertion\nA trial reader opened a cited passage.\n');
  write(
    state,
    contested,
    '# Conflicting assertion\nAn independent reader accepted the whole journey.\n'
  );
  const contract = JSON.parse(
    readFileSync(join(repository, '.epr-meta/elohim/algorithms/recall-contract.json'), 'utf8')
  ) as RecordValue;
  write(state, 'contract.json', JSON.stringify(contract));
  invoke(state, ['git', 'init', '-q']);
  invoke(state, ['git', 'add', upstream, repaired, contested, 'contract.json']);
  invoke(state, [
    'git',
    '-c',
    'core.hooksPath=/dev/null',
    'commit',
    '-q',
    '-m',
    'isolated ceremony fixture',
  ]);
  for (const path of [repaired, contested])
    native(state, [
      'seal',
      path,
      '--on',
      upstream,
      '--governor',
      'cite-seal',
      '--desc',
      `Review ${path}`,
    ]);
  write(
    state,
    upstream,
    '---\ntitle: Source promise\n---\n# Promise\nThe trial reader followed a citation link and opened its passage.\nWhole-journey acceptance remains undecided.\n'
  );
  return state;
}

Given('a ceremony with two assertions depending on the same changed source', function () {
  createFixture(this);
});

When('the agent opens the saved investigation and chooses an assertion', function () {
  choose(fixture(this), repaired);
});

Then(
  'the selected claim shows its stale report reference and the purpose of preventing unsupported acceptance',
  function () {
    const state = fixture(this);
    assert.equal(record(state.view.orientation).intent, intent);
    assert.ok(rows(record(state.view.orientation).guiding_context).every(item => item.source));
    assert.equal(record(record(state.view.node).edge).verdict, 'stale');
    assert.ok(record(state.view.node).current_claim);
    const passage = rows(state.view.actions).find(item =>
      String(item.label).startsWith('Open to:')
    );
    assert.ok(passage, 'the local story links directly to its evidence passage');
    const view = follow(state, passage);
    assert.ok(
      JSON.stringify(view.evidence).includes('followed a citation link and opened its passage')
    );
  }
);

Then('both affected entries are listed as separately reviewable claims', function () {
  const state = fixture(this);
  const groups = rows(record(state.opened?.concerns).groups);
  assert.equal(groups.length, 1);
  const edges = rows(groups[0].edges);
  assert.equal(edges.length, 2);
  assert.deepEqual(
    new Set(edges.map(edge => record(edge.slot).from)),
    new Set([repaired, contested])
  );
  assert.equal(native(state, ['status']).edges_stale, 2, 'inspection does not author closure');
});

Given(
  'the agent inspected the disputed acceptance claim and saved the question of who should arrange independent evaluation',
  function () {
    const state = createFixture(this);
    choose(state, contested);
    inspect(state, 'conflict');
  }
);

When('a fresh process resumes the ceremony', function () {
  journey(fixture(this), 'resume');
});

Then(
  'it recovers the purpose, inspected passage receipt and task of identifying the owner to arrange evaluation',
  function () {
    const state = fixture(this);
    assert.equal(record(state.view.orientation).intent, intent);
    const continuation = record(state.view.continuation);
    assert.equal(continuation.next_action, nextAction);
    assert.ok(JSON.stringify(continuation.evidence).includes(upstream));
    assert.ok(JSON.stringify(continuation.unresolved_questions).includes(question));
    assert.equal(continuation.repeated_reads, 0);
  }
);

When('the inspected passage changes and a fresh process resumes the ceremony', function () {
  const state = fixture(this);
  write(
    state,
    upstream,
    '---\ntitle: Source promise\n---\n# Promise\nPassage inspection no longer works.\nOwner judgment is required.\n'
  );
  journey(state, 'resume');
});

Then(
  'the saved report observation is marked unusable until the changed passage is inspected again',
  function () {
    const state = fixture(this);
    const resumed = state.view;
    assert.ok(
      (record(resumed.evidence_check).revalidation_required as string[]).includes(state.receipt!)
    );
    const context = journey(state, 'context');
    assert.ok(
      rows(record(context.node).findings).every(item => item.evidence_state !== 'receipt-valid')
    );
  }
);

Given('a ceremony with an unavailable optional retrieval provider', function () {
  const state = createFixture(this);
  write(state, 'bin/mempalace', '#!/bin/sh\nexit 127\n');
  chmodSync(join(state.root, 'bin/mempalace'), 0o755);
  state.env.PATH = `${join(state.root, 'bin')}:${state.env.PATH}`;
  choose(state, repaired);
  state.unavailable = journey(
    state,
    'search',
    ['--provider', 'mempalace', '--query', 'promise'],
    2
  );
  assert.ok((state.unavailable.unresolved as string[]).length);
});

When('the agent chooses an authorized local alternative', function () {
  journey(fixture(this), 'search', [
    '--provider',
    'local',
    '--search-scope',
    'genesis',
    '--query',
    'source',
    '--name',
    'source.md',
  ]);
});

Then(
  'the local result retains the repair purpose and explicitly requires inspecting its source before using it as evidence',
  function () {
    const state = fixture(this);
    assert.equal(record(state.view.orientation).intent, intent);
    assert.equal(record(state.view.orientation).provider, 'local');
    assert.ok(rows(record(state.view.retrieval).candidates).some(item => item.path === upstream));
    assert.match(String(record(state.view.retrieval).authority), /verify.*evidence/i);
    inspect(state, 'evidence-ready');
  }
);

Then("the search view says the unavailable service's ranking is unknown", function () {
  const retrieval = record(fixture(this).unavailable?.retrieval);
  assert.match(String(retrieval.authority), /ranking.*unknown/i);
  assert.equal(record(retrieval.declaration).ranking, null);
});

Given(
  'the revised report records the trial reader opening a cited passage and leaves whole-workflow acceptance undecided',
  function () {
    const state = createFixture(this);
    choose(state, contested);
    inspect(state, 'conflict');
    choose(state, repaired);
    inspect(state, 'evidence-ready');
  }
);

When('the agent updates only the supported reference and checks the affected entries', function () {
  const state = fixture(this);
  const prepared = journey(state, 'prepare', [
    '--kind',
    'repair',
    '--finding',
    'The inspected source still supports this assertion.',
  ]);
  const argv = record(prepared.prepared_action).argv;
  assert.ok(Array.isArray(argv));
  invoke(state, argv as string[]);
  journey(state, 'reconcile');
  const status = native(state, ['status']);
  assert.equal(status.edges_sealed, 1);
  assert.equal(status.edges_stale, 1);
  journey(state, 'remember', [
    '--finding',
    'The supported reference now matches the report; independent acceptance remains unresolved.',
    '--question',
    question,
    '--next-action',
    nextAction,
    '--evidence',
    state.receipt!,
  ]);
  journey(state, 'finish', [
    '--outcome',
    'Observed one supported edge resealed; acceptance is not established.',
    '--question',
    question,
  ]);
});

Then(
  'completion reports the reference repair without claiming independent acceptance',
  function () {
    const outcome = record(fixture(this).view.outcome);
    assert.match(String(outcome.report), /acceptance is not established/);
    assert.match(String(outcome.standing), /consult native evidence for acceptance/);
  }
);

Then(
  'the handover identifies the disputed whole-workflow claim as undecided and gives the next agent its purpose and owner-identification task',
  function () {
    const view = journey(fixture(this), 'resume');
    assert.equal(record(view.orientation).intent, intent);
    assert.ok(JSON.stringify(record(view.continuation).unresolved_questions).includes(question));
    const continuation = record(view.continuation);
    assert.equal(continuation.next_action, nextAction);
    const historyChoice = rows(view.actions).find(item =>
      String(item.label).startsWith('Expand retained findings')
    );
    assert.ok(historyChoice, 'the handover exposes omitted findings through a linked choice');
    const history = follow(fixture(this), historyChoice);
    assert.equal(record(history.orientation).intent, intent);
    assert.equal(history.next_action, nextAction);
    assert.ok(
      rows(record(history.history).findings).some(
        item =>
          record(item.edge).from === contested &&
          item.classification === 'conflict' &&
          String(item.claim).includes('acceptance remains undecided')
      )
    );
    assert.equal(native(fixture(this), ['status']).edges_stale, 1);
  }
);

After(function () {
  const state = fixtures.get(this);
  if (state) rmSync(state.root, { recursive: true, force: true });
  fixtures.delete(this);
});

Then('the report observation is reusable while the acceptance question remains open', function () {
  const state = fixture(this);
  assert.ok(
    (record(state.view.evidence_check).unchanged_receipts as string[]).includes(state.receipt!)
  );
  assert.deepEqual(record(state.view.evidence_check).revalidation_required, []);
  assert.equal(record(state.view.continuation).repeated_reads, 0);
  const context = journey(state, 'context');
  assert.ok(
    rows(record(context.node).findings).some(item => item.evidence_state === 'receipt-valid')
  );
});

Then('the local search finds the report for direct passage inspection', function () {
  const retrieval = record(fixture(this).view.retrieval);
  assert.ok(rows(retrieval.candidates).some(item => item.path === upstream));
});

Given('the inspected passage has not changed since that receipt', function () {
  const state = fixture(this);
  assert.equal(readFileSync(join(state.root, upstream), 'utf8'), state.inspectedSource);
});

Then(
  'the concern view shows the supported reference current and the disputed entry still awaiting review against its old reference',
  function () {
    const state = fixture(this);
    const status = native(state, ['status']);
    assert.equal(status.edges_sealed, 1);
    assert.equal(status.edges_stale, 1);
    const concerns = native(state, ['context', '.', '--concerns']);
    const outstanding = rows(concerns.groups).flatMap(group => rows(group.edges));
    assert.equal(outstanding.length, 1);
    assert.equal(record(outstanding[0].slot).from, contested);
    assert.equal(record(outstanding[0].slot).to, upstream);
    assert.equal(outstanding[0].verdict, 'stale');
  }
);
