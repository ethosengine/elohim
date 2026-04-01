import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { walkGraph, topoSort } from './graph-walker.mjs';
import { resolveStep } from './manifest-utils.mjs';

// ── Helpers ──────────────────────────────────────────────────────

function makeManifest(pipeline, steps, gate) {
  return {
    path: `${pipeline}/build-manifest.json`,
    content: {
      manifestVersion: '1.0',
      pipeline,
      description: `Test manifest for ${pipeline}`,
      steps,
      ...(gate ? { gate } : {}),
    },
  };
}

function makeStep(sources = [], depends = [], buildProcess = []) {
  return {
    description: 'test step',
    inputs: { sources, buildProcess },
    outputs: { artifacts: ['test-artifact'], verify: null },
    depends,
    executor: { stage: 'Test', function: null },
  };
}

// ── resolveStep ──────────────────────────────────────────────────

describe('resolveStep', () => {
  it('qualifies bare step names', () => {
    assert.equal(resolveStep('build-angular', 'elohim'), 'elohim:build-angular');
  });

  it('passes through already-qualified names', () => {
    assert.equal(resolveStep('elohim-sophia:build-sophia-umd', 'elohim'), 'elohim-sophia:build-sophia-umd');
  });
});

// ── Source glob matching ─────────────────────────────────────────

describe('source glob matching', () => {
  it('matches a file against a glob pattern', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/src/**']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'my-app');
    assert.ok(result.projects[0].reasons.some(r => r.startsWith('source:')));
  });

  it('does not match unrelated files', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/src/**']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['other/file.txt']);
    assert.equal(result.projects.length, 0);
  });

  it('matches tsconfig glob patterns', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/tsconfig*.json']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/tsconfig.app.json']);
    assert.equal(result.projects.length, 1);
  });
});

// ── BuildProcess file matching ───────────────────────────────────

describe('buildProcess matching', () => {
  it('matches whole-file references', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep([], [], ['Jenkinsfile']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['Jenkinsfile']);
    assert.equal(result.projects.length, 1);
    assert.ok(result.projects[0].reasons.some(r => r.includes('buildProcess: Jenkinsfile')));
  });

  it('matches @function references by file', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep([], [], ['Jenkinsfile@buildAngularApp']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['Jenkinsfile']);
    assert.equal(result.projects.length, 1);
    assert.ok(result.projects[0].reasons.some(r => r.includes('buildProcess: Jenkinsfile@buildAngularApp')));
  });

  it('does not match when referenced file is not changed', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep([], [], ['Jenkinsfile']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['src/main.ts']);
    assert.equal(result.projects.length, 0);
  });
});

// ── Dependency propagation ───────────────────────────────────────

describe('dependency propagation', () => {
  it('marks dependent steps stale when dependency is stale', () => {
    const manifests = [
      makeManifest('lib', {
        build: makeStep(['lib/src/**']),
      }, { projects: { lib: { dir: 'lib' } } }),
      makeManifest('app', {
        build: makeStep([], ['lib:build']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['lib/src/index.ts']);
    assert.equal(result.projects.length, 2);
    const appProject = result.projects.find(p => p.name === 'app');
    assert.ok(appProject);
    assert.ok(appProject.reasons.some(r => r.includes('depends:')));
  });

  it('propagates staleness transitively (A -> B -> C)', () => {
    const manifests = [
      makeManifest('a', {
        build: makeStep(['a/**']),
      }, { projects: { a: { dir: 'a' } } }),
      makeManifest('b', {
        build: makeStep([], ['a:build']),
      }, { projects: { b: { dir: 'b' } } }),
      makeManifest('c', {
        build: makeStep([], ['b:build']),
      }, { projects: { c: { dir: 'c' } } }),
    ];
    const result = walkGraph(manifests, ['a/file.rs']);
    assert.equal(result.projects.length, 3);
    assert.ok(result.projects.find(p => p.name === 'c'));
  });

  it('propagates within same manifest (bare dep names)', () => {
    const manifests = [
      makeManifest('app', {
        compile: makeStep(['app/src/**']),
        bundle: makeStep([], ['compile']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'app');
  });
});

// ── Gate mapping ─────────────────────────────────────────────────

describe('gate mapping', () => {
  it('maps steps to specific gate projects via steps field', () => {
    const manifests = [
      makeManifest('edge', {
        'build-doorway': makeStep(['doorway/**']),
        'build-storage': makeStep(['storage/**']),
      }, {
        projects: {
          doorway: { dir: 'doorway/service', steps: ['build-doorway'] },
          storage: { dir: 'elohim/storage', steps: ['build-storage'] },
        },
      }),
    ];
    const result = walkGraph(manifests, ['doorway/src/main.rs']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'doorway');
    assert.equal(result.projects[0].dir, 'doorway/service');
  });

  it('triggers all gate projects when steps is omitted', () => {
    const manifests = [
      makeManifest('app', {
        build: makeStep(['app/**']),
      }, { projects: { 'my-app': { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'my-app');
  });

  it('returns empty when no gate field exists', () => {
    const manifests = [
      makeManifest('app', { build: makeStep(['app/**']) }),
    ];
    const result = walkGraph(manifests, ['app/src/main.ts']);
    assert.equal(result.projects.length, 0);
  });

  it('returns empty when no manifests provided', () => {
    const result = walkGraph([], ['app/src/main.ts']);
    assert.equal(result.projects.length, 0);
  });
});

// ── Topological output ordering ──────────────────────────────────

describe('output ordering', () => {
  it('orders dependencies before dependents', () => {
    const manifests = [
      makeManifest('sophia', {
        build: makeStep(['sophia/**']),
      }, { projects: { sophia: { dir: 'sophia' } } }),
      makeManifest('app', {
        build: makeStep(['app/**'], ['sophia:build']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    const result = walkGraph(manifests, ['sophia/src/x.ts', 'app/src/y.ts']);
    assert.equal(result.projects.length, 2);
    assert.equal(result.projects[0].name, 'sophia');
    assert.equal(result.projects[1].name, 'app');
  });

  it('stable order for independent projects', () => {
    const manifests = [
      makeManifest('aaa', {
        build: makeStep(['aaa/**']),
      }, { projects: { aaa: { dir: 'aaa' } } }),
      makeManifest('bbb', {
        build: makeStep(['bbb/**']),
      }, { projects: { bbb: { dir: 'bbb' } } }),
    ];
    const result = walkGraph(manifests, ['aaa/x.ts', 'bbb/y.ts']);
    assert.equal(result.projects.length, 2);
  });
});
