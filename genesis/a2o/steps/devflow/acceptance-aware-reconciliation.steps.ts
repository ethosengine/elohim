import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

interface Fixture {
  root: string;
  commitment: string;
  fulfillment?: string;
  review?: string;
  appointment?: string;
  acceptor?: string;
  acceptance?: string;
  context?: Record<string, unknown>;
  text?: string;
}

const fixtures = new WeakMap<object, Fixture>();
const scope = 'plans/experience.md';
const delivery = 'delivery.md';
// The host fixture uses the operator-selected toolchain, with no shell interpolation.
const gitBinary = process.env.GIT_BIN ?? 'git';

function fixture(world: object): Fixture {
  const found = fixtures.get(world);
  assert.ok(found, 'isolated reconciliation fixture must exist');
  return found;
}

function write(root: string, path: string, body: string): void {
  mkdirSync(dirname(join(root, path)), { recursive: true });
  writeFileSync(join(root, path), body);
}

function run(root: string, args: string[], json = true): string {
  return execFileSync(
    process.env.EPR_BIN ?? 'epr',
    [...args, '--root', root, ...(json ? ['--json'] : [])],
    { cwd: root, encoding: 'utf8', timeout: 30_000, maxBuffer: 8 * 1024 * 1024 }
  );
}

function command(root: string, args: string[]): Record<string, unknown> {
  return JSON.parse(run(root, args)) as Record<string, unknown>;
}

function cid(result: Record<string, unknown>, key: string): string {
  assert.equal(typeof result[key], 'string', `CLI response must include ${key}`);
  return result[key] as string;
}

function identityCid(root: string, path: string): string {
  const result = command(root, ['flow', 'context', path]);
  return cid(result.identity as Record<string, unknown>, 'cid');
}

Given(
  'an isolated reconciliation repository with one promised experience and simulated acceptance evidence',
  function () {
    const root = mkdtempSync(join(tmpdir(), 'acceptance-reconciliation-'));
    const state: Fixture = { root, commitment: '' };
    fixtures.set(this, state);
    write(
      root,
      '.claude/epr-meta/recipes.yaml',
      `version: 1
recipes:
  - id: acceptance-fixture
    version: 1
    description: isolated acceptance story
    stages:
      - name: plan
        artifactKind: "doc:plan"
        paths: ["plans/*.md"]
      - name: intent
        artifactKind: "gap:item"
        paths: ["gap-items/*.json"]
    edges: []
`
    );
    write(
      root,
      scope,
      '---\nid: experience\n---\n\nA reader can find the next unresolved assertion.\n'
    );
    write(
      root,
      'gap-items/experience.json',
      JSON.stringify({
        doc: scope,
        items: [{ id: 'experience#1', state: 'OPEN' }],
      })
    );
    write(root, delivery, 'Implemented and verified the context reader.\n');
    const git = (args: string[]): string =>
      execFileSync(gitBinary, args, {
        cwd: root,
        encoding: 'utf8',
        env: {
          ...process.env,
          GIT_AUTHOR_NAME: 'Fixture',
          GIT_AUTHOR_EMAIL: 'fixture@example.test',
          GIT_COMMITTER_NAME: 'Fixture',
          GIT_COMMITTER_EMAIL: 'fixture@example.test',
        },
      });
    git(['init', '-q']);
    git(['add', '.claude/epr-meta/recipes.yaml', scope, 'gap-items/experience.json', delivery]);
    git(['-c', 'core.hooksPath=/dev/null', 'commit', '-q', '-m', 'isolated fixture']);
    command(root, ['flow', 'project']);
    command(root, [
      'actor',
      'claim',
      '--as',
      'agent:implementer@fixture',
      '--session',
      'implementer',
    ]);
    state.commitment = cid(
      command(root, ['flow', 'claim', '--on', 'experience#1', '--session', 'implementer']),
      'commitment_cid'
    );
  }
);

Given('the implementer delivered the experience and technical review approved it', function () {
  const state = fixture(this);
  state.fulfillment = cid(
    command(state.root, [
      'flow',
      'fulfill',
      '--on',
      state.commitment,
      '--report',
      delivery,
      '--status',
      'DONE',
      '--session',
      'implementer',
    ]),
    'record_cid'
  );
  state.review = cid(
    command(state.root, [
      'flow',
      'note',
      '--on',
      state.commitment,
      '--kind',
      'verdict',
      '--verdict',
      'approved',
      '--reason',
      'Technical checks inspected',
      '--as',
      'agent:reviewer@fixture',
    ]),
    'record_cid'
  );
});

Given('an independent agent is appointed to accept that exact promise', function () {
  const state = fixture(this);
  state.acceptor = cid(
    command(state.root, [
      'actor',
      'claim',
      '--as',
      'agent:acceptor@fixture',
      '--session',
      'acceptor',
    ]),
    'recordCid'
  );
  state.appointment = cid(
    command(state.root, [
      'flow',
      'note',
      '--on',
      state.commitment,
      '--kind',
      'ruling',
      '--appoint',
      state.acceptor,
      '--reason',
      'Appointed to exercise the reader',
      '--as',
      'agent:orchestrator@fixture',
    ]),
    'record_cid'
  );
});

When('the appointed agent records its exercised experience and approves acceptance', function () {
  const state = fixture(this);
  assert.ok(state.appointment && state.fulfillment && state.review);
  write(state.root, 'observations.md', 'Tried context; the unresolved assertion was visible.\n');
  const evidenceCid = identityCid(state.root, 'observations.md');
  const deliveryCid = identityCid(state.root, delivery);
  const revision = execFileSync(gitBinary, ['rev-parse', 'HEAD'], {
    cwd: state.root,
    encoding: 'utf8',
  }).trim();
  write(
    state.root,
    'acceptance.json',
    JSON.stringify({
      intent: 'Find the next unresolved assertion',
      revision,
      environment: 'isolated local fixture',
      actions: ['Ran native context for the promised experience'],
      observations: ['The unresolved assertion and its evidence were visible'],
      evidence: [
        { path: 'observations.md', cid: evidenceCid },
        { path: delivery, cid: deliveryCid },
      ],
      limitations: ['Simulated report tests admission, not real product fitness'],
    })
  );
  state.acceptance = cid(
    command(state.root, [
      'flow',
      'note',
      '--on',
      state.commitment,
      '--kind',
      'verdict',
      '--verdict',
      'approved',
      '--purpose',
      'acceptance',
      '--appointment',
      state.appointment,
      '--fulfillment',
      state.fulfillment,
      '--review',
      state.review,
      '--report',
      'acceptance.json',
      '--reason',
      'Exercised the intended reader workflow',
      '--session',
      'acceptor',
    ]),
    'record_cid'
  );
});

When('the external observation artifact changes without editing the ledger', function () {
  write(fixture(this).root, 'observations.md', 'The earlier observation is no longer supported.\n');
});

When('the ceremony reader asks native context what remains', function () {
  const state = fixture(this);
  state.context = command(state.root, ['flow', 'context', scope]);
  state.text = run(state.root, ['flow', 'context', scope], false);
});

Then('the experience has reconciliation state {string}', function (expected: string) {
  const state = fixture(this);
  assert.ok(state.context?.reconciliation, 'native reconciliation projection must be present');
  const projection = state.context.reconciliation as { assertions: { acceptance: string }[] };
  assert.equal(projection.assertions.length, 1, 'the fixture has one scoped assertion');
  assert.equal(projection.assertions[0]?.acceptance, expected);
  assert.ok(state.text?.includes(expected), 'human rendering must agree with JSON');
});

Then('the delivery and technical approval remain visible as evidence', function () {
  const state = fixture(this);
  const reconciliation = JSON.stringify(state.context?.reconciliation);
  assert.ok(state.fulfillment && reconciliation.includes(state.fulfillment));
  assert.ok(state.review && reconciliation.includes(state.review));
});

Then('the original acceptance record remains in the ledger', function () {
  const state = fixture(this);
  assert.ok(state.acceptance);
  assert.ok(
    readFileSync(join(state.root, '.eprfs/status/flows.jsonl'), 'utf8').includes(state.acceptance)
  );
});

After({ tags: '@concern:acceptance-aware-reconciliation' }, function () {
  const state = fixtures.get(this);
  if (state) rmSync(state.root, { recursive: true, force: true });
  fixtures.delete(this);
});
