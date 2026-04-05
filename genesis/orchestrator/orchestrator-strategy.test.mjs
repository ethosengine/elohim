/**
 * Orchestrator Strategy Tests
 *
 * Mirrors the pure functions from genesis/orchestrator/Jenkinsfile
 * to validate changeset routing, dependency propagation, cascade
 * behavior, commit message tags, and topological ordering — all
 * without needing Jenkins.
 *
 * Run: node --test orchestrator-strategy.test.mjs
 */

import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';

// ══════════════════════════════════════════════════════════════════
// PIPELINES config — extracted from Jenkinsfile (keep in sync!)
// ══════════════════════════════════════════════════════════════════

const PIPELINES = {
  'elohim-holochain': {
    changePatterns: ['elohim/holochain/dna/', 'elohim/elohim-cache-core/', 'elohim/holochain/rna/', 'VERSION'],
    dependsOn: [],
    cascades: true,
    manualOnly: false,
    triggersGenesis: true,
  },
  'elohim-edge': {
    changePatterns: ['doorway/doorway-service/', 'doorway/doorway-app/', 'elohim/elohim-agent/elohim-agent-sdk/', 'elohim/holochain/edgenode/', 'elohim/elohim-storage/', 'elohim/elohim-cache-core/', 'crates/', 'VERSION'],
    dependsOn: ['elohim-holochain'],
    cascades: undefined,
    manualOnly: false,
    triggersGenesis: true,
  },
  'elohim': {
    changePatterns: ['app/elohim-app/', 'app/elohim-library/', 'elohim/sdk/', 'VERSION'],
    dependsOn: ['elohim-sophia'],
    cascades: undefined,
    manualOnly: false,
    triggersGenesis: true,
  },
  'elohim-genesis': {
    changePatterns: ['genesis/', 'data/'],
    dependsOn: ['elohim-edge', 'elohim'],
    cascades: undefined,
    manualOnly: false,
    triggersGenesis: false,
  },
  'elohim-steward': {
    changePatterns: ['steward/'],
    dependsOn: ['elohim-holochain'],
    cascades: undefined,
    manualOnly: true,
    triggersGenesis: false,
  },
  'elohim-sophia': {
    changePatterns: ['sophia/'],
    dependsOn: [],
    cascades: true,
    manualOnly: false,
    triggersGenesis: false,
  },
};

// ══════════════════════════════════════════════════════════════════
// Pure functions — ported from Jenkinsfile Groovy
// ══════════════════════════════════════════════════════════════════

const CI_ONLY_PATTERNS = ['genesis/orchestrator/', '.claude', '.github/', '.husky/'];
const CI_ONLY_FILES = ['CLAUDE.md', 'ROADMAP.md'];

/**
 * Mirrors Jenkinsfile analyzePipelineRequirements() — changeset analysis.
 * Simplified: no per-pipeline baselines (not needed for strategy tests).
 */
function analyzePipelineRequirements(changedFiles) {
  const analysis = {};

  for (const [name, config] of Object.entries(PIPELINES)) {
    const matchedPatterns = [];
    const matchedFiles = [];

    for (const file of changedFiles) {
      // Skip CI-only files
      if (file.endsWith('Jenkinsfile') ||
          CI_ONLY_PATTERNS.some(p => file.startsWith(p)) ||
          CI_ONLY_FILES.includes(file)) {
        continue;
      }

      for (const pattern of config.changePatterns) {
        // Match files inside directory OR bare submodule pointer
        if (file.startsWith(pattern) || file === pattern.replace(/\/$/, '')) {
          if (!matchedPatterns.includes(pattern)) {
            matchedPatterns.push(pattern);
          }
          matchedFiles.push(file);
        }
      }
    }

    analysis[name] = {
      shouldRun: !config.manualOnly && matchedPatterns.length > 0,
      manualOnly: config.manualOnly || false,
      matchedPatterns,
      matchedFileCount: matchedFiles.length,
      sampleFiles: matchedFiles.slice(0, 5),
      dependsOn: config.dependsOn || [],
      triggersGenesis: config.triggersGenesis || false,
    };
  }

  return analysis;
}

/**
 * Mirrors Jenkinsfile propagateDependencies() — cascade logic.
 */
function propagateDependencies(pipelines, analysis) {
  const result = new Set(pipelines);
  let added = true;

  while (added) {
    added = false;
    for (const [name, config] of Object.entries(PIPELINES)) {
      if (result.has(name)) continue;
      if (config.manualOnly) continue;

      const deps = config.dependsOn || [];
      const buildingDep = deps.find(dep => {
        if (!result.has(dep)) return false;
        const depConfig = PIPELINES[dep];
        return depConfig.cascades == null ? true : depConfig.cascades;
      });

      if (buildingDep) {
        result.add(name);
        analysis[name].shouldRun = true;
        analysis[name].matchedPatterns.push(`dependency:${buildingDep}`);
        added = true;
      }
    }
  }

  return [...result];
}

/**
 * Mirrors Jenkinsfile orderByDependencies().
 */
function orderByDependencies(pipelineList) {
  const ordered = [];
  const remaining = [...pipelineList];

  while (remaining.length > 0) {
    const readyIdx = remaining.findIndex(name => {
      const deps = PIPELINES[name]?.dependsOn || [];
      return deps.every(dep => !remaining.includes(dep) || ordered.includes(dep));
    });

    if (readyIdx >= 0) {
      ordered.push(remaining.splice(readyIdx, 1)[0]);
    } else {
      ordered.push(...remaining);
      break;
    }
  }

  return ordered;
}

/**
 * Mirrors Jenkinsfile commit message tag parsing.
 */
function parseCommitTags(commitMsg) {
  const buildTagAliases = {
    edge: 'elohim-edge',
    dna: 'elohim-holochain',
    app: 'elohim',
    genesis: 'elohim-genesis',
    sophia: 'elohim-sophia',
    steward: 'elohim-steward',
  };

  const buildTags = [];
  const tagRegex = /\[build:([a-z,-]+)\]/gi;
  let match;

  while ((match = tagRegex.exec(commitMsg)) !== null) {
    for (const tag of match[1].split(',')) {
      const t = tag.trim().toLowerCase();
      if (t === 'all') {
        buildTags.push(
          ...Object.keys(PIPELINES).filter(k => !PIPELINES[k].manualOnly)
        );
      } else if (buildTagAliases[t]) {
        buildTags.push(buildTagAliases[t]);
      }
    }
  }

  return [...new Set(buildTags)];
}

function parseSkipCi(commitMsg) {
  return /\[(skip ci|ci skip|no ci)\]/i.test(commitMsg);
}

/**
 * Full orchestrator simulation: changeset + commit tags + propagation + ordering.
 */
function simulate({ changedFiles = [], commitMsg = '' } = {}) {
  if (parseSkipCi(commitMsg)) {
    return { pipelines: [], analysis: {}, skipped: true };
  }

  const analysis = analyzePipelineRequirements(changedFiles);

  // Collect pipelines from changeset
  let pipelines = Object.entries(analysis)
    .filter(([, info]) => info.shouldRun)
    .map(([name]) => name);

  // Inject commit tag overrides
  const forcedTags = parseCommitTags(commitMsg);
  for (const name of forcedTags) {
    if (!pipelines.includes(name) && PIPELINES[name]) {
      pipelines.push(name);
      analysis[name].shouldRun = true;
      analysis[name].matchedPatterns.push('commit-tag:[build:*]');
    }
  }

  // Propagate dependencies
  pipelines = propagateDependencies(pipelines, analysis);

  // Order by dependencies
  const ordered = orderByDependencies(pipelines);

  return { pipelines: ordered, analysis, skipped: false };
}

// ══════════════════════════════════════════════════════════════════
// TESTS
// ══════════════════════════════════════════════════════════════════

// ── Changeset Routing ───────────────────────────────────────────

describe('changeset routing', () => {
  it('DNA changes trigger elohim-holochain', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
  });

  it('doorway changes trigger elohim-edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['doorway/doorway-service/src/main.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
  });

  it('Angular app changes trigger elohim', () => {
    const { pipelines } = simulate({
      changedFiles: ['app/elohim-app/src/app/lamad/services/content.service.ts'],
    });
    assert.ok(pipelines.includes('elohim'));
  });

  it('sophia submodule pointer triggers elohim-sophia', () => {
    const { pipelines } = simulate({
      changedFiles: ['sophia'],
    });
    assert.ok(pipelines.includes('elohim-sophia'));
  });

  it('sophia directory changes trigger elohim-sophia', () => {
    const { pipelines } = simulate({
      changedFiles: ['sophia/packages/sophia-core/src/index.ts'],
    });
    assert.ok(pipelines.includes('elohim-sophia'));
  });

  it('genesis changes trigger elohim-genesis', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/a2o/features/auth/auth-lifecycle.feature'],
    });
    assert.ok(pipelines.includes('elohim-genesis'));
  });

  it('storage changes trigger elohim-edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/elohim-storage/src/main.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
  });

  it('VERSION triggers holochain, edge, and app', () => {
    const { pipelines } = simulate({
      changedFiles: ['VERSION'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim'));
  });

  it('Jenkinsfile-only changes trigger nothing', () => {
    const { pipelines } = simulate({
      changedFiles: ['Jenkinsfile', 'elohim/holochain/dna/Jenkinsfile'],
    });
    assert.equal(pipelines.length, 0);
  });

  it('orchestrator config changes trigger nothing', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/Jenkinsfile', 'CLAUDE.md'],
    });
    assert.equal(pipelines.length, 0);
  });

  it('steward changes do NOT trigger (manualOnly)', () => {
    const { pipelines } = simulate({
      changedFiles: ['steward/src-tauri/src/lib.rs'],
    });
    assert.ok(!pipelines.includes('elohim-steward'));
  });

  it('unrelated files trigger nothing', () => {
    const { pipelines } = simulate({
      changedFiles: ['docs/README.md', '.gitignore'],
    });
    assert.equal(pipelines.length, 0);
  });
});

// ── Cascade / Dependency Propagation ────────────────────────────

describe('cascade propagation', () => {
  it('DNA (holochain) cascades to edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'), 'should include holochain');
    assert.ok(pipelines.includes('elohim-edge'), 'should cascade to edge');
  });

  it('DNA cascades transitively: holochain → edge → genesis', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
  });

  it('sophia cascades to app (elohim)', () => {
    const { pipelines } = simulate({
      changedFiles: ['sophia/packages/sophia-core/src/index.ts'],
    });
    assert.ok(pipelines.includes('elohim-sophia'), 'should include sophia');
    assert.ok(pipelines.includes('elohim'), 'should cascade to app');
  });

  it('steward is never auto-triggered even when dependency builds', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/lamad/integrity/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(!pipelines.includes('elohim-steward'), 'steward should not cascade (manualOnly)');
  });

  it('edge change cascades to genesis but NOT holochain (upstream)', () => {
    const { pipelines } = simulate({
      changedFiles: ['doorway/doorway-service/src/routes/auth_routes.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis should cascade from edge');
    assert.ok(!pipelines.includes('elohim-holochain'), 'holochain should NOT cascade (its upstream)');
  });

  it('app + edge both trigger → genesis included once', () => {
    const { pipelines } = simulate({
      changedFiles: [
        'doorway/doorway-service/src/main.rs',
        'app/elohim-app/src/app/lamad/services/content.service.ts',
      ],
    });
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim'));
    assert.ok(pipelines.includes('elohim-genesis'));
    // No duplicates
    assert.equal(pipelines.filter(p => p === 'elohim-genesis').length, 1);
  });
});

// ── Commit Message Tags ─────────────────────────────────────────

describe('commit message tags', () => {
  it('[build:edge] forces elohim-edge', () => {
    const tags = parseCommitTags('fix: something [build:edge]');
    assert.deepEqual(tags, ['elohim-edge']);
  });

  it('[build:dna] forces elohim-holochain', () => {
    const tags = parseCommitTags('feat: new zome [build:dna]');
    assert.deepEqual(tags, ['elohim-holochain']);
  });

  it('[build:all] forces all non-manual pipelines', () => {
    const tags = parseCommitTags('chore: rebuild [build:all]');
    assert.ok(tags.includes('elohim-holochain'));
    assert.ok(tags.includes('elohim-edge'));
    assert.ok(tags.includes('elohim'));
    assert.ok(tags.includes('elohim-genesis'));
    assert.ok(tags.includes('elohim-sophia'));
    assert.ok(!tags.includes('elohim-steward'), 'steward is manualOnly');
  });

  it('[build:edge,genesis] forces multiple', () => {
    const tags = parseCommitTags('fix: deploy [build:edge,genesis]');
    assert.ok(tags.includes('elohim-edge'));
    assert.ok(tags.includes('elohim-genesis'));
    assert.equal(tags.length, 2);
  });

  it('case insensitive', () => {
    const tags = parseCommitTags('[Build:Edge]');
    assert.deepEqual(tags, ['elohim-edge']);
  });

  it('multiple [build:] tags in one message', () => {
    const tags = parseCommitTags('fix: stuff [build:dna] and also [build:app]');
    assert.ok(tags.includes('elohim-holochain'));
    assert.ok(tags.includes('elohim'));
  });

  it('[build:edge] + cascade propagation', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/Jenkinsfile'],
      commitMsg: 'fix(ci): cascade change [build:edge]',
    });
    assert.ok(pipelines.includes('elohim-edge'), 'edge forced by tag');
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis cascades from edge');
  });

  it('[build:dna] triggers full chain: dna → edge → genesis', () => {
    const { pipelines } = simulate({
      changedFiles: [],
      commitMsg: 'chore: force rebuild [build:dna]',
    });
    assert.ok(pipelines.includes('elohim-holochain'));
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
  });

  it('[skip ci] suppresses everything', () => {
    const { pipelines, skipped } = simulate({
      changedFiles: ['app/elohim-app/src/app/lamad/services/content.service.ts'],
      commitMsg: 'docs: update readme [skip ci]',
    });
    assert.ok(skipped);
    assert.equal(pipelines.length, 0);
  });

  it('[ci skip] also works', () => {
    const { skipped } = simulate({ commitMsg: '[ci skip]' });
    assert.ok(skipped);
  });

  it('[no ci] also works', () => {
    const { skipped } = simulate({ commitMsg: 'test [no ci]' });
    assert.ok(skipped);
  });

  it('unknown tag is silently ignored', () => {
    const tags = parseCommitTags('[build:banana]');
    assert.equal(tags.length, 0);
  });
});

// ── Ordering ────────────────────────────────────────────────────

describe('dependency ordering', () => {
  it('holochain before edge before genesis', () => {
    const ordered = orderByDependencies(['elohim-genesis', 'elohim-edge', 'elohim-holochain']);
    assert.ok(
      ordered.indexOf('elohim-holochain') < ordered.indexOf('elohim-edge'),
      `holochain (${ordered.indexOf('elohim-holochain')}) should come before edge (${ordered.indexOf('elohim-edge')})`
    );
    assert.ok(
      ordered.indexOf('elohim-edge') < ordered.indexOf('elohim-genesis'),
      `edge (${ordered.indexOf('elohim-edge')}) should come before genesis (${ordered.indexOf('elohim-genesis')})`
    );
  });

  it('sophia before app', () => {
    const ordered = orderByDependencies(['elohim', 'elohim-sophia']);
    assert.equal(ordered[0], 'elohim-sophia');
    assert.equal(ordered[1], 'elohim');
  });

  it('full graph orders correctly', () => {
    const all = ['elohim-genesis', 'elohim', 'elohim-edge', 'elohim-holochain', 'elohim-sophia'];
    const ordered = orderByDependencies(all);
    // holochain and sophia have no deps — they come first (order between them is stable)
    // edge depends on holochain
    // app depends on sophia
    // genesis depends on edge and app
    assert.ok(ordered.indexOf('elohim-holochain') < ordered.indexOf('elohim-edge'));
    assert.ok(ordered.indexOf('elohim-sophia') < ordered.indexOf('elohim'));
    assert.ok(ordered.indexOf('elohim-edge') < ordered.indexOf('elohim-genesis'));
    assert.ok(ordered.indexOf('elohim') < ordered.indexOf('elohim-genesis'));
  });
});

// ── Real-World Scenarios ────────────────────────────────────────

describe('real-world scenarios', () => {
  it('imagodei zome change deploys new hApp (the bug we fixed)', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/holochain/dna/imagodei/coordinator/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'), 'DNA builds');
    assert.ok(pipelines.includes('elohim-edge'), 'edge redeploys (installs new hApp)');
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis seeds and tests');
  });

  it('doorway hotfix rebuilds edge and tests', () => {
    const { pipelines } = simulate({
      changedFiles: ['doorway/doorway-service/src/routes/auth_routes.rs'],
    });
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
    assert.ok(!pipelines.includes('elohim-holochain'), 'no DNA rebuild needed');
    assert.ok(!pipelines.includes('elohim'), 'no app rebuild needed');
  });

  it('pure orchestrator fix with [build:edge] tag forces deploy', () => {
    const { pipelines } = simulate({
      changedFiles: ['genesis/orchestrator/Jenkinsfile'],
      commitMsg: 'fix(ci): cascade change [build:edge]',
    });
    // orchestrator file is CI-only, so changeset yields nothing
    // but [build:edge] forces edge, which cascades to genesis
    assert.ok(pipelines.includes('elohim-edge'));
    assert.ok(pipelines.includes('elohim-genesis'));
    assert.equal(pipelines.filter(p => p === 'elohim-holochain').length, 0);
  });

  it('cross-cutting change: storage + app + sophia', () => {
    const { pipelines } = simulate({
      changedFiles: [
        'elohim/elohim-storage/src/views.rs',
        'app/elohim-app/src/app/lamad/services/content.service.ts',
        'sophia/packages/sophia-core/src/index.ts',
      ],
    });
    assert.ok(pipelines.includes('elohim-edge'), 'storage triggers edge');
    assert.ok(pipelines.includes('elohim-sophia'), 'sophia triggers sophia');
    assert.ok(pipelines.includes('elohim'), 'app triggers app (+ sophia cascade)');
    assert.ok(pipelines.includes('elohim-genesis'), 'genesis cascades');
  });

  it('empty changeset with no tags does nothing', () => {
    const { pipelines } = simulate({ changedFiles: [] });
    assert.equal(pipelines.length, 0);
  });

  it('shared cache-core change triggers both holochain and edge', () => {
    const { pipelines } = simulate({
      changedFiles: ['elohim/elohim-cache-core/src/lib.rs'],
    });
    assert.ok(pipelines.includes('elohim-holochain'), 'cache-core in holochain patterns');
    assert.ok(pipelines.includes('elohim-edge'), 'cache-core in edge patterns');
  });
});
