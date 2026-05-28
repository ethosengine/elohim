/**
 * Pipeline registry — single source of truth for pipeline-level metadata.
 *
 * Loads every build-manifest.json in the workspace and exposes the
 * pipeline-level fields (jenkinsPath, manualOnly, triggersGenesis,
 * cascades, dependsOn) that previously lived in Jenkinsfile.PIPELINES
 * and orchestrator-strategy.mjs.PIPELINES.
 *
 * Replaces orchestrator-strategy.mjs as of plan
 * 2026-05-28-orchestrator-clean-build-triggers.
 */

import { loadManifests } from './manifest-utils.mjs';

/**
 * @returns {Map<string, {pipeline: string, jenkinsPath?: string,
 *   manualOnly: boolean, triggersGenesis: boolean, cascades: boolean,
 *   dependsOn: string[], manifestPath: string}>}
 */
export function loadPipelineRegistry(rootDir) {
  const manifests = loadManifests(rootDir);
  const registry = new Map();

  for (const { path, content } of manifests) {
    if (!content.pipeline) continue;
    if (registry.has(content.pipeline)) {
      throw new Error(
        `Duplicate pipeline name '${content.pipeline}' in ${path} and ${registry.get(content.pipeline).manifestPath}`
      );
    }
    registry.set(content.pipeline, {
      pipeline: content.pipeline,
      jenkinsPath: content.jenkinsPath,
      manualOnly: content.manualOnly === true,
      triggersGenesis: content.triggersGenesis === true,
      cascades: content.cascades === undefined ? true : content.cascades === true,
      dependsOn: Array.isArray(content.dependsOn) ? content.dependsOn : [],
      manifestPath: path,
    });
  }

  return registry;
}

export function nonManualPipelines(registry) {
  return [...registry.values()]
    .filter(p => !p.manualOnly)
    .map(p => p.pipeline);
}

export function dispatchablePipelines(registry) {
  return [...registry.values()]
    .filter(p => typeof p.jenkinsPath === 'string' && p.jenkinsPath.length > 0)
    .map(p => p.pipeline);
}

export function pipelinesThatTriggerGenesis(registry) {
  return [...registry.values()]
    .filter(p => p.triggersGenesis)
    .map(p => p.pipeline);
}

export function pipelineDependencyMap(registry) {
  const map = new Map();
  for (const p of registry.values()) {
    map.set(p.pipeline, p.dependsOn);
  }
  return map;
}
