/**
 * Orchestrator Integration Tests
 *
 * Covers the code that survived the orchestrator-strategy.mjs deletion:
 *   - parseCiIgnore / matchesCiIgnore / CI_IGNORE_PATTERNS (ci-ignore.mjs)
 *   - pipeline-list.json drift against pipeline-registry.mjs
 *
 * Deleted tests (covered by sibling test files):
 *   - changeset routing       → graph-walker.test.mjs
 *   - cascade propagation     → jenkinsfile-cps-scope.test.mjs
 *   - commit message tags     → commit-tag-parser.test.mjs
 *   - dependency ordering     → graph-walker.test.mjs (topoSort)
 *   - real-world scenarios    → graph-walker.test.mjs + manifest tests
 *   - mirror vs Jenkinsfile   → no JS mirror exists anymore
 *   - nonManualPipelines      → pipeline-registry.test.mjs
 *
 * Run: node --test orchestrator-integration.test.mjs
 */

import { test, describe } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

import {
  parseCiIgnore,
  matchesCiIgnore,
  CI_IGNORE_PATTERNS,
} from './ci-ignore.mjs';
import { loadGateRegistry, loadPipelineRegistry } from './pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');

// ══════════════════════════════════════════════════════════════════
// .ci-ignore parser / matcher
// ══════════════════════════════════════════════════════════════════

describe('parseCiIgnore', () => {
  test('classifies trailing-slash patterns as prefix', () => {
    const p = parseCiIgnore('.claude/\n.github/\n');
    assert.deepEqual(p, [
      { kind: 'prefix', value: '.claude/' },
      { kind: 'prefix', value: '.github/' },
    ]);
  });

  test('classifies path patterns (containing /) as exact', () => {
    const p = parseCiIgnore('genesis/orchestrator/Jenkinsfile\n');
    assert.deepEqual(p, [
      { kind: 'exact', value: 'genesis/orchestrator/Jenkinsfile' },
    ]);
  });

  test('classifies bare names as basename-anywhere', () => {
    const p = parseCiIgnore('CLAUDE.md\nAGENTS.md\n');
    assert.deepEqual(p, [
      { kind: 'basename', value: 'CLAUDE.md' },
      { kind: 'basename', value: 'AGENTS.md' },
    ]);
  });

  test('strips comments and blank lines', () => {
    const p = parseCiIgnore('# header\n\n.claude/  # trailing\n\nCLAUDE.md\n');
    assert.deepEqual(p, [
      { kind: 'prefix', value: '.claude/' },
      { kind: 'basename', value: 'CLAUDE.md' },
    ]);
  });
});

describe('matchesCiIgnore', () => {
  const patterns = [
    { kind: 'prefix', value: '.claude/' },
    { kind: 'exact', value: 'genesis/orchestrator/Jenkinsfile' },
    { kind: 'basename', value: 'CLAUDE.md' },
  ];

  test('prefix matches files inside the subtree', () => {
    assert.equal(matchesCiIgnore('.claude/memory/CLAUDE.md', patterns), true);
    assert.equal(matchesCiIgnore('.claude/agents/foo.md', patterns), true);
  });

  test('exact matches only the precise path', () => {
    assert.equal(matchesCiIgnore('genesis/orchestrator/Jenkinsfile', patterns), true);
    // A different Jenkinsfile (owned by another pipeline) must not be skipped.
    assert.equal(matchesCiIgnore('elohim/holochain/dna/Jenkinsfile', patterns), false);
  });

  test('basename matches anywhere in the tree', () => {
    assert.equal(matchesCiIgnore('CLAUDE.md', patterns), true);
    assert.equal(matchesCiIgnore('app/elohim-app/CLAUDE.md', patterns), true);
    assert.equal(matchesCiIgnore('app/elohim-app/src/app/elohim/adapters/CLAUDE.md', patterns), true);
  });

  test('returns false for non-matching files', () => {
    assert.equal(matchesCiIgnore('app/elohim-app/src/main.ts', patterns), false);
    assert.equal(matchesCiIgnore('CLAUDE.txt', patterns), false);
    assert.equal(matchesCiIgnore('not-claude.md', patterns), false);
  });
});

describe('CI_IGNORE_PATTERNS (loaded from repo-root .ci-ignore)', () => {
  test('includes the agent-instruction basenames', () => {
    const basenames = CI_IGNORE_PATTERNS
      .filter(p => p.kind === 'basename')
      .map(p => p.value);
    assert.ok(basenames.includes('CLAUDE.md'), '.ci-ignore must list CLAUDE.md');
    assert.ok(basenames.includes('AGENTS.md'), '.ci-ignore must list AGENTS.md');
    assert.ok(basenames.includes('GEMINI.md'), '.ci-ignore must list GEMINI.md');
  });

  test('includes the .claude/ subtree', () => {
    const prefixes = CI_IGNORE_PATTERNS
      .filter(p => p.kind === 'prefix')
      .map(p => p.value);
    assert.ok(prefixes.includes('.claude/'), '.ci-ignore must list .claude/');
  });
});

// ══════════════════════════════════════════════════════════════════
// Jenkinsfile dead-helper guard
// ══════════════════════════════════════════════════════════════════

test('dead change-detection helpers are retired from the Jenkinsfile', () => {
  const jf = readFileSync(new URL('./Jenkinsfile', import.meta.url), 'utf8');
  for (const dead of ['loadCiIgnore', 'matchesCiIgnore', 'propagateDependencies']) {
    assert.equal(jf.includes(dead), false, `${dead} must be fully removed from the Jenkinsfile`);
  }
  assert.ok(jf.includes('def analyzeChangeset'), 'analyzeChangeset must remain (it is live)');
  assert.equal(/DEPRECATED: advisory only, will be removed/.test(jf), false,
    'the inverted DEPRECATED tag on analyzeChangeset must be removed');
});

// ══════════════════════════════════════════════════════════════════
// Selected dependency completion barrier
// ══════════════════════════════════════════════════════════════════

describe('long-running dependency completion barrier', () => {
  const jenkinsfile = readFileSync(
    resolve(__dirname, 'Jenkinsfile'),
    'utf8'
  );
  const helper = jenkinsfile.slice(
    jenkinsfile.indexOf('def needsDetachedDependencyBarrier('),
    jenkinsfile.indexOf('/**\n * Trigger a pipeline.')
  );
  const trigger = jenkinsfile.slice(
    jenkinsfile.indexOf('def triggerPipeline('),
    jenkinsfile.indexOf('def autoModeAnalyze()')
  );

  test('only a selected long-running producer with a selected dependent gets a barrier', () => {
    assert.match(helper, /producerConfig\.longRunning == true/);
    assert.match(helper, /selectedPipelines\.any/);
    assert.match(helper, /dependencies\.contains\(producer\)/);
    assert.match(
      jenkinsfile,
      /needsDetachedDependencyBarrier\(name, pipelines\)/,
      'the selected execution plan must decide the barrier at both dispatch call sites'
    );
    assert.equal(
      [...jenkinsfile.matchAll(/needsDetachedDependencyBarrier\(name, pipelines\)/g)].length,
      2,
      'single and parallel dependency levels must both enforce the barrier'
    );
  });

  test('barrier starts detached and parent abort cannot cancel the producer', () => {
    assert.match(helper, /waitForStart:\s*true/);
    assert.match(helper, /started\?\.externalizableId/);
    assert.match(helper, /runId:\s*started\.externalizableId/);
    assert.match(helper, /propagateAbort:\s*false/);
    assert.equal(
      [...trigger.matchAll(/awaitDetachedBuild\(jobName, jobBranch,/g)].length,
      2,
      'monorepo and cross-repository dispatches must share the safe wait path'
    );
    assert.match(trigger, /dispatchResult\(result, shouldWait \|\| detachedBarrier\)/);
  });

  test('standalone long-running pipelines retain the existing asynchronous default', () => {
    assert.match(trigger, /dependencyBarrier && config\.longRunning == true/);
    assert.match(trigger, /wait:\s*shouldWait/);
    assert.match(trigger, /fire-and-forget \(longRunning\)/);
  });

  test('app dependency orders coupled delivery without selecting edge for app-only work', () => {
    const registry = loadPipelineRegistry(ROOT);
    const appDependencies = registry.get('elohim').dependsOn;
    const edgeDependencies = registry.get('elohim-edge').dependsOn;
    assert.deepEqual(appDependencies, ['elohim-sophia', 'elohim-edge']);
    assert.ok(edgeDependencies.includes('elohim-holochain'));
    assert.ok(!edgeDependencies.includes('elohim'));
    assert.match(
      jenkinsfile,
      /deps\.every \{ dep -> !nonGenesis\.contains\(dep\) \|\| placed\.contains\(dep\) \}/,
      'an absent dependency must not be selected by pipeline ordering'
    );
    assert.match(jenkinsfile, /if \(hasGenesis\) levelDesc \+= ' → genesis'/);
  });
});

// ══════════════════════════════════════════════════════════════════
// Barrier pair: edge starts beside DNA; the hApp candidate and the DNA
// verdict are barriers INSIDE edge (one-head-delivered Lane C2)
// ══════════════════════════════════════════════════════════════════

describe('DNA → edge barrier pair', () => {
  const orchestrator = readFileSync(resolve(__dirname, 'Jenkinsfile'), 'utf8');
  const edge = readFileSync(resolve(ROOT, 'elohim/holochain/Jenkinsfile'), 'utf8');
  const dna = readFileSync(resolve(ROOT, 'elohim/holochain/dna/Jenkinsfile'), 'utf8');
  const fn = (text, name) => {
    const at = text.indexOf(`def ${name}(`);
    assert.ok(at >= 0, `def ${name}( must exist`);
    return text.slice(at, text.indexOf('\n}\n', at) + 2);
  };
  const stage = (text, name) => {
    const at = text.indexOf(`stage('${name}')`);
    assert.ok(at >= 0, `stage '${name}' must exist`);
    return text.slice(at, text.indexOf("\n        stage('", at + 1));
  };

  test('the pair is declared once, in the orchestrator, and dependsOn still names it', () => {
    assert.match(orchestrator, /^@Field Map BARRIER_PAIRS = \['elohim-holochain': 'elohim-edge'\]$/m);
    assert.ok(loadPipelineRegistry(ROOT).get('elohim-edge').dependsOn.includes('elohim-holochain'),
      'edge keeps its declared dependency — the barrier changes WHERE it is enforced, not WHETHER');
  });

  test('leveling puts the pair in one level: the consumer ignores its producer, the producer waits for its consumer', () => {
    const level = fn(orchestrator, 'groupByDependencyLevel');
    assert.match(level, /dependsOn \?: \[\]\) - \[barrierProducerOf\(name\)\]/);
    assert.match(level, /ready\.contains\(consumer\)/);
    assert.match(level, /if \(currentLevel\.isEmpty\(\)\) currentLevel = ready/,
      'a producer whose consumer can never be placed still runs');
  });

  test('the parallel level hands the producer run ID and candidate tag to the consumer', () => {
    assert.match(orchestrator, /def barrierRuns = barrierRunsIn\(runnable\)/);
    assert.match(orchestrator, /triggerPipeline\(name, env\.BRANCH_NAME, null, changedFiles, dependencyBarrier, barrierRuns\)/);
    const params = fn(orchestrator, 'barrierParams');
    assert.match(params, /stringParam\(name: 'DNA_RUN_ID', value: runId\)/);
    assert.match(params, /stringParam\(name: 'HAPP_CANDIDATE_TAG', value: tag\)/);
    assert.match(params, /"candidate-\$\{\(env\.GIT_COMMIT_FULL \?: ''\)\.take\(8\)\}"/,
      'the tag is candidate-<commit8>, the DNA pipeline\'s own GIT_COMMIT_HASH shape');
    assert.match(fn(orchestrator, 'awaitDetachedBuild'), /barrierRuns\[name\] = started\.externalizableId/);
  });

  test('every parallel branch runs through triggerInLevel, whose finally always fills a producer slot (M1)', () => {
    assert.match(orchestrator, /def result = triggerInLevel\(name, pipelines, changedFiles, barrierRuns\)/);
    const level = fn(orchestrator, 'triggerInLevel');
    assert.ok(level.indexOf('try {') < level.indexOf('barrierProducerOf(name)'),
      'the whole branch body is inside the try — a throw before dispatch still releases the consumer');
    assert.match(level, /\} finally \{\n\s*if \(barrierRuns\?\.containsKey\(name\) && barrierRuns\[name\] == null\) barrierRuns\[name\] = BARRIER_FAILED_START/);
    assert.match(level, /waitUntil\(quiet: true\) \{ barrierRuns\[producer\] != null \|\| System\.currentTimeMillis\(\) > deadline \}/,
      'the consumer wait is bounded');
    assert.match(orchestrator, /^@Field long BARRIER_START_WAIT_MS = 60L \* 60 \* 1000$/m);
  });

  test('I1: a producer that failed to start SKIPS its consumer; only a missing job runs it standalone', () => {
    const level = fn(orchestrator, 'triggerInLevel');
    assert.match(level,
      /if \(!runId && barrierRuns\[producer\] != BARRIER_NOT_PROVISIONED\) \{[\s\S]*?return \[success: false, result: 'SKIPPED-BY-UPSTREAM-FAILURE', upstream: producer\]/);
    assert.ok(level.indexOf("'SKIPPED-BY-UPSTREAM-FAILURE'") < level.indexOf('triggerPipeline('),
      'the skip returns before any dispatch');
    const trigger = fn(orchestrator, 'triggerPipeline');
    const noItem = trigger.slice(trigger.indexOf("contains('No item named')"));
    assert.match(noItem.slice(0, 300), /barrierRuns\[name\] = BARRIER_NOT_PROVISIONED/,
      'only the "No item named" branch marks NOT_PROVISIONED');
    const runIdOf = fn(orchestrator, 'barrierRunId');
    assert.match(runIdOf, /v != BARRIER_FAILED_START && v != BARRIER_NOT_PROVISIONED/, 'markers are never a run ID');
    assert.match(fn(orchestrator, 'barrierParams'), /if \(!runId\) return \[\]/);
  });

  test('I1/N2: a skip is excluded from levelFailed only when its upstream failed here', () => {
    const doom = fn(orchestrator, 'doomDependents');
    assert.match(doom, /def failedHere = ran\.findAll \{ !results\[it\]\?\.success \}/);
    const cond = doom.match(/def levelFailed = failedHere\.findAll \{ (.+) \}\n/)[1];
    // Evaluate the exact Groovy condition (safe-navigation mapped to JS).
    const counted = new Function('it', 'results', 'failedHere',
      `return ${cond.replace(/\?\./g, '?.').replace(/failedHere\.contains\(/g, 'failedHere.includes(')}`);
    const results = {
      dnaRed: { success: false, result: 'FAILURE' },
      edgeSkip: { success: false, result: 'SKIPPED-BY-UPSTREAM-FAILURE', upstream: 'dnaRed' },
      dnaGreen: { success: true, result: 'SUCCESS' },
      edgeOrphan: { success: false, result: 'SKIPPED-BY-UPSTREAM-FAILURE', upstream: 'dnaGreen' },
      edgeLate: { success: false, result: 'SKIPPED-PRODUCER-NOT-STARTED', upstream: 'dnaGreen' },
    };
    const failedHere = ['dnaRed', 'edgeSkip', 'edgeOrphan', 'edgeLate'];
    assert.deepEqual(failedHere.filter((it) => counted(it, results, failedHere)), ['dnaRed', 'edgeOrphan', 'edgeLate'],
      'the producer failure counts once; a skip whose producer did not fail here counts, and so does a wait timeout');
  });

  test('N2: a producer-start timeout is its own counted result, never SKIPPED-BY-UPSTREAM-FAILURE', () => {
    const level = fn(orchestrator, 'triggerInLevel');
    assert.match(level,
      /if \(barrierRuns\[producer\] == null\) \{[\s\S]*?return \[success: false, result: 'SKIPPED-PRODUCER-NOT-STARTED', upstream: producer\]/);
    assert.ok(level.indexOf("'SKIPPED-PRODUCER-NOT-STARTED'") < level.indexOf("'SKIPPED-BY-UPSTREAM-FAILURE'"),
      'the timeout is classified before the failed-start marker');
  });

  test('I2: the consumer baseline is held whenever the producer verdict is not SUCCESS (UNSTABLE included)', () => {
    const doom = fn(orchestrator, 'doomDependents');
    assert.ok(doom.indexOf('holdBarrierConsumers(ran, results, pipelineBaselines)') < doom.indexOf('if (!levelFailed) return []'),
      'the hold runs even when nothing failed — an UNSTABLE producer counts as success');
    const hold = fn(orchestrator, 'holdBarrierConsumers');
    assert.match(hold, /if \(!c\.deployRefused && results\[producer\]\?\.result == 'SUCCESS'\) return/);
    assert.match(hold, /!c\?\.barrierRun/, 'a standalone consumer deployed — never held');
    assert.match(hold, /pipelineBaselines\[consumer\] = before\[consumer\]/);
    assert.match(fn(orchestrator, 'triggerInLevel'), /if \(runId\) result\.barrierRun = runId/);
  });

  test('edge declares both params and gates the Alpha deploy on the DNA verdict (Review Focus 4)', () => {
    assert.match(edge, /string\(name: 'DNA_RUN_ID', defaultValue: ''/);
    assert.match(edge, /string\(name: 'HAPP_CANDIDATE_TAG', defaultValue: ''/);
    const verdict = fn(edge, 'awaitDnaVerdict');
    assert.match(verdict, /if \(!params\.DNA_RUN_ID\) \{ return true \}/);
    assert.match(verdict, /waitForBuild\(runId: params\.DNA_RUN_ID, propagate: false, propagateAbort: false\)/);
    assert.match(verdict, /if \(dna\.result != 'SUCCESS'\) \{ unstable\([^)]*\); return false \}/);
    assert.ok(verdict.indexOf("env.DEPLOY_REFUSED = 'true'") < verdict.indexOf("if (dna.result != 'SUCCESS')") &&
      verdict.indexOf("env.DEPLOY_REFUSED = 'false'") > verdict.indexOf('edge-deploy-budget.sh'),
      'refused until both the verdict and the budget pass');
    assert.match(stage(edge, 'Deploy Edge Node - Alpha'),
      /script \{\n\s*if \(!awaitDnaVerdict\(\)\) \{ return \}\n/,
      'the verdict is the FIRST statement of the deploy body; a refusal skips the whole deploy');
  });

  test('N1: a deploy refused after a SUCCESS verdict (budget) is published and held', () => {
    assert.match(fn(orchestrator, 'dispatchResult'),
      /deployRefused: \(result\.description \?: ''\)\.toString\(\)\.startsWith\('DEPLOY-REFUSED:'\)/);
    const describe = fn(edge, 'edgeRunDescription');
    assert.match(describe, /env\.DEPLOY_REFUSED == 'true' \? "DEPLOY-REFUSED: \$\{env\.DEPLOY_REFUSED_REASON\}/);
    assert.match(edge, /^\s*currentBuild\.description = edgeRunDescription\(\)$/m,
      'the post-always summary keeps the refusal prefix instead of overwriting it');
    assert.doesNotMatch(edge, /currentBuild\.description = \[/, 'no second description writer drops the prefix');
    const verdict = fn(edge, 'awaitDnaVerdict');
    assert.ok(verdict.indexOf('currentBuild.description = edgeRunDescription()') < verdict.indexOf('if (budget != 0)'),
      'the budget refusal is published before it returns');
    // The hold decision, evaluated: budget refusal after DNA SUCCESS is held;
    // an ordinary UNSTABLE edge beside a green DNA is not.
    const cond = fn(orchestrator, 'holdBarrierConsumers').match(/if \((!c\.deployRefused && results\[producer\]\?\.result == 'SUCCESS')\) return/)[1];
    const skipsHold = new Function('c', 'results', 'producer', `return ${cond}`);
    const green = { dna: { result: 'SUCCESS' } };
    assert.equal(skipsHold({ deployRefused: true }, green, 'dna'), false, 'budget refusal → held');
    assert.equal(skipsHold({ deployRefused: false }, green, 'dna'), true, 'ordinary UNSTABLE beside green DNA → delivered');
    assert.equal(skipsHold({ deployRefused: false }, { dna: { result: 'UNSTABLE' } }, 'dna'), false, 'UNSTABLE DNA → held');
  });

  test('M5: the verdict gate refuses every DNA result except SUCCESS', () => {
    // Groovy cannot run here; the refusal condition is a plain string compare,
    // so evaluate the exact expression from the Jenkinsfile over every result.
    const cond = fn(edge, 'awaitDnaVerdict').match(/if \((dna\.result != 'SUCCESS')\) \{ unstable/)[1];
    const refuses = new Function('dna', `return ${cond}`);
    for (const result of ['FAILURE', 'UNSTABLE', 'ABORTED', 'NOT_BUILT', null, undefined]) {
      assert.equal(refuses({ result }), true, `DNA ${result} must refuse the deploy`);
    }
    assert.equal(refuses({ result: 'SUCCESS' }), false);
  });

  test('I3: the deploy starts only with budget left; M2: validation skips after a refusal', () => {
    assert.match(fn(edge, 'awaitDnaVerdict'),
      /edge-deploy-budget\.sh' \$\{currentBuild\.startTimeInMillis\} 240 115", returnStatus: true\)/);
    assert.match(fn(edge, 'awaitDnaVerdict'), /if \(env\.DNA_VERDICT\) \{ return env\.DEPLOY_REFUSED != 'true' \}/);
    assert.match(stage(edge, 'Dataplane Validation'), /expression \{ env\.DEPLOY_REFUSED != 'true' \}/);
    for (const name of ['Deploy Edge Node - Staging', 'Deploy Edge Node - Prod']) {
      assert.match(stage(edge, name), /script \{\n\s*if \(!awaitDnaVerdict\(\)\) \{ return \}\n/, `${name} gates on the verdict`);
    }
  });

  test('edge packages the candidate, bounded, and falls back loudly', () => {
    const resolveTag = fn(edge, 'resolveHappSourceTag');
    assert.match(resolveTag, /await-happ-candidate\.sh' '\$\{candidate\}' 2400"/);
    assert.match(resolveTag, /if \(rc == 0\) \{\n\s*tag = candidate/);
    assert.match(resolveTag, /unstable\(/);
    assert.match(resolveTag, /FALLBACK/);
    assert.match(resolveTag, /env\.HAPP_SOURCE_TAG = tag/, 'one resolution per run: both images carry the same bytes');
    for (const name of ['Build Edge Node Image', 'Build hApp Installer']) {
      assert.match(stage(edge, name), /fetchEdgeHapp\('/, `${name} fetches through the candidate barrier`);
    }
  });

  test('DNA publishes the candidate inside Build DNA, after the stash, before any test stage', () => {
    const build = stage(dna, 'Build DNA');
    assert.ok(build.indexOf("stash name: 'happ-bundle'") < build.indexOf('pushHappCandidate(props.GIT_COMMIT_HASH'),
      'the candidate is pushed after the happ is packed and stashed');
    assert.match(fn(dna, 'pushHappCandidate'), /push-happ-candidate\.sh/);
    assert.doesNotMatch(fn(dna, 'pushHappCandidate'), /dev-latest|floatingTag/);
  });
});

// ══════════════════════════════════════════════════════════════════
// pre-push guard: no references to deleted orchestrator-strategy module
// ══════════════════════════════════════════════════════════════════

test('pre-push uses the manifest gate registry and no deleted strategy module', () => {
  // .husky/pre-push is a POSIX shim that execs pre-push.bash — the gates live there.
  const hook = readFileSync(new URL('../../.husky/pre-push.bash', import.meta.url), 'utf8');
  assert.equal(hook.includes('orchestrator-strategy'), false,
    'pre-push must not reference the deleted orchestrator-strategy.mjs/.test.mjs');
  assert.ok(hook.includes('gate-runner.mjs --changed-file-list'),
    'pre-push must select projects through the shared manifest gate runner');

  const pipelineList = loadGateRegistry(ROOT).get('pipeline-list-fresh');
  assert.ok(pipelineList, 'pipeline-list-fresh must remain manifest-declared');
  assert.deepEqual(pipelineList.inputs.sources, ['**/build-manifest.json']);
  assert.ok(pipelineList.inputs.buildProcess.includes('genesis/orchestrator/pipeline-registry.mjs'));
});

// ══════════════════════════════════════════════════════════════════
// guard: no dangling orchestrator-strategy references in runtime files
// ══════════════════════════════════════════════════════════════════

test('runtime orchestrator files carry no dangling orchestrator-strategy references', () => {
  const files = ['ci-ignore.mjs', 'justfile', 'scripts/count-pipeline-failures.sh', 'scripts/pipeline-trajectory.mjs'];
  for (const f of files) {
    const txt = readFileSync(new URL(`./${f}`, import.meta.url), 'utf8');
    assert.equal(/orchestrator-strategy/.test(txt), false, `${f} must not reference the deleted orchestrator-strategy module`);
  }
});

// ══════════════════════════════════════════════════════════════════
// pipeline-list.json drift
// ══════════════════════════════════════════════════════════════════

describe('pipeline-list.json drift', () => {
  test('pipeline-list.json matches what generate-pipeline-list.mjs would produce', () => {
    const registry = loadPipelineRegistry(ROOT);
    const expected = [...registry.values()]
      .filter(p => p.jenkinsPath)
      .map(p => ({
        name: p.pipeline,
        manualOnly: p.manualOnly,
        triggersGenesis: p.triggersGenesis,
        cascades: p.cascades,
      }));
    const actual = JSON.parse(readFileSync(
      resolve(__dirname, 'pipeline-list.json'), 'utf8'
    )).pipelines;
    assert.deepStrictEqual(
      actual.sort((a, b) => a.name.localeCompare(b.name)),
      expected.sort((a, b) => a.name.localeCompare(b.name)),
      'pipeline-list.json is stale — run node scripts/generate-pipeline-list.mjs'
    );
  });
});

// ══════════════════════════════════════════════════════════════════
// ABORTED is waste, not failure — Jenkinsfile ↔ pipeline-results.mjs
//
// pipeline-results.mjs is the declared single source of truth:
//   TERMINAL_FAILURE_RESULTS = {FAILURE};  WASTED_RESULTS = {ABORTED}
// The orchestrator Jenkinsfile is a Groovy consumer that cannot import
// it, so these tests hold the two in agreement statically.
//
// Regression: orchestrator/dev #1845 went UNSTABLE with "Genesis failed
// - seeding or tests may have issues" because an operator pressed Stop
// on elohim-genesis/dev #1574. `propagate: false` suppresses downstream
// RESULT propagation but NOT interruption, so the abort arrived as a
// FlowInterruptedException and triggerPipeline's catch flattened it to
// ERROR alongside genuine failures. That manufactured a red build and a
// CI-findings fingerprint (9b7f3c58a51a) out of a deliberate human stop.
// ══════════════════════════════════════════════════════════════════

describe('ABORTED classification (Jenkinsfile honours pipeline-results.mjs)', () => {
  const jenkinsfile = readFileSync(
    resolve(__dirname, 'Jenkinsfile'), 'utf8'
  );

  test('triggerPipeline classifies an interruption as ABORTED, not ERROR', () => {
    assert.ok(
      /FlowInterruptedException/.test(jenkinsfile),
      'triggerPipeline must recognise FlowInterruptedException — otherwise a ' +
      'downstream abort is flattened into the generic ERROR branch'
    );
    const catchBlock = jenkinsfile.slice(
      jenkinsfile.indexOf('} catch (Exception e) {'),
      jenkinsfile.indexOf('def autoModeAnalyze()')
    );
    assert.ok(
      catchBlock.includes('FlowInterruptedException'),
      'the interruption check must live inside triggerPipeline\'s catch block'
    );
    assert.ok(
      catchBlock.indexOf('FlowInterruptedException') <
        catchBlock.indexOf("result: 'ERROR'"),
      'the ABORTED branch must precede the generic ERROR fallthrough'
    );
    assert.ok(
      /result: 'ABORTED'/.test(catchBlock),
      'the interruption branch must report ABORTED so downstream reporting can ' +
      'tell waste from a verdict'
    );
  });

  test('dispatchResult carries a wasted flag keyed on ABORTED', () => {
    assert.ok(
      /wasted: result\.result == 'ABORTED'/.test(jenkinsfile),
      'dispatchResult must flag ABORTED as wasted for the non-throwing path ' +
      '(a downstream that ends ABORTED without interrupting the parent)'
    );
  });

  test('a wasted downstream never marks the orchestrator UNSTABLE', () => {
    // The genesis handler is the only downstream report that calls unstable().
    const genesisIdx = jenkinsfile.indexOf(
      "unstable('Genesis failed - seeding or tests may have issues')"
    );
    assert.ok(genesisIdx > 0, 'genesis unstable() call site not found');
    // Walk back to the start of the if/else chain that guards it.
    const chain = jenkinsfile.slice(
      jenkinsfile.lastIndexOf('if (genesisResult.success)', genesisIdx),
      genesisIdx
    );
    assert.ok(
      /else if \(genesisResult\.wasted\)/.test(chain),
      'the genesis result chain must short-circuit on wasted BEFORE reaching ' +
      'unstable() — an abort is not evidence that seeding or tests failed'
    );
  });

  test('recordPipelineResult reports waste distinctly from failure', () => {
    const fn = jenkinsfile.slice(
      jenkinsfile.indexOf('def recordPipelineResult('),
      jenkinsfile.indexOf('def parseBuildTagTokens(')
    );
    assert.ok(
      /else if \(result\.wasted\)/.test(fn),
      'recordPipelineResult must branch on wasted so the generic dispatch path ' +
      'matches the genesis path'
    );
  });

  test('the summary fail count excludes waste', () => {
    assert.ok(
      /def failCount = results\.count \{ k, v -> !v\?\.success && !v\?\.wasted \}/
        .test(jenkinsfile),
      'failCount must exclude wasted builds — otherwise the summary reports ' +
      '"ATTENTION: Build failures detected!" for a deliberate stop'
    );
    assert.ok(
      /def wastedCount = results\.count \{ k, v -> v\?\.wasted \}/.test(jenkinsfile),
      'waste needs its own count so supersede-thrash and operator-aborts stay visible'
    );
  });
});

// ══════════════════════════════════════════════════════════════════
// Failed dispatches still publish a current actual build graph
//
// Regression: orchestrator/dev #1859-#1868 threw from Execute Builds
// after a downstream failure. Declarative skipped Post Actual Build
// Graph, while post.always trusted a same-named file in the persistent
// controller workspace. Build #1868 claimed both graphs were archived,
// but actual-build-graph.json returned 404 and its summary reported zero
// failed downstreams.
// ══════════════════════════════════════════════════════════════════

describe('failed dispatch build-graph publication', () => {
  const jenkinsfile = readFileSync(resolve(__dirname, 'Jenkinsfile'), 'utf8');
  const executeStage = jenkinsfile.slice(
    jenkinsfile.indexOf("stage('Execute Builds')"),
    jenkinsfile.indexOf("stage('Post Actual Build Graph')"),
  );
  const backstop = jenkinsfile.slice(
    jenkinsfile.indexOf('def runReconciliationBackstop()'),
    jenkinsfile.indexOf('def maintainBuildStateContinuity()'),
  );

  test('Execute Builds records current results and graph before fail-fast throws', () => {
    assert.match(
      executeStage,
      /catchError\(buildResult: 'FAILURE', stageResult: 'FAILURE'\)/,
      'the genuine failure must stay red while later observational stages continue',
    );
    const throwAt = executeStage.indexOf('error "Build(s) failed:');
    assert.ok(throwAt > 0, 'fail-fast throw not found');
    const beforeThrow = executeStage.slice(0, throwAt);
    assert.ok(
      beforeThrow.lastIndexOf('env.BUILD_RESULTS = writeJSON') >
        beforeThrow.lastIndexOf('def levelFailed ='),
      'BUILD_RESULTS must be checkpointed in the failure branch before error()',
    );
    assert.ok(
      beforeThrow.lastIndexOf('writeJSON(file: env.ACTUAL_BUILD_GRAPH_FILE') >
        beforeThrow.lastIndexOf('def levelFailed ='),
      'actual-build-graph.json must be written in the failure branch before error()',
    );
  });

  test('backstop rejects stale controller graph files', () => {
    assert.match(
      backstop,
      /predicted\.buildNumber[\s\S]*env\.BUILD_NUMBER/,
      'the copied predicted graph must be tied to the current Jenkins build',
    );
    assert.match(
      backstop,
      /env\.ACTUAL_BUILD_GRAPH_POSTED != 'true'/,
      'actual graph recovery must use a current-build stage receipt',
    );
    assert.equal(
      backstop.includes('if (!fileExists(actualFile))'),
      false,
      'a static controller-workspace filename is not evidence of a current artifact',
    );
    assert.match(
      backstop,
      /env\.BUILD_GRAPH_RECONCILED != 'true'/,
      'reconciliation recovery must use a current-build stage receipt',
    );
  });
});
