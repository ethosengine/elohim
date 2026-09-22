/**
 * Already-built dispatch filter — the predicate that stops the orchestrator
 * from re-dispatching (and re-rolling the fleet for) a range a pipeline has
 * already built, without ever dropping work a consumer still needs.
 *
 * Lived cost, twice:
 *   - #1886 (TIMER, 2026-09-21) carried `elohim-edge: bc81e107` — the SAME sha
 *     as its own HEAD — and still dispatched elohim-edge/dev #1471, which
 *     rebuilt the images edge #1470 had shipped SUCCESS the day before and
 *     rolled all seven alpha peers for 3h04m.
 *   - #1888 (WEBHOOK, 2026-09-22 04:51Z, HEAD 7d27b04c) carried
 *     `__global__: 82dcb5c8` (frozen: #1884–#1887 all died on the 4h timeout)
 *     beside `elohim-holochain: 86e7c220`, and dispatched the ~53min DNA
 *     pipeline although `86e7c220..7d27b04c` touches nothing under
 *     elohim/holochain.
 *
 * The matcher is NOT tested here, because there is no matcher here: phase 2 is
 * `build-graph.groovy::walkBuildGraph`, called from the Jenkinsfile. What is
 * tested is the git/provenance phase, the decision phase (including producer
 * readiness), the Jenkinsfile wiring, and that the CLI the Jenkinsfile actually
 * shells out to RUNS — see 'the Jenkinsfile sh lines execute'.
 *
 * Run:
 *   node --test genesis/orchestrator/timer-dispatch.test.mjs
 */
import { describe, test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, execSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { isBuiltin } from 'node:module';
import {
  decideDispatch,
  defaultDeps,
  partitionViolation,
  planNarrowGroups,
} from './timer-dispatch.mjs';

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, '../..');

const HEAD = '7d27b04c788302827c4f65fad34802e5952875aa';
const GLOBAL = '82dcb5c8a3096295ed55a66abe58a68eabb80bb2';
const BUILT = '86e7c2202d2844ad6313c3446f35aea49584f7a9';
const OTHER = 'bc81e1071b4cb36aa702a9b0dfe68db6e62ea1b1';

/**
 * A git+manifest+controller double. `ancestryOrder` is oldest→newest;
 * `provenance` / `dependsOn` / `jobs` / `lastSuccess` model the manifests and
 * the controller. `fail` makes one dep throw.
 */
function stubDeps({
  commits = [GLOBAL, BUILT, OTHER],
  ancestryOrder = [GLOBAL, BUILT, HEAD],
  files = {},
  provenance = {},
  dependsOn = {},
  fail,
} = {}) {
  return {
    hasCommit(sha) {
      if (fail === 'hasCommit') throw new Error('fatal: not a git repository');
      return commits.includes(sha);
    },
    isAncestor(a, b) {
      if (fail === 'isAncestor') throw new Error('fatal: bad revision');
      const ia = ancestryOrder.indexOf(a);
      const ib = ancestryOrder.indexOf(b);
      return ia !== -1 && ib !== -1 && ia < ib;
    },
    changedFiles(from) {
      if (fail === 'changedFiles') throw new Error('fatal: bad object');
      return files[from] ?? [];
    },
    async provenanceOf(name) {
      if (fail === 'provenanceOf') throw new Error('ENOENT build-manifest.json');
      return Object.hasOwn(provenance, name) ? provenance[name] : 'verdict';
    },
    async dependsOn(name) {
      if (fail === 'dependsOn') throw new Error('ENOENT build-manifest.json');
      return dependsOn[name] ?? [];
    },
  };
}

// ── Phase 1: git + provenance, no matching ──────────────────────────────────

describe('planNarrowGroups — the git/provenance phase', () => {
  const base = {
    trigger: 'WEBHOOK',
    headSha: HEAD,
    branch: 'dev',
    pipelines: ['elohim-holochain', 'elohim-edge', 'elohim-genesis'],
    baselines: {
      __global__: GLOBAL,
      'elohim-holochain': BUILT,
      'elohim-edge': BUILT,
      'elohim-genesis': GLOBAL,
    },
    forced: [],
  };

  test('groups candidates by DISTINCT baseline and carries the narrow file list', async () => {
    const out = await planNarrowGroups(
      base,
      stubDeps({ files: { [BUILT]: ['elohim/elohim-storage/src/lib.rs'] } }),
    );
    assert.deepEqual(Object.keys(out.groups), [BUILT]);
    assert.deepEqual(out.groups[BUILT].pipelines, ['elohim-holochain', 'elohim-edge']);
    assert.deepEqual(out.groups[BUILT].changedFiles, ['elohim/elohim-storage/src/lib.rs']);
    // baseline == __global__ is never a candidate, and says why.
    assert.ok(!out.groups[BUILT].pipelines.includes('elohim-genesis'));
    assert.match(out.notes['elohim-genesis'], /equals the global diff base/);
  });

  test('emits one group per baseline so the Groovy walker runs once per range', async () => {
    const out = await planNarrowGroups(
      {
        ...base,
        baselines: { ...base.baselines, 'elohim-edge': OTHER },
      },
      stubDeps({
        ancestryOrder: [GLOBAL, OTHER, BUILT, HEAD],
        files: { [BUILT]: ['a'], [OTHER]: ['b'] },
      }),
    );
    assert.deepEqual(Object.keys(out.groups).sort(), [BUILT, OTHER].sort());
    assert.deepEqual(out.groups[BUILT].pipelines, ['elohim-holochain']);
    assert.deepEqual(out.groups[OTHER].pipelines, ['elohim-edge']);
  });

  test('records provenance from the manifest longRunning field', async () => {
    const out = await planNarrowGroups(
      base,
      stubDeps({
        files: { [BUILT]: [] },
        provenance: { 'elohim-holochain': 'dispatch-only', 'elohim-edge': 'verdict' },
      }),
    );
    assert.equal(out.provenance['elohim-holochain'], 'dispatch-only');
    assert.equal(out.provenance['elohim-edge'], 'verdict');
  });

  const noCandidates = async (state, deps = stubDeps({ files: { [BUILT]: [] } })) => {
    const out = await planNarrowGroups(state, deps);
    assert.deepEqual(out.groups, {});
  };

  test('every uncertainty yields no candidate at all', async () => {
    const one = { ...base, pipelines: ['elohim-holochain'] };
    // forced
    await noCandidates({ ...one, forced: ['elohim-holochain'] });
    // missing / short / non-string baseline
    for (const b of [undefined, null, '', '86e7c220', 'not-a-sha', 42]) {
      await noCandidates({ ...one, baselines: { __global__: GLOBAL, 'elohim-holochain': b } });
    }
    // missing / unparseable __global__
    for (const g of [undefined, '', '82dcb5c8', 'nope']) {
      await noCandidates({ ...one, baselines: { __global__: g, 'elohim-holochain': BUILT } });
    }
    // unusable HEAD
    for (const h of ['', undefined, 'HEAD', '7d27b04c']) {
      await noCandidates({ ...one, headSha: h });
    }
    // B_P older than, or equal to, the global base
    await noCandidates({ ...one, baselines: { __global__: BUILT, 'elohim-holochain': GLOBAL } });
    await noCandidates({ ...one, baselines: { __global__: GLOBAL, 'elohim-holochain': GLOBAL } });
    // unrelated branch (ancestor in neither direction)
    await noCandidates(
      { ...one, baselines: { __global__: GLOBAL, 'elohim-holochain': OTHER } },
      stubDeps({ ancestryOrder: [GLOBAL, BUILT, HEAD], files: {} }),
    );
    // unresolvable commit (force-push / shallow clone)
    await noCandidates(one, stubDeps({ commits: [GLOBAL], files: {} }));
    // git failures
    for (const f of ['hasCommit', 'isAncestor', 'changedFiles', 'provenanceOf']) {
      await noCandidates(one, stubDeps({ files: { [BUILT]: [] }, fail: f }));
    }
    // provenance undeterminable (no manifest in this checkout)
    await noCandidates(one, stubDeps({ files: { [BUILT]: [] }, provenance: { 'elohim-holochain': null } }));
  });

  test('__global__ is a baseline key, never a pipeline name', async () => {
    const out = await planNarrowGroups(
      { ...base, pipelines: ['__global__'] },
      stubDeps({ files: { [BUILT]: [] } }),
    );
    assert.deepEqual(out.groups, {});
  });

  test('never throws, whatever it is handed', async () => {
    for (const state of [undefined, {}, { pipelines: null }, { headSha: 5, pipelines: ['x'] }]) {
      await planNarrowGroups(state, stubDeps());
    }
  });
});

// ── Phase 3: the decision ───────────────────────────────────────────────────

describe('#1888 replayed — the Groovy walk decides, the module obeys', () => {
  const state = {
    trigger: 'WEBHOOK',
    headSha: HEAD,
    pipelines: ['elohim-holochain', 'elohim-edge', 'elohim-epr', 'elohim-genesis'],
  };
  // The REAL facts, not a convenient fiction: elohim/holochain/dna carries
  // `longRunning: true`, and elohim/holochain (elohim-edge) carries
  // `dependsOn: ['elohim-holochain', 'elohim-conductor']`. The earlier cut of
  // this fixture claimed DNA was verdict-backed and had no consumers, which
  // hid the producer-readiness rule entirely.
  const plan = {
    groups: {
      [BUILT]: {
        changedFiles: ['elohim/elohim-storage/src/lib.rs', 'doorway/doorway-service/src/http.rs'],
        pipelines: ['elohim-holochain', 'elohim-edge', 'elohim-epr'],
      },
    },
    provenance: {
      'elohim-holochain': 'dispatch-only',
      'elohim-edge': 'verdict',
      'elohim-epr': 'verdict',
    },
    notes: {},
  };
  const realEdges = {
    dependsOn: {
      'elohim-edge': ['elohim-holochain', 'elohim-conductor'],
      'elohim-genesis': ['elohim-edge', 'elohim'],
    },
  };
  // What walkBuildGraph returns for that narrow range: edge, not holochain/epr.
  const walks = { [BUILT]: ['elohim-edge', 'elohim-genesis'] };

  test('epr is skipped, but DNA is KEPT because surviving edge depends on it', async () => {
    const out = await decideDispatch(state, plan, walks, stubDeps(realEdges));
    assert.deepEqual(out.skipped, ['elohim-epr']);
    assert.deepEqual(out.dispatch, ['elohim-holochain', 'elohim-edge', 'elohim-genesis']);
    assert.equal(out.logLines.length, 2);
    assert.ok(out.logLines[0].includes('elohim-holochain KEPT'));
    assert.ok(out.logLines[0].includes('surviving pipeline(s) elohim-edge'));
  });

  test('genesis alone still keeps DNA: it reaches it through the SKIPPED edge', async () => {
    // Survivors are traversal ROOTS and traversal continues through skipped
    // intermediates — genesis -> edge -> DNA. A skipped edge contributes no
    // fresh artifact binding, so it cannot shield DNA.
    const out = await decideDispatch(state, plan, { [BUILT]: ['elohim-genesis'] }, stubDeps(realEdges));
    assert.deepEqual(out.skipped, ['elohim-edge', 'elohim-epr']);
    assert.deepEqual(out.dispatch, ['elohim-holochain', 'elohim-genesis']);
    assert.ok(out.logLines[0].includes('elohim-holochain KEPT'));
  });

  test('when the whole wave is unchanged, the 53-minute DNA dispatch really is dropped', async () => {
    // This is the shape the filter actually buys: nothing survives, so nothing
    // reaches DNA and the optimistic baseline is enough.
    const out = await decideDispatch(
      { ...state, pipelines: ['elohim-holochain', 'elohim-edge', 'elohim-epr'] },
      plan,
      { [BUILT]: [] },
      stubDeps(realEdges),
    );
    assert.deepEqual(out.skipped, ['elohim-holochain', 'elohim-edge', 'elohim-epr']);
    assert.deepEqual(out.dispatch, []);
    assert.ok(
      out.logLines[0].includes(
        'dispatched (longRunning: verdict never recorded), nothing surviving needs it',
      ),
    );
  });

  test('the verdict-backed log line names trigger, pipeline, short baseline and reason', async () => {
    const out = await decideDispatch(state, plan, walks, stubDeps(realEdges));
    assert.equal(
      out.logLines[1],
      '⏭️  WEBHOOK already-built filter: elohim-epr skipped — baseline 86e7c220 ' +
        'built green, no watched input changed since (force with [build:*])',
    );
  });

  test('a group the walker did not answer for dispatches every member', async () => {
    for (const walked of [{}, { [BUILT]: null }, { [BUILT]: 'nope' }]) {
      const out = await decideDispatch(state, plan, walked, stubDeps(realEdges));
      assert.deepEqual(out.skipped, []);
      assert.deepEqual(out.dispatch, state.pipelines);
      assert.deepEqual(out.logLines, []);
    }
  });

  test('the walker including a pipeline always wins', async () => {
    const out = await decideDispatch(
      state,
      plan,
      { [BUILT]: ['elohim-holochain', 'elohim-edge', 'elohim-epr'] },
      stubDeps(realEdges),
    );
    assert.deepEqual(out.skipped, []);
  });

  test('the filter is trigger-agnostic — trigger is a label, not a gate', async () => {
    for (const trigger of ['TIMER', 'MANUAL', 'WEBHOOK', '', undefined]) {
      const out = await decideDispatch({ ...state, trigger }, plan, walks, stubDeps(realEdges));
      assert.deepEqual(out.skipped, ['elohim-epr'], `trigger=${trigger}`);
    }
  });

  test('an empty plan stays empty', async () => {
    const out = await decideDispatch({ ...state, pipelines: [] }, plan, walks, stubDeps());
    assert.deepEqual(out.dispatch, []);
    assert.deepEqual(out.skipped, []);
  });
});

describe('the skip reason names what the baseline actually proves', () => {
  // ONE decision, two provenances. elohim-holochain is `longRunning: true` in
  // elohim/holochain/dna/build-manifest.json, so recordPipelineResult advances
  // its baseline at DISPATCH (Jenkinsfile:750 -> :622-628 -> :521-529) and
  // "green" was never recorded. elohim-edge waits for its verdict
  // (Jenkinsfile:530-535), so its baseline really is a green build.
  const state = {
    trigger: 'WEBHOOK',
    headSha: HEAD,
    branch: 'dev',
    pipelines: ['elohim-holochain', 'elohim-edge'],
  };
  const plan = {
    groups: { [BUILT]: { changedFiles: [], pipelines: ['elohim-holochain', 'elohim-edge'] } },
    provenance: { 'elohim-holochain': 'dispatch-only', 'elohim-edge': 'verdict' },
    notes: {},
  };
  const walks = { [BUILT]: [] };

  test('longRunning reads "dispatched", verdict-backed reads "built green"', async () => {
    // No consumer of holochain survives, so producer readiness does not apply.
    const out = await decideDispatch(state, plan, walks, stubDeps());
    assert.deepEqual(out.skipped, ['elohim-holochain', 'elohim-edge']);
    assert.equal(out.logLines.length, 2);
    assert.equal(
      out.logLines[0],
      '⏭️  WEBHOOK already-built filter: elohim-holochain skipped — baseline 86e7c220 ' +
        'dispatched (longRunning: verdict never recorded), nothing surviving needs it, ' +
        'no watched input changed since (force with [build:*])',
    );
    assert.equal(
      out.logLines[1],
      '⏭️  WEBHOOK already-built filter: elohim-edge skipped — baseline 86e7c220 ' +
        'built green, no watched input changed since (force with [build:*])',
    );
    assert.ok(!out.logLines[0].includes('built green'));
    assert.ok(!out.logLines[1].includes('longRunning'));
  });
});

describe('producer readiness — an optimistic producer a consumer still needs', () => {
  // edge dependsOn holochain; holochain's baseline records only a dispatch.
  const state = {
    trigger: 'WEBHOOK',
    headSha: HEAD,
    pipelines: ['elohim-holochain', 'elohim-edge'],
  };
  const plan = {
    groups: { [BUILT]: { changedFiles: ['x'], pipelines: ['elohim-holochain'] } },
    provenance: { 'elohim-holochain': 'dispatch-only' },
    notes: {},
  };
  // The narrow walk excludes holochain, so it is skippable on inputs alone.
  const walks = { [BUILT]: ['elohim-edge'] };
  const graph = { dependsOn: { 'elohim-edge': ['elohim-holochain'] } };

  test('consumer selected + dispatch-only → producer KEPT, unconditionally', async () => {
    const out = await decideDispatch(state, plan, walks, stubDeps(graph));
    assert.deepEqual(out.skipped, []);
    assert.deepEqual(out.dispatch, ['elohim-holochain', 'elohim-edge']);
    assert.equal(out.logLines.length, 1);
    assert.equal(
      out.logLines[0],
      '▶️  WEBHOOK already-built filter: elohim-holochain KEPT — baseline 86e7c220 only ' +
        'records a dispatch (longRunning: verdict never recorded) and surviving pipeline(s) ' +
        'elohim-edge reach it through dependsOn; a floating artifact tag cannot be proven ' +
        'from a baseline sha',
    );
  });

  test('NO controller-evidence hook exists to bypass that — the escape is gone', async () => {
    // Astra r2's NEW P1: `dev-latest` is a floating tag the DNA job overwrites
    // (dna/Jenkinsfile:1057) BEFORE its later publish steps, so DNA #10 can be
    // the lastSuccessfulBuild at B_P while #11 has already replaced the bytes
    // and then failed. A historical build query cannot prove what the tag
    // names now, so there is no query.
    const mod = await import('./timer-dispatch.mjs');
    assert.equal(mod.lastSuccessfulRevision, undefined);
    const real = defaultDeps(ROOT);
    assert.equal(real.lastSuccessfulRevision, undefined, 'no controller client on defaultDeps');
    assert.equal(real.jenkinsJob, undefined, 'no job/branch resolution needed any more');
    // Scan CODE only — the header comment legitimately explains the removal.
    const code = readFileSync(join(HERE, 'timer-dispatch.mjs'), 'utf8')
      .replace(/\/\*[\s\S]*?\*\//g, '')
      .replace(/(^|\s)\/\/[^\n]*/g, '$1');
    for (const token of ['lastSuccessfulBuild', 'JENKINS_URL', 'fetch(', 'AbortSignal']) {
      assert.ok(!code.includes(token), `${token} must be gone from the module's code`);
    }

    // And a dep object that DOES offer such a hook changes nothing: the
    // producer is still kept even though the stub would have said SUCCESS.
    const deps = stubDeps(graph);
    let asked = false;
    deps.lastSuccessfulRevision = async () => {
      asked = true;
      return { sha1: BUILT, result: 'SUCCESS', buildNumber: 10 };
    };
    const out = await decideDispatch(state, plan, walks, deps);
    assert.equal(asked, false, 'nothing may consult a controller');
    assert.deepEqual(out.skipped, [], 'a SUCCESS at B_P must NOT unlock the skip');
    assert.ok(out.logLines[0].includes('KEPT'));
  });

  test('no surviving consumer → producer skipped', async () => {
    const out = await decideDispatch(state, plan, walks, stubDeps({ dependsOn: {} }));
    assert.deepEqual(out.skipped, ['elohim-holochain']);
    assert.ok(out.logLines[0].includes('nothing surviving needs it'));
  });

  test('survivors are ROOTS: traversal continues through a skipped intermediate', async () => {
    // app survives; edge is skipped; app -> edge -> DNA still keeps DNA,
    // because a skipped edge contributes no fresh artifact binding either.
    const chained = { ...state, pipelines: ['elohim-holochain', 'elohim-edge', 'elohim'] };
    const both = {
      groups: {
        [BUILT]: { changedFiles: ['x'], pipelines: ['elohim-holochain', 'elohim-edge'] },
      },
      provenance: { 'elohim-holochain': 'dispatch-only', 'elohim-edge': 'verdict' },
      notes: {},
    };
    const out = await decideDispatch(chained, both, { [BUILT]: ['elohim'] }, stubDeps({
      dependsOn: { elohim: ['elohim-edge'], 'elohim-edge': ['elohim-holochain'] },
    }));
    assert.deepEqual(out.skipped, ['elohim-edge']);
    assert.deepEqual(out.dispatch, ['elohim-holochain', 'elohim']);
  });

  test('a dependsOn read that throws keeps the producer and never escapes', async () => {
    // An unknown consumer set is not "no consumers".
    const out = await decideDispatch(state, plan, walks, stubDeps({ ...graph, fail: 'dependsOn' }));
    assert.deepEqual(out.skipped, []);
    assert.ok(out.logLines[0].includes('KEPT'));
    assert.ok(out.logLines[0].includes('dependsOn unreadable'));
  });

  test('a verdict-backed producer stays freely skippable', async () => {
    const out = await decideDispatch(
      state,
      { ...plan, provenance: { 'elohim-holochain': 'verdict' } },
      walks,
      stubDeps(graph),
    );
    assert.deepEqual(out.skipped, ['elohim-holochain']);
    assert.ok(out.logLines[0].includes('built green'));
  });

  test('never throws, whatever it is handed', async () => {
    for (const args of [
      [undefined, undefined, undefined],
      [{}, {}, {}],
      [{ pipelines: ['x'] }, { groups: 'nope' }, 'nope'],
      [{ pipelines: ['x'] }, { groups: { [BUILT]: {} } }, { [BUILT]: [] }],
    ]) {
      await decideDispatch(args[0], args[1], args[2], stubDeps());
    }
  });

  test('every decision decideDispatch produces is an exact partition', async () => {
    for (const walked of [walks, { [BUILT]: [] }, {}, { [BUILT]: ['elohim-holochain'] }]) {
      const out = await decideDispatch(state, plan, walked, stubDeps(graph));
      assert.equal(partitionViolation(state.pipelines, out.dispatch, out.skipped), null);
    }
  });
});

// ── The exact-partition predicate ───────────────────────────────────────────

describe('partitionViolation — the guard both decoders enforce', () => {
  const planned = ['DNA', 'edge'];

  test('accepts only an exact partition', () => {
    assert.equal(partitionViolation(planned, ['DNA', 'edge'], []), null);
    assert.equal(partitionViolation(planned, [], ['DNA', 'edge']), null);
    assert.equal(partitionViolation(planned, ['edge'], ['DNA']), null);
    assert.equal(partitionViolation([], [], []), null);
  });

  test("Astra's two probes are both rejected", () => {
    // Empties the wave behind two invented, duplicated names.
    assert.match(
      partitionViolation(planned, [], ['invented', 'invented']),
      /never planned/,
    );
    // Silently loses DNA by naming edge on both sides.
    assert.match(partitionViolation(planned, ['edge'], ['edge']), /more than once/);
  });

  test('invented names in skipped are rejected, not just in dispatch', () => {
    assert.match(partitionViolation(planned, ['DNA', 'edge'], ['ghost']), /skipped names 'ghost'/);
    assert.match(partitionViolation(planned, ['ghost'], []), /dispatch names 'ghost'/);
  });

  test('duplicates within one list are rejected', () => {
    assert.match(partitionViolation(planned, ['DNA', 'DNA'], ['edge']), /more than once/);
    assert.match(partitionViolation(planned, ['DNA'], ['edge', 'edge']), /more than once/);
  });

  test('omission is rejected and names what is missing', () => {
    assert.match(partitionViolation(planned, ['DNA'], []), /omits edge/);
    assert.match(partitionViolation(planned, [], []), /omits DNA, edge/);
  });

  test('non-lists and non-strings are rejected', () => {
    for (const bad of [undefined, null, {}, 'edge', 7]) {
      assert.match(partitionViolation(planned, bad, []), /must both be arrays/);
      assert.match(partitionViolation(planned, [], bad), /must both be arrays/);
    }
    assert.match(partitionViolation(planned, [7], []), /non-string/);
    assert.match(partitionViolation(planned, [], [null]), /non-string/);
  });
});

// ── Against this repository ─────────────────────────────────────────────────

describe('against this repository (real git, real manifests)', () => {
  const resolvable = (sha) => {
    try {
      execFileSync('git', ['-C', ROOT, 'cat-file', '-e', `${sha}^{commit}`], { stdio: 'ignore' });
      return true;
    } catch {
      return false;
    }
  };
  const have = [HEAD, GLOBAL, BUILT].every(resolvable);
  const state = {
    trigger: 'WEBHOOK',
    headSha: HEAD,
    branch: 'dev',
    pipelines: ['elohim-holochain', 'elohim-edge', 'elohim-genesis'],
    baselines: {
      __global__: GLOBAL,
      'elohim-holochain': BUILT,
      'elohim-edge': BUILT,
      'elohim-genesis': GLOBAL,
    },
    forced: [],
  };

  test('#1888: the real narrow diff is emitted with real provenance', { skip: !have }, async () => {
    const out = await planNarrowGroups(state, defaultDeps(ROOT));
    assert.deepEqual(Object.keys(out.groups), [BUILT]);
    assert.deepEqual(out.groups[BUILT].pipelines, ['elohim-holochain', 'elohim-edge']);
    // 86e7c220..7d27b04c is a real, non-empty range that touches nothing under
    // elohim/holochain — the whole point of #1888.
    const files = out.groups[BUILT].changedFiles;
    assert.ok(files.length > 0);
    assert.equal(files.filter((f) => f.startsWith('elohim/holochain')).length, 0);
    assert.ok(files.some((f) => f.startsWith('elohim/elohim-storage/')));
    // elohim/holochain/dna/build-manifest.json really is longRunning: true.
    assert.equal(out.provenance['elohim-holochain'], 'dispatch-only');
    assert.equal(out.provenance['elohim-edge'], 'verdict');
    // genesis shares the global baseline, so it is never a candidate.
    assert.ok(!out.groups[BUILT].pipelines.includes('elohim-genesis'));
  });

  test('a pipeline with a missing manifest cannot be filtered', { skip: !have }, async (t) => {
    // Submodule initialization varies between developer and push checkouts.
    // Exercise real manifest discovery against an explicitly empty fixture.
    const manifestRoot = mkdtempSync(join(tmpdir(), 'dispatch-missing-manifest-'));
    t.after(() => rmSync(manifestRoot, { recursive: true, force: true }));
    const out = await planNarrowGroups(
      {
        ...state,
        pipelines: ['elohim-sophia'],
        baselines: { __global__: GLOBAL, 'elohim-sophia': BUILT },
      },
      { ...defaultDeps(ROOT), provenanceOf: defaultDeps(manifestRoot).provenanceOf },
    );
    assert.deepEqual(out.groups, {});
    assert.match(out.notes['elohim-sophia'], /provenance unknown/);
  });

  test('the whole planning import graph is node_modules-free', async () => {
    // Astra's P1-3: the orchestrator reaches planning on a clean checkout with
    // no install, and this worktree would otherwise resolve picomatch from
    // /projects/elohim/node_modules, OUTSIDE the worktree. Walk the static
    // import graph of every module the CLI loads and assert every specifier is
    // either node:-prefixed or a relative path.
    const seen = new Set();
    const queue = [resolve(HERE, 'timer-dispatch.mjs')];
    const bare = [];
    while (queue.length > 0) {
      const file = queue.shift();
      if (seen.has(file)) continue;
      seen.add(file);
      const src = readFileSync(file, 'utf8');
      for (const m of src.matchAll(/(?:^|[^\w$])(?:import|from)\s*\(?\s*['"]([^'"]+)['"]/g)) {
        const spec = m[1];
        // A builtin resolves with or without the node: prefix and needs no
        // install; manifest-utils.mjs imports bare 'fs'/'path'.
        if (isBuiltin(spec)) continue;
        if (spec.startsWith('./') || spec.startsWith('../')) {
          queue.push(resolve(dirname(file), spec));
          continue;
        }
        bare.push(`${file} -> ${spec}`);
      }
    }
    assert.deepEqual(bare, [], `planning must not need node_modules: ${bare.join(', ')}`);
    assert.ok(seen.size >= 2, 'expected timer-dispatch.mjs plus manifest-utils.mjs');
    // And graph-walker.mjs (the picomatch consumer) must not be on the graph.
    for (const file of seen) assert.ok(!file.endsWith('graph-walker.mjs'), file);
  });

  test('the CLI runs with no ancestor node_modules on the resolution path', () => {
    // Belt to the static check's braces: copy the two planning modules into a
    // bare temp dir (no ancestor node_modules anywhere above it), run the real
    // CLI there, and require real output.
    const dir = mkdtempSync(join(tmpdir(), 'orch-plan-'));
    for (const f of ['timer-dispatch.mjs', 'manifest-utils.mjs']) {
      writeFileSync(join(dir, f), readFileSync(join(HERE, f)));
    }
    writeFileSync(join(dir, 'state.json'), JSON.stringify({ headSha: HEAD, pipelines: [] }));
    const env = { ...process.env };
    delete env.NODE_PATH;
    const stdout = execFileSync(
      process.execPath,
      [join(dir, 'timer-dispatch.mjs'), 'groups', join(dir, 'state.json')],
      { cwd: dir, env, encoding: 'utf8' },
    );
    assert.deepEqual(JSON.parse(stdout), { groups: {}, provenance: {}, notes: {} });
  });
});

// ── The Jenkinsfile's own sh lines ───────────────────────────────────────────

describe('the Jenkinsfile sh lines execute', () => {
  const jf = readFileSync(join(HERE, 'Jenkinsfile'), 'utf8');
  const helper = jf.slice(
    jf.indexOf('def applyAlreadyBuiltFilter('),
    jf.indexOf('\ndef ', jf.indexOf('def applyAlreadyBuiltFilter(') + 1),
  );

  /**
   * Extract the two `sh(script: "...")` command strings from the helper and
   * resolve the Groovy locals they interpolate. This is the check Astra broke:
   * a substring assertion still passed after the executable was changed to
   * `n0de`, because nothing ever ran it. These RUN.
   */
  const commands = () => {
    const locals = {};
    for (const m of helper.matchAll(/String (\w+) = '([^']*)'/g)) locals[m[1]] = m[2];
    const found = [];
    for (const m of helper.matchAll(/sh\(script: "([^"]+)", returnStatus: true\)/g)) {
      found.push(m[1].replace(/\$\{(\w+)\}/g, (_, k) => {
        assert.ok(Object.hasOwn(locals, k), `unresolved Groovy local \${${k}}`);
        return locals[k];
      }));
    }
    return found;
  };

  test('both phases are invoked through one resolvable CLI local', () => {
    const cmds = commands();
    assert.equal(cmds.length, 2, `expected 2 sh(returnStatus) calls, got ${cmds.length}`);
    assert.match(cmds[0], /^node genesis\/orchestrator\/timer-dispatch\.mjs groups /);
    assert.match(cmds[1], /^node genesis\/orchestrator\/timer-dispatch\.mjs decide /);
  });

  test("walkNarrowGroups writes the very file the decide phase reads", () => {
    // Astra r2: a wrong output path INSIDE walkNarrowGroups escapes an
    // execution suite that stubs the walks file. So compare the Jenkinsfile's
    // own wiring end to end, by NAME.
    const call = helper.match(/walkNarrowGroups\((\w+), (\w+)\)/);
    assert.ok(call, 'applyAlreadyBuiltFilter must call walkNarrowGroups(planLocal, walksLocal)');
    const [, planLocal, walksLocal] = call;

    // The decide invocation must read exactly those two locals, in that order,
    // after the state file.
    // The executable is itself a local (${cli}), so match from the verb.
    const decide = helper.match(/ decide \$\{(\w+)\} \$\{(\w+)\} \$\{(\w+)\}/);
    assert.ok(decide, 'the decide sh line must interpolate three file locals');
    assert.equal(decide[2], planLocal, 'decide must read the plan file walkNarrowGroups was given');
    assert.equal(decide[3], walksLocal, 'decide must read the walks file walkNarrowGroups writes');

    // The groups phase must WRITE that same plan file.
    const groups = helper.match(/ groups \$\{(\w+)\} > \$\{(\w+)\}/);
    assert.ok(groups, 'the groups sh line must redirect into a file local');
    assert.equal(groups[2], planLocal, 'groups must write the plan file walkNarrowGroups reads');

    // And inside walkNarrowGroups, the parameters must actually be used that
    // way round: read the plan param, write the walks param.
    const body = jf.slice(
      jf.indexOf('def walkNarrowGroups('),
      jf.indexOf('\ndef ', jf.indexOf('def walkNarrowGroups(') + 1),
    );
    const params = body.match(/def walkNarrowGroups\(String (\w+), String (\w+)\)/);
    assert.ok(params, 'walkNarrowGroups must take two String params');
    assert.ok(
      body.includes(`readJSON(file: ${params[1]})`),
      `walkNarrowGroups must read its first param (${params[1]})`,
    );
    assert.ok(
      body.includes(`writeJSON file: ${params[2]}, json: walks`),
      `walkNarrowGroups must write its second param (${params[2]})`,
    );
    // Nothing else may be written from that def — a stray literal path here is
    // exactly the escape this test exists to close.
    const writes = [...body.matchAll(/writeJSON file: ([^,]+),/g)].map((m) => m[1].trim());
    assert.deepEqual(writes, [params[2], params[2]], 'walkNarrowGroups writes only its walks param');
  });

  test('the extracted commands actually run from the repo root', () => {
    const dir = mkdtempSync(join(tmpdir(), 'orch-sh-'));
    const state = {
      trigger: 'WEBHOOK',
      headSha: HEAD,
      branch: 'dev',
      pipelines: [],
      baselines: { __global__: GLOBAL },
      forced: [],
    };
    const [groupsCmd, decideCmd] = commands();
    // Substitute the workspace-relative state/plan/walks/decision paths for
    // temp-dir ones so the real workspace is never written to.
    const files = {};
    const swap = (cmd) =>
      cmd.replace(/\.orch-already-built-[\w-]+\.json/g, (name) => {
        files[name] ??= join(dir, name);
        return files[name];
      });
    writeFileSync(join(dir, '.orch-already-built-state.json'), JSON.stringify(state));
    files['.orch-already-built-state.json'] = join(dir, '.orch-already-built-state.json');

    execSync(swap(groupsCmd), { cwd: ROOT, encoding: 'utf8' });
    const plan = JSON.parse(readFileSync(files['.orch-already-built-plan.json'], 'utf8'));
    assert.deepEqual(plan.groups, {});

    // walkNarrowGroups is Groovy; stand in for it with the empty walk it would
    // write for an empty group set.
    files['.orch-already-built-walks.json'] = join(dir, '.orch-already-built-walks.json');
    writeFileSync(files['.orch-already-built-walks.json'], '{}');

    execSync(swap(decideCmd), { cwd: ROOT, encoding: 'utf8' });
    const decision = JSON.parse(readFileSync(files['.orch-already-built-decision.json'], 'utf8'));
    assert.deepEqual(decision, { dispatch: [], skipped: [], logLines: [] });
  });
});

describe('the decide CLI refuses to print a non-partition', () => {
  test('a violation prints {"error":…} and exits non-zero, so the rc guard fires', () => {
    // decideDispatch cannot normally produce a bad partition, so provoke one
    // the only way the contract allows: a planned list that repeats a name.
    // The self-check must catch it BEFORE anything reaches the Jenkinsfile.
    const dir = mkdtempSync(join(tmpdir(), 'orch-decide-'));
    const state = join(dir, 'state.json');
    const plan = join(dir, 'plan.json');
    const walks = join(dir, 'walks.json');
    writeFileSync(state, JSON.stringify({ trigger: 'WEBHOOK', headSha: HEAD, pipelines: ['dup', 'dup'] }));
    writeFileSync(plan, JSON.stringify({ groups: {}, provenance: {}, notes: {} }));
    writeFileSync(walks, '{}');

    let status = 0;
    let stdout = '';
    try {
      stdout = execFileSync(
        process.execPath,
        [join(HERE, 'timer-dispatch.mjs'), 'decide', state, plan, walks],
        { encoding: 'utf8' },
      );
    } catch (err) {
      status = err.status;
      stdout = err.stdout;
    }
    assert.notEqual(status, 0, 'a non-partition must exit non-zero');
    const body = JSON.parse(stdout);
    assert.match(body.error, /more than once/);
    assert.equal(body.dispatch, undefined, 'no dispatch list may be offered alongside an error');
  });

  test('a normal decision still prints a clean partition and exits 0', () => {
    const dir = mkdtempSync(join(tmpdir(), 'orch-decide-ok-'));
    const state = join(dir, 'state.json');
    const plan = join(dir, 'plan.json');
    const walks = join(dir, 'walks.json');
    writeFileSync(
      state,
      JSON.stringify({ trigger: 'WEBHOOK', headSha: HEAD, pipelines: ['elohim-epr', 'elohim-edge'] }),
    );
    writeFileSync(
      plan,
      JSON.stringify({
        groups: { [BUILT]: { changedFiles: ['x'], pipelines: ['elohim-epr'] } },
        provenance: { 'elohim-epr': 'verdict' },
        notes: {},
      }),
    );
    writeFileSync(walks, JSON.stringify({ [BUILT]: ['elohim-edge'] }));
    const out = JSON.parse(
      execFileSync(
        process.execPath,
        [join(HERE, 'timer-dispatch.mjs'), 'decide', state, plan, walks],
        { encoding: 'utf8' },
      ),
    );
    assert.deepEqual(out.skipped, ['elohim-epr']);
    assert.deepEqual(out.dispatch, ['elohim-edge']);
    assert.equal(partitionViolation(['elohim-epr', 'elohim-edge'], out.dispatch, out.skipped), null);
  });
});

// ── Static wiring ────────────────────────────────────────────────────────────

describe('Jenkinsfile wiring', () => {
  const jf = readFileSync(join(HERE, 'Jenkinsfile'), 'utf8');
  const bodyOf = (name) => {
    const start = jf.indexOf(`def ${name}(`);
    assert.notEqual(start, -1, `${name} not found`);
    return jf.slice(start, jf.indexOf('\ndef ', start + 1));
  };

  test('the cron trigger is classified TIMER, not MANUAL', () => {
    assert.ok(jf.includes('env.BUILD_TRIGGER = classifyBuildTrigger()'));
    const body = bodyOf('classifyBuildTrigger');
    assert.ok(body.includes('TimerTrigger') && body.includes("return 'TIMER'"));
    assert.ok(body.includes("'WEBHOOK'") && body.includes("'MANUAL'"));
  });

  test('the filter is trigger-agnostic and keeps no matcher of its own', () => {
    const body = bodyOf('applyAlreadyBuiltFilter');
    assert.doesNotMatch(
      body,
      /env\.BUILD_TRIGGER != '/,
      '#1888 was a WEBHOOK — the filter must not be gated on a trigger class',
    );
    assert.match(body, /timer-dispatch\.mjs/);
    assert.match(body, /walkNarrowGroups\(/, 'phase 2 must be the real Groovy walker');
  });

  test('phase 2 calls build-graph.groovy::walkBuildGraph, not a JS matcher', () => {
    const body = bodyOf('walkNarrowGroups');
    assert.match(body, /load\('genesis\/orchestrator\/build-graph\.groovy'\)/);
    assert.match(body, /walkBuildGraph\(files\)/);
    assert.match(body, /res\.pipelineSteps\.keySet\(\)/);
    // And the module must not import the picomatch-bearing walker.
    const mjs = readFileSync(join(HERE, 'timer-dispatch.mjs'), 'utf8');
    assert.doesNotMatch(
      mjs,
      /(?:^|[^\w$])(?:import|from)\s*\(?\s*['"][^'"]*graph-walker\.mjs['"]/,
      'planning must not import graph-walker.mjs (it carries picomatch)',
    );
    assert.doesNotMatch(
      mjs,
      /(?:^|[^\w$])(?:import|from|require)\s*\(?\s*['"]picomatch['"]/,
      'planning must not import picomatch',
    );
  });

  test('phase 2 restores build-state.json from the env bridge', () => {
    // loadBuildState() re-copies the PREVIOUS build's artifact over the file
    // runBuildGraph just saved; without the restore, :2262's rewrite archives
    // the old state (#1844's shape).
    const body = bodyOf('walkNarrowGroups');
    assert.match(body, /finally \{[\s\S]*BUILD_STATE_JSON[\s\S]*writeFile file: 'build-state\.json'/);
  });

  test('the decoder requires an EXACT partition, not just two lists', () => {
    // The .mjs side of this predicate is executable (see partitionViolation);
    // the Groovy decoder is source-asserted, arm by arm, because running it
    // needs a controller.
    const body = bodyOf('decodeAlreadyBuiltDecision');
    assert.match(body, /instanceof List/, 'both list types must be checked');
    assert.match(body, /instanceof CharSequence/, 'every member must be a String');
    assert.match(body, /!graphPipelines\.contains\(name\)/, 'invented names must be rejected');
    assert.match(body, /seen\.contains\(name\)/, 'duplicates and overlap must be rejected');
    assert.match(
      body,
      /seen\.size\(\) != graphPipelines\.size\(\)/,
      'the union must cover the planned set exactly',
    );
    // Both lists must be walked — checking only `dispatch` is what let
    // {"dispatch":[],"skipped":["invented","invented"]} through.
    assert.match(body, /for \(int pass = 0; pass < 2; pass\+\+\)/, 'skipped must be walked too');
    assert.match(body, /skippedRaw/);
    assert.match(body, /dispatch: null/, 'an untrusted decision must return null, not []');
    assert.doesNotMatch(body, /out\.dispatch \?: \[\]/, '`?: []` would silently empty the plan');
    const filter = bodyOf('applyAlreadyBuiltFilter');
    assert.match(filter, /if \(dispatch == null\) return graphPipelines/);
  });

  test('the module self-checks the same partition and exits non-zero on violation', () => {
    const mjs = readFileSync(join(HERE, 'timer-dispatch.mjs'), 'utf8');
    assert.match(mjs, /partitionViolation\(planned, out\.dispatch, out\.skipped\)/);
    assert.match(mjs, /JSON\.stringify\(\{ error: violation \}\)/);
    assert.match(mjs, /process\.exit\(3\)/, 'a violation must fail the rc guard too');
  });

  test('the state and out-file decodes are inside the fallback guard', () => {
    const body = bodyOf('applyAlreadyBuiltFilter');
    const tryIdx = body.indexOf('try {');
    const catchIdx = body.indexOf('} catch (Exception e) {');
    assert.ok(tryIdx > 0 && catchIdx > tryIdx);
    const guarded = body.slice(tryIdx, catchIdx);
    assert.match(guarded, /readJSON\(text: env\.PIPELINE_BASELINES/, 'state decode must be guarded');
    assert.match(guarded, /decodeAlreadyBuiltDecision\(/, 'out-file decode must be guarded');
    assert.match(guarded, /writeJSON file: stateFile/);
  });

  test('Jenkins interruption is re-thrown, never swallowed', () => {
    const body = bodyOf('applyAlreadyBuiltFilter');
    assert.match(body, /catch \(Exception e\)/, 'catch Exception, never Throwable');
    assert.doesNotMatch(body, /catch \(Throwable/);
    assert.match(body, /FlowInterrupted/);
    assert.match(body, /InterruptedException/);
    assert.match(body, /throw e/);
  });

  // Every helper here runs unattended on the next webhook, so none may depend
  // on a sandbox construct this Jenkins has not already executed.
  const CODE_ONLY = (body) =>
    body
      .split('\n')
      .filter((l) => !l.trimStart().startsWith('*') && !l.trimStart().startsWith('//'))
      .join('\n');
  const BANNED_ON_SANDBOX_LISTS = [
    '.collect', '.join(\'\\n\')', '.any', '.find{', '.find {',
    '.findAll', '.each', '.inject', '.grep', '@NonCPS', 'Class.forName',
  ];

  for (const helper of [
    'classifyBuildTrigger',
    'walkNarrowGroups',
    'decodeAlreadyBuiltDecision',
    'applyAlreadyBuiltFilter',
  ]) {
    test(`${helper} uses no closure or collection method on a sandbox list`, () => {
      const code = CODE_ONLY(bodyOf(helper));
      for (const banned of BANNED_ON_SANDBOX_LISTS) {
        assert.ok(!code.includes(banned), `${helper} must not use ${banned}`);
      }
      assert.ok(
        !/->/.test(code) && !/\{\s*it\b/.test(code),
        `${helper} must not declare a closure`,
      );
      assert.match(code, /for \(int i = 0; i </, `${helper} must iterate with a plain indexed loop`);
    });
  }

  test('every touched def stays under the 8000-byte comment-stripped ceiling', () => {
    for (const helper of ['walkNarrowGroups', 'decodeAlreadyBuiltDecision', 'applyAlreadyBuiltFilter']) {
      const size = CODE_ONLY(bodyOf(helper)).length;
      assert.ok(size < 8000, `${helper} is ${size}B, over DEF_HARD 8000`);
    }
  });

  test('classifyBuildTrigger keeps the shortDescription fallback', () => {
    const body = bodyOf('classifyBuildTrigger');
    assert.ok(body.includes("contains('Started by timer')"));
    assert.ok(body.includes('cause?.shortDescription'));
    assert.ok(body.includes('cause?._class'));
  });

  test('the filter runs before the genesis auto-include', () => {
    const routing = jf.slice(jf.indexOf('def applyBuildGraphRouting('));
    const filterIdx = routing.indexOf('applyAlreadyBuiltFilter(');
    const genesisIdx = routing.indexOf('Genesis auto-include');
    assert.ok(filterIdx > 0);
    assert.ok(filterIdx < genesisIdx);
  });

  test('[deploy-only] stays webhook-gated (a TIMER is not a WEBHOOK)', () => {
    const idx = jf.indexOf("DEPLOY_ONLY_FROM_TAG = 'true'");
    assert.ok(idx > 0);
    assert.match(jf.slice(idx - 400, idx), /if \(env\.BUILD_TRIGGER == 'WEBHOOK'\) \{/);
  });
});
