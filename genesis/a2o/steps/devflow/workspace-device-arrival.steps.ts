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
  openSync,
  closeSync,
  readFileSync,
  readdirSync,
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
  bins?: string; // the executables' place this workspace names (BIN_STORE)
  layout?: { value: ObjectValue; status: number };
  keyStatBefore?: { a: number; b: number };
  home?: HomeDevice; // the device whose key lives under its own $HOME (durable-home scenarios)
}
interface HomeDevice {
  home: string;
  key: string; // <home>/.config/elohim/device/ed25519.seed, the default placement
  marker?: Record<string, string>; // an ephemeral-platform marker the child runs with
  ownDeclaration?: string; // the bytes of this device's own declaration, kept when a foreign one replaces it
  moved?: { key: string; dir: string }; // where the key travelled to, when the story moves it
  exit?: { status: number | null; stdout: string; stderr: string };
}
const fixtures = new WeakMap<object, Fixture>();
const repository = resolve('../..');
const binary = process.env.EPR_BIN ?? 'epr';
const MODEL_MANIFEST = '.epr-meta/elohim/algorithms/embedding-models/all-minilm-l6-v2.json';
const INDEX_MEASURE = '.epr-meta/elohim/algorithms/recall-semantic-index.json';
const RECALL_CONTRACT = '.epr-meta/elohim/algorithms/recall-contract.json'; // `session open` initializes recall
const HANDLE = 'matthew';
const DECLARATION = 'persistence.json'; // the durable-home note beside the device key

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
    EPR_STORE: join(f.base, 'bin-store/epr'),
    BRIT_STORE: join(f.base, 'bin-store/brit'),
    // Leave EPR_BIN absent when the fixture is using PATH: the PATH entry may be the stable
    // launcher, while EPR_BIN is reserved for the real candidate supplied by the gate.
    ...(binary === 'epr' ? {} : { EPR_BIN: binary }),
    ELOHIM_SESSION_ID: f.session,
    EPR_BIN_TOOLS: join(
      repository,
      'genesis/orchestrator/manifests/runtime/workspace-device.manifest.json'
    ),
    ...extra,
  };
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
  const r = run(f, [env.EPR_BIN ?? binary, 'device', 'status', '--root', f.root], env, null);
  assert.notEqual(r.status, null, `preflight did not run\n${r.stderr}`);
  return { lines: r.stdout.trim().split('\n'), status: r.status as number };
}
function line(f: Fixture, name: string): string {
  assert.ok(f.preflight, 'the preflight has not run');
  const found = f.preflight.lines.find(l =>
    ['ok', 'degraded', 'REFUSED', 'unknown'].some(state => l.startsWith(`${state} ${name}:`))
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
  return run(
    f,
    ['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', ...args],
    { PATH: process.env.PATH, HOME: f.base }
  );
}
function gitOut(f: Fixture, args: string[]): string {
  return git(f, args).stdout;
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
    for (const rel of [MODEL_MANIFEST, INDEX_MEASURE, RECALL_CONTRACT]) {
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
    writeFileSync(
      join(base, 'config/berth/moorings', `${f.session}.json`),
      JSON.stringify({ session: f.session })
    );
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
    // The roster as the other device began it is on the integration branch, fetched here.
    commitRoster(f, "matthew's roster, begun on the other device");
    fetchedAt(f, 'HEAD');
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
  'this workspace enrolls for {string} and receives an enrollment request to hand to the other device',
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

/** The roster line names the standing; whether it is published is the publication scenarios'. */
function assertStands(l: string, handle: string) {
  assert.ok(!l.startsWith('REFUSED'), l);
  assert.ok(l.includes(`standing human:${handle} `), `${l}\n  does not stand for ${handle}`);
}

Then(
  'the arrival preflight here says this workspace stands for {string}',
  function (handle: string) {
    const f = state(this);
    f.preflight = preflight(f, deviceEnv(f, f.keyB));
    assertStands(line(f, 'roster bound'), handle);
  }
);

Then(
  'the {string} line says the device stands for {string}',
  function (name: string, handle: string) {
    assertStands(line(state(this), name), handle);
  }
);

Given('no declared place holds the embedding model', function () {
  const f = state(this);
  // Every resolve entry either names an unset variable or an empty directory in this fixture's homes.
  mkdirSync(join(f.base, 'empty-models'), { recursive: true });
  f.preflight = preflight(
    f,
    deviceEnv(f, f.keyB, { EPR_EMBED_MODEL_DIR: join(f.base, 'empty-models') })
  );
});

/** The whole ceremony for this workspace: enroll here, authorize on the other device, bind here. */
function joinHere(f: Fixture) {
  f.request = epr(f, deviceEnv(f, f.keyB), [
    'actor',
    'device',
    'enroll',
    '--handle',
    HANDLE,
    '--json',
  ]);
  f.authorization = epr(f, deviceEnv(f, f.keyA), [
    'actor',
    'device',
    'authorize',
    JSON.stringify(f.request),
    '--json',
  ]);
  epr(f, deviceEnv(f, f.keyB), [
    'actor',
    'device',
    'bind',
    JSON.stringify(f.authorization),
    '--json',
  ]);
}

Given('the core arrival conditions are met on this workspace', function () {
  const f = state(this);
  joinHere(f);
  // Candidate execution makes a fixture local-first; no downloaded brit release or model needed.
  f.preflight = preflight(
    f,
    deviceEnv(f, f.keyB, {
      EPR_BIN: binary,
      BRIT_BIN: binary,
      PATH: '/usr/local/bin:/usr/bin:/bin',
    })
  );
});

Then('every line reads ok, degraded or unknown and the preflight exits zero', function () {
  const f = state(this);
  assert.equal(f.preflight?.status, 0, f.preflight?.lines.join('\n'));
  // The condition list grows with the device (hooks, trust, packs, daemon, supervisor, toolchain);
  // the claim is that none of them refuses, so no count is asserted. Named lines have their own steps.
  // Unknown is a fact this device cannot read alone (a branch never fetched, no saved peer status).
  const lines = f.preflight?.lines ?? [];
  assert.ok(lines.length > 0, 'the preflight printed no condition');
  const refused = lines.filter(line => !/^(ok|degraded|unknown) /.test(line));
  assert.deepEqual(refused, [], lines.join('\n'));
});

Then('the {string} line is degraded and names {string}', function (name: string, cure: string) {
  const value = line(state(this), name);
  assert.ok(value.startsWith('degraded'), value);
  assert.ok(value.includes(cure), value);
});

// ── Where the device keeps its things (the device layout) ───────────────────────────────────
/** `epr device layout <verb>` for this workspace, with its named executables' place if any. */
function layoutRun(f: Fixture, args: string[], bins: string | undefined, expected: number | null) {
  const env = deviceEnv(f, f.keyB, bins ? { BIN_STORE: bins } : {});
  return run(f, [env.EPR_BIN ?? binary, 'device', 'layout', ...args], env, expected);
}
function location(f: Fixture, field: string): { path?: string; origin: ObjectValue } {
  assert.ok(f.layout, 'the layout has not been shown');
  const found = object(f.layout.value[field]);
  return { path: found.path as string | undefined, origin: object(found.origin) };
}

Given('this workspace names its own place for executables', function () {
  const f = state(this);
  f.bins = join(f.base, 'named-bins');
});

Given('this device remembered a place for its executables', function () {
  const f = state(this);
  const remembered = join(f.base, 'remembered-bins');
  const saved = layoutRun(f, ['save', '--field', 'binStore'], remembered, 0);
  assert.ok(saved.stdout.includes(remembered), saved.stdout);
});

Given('this workspace names a different place for executables', function () {
  const f = state(this);
  f.bins = join(f.base, 'named-bins');
});

When("this device's layout is shown", function () {
  const f = state(this);
  const r = layoutRun(f, ['show', '--json'], f.bins, null);
  assert.notEqual(r.status, null, `layout show did not run\n${r.stderr}`);
  f.layout = { value: object(JSON.parse(r.stdout)), status: r.status as number };
});

Then(
  'the executables are kept in the place this workspace named, chosen by {string}',
  function (variable: string) {
    const f = state(this);
    const bins = location(f, 'binStore');
    assert.equal(bins.path, f.bins);
    assert.deepEqual(bins.origin, { kind: 'env', name: variable });
  }
);

Then(
  'the checkout named is the repository this workspace stands in, found where it stands',
  function () {
    const f = state(this);
    const root = location(f, 'projectRoot');
    assert.equal(root.path, f.root);
    assert.deepEqual(root.origin, { kind: 'discovered' });
  }
);

Then("the config place is under this workspace's own config home", function () {
  const f = state(this);
  const config = location(f, 'configDir');
  // The fixture names its key file explicitly, so the config place is that key's directory.
  assert.equal(config.path, dirname(f.keyB));
  assert.deepEqual(config.origin, { kind: 'env', name: 'ELOHIM_DEVICE_KEY_FILE' });
});

Then('the remembered place is reported beside it as a difference', function () {
  const f = state(this);
  assert.ok(f.layout);
  const differs = f.layout.value.differs as ObjectValue[];
  assert.deepEqual(differs, [
    { field: 'binStore', env: f.bins, saved: join(f.base, 'remembered-bins') },
  ]);
});

Then('showing the layout succeeds', function () {
  const f = state(this);
  assert.equal(f.layout?.status, 0);
});

// ---------------------------------------------------------------------------
// A device's home is durable or ephemeral, and the native tool is told which.
// The child runs with the environment of a person's own machine: no key override, no
// XDG_CONFIG_HOME (so the key lands under $HOME), and NO ephemeral-platform marker unless the
// scenario sets one. deviceEnv builds a fresh env rather than inheriting this process's, which is
// itself a Che workspace; the explicit allow-list below removes anything that would leak back in.
// ---------------------------------------------------------------------------
const MARKERS = ['DEVWORKSPACE_ID', 'CHE_WORKSPACE_ID', 'DEVWORKSPACE_NAMESPACE'];

function homeDevice(f: Fixture, name: string): HomeDevice {
  const home = join(f.base, `home-${name}`);
  mkdirSync(home, { recursive: true });
  return { home, key: join(home, '.config/elohim/device/ed25519.seed') };
}
function homeEnv(f: Fixture, d: HomeDevice, keyOverride?: string): NodeJS.ProcessEnv {
  const env = deviceEnv(f, f.keyB, { HOME: d.home, ...d.marker });
  delete env.XDG_CONFIG_HOME;
  delete env.ELOHIM_DEVICE_KEY_FILE;
  if (keyOverride) env.ELOHIM_DEVICE_KEY_FILE = keyOverride;
  for (const name of MARKERS) {
    if (!d.marker || !(name in d.marker)) delete env[name];
  }
  return env;
}
function currentHome(f: Fixture): HomeDevice {
  assert.ok(f.home, 'no home device has been set up');
  return f.home;
}
function sessionOpen(f: Fixture, d: HomeDevice, keyOverride?: string) {
  // `session open` leaves a detached holder process that inherits its stdio, and spawnSync's
  // pipes would wait for it; the child writes to files instead, so the call returns at exit.
  const out = join(f.base, `open-${Date.now()}.out`);
  const err = join(f.base, `open-${Date.now()}.err`);
  const fds = [openSync(out, 'w'), openSync(err, 'w')];
  const result = spawnSync(binary, ['session', 'open', '--model', 'fixture-model'], {
    cwd: f.root,
    // The declared session is owned by this test process, the child's parent.
    env: { ...homeEnv(f, d, keyOverride), ELOHIM_SESSION_OWNER_PID: String(process.pid) },
    stdio: ['ignore', fds[0], fds[1]],
    timeout: 30_000,
  });
  fds.forEach(closeSync);
  d.exit = {
    status: result.status,
    stdout: readFileSync(out, 'utf8'),
    stderr: readFileSync(err, 'utf8'),
  };
  return d.exit;
}
/** Every session record under a key directory: none when the directory was never created. */
function sessionRecords(keyPath: string): string[] {
  const dir = join(dirname(keyPath), 'sessions');
  return existsSync(dir) ? readdirSync(dir) : [];
}
function bindHomeDevice(f: Fixture, d: HomeDevice) {
  const env = homeEnv(f, d);
  const request = epr(f, env, ['actor', 'device', 'enroll', '--handle', HANDLE, '--json']);
  const authorization = epr(f, deviceEnv(f, f.keyA), [
    'actor',
    'device',
    'authorize',
    JSON.stringify(request),
    '--json',
  ]);
  epr(f, env, ['actor', 'device', 'bind', JSON.stringify(authorization), '--json']);
  assert.ok(existsSync(d.key), 'the device key was minted under its home');
}
function declare(f: Fixture, d: HomeDevice, expected: number | null = 0) {
  return run(
    f,
    [binary, 'actor', 'device', 'persistence', '--declare', 'durable-home', '--root', f.root],
    homeEnv(f, d),
    expected
  );
}

Given('a device whose key lives under a home that survives restarts', function () {
  const f = state(this);
  f.home = homeDevice(f, 'laptop');
  assert.ok(!existsSync(f.home.key));
});

Given('the device stands for {string} on the roster', function (handle: string) {
  const f = state(this);
  assert.equal(handle, HANDLE);
  bindHomeDevice(f, currentHome(f));
  assert.ok(rosterRows(f).length >= 2, 'the roster carries this device');
});

Given('no container platform marker is set', function () {
  const f = state(this);
  const d = currentHome(f);
  assert.equal(d.marker, undefined);
  const env = homeEnv(f, d);
  for (const name of MARKERS) assert.equal(env[name], undefined, `${name} leaked into the child`);
});

When('the person declares the home durable on that device', function () {
  const f = state(this);
  const d = currentHome(f);
  d.exit = declare(f, d);
});

Then('a persistence declaration signed by the device key sits beside the key', function () {
  const d = currentHome(state(this));
  const path = join(dirname(d.key), DECLARATION);
  assert.ok(existsSync(path), `no declaration at ${path}`);
  assert.equal(statSync(path).mode & 0o777, 0o600);
  const body = object(JSON.parse(readFileSync(path, 'utf8')));
  assert.equal(body.basis, 'durable-home');
  assert.match(String(body.device), /^did:key:/);
  assert.match(String(body.sig), /^[0-9a-f]+$/);
  const answer = object(JSON.parse(d.exit?.stdout ?? '{}'));
  assert.equal(answer.act, 'declared', d.exit?.stdout);
});

Then('declaring a second time reports the existing declaration and rewrites nothing', function () {
  const f = state(this);
  const d = currentHome(f);
  const path = join(dirname(d.key), DECLARATION);
  const before = readFileSync(path, 'hex');
  const mtime = statSync(path).mtimeMs;
  const again = declare(f, d);
  const answer = object(JSON.parse(again.stdout));
  assert.equal(answer.act, 'already-declared', again.stdout);
  assert.equal(readFileSync(path, 'hex'), before, 'the declaration bytes changed');
  assert.equal(statSync(path).mtimeMs, mtime, 'the declaration file was rewritten');
});

Then('a native session opens on the device with the basis {string}', function (basis: string) {
  const f = state(this);
  const d = currentHome(f);
  const opened = sessionOpen(f, d);
  assert.equal(opened.status, 0, `${opened.stdout}\n${opened.stderr}`);
  // `session open` prints the envelope, which does not carry the basis; the basis is read from
  // the same resolver the open consulted, under the same environment.
  const read = run(
    f,
    [binary, 'actor', 'device', 'persistence', '--root', f.root],
    homeEnv(f, d),
    0
  );
  assert.equal(object(JSON.parse(read.stdout)).basis, basis, read.stdout);
  assert.ok(sessionRecords(d.key).length > 0, 'the session left a record beside the key');
});

When('the arrival preflight runs on that device( again)', function () {
  const f = state(this);
  // homeEnv removes every container-platform marker: this test host is itself a Che workspace.
  f.preflight = preflight(f, homeEnv(f, currentHome(f)));
});

Then(
  'the {string} line reads ok and names the persistence basis {string}',
  function (name: string, basis: string) {
    const l = line(state(this), name);
    assert.ok(l.startsWith('ok'), l);
    assert.ok(l.includes(`persistence ${basis}`), `${l}\n  does not name the basis ${basis}`);
  }
);

Given(
  'a workspace whose key lives under $HOME and whose platform sets DEVWORKSPACE_ID',
  function () {
    const f = state(this);
    f.home = homeDevice(f, 'che');
    f.home.marker = { DEVWORKSPACE_ID: 'workspace-fixture' };
  }
);

Given('the workspace stands for {string} on the roster', function (handle: string) {
  const f = state(this);
  assert.equal(handle, HANDLE);
  const d = currentHome(f);
  // Enrolling and binding are not refused by the marker; only the persistence decision is.
  bindHomeDevice(f, d);
});

Given('a persistence declaration signed by that key sits beside the key', function () {
  const f = state(this);
  const d = currentHome(f);
  // The declaration is signed on an honest home (no marker): declaring is itself refused on a
  // platform that announces itself, which is the scenario's last assertion.
  const honest: HomeDevice = { home: d.home, key: d.key };
  declare(f, honest);
  assert.ok(existsSync(join(dirname(d.key), DECLARATION)));
});

When('a native session is opened there', function () {
  const f = state(this);
  const d = currentHome(f);
  sessionOpen(f, d);
});

Then('it is refused with {string}', function (code: string) {
  const d = currentHome(state(this));
  const out = d.exit;
  assert.ok(out && out.status !== 0, `expected a refusal: ${JSON.stringify(out)}`);
  assert.ok(`${out.stdout}${out.stderr}`.includes(code), `${out.stdout}\n${out.stderr}`);
});

Then(
  'the refusal names setting XDG_CONFIG_HOME to a persistent directory outside $HOME',
  function () {
    const out = currentHome(state(this)).exit;
    const text = `${out?.stdout}${out?.stderr}`;
    assert.ok(
      text.includes('set XDG_CONFIG_HOME to an absolute persistent config directory outside $HOME'),
      text
    );
  }
);

Then('no session record was written', function () {
  const d = currentHome(state(this));
  assert.deepEqual(sessionRecords(d.key), []);
});

Then('declaring the home durable there is refused the same way', function () {
  const f = state(this);
  const d = currentHome(f);
  const refused = declare(f, d, null);
  assert.notEqual(refused.status, 0, refused.stdout);
  const text = `${refused.stdout}${refused.stderr}`;
  assert.ok(text.includes('envelope-home-ephemeral-platform'), text);
  assert.ok(text.includes('set XDG_CONFIG_HOME to an absolute persistent config directory'), text);
});

Given('the persistence declaration beside its key was copied from another device', function () {
  const f = state(this);
  const d = currentHome(f);
  const path = join(dirname(d.key), DECLARATION);
  // This device's own declaration, kept so the story's later "moved together" step moves the
  // declaration that really belongs to this key.
  declare(f, d);
  d.ownDeclaration = readFileSync(path, 'utf8');
  // Another device, on its own durable home, declares; its note is copied over this one's.
  const other = homeDevice(f, 'other-laptop');
  epr(f, homeEnv(f, other), ['actor', 'device', 'enroll', '--handle', HANDLE, '--json']);
  declare(f, other);
  writeFileSync(path, readFileSync(join(dirname(other.key), DECLARATION)), { mode: 0o600 });
});

Then('it is refused with {string} naming the device mismatch', function (code: string) {
  const d = currentHome(state(this));
  const text = `${d.exit?.stdout}${d.exit?.stderr}`;
  assert.notEqual(d.exit?.status, 0, text);
  assert.ok(text.includes(code), text);
  assert.ok(text.includes('device mismatch'), text);
});

When('the key and its declaration are instead moved together into another directory', function () {
  const f = state(this);
  const d = currentHome(f);
  assert.ok(d.ownDeclaration, "this device's own declaration was not kept");
  const dir = join(d.home, 'moved-elsewhere');
  mkdirSync(dir, { recursive: true });
  const key = join(dir, 'ed25519.seed');
  copyFileSync(d.key, key);
  writeFileSync(join(dir, DECLARATION), d.ownDeclaration, { mode: 0o600 });
  d.moved = { key, dir };
});

When('a native session is opened from that directory', function () {
  const f = state(this);
  const d = currentHome(f);
  assert.ok(d.moved);
  sessionOpen(f, d, d.moved.key);
});

Then('it is refused with {string} naming the directory mismatch', function (code: string) {
  const d = currentHome(state(this));
  const text = `${d.exit?.stdout}${d.exit?.stderr}`;
  assert.notEqual(d.exit?.status, 0, text);
  assert.ok(text.includes(code), text);
  assert.ok(text.includes('directory mismatch'), text);
});

Then('no session record was written in either place', function () {
  const d = currentHome(state(this));
  assert.ok(d.moved);
  assert.deepEqual(sessionRecords(d.key), []);
  assert.deepEqual(sessionRecords(d.moved.key), []);
});

// ── Publication, sibling exchange and the next step ─────────────────────────────────────────
const ROSTER_REL = `.eprfs/status/participants/${HANDLE}.jsonl`;
const PUBLISHED_REF = 'refs/remotes/origin/dev'; // the declared remote-tracking ref (publishedRef)
const AS_OF_LAST_FETCH = 'as of the last fetch';
const SIBLING_EXCHANGE = 'sibling exchange';
const NEXT_STEP = 'next step';
/** The ceremony's order, as the feature's glossary states it. */
const CEREMONY = ['epr binary', 'device key', 'roster bound', 'peer daemon', SIBLING_EXCHANGE];

function commitRoster(f: Fixture, message: string) {
  git(f, ['-C', f.root, 'add', ROSTER_REL]);
  git(f, ['-C', f.root, 'commit', '-q', '-m', message]);
}
/** Stands in for a push through the gate followed by a fetch: the remote-tracking ref moves. */
function fetchedAt(f: Fixture, revision: string) {
  git(f, ['-C', f.root, 'update-ref', PUBLISHED_REF, revision]);
}
function didOf(note: ObjectValue | undefined, field: string): string {
  const did = String(note?.[field] ?? '');
  assert.ok(did.startsWith('did:key:'), `no ${field} did in ${JSON.stringify(note)}`);
  return did;
}
/** A syntactically whole channel root CID (CIDv1, raw, BLAKE3-256 multihash) over fixed bytes. */
function fixtureRoot(seed: string): string {
  const bytes = Buffer.concat([Buffer.from([0x01, 0x55, 0x1e, 0x20]), Buffer.alloc(32, seed)]);
  const alphabet = 'abcdefghijklmnopqrstuvwxyz234567';
  let bits = 0;
  let value = 0;
  let out = 'b';
  for (const byte of bytes) {
    value = (value << 8) | byte;
    bits += 8;
    while (bits >= 5) {
      out += alphabet[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += alphabet[(value << (5 - bits)) & 31];
  return out;
}
/**
 * The head table this workspace's peer process saves (eprfs-peer `HeadTable`, `heads.json` in
 * `<XDG_CONFIG_HOME>/epr/peer`), holding the other device's head when one has arrived.
 */
function savePeerStatus(f: Fixture, otherHead?: { seq: number; acks?: number }) {
  const env = deviceEnv(f, f.keyB);
  const dir = join(String(env.XDG_CONFIG_HOME), 'epr/peer');
  mkdirSync(dir, { recursive: true });
  const now = Date.now();
  const devices: Record<string, unknown> = {};
  if (otherHead) {
    const ours = didOf(f.request, 'controller');
    devices[didOf(f.authorization, 'authorizedBy')] = {
      seq: otherHead.seq,
      root: fixtureRoot(`other-${otherHead.seq}`),
      at: now,
      folded: otherHead.acks === undefined ? {} : { [ours]: otherHead.acks },
      folded_at: now,
      events_folded: otherHead.seq,
    };
  }
  const table = {
    version: 1,
    own: null,
    devices,
    dropped: { unadmitted: 0, forged: 0, rollback: 0, replayed: 0, malformed: 0 },
  };
  writeFileSync(join(dir, 'heads.json'), `${JSON.stringify(table, null, 2)}\n`);
}

Given(
  "the integration branch, as last fetched here, carries matthew's roster with only the other device on it",
  function () {
    const f = state(this);
    const published = gitOut(f, ['-C', f.root, 'show', `${PUBLISHED_REF}:${ROSTER_REL}`]);
    assert.equal(published.split('\n').filter(l => l.trim()).length, 1, published);
  }
);

Given('this workspace has never fetched the integration branch', function () {
  const f = state(this);
  git(f, ['-C', f.root, 'update-ref', '-d', PUBLISHED_REF]);
});

Given('this workspace has joined {string} through the other device', function (handle: string) {
  const f = state(this);
  assert.equal(handle, HANDLE);
  joinHere(f);
  assert.equal(rosterRows(f).length, 2, 'the binding is on the roster here');
});

Given("this workspace's binding is committed here but not yet published", function () {
  commitRoster(state(this), 'bind this workspace');
});

When('the binding reaches the integration branch and is fetched here', function () {
  fetchedAt(state(this), 'HEAD');
});

Then(
  'the {string} line is degraded and says the binding is local only, as of the last fetch',
  function (name: string) {
    const l = line(state(this), name);
    assert.ok(l.startsWith('degraded'), l);
    assert.ok(l.includes('local only') && l.includes(AS_OF_LAST_FETCH), l);
    assert.ok(l.includes(HANDLE), `${l}\n  does not name the person`);
  }
);

Then(
  'the {string} line reads ok and says the binding is published, as of the last fetch',
  function (name: string) {
    const l = line(state(this), name);
    assert.ok(l.startsWith('ok'), l);
    assert.ok(l.includes(`published on origin/dev (${AS_OF_LAST_FETCH})`), l);
  }
);

Then('the {string} line is unknown and names {string}', function (name: string, cure: string) {
  const l = line(state(this), name);
  assert.ok(l.startsWith('unknown'), l);
  assert.ok(l.includes(cure), `${l}\n  does not name ${cure}`);
});

Given(
  "this workspace's peer has saved its status, and nothing has arrived from the other device",
  function () {
    savePeerStatus(state(this));
  }
);

When(
  "the other device's head arrives here without acknowledging this workspace's head",
  function () {
    savePeerStatus(state(this), { seq: 3 });
  }
);

When("the other device's next head acknowledges this workspace's head", function () {
  savePeerStatus(state(this), { seq: 4, acks: 1 });
});

Then(
  'the {string} line is degraded and says no head from a sibling has arrived',
  function (name: string) {
    const l = line(state(this), name);
    assert.ok(l.startsWith('degraded'), l);
    assert.ok(l.includes('no verified head') && l.includes('nothing has been exchanged'), l);
  }
);

Then(
  "the {string} line is degraded and says no sibling acknowledges this workspace's head",
  function (name: string) {
    const l = line(state(this), name);
    assert.ok(l.startsWith('degraded'), l);
    assert.ok(l.includes("no sibling acknowledges this device's head"), l);
  }
);

Then('the {string} line reads ok and names the other device', function (name: string) {
  const f = state(this);
  const l = line(f, name);
  assert.ok(l.startsWith('ok'), l);
  const other = didOf(f.authorization, 'authorizedBy');
  // The line names the sibling by its short form: the did's first sixteen and last four characters.
  assert.ok(l.includes(other.slice(0, 16)) && l.includes(other.slice(-4)), `${l}\n  ${other}`);
  assert.ok(l.includes('ago'), `${l}\n  does not say how long ago`);
});

Then(
  "the last line is the next step, repeating the cure of the first unmet condition in the ceremony's order",
  function () {
    const f = state(this);
    assert.ok(f.preflight, 'the preflight has not run');
    const lines = f.preflight.lines;
    const last = lines.at(-1) ?? '';
    assert.equal(
      lines.filter(l => l.includes(` ${NEXT_STEP}: `)).length,
      1,
      'exactly one next-step line'
    );
    assert.ok(!last.startsWith('REFUSED'), `the next step refused: ${last}`);
    const owed = CEREMONY.map(name => line(f, name)).find(
      l => !l.startsWith('ok ') && l.includes(' — ')
    );
    if (!owed) {
      assert.match(last, /^ok next step: nothing owed/, last);
      return;
    }
    const name = CEREMONY.find(n => owed.includes(` ${n}: `));
    const prefix = ` ${NEXT_STEP}: ${name} — `;
    assert.ok(last.includes(prefix), `${last}\n  does not name ${name}`);
    const cure = last.slice(last.indexOf(prefix) + prefix.length);
    assert.ok(owed.endsWith(` — ${cure}`), `${last}\n  does not repeat the cure of ${owed}`);
  }
);
