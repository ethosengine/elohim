#!/usr/bin/env node
// Graph walker: matches changed files against build manifest source globs,
// propagates staleness through dependency edges, and maps to gate projects.
//
// Library usage: import { walkGraph } from './graph-walker.mjs'
// CLI usage: echo "file1\nfile2" | node graph-walker.mjs
//            node graph-walker.mjs --dependents a,b   (transitive consumers, one per line)

import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import picomatch from 'picomatch';
import { loadManifests, resolveStep } from './manifest-utils.mjs';
import { filterChanged } from './ci-ignore.mjs';
import { stepClass, deriveRunClass, capManifestOnly, manifestOnlyLine } from './pipeline-registry.mjs';

/**
 * Topologically sort steps using Kahn's algorithm.
 * Returns qualified step names in dependency order (dependencies first).
 *
 * @param {Map<string, {step: object, pipeline: string, manifest: object}>} stepIndex
 * @returns {string[]}
 */
export function topoSort(stepIndex) {
  const inDegree = new Map();
  const adj = new Map();

  for (const qualified of stepIndex.keys()) {
    inDegree.set(qualified, 0);
    adj.set(qualified, []);
  }

  for (const [qualified, { step, pipeline }] of stepIndex) {
    for (const dep of step.depends) {
      const qualDep = resolveStep(dep, pipeline);
      if (!stepIndex.has(qualDep)) continue;
      adj.get(qualDep).push(qualified);
      inDegree.set(qualified, inDegree.get(qualified) + 1);
    }
  }

  const queue = [];
  for (const [node, deg] of inDegree) {
    if (deg === 0) queue.push(node);
  }

  const order = [];
  while (queue.length > 0) {
    const node = queue.shift();
    order.push(node);
    for (const neighbor of adj.get(node)) {
      const newDeg = inDegree.get(neighbor) - 1;
      inDegree.set(neighbor, newDeg);
      if (newDeg === 0) queue.push(neighbor);
    }
  }

  return order;
}

/**
 * Transitive consumers of any failed pipeline (backlog row 19). The
 * orchestrator dooms exactly these after a level fails; every other pipeline
 * keeps its wave. Pure: walks `dependsOn` in reverse.
 *
 * @param {Record<string, {dependsOn?: string[]}>} pipelineMeta
 * @param {string[]} failed
 * @returns {string[]} sorted, excluding the failed names themselves
 */
export function dependentsClosure(pipelineMeta, failed) {
  const consumers = new Map();
  for (const [name, meta] of Object.entries(pipelineMeta)) {
    for (const dep of meta?.dependsOn ?? []) {
      if (!consumers.has(dep)) consumers.set(dep, []);
      consumers.get(dep).push(name);
    }
  }
  const failedSet = new Set(failed);
  const doomed = new Set();
  const queue = [...failedSet];
  while (queue.length > 0) {
    for (const consumer of consumers.get(queue.shift()) ?? []) {
      if (failedSet.has(consumer) || doomed.has(consumer)) continue;
      doomed.add(consumer);
      queue.push(consumer);
    }
  }
  return [...doomed].sort();
}

function matchInputs(inputs, changedFiles) {
  const reasons = [];
  if (!inputs) return reasons;

  for (const pattern of inputs.sources || []) {
    const matcher = picomatch(pattern);
    for (const file of changedFiles) {
      if (matcher(file)) {
        reasons.push(`source: ${file}`);
        break;
      }
    }
  }

  for (const ref of inputs.buildProcess || []) {
    const fileName = ref.split('@')[0];
    if (changedFiles.includes(fileName)) reasons.push(`buildProcess: ${ref}`);
  }
  return reasons;
}

/**
 * Walk the build graph to determine which gate projects are affected by changed files.
 *
 * @param {Array<{path: string, content: object}>} manifests - Loaded manifests
 * @param {string[]} changedFiles - List of changed file paths (relative to repo root)
 * @returns {{ projects: Array<{name: string, dir: string, reasons: string[]}>,
 *             pipelines: Array<{name: string, reasons: string[],
 *               steps: Array<{name: string, class: string}>, runClass: string}> }}
 *   `projects` = local gate projects (what the pre-push hook builds).
 *   `pipelines` = pipelines with stale steps (what Jenkins dispatches). These
 *   are NOT the same set — see Phase 5.
 */
/**
 * Phase 4 as a function: map a stale-step set (qualified name → reasons) to the
 * gate projects that own those steps, plus gate-only projects whose `inputs`
 * match the changed files. Ordered by topological position of the earliest
 * stale step. `stale` may come from the path-only walk (walkGraph) or from
 * an external oracle (gate-oracle.mjs).
 */
export function projectsFromStale(manifests, stale, changedFiles) {
  const stepIndex = new Map();
  for (const { content } of manifests) {
    for (const [name, step] of Object.entries(content.steps)) {
      stepIndex.set(`${content.pipeline}:${name}`, { step, pipeline: content.pipeline, manifest: content });
    }
  }
  const order = topoSort(stepIndex);
  const projectMap = new Map();

  for (const { content } of manifests) {
    if (!content.gate?.projects) continue;
    for (const [projectName, config] of Object.entries(content.gate.projects)) {
      const triggerSteps = config.steps || (config.inputs ? [] : Object.keys(content.steps));
      const reasons = matchInputs(config.inputs, changedFiles);
      let minOrder = Infinity;
      for (const stepName of triggerSteps) {
        const qualified = `${content.pipeline}:${stepName}`;
        if (stale.has(qualified)) {
          reasons.push(...stale.get(qualified));
          const idx = order.indexOf(qualified);
          if (idx >= 0 && idx < minOrder) minOrder = idx;
        }
      }
      if (reasons.length > 0) {
        projectMap.set(projectName, { dir: config.dir, reasons: [...new Set(reasons)], minOrder });
      }
    }
  }

  return [...projectMap.entries()]
    .sort((a, b) => a[1].minOrder - b[1].minOrder)
    .map(([name, { dir, reasons }]) => ({ name, dir, reasons }));
}

export function walkGraph(manifests, changedFiles) {
  if (manifests.length === 0) return { projects: [], pipelines: [] };

  // Phase 1: Build index
  const stepIndex = new Map();
  for (const { content } of manifests) {
    for (const [name, step] of Object.entries(content.steps)) {
      const qualified = `${content.pipeline}:${name}`;
      stepIndex.set(qualified, { step, pipeline: content.pipeline, manifest: content });
    }
  }

  // Phase 2: Mark stale (source globs + buildProcess files)
  const stale = new Map();

  for (const [qualified, { step }] of stepIndex) {
    const reasons = matchInputs(step.inputs, changedFiles);

    if (reasons.length > 0) {
      stale.set(qualified, reasons);
    }
  }

  // Phase 3+4: the path-only walk does NOT propagate staleness through
  // dependencies — "did files in this project change?" is its whole question.
  // Propagation (one hop, locally) comes from the rakia oracle in gate-oracle.mjs,
  // which hands projectsFromStale an externally computed stale set.
  const projects = projectsFromStale(manifests, stale, changedFiles);

  // Phase 5: Map stale steps to PIPELINES.
  //
  // Distinct from Phase 4 on purpose. Gate projects answer "what should the
  // pre-push hook build locally?"; pipelines answer "what will Jenkins
  // dispatch?" — which is what build-graph.groovy computes from stale steps.
  // A pipeline can have zero gate projects and still be dispatched: the custom
  // conductor image (elohim-conductor) is built in another repo and has no
  // local gate at all. Without this phase, preview.mjs reports "nothing will
  // build" for a conductor bump that CI does in fact build.
  //
  // Each pipeline also carries its stale steps with their declared class and
  // the run class those derive (the max; absent class = build) — the same
  // derivation build-graph.groovy sends downstream as RUN_CLASS.
  const pipelineMap = new Map();
  for (const [qualified, reasons] of stale) {
    const { pipeline, step } = stepIndex.get(qualified);
    if (!pipelineMap.has(pipeline)) pipelineMap.set(pipeline, { reasons: [], steps: [], triggers: [], opaque: false });
    const entry = pipelineMap.get(pipeline);
    entry.reasons.push(...reasons);
    // Every changed file that made this step stale (matchInputs keeps only the
    // first per pattern); a build-process trigger is never manifest-only.
    if (reasons.some((r) => !r.startsWith('source: '))) entry.opaque = true;
    const matchers = (step.inputs?.sources || []).map((p) => picomatch(p));
    for (const file of changedFiles) if (matchers.some((m) => m(file))) entry.triggers.push(file);
    entry.steps.push({ name: qualified.slice(pipeline.length + 1), class: stepClass(step) });
  }
  const pipelines = [...pipelineMap.entries()].map(([name, { reasons, steps, triggers, opaque }]) => {
    const derived = deriveRunClass(steps.map((s) => s.class));
    const runClass = opaque ? derived : capManifestOnly(derived, triggers);
    // Same line the dispatcher echoes (build-graph.groovy); stderr, because
    // gate consumers parse stdout.
    if (runClass !== derived) process.stderr.write(`${manifestOnlyLine(name)}\n`);
    return { name, reasons, steps, runClass };
  });

  return { projects, pipelines };
}

// ── CLI mode ─────────────────────────────────────────────────────

const isMain = import.meta.url === `file://${process.argv[1]}` ||
               import.meta.url === `file://${resolve(process.argv[1])}`;

const dependentsIdx = isMain ? process.argv.indexOf('--dependents') : -1;

if (isMain && dependentsIdx !== -1) {
  // `--dependents a,b` → the pipelines a failure of a/b dooms, one per line.
  // Reads no stdin: the orchestrator's fail-forward block calls it bare.
  const ROOT = resolve(dirname(new URL(import.meta.url).pathname), '../..');
  const failed = (process.argv[dependentsIdx + 1] ?? '').split(',').map(s => s.trim()).filter(Boolean);
  const meta = Object.fromEntries(
    loadManifests(ROOT)
      .filter(({ content }) => content.pipeline)
      .map(({ content }) => [content.pipeline, { dependsOn: Array.isArray(content.dependsOn) ? content.dependsOn : [] }]),
  );
  const doomed = dependentsClosure(meta, failed);
  if (doomed.length > 0) process.stdout.write(doomed.join('\n') + '\n');
} else if (isMain) {
  const ROOT = resolve(dirname(new URL(import.meta.url).pathname), '../..');
  // Read stdin via fd 0 (works for both terminal and pipe; '/dev/stdin'
  // fails with ENXIO when stdin is a child-process pipe).
  const input = readFileSync(0, 'utf8');
  const rawFiles = input.split('\n').map(f => f.trim()).filter(Boolean);
  // Apply .ci-ignore at the CLI boundary so husky and any other stdin
  // consumer see the same filtered list as the Jenkinsfile does.
  const changedFiles = filterChanged(rawFiles);
  const manifests = loadManifests(ROOT);
  const result = walkGraph(manifests, changedFiles);

  if (process.argv.includes('--shell-lines')) {
    // Tab-separated lines that shell scripts can parse with awk -F'\t' instead
    // of eval'ing a constructed shell string (which has historically been the
    // source of silent partial failures in .husky/pre-push). The PROJECTS line
    // and DIRS line are positionally aligned (i-th project corresponds to i-th
    // dir).
    process.stdout.write('PROJECTS\t' + result.projects.map(p => p.name).join(' ') + '\n');
    process.stdout.write('DIRS\t' + result.projects.map(p => p.dir).join(' ') + '\n');
    process.exit(0);
  }

  process.stdout.write(JSON.stringify(result) + '\n');
}
