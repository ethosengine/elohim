/**
 * delivery-series — the push-delivers-within-budget habit's check.
 * Fixtures mirror orchestrator/dev #1891 (timed out with app ABORTED) and a
 * delivered app-only run. Run: node --test genesis/orchestrator/delivery-series.test.mjs
 */

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import {
  classifyRun,
  normalizeStages,
  phaseCases,
  STAGE_BOUNDS,
  stageRun,
  summarize,
  summarizeStages,
  stagesVerdict,
  verdict,
} from './delivery-series.mjs';

const run1891 = classifyRun(
  { number: 1891, result: 'ABORTED', durationMs: 244.5 * 60_000 },
  { pipelines: ['elohim', 'elohim-edge', 'elohim-holochain', 'elohim-genesis'] },
  {
    executionOrder: ['elohim', 'elohim-edge', 'elohim-holochain', 'elohim-genesis'],
    results: {
      'elohim-holochain': { result: 'SUCCESS' },
      'elohim-edge': { result: 'SUCCESS' },
      elohim: { result: 'ABORTED', error: 'Timeout has been exceeded' },
    },
    abortedBeforeStart: ['elohim-genesis'],
  }
);

const delivered = n =>
  classifyRun(
    { number: n, result: 'SUCCESS', durationMs: 40 * 60_000 },
    null,
    { executionOrder: ['elohim'], results: { elohim: { result: 'UNSTABLE' } } }
  );

test('a run whose planned pipeline was cut off is not delivered, and is a timeout', () => {
  assert.equal(run1891.delivered, false);
  assert.equal(run1891.timedOut, true);
  assert.equal(run1891.outcome['elohim-genesis'], 'NOT_DISPATCHED');
  assert.equal(run1891.minutes, 244.5);
});

test('UNSTABLE counts as delivered; no-op runs say nothing', () => {
  assert.equal(delivered(1).delivered, true);
  assert.equal(classifyRun({ number: 2, result: 'SUCCESS', durationMs: 30_000 }, { pipelines: [] }, null), null);
});

test('a fire-and-forget pipeline the orchestrator never awaited does not block delivery', () => {
  const r = classifyRun({ number: 3, result: 'SUCCESS', durationMs: 60_000 }, null, {
    executionOrder: ['elohim', 'elohim-storybook'],
    results: { elohim: { result: 'SUCCESS' }, 'elohim-storybook': { result: 'DISPATCHED' } },
  });
  assert.equal(r.delivered, true);
});

test('an orchestrator aborted before archiving its graph is an undelivered timeout', () => {
  const r = classifyRun({ number: 1884, result: 'ABORTED', durationMs: 251.7 * 60_000 }, { pipelines: ['elohim-edge'] }, null);
  assert.equal(r.delivered, false);
  assert.equal(r.timedOut, true);
});

test('verdict: the 09-19..09-22 series is outside bounds; a delivering series is within', () => {
  const starved = summarize(Array.from({ length: 8 }, () => run1891), { window: 10 });
  assert.equal(starved.delivered, 0);
  assert.equal(verdict(starved), 1);

  const healthy = summarize([delivered(10), delivered(9), delivered(8), run1891, delivered(7)], { window: 5 });
  assert.equal(healthy.rate, 0.8);
  assert.equal(healthy.p90DeliveredMin, 40);
  assert.equal(verdict(healthy), 0);
});

test('too little evidence is never read as health', () => {
  assert.equal(verdict(summarize([delivered(1)], { window: 10 })), 2);
  assert.equal(verdict(summarize([null, null], { window: 10 })), 2);
});

// ── --stages: per-stage wall clock and cost per delivered bundle (Lane D1) ────────────────
// Fixtures mirror elohim/dev #1725 (the 7200 s readiness wait inside "Publish and Verify App
// Delivery": 122.5 min, FAILED, 0 delivered) and a phased build shaped the way Lane A records
// concern-scoped phases in the junit report (classname elohim-app.deploy.<env>).
const describe1725 = {
  stages: [
    { name: 'Build App', status: 'SUCCESS', durationMillis: 54_000 },
    { name: 'Unit Test', status: 'SUCCESS', durationMillis: 120_000 },
    { name: 'Publish and Verify App Delivery', status: 'FAILED', durationMillis: 122.5 * 60_000 },
    { name: 'Build Image', status: 'NOT_EXECUTED', durationMillis: 0 },
    { name: 'Running', status: 'IN_PROGRESS', durationMillis: 5 },
  ],
};

test('normalizeStages speaks the pipeline-trajectory vocabulary', () => {
  const s = normalizeStages(describe1725);
  assert.deepEqual(s.map(x => x.result), ['SUCCESS', 'SUCCESS', 'FAILURE', 'NOT_BUILT', null]);
  assert.equal(s[2].durationMs, 122.5 * 60_000);
  assert.deepEqual(normalizeStages(null), []);
});

const run1725 = stageRun({ number: 1725, result: 'FAILURE', durationMs: 129.5 * 60_000 }, normalizeStages(describe1725), []);
const deliveredRun = n =>
  stageRun(
    { number: n, result: 'SUCCESS', durationMs: 30 * 60_000 },
    normalizeStages({ stages: [{ name: 'Publish and Verify App Delivery', status: 'SUCCESS', durationMillis: 8 * 60_000 }] }),
    []
  );

test('an unphased build falls back to the stage: the readiness wait is not split out', () => {
  assert.equal(run1725.split, 'stage');
  assert.equal(run1725.publishVerifyMin, 122.5);
  assert.equal(run1725.phases.readiness, null);
  assert.equal(run1725.delivered, 0);
  assert.equal(deliveredRun(1).delivered, 1);
});

const report = {
  suites: [
    {
      cases: [
        { className: 'elohim-app.deploy.dev', name: 'readiness doorway-alpha.elohim.host', status: 'PASSED', duration: 4 },
        { className: 'elohim-app.deploy.dev', name: 'publish.seed doorway-alpha.elohim.host elohim-app (browser)', status: 'PASSED', duration: 120 },
        { className: 'elohim-app.deploy.dev', name: 'publish.author elohim-app (browser)', status: 'PASSED', duration: 60 },
        { className: 'elohim-app.deploy.dev', name: 'verify.shell doorway-alpha.elohim.host', status: 'PASSED', duration: 30 },
        { className: 'elohim-app.deploy.dev', name: 'converge.declare elohim-app', status: 'FAILED', duration: 600 },
        { className: 'elohim-app.unit', name: 'publish.seed is not a deploy phase here', status: 'PASSED', duration: 999 },
      ],
    },
  ],
};

test('phaseCases reads only the deploy classname and the concern-scoped phase names', () => {
  const p = phaseCases(report);
  assert.equal(p.length, 5);
  assert.deepEqual(p.map(c => c.phase), ['readiness', 'publish', 'publish', 'verify', 'converge']);
  assert.deepEqual(phaseCases(null), []);
});

test('a phased build splits readiness / publish / verify and counts the verified bundle', () => {
  const r = stageRun({ number: 1800, result: 'UNSTABLE', durationMs: 40 * 60_000 }, [], phaseCases(report));
  assert.equal(r.split, 'phases');
  assert.equal(r.phases.readiness, 4 / 60);
  assert.equal(r.phases.publish, 3);
  assert.equal(r.phases.verify, 0.5);
  assert.equal(r.publishVerifyMin, 3.5);
  assert.equal(r.delivered, 1, 'verify.shell passed → one delivered bundle, converge is measured not gating');
});

test('a stage marked FAILED only because an earlier stage failed has no wall clock', () => {
  const r = stageRun(
    { number: 1717, result: 'FAILURE', durationMs: 4.9 * 60_000 },
    normalizeStages({
      stages: [
        { name: 'Unit Test', status: 'FAILED', durationMillis: 90_000 },
        { name: 'Publish and Verify App Delivery', status: 'FAILED', durationMillis: 160 },
      ],
    }),
    []
  );
  assert.equal(r.publishVerifyMin, null);
  assert.deepEqual(r.stages.map(s => s.name), ['Unit Test']);
});

test('a readiness refusal delivers nothing', () => {
  const refused = { suites: [{ cases: [{ className: 'elohim-app.deploy.dev', name: 'readiness doorway-alpha', status: 'FAILED', duration: 3 }] }] };
  const r = stageRun({ number: 1801, result: 'UNSTABLE', durationMs: 12 * 60_000 }, [], phaseCases(refused));
  assert.equal(r.delivered, 0);
  assert.equal(r.publishVerifyMin, null);
});

test('summarizeStages prices the #1719–#1725 incident: hours spent, none delivered', () => {
  const s = summarizeStages(Array.from({ length: 7 }, () => run1725), { window: 10 });
  assert.equal(s.considered, 7);
  assert.equal(s.delivered, 0);
  assert.equal(s.pipelineHours, 15.11);
  assert.equal(s.costPerDeliveredHours, null, 'no delivered bundle: cost is unbounded, never 0');
  assert.equal(s.publishVerify.p90Min, 122.5);
  assert.equal(s.stages['Publish and Verify App Delivery'].p90Min, 122.5);
  assert.equal(s.stages['Build Image'], undefined, 'a stage that never executed has no wall clock');
  assert.equal(stagesVerdict(s), 1);
});

test('a delivering series is within the declared bounds', () => {
  const s = summarizeStages([deliveredRun(4), deliveredRun(3), deliveredRun(2), deliveredRun(1)], { window: 10 });
  assert.equal(s.delivered, 4);
  assert.equal(s.costPerDeliveredHours, 0.5);
  assert.equal(s.publishVerify.p50Min, 8);
  assert.equal(stagesVerdict(s), 0);
  assert.equal(stagesVerdict(s, { maxPublishVerifyP90Min: 5 }), 1);
});

test('too few builds is never read as health', () => {
  assert.equal(stagesVerdict(summarizeStages([deliveredRun(1)], { window: 10 })), 2);
});

test('the --stages defaults are the bounds declared in measures.yaml, not a second copy', () => {
  const text = readFileSync(new URL('../../.claude/epr-meta/measures.yaml', import.meta.url), 'utf8');
  const hard = id => {
    const block = text.split(/\n(?=  - id: )/).find(b => b.startsWith(`  - id: ${id}\n`));
    assert.ok(block, `${id} is declared`);
    return Number(block.match(/\n    hard: ([0-9.]+)/)[1]);
  };
  assert.equal(STAGE_BOUNDS.maxPublishVerifyP90Min, hard('stage-wallclock-ceiling'));
  assert.equal(STAGE_BOUNDS.maxCostHours, hard('delivery-cost-ceiling'));
});

test('a stage absent from a build is no sample: p50/p90 over present samples only, n per stage', () => {
  // A phased build carries a stage the unsplit builds never ran; a refused phased build has
  // no verify leg. Absence must shrink n, never enter the percentile as NaN.
  const phased = (n, readinessWaitMin, cases) =>
    stageRun(
      { number: n, result: 'UNSTABLE', durationMs: 40 * 60_000 },
      normalizeStages({
        stages: [
          { name: 'Build App', status: 'SUCCESS', durationMillis: 60_000 },
          { name: 'Await Doorway Readiness', status: 'SUCCESS', durationMillis: readinessWaitMin * 60_000 },
        ],
      }),
      phaseCases({ suites: [{ cases }] })
    );
  const leg = (name, status, duration) => ({ className: 'elohim-app.deploy.dev', name, status, duration });
  const ok = phased(1900, 4, [leg('readiness a', 'PASSED', 60), leg('publish.seed a', 'PASSED', 120), leg('verify.shell a', 'PASSED', 30)]);
  const refused = phased(1899, 12, [leg('readiness a', 'FAILED', 300)]);
  const unsplit = stageRun(
    { number: 1898, result: 'SUCCESS', durationMs: 30 * 60_000 },
    normalizeStages({
      stages: [
        { name: 'Build App', status: 'SUCCESS', durationMillis: 120_000 },
        { name: 'Publish and Verify App Delivery', status: 'SUCCESS', durationMillis: 8 * 60_000 },
      ],
    }),
    []
  );
  const s = summarizeStages([ok, refused, unsplit, deliveredRun(1897)], { window: 10 });

  const finiteOrNull = v => v === null || Number.isFinite(v);
  for (const [name, st] of [...Object.entries(s.stages), ...Object.entries(s.phases), ['publishVerify', s.publishVerify]]) {
    assert.ok(Number.isInteger(st.n), `${name}: n is a count`);
    assert.ok(finiteOrNull(st.p50Min) && finiteOrNull(st.p90Min), `${name}: p50/p90 finite or null, never NaN`);
    if (st.n === 0) assert.deepEqual([st.p50Min, st.p90Min], [null, null], `${name}: n=0 reads as absent, not a number`);
  }
  assert.ok(!/NaN/.test(JSON.stringify(s)), 'no NaN reaches the JSON (it would serialize as null)');

  assert.deepEqual(s.stages['Await Doorway Readiness'], { n: 2, p50Min: 4, p90Min: 12 }, 'only the two phased builds ran it');
  assert.deepEqual(s.stages['Publish and Verify App Delivery'], { n: 2, p50Min: 8, p90Min: 8 });
  assert.deepEqual(s.stages['Build App'], { n: 3, p50Min: 1, p90Min: 2 });
  assert.deepEqual(s.phases.readiness, { n: 2, p50Min: 1, p90Min: 5 });
  assert.deepEqual(s.phases.verify, { n: 1, p50Min: 0.5, p90Min: 0.5 }, 'the refused build ran no verify leg');
  assert.equal(s.phases.converge.n, 0);

  const allUnsplit = summarizeStages([unsplit, deliveredRun(1)], { window: 10 });
  assert.deepEqual(allUnsplit.phases.readiness, { n: 0, p50Min: null, p90Min: null });
});

// ── --from attestations: the series read from brit build notes, not the Jenkins API ─────────

import { execFileSync, spawnSync } from 'node:child_process';
import { mkdtempSync, realpathSync, rmSync } from 'node:fs';
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
import { attestationRuns, britSigner, readAttestationSeries, readBuildAttestations } from './delivery-series.mjs';
import { selfAgentId } from './gate-evidence.mjs';
import { chmodSync, mkdirSync, writeFileSync } from 'node:fs';

const at = (step, commit, success, minutes, endIso) => ({
  step,
  commit,
  success,
  durationMs: minutes * 60_000,
  harvestedAt: endIso,
});

test('attestations group by commit into the orchestrator series shape, newest first', () => {
  const runs = attestationRuns([
    at('elohim-edge', 'aaaa1111aaaa1111', true, 30, '2026-09-24T10:30:00Z'),
    at('elohim', 'aaaa1111aaaa1111', true, 20, '2026-09-24T10:55:00Z'),
    at('elohim-edge', 'bbbb2222bbbb2222', true, 30, '2026-09-24T12:30:00Z'),
    at('elohim', 'bbbb2222bbbb2222', false, 120, '2026-09-24T14:40:00Z'),
  ]);
  assert.equal(runs.length, 2);
  const [newest, older] = runs;
  assert.equal(newest.commit, 'bbbb2222bbbb2222');
  assert.equal(newest.number, 'bbbb2222bbbb');
  assert.equal(newest.planned, null);
  assert.deepEqual(newest.outcome, { 'elohim-edge': 'SUCCESS', elohim: 'FAILURE' });
  assert.equal(newest.delivered, 'unknown');
  assert.equal(newest.result, null);
  // Harvest timestamps are not Jenkins start/end times.
  assert.equal(newest.minutes, null);
  assert.equal(older.delivered, 'unknown');
  assert.equal(older.minutes, null);
  // A build note says nothing about a timeout: unknown, never "no timeout".
  assert.equal(newest.timedOut, null);
  const s = summarize(runs, { window: 10 });
  assert.equal(s.delivered, null);
  assert.equal(s.timedOut, null);
  assert.equal(s.lastDelivered, null);
});

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

function repoWithNotes(inherited = process.env) {
  const dir = scratch('delivery-attest-');
  const git = (...args) => fixtureGit(dir, args, inherited);
  git('init', '-q', '.');
  const commit = () => {
    git('-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '--allow-empty', '-m', 'c');
    return git('rev-parse', 'HEAD');
  };
  const note = (step, sha, body) =>
    git('-c', 'user.name=brit', '-c', 'user.email=b@b', 'notes', '--ref', `refs/notes/brit/build/${encodeURIComponent(step)}`, 'add', '-f', '-m', JSON.stringify(body), sha);
  // The payload brit-build-ref writes: the whole BuildAttestationContentNode, camelCase, with
  // each CID serialized as its bytes.
  const cid = [1, 85, 18, 32, ...Array(32).fill(0)];
  const node = (step, success, ms, builtAt) => ({
    manifestCid: cid,
    stepName: step,
    inputsHash: 'tree',
    outputCid: cid,
    agentId: '00',
    hardwareProfile: {},
    buildDurationMs: ms,
    builtAt,
    success,
    // 128 hex chars: the shape of a real ed25519 signature. The bytes are placeholders — no reader
    // in this file verifies them; the NEW stub refuses only a signature that is not hex at all.
    signature: 'ab'.repeat(64),
  });
  const c1 = commit();
  note('ci:elohim', c1, node('ci:elohim', true, 600_000, '2026-09-24T10:10:00+00:00'));
  const c2 = commit();
  note('ci:elohim', c2, node('ci:elohim', false, 1_200_000, '2026-09-24T11:20:00+00:00'));
  note('ci:elohim-edge', c2, node('ci:elohim-edge', true, 600_000, '2026-09-24T11:00:00+00:00'));
  note('gate:orchestrator', c2, node('gate:orchestrator', true, 100, '2026-09-24T11:20:00+00:00'));
  note('ci:elohim-edge:build-storage', c2, node('ci:elohim-edge:build-storage', true, 100, '2026-09-24T11:20:00+00:00'));
  return { dir, c1, c2 };
}

test('only ci:<pipeline> notes form the series', () => {
  const { dir, c1, c2 } = repoWithNotes();
  // Pinned to the raw reader: the fixture notes are unsigned, and a verifying brit on PATH would refuse them.
  const got = readBuildAttestations(dir, { buildRefBin: null }).sort((a, b) => a.harvestedAt.localeCompare(b.harvestedAt));
  assert.ok(got.every(a => a.inputsHashKind === 'tree'));
  assert.ok(got.every(a => a.verified === false && a.reader === 'raw-git'));
  assert.deepEqual(
    got.map(a => [a.step, a.commit, a.success, a.durationMs]),
    [
      ['elohim', c1, true, 600_000],
      ['elohim-edge', c2, true, 600_000],
      ['elohim', c2, false, 1_200_000],
    ]
  );
  assert.deepEqual(readBuildAttestations(scratch('no-git-'), { buildRefBin: null }), []);
});

test('--from attestations prints the same JSON shape as the Jenkins series, sourced from notes', () => {
  const { dir, c2 } = repoWithNotes();
  const cli = join(import.meta.dirname, 'delivery-series.mjs');
  // Pinned to the raw reader: a host's PATH may carry a real brit-build-ref, and this test is about shape.
  const r = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir, '--json', '--window', '2'], { env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: '/nonexistent/brit-build-ref' }, encoding: 'utf8' });
  const out = JSON.parse(r.stdout);
  assert.equal(out.source, 'attestations');
  assert.equal(out.verification.reader, 'raw-git');
  assert.equal(out.considered, 2);
  assert.equal(out.delivered, null);
  assert.equal(out.runs[0].commit, c2);
  assert.deepEqual(out.runs[0].outcome, { 'elohim-edge': 'SUCCESS', elohim: 'FAILURE' });
  for (const key of ['window', 'considered', 'delivered', 'rate', 'timedOut', 'p50DeliveredMin', 'p90DeliveredMin', 'lastDelivered', 'runs', 'bounds', 'exit']) {
    assert.ok(key in out, `shape keeps ${key}`);
  }
  // Partial notes never establish delivery or wall-clock bounds.
  assert.equal(out.partial, 'partial: commit-level view; wave wall-clock unknown');
  assert.equal(out.p90DeliveredMin, null);
  assert.equal(out.exit, 2);
  assert.equal(r.status, 2);
});

test('--stages is not in a build note: refused as not measured, never guessed', () => {
  const { dir } = repoWithNotes();
  const cli = join(import.meta.dirname, 'delivery-series.mjs');
  const r = spawnSync('node', [cli, '--stages', '--from', 'attestations', '--repo', dir], { encoding: 'utf8' });
  assert.equal(r.status, 2);
  assert.match(r.stderr, /NOT MEASURED/);
});


test('default remains Jenkins even with enough old notes; explicit notes label both formats', () => {
  const { dir } = repoWithNotes();
  const cli = join(import.meta.dirname, 'delivery-series.mjs');
  const env = { ...fixtureGitEnv(dir), JENKINS: 'http://127.0.0.1:1', BRIT_BUILD_REF_BIN: '/nonexistent/brit-build-ref' };
  for (const window of ['1', '2', '5']) {
    const r = spawnSync('node', [cli, '--repo', dir, '--json', '--window', window], { env, encoding: 'utf8' });
    assert.equal(r.status, 2);
    assert.match(r.stderr, /Jenkins unreadable/);
    assert.equal(r.stdout, '');
  }
  const text = spawnSync('node', [cli, '--repo', dir, '--from', 'attestations'], { env, encoding: 'utf8' });
  assert.equal(text.status, 2);
  assert.match(text.stdout, /partial: commit-level view; wave wall-clock unknown/);
  assert.match(text.stdout, /source: attestations, UNVERIFIED — raw git notes, no brit-build-ref on BRIT_BUILD_REF_BIN or PATH/);
  assert.doesNotMatch(text.stdout, /DELIVERED|bound/);
});

test('a successful note cannot prove a wave with an uncompleted planned pipeline delivered', () => {
  const runs = attestationRuns([{ ...at('elohim', 'a'.repeat(40), true, 60, '2026-10-29T10:00:00Z'),
    executionOrder: ['elohim', 'elohim-edge'] }]);
  assert.equal(runs[0].delivered, 'unknown');
  assert.equal(runs[0].minutes, null);
  assert.equal(runs[0].harvestedAt, '2026-10-29T10:00:00Z');
  assert.equal(summarize(runs).p90DeliveredMin, null);
  assert.equal(verdict(summarize(runs)), 2);
});

test('delivery fixture sanitizes hostile Git environment', () => {
  const decoy = scratch('delivery-decoy-');
  fixtureGit(decoy, ['init', '-q', '.']);
  const { dir, c2 } = repoWithNotes({ ...process.env, GIT_DIR: join(decoy, '.git'), GIT_COMMON_DIR: join(decoy, '.git'), GIT_WORK_TREE: decoy, GIT_CONFIG_COUNT: '1', GIT_CONFIG_KEY_0: 'core.bare', GIT_CONFIG_VALUE_0: 'true' });
  assert.equal(fixtureGit(dir, ['rev-parse', 'HEAD']), c2);
  assert.equal(fixtureGit(decoy, ['for-each-ref']), '');
});

// ── brit verified reads: a note brit refuses is absent; raw reads say they are unverified ────
// Two stub binaries, written below into a fenced scratch dir at test time (never into the tree:
// in-tree fixtures poison the manifest walkers, and a path outside the tree is gone after a Che
// restart). NEW prints the exact streams of brit-build-ref at 7e4a83115e (`signer --json`;
// verified `build get`: miss line then `unverifiable: <sha> <reason>` on stderr, exit 0; git
// failure exit 1 `Error: git command failed: …`). OLD prints the pinned 76ba95d8a0's (no `signer`,
// clap exit 2; `build get` echoes the stored JSON unverified; a body serde_json cannot parse is
// `Error: JSON error: …` exit 1). Neither runs ed25519 over DAG-CBOR: NEW refuses a signature that
// is not 128 hex chars with the real binary's `invalid signature hex` reason, reports the all-`ff`
// sentinel as `does not verify against agentId`, and accepts any other 128-hex signature. Every
// NEW-contract string here is reasoned from brit-daily-driver's tests, not executed against it.
// Only ci:<pipeline> refs are read at all: gate:* and ci:<pipeline>:<step> notes never reach brit.

const NEW_AGENT = 'ab'.repeat(32);

const STUB_COMMON = [
  "import { execFileSync } from 'node:child_process';",
  'const argv = process.argv.slice(2);',
  'const opt = (flag, fallback) => { const i = argv.indexOf(flag); return i >= 0 ? argv[i + 1] : fallback; };',
  "const repo = opt('--repo', process.cwd());",
  "const rest = argv.filter((a, i) => !(a === '--repo' || argv[i - 1] === '--repo'));",
  "const enc = s => s.replace(/:/g, '%3A').replace(/@/g, '%40').replace(/ /g, '%20');",
  "const usage = sub => { process.stderr.write(\"error: unrecognized subcommand '\" + sub + \"'\\n\\nUsage: brit-build-ref [OPTIONS] <COMMAND>\\n\\nFor more information, try '--help'.\\n\"); process.exit(2); };",
  "const git = args => execFileSync('git', args, { cwd: repo, stdio: ['ignore', 'pipe', 'pipe'] }).toString();",
].join('\n');

const NEW_STUB_SRC = `#!/usr/bin/env node
// Stub of brit-build-ref at 7e4a83115e (daily-driver): the NEW contract's exact strings.
${STUB_COMMON}
if (rest[0] === 'signer') {
  if (rest.includes('--json')) console.log('{"agentId":"${NEW_AGENT}"}');
  else console.log('${NEW_AGENT}');
  process.exit(0);
}
if (rest[0] === 'reach') usage('reach');
if (rest[0] !== 'build' || rest[1] !== 'get') usage(rest[0] ?? '');
const step = opt('--step');
const commit = opt('--commit');
const ref = 'refs/notes/brit/build/' + enc(step);
let sha;
try { sha = git(['rev-parse', commit]).trim(); } catch (e) { process.stderr.write('Error: git command failed: git rev-parse ' + commit + ': ' + e.stderr); process.exit(1); }
if (process.env.BRIT_STUB_GIT_FAIL === '1') {
  process.stderr.write('Error: git command failed: git notes --ref ' + ref + ' list: fatal: stub refused\\n');
  process.exit(1);
}
let body;
try { body = git(['notes', '--ref', ref, 'show', sha]); } catch { body = null; }
const miss = 'no build ref found for step=' + step + ' commit=' + commit;
const refuse = reason => { process.stderr.write(miss + '\\nunverifiable: ' + sha + ' ' + reason + '\\n'); process.exit(0); };
if (body === null) { process.stderr.write(miss + '\\n'); process.exit(0); }
let node;
try { node = JSON.parse(body); } catch (e) { refuse('does not decode: ' + e.message); }
const fields = ['agentId','buildDurationMs','builtAt','hardwareProfile','inputsHash','manifestCid','outputCid','signature','stepName','success'];
const extra = Object.keys(node).find(k => !fields.includes(k));
if (extra) refuse('does not decode: unknown field \`' + extra + '\`, expected one of ' + fields.map(f => '\`' + f + '\`').join(', ') + ' at line 1 column 1');
if (!/^[0-9a-f]{128}$/.test(String(node.signature))) refuse('signature does not verify: invalid signature hex');
if (node.signature === 'f'.repeat(128)) refuse('signature does not verify against agentId ' + JSON.stringify(node.agentId));
if (node.stepName !== step) refuse('stepName ' + JSON.stringify(node.stepName) + " does not match this ref's " + JSON.stringify(step));
console.log(JSON.stringify(Object.fromEntries(fields.map(f => [f, node[f]])), null, 2));
`;

const OLD_STUB_SRC = `#!/usr/bin/env node
// Stub of the PINNED brit-build-ref (76ba95d8a0): no \`signer\`; \`build get\` prints the stored JSON
// back without verifying it (old build_cmd.rs get), miss line on stderr exit 0, and a body
// serde_json cannot parse is anyhow over RefError::Json: 'Error: JSON error: …' exit 1.
${STUB_COMMON}
if (rest[0] !== 'build' || rest[1] !== 'get') usage(rest[0] ?? '');
const step = opt('--step');
const commit = opt('--commit');
let body;
try { body = git(['notes', '--ref', 'refs/notes/brit/build/' + enc(step), 'show', git(['rev-parse', commit]).trim()]); } catch { body = null; }
if (body === null) { process.stderr.write('no build ref found for step=' + step + ' commit=' + commit + '\\n'); process.exit(0); }
let node;
try { node = JSON.parse(body); } catch { process.stderr.write('Error: JSON error: expected ident at line 1 column 2\\n'); process.exit(1); }
console.log(JSON.stringify(node, null, 2));
`;

function writeStubs() {
  const dir = scratch('brit-stubs-');
  const put = (name, src) => {
    const path = join(dir, name);
    writeFileSync(path, src);
    chmodSync(path, 0o755);
    return path;
  };
  return { NEW_BRIT: put('brit-build-ref-new', NEW_STUB_SRC), OLD_BRIT: put('brit-build-ref-old', OLD_STUB_SRC) };
}
const { NEW_BRIT, OLD_BRIT } = writeStubs();

const notesRef = step => `refs/notes/brit/build/${encodeURIComponent(step)}`;
const addNote = (dir, step, sha, body) =>
  fixtureGit(dir, ['-c', 'user.name=brit', '-c', 'user.email=b@b', 'notes', '--ref', notesRef(step), 'add', '-f', '-m', JSON.stringify(body), sha]);
const showNote = (dir, step, sha) => JSON.parse(fixtureGit(dir, ['notes', '--ref', notesRef(step), 'show', sha]));

/** repoWithNotes plus one ci: note the NEW brit refuses (bad signature) and one gate: note outside the series. */
function repoWithRefusedNote() {
  const r = repoWithNotes();
  const body = { manifestCid: [1], stepName: 'ci:elohim-edge', inputsHash: 't', outputCid: [1], agentId: '00', hardwareProfile: {}, buildDurationMs: 60_000, builtAt: '2026-09-24T10:05:00+00:00', success: true, signature: 'f'.repeat(128) };
  addNote(r.dir, 'ci:elohim-edge', r.c1, body);
  addNote(r.dir, 'gate:x', r.c2, { ...body, stepName: 'gate:x', signature: 'ab'.repeat(64), builtAt: '2026-09-24T11:30:00+00:00' });
  return r;
}

/** repoWithNotes plus one ci: note whose body is not JSON at all, on a commit of its own. */
function repoWithStrayBody() {
  const r = repoWithNotes();
  fixtureGit(r.dir, ['-c', 'user.name=t', '-c', 'user.email=t@t', 'commit', '-q', '--allow-empty', '-m', 'c3']);
  const c3 = fixtureGit(r.dir, ['rev-parse', 'HEAD']);
  fixtureGit(r.dir, ['-c', 'user.name=brit', '-c', 'user.email=b@b', 'notes', '--ref', notesRef('ci:stray'), 'add', '-f', '-m', 'not json', c3]);
  return { ...r, c3 };
}

test('the stubs are written at test time and answer the pinned/new probes exactly', () => {
  const { dir } = repoWithNotes();
  const probe = (bin, args) => spawnSync(bin, ['--repo', dir, ...args], { encoding: 'utf8' });
  const oldSigner = probe(OLD_BRIT, ['signer', '--json']);
  assert.equal(oldSigner.status, 2);
  assert.equal(oldSigner.stderr.split('\n')[0], "error: unrecognized subcommand 'signer'");
  const newSigner = probe(NEW_BRIT, ['signer', '--json']);
  assert.equal(newSigner.status, 0);
  assert.deepEqual(JSON.parse(newSigner.stdout), { agentId: NEW_AGENT });
  const miss = probe(NEW_BRIT, ['build', 'get', '--step', 'nope', '--commit', 'HEAD']);
  assert.equal(miss.status, 0);
  assert.equal(miss.stdout, '');
  assert.equal(miss.stderr, 'no build ref found for step=nope commit=HEAD\n');
});

test('britSigner prefers `signer --json`; the seed file is read only when brit lacks the subcommand', () => {
  const { dir } = repoWithNotes();
  assert.deepEqual(britSigner(NEW_BRIT, dir), { agentId: NEW_AGENT, via: 'signer' });
  // OLD pinned binary: exit 2 "unrecognized subcommand 'signer'" → the seed-file read gate-evidence uses today.
  assert.deepEqual(britSigner(OLD_BRIT, dir), { agentId: null, via: 'agent-key' });
  const common = fixtureGit(dir, ['rev-parse', '--git-common-dir']);
  mkdirSync(join(dir, common, 'brit'), { recursive: true });
  writeFileSync(join(dir, common, 'brit/agent-key'), Buffer.alloc(32, 7));
  const viaFile = britSigner(OLD_BRIT, dir);
  assert.equal(viaFile.via, 'agent-key');
  assert.equal(viaFile.agentId, selfAgentId(dir, undefined, { buildRefBin: OLD_BRIT }), 'the fallback is the same derivation gate-evidence uses');
  assert.deepEqual(britSigner(null, dir), { agentId: null, via: 'none' });
});

test('through the NEW brit, a note it reports unverifiable is absent and named once on the warn stream', () => {
  const { dir, c1, c2 } = repoWithRefusedNote();
  const warnings = [];
  const got = readBuildAttestations(dir, { buildRefBin: NEW_BRIT, warn: l => warnings.push(l) });
  assert.deepEqual(
    got.map(a => [a.step, a.commit, a.success]).sort(),
    [['elohim', c1, true], ['elohim', c2, false], ['elohim-edge', c2, true]].sort(),
    'the refused ci:elohim-edge@c1 note is not a build; ci: round-trips through %3A; gate:x is outside the series'
  );
  assert.ok(got.every(a => a.verified === true && a.reader === 'brit-verified'));
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /refs\/notes\/brit\/build\/ci%3Aelohim-edge/);
  assert.match(warnings[0], new RegExp(c1));
  assert.match(warnings[0], /signature does not verify against agentId "00"/);
});

test('through the NEW brit, an unsigned note (empty signature) is refused as invalid signature hex', () => {
  const { dir, c1 } = repoWithNotes();
  addNote(dir, 'ci:elohim', c1, { ...showNote(dir, 'ci:elohim', c1), signature: '' });
  const warnings = [];
  const got = readBuildAttestations(dir, { buildRefBin: NEW_BRIT, warn: l => warnings.push(l) });
  assert.deepEqual(got.map(a => a.commit).includes(c1), false);
  assert.equal(warnings.length, 1);
  assert.match(warnings[0], /signature does not verify: invalid signature hex — read as absent/);
});

test('through the OLD pinned brit, every read is labelled unverified (it echoes what is stored)', () => {
  const { dir } = repoWithRefusedNote();
  const warnings = [];
  const got = readBuildAttestations(dir, { buildRefBin: OLD_BRIT, warn: l => warnings.push(l) });
  assert.equal(got.length, 4, 'the old binary cannot refuse the bad signature');
  assert.ok(got.every(a => a.verified === false && a.reader === 'brit-unverified'));
  assert.deepEqual(warnings, []);
});

test('with no brit at all, raw git notes are read and labelled raw-git, never verified', () => {
  const { dir } = repoWithRefusedNote();
  const got = readBuildAttestations(dir, { buildRefBin: null });
  assert.equal(got.length, 4);
  assert.ok(got.every(a => a.verified === false && a.reader === 'raw-git'));
});

test('one body that is not JSON never collapses the series: absent and named on every reader', () => {
  const { dir, c1, c2, c3 } = repoWithStrayBody();
  const good = got => got.map(a => [a.step, a.commit]).sort();
  const expect = [['elohim', c1], ['elohim', c2], ['elohim-edge', c2]].sort();

  const raw = readBuildAttestations(dir, { buildRefBin: null });
  assert.deepEqual(good(raw), expect, 'raw git skips a body that is not an attestation');

  // Pinned brit: `Error: JSON error: …` exit 1 is that binary's "does not decode" — absent, counted, named.
  const oldWarn = [];
  const viaOld = readAttestationSeries(dir, { buildRefBin: OLD_BRIT, warn: l => oldWarn.push(l) });
  assert.deepEqual(good(viaOld.attestations), expect);
  assert.equal(viaOld.reader, 'brit-unverified');
  assert.equal(viaOld.unverifiable, 1);
  assert.equal(oldWarn.length, 1);
  assert.match(oldWarn[0], new RegExp(`delivery-series: unverifiable refs/notes/brit/build/ci%3Astray @ ${c3}: does not decode: expected ident at line 1 column 2 \\(pinned brit-build-ref exit 1\\) — read as absent`));

  // NEW brit says it on its own stream, exit 0.
  const newWarn = [];
  const viaNew = readAttestationSeries(dir, { buildRefBin: NEW_BRIT, warn: l => newWarn.push(l) });
  assert.deepEqual(good(viaNew.attestations), expect);
  assert.equal(viaNew.unverifiable, 1);
  assert.match(newWarn[0], /refs\/notes\/brit\/build\/ci%3Astray .*does not decode: /);

  // The verified reader never maps exit 1 to absent: under it an exit 1 is a failure, not a decode.
  const cli = join(import.meta.dirname, 'delivery-series.mjs');
  const r = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir, '--json'], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: OLD_BRIT } });
  const out = JSON.parse(r.stdout);
  assert.equal(out.source, 'attestations', 'the CLI series survives the stray body under the pinned binary');
  assert.equal(out.verification.unverifiable, 1);
  assert.equal(out.considered, 2);
});

test('a git failure inside `build get` is an error, never an empty series', () => {
  const { dir } = repoWithRefusedNote();
  const env = { ...process.env, BRIT_STUB_GIT_FAIL: '1' };
  assert.throws(
    () => readBuildAttestations(dir, { buildRefBin: NEW_BRIT, env }),
    /brit-build-ref build get .*exit 1: Error: git command failed: git notes --ref refs\/notes\/brit\/build\/\S+ list: fatal: stub refused/
  );
});

test('a repository whose every note is refused is named as such, not read as empty, and never falls back to Jenkins', () => {
  const { dir, c1, c2 } = repoWithNotes();
  for (const [step, sha] of [['ci:elohim', c1], ['ci:elohim', c2], ['ci:elohim-edge', c2]]) {
    addNote(dir, step, sha, { ...showNote(dir, step, sha), signature: 'f'.repeat(128) });
  }
  const cli = join(import.meta.dirname, 'delivery-series.mjs');
  // Jenkins is unreachable from this test: a fallback would surface as `Jenkins unreadable`.
  const r = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir, '--json'], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: NEW_BRIT, JENKINS: 'http://127.0.0.1:1' }, timeout: 60_000 });
  assert.match(r.stderr, /no brit build attestations under refs\/notes\/brit\/build\/ in .* \(3 listed, every one unverifiable — read as absent\)\./);
  assert.doesNotMatch(r.stderr, /Jenkins/);
  assert.equal((r.stderr.match(/— read as absent$/gm) ?? []).length, 3, 'each refused note is still named once');
  const out = JSON.parse(r.stdout);
  assert.equal(r.status, 2);
  assert.deepEqual([out.source, out.considered, out.verification.attestations, out.verification.unverifiable], ['attestations', 0, 0, 3]);
});

test('the series carries the verification label into runs and the CLI output', () => {
  const { dir, c1 } = repoWithRefusedNote();
  const runs = attestationRuns(readBuildAttestations(dir, { buildRefBin: NEW_BRIT, warn: () => {} }));
  assert.ok(runs.every(r => r.verified === true));
  const raw = attestationRuns(readBuildAttestations(dir, { buildRefBin: null }));
  assert.ok(raw.every(r => r.verified === false));
  assert.ok('elohim-edge' in raw.find(r => r.commit === c1).outcome, 'raw reads keep the refused note');

  const cli = join(import.meta.dirname, 'delivery-series.mjs');
  const viaNew = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir, '--json'], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: NEW_BRIT } });
  const out = JSON.parse(viaNew.stdout);
  assert.equal(out.source, 'attestations');
  assert.deepEqual(out.verification, { reader: 'brit-verified', buildRef: NEW_BRIT, signer: NEW_AGENT, attestations: 3, unverifiable: 1 });
  assert.ok(out.runs.every(r => r.verified === true));
  assert.match(viaNew.stderr, /delivery-series: unverifiable refs\/notes\/brit\/build\/ci%3Aelohim-edge @ \w{40}: signature does not verify/);

  const viaOld = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: OLD_BRIT } });
  assert.match(viaOld.stdout, /source: attestations, UNVERIFIED — brit-build-ref .* reads notes without verifying them/);

  const verified = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: NEW_BRIT } });
  assert.match(verified.stdout, /^partial: commit-level view; wave wall-clock unknown\nsource: attestations, brit-verified$/m);

  // A declared-but-missing binary resolves to nothing: raw notes, said so.
  const rawCli = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: '/nonexistent/brit-build-ref' } });
  assert.match(rawCli.stdout, /source: attestations, UNVERIFIED — raw git notes, no brit-build-ref on BRIT_BUILD_REF_BIN or PATH/);
  const rawJson = JSON.parse(spawnSync('node', [cli, '--from', 'attestations', '--repo', dir, '--json'], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: '/nonexistent/brit-build-ref' } }).stdout);
  assert.deepEqual(rawJson.verification, { reader: 'raw-git', buildRef: null, signer: null, attestations: 4, unverifiable: 0 });

  const failed = spawnSync('node', [cli, '--from', 'attestations', '--repo', dir], { encoding: 'utf8', env: { ...fixtureGitEnv(dir), BRIT_BUILD_REF_BIN: NEW_BRIT, BRIT_STUB_GIT_FAIL: '1' } });
  assert.equal(failed.status, 2, 'a git failure is NOT MEASURED, not a fallback to Jenkins');
  assert.match(failed.stderr, /NOT MEASURED/);
});
