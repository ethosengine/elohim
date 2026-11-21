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
import { stepClass, deriveRunClass, capManifestOnly, manifestOnlyLine, runClassRank } from './pipeline-registry.mjs';
import { LEGACY_ALL, isGovernancePath, matchInputs } from './gate-inputs.mjs';

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

// The governance-path rule and today's matcher live in gate-inputs.mjs, the one place a gate's
// input class is decided; isGovernancePath stays exported here for its existing importers.
export { isGovernancePath };

/**
 * The stale set of every step under an input class: qualified step → reasons. LEGACY_ALL is the
 * matcher CI prediction and the local gate always used; SOURCES (gate-inputs.mjs) drops a step's
 * own buildProcess, for the local gate path only.
 */
export function staleSteps(manifests, changedFiles, cls = LEGACY_ALL) {
  const stale = new Map();
  for (const { content } of manifests) {
    for (const [name, step] of Object.entries(content.steps)) {
      const reasons = cls.stepReasons(step.inputs, changedFiles);
      if (reasons.length > 0) stale.set(`${content.pipeline}:${name}`, reasons);
    }
  }
  return stale;
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
      // A local quality gate may explicitly consume policy bytes. Keep those
      // inputs separate from pipeline sources so governance never rolls a fleet.
      const reasons = matchInputs(config.inputs, changedFiles, { includeGovernance: true });
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

/**
 * build-graph.groovy matchesGlob (:186-218), ported literally: the pattern becomes a regex by
 * escaping `.` only; a double star followed by a slash becomes `(.+/)?`, a remaining double star
 * `.*`, a single star `[^/]*` and `?` `[^/]`; the regex must match the whole path. Unlike
 * picomatch's default it matches dotfiles (the storage source `elohim/sdk/` + double star matches
 * `elohim/sdk/schemas/.gitignore`). Other regex characters keep their regex meaning, as in Groovy.
 */
export function ciGlobMatch(filePath, pattern) {
  const file = filePath.startsWith('./') ? filePath.slice(2) : filePath;
  const pat = pattern.startsWith('./') ? pattern.slice(2) : pattern;
  const SS = '\u0000SS\u0000';
  const DS = '\u0000DS\u0000';
  const regex = pat
    .replaceAll('.', '\\.')
    .replaceAll('**/', SS)
    .replaceAll('**', DS)
    .replaceAll('*', '[^/]*')
    .replaceAll('?', '[^/]')
    .replaceAll(SS, '(.+/)?')
    .replaceAll(DS, '.*');
  return new RegExp(`^(?:${regex})$`).test(file);
}

/**
 * build-graph.groovy extractFunctionBody (:249-265): the body after `def <name>(…) {`, by brace
 * count, or null when no such definition is found.
 */
export function extractFunctionBody(text, name) {
  const m = new RegExp(`def\\s+${name}\\s*\\([^)]*\\)\\s*\\{`).exec(text);
  if (!m) return null;
  let depth = 1;
  let pos = m.index + m[0].length;
  while (pos < text.length && depth > 0) {
    if (text[pos] === '{') depth += 1;
    else if (text[pos] === '}') depth -= 1;
    pos += 1;
  }
  return text.slice(m.index + m[0].length, pos - 1).trim();
}

/**
 * What build-graph.groovy walkBuildGraph computes (:802-864), for the fleet-roll guard:
 * - composeGraph (:58-137): steps keyed `pipeline:step`; an omitted `depends` is []; a bare
 *   entry is qualified to its own pipeline; a second manifest naming a pipeline already seen is
 *   an error and skipped (:66-75); a dependency on a pipeline with no manifest is pruned (:104-119);
 *   a dependency on a missing step of a present pipeline is an error (:121-131); then, only when
 *   nothing else failed, cycles (self-dependencies included) are errors (:140-178). Any error
 *   stops CI's graph (:815-827): `errors` is returned and nothing is predicted.
 * - a step is stale by source when a changed, non-governance file matches one of its sources with
 *   CI's glob (checkSourceChanges :234-247, reason `source: <file> matches <pattern>`, files outer,
 *   patterns inner); else by build process (:286-357), each reference in order: with `readFile`,
 *   a file CI cannot read, or a named function it cannot find, is stale (:307-321); a referenced
 *   file in the changeset is stale (the cold-start and no-prior-hash rule, :329-348). CI's stored
 *   hashes are not visible here: a changed file whose referenced function CI would hash as
 *   unchanged still reads as stale.
 * - staleness propagates to a fixed point over `depends`, whatever the dependency's class or
 *   manual flag, reason `depends: <first stale dependency>` (propagateStaleness :360-375);
 * - a manual-only pipeline's stale steps propagate but are not dispatched (:848);
 * - a pipeline's run class is its stale steps' highest class (deriveRunClasses :471-482), capped
 *   at verify when every stale step is stale by source, none has a stale dependency, and every
 *   file matching its sources is a build-manifest.json (manifestOnlyPipelines :491-521).
 * Each stale step carries `cause`: the changed or unreadable file behind it, through propagation.
 */
export function walkGraphCi(manifests, changedFiles, { readFile = null } = {}) {
  const steps = new Map();
  const meta = {};
  const owner = {};
  const errors = [];
  const normalized = [];
  for (const manifest of manifests) {
    const { path, content } = manifest;
    const pipeline = content.pipeline;
    if (Object.hasOwn(meta, pipeline)) { errors.push(`Duplicate pipeline name '${pipeline}' in ${path} and ${owner[pipeline]}`); continue; }
    owner[pipeline] = path;
    const stepDefs = Object.fromEntries(Object.entries(content.steps || {}).map(([n, s]) => [n, { ...s, depends: s.depends ?? [] }]));
    normalized.push({ ...manifest, content: { ...content, steps: stepDefs } });
    meta[pipeline] = {
      jenkinsPath: typeof content.jenkinsPath === 'string' && content.jenkinsPath.trim() ? content.jenkinsPath : null,
      manualOnly: content.manualOnly === true,
      stepClasses: Object.fromEntries(Object.entries(stepDefs).map(([n, s]) => [n, stepClass(s)])),
    };
    for (const [name, step] of Object.entries(stepDefs)) {
      steps.set(`${pipeline}:${name}`, {
        pipeline, name, step, cls: stepClass(step), manualOnly: content.manualOnly === true,
        depends: step.depends.map(dep => resolveStep(dep, pipeline)),
      });
    }
  }
  // Kept: an existing step, or a missing step of a present pipeline (an error below). Pruned: a
  // dependency on a pipeline with no manifest.
  for (const s of steps.values()) s.depends = s.depends.filter(dep => steps.has(dep) || Object.hasOwn(meta, dep.split(':', 1)[0]));
  for (const [qualified, s] of steps) {
    for (const dep of s.depends) if (!steps.has(dep)) errors.push(`Step '${qualified}' depends on '${dep}' which does not exist.`);
  }
  if (!errors.length) {
    const visited = new Set();
    const inStack = new Set();
    const dfs = (node, path) => {
      const s = steps.get(node);
      if (!s) return;
      visited.add(node);
      inStack.add(node);
      const here = [...path, node];
      for (const dep of s.depends) {
        if (!visited.has(dep)) dfs(dep, here);
        else if (inStack.has(dep)) {
          errors.push(`Dependency cycle detected: ${[...here.slice(here.indexOf(dep)), dep].join(' -> ')}`);
          inStack.delete(node);
          return;
        }
      }
      inStack.delete(node);
    };
    for (const node of steps.keys()) if (!visited.has(node)) dfs(node, []);
  }
  if (errors.length) return { projects: [], pipelines: [], meta, errors };
  const stale = new Map();
  for (const [qualified, s] of steps) {
    let hit = null;
    for (const file of changedFiles) {
      if (isGovernancePath(file)) continue;
      const pattern = (s.step.inputs?.sources || []).find(p => ciGlobMatch(file, p));
      if (pattern) { hit = { reason: `source: ${file} matches ${pattern}`, cause: file }; break; }
    }
    for (const ref of hit ? [] : (s.step.inputs?.buildProcess || [])) {
      const [file, fn] = ref.split('@', 2);
      if (readFile) {
        const text = readFile(file); // a string, null, or { why } when the reader can say why not
        if (typeof text !== 'string') { hit = { reason: `buildProcess: cannot read ${file}${text?.why ? ` (${text.why})` : ''}`, cause: file }; break; }
        if (fn && extractFunctionBody(text, fn) === null) { hit = { reason: `buildProcess: function ${fn} not found in ${file}`, cause: file }; break; }
      }
      if (changedFiles.includes(file)) { hit = { reason: `buildProcess: ${ref} in changeset`, cause: file }; break; }
    }
    if (hit) stale.set(qualified, hit);
  }
  for (let changed = true; changed;) {
    changed = false;
    for (const [qualified, s] of steps) {
      if (stale.has(qualified)) continue;
      const dep = s.depends.find(d => stale.has(d));
      if (dep) { stale.set(qualified, { reason: `depends: ${dep}`, cause: stale.get(dep).cause, via: stale.get(dep).via ?? dep }); changed = true; }
    }
  }
  const byPipeline = new Map();
  for (const [qualified, hit] of stale) {
    const s = steps.get(qualified);
    if (s.manualOnly) continue;
    if (!byPipeline.has(s.pipeline)) byPipeline.set(s.pipeline, []);
    byPipeline.get(s.pipeline).push({ name: s.name, class: s.cls, reason: hit.reason, cause: hit.cause, via: hit.via ?? null, qualified });
  }
  const pipelines = [...byPipeline].map(([name, list]) => {
    let runClass = deriveRunClass(list.map(s => s.class));
    let onlyManifests = true;
    let triggers = 0;
    for (const s of list) {
      const step = steps.get(s.qualified);
      if (!s.reason.startsWith('source:') || step.depends.some(d => stale.has(d))) { onlyManifests = false; break; }
      for (const file of changedFiles) {
        if (!(step.step.inputs?.sources || []).some(p => ciGlobMatch(file, p))) continue;
        triggers += 1;
        if (file !== 'build-manifest.json' && !file.endsWith('/build-manifest.json')) onlyManifests = false;
      }
      if (!onlyManifests) break;
    }
    if (onlyManifests && triggers > 0 && runClassRank(runClass) > runClassRank('verify')) runClass = 'verify';
    return { name, steps: list.map(({ qualified, ...rest }) => rest), reasons: list.map(s => s.reason), runClass };
  });
  const projects = projectsFromStale(normalized, new Map([...stale].map(([q, h]) => [q, [h.reason]])), changedFiles);
  return { projects, pipelines, meta, errors: [] };
}

/**
 * `gateInputs` is the input class of the LOCAL gate path's borrowed steps (gate-inputs.mjs policy
 * `cls`), default LEGACY_ALL. It never reaches CI prediction: `ci` ignores it, and the path-only
 * walk's `pipelines` are always computed from the legacy stale set.
 */
export function walkGraph(manifests, changedFiles, { ci = false, readFile = null, gateInputs = LEGACY_ALL } = {}) {
  if (ci) return walkGraphCi(manifests, changedFiles, { readFile });
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
  // Propagation (one hop, locally) comes from the graph oracle (`brit build affected`) in gate-oracle.mjs,
  // which hands projectsFromStale an externally computed stale set.
  const gateStale = gateInputs === LEGACY_ALL ? stale : staleSteps(manifests, changedFiles, gateInputs);
  const projects = projectsFromStale(manifests, gateStale, changedFiles);

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
