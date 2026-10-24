/**
 * Gate evidence by input closure — the gate asks "has THIS closure been built?" before it
 * pays the recipe, and records its own verdict after. Report-only in this slice.
 *
 * Run:
 *   node --test genesis/orchestrator/gate-evidence.test.mjs
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, writeFileSync, chmodSync, rmSync, readFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import {
  attestationStep,
  closureKey,
  describeEvidence,
  dirtyPaths,
  findEvidence,
  gateEvidence,
  recordEvidence,
  resolveBin,
  stepFingerprints,
} from './gate-evidence.mjs';

const project = {
  name: 'elohim-storage',
  dir: 'elohim/elohim-storage',
  pipeline: 'elohim-edge',
  steps: ['cargo-build-storage'],
  manifestPath: 'elohim/holochain/build-manifest.json',
  run: { kind: 'just', recipe: 'gate', cargo: { workspace: 'elohim', profile: 'dev', rustflags: '--cfg getrandom_backend="custom"' } },
};

/** A spawn double keyed by the first argv tokens; records every call. */
function fakeSpawn(table) {
  const calls = [];
  const spawn = (bin, args, opts) => {
    calls.push({ bin, args, opts });
    for (const [match, reply] of table) {
      if (match(bin, args)) return typeof reply === 'function' ? reply(bin, args) : reply;
    }
    return { status: 127, stdout: '', stderr: `no reply for ${bin} ${args.join(' ')}` };
  };
  return { spawn, calls };
}

const fingerprintReply = (fp) => (_bin, args) => ({
  status: 0,
  stdout: JSON.stringify({ manifest: args[2], commit: args[6], fingerprints: [{ pipeline: 'elohim-edge', step: args[4], fingerprint: fp, input_count: 3 }] }),
  stderr: '',
});
const isFingerprint = (bin, args) => bin.endsWith('brit') && args[0] === 'build' && args[1] === 'fingerprint';
const isRevParse = (bin, args) => bin === 'git' && args[0] === 'rev-parse';
const isStatus = (bin, args) => bin === 'git' && args[0] === 'status';
const isFind = (bin, args) => bin.endsWith('brit-build-ref') && args[2] === 'build' && args[3] === 'find';
const isPut = (bin, args) => bin.endsWith('brit-build-ref') && args[2] === 'build' && args[3] === 'put';

test('the attestation step is the gate project, not a manifest step', () => {
  assert.equal(attestationStep(project), 'gate:elohim-storage');
});

test('the closure key is a function of fingerprints, manifest blob, recipe and env — and of nothing else', () => {
  const base = { project, fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-a' }], manifestBlob: 'blob1', cargoEnv: { RUSTUP_TOOLCHAIN: '1.96.1', CARGO_BUILD_JOBS: '1' } };
  const k1 = closureKey(base);
  assert.match(k1, /^[0-9a-f]{64}$/);
  // Same closure on another commit / branch / checkout: same key (nothing about them is in the material).
  assert.equal(closureKey({ ...base }), k1);
  // Env order does not matter; env VALUE does.
  assert.equal(closureKey({ ...base, cargoEnv: { CARGO_BUILD_JOBS: '1', RUSTUP_TOOLCHAIN: '1.96.1' } }), k1);
  assert.notEqual(closureKey({ ...base, cargoEnv: { RUSTUP_TOOLCHAIN: '1.99.0', CARGO_BUILD_JOBS: '1' } }), k1);
  assert.notEqual(closureKey({ ...base, fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-b' }] }), k1);
  assert.notEqual(closureKey({ ...base, manifestBlob: 'blob2' }), k1);
  assert.notEqual(closureKey({ ...base, project: { ...project, run: { ...project.run, recipe: 'gate-fast' } } }), k1);
});

test('step fingerprints come from brit per declared step; gate-only projects are reported, not guessed', () => {
  const { spawn, calls } = fakeSpawn([[isFingerprint, fingerprintReply('bafy-a')]]);
  const fp = stepFingerprints(project, 'abc123', { britBin: '/x/brit', root: '/repo', spawn });
  assert.deepEqual(fp, { fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-a' }] });
  assert.equal(calls.length, 1);
  assert.deepEqual(calls[0].args.slice(0, 2), ['build', 'fingerprint']);
  assert.equal(calls[0].args[2], '/repo/elohim/holochain/build-manifest.json');
  assert.deepEqual(calls[0].args.slice(3), ['--step', 'cargo-build-storage', '--commit', 'abc123']);

  const gateOnly = { ...project, name: 'elohim-error', steps: [], inputs: { sources: ['elohim/elohim-error/**'] } };
  assert.deepEqual(stepFingerprints(gateOnly, 'abc123', { britBin: '/x/brit', root: '/repo', spawn }), { error: 'gate-only inputs, no step fingerprint' });

  const failing = fakeSpawn([[isFingerprint, { status: 1, stdout: '', stderr: 'could not resolve commit' }]]);
  assert.match(stepFingerprints(project, 'nope', { britBin: '/x/brit', root: '/repo', spawn: failing.spawn }).error, /fingerprint of cargo-build-storage failed: could not resolve commit/);
});

test('describeEvidence prefers the newest green, names signer, age and commit, and calls a miss a miss', () => {
  const now = Date.parse('2026-10-14T12:00:00Z');
  assert.equal(describeEvidence([], now), 'miss');
  const rows = [
    { commit: 'aaaaaaaaa1', attestation: { success: false, agentId: 'did:key:zRED', builtAt: '2026-10-14T11:00:00Z' } },
    { commit: 'bbbbbbbbb2', attestation: { success: true, agentId: 'did:key:zGREEN1', builtAt: '2026-10-13T12:00:00Z' } },
    { commit: 'ccccccccc3', attestation: { success: true, agentId: 'did:key:zGREEN2', builtAt: '2026-10-14T09:00:00Z' } },
  ];
  assert.equal(describeEvidence(rows, now), 'hit green (signer did:key:zGRE, 3h ago, commit ccccccccc, 3 attestations)');
  assert.equal(describeEvidence([rows[0]], now), 'hit red (signer did:key:zRED, 60m ago, commit aaaaaaaaa)');
});

test('gateEvidence resolves the closure, asks brit-build-ref once with the key, and reports the hit', () => {
  const dir = mkdtempSync(join(tmpdir(), 'gate-evidence-'));
  try {
    for (const name of ['brit', 'brit-build-ref']) {
      writeFileSync(join(dir, name), '#!/bin/sh\nexit 0\n');
      chmodSync(join(dir, name), 0o755);
    }
    const env = { BRIT_BIN: join(dir, 'brit'), BRIT_BUILD_REF_BIN: join(dir, 'brit-build-ref'), PATH: '' };
    const now = Date.parse('2026-10-14T12:00:00Z');
    const { spawn, calls } = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      [isRevParse, { status: 0, stdout: 'blob1\n', stderr: '' }],
      [isStatus, { status: 0, stdout: '', stderr: '' }],
      [isFind, (_bin, args) => {
        const key = args[args.indexOf('--inputs-hash') + 1];
        return { status: 0, stdout: JSON.stringify([{ commit: 'ffffffff1', attestation: { success: true, agentId: 'did:key:zSIGNER', builtAt: '2026-10-14T11:30:00Z', inputsHash: key } }]), stderr: '' };
      }],
    ]);
    const cargoEnv = { RUSTUP_TOOLCHAIN: '1.96.1' };
    const result = gateEvidence(project, { env, root: '/repo', commit: 'abc123', cargoEnv, spawn, now });
    const expectedKey = closureKey({ project, fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-a' }], manifestBlob: 'blob1', cargoEnv });
    assert.equal(result.key, expectedKey);
    assert.equal(result.line, 'hit green (signer did:key:zSIG, 30m ago, commit ffffffff1)');
    assert.equal(result.dirty, false);
    const status = calls.find(c => isStatus(c.bin, c.args));
    assert.deepEqual(status.args, ['status', '--porcelain', '--untracked-files=normal', '--', 'elohim/elohim-storage', 'elohim/holochain/build-manifest.json']);
    const find = calls.filter(c => isFind(c.bin, c.args));
    assert.equal(find.length, 1, 'one lookup per project');
    assert.deepEqual(find[0].args, ['--repo', '/repo', 'build', 'find', '--step', 'gate:elohim-storage', '--inputs-hash', expectedKey]);
    const rev = calls.find(c => isRevParse(c.bin, c.args));
    assert.deepEqual(rev.args, ['rev-parse', 'abc123:elohim/holochain/build-manifest.json']);

    // Nothing recorded yet: a miss is a miss and the key is still returned for the put after the run.
    const missing = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      [isRevParse, { status: 0, stdout: 'blob1\n', stderr: '' }],
      [isStatus, { status: 0, stdout: '', stderr: '' }],
      [isFind, { status: 0, stdout: '[]', stderr: '' }],
    ]);
    const miss = gateEvidence(project, { env, root: '/repo', commit: 'abc123', cargoEnv, spawn: missing.spawn, now });
    assert.equal(miss.line, 'miss');
    assert.equal(miss.key, expectedKey);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a dirty working tree under the project is named on the line and blocks the record', () => {
  const dir = mkdtempSync(join(tmpdir(), 'gate-evidence-'));
  try {
    for (const name of ['brit', 'brit-build-ref']) {
      writeFileSync(join(dir, name), '#!/bin/sh\nexit 0\n');
      chmodSync(join(dir, name), 0o755);
    }
    const env = { BRIT_BIN: join(dir, 'brit'), BRIT_BUILD_REF_BIN: join(dir, 'brit-build-ref'), PATH: '' };
    const { spawn, calls } = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      [isRevParse, { status: 0, stdout: 'blob1\n', stderr: '' }],
      [isStatus, { status: 0, stdout: ' M elohim/elohim-storage/src/lib.rs\n?? elohim/elohim-storage/src/new.rs\n', stderr: '' }],
      [isFind, { status: 0, stdout: '[]', stderr: '' }],
      [isPut, { status: 0, stdout: 'bafy…', stderr: '' }],
    ]);
    const result = gateEvidence(project, { env, root: '/repo', commit: 'abc123', cargoEnv: {}, spawn, now: Date.now() });
    assert.equal(result.dirty, true);
    assert.match(result.line, /^miss; working tree dirty under elohim\/elohim-storage \(2 paths\): the evidence names the commit, not these bytes, and nothing will be recorded$/);
    const put = recordEvidence(project, result.key, { success: true, seconds: 10, commit: 'abc123', buildRefBin: env.BRIT_BUILD_REF_BIN, root: '/repo', dirty: result.dirty, spawn });
    assert.equal(put.status, 1);
    assert.match(put.stderr, /working tree dirty/);
    assert.equal(calls.filter(c => isPut(c.bin, c.args)).length, 0, 'no attestation is written for bytes that are not the commit\'s');
    assert.equal(dirtyPaths(project, { root: '/repo', spawn: () => ({ status: 128, stdout: '', stderr: 'fatal' }) }), null);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('a pushed commit that is not checked out can be inspected but never recorded', () => {
  const dir = mkdtempSync(join(tmpdir(), 'gate-evidence-'));
  try {
    for (const name of ['brit', 'brit-build-ref']) {
      writeFileSync(join(dir, name), '#!/bin/sh\nexit 0\n');
      chmodSync(join(dir, name), 0o755);
    }
    const env = {
      BRIT_BIN: join(dir, 'brit'),
      BRIT_BUILD_REF_BIN: join(dir, 'brit-build-ref'),
      GATE_EVIDENCE_RECORD: '0',
      PATH: '',
    };
    const { spawn } = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      [isRevParse, { status: 0, stdout: 'blob1\n', stderr: '' }],
      [isStatus, { status: 0, stdout: '', stderr: '' }],
      [isFind, { status: 0, stdout: '[]', stderr: '' }],
      [isPut, { status: 0, stdout: 'must-not-land', stderr: '' }],
    ]);

    const result = gateEvidence(project, {
      env, root: '/repo', commit: 'pushed-not-head', cargoEnv: {}, spawn,
    });
    assert.equal(result.dirty, true);
    assert.match(result.line, /pushed commit is not checked out, nothing will be recorded/);
    const put = recordEvidence(project, result.key, {
      success: true,
      seconds: 1,
      commit: 'pushed-not-head',
      buildRefBin: env.BRIT_BUILD_REF_BIN,
      root: '/repo',
      dirty: result.dirty,
      spawn,
    });
    assert.notEqual(put.status, 0);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test('pre-push disables evidence recording when the pushed SHA differs from HEAD', () => {
  const hook = readFileSync(new URL('../../.husky/pre-push.bash', import.meta.url), 'utf8');
  assert.match(hook, /if \[ -n "\$GATE_COMMIT" \] && \[ "\$GATE_COMMIT" != "\$CHECKED_OUT_HEAD" \]; then/);
  assert.match(hook, /export GATE_EVIDENCE_RECORD=0/);
  assert.match(hook, /gate evidence will not be recorded: pushed SHA/);
});

test('gateEvidence is fail-open: no brit, an attested project, or a refused read all read as unavailable', () => {
  const env = { PATH: '', BRIT_BIN: '', BRIT_BUILD_REF_BIN: '' };
  assert.match(gateEvidence(project, { env, root: '/repo', commit: 'abc' }).line, /^unavailable \(brit not on/);
  assert.equal(gateEvidence({ ...project, run: { kind: 'attested' } }, { env, root: '/repo', commit: 'abc' }).line, 'unavailable (attested project, no local recipe)');
  assert.equal(resolveBin('brit', ['BRIT_BIN'], { BRIT_BIN: '/definitely/not/here', PATH: '' }), null);
});

test('recordEvidence puts the verdict against the closure with the manifest file and a stored verdict, red included', () => {
  const { spawn, calls } = fakeSpawn([[isPut, { status: 0, stdout: 'bafyrei…\n', stderr: '' }]]);
  const r = recordEvidence(project, 'k'.repeat(64), { success: false, seconds: 1875.4, commit: 'abc123', buildRefBin: '/x/brit-build-ref', root: '/repo', jobs: 1, dirty: false, spawn });
  assert.equal(r.status, 0);
  const args = calls[0].args;
  assert.deepEqual(args.slice(0, 6), ['--repo', '/repo', 'build', 'put', '--step', 'gate:elohim-storage']);
  assert.equal(args[args.indexOf('--manifest-path') + 1], '/repo/elohim/holochain/build-manifest.json');
  assert.deepEqual(JSON.parse(args[args.indexOf('--output-json') + 1]), { exit: 1, seconds: 1875, jobs: 1 });
  assert.equal(args[args.indexOf('--inputs-hash') + 1], 'k'.repeat(64));
  assert.equal(args[args.indexOf('--success') + 1], 'false');
  assert.equal(args[args.indexOf('--duration-ms') + 1], '1875400');
  assert.equal(args[args.indexOf('--commit') + 1], 'abc123');
  assert.equal(findEvidence(project, 'k', { buildRefBin: '/x/brit-build-ref', root: '/repo', spawn: () => ({ status: 0, stdout: 'nope', stderr: '' }) }).error, 'build find: not JSON');
});
