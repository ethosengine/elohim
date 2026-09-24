/* eslint-disable sonarjs/no-clear-text-protocols -- this fixture exercises loopback-only HTTP endpoints */
import { strict as assert } from 'node:assert';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  deregisterFixtureDoorway,
  fixtureDoorwayLaunch,
  startOwnedChild,
  stopFixtureProcesses,
  teardownFixtureDoorways,
  withExtraSsrSlug,
  type DeregistrationOutcome,
  type LaunchTemplate,
} from '../owned-doorway-pair.js';
import { stopOwnedProcess } from '../owned-doorway-process.js';

// A fake, never-written scenario root — fixtureDoorwayLaunch only does string
// composition, no I/O, so this never needs to exist on disk.
const SCENARIO_DIR = '/owned/scenario';
const PRIMARY_URL = 'http://127.0.0.1:8091'; // NOSONAR -- loopback fixture transport
const EXTRA_URLS = ['http://127.0.0.1:8090', 'http://127.0.0.1:8092'];
const CANONICAL_SSR_BUNDLE_PATH = '/repo/app/elohim-app/dist/elohim-app/server/main.server.mjs';
const CANONICAL_NODE_KEY_FILE = '/repo/genesis/local-dev/household-dowell/doorway-a-node.key';

function fixtureAInput(): Parameters<typeof fixtureDoorwayLaunch>[1] {
  return {
    id: 'fixture-a',
    listenPort: 19001,
    healthPort: 19101,
    mongoPort: 19201,
    database: 'fixture-a-db',
    primaryUrl: PRIMARY_URL,
    extraUrls: EXTRA_URLS,
    scenarioDir: SCENARIO_DIR,
  };
}

const template: LaunchTemplate = {
  executable: '/owned/doorway',
  executableHash: 'abc',
  runningExecutable: '/proc/1/exe',
  cwd: '/owned',
  argv: [
    '/owned/doorway',
    '--listen',
    '0.0.0.0:8888',
    '--conductor-url',
    'ws://127.0.0.1:4444',
    '--storage-url',
    'http://127.0.0.1:8090', // NOSONAR -- loopback fixture transport
    '--storage-urls',
    'http://127.0.0.1:8091,http://127.0.0.1:8092',
  ],
  env: {
    SECRET_NOT_FOR_RECEIPT: 'preserved-without-being-returned-by-the-receipt',
    SSR_BUNDLE_SLUGS: 'elohim-host-landing,epr-app-deliverability-old-run',
    // The canonical doorway's real launch env — inherited via `...template.env`
    // spread — points at the repo dist and the canonical node identity. A
    // fixture doorway must override BOTH: sharing the dist means the fixture's
    // materialize/reconcile scratch lands back in the tree the packager zips
    // (the household server-bundle feedback loop); sharing the node key file
    // means the fixture doorway signs with the canonical doorway's identity.
    SSR_BUNDLE_PATH: CANONICAL_SSR_BUNDLE_PATH,
    DOORWAY_NODE_KEY_FILE: CANONICAL_NODE_KEY_FILE,
  },
};

void test('fixture launch preserves the template and rewrites only identity-local routing', () => {
  const launch = fixtureDoorwayLaunch(template, fixtureAInput());
  assert.equal(launch.argv[launch.argv.indexOf('--conductor-url') + 1], 'ws://127.0.0.1:4444');
  assert.equal(launch.argv[launch.argv.indexOf('--listen') + 1], '127.0.0.1:19001');
  assert.equal(launch.argv[launch.argv.indexOf('--storage-url') + 1], PRIMARY_URL);
  assert.equal(launch.argv[launch.argv.indexOf('--storage-urls') + 1], EXTRA_URLS.join(','));
  assert.equal(launch.env['DOORWAY_ID'], 'fixture-a');
  assert.equal(launch.env['MONGODB_DB'], 'fixture-a-db');
  assert.equal(template.argv[2], '0.0.0.0:8888', 'canonical template stays immutable');
});

void test('fixture launch never inherits the canonical dist as its SSR bundle path', () => {
  const launch = fixtureDoorwayLaunch(template, fixtureAInput());
  const ssrPath = launch.env['SSR_BUNDLE_PATH'];
  assert.ok(ssrPath, 'fixture launch must set SSR_BUNDLE_PATH');
  assert.ok(
    ssrPath?.startsWith(`${SCENARIO_DIR}/`),
    `fixture SSR_BUNDLE_PATH must live under its own scenario dir, got ${ssrPath}`
  );
  assert.ok(
    !ssrPath?.includes('/dist/elohim-app/server'),
    `fixture SSR_BUNDLE_PATH must never point into the repo dist, got ${ssrPath}`
  );
  assert.equal(
    template.env['SSR_BUNDLE_PATH'],
    CANONICAL_SSR_BUNDLE_PATH,
    'canonical template stays immutable'
  );
});

void test('fixture launch never inherits the canonical doorway node identity file', () => {
  const launchA = fixtureDoorwayLaunch(template, fixtureAInput());
  const launchB = fixtureDoorwayLaunch(template, {
    id: 'fixture-b',
    listenPort: 19002,
    healthPort: 19102,
    mongoPort: 19201,
    database: 'fixture-b-db',
    primaryUrl: EXTRA_URLS[0],
    extraUrls: [PRIMARY_URL, EXTRA_URLS[1]],
    scenarioDir: SCENARIO_DIR,
  });
  const keyFileA = launchA.env['DOORWAY_NODE_KEY_FILE'];
  const keyFileB = launchB.env['DOORWAY_NODE_KEY_FILE'];
  assert.ok(keyFileA, 'fixture launch must set DOORWAY_NODE_KEY_FILE');
  assert.notEqual(
    keyFileA,
    template.env['DOORWAY_NODE_KEY_FILE'],
    'fixture must not sign with the canonical doorway identity'
  );
  assert.ok(keyFileA?.startsWith(`${SCENARIO_DIR}/`), `got ${keyFileA}`);
  assert.notEqual(keyFileA, keyFileB, 'each fixture doorway needs its own identity file');
});

void test('restart slug replacement keeps ordinary SSR config and removes the prior run slug', () => {
  const env = withExtraSsrSlug(template.env, 'epr-app-deliverability-next');
  assert.equal(env['SSR_BUNDLE_SLUGS'], 'elohim-host-landing,epr-app-deliverability-next');
  assert.equal(
    template.env['SSR_BUNDLE_SLUGS'],
    'elohim-host-landing,epr-app-deliverability-old-run'
  );
});

void test('partial-start cleanup attempts doorways before mongo even when one stop refuses ownership', async () => {
  const calls: string[] = [];
  await assert.rejects(
    stopFixtureProcesses(
      [
        { handle: { pid: 11, ticks: '1', executable: '/doorway' }, label: 'doorway b' },
        { handle: { pid: 12, ticks: '2', executable: '/doorway' }, label: 'doorway a' },
        { handle: { pid: 13, ticks: '3', executable: '/mongod' }, label: 'mongod' },
      ],
      async (_handle, label) => {
        calls.push(label);
        if (label === 'doorway b') return await Promise.reject(new Error('wrong owner'));
        return await Promise.resolve('term');
      }
    ),
    /cleanup failed/
  );
  assert.deepEqual(calls, ['doorway b', 'doorway a', 'mongod']);
});

void test('stop refuses a recycled or wrong-owner pid before sending a signal', async () => {
  await assert.rejects(
    stopOwnedProcess(
      { pid: process.pid, ticks: 'definitely-not-this-process', executable: process.execPath },
      'wrong owner',
      1
    ),
    /refuse a recycled wrong owner PID/
  );
  assert.doesNotThrow(() => process.kill(process.pid, 0));
});

void test('owned child launch waits for a real log descriptor before spawn', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'owned-doorway-child-test-'));
  try {
    const started = await startOwnedChild(
      '/bin/sleep',
      ['/bin/sleep', '30'],
      process.env,
      dir,
      join(dir, 'child.log')
    );
    assert.ok(started.handle.pid > 1);
    assert.equal(await stopOwnedProcess(started.handle, 'fixture test child'), 'term');
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

void test('owned child that ignores TERM is confirmed gone after bounded KILL', async () => {
  const dir = mkdtempSync(join(tmpdir(), 'owned-doorway-kill-test-'));
  try {
    const started = await startOwnedChild(
      process.execPath,
      [process.execPath, '-e', "process.on('SIGTERM',()=>{}); setInterval(()=>{},1000)"],
      process.env,
      dir,
      join(dir, 'child.log')
    );
    await new Promise(resolve => setTimeout(resolve, 100));
    assert.equal(
      await stopOwnedProcess(started.handle, 'TERM-ignoring fixture child', 100),
      'kill'
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// ── Teardown deregisters the scenario doorways (conductor-store growth report
//    2026-09-24 §7.1: two doorways per run stayed on the roster forever and
//    every sibling attested each one every probe round) ─────────────────────

const FIXTURE_A_URL = 'http://127.0.0.1:19001';
const FIXTURE_B_URL = 'http://127.0.0.1:19002';

function fakeFetch(
  respond: (url: string, init?: RequestInit) => Response | Promise<Response>,
  calls: { url: string; method?: string }[]
): typeof fetch {
  return (async (input: string | URL | Request, init?: RequestInit) => {
    let url: string;
    if (typeof input === 'string') url = input;
    else if (input instanceof URL) url = input.href;
    else url = input.url;
    calls.push({ url, method: init?.method });
    return await respond(url, init);
  }) as typeof fetch;
}

void test('deregistration POSTs the doorway own verb and reports the links it removed', async () => {
  const calls: { url: string; method?: string }[] = [];
  const outcome = await deregisterFixtureDoorway(
    FIXTURE_A_URL,
    fakeFetch(
      () =>
        new Response(JSON.stringify({ doorwayId: 'fixture-a', linksDeleted: 4 }), {
          status: 200,
          headers: { 'content-type': 'application/json' },
        }),
      calls
    )
  );
  assert.deepEqual(calls, [
    { url: `${FIXTURE_A_URL}/admin/federation/deregister`, method: 'POST' },
  ]);
  assert.deepEqual(outcome, { status: 'deregistered', linksDeleted: 4 });
});

void test('deregistration names an older binary unsupported and never throws on failure', async () => {
  const calls: { url: string; method?: string }[] = [];
  assert.deepEqual(
    await deregisterFixtureDoorway(
      FIXTURE_A_URL,
      fakeFetch(() => new Response('not found', { status: 404 }), calls)
    ),
    { status: 'unsupported', httpStatus: 404 }
  );
  const failed = await deregisterFixtureDoorway(
    FIXTURE_A_URL,
    fakeFetch(
      () => new Response('{"error":"deregistration zome call failed"}', { status: 502 }),
      calls
    )
  );
  assert.equal(failed.status, 'failed');
  const unreachable = await deregisterFixtureDoorway(
    FIXTURE_A_URL,
    fakeFetch(async () => await Promise.reject(new Error('ECONNREFUSED')), calls)
  );
  assert.equal(unreachable.status, 'failed');
});

void test('teardown deregisters every scenario doorway while it runs, then stops them and mongod', async () => {
  const events: string[] = [];
  const outcomes = await teardownFixtureDoorways(
    [
      {
        name: 'b',
        url: FIXTURE_B_URL,
        handle: { pid: 21, ticks: '1', executable: '/d' },
      },
      {
        name: 'a',
        url: FIXTURE_A_URL,
        handle: { pid: 22, ticks: '2', executable: '/d' },
      },
    ],
    { pid: 23, ticks: '3', executable: '/mongod' },
    {
      deregister: async (url: string): Promise<DeregistrationOutcome> => {
        events.push(`deregister ${url}`);
        return await Promise.resolve({ status: 'deregistered', linksDeleted: 4 });
      },
      stop: async (_handle, label) => {
        events.push(`stop ${label}`);
        return await Promise.resolve('term');
      },
    }
  );
  assert.deepEqual(events, [
    `deregister ${FIXTURE_B_URL}`,
    `deregister ${FIXTURE_A_URL}`,
    'stop fixture doorway b',
    'stop fixture doorway a',
    'stop fixture mongod',
  ]);
  assert.deepEqual(outcomes, {
    b: { status: 'deregistered', linksDeleted: 4 },
    a: { status: 'deregistered', linksDeleted: 4 },
  });
});

void test('a failed deregistration is recorded and never keeps the fixture processes alive', async () => {
  const stopped: string[] = [];
  const outcomes = await teardownFixtureDoorways(
    [
      {
        name: 'a',
        url: FIXTURE_A_URL,
        handle: { pid: 22, ticks: '2', executable: '/d' },
      },
    ],
    { pid: 23, ticks: '3', executable: '/mongod' },
    {
      deregister: async () => await Promise.resolve({ status: 'failed', detail: '502' } as const),
      stop: async (_handle, label) => {
        stopped.push(label);
        return await Promise.resolve('term');
      },
    }
  );
  assert.deepEqual(stopped, ['fixture doorway a', 'fixture mongod']);
  assert.deepEqual(outcomes, { a: { status: 'failed', detail: '502' } });
});
