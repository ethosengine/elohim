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

// ── Run classes ─────────────────────────────────────────────────
//
// A step declares `class` in its build-manifest (rakia `steps[].class`,
// default build). A dispatch's class is the HIGHEST class among its stale
// steps, and a class executes only the steps at or below it. Mirrored in
// build-graph.groovy (RUN_CLASS_ORDER, deriveRunClasses, runClassSelections)
// and the orchestrator Jenkinsfile (parseRunClassTag, runClassFor); the
// run-class.test.mjs contract pins the mirrors to this policy.

/** Run classes, highest first. */
export const RUN_CLASSES = Object.freeze(['build', 'deploy', 'verify', 'measure', 'profile']);
export const DEFAULT_RUN_CLASS = 'build';

/** A step's declared class; absent or unknown reads as build. */
export function stepClass(step) {
  const cls = step?.class;
  return RUN_CLASSES.includes(cls) ? cls : DEFAULT_RUN_CLASS;
}

/** Higher rank = higher class (build highest). Unknown ranks as build. */
export function runClassRank(cls) {
  const i = RUN_CLASSES.indexOf(cls);
  return RUN_CLASSES.length - 1 - (i < 0 ? 0 : i);
}

/** The highest of a set of step classes; null when there is nothing stale. */
export function deriveRunClass(classes) {
  let best = null;
  for (const raw of classes) {
    const cls = RUN_CLASSES.includes(raw) ? raw : DEFAULT_RUN_CLASS;
    if (best === null || runClassRank(cls) > runClassRank(best)) best = cls;
  }
  return best;
}

/** Does a run of `runClass` execute a step of `cls`? (at or below it) */
export function classAllows(runClass, cls) {
  return runClassRank(cls) <= runClassRank(runClass);
}

/**
 * `[run:verify|measure|profile]` in a commit message. build is `[build:*]`'s
 * and deploy is `[deploy-only]`'s, so neither is a run tag.
 */
export function parseRunClassTag(message) {
  const m = /\[run:(verify|measure|profile)\]/i.exec(message ?? '');
  return m ? m[1].toLowerCase() : null;
}

/**
 * The dispatch set and each pipeline's class.
 *
 * @param {object} args
 * @param {Record<string,string[]>} args.staleSteps  pipeline → stale local step names
 * @param {Record<string,Record<string,string>>} args.stepClasses  pipeline → step → class
 * @param {string|null} [args.tag]  a parsed `[run:*]` class
 * @param {string[]} [args.forced]  `[build:*]` pipelines — always build
 * @returns {Record<string,string>} pipeline → run class
 */
export function planRunClasses({ staleSteps = {}, stepClasses = {}, tag = null, forced = [] }) {
  const plan = {};
  if (tag) {
    for (const [pipeline, steps] of Object.entries(stepClasses)) {
      if (Object.values(steps).some((cls) => classAllows(tag, cls))) plan[pipeline] = tag;
    }
  } else {
    for (const [pipeline, steps] of Object.entries(staleSteps)) {
      const cls = deriveRunClass(steps.map((s) => stepClasses[pipeline]?.[s]));
      if (cls) plan[pipeline] = cls;
    }
  }
  for (const pipeline of forced) plan[pipeline] = DEFAULT_RUN_CLASS;
  return plan;
}

function stepClassesOf(steps) {
  const out = {};
  for (const [name, step] of Object.entries(steps || {})) out[name] = stepClass(step);
  return out;
}

/**
 * @returns {Map<string, {pipeline: string, jenkinsPath?: string,
 *   manualOnly: boolean, triggersGenesis: boolean, cascades: boolean,
 *   dependsOn: string[], longRunning: boolean,
 *   deploymentCheck: Record<string,string>|null,
 *   stepClasses: Record<string,string>, manifestPath: string}>}
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
      stepClasses: stepClassesOf(content.steps),
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
