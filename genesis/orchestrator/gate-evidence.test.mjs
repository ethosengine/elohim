/**
 * Gate evidence by input closure — the gate asks "has THIS closure been built?" before it
 * pays the recipe, and records its own verdict after. Reuse is opt-in: local self-signed first,
 * then an admitted sibling verdict under the D5 fences.
 *
 * Run:
 *   node --test genesis/orchestrator/gate-evidence.test.mjs
 */
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, chmodSync, rmSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { nativeGateState } from './gate-source-state.mjs';
import {
  attestationStep,
  closureKey,
  describeEvidence,
  dirtyPaths,
  emitGateVerdict,
  findEvidence,
  gateEvidence,
  recordEvidence,
  reusableHit,
  resolveBin,
  selfAgentId,
  siblingGovernanceChange,
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
const isPeerVerdicts = (bin, args) => bin === 'epr' && args[0] === 'peer' && args[1] === 'verdicts';
const isMergeBase = (bin, args) => bin === 'git' && args[0] === 'merge-base';
const sources = ['elohim/elohim-storage/src/**', '.claude/data/**', 'elohim/brit'];
const isShow = (bin, args) => bin === 'git' && args[0] === 'show';
const isTree = (bin, args) => bin === 'git' && args[0] === 'ls-tree';
const closureReplies = [
  [isRevParse, { status: 0, stdout: 'blob1\n', stderr: '' }],
  [isShow, { status: 0, stdout: JSON.stringify({ steps: { 'cargo-build-storage': { inputs: { sources } } } }), stderr: '' }],
  [isTree, { status: 0, stdout: '', stderr: '' }],
];
const closureMaterial = { recipeFile: 'blob1', runner: 'blob1', dirTree: 'blob1', cargoConfig: 'blob1', gitlinks: [], reads: { scope: 'tree', oid: 'blob1' } };

test('the attestation step is the gate project, not a manifest step', () => {
  assert.equal(attestationStep(project), 'gate:elohim-storage');
});

test('the closure key includes fingerprints, manifest blob, recipe and env', () => {
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
      ...closureReplies,
      [isStatus, { status: 0, stdout: '', stderr: '' }],
      [isFind, (_bin, args) => {
        const key = args[args.indexOf('--inputs-hash') + 1];
        return { status: 0, stdout: JSON.stringify([{ commit: 'ffffffff1', attestation: { success: true, agentId: 'did:key:zSIGNER', builtAt: '2026-10-14T11:30:00Z', inputsHash: key } }]), stderr: '' };
      }],
    ]);
    const cargoEnv = { RUSTUP_TOOLCHAIN: '1.96.1' };
    const result = gateEvidence(project, { env, root: '/repo', commit: 'abc123', cargoEnv, spawn, now });
    const expectedKey = closureKey({ project, fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-a' }], manifestBlob: 'blob1', cargoEnv, ...closureMaterial });
    assert.equal(result.key, expectedKey);
    assert.equal(result.line, 'hit green (signer did:key:zSIG, 30m ago, commit ffffffff1); closure scope tree (unknown reads)');
    assert.equal(result.dirty, false);
    const status = calls.find(c => isStatus(c.bin, c.args));
    assert.deepEqual(status.args, ['status', '--porcelain', '--untracked-files=normal', '--', 'elohim/elohim-storage', 'elohim/holochain/build-manifest.json', 'elohim/elohim-storage/src/', '.claude/data/', 'elohim/brit', 'elohim/elohim-storage/justfile', '.']);
    const find = calls.filter(c => isFind(c.bin, c.args));
    assert.equal(find.length, 1, 'one lookup per project');
    assert.deepEqual(find[0].args, ['--repo', '/repo', 'build', 'find', '--step', 'gate:elohim-storage', '--inputs-hash', expectedKey]);
    const rev = calls.find(c => isRevParse(c.bin, c.args));
    assert.deepEqual(rev.args, ['rev-parse', 'abc123:elohim/holochain/build-manifest.json']);

    // Nothing recorded yet: a miss is a miss and the key is still returned for the put after the run.
    const missing = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      ...closureReplies,
      [isStatus, { status: 0, stdout: '', stderr: '' }],
      [isFind, { status: 0, stdout: '[]', stderr: '' }],
    ]);
    const miss = gateEvidence(project, { env, root: '/repo', commit: 'abc123', cargoEnv, spawn: missing.spawn, now });
    assert.equal(miss.line, 'miss; closure scope tree (unknown reads)');
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
      ...closureReplies,
      [isStatus, { status: 0, stdout: ' M elohim/elohim-storage/src/lib.rs\n?? elohim/elohim-storage/src/new.rs\n', stderr: '' }],
      [isFind, { status: 0, stdout: '[]', stderr: '' }],
      [isPut, { status: 0, stdout: 'bafy…', stderr: '' }],
    ]);
    const result = gateEvidence(project, { env, root: '/repo', commit: 'abc123', cargoEnv: {}, spawn, now: Date.now() });
    assert.equal(result.dirty, true);
    assert.match(result.line, /^miss; closure scope tree \(unknown reads\); working tree dirty under elohim\/elohim-storage \(2 paths\): the evidence names the commit, not these bytes, and nothing will be recorded$/);
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
      ...closureReplies,
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

test('a completed brit record emits one device-gated crossing verdict and failures stay advisory', () => {
  const isLeases = (bin, args) => bin === 'epr' && args[0] === 'peer' && args[1] === 'leases';
  const isNote = (bin, args) => bin === 'epr' && args[0] === 'flow' && args[1] === 'note';
  const isRevParse = (bin, args) => bin === 'git' && args.includes('rev-parse');
  const sha = 'abc0000000000000000000000000000000000000';
  const landed = fakeSpawn([
    [isLeases, { status: 0, stdout: '{"standing":"device"}' }],
    [isRevParse, { status: 0, stdout: `${sha}\n` }],
    [isNote, { status: 0, stdout: '' }],
  ]);
  assert.equal(emitGateVerdict(project, 'closure', {
    success: false, seconds: 12.6, commit: 'abc',
    put: { stdout: '{"attestationId":"bafyatt","agentId":"agent"}' },
    root: '/repo', spawn: landed.spawn,
  }), true);
  const note = landed.calls.find(call => isNote(call.bin, call.args));
  assert.ok(note);
  for (const slot of [
    'project=elohim-storage', 'closure=closure', `commit=${sha}`, 'success=false', 'seconds=13',
    'attestationId=bafyatt', 'agentId=agent',
  ]) assert.ok(note.args.includes(slot), slot);

  const skipped = fakeSpawn([[isLeases, { status: 0, stdout: '{"standing":"not_a_device"}' }]]);
  const warnings = [];
  assert.equal(emitGateVerdict(project, 'closure', {
    success: true, seconds: 1, commit: 'abc', put: {}, root: '/repo', spawn: skipped.spawn,
    warn: line => warnings.push(line),
  }), true);
  assert.equal(skipped.calls.length, 1);
  assert.deepEqual(warnings, []);

  // `HEAD` crosses as the sha it named here; a commit that does not resolve is not emitted.
  const unresolved = fakeSpawn([
    [isLeases, { status: 0, stdout: '{"standing":"device"}' }],
    [isRevParse, { status: 128, stdout: '' }],
  ]);
  assert.equal(emitGateVerdict(project, 'closure', {
    success: true, seconds: 1, commit: 'HEAD', put: {}, root: '/repo', spawn: unresolved.spawn,
    warn: line => warnings.push(line),
  }), false);
  assert.ok(!unresolved.calls.some(call => isNote(call.bin, call.args)));
  assert.match(warnings.pop(), /commit HEAD does not resolve/);

  const failed = fakeSpawn([
    [isLeases, { status: 0, stdout: '{"standing":"device"}' }],
    [isRevParse, { status: 0, stdout: `${sha}\n` }],
    [isNote, { status: 2, stdout: '', stderr: 'refused\n' }],
  ]);
  assert.equal(emitGateVerdict(project, 'closure', {
    success: true, seconds: 1, commit: 'abc', put: {}, root: '/repo', spawn: failed.spawn,
    warn: line => warnings.push(line),
  }), false);
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /not emitted \(refused\)/);
});

const seed = '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
const publicKey = 'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a';

function evidenceFixture(t) {
  const root = mkdtempSync(join(tmpdir(), 'gate-evidence-'));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const name of ['brit', 'brit-build-ref', 'rakia']) {
    writeFileSync(join(root, name), '#!/bin/sh\nexit 0\n');
    chmodSync(join(root, name), 0o755);
  }
  return {
    root,
    env: { BRIT_BIN: join(root, 'brit'), BRIT_BUILD_REF_BIN: join(root, 'brit-build-ref'), RAKIA_BIN: join(root, 'rakia'), PATH: '' },
  };
}

// Model committed Rust source separately from the checkout: no source files are
// written in this fixture, so evidence must use the requested commit's bytes.
function readEvidenceFixture(t, {
  gateProject = project,
  text = `include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/data.json"));
include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/absent.json"));
include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../fixtures/schemas/", $name));`,
  declared = sources,
} = {}) {
  const { root, env } = evidenceFixture(t);
  const files = {
    [`${project.dir}/Cargo.toml`]: '[package]',
    [`${project.dir}/src/lib.rs`]: text,
  };
  return ({ commit = 'abc', tree = 'tree1', oids = {}, status = '', unreadable = false } = {}) => {
    const { spawn, calls } = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      [(bin, args) => isRevParse(bin, args) && args[1].endsWith('^{tree}'), { status: tree ? 0 : 128, stdout: tree || '' }],
      [(bin, args) => isRevParse(bin, args) && args[1].includes(':fixtures/'), (_bin, args) => {
        const path = args[1].slice(args[1].indexOf(':') + 1);
        const oid = Object.hasOwn(oids, path) ? oids[path] : path === 'fixtures/absent.json' ? null : `oid:${path}`;
        return { status: oid ? 0 : 128, stdout: oid || '' };
      }],
      [(bin, args) => isTree(bin, args) && args.includes('--name-only'), { status: 0, stdout: Object.keys(files).join('\0') + '\0' }],
      [isShow, (_bin, args) => {
        const path = args[1].slice(args[1].indexOf(':') + 1);
        if (path === project.manifestPath) return { status: 0, stdout: JSON.stringify({ steps: { 'cargo-build-storage': { inputs: { sources: declared } } } }) };
        return !unreadable && Object.hasOwn(files, path) ? { status: 0, stdout: files[path] } : { status: 128, stdout: '' };
      }],
      ...closureReplies,
      [isStatus, { status: 0, stdout: status }],
      [isFind, { status: 0, stdout: '[]' }],
    ]);
    return { result: gateEvidence(gateProject, { root, env, commit, spawn }), calls };
  };
}

test('paths scope hashes sorted file and directory OIDs at the commit, retaining absent paths', t => {
  const inspect = readEvidenceFixture(t);
  const { result, calls } = inspect();
  assert.equal(result.line, 'miss; closure scope paths (3 reads)');
  const reads = { scope: 'paths', oids: [
    { path: 'fixtures/absent.json', oid: null },
    { path: 'fixtures/data.json', oid: 'oid:fixtures/data.json' },
    { path: 'fixtures/schemas', oid: 'oid:fixtures/schemas' },
  ] };
  assert.equal(result.key, closureKey({
    project, fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-a' }],
    manifestBlob: 'blob1', ...closureMaterial, reads,
  }));
  for (const { path } of reads.oids) {
    assert.ok(calls.some(c => isRevParse(c.bin, c.args) && c.args[1] === `abc:${path}`));
  }
  assert.ok(calls.some(c => isShow(c.bin, c.args) && c.args[1] === `abc:${project.dir}/src/lib.rs`));
  assert.notEqual(inspect({ commit: 'changed-read', tree: 'tree2', oids: { 'fixtures/data.json': 'changed' } }).result.key, result.key);
  assert.notEqual(inspect({ commit: 'new-file', tree: 'tree3', oids: { 'fixtures/absent.json': 'appeared' } }).result.key, result.key);
  assert.notEqual(inspect({ commit: 'changed-dir', tree: 'tree4', oids: { 'fixtures/schemas': 'changed-tree' } }).result.key, result.key);
  assert.equal(inspect({ commit: 'unrelated-file', tree: 'tree5' }).result.key, result.key,
    'unrelated repository changes do not widen a resolved read closure');
});

test('tree scope changes the key when any repository file changes', t => {
  const inspect = readEvidenceFixture(t, { text: 'include_str!(unknown!());' });
  const first = inspect();
  assert.equal(first.result.line, 'miss; closure scope tree (unresolved reads)');
  assert.notEqual(inspect({ commit: 'changed-file', tree: 'tree2' }).result.key, first.result.key);
  assert.equal(inspect({ commit: 'same-bytes', tree: 'tree1' }).result.key, first.result.key);
});

test('non-Rust projects and Cargo steps without covered Rust files get unknown tree scope', t => {
  for (const options of [
    { gateProject: { ...project, run: { kind: 'root-just', recipe: 'gate' } } },
    { declared: ['package.json'] },
  ]) {
    const { result, calls } = readEvidenceFixture(t, options)();
    assert.equal(result.line, 'miss; closure scope tree (unknown reads)');
    assert.ok(!calls.some(c => isShow(c.bin, c.args) && c.args[1].endsWith('.rs')));
    if (options.gateProject) assert.ok(!calls.some(c => isTree(c.bin, c.args) && c.args.includes('--name-only')));
  }
});

test('an unreadable Rust source falls back to tree scope and a missing tree refuses evidence', t => {
  const inspect = readEvidenceFixture(t);
  assert.equal(inspect({ unreadable: true }).result.line, 'miss; closure scope tree (unknown reads)');
  const { result, calls } = inspect({ tree: null });
  assert.equal(result.line, 'unavailable (read tree absent at abc)');
  assert.equal(result.key, undefined);
  assert.ok(!calls.some(c => isFind(c.bin, c.args)));
});

test('dirtyPaths checks read files, directories and absent paths, or the whole tree including hook ledgers', t => {
  const resolved = readEvidenceFixture(t)({ status: ' M fixtures/data.json\n' });
  assert.equal(resolved.result.dirty, true);
  const paths = resolved.calls.find(c => isStatus(c.bin, c.args)).args;
  for (const path of ['fixtures/absent.json', 'fixtures/data.json', 'fixtures/schemas']) assert.ok(paths.includes(path));
  assert.ok(!paths.includes('.'));
  const tree = readEvidenceFixture(t, { text: 'include_str!(unknown!());', declared: [`${project.dir}/src/**`] })({
    status: ' M .claude/data/governance-findings.jsonl\n',
  });
  assert.equal(tree.result.dirty, true);
  assert.ok(tree.calls.find(c => isStatus(c.bin, c.args)).args.includes('.'));
  assert.equal(reusableHit({ ...tree.result, rows: [greenRow] }, reuseContext).reuse, false);
});

test('read scans are cached per project and commit, and moving refs invalidate the cache', t => {
  const inspect = readEvidenceFixture(t);
  const first = inspect({ commit: 'HEAD' });
  assert.match(first.result.line, /scope paths/);
  const cached = inspect({ commit: 'HEAD' });
  assert.equal(cached.result.key, first.result.key);
  assert.ok(!cached.calls.some(c => isTree(c.bin, c.args) && c.args.includes('--name-only')));
  const moved = inspect({ commit: 'HEAD', tree: 'tree2' });
  assert.ok(moved.calls.some(c => isShow(c.bin, c.args) && c.args[1].endsWith('/src/lib.rs')));
  const otherCommit = inspect({ commit: 'other', tree: 'tree2' });
  assert.ok(otherCommit.calls.some(c => isShow(c.bin, c.args) && c.args[1].endsWith('/src/lib.rs')));
});

test('selfAgentId derives the RFC 8032 public key from the common-dir seed', t => {
  const { root } = evidenceFixture(t);
  mkdirSync(join(root, 'common/brit'), { recursive: true });
  writeFileSync(join(root, 'common/brit/agent-key'), Buffer.from(seed, 'hex'));
  for (const common of ['common', join(root, 'common')]) {
    const { spawn, calls } = fakeSpawn([[isRevParse, { status: 0, stdout: `${common}\n` }]]);
    assert.equal(selfAgentId(root, spawn), publicKey);
    assert.deepEqual(calls[0].args, ['rev-parse', '--git-common-dir']);
    assert.equal(calls[0].opts.cwd, root);
  }
});

test('selfAgentId returns null for missing or wrong-length keys and refused git reads', t => {
  const { root } = evidenceFixture(t);
  const { spawn } = fakeSpawn([[isRevParse, { status: 0, stdout: '.git\n' }]]);
  assert.equal(selfAgentId(root, spawn), null);
  mkdirSync(join(root, '.git/brit'), { recursive: true });
  for (const length of [0, 31, 33, 64]) {
    writeFileSync(join(root, '.git/brit/agent-key'), Buffer.alloc(length));
    assert.equal(selfAgentId(root, spawn), null);
  }
  assert.equal(selfAgentId(root, () => { throw new Error('unreadable'); }), null);
});

const greenRow = { commit: 'aaaaaaaaa1', attestation: { success: true, agentId: publicKey, builtAt: '2026-10-24T12:00:00Z' } };
const cleanEvidence = { rows: [greenRow], key: 'closure', dirty: false };
const reuseContext = { selfId: publicKey, enabled: true, mode: 'reuse', selectorCheck: () => ({ ok: true }) };
const unexpectedSelector = () => { throw new Error('selector must not be consulted'); };
const siblingGreen = {
  source: 'sibling', device: 'did:key:z6Mksibling0001', project: project.name, closure: 'closure',
  commit: 'siblings1', success: true, seconds: 12, at: '2026-10-24T12:00:00Z', headSeq: 7,
};

test('reusableHit refuses report mode before all other checks', () => {
  assert.deepEqual(reusableHit({}, { mode: 'report', enabled: false, selfId: null, selectorCheck: unexpectedSelector }), { reuse: false, why: 'report-only' });
});

test('reusableHit refuses a project not enabled before checking the workspace key', () => {
  assert.deepEqual(reusableHit({}, { ...reuseContext, enabled: false, selfId: null, selectorCheck: unexpectedSelector }), { reuse: false, why: 'project not enabled for reuse' });
});

test('reusableHit refuses a missing workspace key before checking rows', () => {
  assert.deepEqual(reusableHit({}, { ...reuseContext, selfId: null, selectorCheck: unexpectedSelector }), { reuse: false, why: 'no workspace key' });
});

test('reusableHit refuses the newest red even when an older attestation is green', function reuse_never_overrules_a_newer_red() {
  const newer = { commit: 'bbbbbbbbb2', attestation: { success: false, agentId: 'other', builtAt: '2026-10-24T13:00:00Z' } };
  for (const rows of [[greenRow, newer], [newer, greenRow]]) {
    assert.deepEqual(reusableHit({ rows, dirty: true }, { ...reuseContext, selectorCheck: unexpectedSelector }), { reuse: false, why: 'newest attestation is red' });
  }
});

test('reusableHit names a closure with no attestation as a miss, not as red', () => {
  assert.deepEqual(reusableHit({ rows: [], dirty: false }, { ...reuseContext, selectorCheck: unexpectedSelector }), { reuse: false, why: 'no attestation for this closure yet' });
});

test('reusableHit refuses another signer before checking dirty inputs', () => {
  const row = { ...greenRow, attestation: { ...greenRow.attestation, agentId: 'f'.repeat(64) } };
  assert.deepEqual(reusableHit({ rows: [row], dirty: true }, { ...reuseContext, selectorCheck: unexpectedSelector }), { reuse: false, why: 'signer ffffffffffff is not this workspace' });
});

test('reusableHit refuses dirty declared inputs before consulting the selector', () => {
  assert.deepEqual(reusableHit({ ...cleanEvidence, dirty: true }, { ...reuseContext, selectorCheck: unexpectedSelector }), { reuse: false, why: 'declared inputs dirty' });
});

test('reusableHit refuses unknown working tree state before consulting the selector', () => {
  for (const dirty of [null, undefined]) {
    assert.deepEqual(reusableHit({ ...cleanEvidence, dirty }, { ...reuseContext, selectorCheck: unexpectedSelector }), { reuse: false, why: 'working tree state unknown' });
  }
});

test('reusableHit preserves the selector refusal message', () => {
  assert.deepEqual(reusableHit(cleanEvidence, { ...reuseContext, selectorCheck: commit => {
    assert.equal(commit, greenRow.commit);
    return { ok: false, message: 'attested commit unreadable' };
  } }), { reuse: false, why: 'attested commit unreadable' });
});

test('reusableHit accepts only the newest green self-signed row without mutating evidence', function reuse_accepts_only_self_signed_newest_green() {
  const older = { ...greenRow, attestation: { ...greenRow.attestation, success: false, builtAt: '2026-10-24T11:00:00Z' } };
  const rows = Object.freeze([older, greenRow]);
  assert.deepEqual(reusableHit({ ...cleanEvidence, rows }, reuseContext), { reuse: true, row: greenRow });
  assert.equal(rows[0], older);
});

test('reusableHit accepts an admitted sibling green and labels its independence', function reuse_accepts_admitted_sibling_green() {
  assert.deepEqual(reusableHit(
    { rows: [], key: 'closure', project: project.name, dirty: false },
    { ...reuseContext, candidates: [siblingGreen], admitCandidate: () => true },
  ), { reuse: true, row: siblingGreen, independence: 'sibling' });
});

test('reusableHit refuses a sibling candidate the admission callback does not admit', () => {
  assert.deepEqual(reusableHit(
    { rows: [], key: 'closure', project: project.name, dirty: false },
    { ...reuseContext, candidates: [siblingGreen], admitCandidate: () => false, selectorCheck: unexpectedSelector },
  ), { reuse: false, why: 'no attestation for this closure yet' });
});

test('a sibling red forces a local run without ordering device clocks', function sibling_red_forces_local_run() {
  const red = { ...siblingGreen, success: false, at: '2020-01-01T00:00:00Z' };
  assert.deepEqual(reusableHit(
    { rows: [], key: 'closure', project: project.name, dirty: false },
    { ...reuseContext, candidates: [siblingGreen, red], admitCandidate: () => true, selectorCheck: unexpectedSelector },
  ), { reuse: false, why: 'sibling verdict is red' });
});

test('a local red and sibling green force a local run', () => {
  const localRed = { commit: 'local-red', attestation: { success: false, agentId: publicKey } };
  assert.deepEqual(reusableHit(
    { rows: [localRed], key: 'closure', project: project.name, dirty: false },
    { ...reuseContext, candidates: [siblingGreen], admitCandidate: () => true, selectorCheck: unexpectedSelector },
  ), { reuse: false, why: 'local attestation is red' });
});

test('dirty inputs force a run despite an admitted sibling green', () => {
  assert.deepEqual(reusableHit(
    { rows: [], key: 'closure', project: project.name, dirty: true },
    { ...reuseContext, candidates: [siblingGreen], admitCandidate: () => true, selectorCheck: unexpectedSelector },
  ), { reuse: false, why: 'declared inputs dirty' });
});

test('a selector-changing sibling commit forces a run', () => {
  assert.deepEqual(reusableHit(
    { rows: [], key: 'closure', project: project.name, dirty: false },
    { ...reuseContext, candidates: [siblingGreen], admitCandidate: () => true, selectorCheck: commit => {
      assert.equal(commit, siblingGreen.commit);
      return { ok: false, message: 'changed since sibling verdict' };
    } },
  ), { reuse: false, why: 'changed since sibling verdict' });
});

test('the sibling governance class is data-driven and fails closed', function sibling_governance_class_fails_closed() {
  for (const path of [
    'genesis/orchestrator/gate-evidence.mjs',
    'genesis/orchestrator/gate-anything.mjs',
    'genesis/agentic/pool-policy.json',
    'elohim/eprfs/eprfs-peer/src/fold.rs',
    'elohim/eprfs/epr-cli/src/peer.rs',
    '.claude/epr-meta/measures.yaml',
  ]) {
    const { spawn } = fakeSpawn([
      [(bin, args) => bin === 'git' && args[0] === 'merge-base', { status: 0, stdout: 'base\n' }],
      [(bin, args) => bin === 'git' && args[0] === 'diff', { status: 0, stdout: `${path}\n` }],
    ]);
    assert.deepEqual(siblingGovernanceChange({ root: '/repo', spawn }), {
      changed: true, why: `governance-class change: ${path}`, paths: [path],
    });
  }
  const absent = siblingGovernanceChange({ root: '/repo', spawn: () => ({ status: 1, stdout: '' }) });
  assert.deepEqual(absent, { changed: true, why: 'integration merge base unavailable' });
});

test('closure key v3 includes every new piece of material and is deterministic', () => {
  const base = { project, fingerprints: [], manifestBlob: 'manifest', cargoEnv: {}, ...closureMaterial, rustc: 'compiler', gitlinks: [{ path: 'elohim/brit', sha: 'abc' }], packages: [] };
  const key = closureKey(base);
  assert.equal(closureKey({ ...base }), key);
  for (const [piece, value] of Object.entries({
    recipeFile: 'recipe2', runner: 'runner2', dirTree: 'tree2', cargoConfig: 'config2',
    rustc: 'compiler2', gitlinks: [{ path: 'elohim/brit', sha: 'def' }], packages: ['some-crate'],
  })) {
    assert.notEqual(closureKey({ ...base, [piece]: value }), key, piece);
  }
  const canonical = value => Array.isArray(value) ? value.map(canonical)
    : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(k => [k, canonical(value[k])])) : value;
  const material = { v: 3, project: project.name, run: project.run, manifest: 'manifest', steps: [], env: {}, ...closureMaterial, rustc: 'compiler', gitlinks: base.gitlinks, packages: [] };
  assert.equal(key, createHash('sha256').update(JSON.stringify(canonical(material))).digest('hex'));
});

test('gateEvidence reads closure material at the commit and hashes the child compiler and sorted packages', t => {
  const { root, env } = evidenceFixture(t);
  const declared = ['vendor/**', 'elohim/brit'];
  const compiler = 'rustc 1.96.1\nbinary: rustc\n';
  const { spawn, calls } = fakeSpawn([
    [isFingerprint, fingerprintReply('bafy-a')],
    [isShow, { status: 0, stdout: JSON.stringify({ steps: { 'cargo-build-storage': { inputs: { sources: declared } } } }) }],
    [isTree, { status: 0, stdout: '160000 commit zzz\tvendor/z\n100644 blob regular\tvendor/file\n160000 commit aaa\tvendor/a\n160000 commit bbb\telohim/brit\n160000 commit no\tvendor\n160000 commit no\tvendors/other\n' }],
    [(bin) => bin === 'rustc', { status: 0, stdout: compiler }],
    ...closureReplies,
    [isStatus, { status: 0, stdout: '' }],
    [isFind, { status: 0, stdout: '[]' }],
  ]);
  const cargoEnv = { RUSTUP_TOOLCHAIN: '1.96.1' };
  const result = gateEvidence(project, { root, env: { ...env, RUSTUP_TOOLCHAIN: 'nightly', GATE_CARGO_PACKAGES: ' zeta  alpha\n' }, commit: 'gated-sha', cargoEnv, spawn });
  assert.equal(result.key, closureKey({
    project, fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-a' }], manifestBlob: 'blob1', cargoEnv,
    ...closureMaterial, rustc: createHash('sha256').update(compiler).digest('hex'),
    gitlinks: [{ path: 'elohim/brit', sha: 'bbb' }, { path: 'vendor/a', sha: 'aaa' }, { path: 'vendor/z', sha: 'zzz' }], packages: ['alpha', 'zeta'],
  }));
  assert.deepEqual(calls.find(c => c.bin === 'rustc').args, ['-vV']);
  assert.equal(calls.find(c => c.bin === 'rustc').opts.env.RUSTUP_TOOLCHAIN, '1.96.1');
  assert.deepEqual(calls.find(c => isShow(c.bin, c.args)).args, ['show', 'gated-sha:elohim/holochain/build-manifest.json']);
  assert.ok(calls.some(c => isTree(c.bin, c.args) && JSON.stringify(c.args) === JSON.stringify(['ls-tree', '-r', 'gated-sha'])));
  for (const path of ['elohim/elohim-storage/justfile', 'genesis/orchestrator/run-local-gate.sh', 'elohim/elohim-storage', '.cargo/config.toml']) {
    assert.ok(calls.some(c => isRevParse(c.bin, c.args) && c.args[1] === `gated-sha:${path}`), path);
  }
});

test('closure reads tolerate absent optional objects and a failed compiler', t => {
  const { root, env } = evidenceFixture(t);
  const { spawn } = fakeSpawn([
    [isFingerprint, fingerprintReply('bafy-a')],
    [(bin, args) => isRevParse(bin, args) && args[1].endsWith(':.cargo/config.toml'), () => { throw new Error('absent'); }],
    [isTree, { status: 128, stdout: '' }],
    [(bin) => bin === 'rustc', () => { throw new Error('no compiler'); }],
    ...closureReplies,
    [isStatus, { status: 0, stdout: '' }],
    [isFind, { status: 0, stdout: '[]' }],
  ]);
  const result = gateEvidence(project, { root, env, commit: 'abc', spawn });
  assert.equal(result.key, closureKey({ project, fingerprints: [{ step: 'cargo-build-storage', fingerprint: 'bafy-a' }], manifestBlob: 'blob1', ...closureMaterial, cargoConfig: null, rustc: null, gitlinks: null }));
});

test('required recipe, runner and dir tree objects refuse an unavailable closure', t => {
  const { root, env } = evidenceFixture(t);
  for (const [piece, absent] of [
    ['recipeFile', ref => ref.endsWith('/justfile') || ref.endsWith(':justfile')],
    ['runner', ref => ref.endsWith(':genesis/orchestrator/run-local-gate.sh')],
    ['dirTree', ref => ref.endsWith(':elohim/elohim-storage')],
  ]) {
    const { spawn, calls } = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      [(bin, args) => isRevParse(bin, args) && absent(args[1]), { status: 128, stdout: '' }],
      ...closureReplies,
    ]);
    assert.equal(gateEvidence(project, { root, env, commit: '123456789abcd', spawn }).line, `unavailable (${piece} absent at 123456789)`);
    assert.equal(calls.filter(c => isFind(c.bin, c.args)).length, 0);
  }
});

test('root recipes, missing local justfiles and the root directory use the root objects', t => {
  const { root, env } = evidenceFixture(t);
  for (const p of [project, { ...project, run: { kind: 'root-just', recipe: 'gate' } }, { ...project, dir: '.' }]) {
    const { spawn, calls } = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      [(bin, args) => isRevParse(bin, args) && args[1] === 'abc:elohim/elohim-storage/justfile', { status: 128, stdout: '' }],
      ...closureReplies,
      [isStatus, { status: 0, stdout: '' }],
      [isFind, { status: 0, stdout: '[]' }],
    ]);
    assert.equal(gateEvidence(p, { root, env, commit: 'abc', spawn }).dirty, false);
    assert.ok(calls.some(c => isRevParse(c.bin, c.args) && c.args[1] === 'abc:justfile'));
    assert.ok(calls.find(c => isStatus(c.bin, c.args)).args.includes('justfile'));
    if (p.dir === '.') assert.ok(calls.some(c => isRevParse(c.bin, c.args) && c.args[1] === 'abc^{tree}'));
    if (p.run.kind === 'root-just') assert.ok(!calls.some(c => isRevParse(c.bin, c.args) && c.args[1] === 'abc:elohim/elohim-storage/justfile'));
  }
});

test('dirtyPaths passes declared source pathspecs and the recipe, including hook ledgers', () => {
  const { spawn, calls } = fakeSpawn([
    ...closureReplies,
    [isStatus, { status: 0, stdout: ' M .claude/data/governance-findings.jsonl\n' }],
  ]);
  assert.deepEqual(dirtyPaths(project, { root: '/repo', spawn }), ['.claude/data/governance-findings.jsonl']);
  assert.deepEqual(calls.find(c => isStatus(c.bin, c.args)).args, [
    'status', '--porcelain', '--untracked-files=normal', '--', project.dir, project.manifestPath,
    'elohim/elohim-storage/src/', '.claude/data/', 'elohim/brit', 'elohim/elohim-storage/justfile', '.',
  ]);
  assert.equal(dirtyPaths(project, { root: '/repo', spawn: () => { throw new Error('unreadable'); } }), null);
});

test('an unreadable step declaration leaves the working tree state unknown', t => {
  const { root, env } = evidenceFixture(t);
  const { spawn } = fakeSpawn([
    [isFingerprint, fingerprintReply('bafy-a')],
    [isShow, { status: 0, stdout: '{}' }],
    ...closureReplies,
    [isStatus, { status: 0, stdout: '' }],
    [isFind, { status: 0, stdout: '[]' }],
  ]);
  assert.equal(gateEvidence(project, { root, env, commit: 'abc', spawn }).dirty, null);
});

test('reuseSelector requires a readable diff and a real oracle answer when files changed', async function reuse_selector_requires_the_oracle(t) {
  const { reuseSelector } = await import('./gate-runner.mjs');
  const { root, env } = evidenceFixture(t);
  writeFileSync(join(root, 'build-manifest.json'), JSON.stringify({
    pipeline: 'fixture', steps: { build: { depends: [], inputs: { sources: ['src/**'] } } },
    gate: { projects: { fixture: { dir: '.', steps: ['build'], run: { kind: 'root-just', recipe: 'gate' } } } },
  }));
  const p = { name: 'fixture' };
  const cases = [
    { diff: { status: 128, stdout: '' }, expected: { ok: false, message: 'attested commit unreadable' } },
    { diff: { status: 0, stdout: '' }, expected: { ok: true } },
    { diff: { status: 0, stdout: 'other/file\n' }, oracle: { status: 1, stdout: '' }, expected: { ok: false, message: 'rakia oracle unavailable; reuse needs it' } },
    { diff: { status: 0, stdout: 'src/file\n' }, oracle: { status: 0, stdout: JSON.stringify({ affected: [{ qualified_name: 'fixture:build', affected_by: [{ kind: 'changedFile', path: 'src/file' }] }] }) }, expected: { ok: false, message: 'changed since attested commit and selects this gate: src/file' } },
    { diff: { status: 0, stdout: 'other/file\n' }, oracle: { status: 0, stdout: '{"affected":[]}' }, expected: { ok: true } },
  ];
  for (const c of cases) {
    const { spawn, calls } = fakeSpawn([
      [(bin, args) => bin === 'git' && args[0] === 'diff', c.diff],
      [(bin) => bin.endsWith('rakia'), c.oracle],
    ]);
    assert.deepEqual(reuseSelector(p, 'new', { root, env: { ...env, GATE_ORACLE: 'path' }, spawn })('old'), c.expected);
    assert.deepEqual(calls[0].args, ['diff', '--name-only', 'old', 'new']);
    if (!c.diff.stdout) assert.equal(calls.length, 1);
  }
});

test('runProject reuses without executing or attesting and observes run-kind=reused', async t => {
  const { runProject } = await import('./gate-runner.mjs');
  const { root, env } = evidenceFixture(t);
  mkdirSync(join(root, 'genesis/agentic'), { recursive: true });
  writeFileSync(join(root, 'genesis/agentic/pool-policy.json'), JSON.stringify({ gate_reuse: { projects: [project.name] } }));
  mkdirSync(join(root, '.git/brit'), { recursive: true });
  writeFileSync(join(root, '.git/brit/agent-key'), Buffer.from(seed, 'hex'));
  const { spawn, calls } = fakeSpawn([
    [isFingerprint, fingerprintReply('bafy-a')],
    [(bin, args) => isRevParse(bin, args) && args[1] === '--git-common-dir', { status: 0, stdout: '.git\n' }],
    ...closureReplies,
    [isStatus, { status: 0, stdout: '' }],
    [isFind, { status: 0, stdout: JSON.stringify([greenRow]) }],
    [(bin, args) => bin === 'git' && args[0] === 'diff', { status: 0, stdout: '' }],
    [(bin) => bin === 'epr', { status: 0, stdout: '' }],
  ]);
  const lines = [];
  const status = runProject({ ...project, run: { kind: 'just', recipe: 'gate' } }, false, false, {
    root, env: { ...env, GATE_EVIDENCE: 'reuse', GATE_RUN_KIND: 'full-gate' }, spawn, write: line => lines.push(line),
  });
  assert.equal(status, 0);
  assert.match(lines.join(''), /\[gate\] elohim-storage reused: green .* at aaaaaaaaa \(signer this workspace\)/);
  assert.equal(calls.filter(c => c.bin === 'bash' || isPut(c.bin, c.args)).length, 0);
  const note = calls.find(c => c.bin === 'epr');
  assert.ok(note.args.includes('run-kind=reused'));
  assert.ok(note.args.includes('exit=ok'));
  assert.equal(note.args[note.args.indexOf('--value') + 1], '0');
});

test('runProject reuses a sibling green, labels it sibling, and records sibling reuse', async t => {
  const { runProject } = await import('./gate-runner.mjs');
  const { root, env } = evidenceFixture(t);
  mkdirSync(join(root, 'genesis/agentic'), { recursive: true });
  writeFileSync(join(root, 'genesis/agentic/pool-policy.json'), JSON.stringify({ gate_reuse: { projects: [project.name] } }));
  mkdirSync(join(root, '.git/brit'), { recursive: true });
  writeFileSync(join(root, '.git/brit/agent-key'), Buffer.from(seed, 'hex'));
  const { spawn, calls } = fakeSpawn([
    [isFingerprint, fingerprintReply('bafy-a')],
    [(bin, args) => isRevParse(bin, args) && args[1] === '--git-common-dir', { status: 0, stdout: '.git\n' }],
    ...closureReplies,
    [isStatus, { status: 0, stdout: '' }],
    [isFind, { status: 0, stdout: '[]' }],
    [isPeerVerdicts, (_bin, args) => ({ status: 0, stdout: JSON.stringify([{
      ...siblingGreen,
      closure: args[args.indexOf('--closure') + 1],
    }]) })],
    [isMergeBase, { status: 0, stdout: 'base\n' }],
    [(bin, args) => bin === 'git' && args[0] === 'diff', { status: 0, stdout: '' }],
    [(bin) => bin === 'epr', { status: 0, stdout: '' }],
  ]);
  const lines = [];
  assert.equal(runProject({ ...project, run: { kind: 'just', recipe: 'gate' } }, false, false, {
    root, env: { ...env, GATE_EVIDENCE: 'reuse' }, spawn, write: line => lines.push(line),
  }), 0);
  assert.match(lines.join(''), /reused: green .* \(signer sibling did:key:z6Mksibling0001, independence: sibling\)/);
  assert.equal(calls.filter(c => c.bin === 'bash' || isPut(c.bin, c.args)).length, 0);
  const cycle = calls.find(c => c.bin === 'epr' && c.args.includes('run-kind=sibling-reused'));
  assert.ok(cycle, 'sibling reuse has its own gate-cycle run kind');
});

test('a governance-class change refuses a sibling verdict and runs the gate', async function governance_change_runs_gate(t) {
  const { runProject } = await import('./gate-runner.mjs');
  const { root, env } = evidenceFixture(t);
  mkdirSync(join(root, 'genesis/agentic'), { recursive: true });
  writeFileSync(join(root, 'genesis/agentic/pool-policy.json'), JSON.stringify({ gate_reuse: { projects: [project.name] } }));
  mkdirSync(join(root, '.git/brit'), { recursive: true });
  writeFileSync(join(root, '.git/brit/agent-key'), Buffer.from(seed, 'hex'));
  const { spawn, calls } = fakeSpawn([
    [isFingerprint, fingerprintReply('bafy-a')],
    [(bin, args) => isRevParse(bin, args) && args[1] === '--git-common-dir', { status: 0, stdout: '.git\n' }],
    ...closureReplies,
    [isStatus, { status: 0, stdout: '' }],
    [isFind, { status: 0, stdout: '[]' }],
    [isPeerVerdicts, (_bin, args) => ({ status: 0, stdout: JSON.stringify([{
      ...siblingGreen, closure: args[args.indexOf('--closure') + 1],
    }]) })],
    [isMergeBase, { status: 0, stdout: 'base\n' }],
    [(bin, args) => bin === 'git' && args[0] === 'diff', { status: 0, stdout: 'genesis/orchestrator/gate-runner.mjs\n' }],
    [(bin) => bin === 'bash', { status: 0, stdout: '', stderr: '' }],
    [isPut, { status: 0, stdout: 'bafyrecorded\n', stderr: '' }],
    [(bin) => bin === 'epr', { status: 0, stdout: '' }],
  ]);
  const lines = [];
  assert.equal(runProject({ ...project, run: { kind: 'just', recipe: 'gate' } }, false, false, {
    root, env: { ...env, GATE_EVIDENCE: 'reuse' }, spawn, write: line => lines.push(line), warn: () => {},
  }), 0);
  assert.match(lines.join(''), /not reused: governance-class change: genesis\/orchestrator\/gate-runner\.mjs/);
  assert.equal(calls.filter(c => c.bin === 'bash').length, 1);
});

test('epr peer verdicts failure is a miss and the gate runs', async t => {
  const { runProject } = await import('./gate-runner.mjs');
  const { root, env } = evidenceFixture(t);
  mkdirSync(join(root, 'genesis/agentic'), { recursive: true });
  writeFileSync(join(root, 'genesis/agentic/pool-policy.json'), JSON.stringify({ gate_reuse: { projects: [project.name] } }));
  mkdirSync(join(root, '.git/brit'), { recursive: true });
  writeFileSync(join(root, '.git/brit/agent-key'), Buffer.from(seed, 'hex'));
  const { spawn, calls } = fakeSpawn([
    [isFingerprint, fingerprintReply('bafy-a')],
    [(bin, args) => isRevParse(bin, args) && args[1] === '--git-common-dir', { status: 0, stdout: '.git\n' }],
    ...closureReplies,
    [isStatus, { status: 0, stdout: '' }],
    [isFind, { status: 0, stdout: '[]' }],
    [isPeerVerdicts, { status: 2, stdout: '', stderr: 'not a device' }],
    [(bin) => bin === 'bash', { status: 0, stdout: '', stderr: '' }],
    [isPut, { status: 0, stdout: 'bafyrecorded\n', stderr: '' }],
    [(bin) => bin === 'epr', { status: 0, stdout: '' }],
  ]);
  assert.equal(runProject({ ...project, run: { kind: 'just', recipe: 'gate' } }, false, false, {
    root, env: { ...env, GATE_EVIDENCE: 'reuse' }, spawn, write: () => {}, warn: () => {},
  }), 0);
  assert.equal(calls.filter(c => c.bin === 'bash').length, 1);
});

test('runProject keeps running and recording in report mode or with an unreadable policy', async t => {
  const { runProject } = await import('./gate-runner.mjs');
  const { root, env } = evidenceFixture(t);
  mkdirSync(join(root, 'genesis/agentic'), { recursive: true });
  for (const [mode, policy] of [['report', null], ['reuse', null], ['reuse', 'not JSON'], ['reuse', 'null']]) {
    if (policy) writeFileSync(join(root, 'genesis/agentic/pool-policy.json'), policy);
    const { spawn, calls } = fakeSpawn([
      [isFingerprint, fingerprintReply('bafy-a')],
      ...closureReplies,
      [isStatus, { status: 0, stdout: '' }],
      [isFind, { status: 0, stdout: JSON.stringify([greenRow]) }],
      [isPut, { status: 0, stdout: 'recorded' }],
      [(bin) => bin === 'bash' || bin === 'epr', { status: 0, stdout: '' }],
    ]);
    const lines = [];
    assert.equal(runProject({ ...project, run: { kind: 'just', recipe: 'gate' } }, false, false, {
      root, env: { ...env, GATE_EVIDENCE: mode }, spawn, write: line => lines.push(line),
    }), 0);
    assert.equal(calls.filter(c => c.bin === 'bash').length, 1);
    assert.equal(calls.filter(c => isPut(c.bin, c.args)).length, 1);
    if (mode === 'reuse') assert.match(lines.join(''), /not reused: project not enabled for reuse/);
    else assert.doesNotMatch(lines.join(''), /not reused:|reused:/);
  }
});

test('a native evaluator older than the verb names itself, not the closure', function evaluator_miss_names_the_device(t) {
  const project = { dir: 'elohim/eprfs' };
  const fake = result => () => ({ status: 2, stdout: '', stderr: '', ...result });
  const old = nativeGateState(project, { root: '/nowhere', env: { EPR_BIN: '/old/epr' }, toolchain: '1.96.1',
    run: fake({ stderr: 'epr: invalid arguments: unknown command `bin`\nusage: epr ...' }) });
  assert.match(old.miss, /\/old\/epr predates `epr bin source-state`; set EPR_BIN/);
  assert.equal(old.key, undefined);
  const absent = nativeGateState(project, { root: '/nowhere', env: { EPR_BIN: '/no/epr' }, toolchain: '1.96.1',
    run: fake({ error: Object.assign(new Error('spawn ENOENT'), { code: 'ENOENT' }) }) });
  assert.match(absent.miss, /\/no\/epr did not run \(ENOENT\)/);
  const dirty = nativeGateState(project, { root: '/nowhere', env: { EPR_BIN: '/new/epr' }, toolchain: '1.96.1',
    run: fake({ status: 0, stdout: JSON.stringify({ clean: false, status: ' M x', closureCid: '', closure: [] }) }) });
  assert.equal(dirty.miss, 'source closure dirty');
  assert.equal(nativeGateState(project, { root: '/nowhere', env: {} }).miss, 'no toolchain declared');
});
