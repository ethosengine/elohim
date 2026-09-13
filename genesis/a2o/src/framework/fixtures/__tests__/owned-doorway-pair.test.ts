/* eslint-disable sonarjs/no-clear-text-protocols -- this fixture exercises loopback-only HTTP endpoints */
import { strict as assert } from 'node:assert';
import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';

import {
  fixtureDoorwayLaunch,
  startOwnedChild,
  stopFixtureProcesses,
  withExtraSsrSlug,
  type LaunchTemplate,
} from '../owned-doorway-pair.js';
import { stopOwnedProcess } from '../owned-doorway-process.js';

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
  },
};

void test('fixture launch preserves the template and rewrites only identity-local routing', () => {
  const launch = fixtureDoorwayLaunch(template, {
    id: 'fixture-a',
    listenPort: 19001,
    healthPort: 19101,
    mongoPort: 19201,
    database: 'fixture-a-db',
    primaryUrl: 'http://127.0.0.1:8091', // NOSONAR -- loopback fixture transport
    extraUrls: ['http://127.0.0.1:8090', 'http://127.0.0.1:8092'],
  });
  assert.equal(launch.argv[launch.argv.indexOf('--conductor-url') + 1], 'ws://127.0.0.1:4444');
  assert.equal(launch.argv[launch.argv.indexOf('--listen') + 1], '127.0.0.1:19001');
  assert.equal(launch.argv[launch.argv.indexOf('--storage-url') + 1], 'http://127.0.0.1:8091');
  assert.equal(
    launch.argv[launch.argv.indexOf('--storage-urls') + 1],
    'http://127.0.0.1:8090,http://127.0.0.1:8092'
  );
  assert.equal(launch.env['DOORWAY_ID'], 'fixture-a');
  assert.equal(launch.env['MONGODB_DB'], 'fixture-a-db');
  assert.equal(template.argv[2], '0.0.0.0:8888', 'canonical template stays immutable');
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
