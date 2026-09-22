/**
 * Timer already-built filter — the predicate that stops a nightly cron run
 * from re-dispatching (and re-rolling the fleet for) a commit a pipeline
 * already built green.
 *
 * Lived cost (2026-09-21): orchestrator/dev #1886 fired on the 09:00Z cron,
 * carried per-pipeline baseline `elohim-edge: bc81e107` — the SAME sha as its
 * own HEAD — and still dispatched elohim-edge/dev #1471, which rebuilt the
 * identical images edge #1470 had already shipped SUCCESS the day before and
 * rolled all seven alpha peers for 3h04m. Every roll restarts the conductor
 * pods and leaves storage cells answering CellDisabled for hours (first
 * recovery +108min; the authoring hosts' lamad cells still down +9h), so the
 * daily timer was reopening a daily write outage for no new information.
 *
 * Run:
 *   node --test genesis/orchestrator/timer-dispatch.test.mjs
 */
import { describe, test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { filterTimerDispatch, isAlreadyBuiltAt } from './timer-dispatch.mjs';

const HEAD = 'bc81e1071b4cb36aa702a9b0dfe68db6e62ea1b1';
const OTHER = '82dcb5c8f00dd41e27b59f8a5f1d90b2e4a6c731';

describe('isAlreadyBuiltAt', () => {
  test('true only for an exact full-sha match', () => {
    assert.ok(isAlreadyBuiltAt(HEAD, HEAD));
    assert.ok(!isAlreadyBuiltAt(OTHER, HEAD));
  });

  test('a short sha is never proof — the baseline record stores full shas', () => {
    assert.ok(!isAlreadyBuiltAt(HEAD.slice(0, 8), HEAD));
    assert.ok(!isAlreadyBuiltAt(HEAD, HEAD.slice(0, 8)));
  });

  test('missing / malformed records prove nothing', () => {
    assert.ok(!isAlreadyBuiltAt(undefined, HEAD));
    assert.ok(!isAlreadyBuiltAt(null, HEAD));
    assert.ok(!isAlreadyBuiltAt('', HEAD));
    assert.ok(!isAlreadyBuiltAt('not-a-sha', 'not-a-sha'));
    assert.ok(!isAlreadyBuiltAt(HEAD, ''));
  });
});

describe('filterTimerDispatch on a TIMER run', () => {
  const base = {
    trigger: 'TIMER',
    headSha: HEAD,
    pipelines: ['elohim-holochain', 'elohim-edge', 'elohim', 'elohim-genesis'],
    baselines: {
      __global__: OTHER,
      'elohim-edge': HEAD,
      'elohim-genesis': OTHER,
    },
    forced: [],
  };

  test('#1886 replayed: edge is dropped, everything else still dispatches', () => {
    const out = filterTimerDispatch(base);
    assert.deepEqual(out.dispatch, ['elohim-holochain', 'elohim', 'elohim-genesis']);
    assert.deepEqual(out.skipped, ['elohim-edge']);
  });

  test('a [build:*] force-include always wins over the filter', () => {
    const out = filterTimerDispatch({ ...base, forced: ['elohim-edge'] });
    assert.deepEqual(out.skipped, []);
    assert.deepEqual(out.dispatch, base.pipelines);
  });

  test('__global__ is a baseline key, never a pipeline name', () => {
    const out = filterTimerDispatch({
      ...base,
      pipelines: ['__global__', 'elohim-edge'],
    });
    assert.deepEqual(out.skipped, ['elohim-edge']);
    assert.deepEqual(out.dispatch, ['__global__']);
  });

  test('a new commit skips nothing — no baseline can equal an unbuilt HEAD', () => {
    const out = filterTimerDispatch({ ...base, headSha: OTHER.replace(/.$/, '0') });
    assert.deepEqual(out.skipped, []);
    assert.deepEqual(out.dispatch, base.pipelines);
  });

  test('an unknown HEAD sha is not proof of anything — dispatch unchanged', () => {
    for (const headSha of ['', undefined, 'HEAD', 'bc81e107']) {
      const out = filterTimerDispatch({ ...base, headSha });
      assert.deepEqual(out.skipped, [], `headSha=${headSha}`);
      assert.deepEqual(out.dispatch, base.pipelines, `headSha=${headSha}`);
    }
  });

  test('missing baselines (cold start) skip nothing', () => {
    assert.deepEqual(filterTimerDispatch({ ...base, baselines: {} }).skipped, []);
    assert.deepEqual(filterTimerDispatch({ ...base, baselines: undefined }).skipped, []);
  });

  test('an empty plan stays empty', () => {
    const out = filterTimerDispatch({ ...base, pipelines: [] });
    assert.deepEqual(out.dispatch, []);
    assert.deepEqual(out.skipped, []);
  });
});

describe('filterTimerDispatch on every other trigger', () => {
  const sameShaEverywhere = {
    headSha: HEAD,
    pipelines: ['elohim-edge'],
    baselines: { 'elohim-edge': HEAD },
    forced: [],
  };

  test('WEBHOOK / MANUAL / unset keep today behaviour verbatim', () => {
    for (const trigger of ['WEBHOOK', 'MANUAL', '', undefined]) {
      const out = filterTimerDispatch({ ...sameShaEverywhere, trigger });
      assert.deepEqual(out.dispatch, ['elohim-edge'], `trigger=${trigger}`);
      assert.deepEqual(out.skipped, [], `trigger=${trigger}`);
    }
  });
});

describe('Jenkinsfile wiring', () => {
  const jf = readFileSync(new URL('./Jenkinsfile', import.meta.url), 'utf8');

  test('the cron trigger is classified TIMER, not MANUAL', () => {
    assert.ok(
      jf.includes('env.BUILD_TRIGGER = classifyBuildTrigger()'),
      'BUILD_TRIGGER must come from classifyBuildTrigger()',
    );
    const start = jf.indexOf('def classifyBuildTrigger(');
    assert.notEqual(start, -1, 'classifyBuildTrigger helper not found');
    const body = jf.slice(start, jf.indexOf('\ndef ', start + 1));
    assert.ok(
      body.includes("TimerTrigger") && body.includes("return 'TIMER'"),
      'a timer run must be distinguishable from an operator-started run',
    );
    assert.ok(
      body.includes("'WEBHOOK'") && body.includes("'MANUAL'"),
      'the other two trigger classes must still be reachable',
    );
  });

  test('the filter is gated on TIMER only', () => {
    const start = jf.indexOf('def applyTimerAlreadyBuiltFilter(');
    assert.notEqual(start, -1, 'applyTimerAlreadyBuiltFilter helper not found');
    const body = jf.slice(start, jf.indexOf('\ndef ', start + 1));
    assert.match(body, /env\.BUILD_TRIGGER != 'TIMER'/);
    assert.match(body, /timer-dispatch\.mjs/, 'the predicate has ONE home — the .mjs module');
  });

  // Both helpers run on the FIRST live cron after this lands, so neither may
  // depend on a sandbox construct this Jenkins has not already executed. A
  // Script Security rejection there is a stageless red on a 09:00Z run nobody
  // is watching. Only plain indexed loops, map-property reads, String.contains
  // and ?: are allowed on the cause list and on readJSON-derived arrays.
  const CODE_ONLY = (body) =>
    body
      .split('\n')
      .filter((l) => !l.trimStart().startsWith('*') && !l.trimStart().startsWith('//'))
      .join('\n');
  const BANNED_ON_SANDBOX_LISTS = [
    '.collect', '.join(\'\\n\')', '.any', '.find{', '.find {',
    '.findAll', '.each', '.inject', '.grep', '@NonCPS', 'Class.forName',
  ];

  for (const helper of ['classifyBuildTrigger', 'applyTimerAlreadyBuiltFilter']) {
    test(`${helper} uses no closure or collection method on a sandbox list`, () => {
      const start = jf.indexOf(`def ${helper}(`);
      assert.notEqual(start, -1, `${helper} not found`);
      const code = CODE_ONLY(jf.slice(start, jf.indexOf('\ndef ', start + 1)));
      for (const banned of BANNED_ON_SANDBOX_LISTS) {
        assert.ok(
          !code.includes(banned),
          `${helper} must not use ${banned} — it runs unproven on the first live cron`,
        );
      }
      assert.ok(
        !/->/.test(code) && !/\{\s*it\b/.test(code),
        `${helper} must not declare a closure`,
      );
      assert.match(code, /for \(int i = 0; i </, `${helper} must iterate with a plain indexed loop`);
    });
  }

  test('classifyBuildTrigger keeps the shortDescription fallback', () => {
    const start = jf.indexOf('def classifyBuildTrigger(');
    const body = jf.slice(start, jf.indexOf('\ndef ', start + 1));
    assert.ok(body.includes("contains('Started by timer')"), 'shortDescription fallback removed');
    assert.ok(body.includes('cause?.shortDescription'), 'safe-navigated property read expected');
    assert.ok(body.includes('cause?._class'), '_class read must be safe-navigated');
  });

  test('the filter runs before the genesis auto-include', () => {
    const routing = jf.slice(jf.indexOf('def applyBuildGraphRouting('));
    const filterIdx = routing.indexOf('applyTimerAlreadyBuiltFilter(');
    const genesisIdx = routing.indexOf('Genesis auto-include');
    assert.ok(filterIdx > 0, 'applyBuildGraphRouting must call the timer filter');
    assert.ok(
      filterIdx < genesisIdx,
      'genesis must not be auto-included on behalf of a pipeline the filter dropped',
    );
  });

  test('[deploy-only] stays webhook-gated (a TIMER is not a WEBHOOK)', () => {
    const idx = jf.indexOf("DEPLOY_ONLY_FROM_TAG = 'true'");
    assert.ok(idx > 0);
    assert.match(jf.slice(idx - 400, idx), /if \(env\.BUILD_TRIGGER == 'WEBHOOK'\) \{/);
  });
});
