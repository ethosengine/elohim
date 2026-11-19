/**
 * A device judges a sibling's epr from what it says it needs, before fetching any of it.
 *
 * Every scenario builds a private household of two devices out of temporary directories and
 * drives the real `epr` selected by EPR_BIN. Device A is admitted by the roster ceremony
 * (`epr actor witness | device enroll | authorize | bind | endpoint`); A's release offer is the
 * pair of crossed custody Produce records an admitted sibling's flow log carries, appended to B's
 * flow store with the dag-cbor record CIDs the store re-verifies on read. The follow is
 * `epr bin --tool epr follow --repo <B checkout>` with EPR_FOLLOW_SIBLING=1 against B's own config
 * home, so nothing reaches this workspace's device, peer key or bin store.
 *
 * Observation of each Then (stderr of the follow, and B's `incoming` staging directory):
 * - "fetches device A's release": no requirement gap and `fetch gap: daemon-down` — B's own
 *   `epr peer serve` is not up, so reaching that line is the proof the draw proceeded to a fetch.
 * - "fetches nothing": no `fetch gap` line and nothing staged in `<bin store>/incoming`.
 * - "goes on to its own build": `building locally` / `trying a local build` after the gap line; when the draw was
 *   refused before it began (unpublished), the follow's normal result on stdout and exit 0.
 *
 * The C baseline marker is the `env:isa-c=x86-64` slot on each crossed Produce. Every fixture
 * statement carries it unless a scenario removes it, so a refusal is about the one thing the
 * scenario changes; its absence is refused the same way on every processor, so no step reads
 * this host's CPU.
 */
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  appendFileSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readdirSync,
  rmSync,
  copyFileSync,
  writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';

import { After, Given, Then, When } from '@cucumber/cucumber';

import { mintByteCid } from '../../src/framework/byte-address.js';

type Statement = 'meets' | 'newer-libc' | 'predates' | 'mismatch';

interface Household {
  base: string;
  repoA: string;
  repoB: string;
  keyA: string;
  keyB: string;
  didA: string;
  head: string;
  parent: string;
  statement: Statement;
  /** Whether A's records carry the C baseline marker `env:isa-c=x86-64`. */
  marker: boolean;
  stderr?: string;
  stdout?: string;
  status?: number | null;
}

const households = new WeakMap<object, Household>();
const binary = process.env.EPR_BIN ?? 'epr';
const repository = resolve('../..');
const registry = join(
  repository,
  'genesis/orchestrator/manifests/runtime/workspace-device.manifest.json'
);
const HANDLE = 'matthew';
const AGENT_ID = '4f5e3d2c1b0a99887766554433221100ffeeddccbbaa00998877665544332211';
const RELEASE = 'bafyreiaww7ivxkoouk2ojpwpqtm4pm7hxlvlzl3u7bqhrgnxcdhbg7mwmu';
const KEY = 'bafyreigq2n5xqgbtzbb6ce2ebobfqvqtjqdmmqkn3mmdyx5n6k4hgemrzu';

function household(world: object): Household {
  const found = households.get(world);
  assert.ok(found, 'the Background creates the household first');
  return found;
}

// ---- dag-cbor, just enough to mint the record CIDs a flow log re-verifies on read ----------

const ALPHABET = 'abcdefghijklmnopqrstuvwxyz234567';

function base32Decode(text: string): Buffer {
  const out: number[] = [];
  let bits = 0;
  let value = 0;
  for (const c of text) {
    value = (value << 5) | ALPHABET.indexOf(c);
    bits += 5;
    if (bits >= 8) {
      out.push((value >>> (bits - 8)) & 0xff);
      bits -= 8;
    }
  }
  return Buffer.from(out);
}

function base32Encode(bytes: Uint8Array): string {
  let bits = 0;
  let value = 0;
  let out = '';
  for (const b of bytes) {
    value = (value << 8) | b;
    bits += 8;
    while (bits >= 5) {
      out += ALPHABET[(value >>> (bits - 5)) & 31];
      bits -= 5;
    }
  }
  if (bits > 0) out += ALPHABET[(value << (5 - bits)) & 31];
  return out;
}

/** A CID link: dag-cbor tag 42 over 0x00 + the binary CID. */
class Link {
  constructor(readonly text: string) {}
}

function head(major: number, length: number): Buffer {
  const m = major << 5;
  if (length < 24) return Buffer.from([m | length]);
  if (length < 256) return Buffer.from([m | 24, length]);
  if (length < 65536) return Buffer.from([m | 25, length >> 8, length & 0xff]);
  const b = Buffer.alloc(5);
  b[0] = m | 26;
  b.writeUInt32BE(length, 1);
  return b;
}

function cbor(value: unknown): Buffer {
  if (value === null) return Buffer.from([0xf6]);
  if (value instanceof Link) {
    const bytes = Buffer.concat([Buffer.from([0]), base32Decode(value.text.slice(1))]);
    return Buffer.concat([Buffer.from([0xd8, 42]), head(2, bytes.length), bytes]);
  }
  if (typeof value === 'string') {
    const bytes = Buffer.from(value, 'utf8');
    return Buffer.concat([head(3, bytes.length), bytes]);
  }
  if (typeof value === 'number') {
    // Every number in a flow event is an f64 (dag-cbor never narrows floats).
    const b = Buffer.alloc(9);
    b[0] = 0xfb;
    b.writeDoubleBE(value, 1);
    return b;
  }
  if (Array.isArray(value)) {
    return Buffer.concat([head(4, value.length), ...value.map(cbor)]);
  }
  const entries = Object.entries(value as Record<string, unknown>)
    .map(([k, v]) => [Buffer.from(k, 'utf8'), v] as const)
    .sort((a, b) => a[0].length - b[0].length || Buffer.compare(a[0], b[0]));
  return Buffer.concat([
    head(5, entries.length),
    ...entries.flatMap(([k, v]) => [head(3, k.length), k, cbor(v)]),
  ]);
}

/** CIDv1 dag-cbor (0x71) over sha2-256, base32 lower: the record profile (`bafyrei…`). */
function recordCid(canonical: Buffer): string {
  const digest = createHash('sha256').update(canonical).digest();
  return 'b' + base32Encode(Buffer.concat([Buffer.from([0x01, 0x71, 0x12, 0x20]), digest]));
}

// ---- process helpers ------------------------------------------------------------------------

function deviceEnv(h: Household, who: 'a' | 'b'): NodeJS.ProcessEnv {
  const home = join(h.base, `home-${who}`);
  mkdirSync(home, { recursive: true });
  const env: NodeJS.ProcessEnv = {
    PATH: process.env.PATH,
    HOME: home,
    XDG_CONFIG_HOME: join(home, '.config'),
    XDG_CACHE_HOME: join(home, '.cache'),
    ELOHIM_DEVICE_KEY_FILE: who === 'a' ? h.keyA : h.keyB,
    EPR_STORE: join(home, 'bins/epr'),
    BIN_STORE: join(home, 'bins'),
    EPR_PAYLOAD_STORE: join(home, 'payload'),
    EPR_BIN_TOOLS: registry,
    ELOHIM_SESSION_ID: `honest-draw-${who}`,
    ELOHIM_SESSION_OWNER_PID: String(process.pid),
    EPR_UNDER_TEST: '1',
    EPR_FIXTURE_OWNER_PID: String(process.pid),
  };
  if (binary !== 'epr') env.EPR_BIN = binary;
  return env;
}

function spawn(command: string, argv: string[], env: NodeJS.ProcessEnv, cwd: string) {
  return spawnSync(command, argv, {
    cwd,
    env,
    encoding: 'utf8',
    timeout: 120_000,
    maxBuffer: 16 * 1024 * 1024,
  });
}

function epr(h: Household, who: 'a' | 'b', argv: string[], cwd: string, expected = 0) {
  const r = spawn(binary, argv, deviceEnv(h, who), cwd);
  assert.equal(r.status, expected, `epr ${argv.join(' ')}\n${r.stdout}\n${r.stderr}`);
  return r;
}

function json(text: string): Record<string, unknown> {
  return JSON.parse(text) as Record<string, unknown>;
}

function git(h: Household, cwd: string, args: string[]): string {
  const r = spawn(
    'git',
    [
      '-c',
      'core.hooksPath=/dev/null',
      '-c',
      'user.name=Honest Draw Fixture',
      '-c',
      'user.email=honest-draw@example.invalid',
      ...args,
    ],
    { PATH: process.env.PATH, HOME: h.base },
    cwd
  );
  assert.equal(r.status, 0, `git ${args.join(' ')}\n${r.stdout}\n${r.stderr}`);
  return r.stdout.trim();
}

function gitRepo(h: Household, root: string): void {
  mkdirSync(join(root, '.epr-meta'), { recursive: true });
  writeFileSync(
    join(root, '.epr-meta/repository.yaml'),
    'version: 1\nagent: repo:ethosengine/elohim\n'
  );
  writeFileSync(join(root, 'README.md'), 'fixture\n');
  git(h, root, ['init', '-q', '-b', 'main']);
  git(h, root, ['add', '.epr-meta/repository.yaml', 'README.md']);
  git(h, root, ['commit', '-q', '-m', 'fixture parent']);
}

// ---- the release offer: crossed Produce records ---------------------------------------------

function hostArch(): string {
  const names: Record<string, string> = { x64: 'x86_64', arm64: 'aarch64' };
  return names[process.arch] ?? process.arch;
}

/** The requirement document as serde writes host_contract::Requires: clauses ordered by axis. */
function requirementDocument(glibcMinor: number): Buffer {
  const clause = (axis: string, predicate: unknown) => ({
    axis,
    predicate,
    source: 'extracted',
  });
  return Buffer.from(
    JSON.stringify([
      clause('abi.libc', {
        kind: 'at-least',
        value: { line: 'glibc', components: [2, glibcMinor] },
      }),
      clause('arch', { kind: 'equal', value: hostArch() }),
      clause('os', { kind: 'equal', value: 'linux' }),
    ])
  );
}

function statementSlots(statement: Statement): string[] {
  switch (statement) {
    case 'predates':
      // The CID of a statement, never the statement: what an epr before the crossing wrote.
      return [`env:requires=${mintByteCid(Buffer.from('[]'))}`];
    case 'mismatch': {
      const doc = requirementDocument(0);
      return [
        `env:requires=${mintByteCid(Buffer.from('a different statement'))}`,
        `env:requires-doc=${doc.toString('base64url')}`,
      ];
    }
    default: {
      const doc = requirementDocument(statement === 'meets' ? 0 : 9999);
      return [`env:requires=${mintByteCid(doc)}`, `env:requires-doc=${doc.toString('base64url')}`];
    }
  }
}

/** The C baseline marker a producer writes after checking its build's C (row 29.2). */
const C_BASELINE_MARKER = 'env:isa-c=x86-64';

function appendRecord(h: Household, bin: string, statement: Statement): void {
  const output = mintByteCid(Buffer.from(`${h.head} ${bin}`));
  const scope = recordCid(cbor('repo-scope'));
  const event = {
    action: 'produce',
    provider: AGENT_ID,
    receiver: h.didA,
    resource: new Link(output),
    quantity: { Count: { value: 4096, unit: 'bytes' } },
    process: null,
    inScopeOf: new Link(scope),
    fulfills: [],
    satisfies: [],
    occurredAt: '2026-11-08T10:00:00Z',
    classifiedAs: [
      'custody-blob',
      output,
      `manifest:${recordCid(Buffer.from([0xa0]))}`,
      'copy:self',
      'measure:artifact-produce@1',
      'reason:produce',
      'basis-key:git-tree',
      'env:project=elohim-storage',
      `env:key=${KEY}`,
      `env:output=${output}`,
      'env:bytes=4096',
      `env:agentId=${AGENT_ID}`,
      `via:${h.didA}`,
      `env:commit=${h.head}`,
      `env:release=${RELEASE}`,
      `env:release-bin=${bin}`,
      ...statementSlots(statement),
      ...(h.marker ? [C_BASELINE_MARKER] : []),
    ],
  };
  const cid = recordCid(cbor(event));
  const plain = {
    ...event,
    resource: [...base32Decode(output.slice(1))],
    inScopeOf: [...base32Decode(scope.slice(1))],
    quantity: { Count: { value: 4096, unit: 'bytes' } },
  };
  const dir = join(h.repoB, '.eprfs/status');
  mkdirSync(dir, { recursive: true });
  appendFileSync(
    join(dir, 'flows.jsonl'),
    `${JSON.stringify({ cid, record: { kind: 'event', ...plain } })}\n`
  );
}

function incoming(h: Household): string[] {
  const dir = join(h.base, 'home-b/bins/incoming');
  return existsSync(dir) ? readdirSync(dir) : [];
}

// ---- Background -----------------------------------------------------------------------------

Given("Matthew's devices A and B are admitted to each other", function () {
  const base = mkdtempSync(join(process.env.A2O_TMPDIR ?? tmpdir(), 'honest-draw-'));
  const h: Household = {
    base,
    repoA: join(base, 'repo-a'),
    repoB: join(base, 'repo-b'),
    keyA: join(base, 'keys/a/ed25519.seed'),
    keyB: join(base, 'keys/b/ed25519.seed'),
    didA: '',
    head: '',
    parent: '',
    statement: 'meets',
    marker: true,
  };
  households.set(this, h);
  mkdirSync(dirname(h.keyA), { recursive: true });
  mkdirSync(dirname(h.keyB), { recursive: true });
  gitRepo(h, h.repoA);
  gitRepo(h, h.repoB);

  // A is the chain root: it witnesses Matthew, B enrolls, A authorizes, B binds.
  epr(
    h,
    'a',
    [
      'actor',
      'witness',
      '--subject',
      `human:${HANDLE}`,
      '--as',
      'agent:orchestrator@fixture',
      '--session',
      's1',
      '--basis',
      'operator of this stewarded device; present in this session',
      '--json',
      '--root',
      h.repoA,
    ],
    h.repoA
  );
  const request = json(
    epr(
      h,
      'b',
      ['actor', 'device', 'enroll', '--handle', HANDLE, '--json', '--root', h.repoA],
      h.repoA
    ).stdout
  );
  const authorization = json(
    epr(
      h,
      'a',
      ['actor', 'device', 'authorize', JSON.stringify(request), '--json', '--root', h.repoA],
      h.repoA
    ).stdout
  );
  epr(
    h,
    'b',
    ['actor', 'device', 'bind', JSON.stringify(authorization), '--json', '--root', h.repoA],
    h.repoA
  );
  h.didA = String(authorization.authorizedBy);
  assert.ok(h.didA.startsWith('did:key:'), `A's device identity: ${h.didA}`);

  // Each device publishes the endpoint its siblings reach it at (what admits its crossed records).
  for (const who of ['a', 'b'] as const) {
    const nodeId = epr(h, who, ['peer', 'id'], h.repoA).stdout.trim();
    epr(h, who, ['actor', 'device', 'endpoint', '--node-id', nodeId, '--root', h.repoA], h.repoA);
  }
  // The roster travels by git: B's checkout carries it.
  const rel = `.eprfs/status/participants/${HANDLE}.jsonl`;
  mkdirSync(join(h.repoB, '.eprfs/status/participants'), { recursive: true });
  copyFileSync(join(h.repoA, rel), join(h.repoB, rel));
});

Given('device B follows epr and draws from a sibling first', function () {
  const h = household(this);
  git(h, h.repoB, ['commit', '-q', '--allow-empty', '-m', 'fixture head']);
  h.head = git(h, h.repoB, ['rev-parse', 'HEAD']);
  h.parent = git(h, h.repoB, ['rev-parse', 'HEAD~1']);
  epr(h, 'b', ['device', 'follow', 'bin:epr', '--mode', 'apply'], h.repoB);
});

Given(
  'device A has recorded a release of epr for the commit device B has checked out',
  function () {
    // The records are appended by the scenario's When, once the Givens have said what the
    // release's statement is: a flow log is append-only, so the offer is written once.
    assert.ok(household(this).head.length > 0, 'B has a checked-out commit');
  }
);

// ---- scenario Givens ------------------------------------------------------------------------

Given("device B's integration branch contains that commit", function () {
  const h = household(this);
  git(h, h.repoB, ['update-ref', 'refs/remotes/origin/dev', h.head]);
});

Given("the release's statement crossed with its record and matches its CID", function () {
  household(this).statement = 'meets';
});

Given("the release's record carries the C baseline marker {string}", function (value: string) {
  assert.equal(`env:isa-c=${value}`, C_BASELINE_MARKER);
  household(this).marker = true;
});

Given('device A recorded the release without the C baseline marker', function () {
  household(this).marker = false;
});

Given('device B meets every requirement in the statement', function () {
  // The statement names this host's arch, linux and glibc 2.0 or later: this host's own
  // observation of itself meets it, so there is nothing further to arrange.
  assert.equal(household(this).statement, 'meets');
});

Given("the release's statement says it needs a newer C library than device B has", function () {
  household(this).statement = 'newer-libc';
});

Given(
  "device A recorded the release with an epr from before statements crossed, so the record names its statement's CID but does not carry the statement",
  function () {
    household(this).statement = 'predates';
  }
);

Given("the release's statement crossed with its record but does not match its CID", function () {
  household(this).statement = 'mismatch';
});

Given('device B last fetched its integration branch before that commit landed on it', function () {
  const h = household(this);
  git(h, h.repoB, ['update-ref', 'refs/remotes/origin/dev', h.parent]);
});

// ---- When -----------------------------------------------------------------------------------

When('device B updates its epr', function () {
  const h = household(this);
  for (const bin of ['epr', 'bin-launch']) appendRecord(h, bin, h.statement);
  const env = { ...deviceEnv(h, 'b'), EPR_FOLLOW_SIBLING: '1' };
  const r = spawn(
    binary,
    ['bin', '--registry', registry, '--tool', 'epr', 'follow', '--repo', h.repoB],
    env,
    h.repoB
  );
  h.stderr = `${r.stderr ?? ''}`;
  h.stdout = r.stdout ?? '';
  h.status = r.status;
  assert.notEqual(r.status, null, `the follow did not finish\n${h.stderr}`);
});

// ---- Then -----------------------------------------------------------------------------------

function said(h: Household): string {
  assert.ok(h.stderr !== undefined, 'the follow has not run');
  return h.stderr;
}

/**
 * The follow's own path after a requirement gap: the draw was attempted and declined, and the
 * follow says it builds locally, without refusing.
 */
function ownPathFollows(h: Household): void {
  const text = said(h);
  const at = text.indexOf('follow: nothing drawn from a sibling');
  assert.ok(at >= 0, `expected a gap line in:\n${text}`);
  assert.equal(h.status, 0, `the follow does not refuse:\n${text}`);
  assert.match(
    text.slice(at),
    /building locally|trying a local build/,
    `the follow goes on to its own build after the gap:\n${text}`
  );
}

Then('device B lands nothing', function () {
  const h = household(this);
  const text = said(h);
  assert.ok(!text.includes('drawing epr from a sibling first'), `no draw was begun:\n${text}`);
  const result = JSON.parse((h.stdout ?? '').trim().split('\n').pop() ?? '{}');
  assert.equal(result.followed, false, `nothing is landed:\n${h.stdout}`);
});

Then("device B fetches device A's release", function () {
  const text = said(household(this));
  assert.ok(
    !/unpublished|unmeasured-requirement|unverifiable-requirement|requirement-unmet|no-offer/.test(
      text
    ),
    `a check stopped the draw before the fetch:\n${text}`
  );
  assert.ok(
    text.includes('fetch gap: daemon-down'),
    `the draw reaches the fetch (B's peer daemon is not serving):\n${text}`
  );
});

Then('device B fetches nothing', function () {
  const h = household(this);
  const text = said(h);
  assert.ok(!text.includes('fetch gap'), `nothing may be fetched:\n${text}`);
  assert.deepEqual(incoming(h), [], 'nothing is staged in the bin store incoming directory');
});

Then(
  'device B says the release cannot run here, naming the unmet C library requirement',
  function () {
    const text = said(household(this));
    assert.match(text, /requirement-unmet: [^\n]*cannot run here/, text);
    assert.match(text, /requirement-unmet: [^\n]*abi\.libc unmet/, text);
  }
);

Then('device B goes on to its own build', function () {
  ownPathFollows(household(this));
});

Then(
  'device B says the offer predates the crossing statement and that A should record the release again with a newer epr',
  function () {
    const text = said(household(this));
    assert.match(text, /unmeasured-requirement: /, text);
    assert.match(text, /predates requires-doc/, text);
    assert.match(text, /sibling re-puts the release/, text);
  }
);

Then(
  "device B says the processor the release's C needs is unmeasured, naming the missing marker",
  function () {
    const text = said(household(this));
    assert.match(text, /unmeasured-requirement: [^\n]*records no env:isa-c/, text);
    assert.match(text, /instruction set its C was compiled at is unmeasured/, text);
  }
);

Then(
  'device B says A should rebuild for the declared platform, or B should build for itself',
  function () {
    const text = said(household(this));
    assert.ok(
      text.includes(
        'cure: the producer rebuilds under the declared target, or build for this device'
      ),
      text
    );
    assert.ok(!/unmeasured-requirement: [^\n]*env:isa-c[^\n]*re-put/.test(text), text);
  }
);

Then('device B says the statement cannot be verified', function () {
  const text = said(household(this));
  assert.match(text, /unverifiable-requirement: /, text);
  assert.match(text, /not the env:requires/, text);
});

Then('device B names each branch it checked, with the commit each one points to here', function () {
  const h = household(this);
  const text = said(h);
  assert.match(text, /unpublished: /, text);
  assert.ok(text.includes('refs/heads/dev absent'), text);
  assert.ok(text.includes(`refs/remotes/origin/dev at ${h.parent.slice(0, 12)}`), text);
});

Then('device B says to fetch the remote branch with {string}', function (cure: string) {
  assert.ok(said(household(this)).includes(cure), said(household(this)));
});

After(function () {
  const h = households.get(this);
  if (h) rmSync(h.base, { recursive: true, force: true });
});
