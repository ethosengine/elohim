/**
 * Deploy-pending re-dispatch (native-delivery sprint Lane A3).
 *
 * The App pipeline refuses a deploy in seconds when the fleet is not
 * write-ready: it archives deploy-intent.json, emits a failing junit
 * `readiness` case and goes UNSTABLE. That UNSTABLE delivered nothing, so:
 *   - the orchestrator must NOT advance the App baseline on it, and must record
 *     `__pendingDeploy__: {commit, intentCid, …}` in the level checkpoint;
 *   - a later orchestrator run (push or the existing timer) asks the fleet
 *     ONCE and, only when fleet-write-readiness.sh exits 0, dispatches the App
 *     with RUN_CLASS=deploy + DEPLOY_ONLY=true, outside the already-built
 *     filter.
 *
 * The plan's three cases: intent present + ready ⇒ dispatched; intent present
 * + not ready ⇒ held, no baseline advance; no intent ⇒ nothing. The policy is
 * executable here (timer-dispatch.mjs); the Groovy wiring is source-asserted,
 * because running it needs a controller.
 *
 * Run:
 *   node --test genesis/orchestrator/pending-deploy.test.mjs
 */
import { describe, test } from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  APP_PIPELINE,
  PENDING_DEPLOY_KEY,
  intentCid,
  intentViolation,
  pendingDeployPass,
  planNarrowGroups,
  readinessCase,
  readinessRefusal,
  readRefusalEvidence,
} from './timer-dispatch.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const COMMIT = '7d27b04c788302827c4f65fad34802e5952875aa';
const GLOBAL = '82dcb5c8a3096295ed55a66abe58a68eabb80bb2';
const DOORWAYS = ['https://doorway-alpha.elohim.host', 'https://doorway.elohim.host'];

/** The exact shape scripts/ci/fleet-write-readiness.sh write_intent emits. */
const INTENT_TEXT =
  JSON.stringify({
    kind: 'deploy-intent',
    version: 1,
    commit: COMMIT,
    env: 'dev',
    doorway: 'https://doorway-alpha.elohim.host',
    face: 'catching-up',
    retryAfter: 60,
    doorways: DOORWAYS,
    notReady: [{ doorway: 'https://doorway-alpha.elohim.host', face: 'catching-up', retryAfter: 60 }],
    bundles: [{ slug: 'elohim-host-landing', kind: 'browser', sha256: 'sha256-aa' }],
    recordedAt: '2026-09-24T12:00:00Z',
  }) + '\n';

/** The shape emitAppDeployJunit writes for a refused precondition. */
const JUNIT_REFUSED = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<testsuite name="elohim-app.deploy.dev" tests="1" failures="1">',
  '  <testcase classname="elohim-app.deploy.dev" name="readiness" time="1.204"><failure message="Fleet not write-ready — FLEET-NOT-READY https://doorway-alpha.elohim.host face=catching-up retryAfter=60" type="spa-blob-readiness"/></testcase>',
  '</testsuite>',
].join('\n');

const JUNIT_DELIVERED = [
  '<?xml version="1.0" encoding="UTF-8"?>',
  '<testsuite name="elohim-app.deploy.dev" tests="2" failures="1">',
  '  <testcase classname="elohim-app.deploy.dev" name="readiness" time="0.900"/>',
  '  <testcase classname="elohim-app.deploy.dev" name="verify.shell doorway.elohim.host elohim-host-landing" time="3.000"><failure message="x" type="spa-blob-shell"/></testcase>',
  '</testsuite>',
].join('\n');

const PENDING = readinessRefusal({
  result: 'UNSTABLE',
  intentText: INTENT_TEXT,
  junitText: JUNIT_REFUSED,
  build: 1725,
}).pendingDeploy;

/** dependsOn + probe double; records every probe call. */
function stubDeps({ rc = 0, lines = '', dependsOn = ['elohim-sophia', 'elohim-edge'], fail } = {}) {
  const probes = [];
  return {
    probes,
    async dependsOn() {
      if (fail === 'dependsOn') throw new Error('ENOENT build-manifest.json');
      return dependsOn;
    },
    probe(doorways) {
      probes.push([...doorways]);
      if (fail === 'probe') throw new Error('spawn bash ENOENT');
      return { rc, lines };
    },
  };
}

const withPending = (extra = {}) => ({
  trigger: 'TIMER',
  wave: [],
  suppressed: '',
  baselines: { __global__: GLOBAL, elohim: GLOBAL, [PENDING_DEPLOY_KEY]: PENDING },
  ...extra,
});

// ── Recording: which UNSTABLE holds the baseline ────────────────────────────

describe('readinessRefusal — the UNSTABLE that delivered nothing', () => {
  test('a refused precondition holds, with the intent CID of the exact bytes', () => {
    const out = readinessRefusal({
      result: 'UNSTABLE',
      intentText: INTENT_TEXT,
      junitText: JUNIT_REFUSED,
      build: 1725,
    });
    assert.equal(out.hold, true);
    const sha = createHash('sha256').update(INTENT_TEXT).digest('hex');
    assert.equal(out.pendingDeploy.intentCid, `sha256-${sha}`);
    assert.equal(out.pendingDeploy.intentCid, intentCid(INTENT_TEXT));
    assert.equal(out.pendingDeploy.commit, COMMIT);
    assert.deepEqual(out.pendingDeploy.doorways, DOORWAYS);
    assert.equal(out.pendingDeploy.env, 'dev');
    assert.equal(out.pendingDeploy.build, '1725');
    assert.match(out.reason, /face=catching-up/);
  });

  test('SUCCESS and FAILURE never hold on this rule', () => {
    for (const result of ['SUCCESS', 'FAILURE', 'ABORTED', undefined]) {
      const out = readinessRefusal({ result, intentText: INTENT_TEXT, junitText: JUNIT_REFUSED });
      assert.equal(out.hold, false, String(result));
      assert.equal(out.pendingDeploy, undefined);
    }
  });

  test('an UNSTABLE with no archived intent delivered (some legs flaked) — advance', () => {
    for (const intentText of [null, '', '   \n']) {
      const out = readinessRefusal({ result: 'UNSTABLE', intentText, junitText: JUNIT_DELIVERED });
      assert.equal(out.hold, false);
      assert.match(out.reason, /no deploy-intent\.json archived/);
    }
  });

  test('a junit report whose readiness case passed contradicts the intent — advance', () => {
    const out = readinessRefusal({
      result: 'UNSTABLE',
      intentText: INTENT_TEXT,
      junitText: JUNIT_DELIVERED,
    });
    assert.equal(out.hold, false);
    assert.match(out.reason, /readiness case is passed/);
    const absent = readinessRefusal({
      result: 'UNSTABLE',
      intentText: INTENT_TEXT,
      junitText: '<testsuite><testcase name="publish.seed a b (browser)"/></testsuite>',
    });
    assert.equal(absent.hold, false);
    assert.match(absent.reason, /readiness case is absent/);
  });

  test('an unreadable junit leaves the intent decisive (it is archived only on a refusal)', () => {
    const out = readinessRefusal({ result: 'UNSTABLE', intentText: INTENT_TEXT, junitText: null });
    assert.equal(out.hold, true);
  });

  test('an intent that cannot be re-dispatched is named, never recorded', () => {
    const cases = [
      ['{not json', /not JSON/],
      [JSON.stringify({ kind: 'other' }), /kind is "other"/],
      [JSON.stringify({ kind: 'deploy-intent', commit: 'abc' }), /no full-40-hex commit/],
      [JSON.stringify({ kind: 'deploy-intent', commit: COMMIT, doorways: [] }), /names no doorways/],
      [
        JSON.stringify({ kind: 'deploy-intent', commit: COMMIT, doorways: ["https://x'; rm -rf /"], notReady: [{}] }),
        /not a plain http\(s\) URL/,
      ],
      [
        JSON.stringify({ kind: 'deploy-intent', commit: COMMIT, doorways: DOORWAYS, notReady: [] }),
        /nothing was refused/,
      ],
    ];
    for (const [intentText, why] of cases) {
      const out = readinessRefusal({ result: 'UNSTABLE', intentText, junitText: JUNIT_REFUSED });
      assert.equal(out.hold, false, intentText);
      assert.match(out.reason, why);
    }
    assert.equal(intentViolation(JSON.parse(INTENT_TEXT)), null);
  });

  test('readinessCase reads the self-closing and the failing forms', () => {
    assert.equal(readinessCase(JUNIT_REFUSED), 'refused');
    assert.equal(readinessCase(JUNIT_DELIVERED), 'passed');
    assert.equal(readinessCase(''), 'absent');
    // Another case's failure is not the readiness case's.
    assert.equal(
      readinessCase('<testcase name="readiness"/><testcase name="x"><failure/></testcase>'),
      'passed',
    );
  });
});

// ── The pass: the plan's three cases, and every hold ─────────────────────────

describe('pendingDeployPass — re-dispatch is an event, never a poll', () => {
  test('intent present + fleet ready ⇒ the App is dispatched', async () => {
    const deps = stubDeps({ rc: 0, lines: 'FLEET-READY https://doorway-alpha.elohim.host\n' });
    const out = await pendingDeployPass(withPending(), deps);
    assert.equal(out.dispatch, true);
    assert.equal(out.probeRc, 0);
    assert.deepEqual(deps.probes, [DOORWAYS], 'the probe asks the intent\'s own doorways, once');
    assert.match(out.logLines[0], /RUN_CLASS=deploy DEPLOY_ONLY=true/);
    assert.match(out.logLines[0], /exempt from the already-built filter/);
    assert.equal(APP_PIPELINE, 'elohim');
  });

  test('intent present + fleet not ready ⇒ held (no dispatch)', async () => {
    const deps = stubDeps({
      rc: 3,
      lines: 'FLEET-NOT-READY https://doorway-alpha.elohim.host face=catching-up retryAfter=60\n',
    });
    const out = await pendingDeployPass(withPending(), deps);
    assert.equal(out.dispatch, false);
    assert.equal(out.probeRc, 3);
    assert.deepEqual(out.pending, PENDING, 'the pending intent is carried, not dropped');
    assert.match(out.logLines[0], /held — fleet not write-ready: FLEET-NOT-READY/);
  });

  test('no intent ⇒ nothing: no probe, no log line', async () => {
    for (const baselines of [{}, { __global__: GLOBAL }, { [PENDING_DEPLOY_KEY]: null }]) {
      const deps = stubDeps();
      const out = await pendingDeployPass({ ...withPending(), baselines }, deps);
      assert.deepEqual(out, { dispatch: false, pending: null, probeRc: null, logLines: [] });
      assert.equal(deps.probes.length, 0);
    }
    const deps = stubDeps();
    const out = await pendingDeployPass(undefined, deps);
    assert.equal(out.dispatch, false);
    assert.equal(deps.probes.length, 0);
  });

  test('a probe that cannot judge (exit 2, killed, or never ran) holds', async () => {
    for (const deps of [stubDeps({ rc: 2 }), stubDeps({ rc: null }), stubDeps({ fail: 'probe' })]) {
      const out = await pendingDeployPass(withPending(), deps);
      assert.equal(out.dispatch, false);
      assert.match(out.logLines[0], /held — the (probe could not judge|readiness probe did not run)/);
    }
  });

  test('a suppressed run never probes (run-class tag, validate-only, deploy-only, manual mode)', async () => {
    const deps = stubDeps();
    const out = await pendingDeployPass(
      withPending({ suppressed: '[run:verify] caps this run below deploy' }),
      deps,
    );
    assert.equal(out.dispatch, false);
    assert.equal(deps.probes.length, 0);
    assert.match(out.logLines[0], /\[run:verify\]/);
  });

  test('an App already in the wave is delivered by the wave — no second dispatch', async () => {
    const deps = stubDeps();
    const out = await pendingDeployPass(withPending({ wave: ['elohim-edge', 'elohim'] }), deps);
    assert.equal(out.dispatch, false);
    assert.equal(deps.probes.length, 0);
    assert.match(out.logLines[0], /already in this wave/);
  });

  test('a wave that rolls a producer the App depends on holds (ready now ≠ ready after)', async () => {
    const deps = stubDeps({ rc: 0 });
    const out = await pendingDeployPass(withPending({ wave: ['elohim-holochain', 'elohim-edge'] }), deps);
    assert.equal(out.dispatch, false);
    assert.equal(deps.probes.length, 0);
    assert.match(out.logLines[0], /dispatches elohim-edge, which rolls the fleet/);
  });

  test('an unreadable dependsOn holds', async () => {
    const out = await pendingDeployPass(withPending(), stubDeps({ fail: 'dependsOn' }));
    assert.equal(out.dispatch, false);
    assert.match(out.logLines[0], /dependsOn is unreadable/);
  });

  test('an unusable record is named and never probed', async () => {
    for (const bad of ['sha', { commit: COMMIT }, { ...PENDING, doorways: ['file:///etc'] }, []]) {
      const deps = stubDeps();
      const out = await pendingDeployPass(
        { ...withPending(), baselines: { [PENDING_DEPLOY_KEY]: bad } },
        deps,
      );
      assert.equal(out.dispatch, false);
      assert.equal(deps.probes.length, 0);
      assert.match(out.logLines[0], /present but unusable/);
    }
  });

  test('the level-checkpoint key is bookkeeping, never a pipeline name', async () => {
    const out = await planNarrowGroups({
      headSha: COMMIT,
      pipelines: [PENDING_DEPLOY_KEY],
      baselines: { __global__: GLOBAL, [PENDING_DEPLOY_KEY]: PENDING },
    });
    assert.deepEqual(out, { groups: {}, provenance: {}, notes: {} });
  });
});

// ── The CLI the Jenkinsfile shells out to RUNS ───────────────────────────────

describe('the timer-dispatch.mjs verbs the Jenkinsfile calls execute', () => {
  const cli = join(HERE, 'timer-dispatch.mjs');

  test('readiness-refusal reads the copied artifact directory', () => {
    const dir = mkdtempSync(join(tmpdir(), 'pending-deploy-'));
    try {
      writeFileSync(join(dir, 'deploy-intent.json'), INTENT_TEXT);
      writeFileSync(join(dir, 'deploy-app-dev-junit.xml'), JUNIT_REFUSED);
      assert.deepEqual(readRefusalEvidence(dir), { intentText: INTENT_TEXT, junitText: JUNIT_REFUSED });
      const out = JSON.parse(
        execFileSync('node', [cli, 'readiness-refusal', 'UNSTABLE', dir, '1725'], { encoding: 'utf8' }),
      );
      assert.equal(out.hold, true);
      assert.equal(out.pendingDeploy.intentCid, intentCid(INTENT_TEXT));
      // An empty directory (copyArtifacts optional:true found nothing) advances.
      const empty = mkdtempSync(join(tmpdir(), 'pending-deploy-empty-'));
      const none = JSON.parse(
        execFileSync('node', [cli, 'readiness-refusal', 'UNSTABLE', empty], { encoding: 'utf8' }),
      );
      assert.equal(none.hold, false);
      rmSync(empty, { recursive: true, force: true });
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  test('pending runs the REAL fleet-write-readiness.sh and holds on an unreachable doorway', () => {
    const dir = mkdtempSync(join(tmpdir(), 'pending-deploy-state-'));
    try {
      const state = join(dir, 'state.json');
      writeFileSync(
        state,
        JSON.stringify(
          withPending({
            baselines: { [PENDING_DEPLOY_KEY]: { ...PENDING, doorways: ['http://127.0.0.1:9'] } },
          }),
        ),
      );
      const out = JSON.parse(
        execFileSync('node', [cli, 'pending', state], {
          encoding: 'utf8',
          stdio: ['ignore', 'pipe', 'ignore'],
        }),
      );
      assert.equal(out.dispatch, false);
      assert.ok(out.probeRc === 3 || out.probeRc === 2, `probeRc ${out.probeRc}`);
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });
});

// ── The Groovy wiring (source-asserted) ──────────────────────────────────────

describe('orchestrator Jenkinsfile wiring', () => {
  const jf = readFileSync(join(HERE, 'Jenkinsfile'), 'utf8');
  const bodyOf = (name) => {
    const start = jf.indexOf(`def ${name}(`);
    assert.notEqual(start, -1, `${name} not found`);
    // Up to the next top-level def, doc comment or annotation.
    const ends = ['\ndef ', '\n/**', '\n@NonCPS']
      .map((m) => jf.indexOf(m, start + 1))
      .filter((i) => i !== -1);
    return jf.slice(start, ends.length ? Math.min(...ends) : undefined);
  };
  const code = (body) =>
    body
      .split('\n')
      .filter((l) => !l.trimStart().startsWith('*') && !l.trimStart().startsWith('//'))
      .join('\n');

  test('a readiness refusal does NOT advance the App baseline and records pendingDeploy', () => {
    const body = code(bodyOf('recordPipelineResult'));
    assert.match(body, /def pending = \(result\.success && result\.unstable\) \? readinessRefusal\(name, result\) : null/);
    const branch = body.slice(body.indexOf('} else if (pending != null) {'), body.indexOf('} else {', body.indexOf('} else if (pending != null) {')));
    assert.ok(branch.length > 0, 'the refusal branch exists');
    assert.match(branch, /pipelineBaselines\['__pendingDeploy__'\] = pending/);
    assert.doesNotMatch(branch, /pipelineBaselines\[name\]/, 'the refusal branch must not advance the baseline');
    // The pending check sits AFTER skipped/dispatched and BEFORE the advancing branch.
    assert.ok(body.indexOf('result.dispatched') < body.indexOf('pending != null'));
  });

  test('a delivered App clears the intent; a red re-dispatch drops it', () => {
    const body = code(bodyOf('recordPipelineResult'));
    assert.match(body, /name == 'elohim' && pipelineBaselines\.containsKey\('__pendingDeploy__'\)/);
    assert.match(body, /name == env\.PENDING_DEPLOY_DISPATCH && pipelineBaselines\.containsKey\('__pendingDeploy__'\)/);
  });

  test('the key rides the level checkpoint (archivePipelineBaselines starts from the env bridge)', () => {
    const body = bodyOf('archivePipelineBaselines');
    assert.match(body, /readJSON\(text: env\.PIPELINE_BASELINES\)/);
    assert.doesNotMatch(code(body), /__pendingDeploy__/, 'the checkpoint must not strip the key');
  });

  test('readinessRefusal copies the two App artifacts and runs the tested verb', () => {
    const body = bodyOf('readinessRefusal');
    assert.match(body, /filter: 'deploy-intent\.json,deploy-app-\*-junit\.xml'/);
    assert.match(body, /selector: specific\(result\.buildNumber\.toString\(\)\)/);
    assert.match(body, /timer-dispatch\.mjs readiness-refusal UNSTABLE/);
    assert.match(body, /throw e/, 'interruption is re-thrown');
  });

  test('the pass runs after the already-built filter and the genesis auto-include, before the run classes are stashed', () => {
    const body = bodyOf('applyBuildGraphRouting');
    const filter = body.indexOf('applyAlreadyBuiltFilter(graphPipelines)');
    const genesis = body.indexOf("graphPipelines.add('elohim-genesis')");
    const pass = body.indexOf('applyDeployPendingPass(mode, graphPipelines, runClasses)');
    const stash = body.indexOf('env.PIPELINE_RUN_CLASSES =');
    assert.ok(filter > 0 && genesis > filter && pass > genesis && stash > pass, `${filter} ${genesis} ${pass} ${stash}`);
  });

  test('a dispatch adds the App as class deploy and flags DEPLOY_ONLY for it alone', () => {
    const body = code(bodyOf('applyDeployPendingPass'));
    assert.match(body, /timer-dispatch\.mjs pending/);
    assert.match(body, /runWithStorageAdminKey\(/);
    assert.match(body, /runClasses\['elohim'\] = 'deploy'/);
    assert.match(body, /env\.PENDING_DEPLOY_DISPATCH = 'elohim'/);
    for (const why of ['RUN_CLASS_FROM_TAG', 'EDGE_VALIDATE_ONLY_FROM_TAG', 'DEPLOY_ONLY_FROM_TAG', "mode != 'auto'"]) {
      assert.ok(body.includes(why), `suppression ${why}`);
    }
    const trigger = bodyOf('triggerPipeline');
    assert.match(trigger, /env\.PENDING_DEPLOY_DISPATCH == name/);
  });

  test('the new helpers are closure-free over sandbox lists', () => {
    for (const helper of ['readinessRefusal', 'runWithStorageAdminKey', 'applyDeployPendingPass']) {
      const body = code(bodyOf(helper));
      for (const banned of ['.collect', '.any', '.findAll', '.each', '.inject', '@NonCPS']) {
        assert.ok(!body.includes(banned), `${helper} must not use ${banned}`);
      }
      assert.ok(!/->/.test(body), `${helper} must not declare a closure`);
      assert.ok(body.length < 8000, `${helper} is ${body.length}B`);
    }
  });

  test('the cron is still the one daily slot (a 30-min timer would abortPrevious a live run)', () => {
    assert.match(jf, /cron\('0 9 \* \* \*'\)/);
    assert.match(jf, /disableConcurrentBuilds\(abortPrevious: true\)/);
  });
});
