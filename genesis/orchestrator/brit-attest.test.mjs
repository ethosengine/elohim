// Device-local Jenkins verdicts and produced closure notes, written through brit's verified read:
// a writer reads before it writes, keeps a note with the same tree and verdict, names what it
// replaces, treats a note brit calls `unverifiable:` as absent, defers what it could not read,
// and counts brit's `REFUSED concurrent update` / `refusing to write` as refusals, never failures.
// The brit-build-ref double (STUB) prints the contract's exact stdout/stderr/exit per mode.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync, spawnSync, spawn } from 'node:child_process';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, realpathSync, rmSync, openSync, closeSync, symlinkSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';

function scratch(prefix) {
  const parent = realpathSync(tmpdir());
  const root = realpathSync(resolve(import.meta.dirname, '../..'));
  const rel = relative(root, parent);
  if (!rel || (rel !== '..' && !rel.startsWith(`..${sep}`))) {
    throw new Error('REFUSED: set TMPDIR outside the tree');
  }
  const dir = mkdtempSync(join(parent, prefix));
  scratchDirs.push(dir);
  return dir;
}
const scratchDirs = [];
process.on('exit', () => { for (const dir of scratchDirs) rmSync(dir, { recursive: true, force: true }); });
import { join, relative, resolve, sep } from 'node:path';
import {
  buildNotesRef,
  cidOfDigest,
  encodeRefSegment,
  expectedRefusal,
  main,
  planAttestations,
  rawCid,
  readBuildNote,
  unverifiableNotes,
} from './scripts/brit-attest.mjs';
import { attestationRuns, readBuildAttestations } from './delivery-series.mjs';

const CLI = join(import.meta.dirname, 'scripts/brit-attest.mjs');
const HELPER = join(import.meta.dirname, 'scripts/brit-helper.sh');
const MANIFEST = Buffer.from('{"pipeline":"elohim"}\n');
const graph = {
  commitSha: 'c'.repeat(40), buildNumber: 2179,
  executionOrder: ['elohim-edge', 'elohim', 'elohim-genesis', 'elohim-sophia'],
  results: {
    'elohim-edge': { result: 'SUCCESS', duration: 1_800_000, buildNumber: 1760, url: 'https://jenkins/edge/1760/' },
    elohim: { result: 'FAILURE', duration: 7_200_000.4, buildNumber: 1725, url: 'https://jenkins/app/1725/' },
    'elohim-genesis': { result: 'DISPATCHED', duration: 0 },
    'elohim-sophia': { result: 'UNSTABLE', duration: 60_000 },
  },
};
const ctx = { commit: graph.commitSha, tree: 'a'.repeat(40),
  manifestBytes: p => p === 'elohim-sophia' ? null : MANIFEST,
  machine: { node: 'device', os: 'linux', arch: 'x64' } };
const flag = (args, key) => args[args.indexOf(key) + 1];
const K1 = 'sha256:' + '1'.repeat(64), K2 = 'sha256:' + '2'.repeat(64), D1 = 'sha256:' + '3'.repeat(64);

function fixtureGitEnv(home, inherited = process.env) {
  return {
    ...Object.fromEntries(Object.entries(inherited).filter(([key]) => !key.startsWith('GIT_'))),
    HOME: home,
    GIT_CONFIG_GLOBAL: join(home, '.gitconfig-fixture-disabled'),
    GIT_CONFIG_NOSYSTEM: '1',
  };
}

function fixtureGit(cwd, args, inherited = process.env) {
  return execFileSync('git', args, { cwd, env: fixtureGitEnv(cwd, inherited) }).toString().trim();
}

/** A git repo with one commit and one pipeline manifest, as a CI workspace would hold. */
function workspace(inherited = process.env) {
  const dir = scratch('brit-helper-');
  const git = (...args) => fixtureGit(dir, args, inherited);
  git('init', '-q', '.');
  mkdirSync(join(dir, 'app'));
  writeFileSync(join(dir, 'app', 'build-manifest.json'), MANIFEST);
  mkdirSync(join(dir, 'edge'));
  writeFileSync(join(dir, 'edge', 'build-manifest.json'), '{"pipeline":"elohim-edge"}\n');
  git('add', '.');
  git('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '-m', 'c');
  const sha = git('rev-parse', 'HEAD');
  const graphPath = join(dir, 'actual-build-graph.json');
  writeFileSync(graphPath, JSON.stringify({ ...graph, commitSha: sha }));
  return { dir, sha, graphPath, git };
}


function invoke(w, bin, extra = {}, args = []) {
  return spawnSync('node', [CLI, '--json', w.graphPath, ...args], { encoding: 'utf8',
    env: { ...fixtureGitEnv(w.dir), REPO_ROOT: w.dir, BRIT_BUILD_REF_BIN: bin, ...extra } });
}
function stub(w) {
  const path = join(w.dir, 'brit-stub');
  writeFileSync(path, `#!/bin/sh
printf '%s\\n' "$*" >> '${w.dir}/calls'
if [ "$4" = get ]; then
  if [ -f "${w.dir}/present.$6" ]; then cat "${w.dir}/present.$6";
  elif [ -f '${w.dir}/present' ]; then cat '${w.dir}/present';
  else echo 'no build ref found for step=stub commit=stub' >&2; fi
else
  if [ -f '${w.dir}/fail' ]; then echo boom >&2; exit 101; fi
  echo bafystub
fi
`);
  chmodSync(path, 0o755);
  return path;
}

test('one ci: verdict per attestable pipeline; dispatched and manifest-less skipped', () => {
  const { planned, skipped } = planAttestations(graph, ctx);
  assert.deepEqual(planned.map(p => p.step), ['ci:elohim-edge', 'ci:elohim']);
  for (const p of planned) {
    assert.equal(p.kind, 'tree');
    assert.equal(flag(p.args, '--inputs-hash'), ctx.tree);
    const r = graph.results[p.pipeline];
    assert.deepEqual(JSON.parse(flag(p.args, '--output-json')), { pipeline: p.pipeline, commit: ctx.commit,
      result: r.result, buildNumber: r.buildNumber, url: r.url, durationMs: Math.round(r.duration), orchestratorBuild: 2179 });
    const hw = JSON.parse(flag(p.args, '--hardware'));
    assert.equal(hw.runner, 'jenkins');
    assert.equal(hw.performer, 'service:jenkins');
    assert.equal(hw.provenance, r.url);
    assert.equal(hw.inputsHashKind, 'tree');
    assert.deepEqual(hw.writer, ctx.machine);
    assert.doesNotMatch(p.args.join(' '), /performer_key|agent:deploy-service-/);
  }
  assert.equal(flag(planned[1].args, '--success'), 'false');
  assert.deepEqual(skipped.map(s => s.pipeline), ['elohim-genesis', 'elohim-sophia']);
});

test('archived Jenkins buildNumber strings become numeric provenance', () => {
  const { planned } = planAttestations({ ...graph, buildNumber: '2179' }, ctx);
  assert.equal(JSON.parse(flag(planned[0].args, '--hardware')).orchestratorBuild, 2179);
  assert.equal(JSON.parse(flag(planned[0].args, '--output-json')).orchestratorBuild, 2179);
});

test('closure fields fan out per produced step; a hit writes no step note', () => {
  const g = structuredClone(graph);
  const r = g.results['elohim-edge'];
  Object.assign(r, { closure: { 'build-storage': K1, 'build-doorway': K2 },
    outputDigest: { 'build-storage': D1 }, closureHit: { 'build-storage': false, 'build-doorway': true } });
  const { planned, skipped } = planAttestations(g, ctx);
  const step = planned.find(p => p.step === 'ci:elohim-edge:build-storage');
  assert.equal(step.kind, 'closure-v3');
  assert.equal(flag(step.args, '--inputs-hash'), K1);
  assert.equal(flag(step.args, '--output'), cidOfDigest(D1));
  assert.equal(JSON.parse(flag(step.args, '--hardware')).artifact, `harbor.ethosengine.com/ethosengine/elohim-storage@${D1}`);
  assert.ok(!planned.some(p => p.step.endsWith(':build-doorway')));
  assert.ok(skipped.some(s => s.step === 'build-doorway' && s.why === 'closure hit: nothing produced'));
  const output = JSON.parse(flag(planned[0].args, '--output-json'));
  for (const key of ['closure', 'outputDigest', 'closureHit']) assert.deepEqual(output[key], r[key]);
  r.closureHit['build-doorway'] = false;
  assert.ok(planAttestations(g, ctx).skipped.some(s => s.why === 'no digest archived'));
});

test('cidOfDigest wraps sha256 without re-hashing and refuses malformed digests', () => {
  assert.equal(cidOfDigest('sha256:' + createHash('sha256').update('').digest('hex')), rawCid(Buffer.alloc(0)));
  assert.equal(rawCid(Buffer.alloc(0)), 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku');
  for (const digest of ['bad', 'sha256:' + 'A'.repeat(64), 'sha256:aa', null]) assert.throws(() => cidOfDigest(digest));
});

test('same inputsHash and verdict is present; a different inputsHash or verdict is rewritten and named', () => {
  const w = workspace();
  const bin = stub(w);
  const tree = w.git('rev-parse', `${w.sha}^{tree}`);
  writeFileSync(join(w.dir, 'present.ci:elohim-edge'), JSON.stringify({ inputsHash: tree, success: true }));
  writeFileSync(join(w.dir, 'present.ci:elohim'), JSON.stringify({ inputsHash: tree, success: false }));
  const same = invoke(w, bin);
  assert.equal(same.status, 0, same.stderr);
  assert.equal(JSON.parse(same.stdout).present.length, 2);
  assert.doesNotMatch(readFileSync(join(w.dir, 'calls'), 'utf8'), /build put/);
  assert.match(same.stderr, /2 present$/m);
  // The same tree with a flipped verdict is a new event: replaced, and the put is named first.
  writeFileSync(join(w.dir, 'present.ci:elohim'), JSON.stringify({ inputsHash: tree, success: true }));
  const flipped = invoke(w, bin);
  const flippedReport = JSON.parse(flipped.stdout);
  assert.deepEqual(flippedReport.present.map(p => p.step), ['ci:elohim-edge']);
  assert.deepEqual(flippedReport.written.map(p => [p.step, p.rewritten]), [['ci:elohim', true]]);
  assert.match(flipped.stderr, /ci:elohim replacing refs\/notes\/brit\/build\/ci%3Aelohim on [0-9a-f]{12} \(it attests success for tree/);
  writeFileSync(join(w.dir, 'present.ci:elohim-edge'), JSON.stringify({ inputsHash: 'changed', success: true }));
  writeFileSync(join(w.dir, 'present.ci:elohim'), JSON.stringify({ inputsHash: 'changed', success: false }));
  const changed = invoke(w, bin);
  assert.equal(JSON.parse(changed.stdout).written.filter(w => w.rewritten).length, 2);
  assert.match(changed.stderr, /rewritten/);
});

test('--json emits one report and exits 0 for absent binary, absent commit, bad graph, failed put', () => {
  const w = workspace();
  const bin = stub(w);
  const absent = invoke(w, join(w.dir, 'absent'));
  assert.equal(absent.status, 0);
  assert.deepEqual(JSON.parse(absent.stdout).deferred, [{ why: 'brit-build-ref absent' }]);
  const noBin = invoke(w, '');
  assert.deepEqual(JSON.parse(noBin.stdout).deferred, [{ why: 'brit-build-ref absent' }]);
  writeFileSync(join(w.dir, 'fail'), '');
  const failed = invoke(w, bin);
  assert.equal(failed.status, 0);
  assert.equal(JSON.parse(failed.stdout).written.length, 0);
  assert.equal(JSON.parse(failed.stdout).deferred.length, 2, 'failed puts must be retried');
  assert.match(failed.stderr, /WARN:.*boom/);
  writeFileSync(w.graphPath, JSON.stringify({ ...graph, commitSha: 'f'.repeat(40) }));
  const missing = invoke(w, bin);
  assert.equal(missing.status, 0);
  assert.equal(JSON.parse(missing.stdout).deferred[0].why, `commit ${'f'.repeat(40)} not in this clone`);
  writeFileSync(w.graphPath, '{');
  const invalid = invoke(w, bin);
  assert.equal(invalid.status, 0);
  assert.equal(JSON.parse(invalid.stdout).deferred.length, 1);
});

test('orchestrator archives its graph and calls no brit helper', () => {
  const jf = readFileSync(join(import.meta.dirname, 'Jenkinsfile'), 'utf8');
  const stage = jf.slice(jf.indexOf("stage('Post Actual Build Graph')"), jf.indexOf("stage('Reconcile Build Graph')"));
  assert.match(stage, /archiveArtifacts\(artifacts: env.ACTUAL_BUILD_GRAPH_FILE/);
  assert.match(stage, /env.ACTUAL_BUILD_GRAPH_POSTED = 'true'/);
  assert.doesNotMatch(stage, /brit-helper.sh/);
});

test('closure helpers sit above pipeline and enrich exactly both result sites without shell bodies', () => {
  const jf = readFileSync(join(import.meta.dirname, 'Jenkinsfile'), 'utf8');
  assert.equal(jf.split('results[name] = withClosureFields(name, result)').length - 1, 2);
  const start = jf.indexOf('def hydrateClosureFields('), end = jf.indexOf('def printDecisionMatrix(');
  assert.ok(start > 0 && end < jf.indexOf('\npipeline {'));
  assert.ok(jf.indexOf('def withClosureFields(') < end);
  assert.doesNotMatch(jf.slice(start, end), /\bsh |"""/);
  assert.match(jf.slice(start, end), /\.edge\.buildEnv/);
  assert.equal(JSON.parse(readFileSync(join(import.meta.dirname, 'build-artifacts.json'))).edge.buildEnv, 'build.env');
});

test('brit-helper has no attest arm', () => {
  const r = spawnSync('sh', [HELPER, 'attest', '/unused'], { encoding: 'utf8' });
  assert.equal(r.status, 64);
  assert.match(r.stderr, /\{verify\|plan\|build-ref\}/);
  assert.doesNotMatch(readFileSync(HELPER, 'utf8'), /attest\)|brit-helper.sh attest/);
});

test('real brit notes stay on the device and a second run preserves builtAt',
  { skip: !process.env.BRIT_BUILD_REF_TEST_BIN }, t => {
    const w = workspace();
    const first = invoke(w, process.env.BRIT_BUILD_REF_TEST_BIN);
    assert.equal(first.status, 0, first.stderr);
    assert.equal(JSON.parse(first.stdout).written.length, 2, first.stderr);
    const note = () => JSON.parse(w.git('notes', '--ref', 'refs/notes/brit/build/ci%3Aelohim', 'show', w.sha));
    const before = note();
    assert.equal(before.stepName, 'ci:elohim');
    assert.equal(before.hardwareProfile.performer, 'service:jenkins');
    assert.equal(before.hardwareProfile.inputsHashKind, 'tree');
    assert.match(before.agentId, /^[0-9a-f]{64}$/);
    const second = invoke(w, process.env.BRIT_BUILD_REF_TEST_BIN);
    assert.equal(JSON.parse(second.stdout).present.length, 2, second.stderr);
    assert.equal(note().builtAt, before.builtAt);
    const runs = attestationRuns(readBuildAttestations(w.dir));
    assert.equal(runs.length, 1);
    assert.equal(runs[0].number, 2179);
    assert.deepEqual(runs[0].outcome, { 'elohim-edge': 'SUCCESS', elohim: 'FAILURE' });
    const args = planAttestations(graph, { ...ctx, commit: w.sha }).planned[0].args;
    const started = performance.now();
    execFileSync(process.env.BRIT_BUILD_REF_TEST_BIN, ['--repo', w.dir, ...args], { env: fixtureGitEnv(w.dir) });
    t.diagnostic(`real build put: ${(performance.now() - started).toFixed(3)} ms (one fixture call, including launcher)`);
  });


test('malformed provenance and result records defer before any note access; never use HEAD', () => {
  const w = workspace();
  const bin = stub(w);
  for (const corruption of [{ commitSha: undefined }, { commitSha: 'HEAD' }, { commitSha: 'abc123' },
    { buildNumber: null }, { buildNumber: 'no' }, { results: null }, { results: [] },
    { results: { elohim: null } }, { results: { elohim: {} } },
    { results: { elohim: { result: 'SUCCESS', duration: 'bad' } } },
    { results: { elohim: { result: 'SUCCESS', closure: [] } } }]) {
    writeFileSync(w.graphPath, JSON.stringify({ ...graph, commitSha: w.sha, ...corruption }));
    const r = invoke(w, bin, { GIT_COMMIT_HASH: w.sha });
    assert.equal(r.status, 0, r.stderr);
    const report = JSON.parse(r.stdout);
    assert.equal(report.written.length, 0);
    assert.match(report.deferred[0].why, /invalid/);
    assert.equal(existsSync(join(w.dir, 'calls')), false, 'no get or put for a malformed wave');
  }
});

test('the shared common-dir lock defers both direct and linked-worktree writers', () => {
  const w = workspace();
  const linked = scratch('brit-linked-');
  w.git('worktree', 'add', '--detach', linked, w.sha);
  const bin = stub(w);
  mkdirSync(join(w.dir, '.git', 'brit'));
  const fd = openSync(join(w.dir, '.git', 'brit', 'attest.lock'), 'a');
  try {
    assert.equal(spawnSync('flock', ['-n', '3'], { stdio: ['ignore', 'pipe', 'pipe', fd] }).status, 0);
    for (const repo of [w.dir, linked]) {
      const r = invoke(w, bin, { REPO_ROOT: repo });
      assert.equal(r.status, 0, r.stderr);
      assert.equal(JSON.parse(r.stdout).deferred[0].why, 'deferred: attest lock busy');
    }
    assert.equal(existsSync(join(w.dir, 'calls')), false);
  } finally { closeSync(fd); }
  assert.equal(JSON.parse(invoke(w, bin).stdout).written.length, 2, 'release permits retry');
});

test('simultaneous writers serialize get/compare/put and retry preserves existing notes', async () => {
  const w = workspace();
  const bin = join(w.dir, 'slow-brit');
  const tree = w.git('rev-parse', `${w.sha}^{tree}`);
  // The miss line on stdout (brit's older stream) still reads as absent.
  writeFileSync(bin, `#!/bin/sh
step=; success=; prev=
for a in "$@"; do
  if [ "$prev" = --step ]; then step=$a; fi
  if [ "$prev" = --success ]; then success=$a; fi
  prev=$a
done
if [ "$4" = get ]; then
  if [ -f "${w.dir}/stored.$step" ]; then cat "${w.dir}/stored.$step"; else echo 'no build ref found for step=stub commit=stub'; fi
else
  sleep 0.2
  printf '{"inputsHash":"%s","success":%s}\\n' '${tree}' "$success" > "${w.dir}/stored.$step"
  echo write >> '${w.dir}/writes'
  echo bafystub
fi
`);
  chmodSync(bin, 0o755);
  const launch = () => new Promise((resolve, reject) => {
    const child = spawn('node', [CLI, '--json', w.graphPath], {
      env: { ...fixtureGitEnv(w.dir), REPO_ROOT: w.dir, BRIT_BUILD_REF_BIN: bin },
    });
    let output = '';
    child.stdout.on('data', data => { output += data; });
    child.on('error', reject);
    child.on('close', code => { try { assert.equal(code, 0); resolve(JSON.parse(output)); } catch (e) { reject(e); } });
  });
  const reports = await Promise.all([launch(), launch()]);
  assert.equal(reports.filter(r => r.deferred.some(d => d.why === 'deferred: attest lock busy')).length, 1);
  // One writer wins: it writes each of the two notes once; the other never reaches a put.
  assert.equal(readFileSync(join(w.dir, 'writes'), 'utf8').trim().split('\n').length, 2);
  assert.equal(JSON.parse(invoke(w, bin).stdout).present.length, 2);
});

test('all Node fixture suites refuse in-tree and symlinked scratch before writes', () => {
  const outside = scratch('scratch-link-');
  const root = resolve(import.meta.dirname, '../..');
  const link = join(outside, 'in-tree');
  symlinkSync(root, link);
  for (const suite of ['brit-attest', 'brit-helper', 'delivery-series']) {
    for (const parent of [root, link]) {
      const r = spawnSync('node', ['--test', '--test-name-pattern', 'same inputsHash|fixture Git ignores|only ci:', join(import.meta.dirname, `${suite}.test.mjs`)], {
        env: { ...Object.fromEntries(Object.entries(fixtureGitEnv(outside)).filter(([key]) => key !== 'NODE_TEST_CONTEXT')), TMPDIR: parent }, encoding: 'utf8',
      });
      assert.equal(r.status, 1, `${suite}: ${r.stdout} ${r.stderr}`);
      assert.match(r.stdout + r.stderr, /REFUSED: set TMPDIR outside the tree/);
    }
  }
});

test('attestation child budget defers before a git or note call', () => {
  const w = workspace();
  const r = invoke(w, stub(w), { ATTEST_BUDGET_MS: '0.00001' });
  assert.equal(r.status, 0);
  assert.equal(JSON.parse(r.stdout).written.length, 0);
  assert.equal(existsSync(join(w.dir, 'calls')), false);
});


test('the declared orchestrator gate includes Node and Python attestation regressions', () => {
  const scripts = JSON.parse(readFileSync(join(import.meta.dirname, 'package.json'), 'utf8')).scripts;
  assert.match(scripts.test, /(?:^| )brit-attest\.test\.mjs(?: |$)/);
  const recipes = readFileSync(join(import.meta.dirname, 'justfile'), 'utf8');
  assert.match(recipes, /^gate:.*\btest-harvest\b/m);
  assert.match(recipes, /test-harvest:[\s\S]*ci_harvest_\*_test\.py/);
});

// ---------------------------------------------------------------------------------------------
// The verified-read contract at brit 7e4a83115e (and the pinned 76ba95d8a0, whose `build get` /
// `build put` streams are the same where they overlap), against a double that prints the exact
// stdout/stderr/exit per mode. The read-side helpers are the ones gate-evidence.mjs imports.

/** The brit-build-ref double. Modes come from `stub.env` beside it; every argv lands in `argv.log`. */
export const STUB = `#!/bin/sh
here=$(dirname "$0")
. "$here/stub.env"
printf '%s\\n' "$*" >> "$here/argv.log"
if [ "$1" = "--repo" ]; then shift 2; fi
cmd="$1 $2"
shift 2
step=
commit=HEAD
while [ $# -gt 0 ]; do
  case "$1" in
    --step) step=$2; shift 2;;
    --commit) commit=$2; shift 2;;
    *) shift;;
  esac
done
ref="refs/notes/brit/build/$(printf '%s' "$step" | sed 's/:/%3A/g')"
case "$cmd" in
  "signer --json")
    case "$SIGNER" in
      old) printf "error: unrecognized subcommand 'signer'\\n\\nUsage: brit-build-ref [OPTIONS] <COMMAND>\\n\\nFor more information, try '--help'.\\n" >&2; exit 2;;
      none) echo '{"agentId":null}';;
      broken) printf 'Error: cannot load the repository signer key\\n\\nCaused by:\\n    0: seed has 31 bytes, expected 32\\n' >&2; exit 1;;
      *) printf '{"agentId":"%s"}\\n' "$SIGNER";;
    esac;;
  "build get")
    case "$GET" in
      hit) cat "$here/get.json";;
      miss) echo "no build ref found for step=$step commit=$commit" >&2;;
      unverifiable) echo "no build ref found for step=$step commit=$commit" >&2; echo "unverifiable: $SHA $REASON" >&2;;
      gitfail) echo "Error: git command failed: git notes --ref $ref list: fatal: not a notes commit" >&2; exit 1;;
      junk) echo bafystub;;
    esac;;
  "build find")
    case "$FIND" in
      hit) cat "$here/find.json";;
      miss) echo '[]';;
      unverifiable) cat "$here/find.json"; echo "unverifiable: $SHA $REASON" >&2;;
      gitfail) echo "Error: git command failed: git notes --ref $ref list: fatal: not a notes commit" >&2; exit 1;;
    esac;;
  "build put")
    case "$PUT" in
      ok) echo bafystub;;
      concurrent) echo "Error: REFUSED concurrent update: ref=$ref expected=0000000000000000000000000000000000000000 observed=1111111111111111111111111111111111111111 attempts=3" >&2; exit 1;;
      refuse) echo "Error: refusing to write $ref: signature does not verify: invalid signature hex" >&2; exit 1;;
      boom) echo boom >&2; exit 101;;
    esac;;
  *) printf "error: unrecognized subcommand '%s'\\n" "$cmd" >&2; exit 2;;
esac
`;

/** A verified `build get` hit: the 10 signed fields, CIDs as byte arrays, pretty-printed. */
export function getJson(fields) {
  return JSON.stringify({
    agentId: 'd75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a',
    buildDurationMs: 1800000,
    builtAt: '2026-10-29T01:02:03.456789Z',
    hardwareProfile: '{"runner":"jenkins"}',
    inputsHash: 'tree',
    manifestCid: [1, 85, 18, 32, 185, 1, 2, 3],
    outputCid: [1, 85, 18, 32, 7, 7, 7, 7],
    signature: 'ab'.repeat(64),
    stepName: 'elohim-edge',
    success: true,
    ...fields,
  }, null, 2) + '\n';
}

/** Install the double in its own fenced directory (never inside a repository) and return its path. */
export function installStub(t) {
  const dir = scratch('brit-stub-');
  t?.after(() => rmSync(dir, { recursive: true, force: true }));
  const bin = join(dir, 'brit-build-ref');
  writeFileSync(bin, STUB);
  chmodSync(bin, 0o755);
  const set = modes => {
    const quote = v => `'${String(v).replaceAll("'", `'\\''`)}'`;
    writeFileSync(join(dir, 'stub.env'), Object.entries(modes).map(([k, v]) => `${k}=${quote(v)}`).join('\n') + '\n');
    writeFileSync(join(dir, 'argv.log'), '');
  };
  set({});
  return {
    bin,
    dir,
    set,
    argv: () => readFileSync(join(dir, 'argv.log'), 'utf8').trim().split('\n').filter(Boolean),
    getJson: text => writeFileSync(join(dir, 'get.json'), text),
    findJson: text => writeFileSync(join(dir, 'find.json'), text),
  };
}

/** Run main() in-process with --json, console captured; returns exit code, stderr lines and the report. */
async function attest(w, stub, extra = {}) {
  const lines = [];
  const out = [];
  const error = console.error;
  const log = console.log;
  console.error = (...args) => lines.push(args.join(' '));
  console.log = (...args) => out.push(args.join(' '));
  try {
    const code = await main(['--json', w.graphPath], {
      env: { ...fixtureGitEnv(w.dir), BRIT_BUILD_REF_BIN: stub.bin, REPO_ROOT: w.dir, ...extra },
    });
    return { code, lines, text: lines.join('\n'), report: JSON.parse(out.at(-1)) };
  } finally {
    console.error = error;
    console.log = log;
  }
}

const puts = stub => stub.argv().filter(a => a.includes(' build put ')).map(a => /--step (\S+)/.exec(a)[1]);

test('ref segments encode as brit does: a step name is a notes ref', () => {
  assert.equal(encodeRefSegment('gate:elohim-storage'), 'gate%3Aelohim-storage');
  assert.equal(encodeRefSegment('a b..c@d'), 'a%20b%2E%2Ec%40d');
  assert.equal(encodeRefSegment('x\x01'), 'x%01');
  assert.equal(buildNotesRef('gate:elohim-storage'), 'refs/notes/brit/build/gate%3Aelohim-storage');
  assert.equal(buildNotesRef('ci:elohim-edge:build-storage'), 'refs/notes/brit/build/ci%3Aelohim-edge%3Abuild-storage');
});

test('unverifiable: lines parse to commit and reason; refusals classify from the first matching line', () => {
  assert.deepEqual(unverifiableNotes('no build ref found for step=s commit=HEAD\nunverifiable: abc123 signature does not verify against agentId "ab"\n'),
    [{ commit: 'abc123', reason: 'signature does not verify against agentId "ab"' }]);
  assert.deepEqual(unverifiableNotes(''), []);
  assert.deepEqual(expectedRefusal('Error: REFUSED concurrent update: ref=refs/notes/brit/build/s expected=0 observed=1 attempts=3\n'),
    { kind: 'concurrent update', line: 'REFUSED concurrent update: ref=refs/notes/brit/build/s expected=0 observed=1 attempts=3' });
  assert.deepEqual(expectedRefusal('Error: refusing to write refs/notes/brit/build/s: signature does not verify: invalid signature hex'),
    { kind: 'would not verify', line: 'refusing to write refs/notes/brit/build/s: signature does not verify: invalid signature hex' });
  assert.equal(expectedRefusal('boom'), null);
  assert.equal(expectedRefusal(''), null);
});

test('readBuildNote reads the three answers of a verified get, a failed git read, and a missing binary', t => {
  const stub = installStub(t);
  const opts = { buildRefBin: stub.bin, root: stub.dir };
  stub.set({ GET: 'miss' });
  assert.deepEqual(readBuildNote('gate:x', 'HEAD', opts), { state: 'absent' });
  assert.deepEqual(stub.argv(), [`--repo ${stub.dir} build get --step gate:x --commit HEAD`]);
  stub.set({ GET: 'hit' });
  stub.getJson(getJson({ stepName: 'gate:x', inputsHash: 'closure-1' }));
  const hit = readBuildNote('gate:x', 'HEAD', opts);
  assert.equal(hit.state, 'verified');
  assert.equal(hit.node.inputsHash, 'closure-1');
  assert.deepEqual(hit.node.manifestCid.slice(0, 4), [1, 85, 18, 32], 'CIDs arrive as byte arrays, not bafy strings');
  stub.set({ GET: 'unverifiable', SHA: 'deadbeef01', REASON: 'stepName "other" does not match this ref\'s "gate:x"' });
  assert.deepEqual(readBuildNote('gate:x', 'HEAD', opts),
    { state: 'unverifiable', commit: 'deadbeef01', reason: 'stepName "other" does not match this ref\'s "gate:x"' });
  stub.set({ GET: 'gitfail' });
  const failed = readBuildNote('gate:x', 'HEAD', opts);
  assert.equal(failed.state, 'unreadable');
  assert.match(failed.reason, /^Error: git command failed: git notes --ref refs\/notes\/brit\/build\/gate%3Ax list/);
  stub.set({ GET: 'junk' });
  assert.deepEqual(readBuildNote('gate:x', 'HEAD', opts), { state: 'unreadable', reason: 'build get: not JSON' });
  const absent = readBuildNote('gate:x', 'HEAD', { ...opts, buildRefBin: join(stub.dir, 'absent') });
  assert.equal(absent.state, 'unreadable');
  assert.equal(absent.code, 'ENOENT');
});

test('a writer reads before it writes: get precedes put for every note, and a miss is written', async t => {
  const w = workspace();
  const stub = installStub(t);
  stub.set({ GET: 'miss', PUT: 'ok' });
  const { code, text, report } = await attest(w, stub);
  assert.equal(code, 0);
  const argv = stub.argv();
  assert.deepEqual(argv.map(a => a.split(' ').slice(2, 6).join(' ')), [
    'build get --step ci:elohim-edge', 'build put --step ci:elohim-edge', 'build get --step ci:elohim', 'build put --step ci:elohim',
  ]);
  assert.match(argv[0], new RegExp(`--commit ${w.sha}$`));
  assert.match(text, /written ci:elohim-edge success → bafystub/);
  assert.match(text, /written ci:elohim failure → bafystub/);
  assert.match(text, /2\/2 notes attested on [0-9a-f]{12}$/m);
  assert.doesNotMatch(text, /WARN/);
  assert.equal(report.written.length, 2);
  assert.ok(report.written.every(e => e.rewritten === undefined), 'a miss is written, not rewritten');
  assert.deepEqual([report.present, report.refused, report.deferred], [[], [], []]);
});

test('the same tree and verdict is present and preserved; a different verdict on the same tree is named and replaced', async t => {
  const w = workspace();
  const stub = installStub(t);
  const tree = w.git('rev-parse', `${w.sha}^{tree}`);
  stub.set({ GET: 'hit', PUT: 'ok' });
  stub.getJson(getJson({ inputsHash: tree, success: true }));
  const { text, report } = await attest(w, stub);
  // ci:elohim-edge (success, same tree) is present: its note, and its builtAt, stay; ci:elohim
  // (failure) replaces the success note and says so before the put.
  assert.deepEqual(puts(stub), ['ci:elohim']);
  assert.match(text, /\[brit-attest\] present ci:elohim-edge$/m);
  assert.doesNotMatch(text, /ci:elohim-edge replacing/);
  assert.match(text, /ci:elohim replacing refs\/notes\/brit\/build\/ci%3Aelohim on [0-9a-f]{12} \(it attests success for tree/);
  assert.match(text, /1\/2 notes attested on [0-9a-f]{12}, 1 present$/m);
  assert.deepEqual(report.present.map(p => p.step), ['ci:elohim-edge']);
  assert.deepEqual(report.written.map(p => [p.step, p.rewritten]), [['ci:elohim', true]]);
});

test('a note brit cannot verify is named once with its ref and reason, treated as absent, and replaced', async t => {
  const w = workspace();
  const stub = installStub(t);
  stub.set({ GET: 'unverifiable', SHA: w.sha, REASON: 'signature does not verify against agentId "abcd"', PUT: 'ok' });
  const { text, lines, report } = await attest(w, stub);
  assert.deepEqual(puts(stub), ['ci:elohim-edge', 'ci:elohim']);
  const named = lines.filter(l => /unverifiable/.test(l));
  assert.equal(named.length, 2, 'one line per pipeline note');
  assert.match(named[0], new RegExp(`WARN: refs/notes/brit/build/ci%3Aelohim-edge on ${w.sha} is unverifiable \\(signature does not verify against agentId "abcd"\\); replacing it`));
  assert.match(text, /2\/2 notes attested/);
  assert.ok(report.written.every(e => e.rewritten === true), 'replacing an unverifiable note is a rewrite');
  assert.deepEqual(report.deferred, []);
});

test('a note this device could not read is named, and its write is deferred rather than done blind', async t => {
  const w = workspace();
  const stub = installStub(t);
  stub.set({ GET: 'gitfail', PUT: 'ok' });
  const { code, text, report } = await attest(w, stub);
  assert.equal(code, 0);
  assert.deepEqual(puts(stub), [], 'nothing is written over a note that was not read');
  assert.match(text, /WARN: refs\/notes\/brit\/build\/ci%3Aelohim-edge on [0-9a-f]{12} could not be read \(Error: git command failed: git notes --ref refs\/notes\/brit\/build\/ci%3Aelohim-edge list: fatal: not a notes commit\); ci:elohim-edge not written/);
  assert.match(text, /0\/2 notes attested on [0-9a-f]{12}$/m);
  assert.equal(report.written.length, 0);
  assert.equal(report.deferred.length, 2, 'the harvest retries both notes');
});

test('REFUSED concurrent update and refusing-to-write are expected refusals: warned, counted, never a failure', async t => {
  const w = workspace();
  const stub = installStub(t);
  stub.set({ GET: 'miss', PUT: 'concurrent' });
  const concurrent = await attest(w, stub);
  assert.equal(concurrent.code, 0);
  assert.match(concurrent.text, /WARN: build put ci:elohim-edge refused \(concurrent update, expected; not a failure\): REFUSED concurrent update: ref=refs\/notes\/brit\/build\/ci%3Aelohim-edge expected=0{40} observed=1{40} attempts=3/);
  assert.doesNotMatch(concurrent.text, /failed/);
  assert.match(concurrent.text, /0\/2 notes attested on [0-9a-f]{12}, 2 refused$/m);
  assert.deepEqual(concurrent.report.refused.map(r => [r.step, r.refusal]), [['ci:elohim-edge', 'concurrent update'], ['ci:elohim', 'concurrent update']]);
  assert.equal(concurrent.report.deferred.length, 2, 'the next harvest reads the other writer\'s note');
  assert.match(concurrent.report.deferred[0].why, /^deferred: ci:elohim-edge moved by another writer/);

  stub.set({ GET: 'miss', PUT: 'refuse' });
  const refused = await attest(w, stub);
  assert.equal(refused.code, 0);
  assert.match(refused.text, /WARN: build put ci:elohim refused \(would not verify, expected; not a failure\): refusing to write refs\/notes\/brit\/build\/ci%3Aelohim: signature does not verify: invalid signature hex/);
  assert.doesNotMatch(refused.text, /failed/);
  assert.deepEqual(refused.report.refused.map(r => r.refusal), ['would not verify', 'would not verify']);
  assert.deepEqual(refused.report.deferred, [], 'identical bytes would refuse again: not a retry');

  stub.set({ GET: 'miss', PUT: 'boom' });
  const boom = await attest(w, stub);
  assert.equal(boom.code, 0);
  assert.match(boom.text, /WARN: build put ci:elohim-edge failed: boom/);
  assert.match(boom.text, /0\/2 notes attested on [0-9a-f]{12}$/m);
  assert.deepEqual(boom.report.refused, []);
  assert.equal(boom.report.deferred.length, 2);
});
