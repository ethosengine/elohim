#!/usr/bin/env node
// Graph walker: matches changed files against build manifest source globs,
// propagates staleness through dependency edges, and maps to gate projects.
//
// Library usage: import { walkGraph } from './graph-walker.mjs'
// CLI usage: echo "file1\nfile2" | node graph-walker.mjs

import { readFileSync } from 'fs';
import { resolve, dirname } from 'path';
import picomatch from 'picomatch';
import { loadManifests, resolveStep } from './manifest-utils.mjs';
import { filterChanged } from './ci-ignore.mjs';

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
 * Walk the build graph to determine which gate projects are affected by changed files.
 *
 * @param {Array<{path: string, content: object}>} manifests - Loaded manifests
 * @param {string[]} changedFiles - List of changed file paths (relative to repo root)
 * @returns {{ projects: Array<{name: string, dir: string, reasons: string[]}> }}
 */
export function walkGraph(manifests, changedFiles) {
  if (manifests.length === 0) return { projects: [] };

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
    const reasons = [];

    for (const pattern of step.inputs.sources) {
      const matcher = picomatch(pattern);
      for (const file of changedFiles) {
        if (matcher(file)) {
          reasons.push(`source: ${file}`);
          break;
        }
      }
    }

    for (const ref of step.inputs.buildProcess) {
      const fileName = ref.split('@')[0];
      if (changedFiles.includes(fileName)) {
        reasons.push(`buildProcess: ${ref}`);
      }
    }

    if (reasons.length > 0) {
      stale.set(qualified, reasons);
    }
  }

  // Phase 3: Topo-sort for output ordering (dependencies before dependents)
  // Note: we do NOT propagate staleness through dependencies. The hook's job
  // is "did files in this project change?" — propagation is a Jenkins concern.
  // Source/buildProcess matching is sufficient for quality gate detection.
  const order = topoSort(stepIndex);

  // Phase 4: Map stale steps to gate projects
  const projectMap = new Map();

  for (const { content } of manifests) {
    if (!content.gate?.projects) continue;

    for (const [projectName, config] of Object.entries(content.gate.projects)) {
      const triggerSteps = config.steps || Object.keys(content.steps);
      const reasons = [];
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
        projectMap.set(projectName, { dir: config.dir, reasons, minOrder });
      }
    }
  }

  // Sort by dependency order (lowest topo-sort index first)
  const projects = [...projectMap.entries()]
    .sort((a, b) => a[1].minOrder - b[1].minOrder)
    .map(([name, { dir, reasons }]) => ({ name, dir, reasons }));

  return { projects };
}

// ── CLI mode ─────────────────────────────────────────────────────

const isMain = import.meta.url === `file://${process.argv[1]}` ||
               import.meta.url === `file://${resolve(process.argv[1])}`;

if (isMain) {
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
  process.stdout.write(JSON.stringify(result) + '\n');
}
