// The hand-over record lists verdicts for the head's closure, however old the attesting commit
// (native-throughput item 3): a reuse hit records no note at the head it served.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { closureVerdict, inventory, summaryLine } from './handover-record.mjs';

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

const HEAD = 'a'.repeat(40);
const OLD = 'b'.repeat(40);

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
  const got = closureVerdict(project, HEAD, { root: REPO, env: {}, selfId: 'me', evidenceOf });
  assert.equal(askedFor.commit, HEAD, 'the key is computed for the head');
  assert.equal(askedFor.cargoEnv.RUSTUP_TOOLCHAIN, '1.100.0', 'with the gate\'s own cargo env');
  assert.equal(got.closure, 'k-head');
  assert.equal(got.row.commit, 'c-new', 'newest of this device\'s rows, red included; a sibling\'s is not this device\'s');
  assert.deepEqual(closureVerdict(project, HEAD, { root: REPO, env: {}, selfId: 'me', evidenceOf: () => ({ line: 'unavailable (x)' }) }),
    { error: 'unavailable (x)' });
  assert.equal(closureVerdict(project, HEAD, { root: REPO, env: {}, selfId: 'nobody', evidenceOf }).row, null);
});
