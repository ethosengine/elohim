/**
 * devices-share-one-berth — two devices of one person in ONE container, each with its own checkout,
 * its own berth store, its own fresh keys (made here, never copied, never printed) and its own
 * `epr peer serve` daemon on loopback, admitted to each other by the real roster ceremony.
 *
 * Every verdict comes from the real binaries: `berth` (copied into each device's checkout, so it
 * folds into that checkout's flow store, the store its daemon serves), the real `epr`, and the real
 * pre-push hook (`.husky/pre-push.bash`, installed as each checkout's own pre-push hook; pushes go
 * to a bare `origin`). Nothing here writes a lease, a head or a verdict by hand:
 *
 *   - "silent": the device's daemon is stopped and the steps wait in real time until the head the
 *     other device last folded from it is old enough (the view has no test clock, and none is
 *     added for this);
 *   - "stamped <time> by device X": the one test clock. The claim runs through berth's own `main`
 *     with that device's clock set, at the moment the claim is stamped, to read the given time of
 *     this run (`12:00:00` is the run's base second, within about a second of the real clock). That
 *     is a device whose clock differs from the real one, which a device may have; the stamp is the
 *     claim's own record, and both devices then compare those recorded values.
 *
 * The `epr` used is the one the @requires:epr-cli guard resolves ($EPR_BIN, else `epr` on PATH).
 * With none the scenario is held (skipped, not failed), as every @requires:epr-cli scenario is; with
 * one it must be newer than every tracked eprfs source, or the scenario fails naming the build
 * command (a stale binary once passed a smoke by luck).
 */
import assert from 'node:assert/strict';
import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

import { After, Before, Given, Then, When } from '@cucumber/cucumber';

import {
  berth,
  checkEpr,
  claimedAt,
  DEV,
  type Device,
  env,
  epr,
  type Fixture,
  foldedCurrentHead,
  git,
  HANDLE,
  holds,
  household,
  type Label,
  leases,
  ledger,
  must,
  other,
  otherDeviceBlock,
  peerStatus,
  refusalLine,
  ROSTER,
  run,
  startDaemon,
  STEWARD,
  stopDaemon,
  TAG,
  verdict,
  waitFor,
} from './devices-share-one-berth.fixture.js';
import { holdWithoutEpr } from './epr-binary.js';

const fixtures = new WeakMap<object, Fixture>();

function state(world: object): Fixture {
  const f = fixtures.get(world);
  assert.ok(f, 'the Background did not run');
  return f;
}

// This file loads before the guard, and cucumber skips every later `Before` once one returns
// 'skipped': so this hook holds the scenario itself, the guard's way, before it checks the binary.
Before({ tags: TAG }, function (scenario) {
  const held = holdWithoutEpr(scenario);
  if (held) return held;
  checkEpr();
  return undefined;
});

After({ tags: TAG }, async function () {
  const f = fixtures.get(this);
  if (!f) return;
  await Promise.all(Object.values(f.pending).map(p => p?.outcome));
  await Promise.all(Object.values(f.devices).map(d => stopDaemon(d)));
  // DEVICES_SHARE_ONE_BERTH_KEEP=1 keeps the fixture (logs, stores; keys stay 0600 and unprinted).
  if (process.env.DEVICES_SHARE_ONE_BERTH_KEEP !== '1')
    rmSync(f.base, { recursive: true, force: true });
});

Given(
  'two devices of {string}, {string} and {string}, each with its own berth, its own key and its own peer daemon',
  { timeout: 120_000 },
  async function (person: string, first: string, second: string) {
    assert.deepEqual([person, first, second], [HANDLE, 'A', 'B']);
    fixtures.set(this, await household());
  }
);

Given("matthew's roster admits both devices", function () {
  const f = state(this);
  for (const d of Object.values(f.devices)) {
    const view = leases(d);
    assert.equal(view.standing, 'device', `${d.label}: ${JSON.stringify(view)}`);
    const others = Object.keys((view.others ?? {}) as object);
    assert.deepEqual(others, [f.devices[other(d.label)].did], `${d.label} admits the other device`);
  }
});

Given(
  'session {string} is moored on device A and session {string} is moored on device B',
  async function (onA: string, onB: string) {
    const f = state(this);
    must(
      await berth(f.devices.A, [
        'moor',
        '--session',
        onA,
        '--model',
        'claude-opus-5-5',
        '--lab',
        'anthropic',
      ]),
      'moor A'
    );
    must(
      await berth(f.devices.B, ['moor', '--session', onB, '--model', 'gpt-5.5', '--lab', 'openai']),
      'moor B'
    );
  }
);

Given(
  "the link between the two devices is up, and each device has folded the other's current head",
  { timeout: 60_000 },
  async function () {
    const { A, B } = state(this).devices;
    await foldedCurrentHead(A, B, 30_000);
    await foldedCurrentHead(B, A, 30_000);
  }
);

// ---------------------------------------------------------------- claims on device A or B

Given(
  'session {string} on device A holds the steward-scoped resource {string}',
  { timeout: 30_000 },
  async function (session: string, resource: string) {
    const { A } = state(this).devices;
    must(await berth(A, ['claim', resource, '--session', session]), `A's claim on ${resource}`);
  }
);

Given(
  'session {string} on device A holds the steward-scoped resource {string} because {string}',
  { timeout: 30_000 },
  async function (session: string, resource: string, because: string) {
    const { A } = state(this).devices;
    // A push holds dev-integration exactly as the pre-push hook claims it; a watcher claims its own.
    const args =
      resource === DEV
        ? ['--class', 'verify', '--ttl', '1800', '--note', 'pre-push to dev']
        : ['--ttl', '3600', '--note', because];
    must(
      await berth(A, ['claim', resource, '--session', session, ...args]),
      `A's claim on ${resource}`
    );
  }
);

Given(
  'device B has folded the head from device A that carries that lease',
  { timeout: 30_000 },
  async function () {
    const { A, B } = state(this).devices;
    await waitFor("B's view holds A's dev-integration lease", 20_000, () => {
      const v = verdict(B, DEV);
      return v?.verdict === 'held_by' && v.device === A.did;
    });
  }
);

When(
  'session {string} on device {word} claims {string}',
  { timeout: 30_000 },
  async function (session: string, label: Label, resource: string) {
    const f = state(this);
    f.last = await berth(f.devices[label], ['claim', resource, '--session', session]);
  }
);

When(
  'session {string} on device {word} claims {string} again',
  { timeout: 30_000 },
  async function (session: string, label: Label, resource: string) {
    const f = state(this);
    f.last = await berth(f.devices[label], ['claim', resource, '--session', session]);
  }
);

When(
  'session {string} on device {word}, on its own judgment, claims {string} with {string}',
  { timeout: 30_000 },
  async function (session: string, label: Label, resource: string, flag: string) {
    const f = state(this);
    f.last = await berth(f.devices[label], ['claim', resource, '--session', session, flag]);
  }
);

Then('the claim exits with code {int}', function (code: number) {
  const o = state(this).last;
  assert.ok(o);
  assert.equal(o.status, code, `${o.stdout}\n${o.stderr}`);
});

Then(
  'the reason line reads "held by" device A and session {string} since the time A claimed it',
  function (session: string) {
    const f = state(this);
    assert.ok(f.last);
    const since = claimedAt(f.devices.A, session, DEV);
    assert.equal(
      refusalLine(f.last, DEV),
      `dev-integration: REFUSED — held by ${f.devices.A.did}/${session} since ${since}`
    );
  }
);

Then(
  "device B's berth status lists that lease under device A, read from B's own fold",
  async function () {
    const { A, B } = state(this).devices;
    const status = must(await berth(B, ['status']), 'berth status on B').stdout;
    const block = otherDeviceBlock(status, A.did).join('\n');
    assert.match(block, new RegExp(`dev-integration {2}held by ${A.did}/claude-1 since `), block);
    assert.match(block, /moored claude-1 since /, block);
  }
);

Then(
  'a claim by {string} with {string} is refused the same way, "held by" naming device A, because an override never overrides a held lease',
  { timeout: 30_000 },
  async function (session: string, flag: string) {
    const f = state(this);
    const o = await berth(f.devices.B, ['claim', DEV, '--session', session, flag]);
    assert.equal(o.status, 3, `${o.stdout}\n${o.stderr}`);
    assert.match(
      refusalLine(o, DEV),
      new RegExp(`REFUSED — held by ${f.devices.A.did}/claude-1 since `)
    );
    assert.ok(!ledger(f.devices.B).some(r => r.kind === 'override'), 'no override was recorded');
  }
);

When('session {string} releases {string}', async function (session: string, resource: string) {
  const f = state(this);
  const holder = (['A', 'B'] as Label[]).find(l => holds(f.devices[l], resource, session));
  assert.ok(holder, `${session} holds no ${resource}`);
  must(await berth(f.devices[holder], ['release', resource, '--session', session]), 'release');
});

When(
  'device B folds the head from device A that carries the release',
  { timeout: 30_000 },
  async function () {
    const { B } = state(this).devices;
    await waitFor(
      "B's view no longer holds dev-integration",
      20_000,
      () => verdict(B, DEV) === undefined
    );
  }
);

Then(
  "the claim waits until device A's head acknowledges it, within {int} seconds, and exits {int}",
  function (seconds: number, code: number) {
    const f = state(this);
    const o = f.last;
    assert.ok(o);
    assert.equal(o.status, code, `${o.stdout}\n${o.stderr}`);
    assert.ok(o.ms <= seconds * 1000 + 1000, `took ${o.ms} ms`);
    assert.match(o.stdout, /dev-integration: claimed by codex-1/);
    // Effective only because A acknowledged the head that carried it.
    assert.deepEqual(verdict(f.devices.B, DEV), {
      verdict: 'effective',
      session: 'codex-1',
    });
  }
);

// ---------------------------------------------------------------- pushes through the pre-push hook

When(
  'session {string} on device B pushes to dev commits that carry {string}',
  { timeout: 60_000 },
  async function (session: string, tag: string) {
    const f = state(this);
    const { B } = f.devices;
    const message =
      tag === 'no build tag'
        ? 'feat: a change from device B'
        : `feat: a change from device B ${tag}`;
    writeFileSync(join(B.repo, 'change.txt'), `${message}\n`);
    must(git(B.repo, ['add', 'change.txt']), 'add');
    must(git(B.repo, ['commit', '-q', '-m', message]), 'commit');
    // The push runs the checkout's real pre-push hook, as session `session` of device B.
    f.push = await run(['git', 'push', 'origin', 'HEAD:dev'], {
      cwd: B.repo,
      env: env(B, { BERTH_SESSION: session }),
    });
  }
);

Then('the pre-push hook stops the push before any check runs', function () {
  const o = state(this).push;
  assert.ok(o);
  const out = `${o.stdout}\n${o.stderr}`;
  assert.notEqual(o.status, 0, out);
  assert.match(out, /\[pre-push\] REFUSED: this push needs /, out);
  // The first check after the claims is the Rust style reach gate; it never started.
  assert.doesNotMatch(out, /rust-style|Rust style|PRE-PUSH GATE/, out);
});

Then(
  'it prints the reason line "held by" device A and session {string} for {string}',
  function (session: string, resource: string) {
    const f = state(this);
    assert.ok(f.push);
    assert.equal(
      refusalLine(f.push, resource),
      `${resource}: REFUSED — held by ${f.devices.A.did}/${session} since ${claimedAt(f.devices.A, session, resource)}`
    );
  }
);

Then('no commit from device B reaches dev', function () {
  const f = state(this);
  const dev = must(git(f.origin, ['rev-parse', 'dev']), 'rev-parse dev').stdout.trim();
  assert.equal(dev, f.baseSha);
});

Then(
  'device B holds none of {string}, {string} or {string} afterwards',
  function (first: string, second: string, third: string) {
    const { B } = state(this).devices;
    const view = leases(B);
    for (const resource of [first, second, third]) {
      assert.ok(STEWARD.has(resource));
      assert.equal(holds(B, resource, 'codex-1'), false, `B still holds ${resource}`);
      const v = view[resource] as Record<string, unknown> | undefined;
      assert.ok(!v || v.verdict === 'held_by', `B's view: ${resource} ${JSON.stringify(v)}`);
    }
  }
);

// ---------------------------------------------------------------- a silent device

Given('no session on either device holds {string}', function (resource: string) {
  for (const d of Object.values(state(this).devices)) {
    assert.equal(verdict(d, resource), undefined, `${d.label}: ${resource} is held`);
  }
});

Given(
  "device A's peer daemon stops, and the last head device B folded from A is {int} minutes old",
  { timeout: 300_000 },
  async function (minutes: number) {
    const { A, B } = state(this).devices;
    await stopDaemon(A);
    // Real elapsed time: the view has no test clock, and the head table is never edited.
    await waitFor(
      `the head B folded from A to be ${minutes} minutes old`,
      minutes * 60_000 + 60_000,
      () => {
        const seen = peerStatus(B).devices.find(x => x.device === A.did);
        return seen?.ageS !== null && seen?.ageS !== undefined && seen.ageS >= minutes * 60 + 1;
      }
    );
  }
);

Then(
  "the claim waits {int} seconds for device A's acknowledgement and exits with code {int}",
  function (seconds: number, code: number) {
    const o = state(this).last;
    assert.ok(o);
    assert.equal(o.status, code, `${o.stdout}\n${o.stderr}`);
    assert.ok(o.ms >= seconds * 1000 - 500 && o.ms <= seconds * 1000 + 3000, `took ${o.ms} ms`);
  }
);

Then(
  'the reason line reads "unconfirmed:" naming device A, last seen about {int} minutes ago',
  function (minutes: number) {
    const f = state(this);
    assert.ok(f.last);
    assert.equal(
      refusalLine(f.last, DEV),
      `dev-integration: REFUSED — unconfirmed: ${f.devices.A.did} last seen ${minutes}m ago`
    );
  }
);

Then('no line of the output says the resource is free', function () {
  const o = state(this).last;
  assert.ok(o);
  assert.doesNotMatch(`${o.stdout}\n${o.stderr}`, /\bfree\b/i);
});

Then('device B holds no lease on {string}', function (resource: string) {
  const { B } = state(this).devices;
  assert.equal(holds(B, resource, 'codex-1'), false);
  assert.equal(verdict(B, resource), undefined);
});

Then(
  'a claim by {string} on the container-scoped resource {string} is effective at once',
  async function (session: string, resource: string) {
    const { B } = state(this).devices;
    const o = must(await berth(B, ['claim', resource, '--session', session]), `claim ${resource}`);
    assert.ok(o.ms < 3000, `took ${o.ms} ms`);
    assert.match(o.stdout, new RegExp(`^${resource}: claimed by ${session}`));
  }
);

// ---------------------------------------------------------------- an override of a silent device

Then(
  "the claim waits {int} seconds for device A's acknowledgement, then takes the lease anyway and exits {int}",
  function (seconds: number, code: number) {
    const f = state(this);
    const o = f.last;
    assert.ok(o);
    assert.equal(o.status, code, `${o.stdout}\n${o.stderr}`);
    assert.ok(o.ms >= seconds * 1000 - 500, `took ${o.ms} ms`);
    assert.match(o.stdout, /dev-integration: claimed over a silent device by codex-1/);
    assert.match(
      o.stdout,
      new RegExp(
        String.raw`unconfirmed: ${f.devices.A.did} last seen \d+[smh] ago — override on the record`
      )
    );
  }
);

Then(
  "device B's berth records an override naming device B and session {string}",
  function (session: string) {
    const { A, B } = state(this).devices;
    const row = ledger(B)
      .filter(r => r.kind === 'override' && r.resource === DEV)
      .pop();
    assert.ok(row, "no override row in B's berth");
    assert.equal(row.session, session);
    assert.equal(row.holder, session);
    assert.equal(row.overrode, A.did);
    // It is B's own record in B's channel store: the store B's daemon serves, so it crosses as B's.
    const store = readFileSync(join(B.repo, '.eprfs/status/flows.jsonl'), 'utf8')
      .split('\n')
      .filter(line => line.includes('reason:override') && line.includes(`env:session=${session}`));
    assert.ok(
      store.length > 0 && store.every(line => !line.includes('"via:')),
      'the override is on B as its own'
    );
  }
);

When(
  "device A's peer daemon starts again and folds device B's next head",
  { timeout: 60_000 },
  async function () {
    const f = state(this);
    const { A, B } = f.devices;
    startDaemon(f, A);
    await waitFor("A's view holds B's lease on dev-integration", 45_000, () => {
      const v = verdict(A, DEV);
      return v?.verdict === 'held_by' && v.device === B.did;
    });
  }
);

Then(
  "device A's berth status shows B's lease on {string} marked as an override of A's silence, naming device B and session {string}",
  async function (resource: string, session: string) {
    const { A, B } = state(this).devices;
    const status = must(await berth(A, ['status']), 'berth status on A').stdout;
    const block = otherDeviceBlock(status, B.did).join('\n');
    assert.match(
      block,
      new RegExp(
        String.raw`${resource} {2}held by ${B.did}/${session} since \S+ — an override of this device's silence`
      ),
      block
    );
  }
);

Then(
  'a claim by {string} on {string} on device A exits {int} with the reason line "held by" device B and session {string}',
  { timeout: 30_000 },
  async function (session: string, resource: string, code: number, holder: string) {
    const { A, B } = state(this).devices;
    const o = await berth(A, ['claim', resource, '--session', session]);
    assert.equal(o.status, code, `${o.stdout}\n${o.stderr}`);
    assert.equal(
      refusalLine(o, resource),
      `${resource}: REFUSED — held by ${B.did}/${holder} since ${claimedAt(B, holder, resource)}`
    );
  }
);

Then(
  'once {string} releases {string} and device A folds that release, a claim by {string} on device A exits {int}',
  { timeout: 60_000 },
  async function (holder: string, resource: string, session: string, code: number) {
    const { A, B } = state(this).devices;
    must(await berth(B, ['release', resource, '--session', holder]), 'release on B');
    await waitFor(
      "A's view no longer holds B's lease",
      30_000,
      () => verdict(A, resource) === undefined
    );
    const o = await berth(A, ['claim', resource, '--session', session]);
    assert.equal(o.status, code, `${o.stdout}\n${o.stderr}`);
  }
);

// ---------------------------------------------------------------- two concurrent claims

Given("device A's signing key sorts lower than device B's", function () {
  const { A, B } = state(this).devices;
  assert.ok(A.did < B.did, `${A.did} < ${B.did}`);
});

Given('the link between the two devices is down', async function () {
  const { A, B } = state(this).devices;
  await Promise.all([stopDaemon(A), stopDaemon(B)]);
});

/** Run berth's own `main` with this device's clock reading `target` (unix s) when the claim is stamped. */
const STAMPED_CLAIM = `
import runpy, sys, time
target, path = float(sys.argv[1]), sys.argv[2]
ns = runpy.run_path(path)
Berth = ns["Berth"]
new_lease = Berth._new_lease
def stamped(self, *args, **kwargs):
    real = time.time
    offset = target - real()
    self.now = lambda: real() + offset
    return new_lease(self, *args, **kwargs)
Berth._new_lease = stamped
sys.exit(ns["main"]([path] + sys.argv[3:]))
`;

function clockOffset(time: string): number {
  const match = /^12:00:(\d{2})\.(\d{3})$/.exec(time);
  assert.ok(match, `a time of the run's base minute: ${time}`);
  return Number(match[1]) + Number(match[2]) / 1000;
}

When(
  'session {string} on device {word} claims {string}, stamped {string} by device {word}',
  async function (session: string, label: Label, resource: string, time: string, stamper: Label) {
    const f = state(this);
    assert.equal(label, stamper);
    const d = f.devices[label];
    f.clockBase ??= Math.floor(Date.now() / 1000) + 1;
    const target = (f.clockBase + clockOffset(time)).toFixed(3);
    f.pending[label] = {
      started: Date.now(),
      outcome: run(
        [
          'python3',
          '-c',
          STAMPED_CLAIM,
          target,
          join(d.repo, 'genesis/agentic/bin/berth'),
          'claim',
          resource,
          '--session',
          session,
        ],
        { cwd: d.repo, env: env(d) }
      ),
    };
    // The claim is written (pre-check, then the lease) before the next step brings the link back.
    await waitFor(`${label}'s claim is written`, 5000, () =>
      ledger(d).some(r => r.kind === 'claim' && r.session === session && r.resource === resource)
    );
    const row = ledger(d)
      .filter(r => r.kind === 'claim' && r.session === session)
      .pop();
    assert.equal(Number(row?.claimed_at).toFixed(3), target, `stamped ${time} by device ${label}`);
  }
);

When(
  "the link comes back before either claim's {int}-second wait ends, and each device folds the other's head",
  { timeout: 60_000 },
  async function (seconds: number) {
    const f = state(this);
    const { A, B } = f.devices;
    startDaemon(f, A);
    startDaemon(f, B);
    f.linkBackAt = Date.now();
    for (const label of ['A', 'B'] as Label[]) {
      const p = f.pending[label];
      assert.ok(p);
      assert.ok(
        f.linkBackAt - p.started < seconds * 1000,
        `the link came back ${f.linkBackAt - p.started} ms after ${label}'s claim`
      );
    }
    f.outcomes.A = await f.pending.A?.outcome;
    f.outcomes.B = await f.pending.B?.outcome;
    f.pending = {};
    await foldedCurrentHead(A, B);
    await foldedCurrentHead(B, A);
  }
);

function winnerLabel(winner: string): Label {
  const match = /^device ([AB])$/.exec(winner);
  assert.ok(match, winner);
  return match[1] as Label;
}

Then(
  'the claim on {string} exits {int} and the claim on the other device exits {int}',
  function (winner: string, won: number, lost: number) {
    const f = state(this);
    const w = winnerLabel(winner);
    const ow = f.outcomes[w];
    const ol = f.outcomes[other(w)];
    assert.ok(ow && ol);
    assert.equal(ow.status, won, `${winner}: ${ow.stdout}\n${ow.stderr}`);
    assert.equal(ol.status, lost, `the other: ${ol.stdout}\n${ol.stderr}`);
  }
);

const SESSIONS: Record<Label, string> = { A: 'claude-1', B: 'codex-1' };

Then(
  'the losing claim\'s reason line reads "held by" {string} and names its session',
  function (winner: string) {
    const f = state(this);
    const w = winnerLabel(winner);
    const lost = f.outcomes[other(w)];
    assert.ok(lost);
    assert.match(
      refusalLine(lost, DEV),
      new RegExp(
        String.raw`^dev-integration: REFUSED — held by ${f.devices[w].did}/${SESSIONS[w]} since \S+Z$`
      )
    );
  }
);

Then('the losing device holds no lease on {string}', function (resource: string) {
  const f = state(this);
  const loser = (['A', 'B'] as Label[]).find(l => f.outcomes[l]?.status !== 0);
  assert.ok(loser);
  const d = f.devices[loser];
  assert.equal(holds(d, resource, SESSIONS[loser]), false);
  const v = verdict(d, resource);
  assert.ok(v?.verdict === 'held_by', `the loser's view: ${JSON.stringify(v)}`);
});

Then('the berth status on device A and on device B names the same holder', async function () {
  const f = state(this);
  const w = (['A', 'B'] as Label[]).find(l => f.outcomes[l]?.status === 0);
  assert.ok(w);
  const winner = f.devices[w];
  const loser = f.devices[other(w)];
  // On the winner: its own lease, held by its session, and no other device's.
  const own = must(await berth(winner, ['status']), 'status').stdout;
  assert.match(own, new RegExp(String.raw`\n {4}dev-integration held by ${SESSIONS[w]} for `), own);
  assert.doesNotMatch(otherDeviceBlock(own, loser.did).join('\n'), /dev-integration/);
  // On the loser: the same holder, read from its fold.
  const theirs = must(await berth(loser, ['status']), 'status').stdout;
  assert.match(
    otherDeviceBlock(theirs, winner.did).join('\n'),
    new RegExp(`dev-integration {2}held by ${winner.did}/${SESSIONS[w]} since `)
  );
  assert.match(theirs, /\n {4}dev-integration free/, theirs);
});

// ---------------------------------------------------------------- a retired device
//
// The Contest is `epr actor device contest` on A; A's daemon reloads its roster on its own tick,
// closes what B held open and refuses B as it refuses a stranger. These steps assert what an
// operator reads (`epr peer status` on A counts B's refused connections); the PROOF of the
// clause "drops device B's connections and refuses new ones exactly as it refuses a stranger's"
// — an open connection closed with the not-admitted code, new ones closed with the stranger's
// code, the same tier answer as a stranger's — is eprfs-peer's real-endpoint test
// `a_contested_device_is_disconnected_refused_like_a_stranger_and_counted` (tests/daemon.rs).

/** `epr peer status --json` on `d`: the refused connections it reports for `device`. */
function refusedFrom(d: Device, device: string): number {
  const status = JSON.parse(
    must(epr(d, ['peer', 'status', '--json', '--root', d.repo]), 'peer status').stdout
  ) as { refusedConnections?: { device: string | null; count: number }[] };
  return (status.refusedConnections ?? [])
    .filter(r => r.device === device)
    .reduce((sum, r) => sum + r.count, 0);
}

Given(
  'device B is an old workspace whose work matthew has moved to device A, and it is still running',
  function () {
    const { B } = state(this).devices;
    assert.ok(B.daemon?.exitCode === null, "B's daemon is running");
  }
);

Given(
  'session {string} on device B still holds {string}, left over from a deploy it was watching',
  { timeout: 30_000 },
  async function (session: string, resource: string) {
    const { B } = state(this).devices;
    must(
      await berth(B, [
        'claim',
        resource,
        '--session',
        session,
        '--ttl',
        '3600',
        '--note',
        'watching a deploy',
      ]),
      `B's claim on ${resource}`
    );
  }
);

Given(
  'device A has folded that lease, so a claim by {string} on {string} on device A is refused naming device B',
  { timeout: 60_000 },
  async function (session: string, resource: string) {
    const { A, B } = state(this).devices;
    await waitFor(
      `A's view holds B's ${resource}`,
      20_000,
      () => verdict(A, resource)?.device === B.did
    );
    const o = await berth(A, ['claim', resource, '--session', session]);
    assert.equal(o.status, 3, `${o.stdout}\n${o.stderr}`);
    assert.match(refusalLine(o, resource), new RegExp(`held by ${B.did}/`));
  }
);

When(
  "matthew, on device A, contests device B's entry, signing the Contest row on A and writing it into A's own roster file",
  function () {
    const { A, B } = state(this).devices;
    const out = must(
      epr(A, [
        'actor',
        'device',
        'contest',
        '--device',
        B.did,
        '--basis',
        "an old workspace; matthew's work moved to device A",
        '--root',
        A.repo,
      ]),
      'device contest on A'
    );
    const outcome = JSON.parse(out.stdout) as { device: string; by: string };
    assert.deepEqual([outcome.device, outcome.by], [B.did, A.did]);
    const rows = readFileSync(join(A.repo, ROSTER), 'utf8');
    assert.match(rows, /"kind":"contest"/);
  }
);

Then(
  "device A's berth status no longer lists device B or its lease on {string}, with no pull and no restart",
  async function (resource: string) {
    const { A, B } = state(this).devices;
    const status = must(await berth(A, ['status']), 'berth status on A').stdout;
    assert.ok(!status.includes(B.did), status);
    assert.equal(verdict(A, resource), undefined);
    // No pull: origin's dev is where the fixture left it. No restart: A's daemon started once.
    const dev = must(git(state(this).origin, ['rev-parse', 'dev']), 'rev-parse dev').stdout.trim();
    assert.equal(dev, state(this).baseSha);
    assert.equal(A.daemonStarts, 1);
  }
);

Then(
  'a claim by {string} on {string} on device A exits {int} at once, with no other admitted device to wait for',
  { timeout: 30_000 },
  async function (session: string, resource: string, code: number) {
    const { A } = state(this).devices;
    const o = await berth(A, ['claim', resource, '--session', session]);
    assert.equal(o.status, code, `${o.stdout}\n${o.stderr}`);
    assert.ok(o.ms < 3000, `took ${o.ms} ms`);
  }
);

Then(
  "device A drops device B's connections and refuses new ones exactly as it refuses a stranger's",
  { timeout: 120_000 },
  async function () {
    const { A, B } = state(this).devices;
    // B keeps trying (its gossip redials A, and it fetches A's newer heads from A's tier): each
    // connection A closes or refuses is counted against B's node id.
    await waitFor("A to refuse B's connections", 90_000, () => refusedFrom(A, B.did) >= 1);
    const listed = peerStatus(A).devices.map(x => x.device);
    assert.ok(!listed.includes(B.did), `A still lists B: ${listed.join(', ')}`);
    assert.ok(A.daemon?.exitCode === null && A.daemonStarts === 1, "A's daemon kept running");
  }
);

Then(
  '{string} on device A counts the refused connections from device B',
  function (command: string) {
    assert.equal(command, 'epr peer status');
    const { A, B } = state(this).devices;
    assert.ok(refusedFrom(A, B.did) >= 1);
    const text = must(epr(A, ['peer', 'status', '--root', A.repo]), 'peer status').stdout;
    assert.match(text, /refused connections {2}\S+: \d+/, text);
  }
);

Then(
  'a push to dev from session {string} on device B is refused with the reason line "unconfirmed:" naming device A, because A no longer acknowledges B',
  { timeout: 60_000 },
  async function (session: string) {
    const f = state(this);
    const { A, B } = f.devices;
    writeFileSync(join(B.repo, 'change.txt'), 'a change from a retired device\n');
    must(git(B.repo, ['add', 'change.txt']), 'add');
    must(git(B.repo, ['commit', '-q', '-m', 'feat: a change from device B']), 'commit');
    const o = await run(['git', 'push', 'origin', 'HEAD:dev'], {
      cwd: B.repo,
      env: env(B, { BERTH_SESSION: session }),
    });
    assert.notEqual(o.status, 0);
    assert.match(refusalLine(o, DEV), new RegExp(`REFUSED — unconfirmed: ${A.did} last seen `));
  }
);

Then(
  "device A's next push to dev is not blocked, and it carries the Contest row to the repository",
  { timeout: 60_000 },
  async function () {
    const f = state(this);
    const { A } = f.devices;
    must(git(A.repo, ['add', ROSTER]), 'add roster');
    must(git(A.repo, ['commit', '-q', '-m', 'roster: retire device B']), 'commit');
    const o = await run(['git', 'push', 'origin', 'HEAD:dev'], {
      cwd: A.repo,
      env: env(A, { BERTH_SESSION: 'claude-1' }),
    });
    assert.equal(o.status, 0, `${o.stdout}\n${o.stderr}`);
    assert.match(`${o.stdout}\n${o.stderr}`, new RegExp(`${DEV}: claimed by claude-1`));
    const carried = must(git(f.origin, ['show', `dev:${ROSTER}`]), 'show roster on dev').stdout;
    assert.match(carried, /"kind":"contest"/);
    // The push held dev-integration only while it ran.
    assert.equal(holds(A, DEV, 'claude-1'), false);
  }
);

Then(
  "device A's peer daemon was never restarted, so the sessions moored on A and the leases they hold were not interrupted",
  function () {
    const { A } = state(this).devices;
    assert.equal(A.daemonStarts, 1);
    assert.ok(A.daemon?.exitCode === null);
    assert.ok(holds(A, 'alpha-fleet', 'claude-1'));
  }
);
