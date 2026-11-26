import { test, describe } from 'node:test';
import { strict as assert } from 'node:assert';
import { resolve, dirname, join } from 'path';
import { mkdtempSync, mkdirSync, readFileSync, writeFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { discoverManifests } from './manifest-utils.mjs';
import { existsSync } from 'node:fs';
import { posix } from 'node:path';
import { stripFunctionSuffix } from './gate-inputs.mjs';
import { fileURLToPath } from 'url';
import {
  loadPipelineRegistry,
  nonManualPipelines,
  dispatchablePipelines,
  pipelinesThatTriggerGenesis,
  pipelineDependencyMap,
  loadGateRegistry,
} from './pipeline-registry.mjs';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(__dirname, '../..');

/**
 * Every buildProcess entry of a step a gate borrows that breaks the executor boundary: one that is
 * neither a pipeline file nor a `scripts/ci/` build script, or one the justfile its gate runs from
 * names (root `justfile` for `root-just`; `<dir>/justfile` for `just`, the root one when that is
 * absent, as gate-evidence.mjs recipeAt and run-local-gate.sh resolve it), by its repository path or
 * its path from that justfile's directory. The whole justfile's text is read, not the recipe's
 * closure (gate-recipe-key.mjs): the closure needs `just`, which the orchestrator suite's view does
 * not run, and the whole text is the stricter reading. Full-line comments (first non-blank
 * character `#`, shebang lines `#!` kept) are not read: a comment may mention a pipeline file. What
 * a program the recipe starts reads is not checked here.
 */
function buildProcessOffenders(root) {
  const pipelineFile = file => file.split('/').pop().includes('Jenkinsfile') || file.includes('.github/workflows/');
  const ciScript = file => file.startsWith('scripts/ci/');
  const offenders = [];
  for (const project of loadGateRegistry(root).values()) {
    const manifest = JSON.parse(readFileSync(resolve(root, project.manifestPath), 'utf8'));
    let justfile = null;
    if (project.run?.kind === 'root-just') justfile = 'justfile';
    else if (project.run?.kind === 'just') justfile = existsSync(resolve(root, project.dir, 'justfile')) ? posix.join(project.dir, 'justfile') : 'justfile';
    const text = justfile && existsSync(resolve(root, justfile))
      ? readFileSync(resolve(root, justfile), 'utf8').split('\n').filter(line => !/^\s*#(?!!)/.test(line)).join('\n')
      : null;
    for (const step of project.steps) {
      for (const entry of manifest.steps[step].inputs?.buildProcess ?? []) {
        const file = stripFunctionSuffix(entry);
        if (!pipelineFile(file) && !ciScript(file)) {
          offenders.push(`${project.manifestPath} ${step}: ${entry} is neither a pipeline file nor a scripts/ci/ build script`);
        }
        const names = [...new Set([file, posix.relative(posix.dirname(justfile || 'justfile'), file)])];
        const named = text === null ? null : names.find(name => text.includes(name));
        if (named) offenders.push(`${project.manifestPath} ${step}: ${entry} is named in ${justfile} (${named}), the file gate ${project.name} runs from`);
      }
    }
  }
  return offenders.sort();
}

describe('pipeline-registry', () => {
  const registry = loadPipelineRegistry(ROOT);

  // A step has two input classes. `sources` is what any executor of the step reads. `buildProcess`
  // names the files that define ANOTHER executor's process for it: a pipeline file, a CI build
  // script. A device gate that keyed a borrowed step on its sources class alone
  // (`brit build … --inputs sources`; gate-inputs.mjs's shadow compares it, and no gate keys on it
  // yet) would drop them, which is sound only while no gate reads one.
  // On 2026-11-21 two gate-only TypeScript steps listed their lockfile, workspace file and codegen
  // script as buildProcess; an independent review caught that a lockfile bump would then have reused
  // a stale green. An input the gate's own recipe reads is a source. A gate project's OWN `inputs`
  // block is the gate's, both classes, and is not checked here: nothing is ever dropped from it.
  test('a borrowed step lists only another executor\'s process files as buildProcess, and its gate\'s justfile names none', () => {
    assert.deepEqual(buildProcessOffenders(ROOT), [],
      'a borrowed step breaks the executor boundary: it lists a buildProcess entry that is not a pipeline file or a scripts/ci/ build script, '
      + 'or that a command in the justfile its gate runs from names. If the gate\'s recipe reads the file, list it under sources; '
      + 'if it does not, do not name it in a command');
  });

  test('loads every build-manifest.json with a pipeline field', () => {
    assert.ok(registry.size >= 8, `expected ≥8 pipelines, got ${registry.size}`);
    for (const known of ['elohim', 'elohim-edge', 'elohim-holochain', 'elohim-genesis', 'elohim-sophia']) {
      assert.ok(registry.has(known), `missing pipeline: ${known}`);
    }
  });

  test('nonManualPipelines excludes manualOnly entries', () => {
    const names = nonManualPipelines(registry);
    assert.ok(!names.includes('elohim-steward'), 'elohim-steward should be excluded');
    assert.ok(names.includes('elohim'), 'elohim should be included');
  });

  test('dispatchablePipelines returns only entries with jenkinsPath', () => {
    const names = dispatchablePipelines(registry);
    assert.ok(!names.includes('elohim-doorway-app'), 'graph-only pipeline should be excluded');
    assert.ok(!names.includes('elohim-compute'), 'graph-only pipeline should be excluded');
    assert.ok(names.includes('elohim'), 'elohim has jenkinsPath');
  });

  test('pipelinesThatTriggerGenesis returns the marked set', () => {
    const names = pipelinesThatTriggerGenesis(registry);
    assert.deepStrictEqual(
      [...names].sort(),
      ['elohim', 'elohim-edge', 'elohim-holochain'].sort()
    );
  });

  test('pipelineDependencyMap produces a name → deps map', () => {
    const deps = pipelineDependencyMap(registry);
    // Edge waits on the DNA (hApp artifact) and on the custom conductor image —
    // the storage image bakes the conductor, so it must build after it.
    assert.deepStrictEqual(deps.get('elohim-edge'), ['elohim-holochain', 'elohim-conductor']);
    assert.deepStrictEqual(deps.get('elohim-genesis'), ['elohim-edge', 'elohim']);
    assert.deepStrictEqual(deps.get('elohim-sophia'), []);
  });

  test('longRunning marked correctly', () => {
    assert.strictEqual(registry.get('elohim-holochain').longRunning, true);
    assert.strictEqual(registry.get('elohim-steward').longRunning, true);
    assert.strictEqual(registry.get('elohim-storybook').longRunning, true);
    assert.strictEqual(registry.get('elohim').longRunning, false);
    assert.strictEqual(registry.get('elohim-edge').longRunning, false);
  });

  test('deploymentCheck built from deployment.targets', () => {
    const edge = registry.get('elohim-edge');
    assert.ok(edge.deploymentCheck, 'elohim-edge should have deploymentCheck');
    assert.ok(edge.deploymentCheck.alpha, 'should have alpha endpoint');
    const sophia = registry.get('elohim-sophia');
    assert.strictEqual(sophia.deploymentCheck, null, 'sophia has no deployment');
  });

  test('gate registry preserves manifest-local execution contracts', () => {
    const gates = loadGateRegistry(ROOT);
    assert.ok(gates.size >= 23, `expected ≥23 gates, got ${gates.size}`);
    assert.strictEqual(gates.get('doorway').dir, 'doorway/doorway-service');
    assert.strictEqual(gates.get('doorway').run.kind, 'just');
    assert.deepStrictEqual(gates.get('agentic-tools').run, {
      kind: 'root-just', recipe: '_gate-agentic-tools',
    });
  });
});

test('private proof checkouts cannot introduce duplicate gates; submodules still participate', () => {
  const root = mkdtempSync(join(tmpdir(), 'manifest-discovery-'));
  try {
    for (const [path, name] of [
      ['app/client', 'client'],
      ['genesis/local-dev/proof/repo/app/client', 'client'],
      ['elohim/independent', 'independent'],
    ]) {
      const dir = join(root, path);
      mkdirSync(dir, { recursive: true });
      writeFileSync(join(dir, 'build-manifest.json'), JSON.stringify({
        steps: {},
        gate: { projects: { [name]: { dir: path, steps: [], run: { kind: 'just', recipe: 'check' } } } },
      }));
    }
    writeFileSync(join(root, 'elohim/independent/.git'), 'gitdir: ../../.git/modules/independent\n');
    assert.deepEqual(discoverManifests(root), [
      './app/client/build-manifest.json', './elohim/independent/build-manifest.json',
    ]);
    assert.deepEqual([...loadGateRegistry(root).keys()].sort(), ['client', 'independent']);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test('the executor-boundary check fails and names the entry: a lockfile put back, or a script a command in the gate\'s justfile runs; a comment that names one passes', () => {
  const root = mkdtempSync(join(tmpdir(), 'build-process-boundary-'));
  try {
    mkdirSync(join(root, 'p'), { recursive: true });
    const step = buildProcess => ({ inputs: { sources: ['p/**'], buildProcess }, depends: [] });
    writeFileSync(join(root, 'p/build-manifest.json'), JSON.stringify({
      pipeline: 'fixture',
      steps: {
        lock: step(['pnpm-lock.yaml']),
        image: step(['scripts/ci/image.sh@build', 'p/Jenkinsfile']),
        quiet: step(['scripts/ci/quiet.sh']),
      },
      gate: { projects: {
        a: { dir: 'p', steps: ['lock'], run: { kind: 'root-just', recipe: '_gate-a' } },
        b: { dir: 'p', steps: ['image', 'quiet'], run: { kind: 'just', recipe: 'gate' } },
        owned: { dir: 'p', inputs: { sources: ['p/**'], buildProcess: ['justfile', 'pnpm-lock.yaml'] }, run: { kind: 'root-just', recipe: '_gate-a' } },
      } },
    }));
    writeFileSync(join(root, 'justfile'), '_gate-a:\n    pnpm install --frozen-lockfile\n');
    // A full-line comment may mention an entry (quiet.sh); a command that names one (image.sh) fails.
    writeFileSync(join(root, 'p/justfile'), '#!/usr/bin/env just --justfile\n# CI builds the image with ../scripts/ci/quiet.sh and p/Jenkinsfile\ngate:\n    # not ../scripts/ci/quiet.sh here\n    bash ../scripts/ci/image.sh\n');
    assert.deepEqual(buildProcessOffenders(root), [
      './p/build-manifest.json image: scripts/ci/image.sh@build is named in p/justfile (scripts/ci/image.sh), the file gate b runs from',
      './p/build-manifest.json lock: pnpm-lock.yaml is neither a pipeline file nor a scripts/ci/ build script',
    ]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
