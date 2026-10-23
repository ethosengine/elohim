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
 * The `epr` used is $EPR_BIN, else the eprfs gate target's debug build; it must be newer than every
 * tracked eprfs source, or the scenario fails naming the build command (a stale binary once passed
 * a smoke by luck).
 */
import assert from 'node:assert/strict';
import { type ChildProcess, spawn, spawnSync } from 'node:child_process';
import { createSocket } from 'node:dgram';
import {
  chmodSync,
  closeSync,
  copyFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  openSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';

import { After, Before, Given, Then, When } from '@cucumber/cucumber';

const TAG = '@concern:devices-share-one-berth';
const repository = resolve('../..');
const HANDLE = 'matthew';
// The eprfs gate's own debug target; only read (its mtime and the binary), never written here.
// eslint-disable-next-line sonarjs/publicly-writable-directories
const EPR = process.env.EPR_BIN ?? '/tmp/eprfs-gate-target/debug/epr';
const POLICY = join(repository, 'genesis/agentic/pool-policy.json');
const ROSTER = `.eprfs/status/participants/${HANDLE}.jsonl`;
const DEV = 'dev-integration';
const STEWARD = new Set([DEV, 'alpha-fleet', 'jenkins-dispatch']);

type Label = 'A' | 'B';
interface Outcome {
  status: number | null;
  stdout: string;
  stderr: string;
  ms: number;
}
interface Device {
  label: Label;
  dir: string;
  config: string;
  berthDir: string;
  repo: string;
  log: string;
  did: string;
  node: string;
  port: number;
  daemon?: ChildProcess;
  daemonStarts: number;
}
interface Fixture {
  base: string;
  origin: string;
  baseSha: string;
  devices: Record<Label, Device>;
  last?: Outcome;
  push?: Outcome;
  clockBase?: number;
  pending: Partial<Record<Label, { started: number; outcome: Promise<Outcome> }>>;
  outcomes: Partial<Record<Label, Outcome>>;
  linkBackAt?: number;
}
const fixtures = new WeakMap<object, Fixture>();

function state(world: object): Fixture {
  const f = fixtures.get(world);
  assert.ok(f, 'the Background did not run');
  return f;
}
const other = (label: Label): Label => (label === 'A' ? 'B' : 'A');
const sleep = (ms: number) => new Promise(done => setTimeout(done, ms));

/** The environment one device's processes see: its own config home, berth store and checkout. */
function env(d: Device, extra: Record<string, string> = {}): NodeJS.ProcessEnv {
  const keep = ['PATH', 'LANG', 'GIT_CONFIG_COUNT', 'GIT_CONFIG_KEY_0', 'GIT_CONFIG_VALUE_0'];
  const out: NodeJS.ProcessEnv = {};
  for (const key of keep) if (process.env[key] !== undefined) out[key] = process.env[key];
  return {
    ...out,
    HOME: join(d.dir, 'home'),
    XDG_CONFIG_HOME: d.config,
    BERTH_DIR: d.berthDir,
    CLAUDE_CONFIG_DIR: join(d.dir, 'claude-config'),
    CLAUDE_PROJECT_DIR: repository,
    EPR_BIN: EPR,
    ...extra,
  };
}

function runSync(argv: string[], options: { cwd?: string; env?: NodeJS.ProcessEnv } = {}): Outcome {
  const started = Date.now();
  const r = spawnSync(argv[0], argv.slice(1), {
    cwd: options.cwd,
    env: options.env ?? process.env,
    encoding: 'utf8',
    timeout: 60_000,
    maxBuffer: 8 * 1024 * 1024,
  });
  return {
    status: r.status,
    stdout: r.stdout ?? '',
    stderr: r.stderr ?? '',
    ms: Date.now() - started,
  };
}

function run(
  argv: string[],
  options: { cwd?: string; env?: NodeJS.ProcessEnv } = {}
): Promise<Outcome> {
  const started = Date.now();
  return new Promise(done => {
    const child = spawn(argv[0], argv.slice(1), { cwd: options.cwd, env: options.env });
    let stdout = '';
    let stderr = '';
    child.stdout.on('data', chunk => (stdout += chunk));
    child.stderr.on('data', chunk => (stderr += chunk));
    child.on('close', status => done({ status, stdout, stderr, ms: Date.now() - started }));
  });
}

function must(o: Outcome, what: string): Outcome {
  assert.equal(o.status, 0, `${what} exited ${o.status}\n${o.stdout}\n${o.stderr}`);
  return o;
}

function epr(d: Device, args: string[]): Outcome {
  return runSync([EPR, ...args], { cwd: d.repo, env: env(d) });
}

function eprJson(d: Device, args: string[]): Record<string, unknown> {
  return JSON.parse(must(epr(d, args), `epr ${args.join(' ')} on ${d.label}`).stdout);
}

function berthArgv(d: Device): string[] {
  return ['python3', join(d.repo, 'genesis/agentic/bin/berth')];
}

function berth(d: Device, args: string[], extra: Record<string, string> = {}): Promise<Outcome> {
  return run([...berthArgv(d), ...args], { cwd: d.repo, env: env(d, extra) });
}

function git(cwd: string, args: string[], extra: NodeJS.ProcessEnv = {}): Outcome {
  return runSync(
    ['git', '-c', 'user.name=Fixture', '-c', 'user.email=fixture@example.invalid', ...args],
    { cwd, env: { ...process.env, HOME: cwd, ...extra } }
  );
}

/** `epr peer leases --json` on device `d`: its standing, its channel store and its lease view. */
function leases(d: Device): Record<string, unknown> {
  return eprJson(d, ['peer', 'leases', '--json', '--root', d.repo, '--policy', POLICY]);
}

function verdict(d: Device, resource: string): Record<string, unknown> | undefined {
  const v = leases(d)[resource];
  return v && typeof v === 'object' ? (v as Record<string, unknown>) : undefined;
}

function peerStatus(d: Device): {
  ownSeq: number | null;
  devices: { device: string; seq: number | null; ageS: number | null }[];
} {
  return eprJson(d, ['peer', 'status', '--json', '--root', d.repo]) as never;
}

async function waitFor(what: string, timeoutMs: number, check: () => boolean): Promise<void> {
  const deadline = Date.now() + timeoutMs;
  for (;;) {
    let ok = false;
    try {
      ok = check();
    } catch {
      ok = false;
    }
    if (ok) return;
    if (Date.now() > deadline) assert.fail(`timed out after ${timeoutMs} ms waiting for: ${what}`);
    await sleep(200);
  }
}

function freePort(): Promise<number> {
  return new Promise((done, fail) => {
    const socket = createSocket('udp4');
    socket.on('error', fail);
    socket.bind(0, '127.0.0.1', () => {
      const { port } = socket.address();
      socket.close(() => done(port));
    });
  });
}

function startDaemon(f: Fixture, d: Device): void {
  const peer = f.devices[other(d.label)];
  const log = openSync(d.log, 'a');
  d.daemon = spawn(
    EPR,
    [
      'peer',
      'serve',
      '--root',
      d.repo,
      '--listen',
      `127.0.0.1:${d.port}`,
      '--peer',
      `${peer.node}@127.0.0.1:${peer.port}`,
      '--policy',
      POLICY,
    ],
    { cwd: d.repo, env: env(d), stdio: ['ignore', log, log] }
  );
  closeSync(log);
  d.daemonStarts += 1;
}

async function stopDaemon(d: Device): Promise<void> {
  const daemon = d.daemon;
  if (daemon?.exitCode !== null || daemon.signalCode !== null) return;
  const exited = new Promise(done => daemon.once('exit', done));
  daemon.kill('SIGTERM');
  const killed = await Promise.race([exited.then(() => true), sleep(5000).then(() => false)]);
  if (!killed) {
    daemon.kill('SIGKILL');
    await exited;
  }
}

/** Device `d` has folded `from`'s head as it stands now (a head at least as new as `from`'s own). */
async function foldedCurrentHead(d: Device, from: Device, timeoutMs = 20_000): Promise<void> {
  let own: number | null = null;
  await waitFor(`${d.label} folds ${from.label}'s current head`, timeoutMs, () => {
    own ??= peerStatus(from).ownSeq;
    const seen = peerStatus(d).devices.find(x => x.device === from.did);
    return own !== null && seen !== undefined && seen.seq !== null && seen.seq >= own;
  });
}

function ledger(d: Device): Record<string, unknown>[] {
  const path = join(d.berthDir, 'ledger.jsonl');
  if (!existsSync(path)) return [];
  return readFileSync(path, 'utf8')
    .split('\n')
    .filter(line => line.trim())
    .map(line => JSON.parse(line));
}

/** When `session` on `d` claimed `resource`, as its berth stamped it (RFC 3339, whole seconds). */
function claimedAt(d: Device, session: string, resource: string): string {
  const row = ledger(d)
    .filter(r => r.kind === 'claim' && r.session === session && r.resource === resource)
    .pop();
  assert.ok(row, `no claim of ${resource} by ${session} on ${d.label}`);
  return rfc3339(Number(row.claimed_at));
}

function rfc3339(seconds: number): string {
  return new Date(Math.floor(seconds) * 1000).toISOString().replace(/\.\d{3}Z$/, 'Z');
}

/** `berth status` on `d`: the block it prints for device `did` among the other devices. */
function otherDeviceBlock(status: string, did: string): string[] {
  const section = status.split('other devices')[1] ?? '';
  const lines = section.split('\n');
  const start = lines.findIndex(l => l.startsWith(`    device ${did}`));
  assert.ok(start >= 0, `berth status lists no device ${did}:\n${status}`);
  const block = [lines[start]];
  for (const l of lines.slice(start + 1)) {
    if (!l.startsWith('      ')) break;
    block.push(l);
  }
  return block;
}

function refusalLine(o: Outcome, resource: string): string {
  const line = `${o.stdout}\n${o.stderr}`
    .split('\n')
    .find(l => l.startsWith(`${resource}: REFUSED — `));
  assert.ok(line, `no refusal line for ${resource}:\n${o.stdout}\n${o.stderr}`);
  return line;
}

function holds(d: Device, resource: string, session: string): boolean {
  const who = JSON.parse(
    must(runSync([...berthArgv(d), 'who', resource], { env: env(d) }), 'berth who').stdout
  );
  let mine: { holder?: string }[] = [];
  if (Array.isArray(who)) mine = who;
  else if (who) mine = [who];
  return mine.some((lease: { holder?: string }) => lease.holder === session);
}

// ---------------------------------------------------------------- the fixture

let eprChecked: string | null = null;

/** The `epr` must exist and be newer than every tracked eprfs source: fail loudly otherwise. */
function checkEpr(): void {
  if (eprChecked === EPR) return;
  const build =
    'CARGO_TARGET_DIR=/tmp/eprfs-gate-target RUSTFLAGS= cargo build --manifest-path ' +
    'elohim/eprfs/Cargo.toml -p elohim-epr-cli --bin epr (or set EPR_BIN)';
  assert.ok(existsSync(EPR), `no epr at ${EPR} — build it: ${build}`);
  const files = must(
    runSync(['git', 'ls-files', 'elohim/eprfs'], { cwd: repository }),
    'git ls-files'
  ).stdout.split('\n');
  let newest = 0;
  let newestFile = '';
  for (const file of files) {
    if (!/(\.rs|Cargo\.toml|Cargo\.lock)$/.test(file)) continue;
    const mtime = statSync(join(repository, file)).mtimeMs;
    if (mtime > newest) [newest, newestFile] = [mtime, file];
  }
  assert.ok(
    statSync(EPR).mtimeMs >= newest,
    `${EPR} is older than ${newestFile} — a stale epr measures old code; rebuild: ${build}`
  );
  eprChecked = EPR;
}

Before({ tags: TAG }, function () {
  checkEpr();
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

function makeDevice(
  base: string,
  name: string,
  origin: string
): Omit<Device, 'label' | 'did' | 'node' | 'port'> {
  const dir = join(base, name);
  const repo = join(dir, 'repo');
  for (const sub of ['config', 'berth', 'home', 'claude-config'])
    mkdirSync(join(dir, sub), { recursive: true });
  must(git(dir, ['clone', '-q', '-b', 'dev', origin, repo]), 'clone');
  // berth folds into the flow store of the checkout it lives in: the one this device's daemon serves.
  mkdirSync(join(repo, '.claude'), { recursive: true });
  symlinkSync(join(repository, '.claude/hooks'), join(repo, '.claude/hooks'));
  symlinkSync(join(repository, '.claude/epr-meta'), join(repo, '.claude/epr-meta'));
  // The real pre-push hook, as this checkout's own hook (no git config is changed).
  copyFileSync(join(repository, '.husky/pre-push.bash'), join(repo, '.git/hooks/pre-push'));
  chmodSync(join(repo, '.git/hooks/pre-push'), 0o755);
  return {
    dir,
    config: join(dir, 'config'),
    berthDir: join(dir, 'berth'),
    repo,
    log: join(dir, 'serve.log'),
    daemonStarts: 0,
  };
}

Given(
  'two devices of {string}, {string} and {string}, each with its own berth, its own key and its own peer daemon',
  { timeout: 120_000 },
  async function (person: string, first: string, second: string) {
    assert.deepEqual([person, first, second], [HANDLE, 'A', 'B']);
    const base = mkdtempSync(join(tmpdir(), 'devices-share-one-berth-'));
    // The repository both devices integrate on: a bare origin whose dev holds berth.
    const seed = join(base, 'seed');
    const origin = join(base, 'origin.git');
    mkdirSync(join(seed, '.epr-meta'), { recursive: true });
    mkdirSync(join(seed, 'genesis/agentic/bin'), { recursive: true });
    writeFileSync(
      join(seed, '.epr-meta/repository.yaml'),
      'version: 1\nagent: repo:ethosengine/elohim\n'
    );
    writeFileSync(join(seed, 'README.md'), '# devices-share-one-berth fixture\n');
    writeFileSync(join(seed, '.gitignore'), '.claude/\n.eprfs/status/flows.jsonl\n');
    copyFileSync(
      join(repository, 'genesis/agentic/bin/berth'),
      join(seed, 'genesis/agentic/bin/berth')
    );
    must(git(base, ['init', '-q', '--bare', origin]), 'init origin');
    must(git(seed, ['init', '-q', '-b', 'dev']), 'init seed');
    must(git(seed, ['add', '-A']), 'add');
    must(git(seed, ['commit', '-q', '-m', 'fixture']), 'commit');
    must(git(seed, ['push', '-q', origin, 'dev']), 'seed origin');
    const baseSha = must(git(seed, ['rev-parse', 'HEAD']), 'rev-parse').stdout.trim();

    // Two devices, X first on the roster; each mints its own keys on first use.
    const x = { ...makeDevice(base, 'x', origin), label: 'A' as Label, did: '', node: '', port: 0 };
    const y = { ...makeDevice(base, 'y', origin), label: 'B' as Label, did: '', node: '', port: 0 };
    const carry = (from: Device, to: Device) => {
      mkdirSync(join(to.repo, '.eprfs/status/participants'), { recursive: true });
      copyFileSync(join(from.repo, ROSTER), join(to.repo, ROSTER));
    };
    must(
      epr(x, [
        'actor',
        'witness',
        '--subject',
        `human:${HANDLE}`,
        '--as',
        'agent:fixture@claude-opus-5-5',
        '--session',
        'fixture-x',
        '--basis',
        'the devices-share-one-berth fixture',
        '--root',
        x.repo,
      ]),
      'witness'
    );
    carry(x, y);
    const request = join(base, 'request.json');
    const authorization = join(base, 'authorization.json');
    writeFileSync(
      request,
      must(epr(y, ['actor', 'device', 'enroll', '--handle', HANDLE, '--root', y.repo]), 'enroll')
        .stdout
    );
    writeFileSync(
      authorization,
      must(epr(x, ['actor', 'device', 'authorize', request, '--root', x.repo]), 'authorize').stdout
    );
    must(epr(y, ['actor', 'device', 'bind', authorization, '--root', y.repo]), 'bind');
    carry(y, x);
    for (const d of [x, y]) d.node = must(epr(d, ['peer', 'id']), 'peer id').stdout.trim();
    must(
      epr(x, ['actor', 'device', 'endpoint', '--node-id', x.node, '--root', x.repo]),
      'endpoint x'
    );
    carry(x, y);
    must(
      epr(y, ['actor', 'device', 'endpoint', '--node-id', y.node, '--root', y.repo]),
      'endpoint y'
    );
    carry(y, x);
    for (const d of [x, y]) {
      d.did = String(eprJson(d, ['peer', 'status', '--json', '--root', d.repo]).device);
      d.port = await freePort();
    }
    // "A" is the device whose signing key sorts lower; the story's tie-break examples rely on it.
    const [a, b] = x.did < y.did ? [x, y] : [y, x];
    a.label = 'A';
    b.label = 'B';
    const f: Fixture = {
      base,
      origin,
      baseSha,
      devices: { A: a, B: b },
      pending: {},
      outcomes: {},
    };
    fixtures.set(this, f);
    startDaemon(f, a);
    startDaemon(f, b);
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

// ---------------------------------------------------------------- a retired device (@wip)
//
// Held @wip: three of its steps need what the implementation does not have yet, and they say so
// as `pending` rather than pass on a stand-in — a verb that signs a Contest of ANOTHER device's
// binding on this device (`epr actor contest` contests this device's own standing record), a
// daemon that closes the open connections of a device a reload no longer admits (the gossip gate
// refuses only new ones), and `epr peer status` counting refused connections (the gate's counter
// is in memory only). The push that carries the Contest row also needs the fixture checkout to
// carry the gates the hook runs after its claims.

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
    // No verb signs a Contest of another device's binding: `epr actor contest` contests this
    // device's own standing record.
    return 'pending';
  }
);

Then(
  "device A's berth status no longer lists device B or its lease on {string}, with no pull and no restart",
  async function (resource: string) {
    const { A, B } = state(this).devices;
    const status = must(await berth(A, ['status']), 'berth status on A').stdout;
    assert.ok(!status.includes(B.did), status);
    assert.equal(verdict(A, resource), undefined);
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
  function () {
    // The gossip gate refuses a newly unadmitted device's NEW connections only; an open one stays
    // until it closes.
    return 'pending';
  }
);

Then(
  '{string} on device A counts the refused connections from device B',
  function (command: string) {
    assert.equal(command, 'epr peer status');
    // `epr peer status` does not report refused connections; the gate's count is in memory only.
    return 'pending';
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
  function () {
    // The fixture checkout carries none of the gates the hook runs after its claims.
    return 'pending';
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
