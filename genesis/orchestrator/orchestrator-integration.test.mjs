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
