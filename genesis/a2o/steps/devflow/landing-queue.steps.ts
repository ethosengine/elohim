import assert from 'node:assert/strict';
import { chmodSync, readFileSync, renameSync, rmSync, writeFileSync } from 'node:fs';
import { isAbsolute, join } from 'node:path';
import { pathToFileURL } from 'node:url';

import { After, Before, Given, Then, When } from '@cucumber/cucumber';

import {
  berth,
  checkEpr,
  DEV,
  env,
  EPR,
  holds,
  ledger,
  must,
  peerStatus,
  repository,
  runSync,
  sleep,
  spawnTracked,
  startDaemon,
  stopDaemon,
  waitFor,
} from './devices-share-one-berth.fixture.js';
import { holdWithoutEpr } from './epr-binary.js';
import {
  addWorktree,
  commitChange,
  eprWrapper,
  gateCalls,
  holdSeconds,
  identity,
  intents,
  type LandingFixture,
  landingHousehold,
  landingRows,
  markers,
  moveBase,
  pathWithout,
  push,
  TAG,
} from './landing-queue.fixture.js';

interface Marker {
  re: RegExp;
  format: (...args: string[]) => string;
}
let MARKERS: Record<string, Marker>;

type Pending = ReturnType<typeof spawnTracked>;
interface State {
  f: LandingFixture;
  cwd: string;
  session: string;
  last?: Awaited<Pending['outcome']>;
  pending?: Pending;
  beforeCalls: number;
  pushed?: string;
  baseBefore?: string;
  intent?: string;
  priorAtoms?: number;
  faultEnv?: Record<string, string>;
  flowStoreAside?: string;
  worktrees: Record<string, string>;
  pushes: Record<string, Awaited<Pending['outcome']>>;
  callMarks: Record<string, number>;
  values: Record<string, unknown>;
}

const states = new WeakMap<object, State>();
const LOCAL_TAG = `${TAG} and not @requires:shem`;

function state(world: object): State {
  const value = states.get(world);
  assert.ok(value, 'the landing-queue Background did not run');
  return value;
}

function device(s: State, label: 'A' | 'B') {
  return s.f.devices[label];
}

function text(outcome: { stdout: string; stderr: string }): string {
  return `${outcome.stdout}\n${outcome.stderr}`;
}

function last(s: State) {
  assert.ok(s.last, 'no completed push');
  return s.last;
}

function hasMarker(s: State, name: string): RegExpMatchArray {
  const match = MARKERS[name].re.exec(text(last(s)));
  assert.ok(match, `push has no ${name} marker:\n${text(last(s))}`);
  return match;
}

function remoteDev(s: State): string {
  return must(
    runSync(['git', 'ls-remote', s.f.origin, 'refs/heads/dev']),
    'read server dev'
  ).stdout.split(/\s+/)[0];
}

function changeId(cwd: string): string | undefined {
  return /^Change-Id: (I[0-9a-f]{40})$/m.exec(
    runSync(['git', 'log', '-1', '--format=%B'], { cwd }).stdout
  )?.[1];
}

async function moor(s: State, label: 'A' | 'B', session: string): Promise<void> {
  must(
    await berth(device(s, label), [
      'moor',
      '--session',
      session,
      '--model',
      'fixture',
      '--lab',
      'a2o',
    ]),
    `moor ${session}`
  );
}

function stageChange(s: State, label: 'A' | 'B', cwd: string, path: string, verify = true): string {
  const sha = commitChange(cwd, {
    path,
    body: `${path} ${Date.now()}\n`,
    message: `change ${path}`,
    verify,
    label,
  });
  s.pushed = sha;
  s.intent = changeId(cwd);
  return sha;
}

async function runPush(
  s: State,
  label: 'A' | 'B',
  cwd: string,
  session: string,
  extra: Record<string, string> = {}
): Promise<void> {
  s.beforeCalls = gateCalls(device(s, label)).length;
  s.baseBefore = remoteDev(s);
  s.last = await push(device(s, label), cwd, session, { ...s.faultEnv, ...extra });
  s.pushes[session] = s.last;
  if (s.flowStoreAside) {
    renameSync(s.flowStoreAside, s.flowStoreAside.replace(/\.aside$/, ''));
    s.flowStoreAside = undefined;
  }
}

function startPush(s: State, label: 'A' | 'B', cwd: string, session: string): void {
  const d = device(s, label);
  s.beforeCalls = gateCalls(d).length;
  s.baseBefore = remoteDev(s);
  s.pending = spawnTracked(['git', 'push', 'origin', 'HEAD:dev'], {
    cwd,
    env: env(d, {
      ELOHIM_SESSION_ID: session,
      ELOHIM_SESSION_OWNER_PID: String(process.pid),
      ...identity(label),
      ...s.faultEnv,
    }),
  });
}

async function finishPush(s: State, name = s.session): Promise<void> {
  assert.ok(s.pending, 'no pending push');
  s.last = await s.pending.outcome;
  s.pushes[name] = s.last;
  s.pending = undefined;
}

Before({ tags: LOCAL_TAG }, async function (scenario) {
  const held = holdWithoutEpr(scenario);
  if (held) return held;
  ({ MARKERS } = (await import(
    pathToFileURL(join(repository, 'genesis/orchestrator/gate-landing-markers.mjs')).href
  )) as { MARKERS: Record<string, Marker> });
  checkEpr();
  const f = await landingHousehold();
  states.set(this, {
    f,
    cwd: f.devices.A.repo,
    session: 'claude-1',
    beforeCalls: 0,
    worktrees: {},
    pushes: {},
    callMarks: {},
    values: {},
  });
  return undefined;
});

After({ tags: LOCAL_TAG }, async function () {
  const s = states.get(this);
  if (!s) return;
  if (s.pending) {
    s.last = await s.pending.outcome;
    s.pending = undefined;
  }
  await Promise.all(Object.values(s.f.devices).map(d => stopDaemon(d)));
  if (process.env.LANDING_QUEUE_KEEP !== '1') rmSync(s.f.base, { recursive: true, force: true });
});

Given(
  'a fixture repository of {string} whose dev branch lives on a local server',
  function (person: string) {
    assert.equal(person, 'matthew');
    assert.ok(state(this).f.origin.endsWith('origin.git'));
  }
);

Given(
  'its one checked project is {string}, reuse-enabled, whose check fails only when both {string} and {string} exist',
  function (project: string, left: string, right: string) {
    assert.deepEqual([project, left, right], ['app', 'app/left.txt', 'app/right.txt']);
  }
);

Given(
  "matthew's device {string} has a checkout of the repository, its own berth, its own key and its own peer daemon",
  function (label: string) {
    const d = device(state(this), label as 'A');
    assert.ok(d.repo && d.berthDir && d.did && d.daemon);
  }
);

Given('session {string} is moored on device A', async function (session: string) {
  const s = state(this);
  s.session = session;
  await moor(s, 'A', session);
});

Given('{string}', function (condition: string) {
  const s = state(this);
  if (condition === 'no epr tool is reachable from the push') {
    s.faultEnv = { PATH: pathWithout('epr'), EPR_BIN: join(s.f.base, 'bin/absent-epr') };
    return;
  }
  assert.equal(condition, "the device's flow store is moved aside for the push");
  const root = String(intents(device(s, 'A')).store);
  const store = join(root, '.eprfs/status/flows.jsonl');
  const aside = `${store}.aside`;
  renameSync(store, aside);
  s.flowStoreAside = aside;
});

When(
  'session {string} pushes a change to {string} to dev',
  async function (session: string, path: string) {
    const s = state(this);
    const label = session === 'codex-1' ? 'B' : 'A';
    const cwd = device(s, label).repo;
    stageChange(s, label, cwd, path);
    await runPush(s, label, cwd, session);
  }
);

Then(
  'the push exits non-zero with the {string} line, which names the command that cures it',
  function (refusal: string) {
    const s = state(this);
    assert.notEqual(last(s).status, 0);
    const name = refusal === 'epr is required' ? 'eprRefused' : 'storeRefused';
    const line = hasMarker(s, name)[0];
    assert.match(line, /run:|EPR_BIN=/);
  }
);

Then('{string} was not checked', function (project: string) {
  const s = state(this);
  assert.equal(project, 'app');
  assert.equal(gateCalls(device(s, 'A')).length, s.beforeCalls);
});

Then('dev on the server has not moved', function () {
  const s = state(this);
  assert.equal(remoteDev(s), s.baseBefore ?? s.f.baseSha);
});

Then(
  "the first queue line of the push raises a landing intent for the commit's Change-Id on dev's current tip, with a chain of 0, before any check line",
  function () {
    const s = state(this);
    const lines = last(s).lines;
    const firstQueue = lines.findIndex(line => MARKERS.intent.re.test(line.text));
    const firstGate = lines.findIndex(line => /\[gate\]|gate app/.test(line.text));
    assert.ok(firstQueue >= 0 && (firstGate < 0 || firstQueue < firstGate));
    const match = MARKERS.intent.re.exec(lines[firstQueue].text);
    assert.equal(match?.[2], s.intent);
    assert.equal(match?.[4], '0');
  }
);

Then(
  'every check of {string} finished before the push claimed {string}',
  function (project: string, lock: string) {
    const s = state(this);
    assert.deepEqual([project, lock], ['app', DEV]);
    const claimed = last(s).lines.find(line => MARKERS.claimed.re.test(line.text));
    assert.ok(claimed);
    for (const call of gateCalls(device(s, 'A')).slice(s.beforeCalls)) {
      assert.ok(call.ns < BigInt(claimed.at) * 1_000_000n);
    }
  }
);

Then(
  'the push reports {string} held for less than {int} seconds, a generous bound for a loaded test host',
  async function (lock: string, bound: number) {
    assert.equal(lock, DEV);
    assert.ok((await holdSeconds(last(state(this)))) < bound);
  }
);

Then(
  'within {int} seconds of the push exiting, device A records the intent as landed {string}, with {int} seconds checked again',
  async function (seconds: number, landedAs: string, regated: number) {
    const s = state(this);
    await waitFor(
      'landed row',
      seconds * 1000,
      () => landingRows(device(s, 'A'), 'landed').length > 0
    );
    const slots = landingRows(device(s, 'A'), 'landed').at(-1)?.record.classifiedAs ?? [];
    assert.ok(slots.includes(`env:landed_as=${landedAs}`));
    assert.ok(slots.includes(`env:regated_s=${regated}`));
  }
);

Then('dev on the server is the pushed commit', function () {
  const s = state(this);
  assert.equal(remoteDev(s), s.pushed);
});

Then("the change's resume point is gone, and no session holds {string}", function (lock: string) {
  const s = state(this);
  assert.equal(lock, DEV);
  assert.ok(
    !s.intent ||
      runSync(['git', 'show-ref', '--verify', `refs/landing/${s.intent}`], { cwd: s.cwd })
        .status !== 0
  );
  assert.equal(holds(device(s, 'A'), DEV, s.session), false);
});

Then('no check runs, because {string}', function (_why: string) {
  const s = state(this);
  assert.equal(gateCalls(device(s, 'A')).length, s.beforeCalls);
});

Then(
  'the push still claims {string} and reports how long it held it',
  async function (lock: string) {
    assert.equal(lock, DEV);
    const s = state(this);
    hasMarker(s, 'claimed');
    assert.ok((await markers(last(s))).held.length > 0);
  }
);

Then(
  'within {int} seconds of the push exiting, device A records the intent as landed',
  async function (seconds: number) {
    const s = state(this);
    await waitFor(
      'landed row',
      seconds * 1000,
      () => landingRows(device(s, 'A'), 'landed').length > 0
    );
  }
);

Given('the server refuses the next push to dev', function () {
  const s = state(this);
  const hook = join(s.f.origin, 'hooks/pre-receive');
  writeFileSync(hook, '#!/usr/bin/env bash\nexit 1\n');
  chmodSync(hook, 0o755);
});

Then('the push exits non-zero', function () {
  assert.notEqual(last(state(this)).status, 0);
});

Then(
  'within {int} seconds no session holds {string}',
  async function (seconds: number, lock: string) {
    const s = state(this);
    await waitFor(
      'landing lock released',
      seconds * 1000,
      () => !holds(device(s, 'A'), lock, s.session)
    );
  }
);

Then('device A has no landed record for the intent', function () {
  assert.equal(landingRows(device(state(this), 'A'), 'landed').length, 0);
});

Then(
  "the change's resume point is still there, and the intent is still live in device A's queue",
  function () {
    const s = state(this);
    assert.equal(
      runSync(['git', 'show-ref', '--verify', `refs/landing/${s.intent}`], { cwd: s.cwd }).status,
      0
    );
    assert.ok(
      intents(device(s, 'A')).intents.some(row => row.change === s.intent && row.state === 'live')
    );
  }
);

Given(
  'session {string} is moored on device A and holds {string} for {int} seconds',
  async function (session: string, resource: string, ttl: number) {
    const s = state(this);
    await moor(s, 'A', session);
    must(
      await berth(device(s, 'A'), ['claim', resource, '--session', session, '--ttl', String(ttl)]),
      'holder claim'
    );
  }
);

When(
  'while that push waits for {string}, another commit changing {string} lands on dev on the server',
  function (lock: string, path: string) {
    const s = state(this);
    assert.equal(lock, DEV);
    moveBase(s.f, path);
  }
);

When(
  'while that push waits for {string}, an empty commit that changes no file lands on dev on the server',
  function (lock: string) {
    const s = state(this);
    assert.equal(lock, DEV);
    moveBase(s.f, null);
  }
);

Then(
  'the push rebases the change onto the new dev in place, checks {string} once more, and is refused with {string} the rebased commit and {string}',
  function (_project: string, moved: string, again: string) {
    const s = state(this);
    assert.deepEqual([moved, again], ['candidate moved to', 'push again']);
    hasMarker(s, 'moved');
    assert.equal(gateCalls(device(s, 'A')).length - s.beforeCalls, 1);
  }
);

When('session {string} pushes again', async function (session: string) {
  const s = state(this);
  s.priorAtoms = landingRows(device(s, 'A'), 'atom').length;
  await runPush(s, 'A', s.cwd, session);
});

Then(
  "the second push finds its own intent and raises no other, and device A's queue lists the intent once",
  function () {
    const s = state(this);
    assert.equal(landingRows(device(s, 'A'), 'atom').length, s.priorAtoms);
    const rows = intents(device(s, 'A')).intents;
    assert.equal(rows.filter(row => row.change === s.intent).length, 1);
  }
);

Then('{string} is not checked again', function (project: string) {
  const s = state(this);
  assert.equal(project, 'app');
  assert.equal(gateCalls(device(s, 'A')).length, s.beforeCalls);
});

Then(
  'device A records the intent as landed {string}, with more than {int} seconds checked again',
  function (landedAs: string, seconds: number) {
    const slots = landingRows(device(state(this), 'A'), 'landed').at(-1)?.record.classifiedAs ?? [];
    assert.ok(slots.includes(`env:landed_as=${landedAs}`));
    assert.ok(
      slots.some(slot => slot.startsWith('env:regated_s=') && Number(slot.split('=')[1]) > seconds)
    );
  }
);

Then('the landed commit carries exactly one {string} line', function (prefix: string) {
  const s = state(this);
  const body = must(
    runSync(['git', 'log', '-1', '--format=%B', remoteDev(s)], { cwd: s.cwd }),
    'read landed commit'
  ).stdout;
  assert.equal(body.split('\n').filter(line => line.startsWith(prefix)).length, 1);
});

Then(
  'the push rebases the change, whose tree is the one already checked, and is refused with {string} and {string} without checking {string} again',
  function (_moved: string, _again: string, project: string) {
    const s = state(this);
    assert.equal(project, 'app');
    hasMarker(s, 'moved');
    assert.equal(gateCalls(device(s, 'A')).length, s.beforeCalls);
  }
);

Then(
  'the second push lands as a fast-forward and {string} is not checked again',
  function (project: string) {
    const s = state(this);
    assert.equal(project, 'app');
    assert.equal(last(s).status, 0);
    assert.equal(gateCalls(device(s, 'A')).length, s.beforeCalls);
  }
);

Given(
  'session {string} commits a change to {string} with the commit-message hook skipped, so the commit has no Change-Id',
  function (session: string, path: string) {
    const s = state(this);
    s.session = session;
    stageChange(s, 'A', s.cwd, path, false);
    assert.equal(changeId(s.cwd), undefined);
  }
);

When('session {string} pushes it to dev', async function (session: string) {
  const s = state(this);
  await runPush(s, 'A', s.cwd, session);
});

Then(
  'the push prints that the commit has no Change-Id and no place in the queue, naming {string} as the cure',
  function (cure: string) {
    assert.equal(cure, 'git commit --amend');
    hasMarker(state(this), 'noChangeId');
  }
);

Then('the push lands', function () {
  assert.equal(last(state(this)).status, 0);
});

Then('device A records no landing intent for it', function () {
  assert.equal(landingRows(device(state(this), 'A'), 'atom').length, 0);
});

When(
  'the push and its hook are interrupted once the first check line has printed',
  async function () {
    const s = state(this);
    assert.ok(s.pending?.child.pid);
    await waitFor(
      'first gate line',
      60_000,
      () => gateCalls(device(s, 'A')).length > s.beforeCalls
    );
    process.kill(-s.pending.child.pid, 'SIGINT');
    await finishPush(s);
  }
);

Then(
  'the push prints that its intent was withdrawn and names {string} to resume',
  function (resume: string) {
    assert.equal(resume, 'git push origin HEAD:dev');
    hasMarker(state(this), 'interrupted');
  }
);

Then('device A holds a withdraw record for the intent, written by device A', function () {
  const s = state(this);
  const rows = landingRows(device(s, 'A'), 'withdraw');
  assert.ok(rows.length > 0);
  assert.ok((rows.at(-1)?.record.classifiedAs ?? []).includes(`via:${device(s, 'A').did}`));
});

Then("the change's resume point is still there", function () {
  const s = state(this);
  assert.equal(
    runSync(['git', 'show-ref', '--verify', `refs/landing/${s.intent}`], { cwd: s.cwd }).status,
    0
  );
});

Given('the push reaches an epr tool that {string}', function (fault: string) {
  const s = state(this);
  const mode = fault.includes('no command') ? 'no-intents' : 'non-json';
  const wrapper = eprWrapper(s.f.base, mode);
  s.faultEnv = { EPR_BIN: wrapper };
});

Then(
  'the push prints that the queue is unreadable, with the reason, and that it checks against dev',
  function () {
    hasMarker(state(this), 'degraded');
  }
);

Then('no line of the push says the queue is empty', function () {
  assert.doesNotMatch(text(last(state(this))), /queue.*empty/i);
});

Given(
  'device A has two linked worktrees of the repository, {string} and {string}',
  function (a: string, b: string) {
    const s = state(this);
    s.worktrees[a] = addWorktree(device(s, 'A'), a.replace(/^wt-/, ''));
    s.worktrees[b] = addWorktree(device(s, 'A'), b.replace(/^wt-/, ''));
  }
);

Given(
  'session {string} works in {string} and session {string} works in {string}',
  function (first: string, a: string, second: string, b: string) {
    const s = state(this);
    assert.ok(s.worktrees[a] && s.worktrees[b]);
    s.values[first] = a;
    s.values[second] = b;
  }
);

When(
  'session {string} pushes a change to {string} from {string} to dev',
  function (session: string, path: string, worktree: string) {
    const s = state(this);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    stageChange(s, 'A', cwd, path);
    startPush(s, 'A', cwd, session);
  }
);

When(
  'while that push waits for {string}, session {string} pushes a change to {string} from {string} to dev',
  async function (_lock: string, session: string, path: string, worktree: string) {
    const s = state(this);
    const first = s.pending;
    assert.ok(first);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    stageChange(s, 'A', cwd, path);
    s.last = await push(device(s, 'A'), cwd, session);
    s.pushes[session] = s.last;
    s.pending = first;
  }
);

Then(
  "wt-b's push raises its intent with a chain of {int}, naming wt-a's intent, on the tree of wt-a's change",
  function (length: number) {
    const match = hasMarker(state(this), 'intent');
    assert.equal(Number(match[4]), length);
    assert.notEqual(match[5], 'dev');
  }
);

Then(
  "{string} is checked once for wt-b's change, with wt-a's change beneath it",
  function (project: string) {
    assert.equal(project, 'app');
    assert.ok(gateCalls(device(state(this), 'A')).length > 0);
  }
);

Then("wt-a's change lands as a fast-forward", function () {
  assert.equal(state(this).pushes['claude-1']?.status, 0);
});

Then(
  "dev on the server has moved under wt-b's waiting push to wt-a's commit, which is the base wt-b's change was checked on",
  function () {
    const s = state(this);
    assert.notEqual(remoteDev(s), s.f.baseSha);
  }
);

Then(
  "wt-b's push rebases its change onto wt-a's commit, whose tree is the one already checked, and is refused once with {string} and {string}, without checking {string} again",
  function (_moved: string, _again: string, project: string) {
    assert.equal(project, 'app');
    hasMarker(state(this), 'moved');
  }
);

When(
  'session {string} pushes again from {string}',
  async function (session: string, worktree: string) {
    const s = state(this);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    await runPush(s, 'A', cwd, session);
  }
);

Then(
  "wt-b's change lands as a fast-forward and {string} is not checked again",
  function (_project: string) {
    assert.equal(last(state(this)).status, 0);
  }
);

Given(
  'session {string} has pushed a change to {string} from {string}, whose push waits for {string} with its intent live',
  async function (session: string, path: string, worktree: string, _lock: string) {
    const s = state(this);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    stageChange(s, 'A', cwd, path);
    startPush(s, 'A', cwd, session);
    await waitFor(
      'live first intent',
      60_000,
      () => landingRows(device(s, 'A'), 'intent').length > 0
    );
  }
);

Given(
  "session {string} has pushed a change to {string} from {string}, whose intent chains on wt-a's and whose check of {string} ran with wt-a's change beneath it",
  async function (session: string, path: string, worktree: string, _project: string) {
    const s = state(this);
    const first = s.pending;
    const cwd = s.worktrees[worktree];
    assert.ok(first && cwd);
    stageChange(s, 'A', cwd, path);
    s.last = await push(device(s, 'A'), cwd, session);
    s.pushes[session] = s.last;
    s.pending = first;
    s.callMarks.beforeFate = gateCalls(device(s, 'A')).length;
  }
);

When("wt-a's intent {string}", async function (fate: string) {
  const s = state(this);
  if (fate === 'is withdrawn') {
    assert.ok(s.intent);
    must(
      runSync([EPR, 'flow', 'intend', '--withdraw', s.intent, '--root', device(s, 'A').repo], {
        cwd: device(s, 'A').repo,
        env: env(device(s, 'A')),
      }),
      'withdraw intent'
    );
  } else {
    assert.equal(fate, 'lands as predicted');
    must(await berth(device(s, 'A'), ['release', DEV, '--session', 'holder-1']), 'release holder');
    await finishPush(s, 'claude-1');
  }
});

When(
  'session {string} pushes from {string} until its change lands',
  async function (session: string, worktree: string) {
    const s = state(this);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    do await runPush(s, 'A', cwd, session);
    while (last(s).status !== 0 && MARKERS.moved.re.test(text(last(s))));
  }
);

Then(
  "{string} was checked {string} more for wt-b's change, because {string}",
  function (project: string, times: string, _why: string) {
    const s = state(this);
    assert.equal(project, 'app');
    const delta = gateCalls(device(s, 'A')).length - s.callMarks.beforeFate;
    assert.equal(delta, times === 'once' ? 1 : 0);
  }
);

Given("the landing policy's skew window is {int} seconds", function (seconds: number) {
  const policy = JSON.parse(
    readFileSync(join(device(state(this), 'A').repo, 'genesis/agentic/pool-policy.json'), 'utf8')
  );
  assert.equal(policy.berth.channel.max_skew_s, seconds);
});

When(
  'session {string} in {string} and session {string} in {string} each raise an intent on the same dev, {string} apart',
  async function (one: string, wtOne: string, two: string, wtTwo: string, gap: string) {
    const s = state(this);
    for (const [session, wt, path] of [
      [one, wtOne, 'app/a.txt'],
      [two, wtTwo, 'app/b.txt'],
    ] as const) {
      const cwd = s.worktrees[wt];
      assert.ok(cwd);
      stageChange(s, 'A', cwd, path);
      startPush(s, 'A', cwd, session);
      await waitFor(
        `intent from ${session}`,
        60_000,
        () => landingRows(device(s, 'A'), 'raise').length >= (session === one ? 1 : 2)
      );
      if (session === one) await sleep(Number.parseInt(gap, 10) * 1000);
      if (s.pending) process.kill(-(s.pending.child.pid as number), 'SIGINT');
      await finishPush(s, session);
    }
  }
);

Then(
  "device A's raise stamps on the two intents are {string} {int} seconds apart, so the timing the test arranged really happened",
  function (side: string, seconds: number) {
    const rows = landingRows(device(state(this), 'A'), 'raise').slice(-2);
    const at = rows.map(row =>
      Number(
        (row.record.classifiedAs ?? []).find(slot => slot.startsWith('env:at='))?.split('=')[1]
      )
    );
    const gap = Math.abs(at[1] - at[0]);
    assert.equal(side === 'more than' ? gap > seconds * 1000 : gap < seconds * 1000, true);
  }
);

Then('both worktrees put {string} first, and print the same chain', function (_first: string) {
  const s = state(this);
  const chains = Object.values(s.pushes).map(outcome => MARKERS.intent.re.exec(text(outcome))?.[5]);
  assert.equal(new Set(chains).size, 1);
});

Given(
  'session {string} in {string} has a live intent for a change adding {string}',
  async function (session: string, worktree: string, path: string) {
    const s = state(this);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    stageChange(s, 'A', cwd, path);
    startPush(s, 'A', cwd, session);
    await waitFor('left intent', 60_000, () => landingRows(device(s, 'A'), 'intent').length > 0);
  }
);

When(
  'session {string} in {string} pushes a change adding {string}',
  async function (session: string, worktree: string, path: string) {
    const s = state(this);
    const first = s.pending;
    const cwd = s.worktrees[worktree];
    assert.ok(first && cwd);
    stageChange(s, 'A', cwd, path);
    await runPush(s, 'A', cwd, session);
    s.pending = first;
  }
);

Then(
  "{string} is checked twice for wt-b's change: red on top of wt-a's change, because that tree has both {string} and {string}, then green on dev alone",
  function (project: string, left: string, right: string) {
    assert.deepEqual([project, left, right], ['app', 'app/left.txt', 'app/right.txt']);
    assert.ok(gateCalls(device(state(this), 'A')).length >= 2);
  }
);

Then(
  "wt-b's push prints that it dismissed the chain, naming every intent between dev and its predicted base and the dismissal record, and names no single culprit",
  function () {
    const line = hasMarker(state(this), 'dismissed')[0];
    assert.doesNotMatch(line, /culprit/i);
  }
);

Then(
  'the dismissal record holds the key at the predicted base, the key at dev, the failing project, the intents in the chain, when each device was last heard from, and the time',
  function () {
    const slots =
      landingRows(device(state(this), 'A'), 'dismiss').at(-1)?.record.classifiedAs ?? [];
    for (const key of ['key_predicted', 'key_dev', 'project', 'chain', 'heads', 'now'])
      assert.ok(slots.some(slot => slot.startsWith(`env:${key}=`)));
  }
);

Then(
  "wt-a's intent is no longer live in either worktree, because both speak for device A",
  function () {
    const rows = intents(device(state(this), 'A')).intents;
    assert.ok(
      rows.some(row => row.state !== 'live' && row.settled?.some(item => item.kind === 'dismiss'))
    );
  }
);

Then("wt-b's change lands, and dev on the server loses no commit", function () {
  assert.equal(last(state(this)).status, 0);
});

Then('it raises a new intent that names the dismissed one as superseded', function () {
  const atoms = landingRows(device(state(this), 'A'), 'atom');
  assert.ok(
    (atoms.at(-1)?.record.resourceSpec?.classifiedAs ?? []).some(slot =>
      slot.startsWith('supersedes:')
    )
  );
});

Then(
  "its check of {string} fails on top of the new dev, so the conflict is now wt-a's to resolve, and nothing is reverted",
  function (project: string) {
    assert.equal(project, 'app');
    assert.notEqual(last(state(this)).status, 0);
  }
);

When(
  'session {string} in {string} pushes a change that edits a comment in {string} and a line of {string}',
  async function (session: string, worktree: string, judge: string, app: string) {
    const s = state(this);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    writeFileSync(
      join(cwd, judge),
      `${readFileSync(join(cwd, judge), 'utf8')}\n// fixture governance edit\n`
    );
    writeFileSync(join(cwd, app), `${readFileSync(join(cwd, app), 'utf8')}\nfixture\n`);
    must(
      runSync(['git', 'add', judge, app], { cwd, env: { ...process.env, ...identity('A') } }),
      'add governance change'
    );
    must(
      runSync(['git', 'commit', '-q', '-m', 'governance fixture'], {
        cwd,
        env: { ...process.env, ...identity('A') },
      }),
      'commit governance change'
    );
    await runPush(s, 'A', cwd, session);
  }
);

Then(
  "the push prints {string}, judged by the checks at the commit where wt-a's branch left dev, with verdict reuse on",
  function (line: string) {
    assert.equal(line, 'change class: governance');
    const match = hasMarker(state(this), 'governance');
    assert.equal(match[2], 'reuse');
  }
);

When(
  "session {string} in {string} pushes a change to {string} while wt-a's intent is live",
  async function (session: string, worktree: string, path: string) {
    const s = state(this);
    const cwd = s.worktrees[worktree];
    assert.ok(cwd);
    stageChange(s, 'A', cwd, path);
    await runPush(s, 'A', cwd, session);
  }
);

Then(
  "wt-b's push skips wt-a's intent as governance-class and checks against dev alone",
  function () {
    const skipped = hasMarker(state(this), 'skipped');
    assert.equal(skipped[2], 'governance-class');
  }
);

Given(
  "matthew's second device {string} has its own checkout of the repository, its own berth, its own key and its own peer daemon",
  function (label: string) {
    const d = device(state(this), label as 'B');
    assert.ok(d.repo && d.berthDir && d.did && d.daemon);
  }
);

Given('session {string} is moored on device B', async function (session: string) {
  await moor(state(this), 'B', session);
});

Given(
  'session {string} on device A holds {string}',
  async function (session: string, resource: string) {
    const s = state(this);
    await moor(s, 'A', session);
    must(
      await berth(device(s, 'A'), ['claim', resource, '--session', session, '--ttl', '600']),
      'claim across devices'
    );
  }
);

When(
  'session {string} on device B pushes a change to {string} to dev',
  async function (session: string, path: string) {
    const s = state(this);
    stageChange(s, 'B', device(s, 'B').repo, path);
    await runPush(s, 'B', device(s, 'B').repo, session);
  }
);

When(
  'session {string} on device A pushes a change to {string} to dev',
  async function (session: string, path: string) {
    const s = state(this);
    stageChange(s, 'A', device(s, 'A').repo, path);
    await runPush(s, 'A', device(s, 'A').repo, session);
  }
);

Then("B's checks run and the push prints {string} before it is stopped", function (banner: string) {
  const s = state(this);
  assert.equal(banner, MARKERS.gateBanner.format());
  assert.ok(gateCalls(device(s, 'B')).length > s.beforeCalls);
  assert.ok(text(last(s)).includes(banner));
});

Then(
  'the push is stopped at the landing, needing {string}, {string} device A and session {string}',
  function (resource: string, heldBy: string, session: string) {
    const output = text(last(state(this)));
    assert.match(output, new RegExp(`${resource}.*${heldBy}.*${session}`));
  }
);

Then("B's intent is still live in B's queue", function () {
  assert.ok(intents(device(state(this), 'B')).intents.some(row => row.state === 'live'));
});

Given('device A has a live intent for a change device B has never fetched', async function () {
  const s = state(this);
  stageChange(s, 'A', device(s, 'A').repo, 'app/a.txt');
  startPush(s, 'A', device(s, 'A').repo, 'claude-1');
  await waitFor('A intent', 60_000, () => landingRows(device(s, 'A'), 'intent').length > 0);
});

Then(
  "B's push skips A's intent because its commit is not on this device, and checks against dev alone",
  function () {
    assert.equal(hasMarker(state(this), 'skipped')[2], 'bytes not on this device');
  }
);

Given('device A has a live intent', async function () {
  const s = state(this);
  stageChange(s, 'A', device(s, 'A').repo, 'app/a.txt');
  startPush(s, 'A', device(s, 'A').repo, 'claude-1');
  await waitFor('A live intent', 60_000, () => landingRows(device(s, 'A'), 'intent').length > 0);
});

Given(
  "device A's peer daemon stops, and device B last folded A's head more than {int} seconds ago",
  async function (seconds: number) {
    const s = state(this);
    await stopDaemon(device(s, 'A'));
    await sleep((seconds + 1) * 1000);
  }
);

Then("B's push skips A's intent as dormant", function () {
  assert.equal(hasMarker(state(this), 'skipped')[2], 'dormant');
});

Then("B's change lands", function () {
  assert.equal(last(state(this)).status, 0);
});

When(
  "session {string} on device B writes a withdraw record for A's intent",
  function (_session: string) {
    const s = state(this);
    const intent = intents(device(s, 'A')).intents.find(row => row.state === 'live')?.intent;
    assert.ok(intent);
    must(
      runSync([EPR, 'flow', 'intend', '--withdraw', intent, '--root', device(s, 'B').repo], {
        cwd: device(s, 'B').repo,
        env: env(device(s, 'B')),
      }),
      'advisory withdraw'
    );
  }
);

Then(
  "A's intent is still live in device A's queue, and both devices list B's withdraw as advisory",
  function () {
    const s = state(this);
    for (const label of ['A', 'B'] as const) {
      const view = intents(device(s, label));
      assert.ok(view.intents.some(row => row.state === 'live'));
      assert.ok(view.advisory.some(row => row.kind === 'withdraw'));
    }
  }
);

Then(
  "a push from device B skips A's intent, printing an advisory withdraw by device B",
  async function () {
    const s = state(this);
    stageChange(s, 'B', device(s, 'B').repo, 'app/b.txt');
    await runPush(s, 'B', device(s, 'B').repo, 'codex-1');
    assert.match(hasMarker(s, 'skipped')[2], /^advisory withdraw by /);
  }
);

Then("a push of A's change from device A lands", async function () {
  const s = state(this);
  await runPush(s, 'A', device(s, 'A').repo, 'claude-1');
  assert.equal(last(s).status, 0);
});

Given("device A's peer daemon stops", async function () {
  await stopDaemon(device(state(this), 'A'));
});

Given(
  'the time limit on {string} is lowered to {int} seconds for this test',
  function (resource: string, seconds: number) {
    assert.equal(resource, DEV);
    state(this).values.ttl = seconds;
  }
);

Then(
  "after its checks, B's push prints {string} with when it takes effect and how long it waits",
  function (message: string) {
    assert.equal(message, 'override on the record');
    hasMarker(state(this), 'overrideWait');
  }
);

Then('only after that wait does it print that it claimed {string}', function (resource: string) {
  const lines = last(state(this)).lines;
  assert.equal(resource, DEV);
  assert.ok(
    lines.findIndex(line => MARKERS.overrideWait.re.test(line.text)) <
      lines.findIndex(line => MARKERS.claimed.re.test(line.text))
  );
});

Then(
  "B's change lands no later than {int} seconds after the override takes effect",
  function (seconds: number) {
    assert.ok(last(state(this)).ms < (Number(state(this).values.ttl) + 5 + 30 + seconds) * 1000);
  }
);

Then("B's berth ledger holds an override record naming device A", function () {
  const s = state(this);
  assert.ok(
    ledger(device(s, 'B')).some(
      row => row.kind === 'override' && row.overrode === device(s, 'A').did
    )
  );
});

Given("device B's peer daemon is stopped", async function () {
  await stopDaemon(device(state(this), 'B'));
});

Then(
  "the push is refused at the landing, printing that this device's peer daemon is down and naming {string}",
  function (cure: string) {
    assert.equal(cure, 'epr peer serve');
    assert.notEqual(last(state(this)).status, 0);
    hasMarker(state(this), 'daemonDown');
  }
);

Given(
  "device B's peer daemon is restarted with an allow-list that does not admit landing records",
  async function () {
    const s = state(this);
    await stopDaemon(device(s, 'B'));
    const d = device(s, 'B');
    const declared = d.policy as string;
    const policy = JSON.parse(
      readFileSync(isAbsolute(declared) ? declared : join(d.repo, declared), 'utf8')
    );
    delete policy.berth.channel.crosses['dev-landing@1'];
    const path = join(s.f.base, 'policy-without-landing.json');
    writeFileSync(path, `${JSON.stringify(policy, null, 2)}\n`);
    d.policy = path;
    startDaemon(s.f, d);
  }
);

Then("device B's peer status counts A's landing record as refused", function () {
  const status = peerStatus(device(state(this), 'B'));
  assert.ok(JSON.stringify(status).includes('refused'));
});

Then("A's change lands", function () {
  assert.equal(last(state(this)).status, 0);
});
