import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { execFileSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
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

// ── No dependency propagation (source-only detection) ────────────

describe('no dependency propagation', () => {
  it('does NOT cascade staleness through cross-manifest dependencies', () => {
    const manifests = [
      makeManifest('lib', {
        build: makeStep(['lib/src/**']),
      }, { projects: { lib: { dir: 'lib' } } }),
      makeManifest('app', {
        build: makeStep([], ['lib:build']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    // Only lib sources changed — app should NOT be triggered
    const result = walkGraph(manifests, ['lib/src/index.ts']);
    assert.equal(result.projects.length, 1);
    assert.equal(result.projects[0].name, 'lib');
  });

  it('detects both projects when both have direct source matches', () => {
    const manifests = [
      makeManifest('lib', {
        build: makeStep(['lib/src/**']),
      }, { projects: { lib: { dir: 'lib' } } }),
      makeManifest('app', {
        build: makeStep(['app/src/**'], ['lib:build']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    // Both have source changes — both trigger independently
    const result = walkGraph(manifests, ['lib/src/index.ts', 'app/src/main.ts']);
    assert.equal(result.projects.length, 2);
  });

  it('gate fires when any watched step has direct source match', () => {
    const manifests = [
      makeManifest('app', {
        compile: makeStep(['app/src/**']),
        bundle: makeStep([], ['compile']),
      }, { projects: { app: { dir: 'app' } } }),
    ];
    // compile is directly stale, bundle is not — but gate watches both
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

// ── CLI: .ci-ignore filter ───────────────────────────────────────
// The CLI is what .husky/pre-push pipes $CHANGED into. It must drop
// files listed in repo-root .ci-ignore (basenames like CLAUDE.md,
// subtrees like .claude/, .husky/, .github/) BEFORE matching against
// any manifest glob. A push that only touches CI-ignored paths must
// emit zero projects, matching the Jenkinsfile orchestrator's behavior.

describe('CLI .ci-ignore filtering', () => {
  const CLI = resolve(dirname(fileURLToPath(import.meta.url)), 'graph-walker.mjs');

  function runCli(stdinLines) {
    const out = execFileSync('node', [CLI], {
      input: stdinLines.join('\n'),
      encoding: 'utf8',
    });
    return JSON.parse(out);
  }

  it('emits zero projects when only .claude/ files change', () => {
    const result = runCli(['.claude/skills/foo/SKILL.md', '.claude/settings.json']);
    assert.equal(result.projects.length, 0);
  });

  it('emits zero projects when only CLAUDE.md / AGENTS.md change', () => {
    const result = runCli(['CLAUDE.md', 'app/elohim-app/AGENTS.md', 'sophia/CLAUDE.md']);
    assert.equal(result.projects.length, 0);
  });

  it('emits zero projects when only .husky/ files change', () => {
    const result = runCli(['.husky/pre-push']);
    assert.equal(result.projects.length, 0);
  });

  it('still detects real source changes mixed with ignored ones', () => {
    const result = runCli([
      'CLAUDE.md',                                   // ignored
      '.claude/skills/x.md',                          // ignored
      'app/elohim-app/src/app/app.component.ts',     // real source — should match a manifest
    ]);
    // We don't pin which project — just that the ignored files didn't
    // suppress the real one.
    assert.ok(result.projects.length >= 1,
      `expected at least one project for the real source change; got ${JSON.stringify(result.projects)}`);
  });
});
