/** Fixture-only declaration chain; no live device, bin store or network. */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

interface View {
  manifestCid: string;
  supersedes: string | null;
  follows: { channel: string; mode: string }[];
  processes: { name: string }[];
}
interface Fixture {
  root: string;
  env: NodeJS.ProcessEnv;
  versions: View[];
  notice?: string;
  pin?: string;
}
const fixtures = new WeakMap<object, Fixture>();
function fixture(world: object): Fixture {
  const f = fixtures.get(world);
  assert.ok(f);
  return f;
}
function epr(f: Fixture, ...args: string[]): string {
  const run = spawnSync(
    process.env.EPR_BIN ?? 'epr',
    ['device', ...args, '--root', join(f.root, 'device')],
    { env: f.env, encoding: 'utf8', timeout: 30_000 }
  );
  assert.equal(run.status, 0, `${run.error ?? ''}\n${run.stderr}`);
  return run.stdout.trim();
}
Given('an isolated device with the committed workspace runtime base and no network', function () {
  const root = mkdtempSync(join(tmpdir(), 'declared-device-'));
  mkdirSync(join(root, 'proc'));
  const env: NodeJS.ProcessEnv = {
    ...process.env,
    EPR_PROC_ROOT: join(root, 'proc'),
    ELOHIM_DEVICE_KEY_FILE: join(root, 'device/key'),
    ELOHIM_SESSION_ID: 'target-fixture',
    BIN_STORE: join(root, 'bins'),
    ELOHIM_RUNTIME_CONFIG_PATH: undefined,
    ELOHIM_RUNTIME_MANIFEST_PATH: undefined,
    CARGO_NET_OFFLINE: 'true',
  };
  delete env.EPR_SESSION_LAZY;
  const f: Fixture = { root, env, versions: [] };
  fixtures.set(this, f);
  f.versions.push(JSON.parse(epr(f, 'show', '--json')) as View);
});
When('the device declares {string} in {string} mode', function (channel: string, mode: string) {
  const f = fixture(this);
  f.versions.push(JSON.parse(epr(f, 'follow', channel, '--mode', mode)) as View);
});
When('the device declares that it runs {string}', function (tool: string) {
  const f = fixture(this);
  const head = JSON.parse(epr(f, 'run', tool)) as View;
  assert.ok(head.processes.some(p => p.name === tool));
  f.versions.push(head);
});
Then("the device's watched follow line matches its declared target", function () {
  const f = fixture(this);
  const rendered = epr(f, 'show', '--render', 'runtime-config');
  assert.equal(readFileSync(join(f.root, 'device/runtime-config.toml'), 'utf8').trim(), rendered);
  assert.match(rendered, /bin:epr=apply/);
  assert.match(rendered, /runtime:coordinators:elohim:workspace=observe/);
});
Then('each declaration supersedes the previous manifest by CID', function () {
  const f = fixture(this);
  for (let i = 1; i < f.versions.length; i++)
    assert.equal(f.versions[i].supersedes, f.versions[i - 1].manifestCid);
});
When(
  "the device's watched configuration is lost and the device applies its target again",
  function () {
    const f = fixture(this);
    rmSync(join(f.root, 'device/runtime-config.toml'));
    const replay = JSON.parse(epr(f, 'apply')) as View;
    assert.equal(replay.manifestCid, f.versions.at(-1)?.manifestCid);
  }
);
When('a newer landed tool is available to the pinned session', function () {
  const f = fixture(this);
  const tool = join(f.root, 'bins/epr');
  mkdirSync(join(tool, 'sessions'), { recursive: true });
  f.pin = JSON.stringify({
    cid: 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku',
    source: 'dev-context',
    session: 'target-fixture',
  });
  writeFileSync(join(tool, 'sessions/target-fixture'), f.pin);
  writeFileSync(
    join(tool, 'current.json'),
    JSON.stringify({
      cid: 'bafkreiaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      commit: 'fixture-commit',
      landedAt: '2026-10-25T00:00:00Z',
      subject: 'Carry the landed launcher',
    })
  );
  const run = spawnSync(
    '/usr/bin/python3',
    [resolve('../../.claude/hooks/device-head-notice.py')],
    {
      env: f.env,
      input: '{}',
      encoding: 'utf8',
    }
  );
  assert.equal(run.status, 0, run.stderr);
  f.notice = run.stdout;
});
Then(
  'the session receives one notice naming the old and new tool CIDs and a command to switch',
  function () {
    const f = fixture(this);
    assert.match(
      f.notice ?? '',
      /bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku.*bafkreiaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa/
    );
    assert.match(f.notice ?? '', /Carry the landed launcher/);
    assert.match(f.notice ?? '', /\/clear or epr bin pin --tool epr --cid/);
    const again = spawnSync(
      '/usr/bin/python3',
      [resolve('../../.claude/hooks/device-head-notice.py')],
      {
        env: f.env,
        input: '{}',
        encoding: 'utf8',
      }
    );
    assert.equal(again.status, 0, again.stderr);
    assert.equal(again.stdout, '');
  }
);
Then("the session's recorded tool pin is unchanged", function () {
  const f = fixture(this);
  assert.equal(readFileSync(join(f.root, 'bins/epr/sessions/target-fixture'), 'utf8'), f.pin);
});
When("the device's private manifest chain is removed", function () {
  rmSync(join(fixture(this).root, 'device/manifest'), { recursive: true });
});
Then(
  'the device returns to the committed base with epr and brit followed in apply mode',
  function () {
    const f = fixture(this);
    const reset = JSON.parse(epr(f, 'show', '--json')) as View;
    assert.equal(reset.manifestCid, f.versions[0].manifestCid);
    assert.deepEqual(reset.follows, [
      { channel: 'bin:epr', mode: 'apply' },
      { channel: 'bin:brit', mode: 'apply' },
    ]);
  }
);
After({ tags: '@device-declared-target' }, function () {
  const f = fixtures.get(this);
  if (f) rmSync(f.root, { recursive: true, force: true });
});
