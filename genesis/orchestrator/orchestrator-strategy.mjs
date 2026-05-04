/**
 * Orchestrator Strategy — algorithm module.
 *
 * Pure-function JS port of the orchestrator's changeset routing,
 * dependency propagation, cascade behavior, commit-tag parsing, and
 * topological ordering from genesis/orchestrator/Jenkinsfile.
 *
 * This module is imported by:
 *   - orchestrator-strategy.test.mjs  (the test mirror)
 *   - preview.mjs                      (the `just ci-preview` CLI)
 *
 * Keeping the algorithm in a non-test module means importing it does
 * NOT trigger node:test's auto-discovered `describe`/`it` suites as a
 * side effect.
 *
 * Keep in sync with genesis/orchestrator/Jenkinsfile. Drift detection
 * lives in orchestrator-strategy.test.mjs.
 */

// ══════════════════════════════════════════════════════════════════
// PIPELINES config — extracted from Jenkinsfile (keep in sync!)
// ══════════════════════════════════════════════════════════════════

export const PIPELINES = {
  'elohim-holochain': {
    jenkinsPath: 'elohim/holochain/dna/Jenkinsfile',
    changePatterns: ['elohim/holochain/dna/', 'elohim/elohim-cache-core/', 'elohim/holochain/rna/', 'VERSION'],
    dependsOn: [],
    cascades: true,
    manualOnly: false,
    triggersGenesis: true,
  },
  'elohim-edge': {
    jenkinsPath: 'elohim/holochain/Jenkinsfile',
    changePatterns: ['doorway/doorway-service/', 'doorway/doorway-app/', 'elohim/elohim-agent/elohim-agent-sdk/', 'elohim/holochain/edgenode/', 'elohim/elohim-storage/', 'elohim/elohim-cache-core/', 'crates/', 'genesis/orchestrator/manifests/', 'genesis/orchestrator/environments/', 'genesis/orchestrator/data/', 'VERSION'],
    dependsOn: ['elohim-holochain'],
    cascades: undefined,
    manualOnly: false,
    triggersGenesis: true,
  },
  'elohim': {
    jenkinsPath: 'Jenkinsfile',
    changePatterns: ['app/elohim-app/', 'app/elohim-library/', 'elohim/sdk/', 'VERSION'],
    dependsOn: ['elohim-sophia'],
    cascades: undefined,
    manualOnly: false,
    triggersGenesis: true,
  },
  'elohim-genesis': {
    jenkinsPath: 'genesis/Jenkinsfile',
    changePatterns: ['genesis/', 'data/'],
    dependsOn: ['elohim-edge', 'elohim'],
    cascades: undefined,
    manualOnly: false,
    triggersGenesis: false,
  },
  'elohim-steward': {
    jenkinsPath: 'steward/device/Jenkinsfile',
    changePatterns: ['steward/'],
    dependsOn: ['elohim-holochain'],
    cascades: undefined,
    manualOnly: true,
    triggersGenesis: false,
  },
  'elohim-sophia': {
    jenkinsPath: 'sophia.Jenkinsfile',
    changePatterns: ['sophia/'],
    dependsOn: [],
    cascades: true,
    manualOnly: false,
    triggersGenesis: false,
  },
  'elohim-epr': {
    jenkinsPath: 'elohim/epr/Jenkinsfile',
    changePatterns: ['elohim/epr/', 'elohim/sdk/epr-ts/'],
    dependsOn: [],
    cascades: false,
    manualOnly: false,
    triggersGenesis: false,
  },
  'elohim-storybook': {
    jenkinsPath: 'app/elohim-library/Jenkinsfile',
    changePatterns: [
      'app/elohim-library/projects/**',
      'app/elohim-library/.storybook/**',
      'app/elohim-library/package.json',
      'app/elohim-library/tsconfig.storybook.json',
      'app/elohim-library/angular.json',
      'app/elohim-library/Jenkinsfile',
      'app/elohim-library/images/**',
      'genesis/orchestrator/manifests/elohim-storybook/**',
    ],
    dependsOn: [],
    cascades: false,
    manualOnly: false,
    triggersGenesis: false,
  },
};

// ══════════════════════════════════════════════════════════════════
// Pure functions — ported from Jenkinsfile Groovy
// ══════════════════════════════════════════════════════════════════

export const CI_ONLY_PATTERNS = ['.claude', '.github/', '.husky/'];
export const CI_ONLY_FILES = [
  'CLAUDE.md',
  'ROADMAP.md',
  'genesis/orchestrator/Jenkinsfile',
  'genesis/orchestrator/build-graph.groovy',
];

/**
 * Mirrors Jenkinsfile analyzePipelineRequirements() — changeset analysis.
 * Simplified: no per-pipeline baselines (not needed for strategy tests).
 */
export function analyzePipelineRequirements(changedFiles) {
  const analysis = {};

  // Build reverse index: jenkinsPath → owning pipeline name.
  // A pipeline's own Jenkinsfile triggers ONLY that pipeline.
  const jenkinsfileToPipeline = {};
  for (const [pname, pconfig] of Object.entries(PIPELINES)) {
    if (pconfig.jenkinsPath) {
      jenkinsfileToPipeline[pconfig.jenkinsPath] = pname;
    }
  }

  for (const [name, config] of Object.entries(PIPELINES)) {
    const matchedPatterns = [];
    const matchedFiles = [];

    for (const file of changedFiles) {
      // Skip CI-only files (orchestrator glue, hooks, docs)
      if (CI_ONLY_PATTERNS.some(p => file.startsWith(p)) ||
          CI_ONLY_FILES.includes(file)) {
        continue;
      }

      // Pipeline Jenkinsfiles: route to owning pipeline only.
      if (file.endsWith('Jenkinsfile')) {
        const owningPipeline = jenkinsfileToPipeline[file];
        if (owningPipeline === name) {
          if (!matchedPatterns.includes('jenkinsfile')) {
            matchedPatterns.push('jenkinsfile');
          }
          matchedFiles.push(file);
        }
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
export function propagateDependencies(pipelines, analysis) {
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
export function orderByDependencies(pipelineList) {
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
export function parseCommitTags(commitMsg) {
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

export function parseSkipCi(commitMsg) {
  return /\[(skip ci|ci skip|no ci)\]/i.test(commitMsg);
}

/**
 * Full orchestrator simulation: changeset + commit tags + propagation + ordering.
 */
export function simulate({ changedFiles = [], commitMsg = '' } = {}) {
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
