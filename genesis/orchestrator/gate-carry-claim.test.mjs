import { test } from 'node:test';
import assert from 'node:assert/strict';
import { landerWait } from './gate-carry-claim.mjs';

function fixture(overrides = {}) {
  let elapsed = 0;
  let jump = 0;
  const calls = [], lines = [], sleeps = [];
  const wait = { decision: 'wait', until: 1100, by: 'did:carrier', reason: 'carried' };
  const deps = {
    exec: (args, options) => { calls.push({ args, options }); return { status: 0, stdout: JSON.stringify(wait) }; },
    now: () => 1000 + elapsed + jump,
    monotonic: () => elapsed,
    sleep: seconds => { sleeps.push(seconds); elapsed += seconds; },
    log: line => lines.push(line),
    verdict: (project, closure) => { assert.equal(project, 'p'); assert.equal(closure, 'k'); return []; },
    ...overrides,
  };
  return { deps, calls, lines, sleeps, wait, jump: n => { jump = n; },
    run: (options = {}) => landerWait({ intent: 'intent-cid', project: 'p', closure: 'k', mode: 'on', deps, ...options }) };
}

test('rule 1: off spawns nothing; shadow asks once, prints one line, and waits zero', () => {
  const f = fixture();
  assert.equal(f.run({ mode: 'off' }).outcome, 'off');
  assert.equal(f.calls.length, 0);
  assert.equal(f.lines.length, 0);
  assert.equal(f.run({ mode: 'shadow' }).waitedS, 0);
  assert.equal(f.calls.length, 1);
  assert.equal(f.lines.length, 1);
  assert.match(f.lines[0], /would have waited for did:carrier until 1100/);
  assert.deepEqual(f.sleeps, []);
  assert.deepEqual(f.calls[0], { args: ['flow', 'intend', '--carry-decision', 'intent-cid', '--project', 'p',
    '--closure', 'k', '--role', 'lander', '--max-wait-s', '1800', '--json'],
  options: { encoding: 'utf8', timeout: 5000 } });
});

for (const success of [true, false]) test(`rule 2: a ${success ? 'green' : 'red'} verdict ends the wait without passing a gate`, () => {
  let reads = 0;
  const f = fixture({ verdict: () => ++reads === 2 ? [{ success }] : [] });
  const result = f.run();
  assert.equal(result.outcome, 'verdict-arrived');
  assert.equal(result.waitedS, 3);
  assert.equal(f.calls.length, 2);
  assert.equal(f.lines.length, 1);
  assert.equal(result.by, 'did:carrier');
});

test('rule 3: the earliest carry, own, or hard bound ends the wait', () => {
  for (const [options, seconds, outcome] of [
    [{}, 100, 'carry-lapsed'],
    [{ ownStopAt: 1005 }, 5, 'own-bound-passed'],
    [{ hardCapS: 7 }, 7, 'own-bound-passed'],
    [{ ownStopAt: 999 }, 0, 'own-bound-passed'],
  ]) {
    const result = fixture().run(options);
    assert.equal(result.waitedS, seconds);
    assert.equal(result.outcome, outcome);
  }
  const f = fixture();
  f.wait.until = 10000;
  assert.equal(f.run().waitedS, 1800);
});

test('rule 3: a replay with a later until and a backward clock jump never extend the entry deadline', () => {
  const f = fixture();
  f.wait.until = 1006;
  const sleep = f.deps.sleep;
  f.deps.sleep = seconds => { sleep(seconds); f.wait.until = 9000; f.jump(-5000); };
  assert.equal(f.run().waitedS, 6);
});

test('rule 3: a shorter re-read bound and a forward clock jump end waiting', () => {
  const f = fixture();
  const sleep = f.deps.sleep;
  f.deps.sleep = seconds => { sleep(seconds); f.wait.until = 1004; };
  assert.equal(f.run().waitedS, 4);
  const g = fixture();
  const gsleep = g.deps.sleep;
  g.deps.sleep = seconds => { gsleep(seconds); g.jump(1000); };
  assert.equal(g.run({ ownStopAt: 1050 }).outcome, 'own-bound-passed');
});

test('rule 4: every poll re-asks; a released, lapsed, settled, or silent carry runs here', () => {
  for (const reason of ['carry released', 'carry lapsed', 'settled', 'carrier silent']) {
    const f = fixture();
    const sleep = f.deps.sleep;
    f.deps.sleep = seconds => { sleep(seconds); Object.assign(f.wait, { decision: 'run-here', reason }); };
    const result = f.run();
    assert.equal(result.outcome, 'carry-lapsed');
    assert.equal(result.reason, reason);
    assert.equal(result.waitedS, 3);
    assert.equal(f.calls.length, 2);
  }
});

test('rule 5: no landing intent runs alone with no effects, including the default mode', () => {
  assert.deepEqual(landerWait({}), { waitedS: 0, outcome: 'ran-here-no-carry', reason: 'no landing intent' });
});

test('rule 6: the contract only reads decisions and verdicts, sleeps and logs; no record or lease effect exists', () => {
  const f = fixture();
  assert.equal(f.run({ hardCapS: 1 }).waitedS, 1);
  assert.ok(f.calls.every(call => call.args.slice(0, 3).join(' ') === 'flow intend --carry-decision'));
});

test('NotCarried runs here immediately and records the native reason', () => {
  const f = fixture();
  Object.assign(f.wait, { decision: 'run-here', reason: 'NotCarried' });
  assert.equal(f.run().reason, 'NotCarried');
  assert.deepEqual(f.sleeps, []);
});

test('unavailable decisions fail open with the reason: malformed, nonzero, timeout, and missing binary', () => {
  for (const result of [
    { status: 0, stdout: 'not JSON' },
    { status: 0, stdout: '{}' },
    { status: 0, stdout: '{"decision":"wait","until":"later","by":"x","reason":"x"}' },
    { status: 0, stdout: '{"decision":"other","reason":"x"}' },
    { status: 2, stdout: '' },
    { status: null, error: new Error('ETIMEDOUT') },
    { status: null, error: new Error('ENOENT') },
  ]) {
    const f = fixture({ exec: () => result });
    const answer = f.run();
    assert.equal(answer.outcome, 'decision-unavailable');
    assert.match(answer.reason, /carry decision unavailable:/);
    assert.equal(answer.waitedS, 0);
    assert.deepEqual(f.sleeps, []);
    assert.equal(f.lines.length, 1);
  }
  assert.match(fixture({ exec: () => { throw new Error('missing binary'); } }).run().reason, /missing binary/);
});

test('an unavailable re-read stops an existing wait', () => {
  const f = fixture();
  const sleep = f.deps.sleep;
  f.deps.sleep = seconds => { sleep(seconds); f.wait.decision = 'invalid'; };
  assert.equal(f.run().outcome, 'decision-unavailable');
});
