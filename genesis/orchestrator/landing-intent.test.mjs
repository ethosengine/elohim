import { test } from 'node:test';
import assert from 'node:assert/strict';
import { findLandingIntent } from './landing-intent.mjs';

const HEAD = 'a'.repeat(40);
const CHANGE = `I${'b'.repeat(40)}`;
const ONE = 'bafyreiintentone';
const TWO = 'bafyreiintenttwo';

function intent(id, change = CHANGE) {
  return { intent: id, change, state: 'live' };
}

function fixture({ intents = [intent(ONE)], misses = [], message = `subject\n\nChange-Id: ${CHANGE}\n`, peer = null } = {}) {
  const calls = [];
  const env = { PATH: process.env.PATH, EPR_BIN: process.execPath };
  const output = peer ?? { intents, misses, error: null };
  const spawn = (command, args, options) => {
    calls.push({ command, args, options });
    if (command === 'git' && args[0] === 'rev-parse') return { status: 1, stdout: '', stderr: '' };
    if (command === 'git' && args[0] === 'show') return { status: 0, stdout: message, stderr: '' };
    if (args[0] === 'peer') {
      return { status: 0, stdout: typeof output === 'string' ? output : JSON.stringify(output), stderr: '' };
    }
    throw new Error(`unexpected spawn: ${command} ${args.join(' ')}`);
  };
  return { calls, env, spawn };
}

function find(overrides = {}) {
  const fx = fixture(overrides);
  return { ...findLandingIntent({ head: HEAD, root: '/checkout', env: fx.env, spawn: fx.spawn }), fx };
}

test('an open miss naming the pushed head wins before Change-Id', () => {
  const { intent: found, reason, fx } = find({
    intents: [intent(ONE, `I${'1'.repeat(40)}`), intent(TWO, CHANGE)],
    misses: [{ intent: ONE, commit: HEAD }],
  });
  assert.equal(found, ONE);
  assert.match(reason, /matched pushed head/);
  assert.equal(fx.calls.filter(call => call.args[0] === 'peer').length, 1);
  assert.equal(fx.calls.some(call => call.args[0] === 'show'), false);
});

test('Change-Id matches when no open miss names the head', () => {
  const { intent: found, reason, fx } = find();
  assert.equal(found, ONE);
  assert.equal(reason, `matched Change-Id ${CHANGE}`);
  assert.equal(fx.calls.filter(call => call.args[0] === 'peer').length, 1);
});

test('more than one candidate at the same rank is ambiguous', () => {
  const { intent: found, reason } = find({
    intents: [intent(ONE), intent(TWO)],
    misses: [{ intent: ONE, commit: HEAD }, { intent: TWO, commit: HEAD }],
  });
  assert.equal(found, null);
  assert.match(reason, /^2 open intents match pushed head/);
});

test('no head or Change-Id candidate returns null', () => {
  const other = `I${'c'.repeat(40)}`;
  const { intent: found, reason } = find({ intents: [intent(ONE, other)] });
  assert.equal(found, null);
  assert.equal(reason, `no open intent matches pushed head or Change-Id ${CHANGE}`);
});

test('a timed-out intent read returns null and uses the configured bound', () => {
  const fx = fixture();
  fx.env.LANDING_INTENT_TIMEOUT_S = '2';
  fx.spawn = (command, args, options) => {
    fx.calls.push({ command, args, options });
    if (command === 'git') return { status: 1, stdout: '', stderr: '' };
    return { status: null, stdout: '', stderr: '', error: Object.assign(new Error('timed out'), { code: 'ETIMEDOUT' }) };
  };
  const found = findLandingIntent({ head: HEAD, root: '/checkout', env: fx.env, spawn: fx.spawn });
  assert.equal(found.intent, null);
  assert.match(found.reason, /timed out after 2 s/);
  assert.equal(fx.calls.find(call => call.args[0] === 'peer').options.timeout, 2000);
});

test('a non-zero intent read returns null', () => {
  const fx = fixture();
  fx.spawn = (command, args, options) => {
    fx.calls.push({ command, args, options });
    if (command === 'git') return { status: 1, stdout: '', stderr: '' };
    return { status: 7, stdout: '', stderr: 'peer store unavailable\n' };
  };
  const found = findLandingIntent({ head: HEAD, root: '/checkout', env: fx.env, spawn: fx.spawn });
  assert.deepEqual(found, { intent: null, reason: 'intent read failed: peer store unavailable' });
});

test('unparseable intent JSON returns null', () => {
  const { intent: found, reason } = find({ peer: '{broken' });
  assert.equal(found, null);
  assert.equal(reason, 'intent read returned unparseable JSON');
});
