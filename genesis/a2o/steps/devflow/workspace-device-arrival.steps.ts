/**
 * workspace-device-arrival — an isolated native fixture (no deployed environment, no E2E hooks),
 * in the shape of collective-memory.steps.ts: a temporary committed repository, the REAL `epr`
 * binary, and two device keys that live OUTSIDE every git repository (device_key.rs refuses a key
 * inside one). "The other device" and "this workspace" are the same machine with two key files;
 * the ceremony's property under test is that no key moves, which two files make checkable.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  copyFileSync,
  existsSync,
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

type ObjectValue = Record<string, unknown>;
interface Fixture {
  base: string; // the temp dir; the repo is base/repo, keys under base/keys, homes under base/home-*
  root: string;
  keyA: string; // "the other device"
  keyB: string; // "this workspace"
  session: string;
  request?: ObjectValue;
  authorization?: ObjectValue;
  preflight?: { lines: string[]; status: number };
  keyStatBefore?: { a: number; b: number };
}
const fixtures = new WeakMap<object, Fixture>();
const repository = resolve('../..');
const binary = process.env.EPR_BIN ?? 'epr';
const preflightScript = join(repository, 'genesis/agentic/bin/device-preflight');
const MODEL_MANIFEST = '.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json';
const INDEX_MEASURE = '.epr-meta/elohim/algorithms/recall-semantic-index.json';
const HANDLE = 'matthew';

function state(world: object): Fixture {
  const f = fixtures.get(world);
  assert.ok(f, 'the Background did not run');
  return f;
}
function object(value: unknown): ObjectValue {
  assert.ok(value && typeof value === 'object' && !Array.isArray(value));
  return value as ObjectValue;
}
/** The environment one device sees: its own key file, its own homes, the fixture's config dir. */
function deviceEnv(f: Fixture, key: string, extra: Record<string, string> = {}): NodeJS.ProcessEnv {
  const home = join(f.base, `home-${key === f.keyA ? 'a' : 'b'}`);
  mkdirSync(home, { recursive: true });
  return {
    PATH: process.env.PATH,
    HOME: home,
    XDG_CONFIG_HOME: join(home, '.config'),
    XDG_CACHE_HOME: join(home, '.cache'),
    ELOHIM_DEVICE_KEY_FILE: key,
    CLAUDE_CONFIG_DIR: join(f.base, 'config'),
    CARGO_TARGET_POOL_ROOT: join(f.base, 'no-pool'),
    EPR_BIN: binary === 'epr' ? (whichEpr() ?? 'epr') : binary,
    CLAUDE_CODE_SESSION_ID: f.session,
    ...extra,
  };
}
/** The `epr` the shell would run, found by walking PATH ourselves (no shell is spawned). */
function whichEpr(): string | undefined {
  for (const dir of (process.env.PATH ?? '').split(':').filter(Boolean)) {
    const candidate = join(dir, 'epr');
    try {
      if (statSync(candidate).isFile()) return candidate;
    } catch {
      // not here
    }
  }
  return undefined;
}
function run(f: Fixture, argv: string[], env: NodeJS.ProcessEnv, expected: number | null = 0) {
  const result = spawnSync(argv[0], argv.slice(1), {
    cwd: f.root,
    env,
    encoding: 'utf8',
    timeout: 30_000,
    maxBuffer: 4 * 1024 * 1024,
  });
  if (expected !== null) {
    assert.equal(result.status, expected, `${argv.join(' ')}\n${result.stdout}\n${result.stderr}`);
  }
  return result;
}
function epr(f: Fixture, env: NodeJS.ProcessEnv, args: string[], expected: number | null = 0) {
  const r = run(f, [env.EPR_BIN ?? binary, ...args, '--root', f.root], env, expected);
  return r.stdout.trim() ? object(JSON.parse(r.stdout)) : {};
}
function preflight(f: Fixture, env: NodeJS.ProcessEnv): { lines: string[]; status: number } {
  const r = run(
    f,
    ['python3', preflightScript, '--root', f.root, '--handle', HANDLE, '--session', f.session],
    env,
    null
  );
  assert.notEqual(r.status, null, `preflight did not run\n${r.stderr}`);
  return { lines: r.stdout.trim().split('\n'), status: r.status as number };
}
function line(f: Fixture, name: string): string {
  assert.ok(f.preflight, 'the preflight has not run');
  const found = f.preflight.lines.find(
    l => l.startsWith(`ok ${name}:`) || l.startsWith(`REFUSED ${name}:`)
  );
  assert.ok(found, `no "${name}" line in:\n${f.preflight.lines.join('\n')}`);
  return found;
}
function rosterRows(f: Fixture): ObjectValue[] {
  const path = join(f.root, '.eprfs/status/participants', `${HANDLE}.jsonl`);
  return readFileSync(path, 'utf8')
    .split('\n')
    .filter(l => l.trim())
    .map(l => object(JSON.parse(l)));
}
function git(f: Fixture, args: string[]) {
  run(f, ['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', ...args], {
    PATH: process.env.PATH,
    HOME: f.base,
  });
}

Given(
  'a committed repository whose roster for {string} was begun on another device',
  function (handle: string) {
    assert.equal(handle, HANDLE);
    const base = mkdtempSync(join(tmpdir(), 'device-arrival-'));
    const root = join(base, 'repo');
    mkdirSync(join(root, '.eprfs/status/participants'), { recursive: true });
    mkdirSync(join(base, 'keys'), { recursive: true });
    mkdirSync(join(base, 'config/berth/moorings'), { recursive: true });
    for (const rel of [MODEL_MANIFEST, INDEX_MEASURE]) {
      mkdirSync(dirname(join(root, rel)), { recursive: true });
      copyFileSync(join(repository, rel), join(root, rel));
    }
    writeFileSync(join(root, 'README.md'), '# fixture\n');
    const f: Fixture = {
      base,
      root,
      keyA: join(base, 'keys/other-device.seed'),
      keyB: join(base, 'keys/this-workspace.seed'),
      session: 'arrival-1',
    };
    fixtures.set(this, f);
    git(f, ['init', '-q', root]);
    git(f, ['-C', root, 'add', '-A']);
    git(f, ['-C', root, 'commit', '-q', '-m', 'fixture']);
    writeFileSync(join(base, 'config/berth/moorings', `${f.session}.json`), '{}');
    // The other device witnesses matthew: the roster's genesis row, signed by key A (minted here).
    epr(f, deviceEnv(f, f.keyA), [
      'actor',
      'witness',
      '--subject',
      `human:${HANDLE}`,
      '--as',
      'agent:reader@fixture',
      '--session',
      'other-1',
      '--basis',
      'the person is present at the fixture',
      '--json',
    ]);
    assert.ok(existsSync(f.keyA), 'the other device minted its key');
    assert.equal(rosterRows(f).length, 1, 'one genesis row');
  }
);

After(function () {
  const f = fixtures.get(this);
  if (f) rmSync(f.base, { recursive: true, force: true });
});

Given('this workspace holds no device key', function () {
  const f = state(this);
  assert.ok(!existsSync(f.keyB));
});

When('the arrival preflight runs here', function () {
  const f = state(this);
  f.preflight = preflight(f, deviceEnv(f, f.keyB));
});

Then('the {string} line is REFUSED and names {string}', function (name: string, cure: string) {
  const l = line(state(this), name);
  assert.ok(l.startsWith('REFUSED'), l);
  assert.ok(l.includes(cure), `${l}\n  does not name ${cure}`);
});

Then('the {string} line is REFUSED', function (name: string) {
  const l = line(state(this), name);
  assert.ok(l.startsWith('REFUSED'), l);
});

Then('that cure is addressed to a device already on the roster', function () {
  const l = line(state(this), 'roster bound');
  assert.ok(l.includes('on a device already in'), l);
});

Then('the {string} line also names {string}', function (name: string, cure: string) {
  const l = line(state(this), name);
  assert.ok(l.includes(cure), `${l}\n  does not name ${cure}`);
});

Then('the {string} line reads ok', function (name: string) {
  const l = line(state(this), name);
  assert.ok(l.startsWith('ok'), l);
});

Then('the preflight exits non-zero', function () {
  const f = state(this);
  assert.notEqual(f.preflight?.status, 0);
});

Then('epr still answers, and says this device stands for nobody yet', function () {
  const f = state(this);
  const answer = epr(f, deviceEnv(f, f.keyB), ['actor', 'current', '--device', '--json']);
  assert.ok('standing' in answer, JSON.stringify(answer));
  assert.equal(answer.standing, null);
});

When(
  'this workspace enrolls for {string} and receives a signed request to hand to the other device',
  function (handle: string) {
    const f = state(this);
    f.keyStatBefore = { a: statSync(f.keyA).mtimeMs, b: 0 };
    f.request = epr(f, deviceEnv(f, f.keyB), [
      'actor',
      'device',
      'enroll',
      '--handle',
      handle,
      '--json',
    ]);
    assert.equal(f.request.handle, handle);
    assert.ok(String(f.request.controller).startsWith('did:key:'), JSON.stringify(f.request));
    f.keyStatBefore.b = statSync(f.keyB).mtimeMs;
  }
);

When('the other device authorizes that request', function () {
  const f = state(this);
  assert.ok(f.request);
  f.authorization = epr(f, deviceEnv(f, f.keyA), [
    'actor',
    'device',
    'authorize',
    JSON.stringify(f.request),
    '--json',
  ]);
  assert.ok(f.authorization.authorizedBy, JSON.stringify(f.authorization));
});

When('this workspace binds the authorization it was handed back', function () {
  const f = state(this);
  assert.ok(f.authorization);
  epr(f, deviceEnv(f, f.keyB), [
    'actor',
    'device',
    'bind',
    JSON.stringify(f.authorization),
    '--json',
  ]);
});

Then(
  "matthew's roster carries one binding for this workspace approved by the other device",
  function () {
    const f = state(this);
    const rows = rosterRows(f);
    assert.equal(rows.length, 2, JSON.stringify(rows));
    const text = JSON.stringify(rows[1]);
    assert.ok(
      text.includes(String(f.request?.controller)),
      `the binding names this workspace\n${text}`
    );
    assert.ok(
      text.includes(String(f.authorization?.authorizedBy)),
      `the binding names the approver\n${text}`
    );
    assert.notEqual(object(rows[1].row).kind, 'genesis');
  }
);

Then('the two devices hold different keys and neither key file moved', function () {
  const f = state(this);
  assert.ok(existsSync(f.keyA) && existsSync(f.keyB));
  assert.notEqual(readFileSync(f.keyA, 'hex'), readFileSync(f.keyB, 'hex'));
  assert.equal(
    statSync(f.keyA).mtimeMs,
    f.keyStatBefore?.a,
    "the other device's key was not rewritten"
  );
  assert.equal(
    statSync(f.keyB).mtimeMs,
    f.keyStatBefore?.b,
    "this workspace's key was not rewritten"
  );
});

Then("neither the request nor the authorization carries either device's secret key", function () {
  const f = state(this);
  assert.ok(f.request && f.authorization);
  const notes = JSON.stringify(f.request) + JSON.stringify(f.authorization);
  for (const key of [f.keyA, f.keyB]) {
    const seed = readFileSync(key);
    for (const form of [
      seed.toString('hex'),
      seed.toString('base64'),
      seed.toString('base64url'),
    ]) {
      assert.ok(!notes.includes(form), `a note carries the seed at ${key}`);
    }
  }
});

Then('the {string} line of the arrival preflight reads ok here', function (name: string) {
  const f = state(this);
  f.preflight = preflight(f, deviceEnv(f, f.keyB));
  const l = line(f, name);
  assert.ok(l.startsWith('ok'), l);
});

Given('no declared place holds the embedding model', function () {
  const f = state(this);
  // Every resolve entry either names an unset variable or an empty directory in this fixture's homes.
  mkdirSync(join(f.base, 'empty-models'), { recursive: true });
  f.preflight = preflight(
    f,
    deviceEnv(f, f.keyB, { EPR_EMBED_MODEL_DIR: join(f.base, 'empty-models') })
  );
});

Given('every arrival condition is met on this workspace', function () {
  return 'pending';
});

Then('every line reads ok and the preflight exits zero', function () {
  return 'pending';
});
