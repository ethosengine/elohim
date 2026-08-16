/**
 * Pipeline registry — single source of truth for pipeline-level metadata.
 *
 * Loads every build-manifest.json in the workspace and exposes the
 * pipeline-level fields (jenkinsPath, manualOnly, triggersGenesis,
 * cascades, dependsOn, longRunning, deploymentCheck) that previously
 * lived in Jenkinsfile.PIPELINES and orchestrator-strategy.mjs.PIPELINES.
 *
 * Replaces orchestrator-strategy.mjs as of plan
 * 2026-05-28-orchestrator-clean-build-triggers.
 */

import { resolve, relative } from 'path';
import { loadManifests } from './manifest-utils.mjs';

/**
 * Build a deploymentCheck map from manifest deployment.targets.<env>.healthCheck.
 * Returns null if there are no targets or none have healthCheck — preserves
 * the legacy PIPELINES behaviour where pipelines without deployments have
 * deploymentCheck: null.
 *
 * @param {object|undefined} deployment
 * @returns {Record<string,string>|null}
 */
function buildDeploymentCheck(deployment) {
  const targets = deployment?.targets;
  if (!targets) return null;
  const out = {};
  for (const [envName, target] of Object.entries(targets)) {
    if (target?.healthCheck) out[envName] = target.healthCheck;
  }
  return Object.keys(out).length > 0 ? out : null;
}

/**
 * @returns {Map<string, {pipeline: string, jenkinsPath?: string,
 *   manualOnly: boolean, triggersGenesis: boolean, cascades: boolean,
 *   dependsOn: string[], longRunning: boolean,
 *   deploymentCheck: Record<string,string>|null, manifestPath: string}>}
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
      jenkinsJob: content.jenkinsJob || content.pipeline,
      jenkinsBranch: content.jenkinsBranch || null,
      manualOnly: content.manualOnly === true,
      triggersGenesis: content.triggersGenesis === true,
      cascades: content.cascades === undefined ? true : content.cascades === true,
      dependsOn: Array.isArray(content.dependsOn) ? content.dependsOn : [],
      longRunning: content.longRunning === true,
      deploymentCheck: buildDeploymentCheck(content.deployment),
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

/**
 * Load the local quality-gate registry from build manifests.
 *
 * Unlike the pipeline registry, this preserves the gate execution contract.
 * Humans (`just gate`) and the pre-push hook both consume this map, so adding a
 * gate project never requires a second project-name switch in shell.
 *
 * @returns {Map<string, {name: string, dir: string, pipeline: string,
 *   steps: string[], inputs: object|null, run: object, manifestPath: string}>}
 */
export function loadGateRegistry(rootDir) {
  const manifests = loadManifests(rootDir);
  const registry = new Map();
  const root = resolve(rootDir);

  for (const { path, content } of manifests) {
    for (const [name, config] of Object.entries(content.gate?.projects || {})) {
      if (registry.has(name)) {
        throw new Error(
          `Duplicate gate project '${name}' in ${path} and ${registry.get(name).manifestPath}`
        );
      }

      const dir = resolve(root, config.dir);
      const rel = relative(root, dir);
      if (rel.startsWith('..') || resolve(root, rel) !== dir) {
        throw new Error(`Gate project '${name}' escapes the repository: ${config.dir}`);
      }

      const steps = config.steps || (config.inputs ? [] : Object.keys(content.steps));
      for (const step of steps) {
        if (!Object.hasOwn(content.steps, step)) {
          throw new Error(`Gate project '${name}' references unknown step '${step}' in ${path}`);
        }
      }

      registry.set(name, {
        name,
        dir: config.dir,
        pipeline: content.pipeline,
        steps,
        inputs: config.inputs || null,
        run: config.run,
        manifestPath: path,
      });
    }
  }

  return registry;
}
