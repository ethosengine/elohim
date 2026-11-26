// The hand-over record lists verdicts for the head's closure, however old the attesting commit
// (native-throughput item 3): a reuse hit records no note at the head it served.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { channelFreshness, closureVerdict, crossingLine, crossingReport, inventory, main, summaryLine } from './handover-record.mjs';

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

const HEAD = 'a'.repeat(40);
const OLD = 'b'.repeat(40);
const TREE = 'c'.repeat(40);
const BASE = 'd'.repeat(40);
const CHANGE = `I${'e'.repeat(40)}`;
const INTENT = `b${'f'.repeat(24)}`;

/** brit-build-ref: `build list` names the steps; `build get` holds a note at HEAD for `fresh` only. */
function buildRef() {
  return (_bin, args) => {
    if (args.includes('list')) return { status: 0, stdout: 'gate:fresh\ngate:reused\ngate:never\nother\n', stderr: '' };
    const step = args[args.indexOf('--step') + 1];
    if (step === 'gate:fresh') {
      return { status: 0, stdout: JSON.stringify({ inputsHash: 'k-fresh', outputCid: [1, 2], agentId: 'me', success: true }), stderr: '' };
    }
    return { status: 0, stdout: '', stderr: '' }; // absent at HEAD
  };
}

test('a verdict attested at another commit for the same closure is listed, with its commit', () => {
  const asked = [];
  const lookup = (project, head) => {
    asked.push([project, head]);
    if (project === 'reused') {
      return { closure: 'k-reused', row: { commit: OLD, attestation: { agentId: 'me', success: true, outputCid: [3] } } };
    }
    return { closure: 'k-never', row: null };
  };
  const got = inventory(HEAD, { root: '/r', buildRefBin: '/bin/brit-build-ref', spawn: buildRef(), lookup });
  assert.equal(got.inventory, 'read');
  assert.deepEqual(asked, [['never', HEAD], ['reused', HEAD]], 'only steps with no note at the head are looked up');
  assert.deepEqual(got.verdicts.map(v => [v.project, v.verdictOf, v.attestedAt ?? null, v.closure]), [
    ['fresh', 'green', null, 'k-fresh'],
    ['reused', 'green', OLD, 'k-reused'],
  ]);
  const line = summaryLine({ head: HEAD, destinations: ['refs/heads/self/x'], ...got }, '/log');
  assert.match(line, /fresh green, reused green \(same closure, attested bbbbbbbbbbbb\)/);
});

test('without a lookup, or when it throws, only the head\'s own notes are listed', () => {
  const plain = inventory(HEAD, { root: '/r', buildRefBin: '/b', spawn: buildRef() });
  assert.deepEqual(plain.verdicts.map(v => v.project), ['fresh']);
  const broken = inventory(HEAD, { root: '/r', buildRefBin: '/b', spawn: buildRef(), lookup: () => { throw new Error('no registry'); } });
  assert.deepEqual(broken.verdicts.map(v => v.project), ['fresh']);
  assert.equal(broken.inventory, 'read');
});

test('closureVerdict keys on the head and takes this device\'s newest row for that closure only', () => {
  const project = { name: 'eprfs', run: { kind: 'root-just', cargo: { workspace: 'elohim/eprfs' } } };
  let askedFor = null;
  const evidenceOf = (p, { commit, cargoEnv }) => {
    askedFor = { name: p.name, commit, cargoEnv };
    return {
      key: 'k-head',
      rows: [
        { commit: 'c-sibling', attestation: { agentId: 'sibling', success: true, builtAt: '2026-11-18T03:00:00Z' } },
        { commit: 'c-old', attestation: { agentId: 'me', success: true, builtAt: '2026-11-17T01:00:00Z' } },
        { commit: 'c-new', attestation: { agentId: 'me', success: false, builtAt: '2026-11-17T09:00:00Z' } },
      ],
    };
  };
  const got = closureVerdict(project, HEAD, {
    root: REPO, env: {}, selfId: 'me', evidenceOf,
    siblingsOf: () => ({ candidates: [] }),
  });
  assert.equal(askedFor.commit, HEAD, 'the key is computed for the head');
  assert.equal(askedFor.cargoEnv.RUSTUP_TOOLCHAIN, '1.100.0', 'with the gate\'s own cargo env');
  assert.equal(got.closure, 'k-head');
  assert.equal(got.row.commit, 'c-new', 'newest of this device\'s rows, red included; a sibling\'s is not this device\'s');
  assert.equal(got.green, true, 'a folded green for this device satisfies the closure');
  assert.deepEqual(closureVerdict(project, HEAD, {
    root: REPO, env: {}, selfId: 'me', evidenceOf: () => ({ line: 'unavailable (x)' }),
    siblingsOf: () => { throw new Error('siblings are not read without a closure key'); },
  }),
    { error: 'unavailable (x)' });
  assert.equal(closureVerdict(project, HEAD, {
    root: REPO, env: {}, selfId: 'nobody', evidenceOf, siblingsOf: () => ({ candidates: [] }),
  }).row, null);
});

test('closureVerdict includes an admitted sibling green for the computed closure', () => {
  const project = { name: 'orchestrator', run: { kind: 'just' } };
  let asked = null;
  const got = closureVerdict(project, HEAD, {
    root: REPO, env: {}, selfId: 'me',
    evidenceOf: () => ({ key: 'k-shared', rows: [] }),
    siblingsOf: (p, closure) => {
      asked = [p.name, closure];
      return { candidates: [{ project: p.name, closure, success: true, source: 'sibling' }] };
    },
  });
  assert.deepEqual(asked, ['orchestrator', 'k-shared']);
  assert.equal(got.row, null, 'the inventory remains this device\'s own evidence');
  assert.equal(got.green, true);
});

/** `epr peer status --json`, as a healthy exchange prints it: both siblings folded seconds ago and acknowledge our sequence. */
const FRESH = {
  ownSeq: 95826,
  devices: [
    { device: 'did:key:z6MkLaptop', seq: 24455, ageS: 7, eventsFolded: 31, acked: 95826 },
    { device: 'did:key:z6MkRoot', seq: 92711, ageS: 7, eventsFolded: 3501, acked: 95825 },
  ],
};
const CURE = 'restart this device\'s peer daemon (dry run first: genesis/agentic/bin/peer-liveness), then read `epr peer status`';

function postingRun({
  verdicts = [], projects = ['green', 'miss-a', 'miss-b'], message = `subject\n\nChange-Id: ${CHANGE}\n`,
  env = {}, lookup = name => ({ closure: `${name.charCodeAt(0).toString(16)}`.repeat(64).slice(0, 64), row: null }),
  eprStatus = 0, p90 = name => ({ 'miss-a': 901, 'miss-b': 77 }[name] ?? null), commonDir = null,
  peerStatus = { status: 0, stdout: JSON.stringify(FRESH), stderr: '' },
} = {}) {
  const calls = [];
  const written = [];
  const stdout = [];
  const stderr = [];
  const spawn = (bin, args) => {
    calls.push([bin, [...args]]);
    if (bin === 'git' && args[0] === 'show') return { status: 0, stdout: message, stderr: '' };
    if (bin === 'git' && args[0] === 'rev-parse' && args.includes('--git-common-dir')) {
      return { status: 0, stdout: `${commonDir ?? TREE}\n`, stderr: '' };
    }
    if (bin === 'git' && args[0] === 'rev-parse') return { status: 0, stdout: `${TREE}\n`, stderr: '' };
    if (bin === '/stub/epr' && eprStatus !== 0) return { status: eprStatus, stdout: '', stderr: 'native write failed\n' };
    if (bin === '/stub/epr' && args[0] === 'peer') {
      if (peerStatus instanceof Error) throw peerStatus;
      return peerStatus;
    }
    if (bin === '/stub/epr' && args.includes('--change')) {
      return { status: 0, stdout: JSON.stringify({ mode: 'raise', intent: INTENT }), stderr: '' };
    }
    if (bin === '/stub/epr' && args.includes('--misses')) {
      return { status: 0, stdout: JSON.stringify({ mode: 'misses', intent: INTENT }), stderr: '' };
    }
    throw new Error(`unexpected spawn ${bin} ${args.join(' ')}`);
  };
  const code = main(['--root', '/repo', '--projects', projects.join(' ')], {
    stdin: () => `refs/heads/topic ${HEAD} refs/heads/self/other ${'0'.repeat(40)}\n`,
    env: { PATH: '/stub', ...env }, spawn, now: () => new Date('2026-11-23T12:00:00Z'),
    out: line => stdout.push(line), err: line => stderr.push(line), logOf: () => '/handover.jsonl',
    append: (_log, text) => written.push(text),
    inventoryOf: () => ({ verdicts, unreadable: [], inventory: 'read' }),
    lookupFactory: () => lookup, p90Factory: () => p90,
    classify: () => ({ class: 'low', base: BASE }),
    resolveBinary: name => `/stub/${name}`,
  });
  return { code, calls, written, stdout, stderr };
}

test('a green and two selected misses raise one intent and post exactly the misses', () => {
  const greenClosure = '1'.repeat(64);
  const missA = '2'.repeat(64);
  const missB = '3'.repeat(64);
  const r = postingRun({
    verdicts: [{ project: 'green', closure: greenClosure, verdictOf: 'green' }],
    lookup: name => name === 'green'
      ? { closure: greenClosure, row: { attestation: { success: true } }, green: true }
      : { closure: name === 'miss-a' ? missA : missB, row: null, green: false },
  });
  assert.equal(r.code, 0);
  assert.equal(r.written.length, 1, 'the inventory is durable before misses are posted');
  const epr = r.calls.filter(([bin, args]) => bin === '/stub/epr' && args[0] === 'flow').map(([, args]) => args);
  assert.equal(epr.length, 2);
  assert.deepEqual(epr[0].slice(0, 3), ['flow', 'intend', '--change']);
  assert.equal(epr[0].includes('--raise'), false);
  assert.deepEqual(epr[0].filter((arg, i) => epr[0][i - 1] === '--closure'), [
    `green=${greenClosure}`, `miss-a=${missA}`, `miss-b=${missB}`,
  ]);
  assert.deepEqual(epr[1].filter((arg, i) => epr[1][i - 1] === '--miss'), [
    `miss-a=${missA}`, `miss-b=${missB}`,
  ]);
  assert.equal(epr[1][epr[1].indexOf('--ref') + 1], 'refs/heads/self/other');
  assert.equal(epr[1][epr[1].indexOf('--commit') + 1], HEAD);
  assert.deepEqual(epr[1].slice(0, 3), ['flow', 'intend', '--misses']);
  assert.match(epr[1][epr[1].indexOf('--wait-until') + 1], /^\d+$/);
  assert.match(r.stdout[1], new RegExp(`hand-over: posted 2 misses on intent ${INTENT.slice(0, 12)}: miss-a, miss-b`));
});

test('a folded green for the computed closure suppresses a miss without a verdict at this head', () => {
  const closure = '4'.repeat(64);
  const r = postingRun({
    projects: ['orchestrator'],
    verdicts: [],
    lookup: () => ({ closure, row: { commit: OLD, attestation: { success: true } }, green: true }),
  });
  assert.equal(r.code, 0);
  assert.equal(r.calls.some(([bin]) => bin === '/stub/epr'), false, 'there is no open miss to post');
  assert.deepEqual(r.stderr, []);
});

test('a changed closure without a folded verdict is posted despite a green at this head', () => {
  const oldClosure = '5'.repeat(64);
  const changedClosure = '6'.repeat(64);
  const r = postingRun({
    projects: ['orchestrator'],
    verdicts: [{ project: 'orchestrator', closure: oldClosure, verdictOf: 'green' }],
    lookup: () => ({ closure: changedClosure, row: null, green: false }),
  });
  assert.equal(r.code, 0);
  const misses = r.calls.find(([bin, args]) => bin === '/stub/epr' && args.includes('--misses'))[1];
  assert.deepEqual(misses.filter((arg, i) => misses[i - 1] === '--miss'), [
    `orchestrator=${changedClosure}`,
  ]);
});

test('an uncomputable closure falls back to the head verdict and says so', () => {
  const closure = '7'.repeat(64);
  const r = postingRun({
    projects: ['orchestrator'],
    verdicts: [{ project: 'orchestrator', closure, verdictOf: 'green' }],
    lookup: () => ({ error: 'fingerprint unavailable' }),
  });
  assert.equal(r.code, 0);
  assert.equal(r.calls.some(([bin]) => bin === '/stub/epr'), false, 'the green head fallback suppresses the miss');
  assert.deepEqual(r.stderr, [
    '[pre-push] hand-over: closure key unavailable; used head lookup fallback: orchestrator (fingerprint unavailable; green verdict at head)',
  ]);
});

test('a hand-over from a worktree writes its intent and misses to the device channel store', () => {
  const r = postingRun({ commonDir: '/main/.git' });
  assert.equal(r.code, 0);
  const epr = r.calls.filter(([bin]) => bin === '/stub/epr').map(([, args]) => args);
  assert.equal(epr.length, 3, 'the raise, the misses and one status read');
  for (const args of epr) assert.equal(args[args.indexOf('--root') + 1], '/main');
});

test('no Change-Id records the hand-over and makes no epr call', () => {
  const r = postingRun({ message: 'subject only\n' });
  assert.equal(r.code, 0);
  assert.equal(r.written.length, 1);
  assert.equal(r.calls.some(([bin]) => bin === '/stub/epr'), false);
  assert.deepEqual(r.stderr, [
    `[pre-push] hand-over: no Change-Id on ${HEAD.slice(0, 12)}; misses not posted (git commit --amend adds one)`,
  ]);
});

test('an epr failure is one advisory line and never loses the hand-over record', () => {
  const r = postingRun({ eprStatus: 9 });
  assert.equal(r.code, 0);
  assert.equal(r.written.length, 1);
  assert.deepEqual(r.stderr, ['[pre-push] hand-over: misses not posted: native write failed']);
});

test('HANDOVER_MISSES=off records without inspecting or posting misses', () => {
  const r = postingRun({ env: { HANDOVER_MISSES: 'off' } });
  assert.equal(r.code, 0);
  assert.equal(r.written.length, 1);
  assert.equal(r.calls.some(([bin]) => bin === 'git' || bin === '/stub/epr'), false);
});

test('a selected project without a closure is named and left out', () => {
  const closure = '4'.repeat(64);
  const r = postingRun({
    projects: ['miss-a', 'unknown'],
    lookup: name => name === 'unknown' ? { error: 'key unavailable' } : { closure, row: null },
  });
  assert.equal(r.code, 0);
  const misses = r.calls.find(([bin, args]) => bin === '/stub/epr' && args.includes('--misses'))[1];
  assert.deepEqual(misses.filter((arg, i) => misses[i - 1] === '--miss'), [`miss-a=${closure}`]);
  assert.deepEqual(r.stderr, [
    '[pre-push] hand-over: closure key unavailable; used head lookup fallback: unknown (key unavailable; no verdict at head; left out of misses)',
  ]);
});

// Row 105 (2026-11-25): "posted" was a local write, and nothing said the exchange had stalled.

test('a fresh sibling that acknowledges this device adds no line, and status is read once', () => {
  const r = postingRun();
  assert.equal(r.code, 0);
  assert.equal(r.stdout.length, 2, r.stdout.join('\n'));
  assert.deepEqual(r.stderr, []);
  const reads = r.calls.filter(([bin, args]) => bin === '/stub/epr' && args[0] === 'peer');
  assert.deepEqual(reads.map(([, args]) => args), [['peer', 'status', '--root', '/repo', '--json']]);
});

test('no sibling folded within the policy freshness: one line says the misses have not crossed', () => {
  const stalled = { ownSeq: 95826, devices: [
    { device: 'did:key:z6MkLaptop', ageS: 8540, acked: 95540 },
    { device: 'did:key:z6MkRoot', ageS: 8667, acked: 95540 },
  ] };
  const r = postingRun({ peerStatus: { status: 0, stdout: JSON.stringify(stalled), stderr: '' } });
  assert.equal(r.code, 0, 'a report, never a refusal');
  assert.equal(r.stdout.length, 3);
  assert.match(r.stdout[1], /hand-over: posted 3 misses/);
  assert.equal(r.stdout[2],
    `[pre-push] hand-over: posted locally; no sibling has acknowledged this device since 8540 s ago (freshness 90 s): the misses have not crossed; ${CURE}`);
});

test('a fresh sibling acknowledging an older sequence than the one written is the same one line', () => {
  const behind = { ownSeq: 95826, devices: [
    { device: 'did:key:z6MkLaptop', ageS: 12, acked: 95700 },
    { device: 'did:key:z6MkRoot', ageS: 4000, acked: 95826 },
  ] };
  assert.equal(crossingLine(behind, { freshnessS: 90 }),
    `hand-over: posted locally; no sibling has acknowledged this device past seq 95700 (this device is at seq 95826): the misses have not crossed; ${CURE}`);
  assert.equal(crossingLine({ ...behind, devices: [{ ageS: 12, acked: 95825 }] }, { freshnessS: 90 }), null,
    'one sequence behind is the acknowledgement in flight');
  assert.equal(crossingLine({ ...behind, devices: [{ ageS: 91, acked: 95826 }] }, { freshnessS: 90 })?.includes('since 91 s ago'), true,
    'an acknowledgement is as old as the head that carried it');
});

test('an unreadable status is its own line, never silence and never a refusal', () => {
  for (const peerStatus of [
    { status: 3, stdout: '', stderr: 'epr peer: this device is on no participant roster\n' },
    { status: 0, stdout: 'not json', stderr: '' },
    Object.assign(new Error('spawnSync /stub/epr ETIMEDOUT'), { code: 'ETIMEDOUT' }),
  ]) {
    const r = postingRun({ peerStatus });
    assert.equal(r.code, 0);
    assert.equal(r.stdout.length, 3, r.stdout.join('\n'));
    assert.match(r.stdout[2], /^\[pre-push\] hand-over: posted locally; `epr peer status` could not be read within 5 s \(.+\): whether the misses crossed is not known; read `epr peer status`$/);
    assert.deepEqual(r.stderr, []);
  }
  assert.match(crossingLine({ devices: [] }), /gave no sequence or device list/);
  assert.match(crossingLine({ ownSeq: 4, devices: [] }), /no admitted sibling: nothing else reads the misses/);
});

test('the status read is bounded by HANDOVER_CROSSING_TIMEOUT_S (default 5 s) and HANDOVER_CROSSING=off skips it', () => {
  const timeouts = [];
  const spawn = (_bin, _args, options) => { timeouts.push(options.timeout); return { status: 0, stdout: JSON.stringify(FRESH), stderr: '' }; };
  assert.equal(crossingReport({ epr: 'epr', store: '/s', root: '/s', env: {}, spawn }), null);
  assert.equal(crossingReport({ epr: 'epr', store: '/s', root: '/s', env: { HANDOVER_CROSSING_TIMEOUT_S: '2' }, spawn }), null);
  assert.equal(crossingReport({ epr: 'epr', store: '/s', root: '/s', env: { HANDOVER_CROSSING: 'off' }, spawn }), null);
  assert.deepEqual(timeouts, [5000, 2000]);
});

test('freshness is the policy the daemon loaded (berth.channel.freshness_s), 90 s when unreadable', () => {
  const read = path => {
    if (path === '/device/genesis/agentic/pool-policy.json') return JSON.stringify({ berth: { channel: { freshness_s: 45 } } });
    throw new Error('ENOENT');
  };
  assert.equal(channelFreshness({ rules: { policy: '/device/genesis/agentic/pool-policy.json' } }, '/other', read), 45);
  assert.equal(channelFreshness({}, '/device', read), 45);
  assert.equal(channelFreshness({}, '/nowhere', read), 90);
  assert.equal(channelFreshness({}, REPO), 90, 'the committed policy');
});
