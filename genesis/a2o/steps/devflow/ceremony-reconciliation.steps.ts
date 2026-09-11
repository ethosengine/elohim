import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  statSync,
  writeFileSync,
} from 'node:fs';
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
  genericPurpose?: string;
  refusal?: RecordValue;
  skillRead?: RecordValue;
  packetLimit?: number;
}

const fixtures = new WeakMap<object, Fixture>();
const repository = resolve('../..');
const binary = process.env.EPR_BIN ?? 'epr';
// The footprint measurement is native to `epr flow memory recall`; `--lens <script>` remains only
// as an explicit override for an external lens and is never needed here.
/// Named once: the fixture's copy of the live algorithm artifact, and the flag that carries an
/// unresolved question into a note or a completion.
const contractFile = 'contract.json';
const questionFlag = '--question';
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
          join(state.root, contractFile),
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
    questionFlag,
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
  write(state, contractFile, JSON.stringify(contract));
  invoke(state, ['git', 'init', '-q']);
  invoke(state, ['git', 'add', upstream, repaired, contested, contractFile]);
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
    questionFlag,
    question,
    '--next-action',
    nextAction,
    '--evidence',
    state.receipt!,
  ]);
  journey(state, 'finish', [
    '--outcome',
    'Observed one supported edge resealed; acceptance is not established.',
    questionFlag,
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

// ── the recall journey: the agent's own question, and the tooling layer ──────────────────────────

const skill = '.claude/skills/memory-ceremony/SKILL.md';
// A path the fixture's declared source roots do not cover — the contrast that proves the scope.
const outside = 'outside-every-root.md';
const agentQuestion = 'Which command rebuilds the stale index now the old scripts are gone?';

Given('a ceremony whose declared source scope covers the tooling directory', function () {
  const state = createFixture(this);
  // The fixture writes the LIVE algorithm artifact as its contract, so the scope this step names
  // is the declared one and not a fixture convenience.
  const contract = JSON.parse(readFileSync(join(state.root, contractFile), 'utf8')) as Record<
    string,
    unknown
  >;
  assert.ok(
    (contract.source_roots as string[]).some(root => skill.startsWith(root.replace(/\/$/, '/'))),
    'the declared source roots cover the tooling directory'
  );
});

Given(
  'the recall entry, the one command that opens an investigation, reads a bounded passage and reports a purpose and a byte count',
  function () {
    // The entry answers for itself: its own usage names the three operations this story uses, so
    // "one command" is a fact the story checks rather than a term the narrative asserts.
    const usage = invoke(fixture(this), [binary, 'flow', 'memory', 'recall', '--help']);
    for (const operation of ['open', 'read', 'finish'])
      assert.ok(usage.includes(operation), `the entry's usage names ${operation}: ${usage}`);
    assert.match(usage, /--session/);
  }
);

Given('a skill file in that directory naming the command that rebuilds a stale index', function () {
  write(
    fixture(this),
    skill,
    '---\ntitle: Memory ceremony\n---\n# Phase 4\nRebuild the stale index with the three indexing commands, then stamp the marker.\n'
  );
});

Given(
  'the standing description that entry states as its purpose when nobody brings a question',
  function () {
    const state = fixture(this);
    // Observed, not quoted: a throwaway session opened with NO question is what the generic purpose
    // looks like in the view the agent reads. Without this the final Then would assert a
    // distinction from something the story never shows.
    const generic = record(
      JSON.parse(
        invoke(state, [
          binary,
          'flow',
          'memory',
          'recall',
          'open',
          '--root',
          state.root,
          '--contract',
          join(state.root, contractFile),
          '--session',
          'unnamed',
          '--json',
        ])
      )
    );
    state.genericPurpose = String(record(generic.orientation).intent);
    assert.ok(state.genericPurpose.length > 0, 'the algorithm states some purpose of its own');
    // "A standing description naming no question" is checkable, not decorative: it carries none of
    // the distinctive words of the question this agent is about to bring.
    const distinctive = agentQuestion
      .toLowerCase()
      .split(/[^a-z]+/)
      .filter(word => word.length > 5);
    assert.ok(distinctive.length > 0);
    for (const word of distinctive)
      assert.ok(
        !state.genericPurpose.toLowerCase().includes(word),
        `the standing description already names "${word}", so it is not question-free`
      );
  }
);

Given(
  'a declared packet limit wider than that skill passage and far narrower than the repository',
  function () {
    const state = fixture(this);
    const contract = JSON.parse(
      readFileSync(join(state.root, contractFile), 'utf8')
    ) as RecordValue;
    const limit = Number(record(contract.limits).source_bytes);
    const passage = statSync(join(state.root, skill)).size;
    assert.ok(Number.isFinite(limit) && limit > passage, 'the passage fits inside one packet');
    // Far narrower than the corpus it is drawn from: a limit that admitted the whole tree would
    // make the closing assertion true of a journey that read everything.
    assert.ok(limit < 100_000, 'the packet limit bounds a pass, not the repository');
    state.packetLimit = limit;
  }
);

When(
  'a fresh agent with no prior ceremony context opens the recall entry carrying the question {string}',
  function (asked: string) {
    const state = fixture(this);
    // The session name below has no continuation yet, which is what "no prior context" means here:
    // nothing is inherited, so whatever purpose the view states was decided by this call alone.
    // No --intent either: the documented entry is `open --need '<question>'`.
    assert.equal(asked, agentQuestion, 'the story and the fixture ask the same question');
    const opened = journey(state, 'open', ['--need', agentQuestion]);
    state.opened = opened;
    const choice = rows(opened.actions).find(item => String(item.label).includes(`${repaired} →`));
    assert.ok(choice, `the entry offers navigation to ${repaired}`);
    follow(state, choice);
  }
);

When('it reads the passage of that skill file which names the command', function () {
  const state = fixture(this);
  const view = journey(state, 'read', ['--path', skill, '--lines', '4:5', '--need', agentQuestion]);
  const receipt = rows(record(view.evidence).sources)[0];
  state.receipt = `${String(receipt.path)}:${String(receipt.lines)}`;
  // Held on the fixture rather than read back from `state.view`: the next step deliberately runs a
  // SECOND read, and an assertion about the first one must not silently move to the second.
  state.skillRead = view;
});

Then(
  "the investigation's stated purpose is that question, not the standing description",
  function () {
    const state = fixture(this);
    assert.equal(record(record(state.opened!).orientation).intent, agentQuestion);
    assert.notEqual(agentQuestion, state.genericPurpose);
  }
);

Then(
  'the skill passage is preserved as a receipt recording the exact bytes the agent read',
  function () {
    const read = record(fixture(this).skillRead);
    const source = rows(record(read.evidence).sources)[0];
    assert.equal(source.path, skill);
    assert.match(String(source.content), /Rebuild the stale index/);
    assert.match(String(source.fingerprint), /^sha256:/);
    assert.ok((read.receipt_keys as string[]).includes(fixture(this).receipt!));
  }
);

When('it attempts the same read against a path outside the declared scope', function () {
  // The same verb and the same session as the read above — one path inside the declared scope and
  // one outside it. Running it as a declared ACTION is what lets the assertion below be about an
  // outcome rather than about a probe the story never mentions.
  const state = fixture(this);
  state.refusal = journey(state, 'read', ['--path', outside, '--lines', '1:1'], 2);
});

Then(
  'the out-of-scope read was refused, so the declared scope is what put the skill in reach',
  function () {
    // The skill was readable BECAUSE its directory is named, not because reads succeed generally.
    const refused = record(fixture(this).refusal);
    assert.match(String((refused.unresolved as string[])[0]), /outside declared source scope/);
    assert.match(String(refused.next), /declared source_roots/);
  }
);

Then(
  "the investigation's completion report names that same question, and the counted bytes stay under the packet limit",
  function () {
    const state = fixture(this);
    journey(state, 'remember', [
      '--finding',
      'The skill names the re-mine step; the verb it names must be checked against the shipped CLI.',
      questionFlag,
      agentQuestion,
      '--next-action',
      'Check the named verb against the shipped CLI before quoting it',
      '--evidence',
      state.receipt!,
    ]);
    const finished = journey(state, 'finish', [
      '--outcome',
      'Read the skill passage that names the re-mine step; the verb itself is unverified.',
      questionFlag,
      agentQuestion,
    ]);
    const outcome = record(finished.outcome);
    assert.equal(outcome.intent, agentQuestion);
    assert.equal(outcome.unresolved_frontier, agentQuestion);
    const limit = state.packetLimit!;
    const read = Number(record(record(finished.cumulative).totals).source_bytes ?? 0);
    assert.ok(read > 0, 'the journey counted the bytes it read');
    assert.ok(read <= limit, `${read} source bytes exceeded the declared packet limit ${limit}`);
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
