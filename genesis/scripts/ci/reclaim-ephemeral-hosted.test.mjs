import test from 'node:test';
import assert from 'node:assert/strict';
import { selectReclaimable, run, parseArgs } from './reclaim-ephemeral-hosted.mjs';

const NOW = Date.UTC(2026, 9, 29, 12, 0, 0);
const H = 3_600_000;
const u = (identifier, isActive, key = 'k-' + identifier) => ({ identifier, agentPubKey: key, isActive });

test('inactive prefixed users are reclaimable (both prefixes, with and without @)', () => {
  const r = selectReclaimable([u('e2e-abc', false), u('portal-a2o-x@y.z', false)], NOW, 6);
  assert.equal(r.reclaimable.length, 2);
});

test('old embedded timestamp active user is reclaimable; fresh is skipped', () => {
  const old = u(`e2e-${NOW - 7 * H}@x`, true);
  const fresh = u(`e2e-${NOW - 1 * H}@x`, true);
  const r = selectReclaimable([old, fresh], NOW, 6);
  assert.deepEqual(r.reclaimable, [old]);
  assert.deepEqual(r.skippedActive, [fresh]);
});

test('min-age-hours is configurable', () => {
  const x = u(`e2e-${NOW - 2 * H}`, true);
  assert.equal(selectReclaimable([x], NOW, 1).reclaimable.length, 1);
  assert.equal(selectReclaimable([x], NOW, 3).reclaimable.length, 0);
});

test('active candidate without timestamp is skipped', () => {
  const r = selectReclaimable([u('e2e-nots', true)], NOW, 6);
  assert.equal(r.candidates.length, 1);
  assert.equal(r.reclaimable.length, 0);
  assert.equal(r.skippedActive.length, 1);
});

test('non-prefixed identifiers are never chosen, even inactive or old', () => {
  const users = [
    u('matthew', false),
    u(`alice-${NOW - 100 * H}`, false),
    u('someone@e2e-host.com', false),
    u(`x-e2e-${NOW - 100 * H}`, true),
  ];
  const r = selectReclaimable(users, NOW, 6);
  assert.equal(r.candidates.length, 0);
  assert.equal(r.reclaimable.length, 0);
});

function fakeFetch(users, calls, failDelete = false) {
  return async (url, init) => {
    calls.push({ url, method: init.method, key: init.headers['X-API-Key'] });
    if (init.method === 'GET') return { ok: true, status: 200, json: async () => ({ total: users.length, users }) };
    return failDelete ? { ok: false, status: 500 } : { ok: true, status: 200 };
  };
}
const quiet = () => {};

test('run deletes only reclaimable, encodes key, sends key header', async () => {
  const calls = [];
  const users = [u('e2e-a', false, 'uhCAk/a+b'), u('e2e-live', true), u('real', false)];
  const r = await run({ baseUrl: 'https://d.example/', dryRun: false, minAgeHours: 6, env: { API_KEY_ADMIN: 'sek' }, fetchImpl: fakeFetch(users, calls), now: NOW, log: quiet });
  assert.deepEqual(r, { listed: 3, candidates: 2, freed: 1, skippedActive: 1, failed: 0 });
  const dels = calls.filter((c) => c.method === 'DELETE');
  assert.equal(dels.length, 1);
  assert.equal(dels[0].url, 'https://d.example/admin/hosted-users/' + encodeURIComponent('uhCAk/a+b'));
  assert.ok(calls.every((c) => c.key === 'sek'));
});

test('dry-run issues no DELETE', async () => {
  const calls = [];
  const r = await run({ baseUrl: 'https://d', dryRun: true, minAgeHours: 6, env: { API_KEY_ADMIN: 'k' }, fetchImpl: fakeFetch([u('e2e-a', false)], calls), now: NOW, log: quiet });
  assert.equal(calls.filter((c) => c.method === 'DELETE').length, 0);
  assert.equal(r.freed, 0);
});

test('missing key skips without any request', async () => {
  const calls = [];
  const lines = [];
  const r = await run({ baseUrl: 'https://d', dryRun: false, minAgeHours: 6, env: {}, fetchImpl: fakeFetch([], calls), now: NOW, log: (l) => lines.push(l) });
  assert.equal(r.skipped, true);
  assert.equal(calls.length, 0);
  assert.match(lines[0], /skipped/);
});

test('DELETE failures are counted, not thrown; list failure is survived', async () => {
  const calls = [];
  const r = await run({ baseUrl: 'https://d', dryRun: false, minAgeHours: 6, env: { API_KEY_ADMIN: 'k' }, fetchImpl: fakeFetch([u('e2e-a', false)], calls, true), now: NOW, log: quiet });
  assert.equal(r.failed, 1);
  const bad = await run({ baseUrl: 'https://d', dryRun: false, minAgeHours: 6, env: { API_KEY_ADMIN: 'k' }, fetchImpl: async () => { throw new Error('boom'); }, now: NOW, log: quiet });
  assert.equal(bad.failed, 1);
});

test('parseArgs', () => {
  assert.deepEqual(parseArgs(['https://x', '--dry-run', '--min-age-hours', '2']), { baseUrl: 'https://x', dryRun: true, minAgeHours: 2 });
  assert.equal(parseArgs([]), null);
  assert.equal(parseArgs(['a', 'b']), null);
  assert.equal(parseArgs(['a', '--bogus']), null);
  assert.equal(parseArgs(['a', '--min-age-hours', 'x']), null);
});
