// fleet-roll: a seconds check that is red when CI would select an Edge build and its deploy step
// for a pushed dev or main ref, unless a pushed commit carries `[edge:roll]`. The steward's
// standing rule is that nothing rolls the Edge fleet except deliberately; on 2026-11-19 a one-line
// edit to elohim/sdk/schemas/v1/gate-live-event.schema.json (an input of cargo-build-storage) was
// about to roll all seven alpha peers and was caught only by hand. The red reaches the live run at
// once; whether the push then stops is the hook's declared policy, as for every pre-gate red
// (gate_on_red continue: refused when the hook ends; GATE_ON_RED=stop: at the first red).
// Habit `careless-reds-cost-seconds` (genesis/orchestrator/.epr-meta).
//
// WHAT IT CAN KNOW. "Selects", never "rolls": the guard predicts which Edge steps CI dispatches
// and that Edge runs its deploy stage. Whether pods restart is decided later, by Edge itself: it
// compares each peer's pod template and config before and after apply and skips the storage
// restart when nothing a pod consumes moved (elohim/holochain/Jenkinsfile:2380-2393).
//
// LIMITS. Each protected ref is judged from its own range (destination tip..pushed tip) and its
// tip's tags. Not seen: an Edge step left stale by an earlier landing (CI diffs from its stored
// baseline, which a failed run holds back, Jenkinsfile:217-224, :1450); CI's stored build-process
// hashes (a build-process file counts only when it is in this range); CI's already-built filter
// (:1725-1764), which only drops work. CI's checkout initialises no submodule (:2587-2609; :2414-
// :2420 says its walker cannot see sophia's manifest), so CI may not see the manifests inside
// elohim/brit, elohim/rakia and sophia and then prunes the dependencies on them as external
// (build-graph.groovy:104-119); the guard reads them and propagates through them, which can
// predict a selection CI does not make. A pre-push hook older than the fleet-roll block (a
// worktree pushing through a main checkout whose .husky/pre-push.bash predates it) hands over no
// ref list: then only the union of changed paths is judged, no commit message is read (a forcing
// tag on the tip is not seen there, an accepted limit), a path set that selects no Edge build
// prints nothing, and one that does is red, naming the launcher check and the hook as the fix.
//
// THE CI RULE (read 2026-11-19 at dev 2080441f5; orchestrator = genesis/orchestrator/Jenkinsfile):
// - The graph: graph-walker.mjs walkGraphCi, a port of build-graph.groovy walkBuildGraph (its
//   comment names the lines): CI's glob (dotfiles match, `.epr-meta` excluded), propagation over
//   `depends` to a fixed point, manual-only steps not dispatched, and the manifest-only cap.
// - Tags: the orchestrator and Edge read ONE message, the tip's (orchestrator :2652, Edge :2552).
//   Effective order, highest first:
//   1. [skip ci] / [ci skip] / [no ci] (:2657) sets SKIP_CI; nothing is dispatched (:2758, :2786,
//      :2877, :2927). The post block still archives baselines and a green run advances __global__
//      to that tip (:204-230, :3512), so the next run's diff starts past the push. A later change
//      that selects the same pipeline still builds its bytes; a failed or aborted run keeps the
//      old baseline (:217-224).
//   2. [edge:validate-only] (:2711) dispatches elohim-edge alone (:2841-2844) as verify (:1286),
//      over [build:edge] and [deploy-only]; Edge runs it validate-only (Edge :2551-2568). The other
//      pipelines are dropped from that run, and the baseline moves the same way once it is green.
//   3. [build:…] tokens (parseBuildTagTokens :907): `edge` forces elohim-edge in as build (:1886-
//      :1896, :1913). `all` is expanded at Checkout (:2688-2693) against PIPELINE_REGISTRY_JSON,
//      which only runBuildGraph fills later, in Determine Build Plan (:2413, :1855), so it forces
//      nothing on a fresh run. The guard still reads [build:all] on a protected tip as asking for
//      every pipeline and asks for the declaration (steward ruling 2026-11-19), and says so.
//   4. With no forcing tag, elohim-edge whose only stale step is dataplane-validation is verify
//      (:1874-1876, a literal rule while that step declares no class); a class below deploy (the
//      manifest-only cap) is validate-only too (:1877).
//   5. [run:verify|measure|profile] (:1915-1925) keeps only pipelines declaring a step at or below
//      the class, plus [build:*] force-includes; no elohim-edge step declares a class, so Edge
//      leaves the plan unless forced.
//   6. PIPELINE_RUN_CLASSES is written from that plan (:1962-1964). [deploy-only] (webhook-gated,
//      :2722-2726; a push is a webhook) then sets the dispatch to Edge (and Genesis) (:2828-2834)
//      without touching it, and an Edge absent from it reads as build (runClassFor :931-934): so
//      [deploy-only] over a verify-class Edge stays validate-only (no deploy), and over an absent
//      or build-class Edge runs Edge's deploy stage without its build stages (:1331).
// - Branches: only dev and main deploy. Off them, and for a PR build, Edge is dispatched
//   gate-only (:961-967, :1288; Edge isGateOnly :2598). dev deploys alpha (Edge :3270-3276); main
//   deploys prod (Edge :3538) and, dispatched with FORCE_DEPLOY, alpha too. The staging stage
//   (Edge :3485) needs both !isGateOnly() and a staging branch, so it cannot run today.
// - Dispatch lists name only what CI dispatches: no manual-only pipeline (build-graph.groovy :848),
//   none without a jenkinsPath (:1882), never the orchestrator itself (:1957-1961).
// - Unknown `[edge:…]` tags are inert: the orchestrator matches `[edge:validate-only]` (:2711) and
//   `[edge:gate-only]` (:944) literally; Edge matches `[edge:validate-only]`, `[compute-worker:…]`
//   and `[conductor-roll]` literally (Edge :2556, :2558, :2618, :1930). `[edge:roll]` changes
//   nothing in CI; it is the record in git history of who declared the selection and why.

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { walkGraph } from './graph-walker.mjs';
import { discoverManifests } from './manifest-utils.mjs';
import { parseRunClassTag, runClassRank } from './pipeline-registry.mjs';

/** The checkout this module sits in; ambient GATE_ROOT never redirects the check. */
export const MODULE_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const CHECK = 'fleet-roll';
export const SKIP_CHECK = 'skip-ci';
export const EDGE = 'elohim-edge';
export const VALIDATE_STEP = 'dataplane-validation';
export const SELF_PIPELINE = 'elohim-orchestrator';
export const DEPLOYING_BRANCHES = Object.freeze(['dev', 'main']);
export const ROLL_TAG = '[edge:roll]';
export const MODES = Object.freeze(['refuse', 'warn', 'off']);
const BUFFER = 64 * 1024 * 1024;
const BASELINE = "the next run's diff starts past this push; a later change that selects the same pipeline still builds these bytes; a failed or aborted run keeps the old baseline";
const CHECKOUT_CURE = 'push from a checkout at the pushed commit, with its build-manifest.json files committed';
const short = sha => String(sha ?? '').slice(0, 12);
const isZero = sha => /^0+$/.test(String(sha ?? ''));
const isDeploying = branch => DEPLOYING_BRANCHES.includes(branch);
const branchOf = ref => (String(ref).startsWith('refs/heads/') ? String(ref).slice('refs/heads/'.length) : null);
const row = (check, level, why, cure = '') => ({ check, level, why, cure });

/** `fleet_roll_guard.mode` in pool-policy.json, and why the default `refuse` was taken when it was. */
export function fleetRollMode(root = MODULE_ROOT) {
  let text;
  try { text = readFileSync(join(root, 'genesis/agentic/pool-policy.json'), 'utf8'); }
  catch (error) { return { mode: 'refuse', why: `pool-policy.json could not be read (${error.code || error.message})` }; }
  let policy;
  try { policy = JSON.parse(text); } catch { return { mode: 'refuse', why: 'pool-policy.json is not valid JSON' }; }
  const mode = policy?.fleet_roll_guard?.mode;
  if (mode === undefined) return { mode: 'refuse', why: 'pool-policy.json declares no fleet_roll_guard.mode' };
  if (!MODES.includes(mode)) return { mode: 'refuse', why: `fleet_roll_guard.mode ${JSON.stringify(mode)} is not refuse, warn or off` };
  return { mode, why: null };
}

/** CI's graph over this checkout's manifests (one load, one walk per changed list). */
export function ciWalker(root = MODULE_ROOT) {
  let manifests = null;
  const walked = new Map();
  return changed => {
    const key = changed.join('\n');
    if (!walked.has(key)) walked.set(key, walkGraph(manifests ??= loadNamed(root), changed, { ci: true }));
    return walked.get(key);
  };
}

/** loadManifests, but a file that does not parse is named in the error. */
function loadNamed(root) {
  return discoverManifests(root).map(path => {
    try { return { path, content: JSON.parse(readFileSync(resolve(root, path), 'utf8')) }; }
    catch (error) { throw new Error(`${path}: ${String(error.message).split('\n')[0]}`); }
  });
}

function buildTokens(tip) {
  const tokens = [];
  for (const m of tip.matchAll(/\[build:([a-z,-]+)\]/gi)) tokens.push(...m[1].split(',').map(t => t.trim().toLowerCase()));
  return tokens;
}

const dispatchable = (meta, name) => name !== SELF_PIPELINE && !!meta?.[name]?.jenkinsPath && !meta[name].manualOnly;

/** What CI does with elohim-edge for this walk and tip (header, "THE CI RULE"). Pure. */
export function predict(walked, tip = '') {
  const meta = walked.meta || {};
  const edge = walked.pipelines.find(p => p.name === EDGE);
  const buildSteps = (edge?.steps ?? []).filter(s => s.name !== VALIDATE_STEP);
  const others = walked.pipelines.map(p => p.name).filter(name => name !== EDGE && dispatchable(meta, name));
  const skipTag = /\[(skip ci|ci skip|no ci)\]/i.exec(tip)?.[0] ?? null;
  const validateOnly = /\[edge:validate-only\]/i.test(tip);
  const tokens = buildTokens(tip);
  const forcedEdge = tokens.includes('edge');
  const buildAll = tokens.includes('all');
  const runTag = parseRunClassTag(tip);
  const deployOnly = /\[deploy[-_ ]only\]/i.test(tip);
  let edgeClass = null;
  if (!skipTag && validateOnly) edgeClass = 'verify';
  else if (!skipTag) {
    let inPlan = !!edge;
    let cls = edge ? edge.runClass : null;
    if (edge && !forcedEdge && edge.steps.every(s => s.name === VALIDATE_STEP)) cls = 'verify';
    if (forcedEdge) { inPlan = true; cls = 'build'; }
    if (runTag && !forcedEdge) {
      const declares = Object.values(meta[EDGE]?.stepClasses || {}).some(c => runClassRank(c) <= runClassRank(runTag));
      inPlan = declares;
      cls = declares ? runTag : null;
    }
    if (deployOnly) { cls = inPlan ? cls : 'build'; inPlan = true; }
    edgeClass = inPlan ? cls : null;
  }
  const selects = edgeClass !== null && runClassRank(edgeClass) >= runClassRank('deploy');
  const asks = buildAll && !selects && !skipTag && !validateOnly;
  const forcedBy = forcedEdge ? (tip.match(/\[build:[a-z,-]+\]/gi) || []).find(t => buildTokens(t).includes('edge')) : null;
  return { edge, buildSteps, others, skipTag, validateOnly, forcedBy, buildAll, runTag, deployOnly, edgeClass, selects, asks };
}

function edgeText(p) {
  if (!p.buildSteps.length) return '';
  const causes = [...new Set(p.buildSteps.map(s => s.cause))];
  const through = [...new Set(p.buildSteps.map(s => s.via).filter(v => v && !v.startsWith(`${EDGE}:`)))];
  const more = causes.length > 3 ? ` and ${causes.length - 3} more` : '';
  return `${EDGE} stale steps ${p.buildSteps.map(s => s.name).join(', ')}, from ${causes.slice(0, 3).join(', ')}${more}${through.length ? `, through ${through.join(', ')}` : ''}`;
}

function reachText(p, where) {
  if (p.selects) {
    if (p.forcedBy) return `the tip's ${p.forcedBy} makes CI select an Edge build and its deploy step for this ${where}`;
    if (p.deployOnly && p.edgeClass === 'build' && !p.edge) return `the tip's [deploy-only] makes CI select the Edge deploy step for this ${where} (its build stages are skipped)`;
    if (p.deployOnly) return `CI would select the Edge deploy step for this ${where}; the tip's [deploy-only] skips its build stages`;
    return `CI would select an Edge build and its deploy step for this ${where}`;
  }
  return `the tip's [build:all] asks for every pipeline, Edge included, on this ${where}: CI forces nothing with it today (it expands [build:all] at Checkout against a pipeline registry filled only later, orchestrator Jenkinsfile:2688-2693, :2413), but it is read here as asking, so the selection is asked to be declared`;
}

function cureFor(p) {
  const parts = [];
  if (p.forcedBy) parts.push(`drop ${p.forcedBy} from the tip commit`);
  if (p.asks) parts.push('drop [build:all] from the tip commit');
  if (p.deployOnly && p.selects) parts.push('drop [deploy-only] from the tip commit');
  if (p.buildSteps.length && !p.forcedBy) parts.push('take the Edge build inputs out of this push, or land them with the next deliberate roll');
  parts.push(`or declare the selection: add ${ROLL_TAG} and the reason to a pushed commit message`);
  return `${parts.join('; ')}. [edge:validate-only] on the tip also stops the selection, but it drops every other pipeline from that run and moves the baseline past this push: not a cure for a mixed push.`;
}

/**
 * The rows for one ref: `fleet-roll` and, on a protected ref with a skip tag, `skip-ci`. Pure.
 * @param {{target: string|null, walked: object, tip: string, declaredBy: string|null,
 *   mode: string, policyWhy?: string|null}} args
 */
export function evaluateRef({ target, walked, tip = '', declaredBy = null, mode = 'refuse', policyWhy = null }) {
  if (mode === 'off') return [];
  const deploying = isDeploying(target);
  const where = deploying ? `push to ${target}` : `change, were it pushed to dev (${target ? `target ${target} does not deploy` : 'not a push'})`;
  const level = l => (l === 'REFUSED' && (mode === 'warn' || !deploying) ? 'WARN' : l);
  const p = predict(walked, tip);
  const rows = [];
  const add = r => rows.push(policyWhy ? { ...r, why: `${r.why} (policy: mode ${mode}, the default, because ${policyWhy})` } : r);
  const inputs = edgeText(p);
  if (p.skipTag) {
    if (p.buildSteps.length) {
      add(row(CHECK, 'WARN', `the tip's ${p.skipTag} dispatches nothing for this ${where}, so the Edge build inputs it carries (${inputs}) are not built by this run; ${BASELINE}`,
        `push them under a tip without the skip tag (with ${ROLL_TAG} if the selection is meant)`));
    } else if (p.edge) {
      add(row(CHECK, 'ok', `the tip's ${p.skipTag} dispatches nothing for this ${where}; its Edge inputs feed validation only`));
    }
  } else if (p.validateOnly) {
    if (p.buildSteps.length || p.others.length) {
      const parts = [p.buildSteps.length ? `its Edge build inputs (${inputs}) are not built by this run` : '', p.others.length ? `it drops ${p.others.join(', ')} from the run` : ''].filter(Boolean);
      add(row(CHECK, 'WARN', `the tip's [edge:validate-only] makes ${EDGE} validate only for this ${where}: ${parts.join(', and ')}; once the run ends green, ${BASELINE}`,
        'push those changes under a tip without [edge:validate-only], and keep the measuring commit on its own push'));
    } else if (p.edge) {
      add(row(CHECK, 'ok', `the tip's [edge:validate-only] makes ${EDGE} validate only for this ${where}; no build, no deploy`));
    }
  } else if (p.selects || p.asks) {
    const what = `${reachText(p, where)}${inputs ? `: ${inputs}` : ''}`;
    if (declaredBy) add(row(CHECK, 'ok', `${what} — declared by ${ROLL_TAG} in ${short(declaredBy)}`));
    else {
      add(row(CHECK, level('REFUSED'),
        `${what} — and no pushed commit carries ${ROLL_TAG}. Read from ${target ? "this ref's own range and its tip's tags" : 'the changed paths given, with no commit read'}; an Edge step left stale by an earlier landing is not seen`,
        cureFor(p)));
    }
  } else if (p.edge) {
    const note = p.runTag ? `the tip's [run:${p.runTag}] keeps ${EDGE} out of the dispatch` : `${EDGE} validates only (${p.edge.steps.map(s => s.name).join(', ')})`;
    add(row(CHECK, 'ok', `${note}; CI selects no Edge build or deploy for this ${where}`));
  }
  if (p.skipTag && deploying) {
    const names = walked.pipelines.map(x => x.name).filter(name => dispatchable(walked.meta, name));
    if (names.length) {
      add(row(SKIP_CHECK, 'WARN', `the tip's ${p.skipTag} dispatches none of ${names.join(', ')}, which this ref's paths select; ${BASELINE}`,
        "move the skip tag to a tip whose paths select no pipeline, or drop it from this push's tip"));
    }
  }
  return rows;
}

/** `GATE_PUSH_REFS`: one `<remote ref> <pushed sha> <destination sha>` per line (pre-push.bash). */
export function parseRefs(text) {
  return String(text ?? '').split('\n').map(line => line.trim()).filter(Boolean).map(line => {
    const [ref, local, remote] = line.split(/\s+/);
    return { ref, local, remote, branch: branchOf(ref) };
  });
}

function git(spawn, root, args) {
  const r = spawn('git', ['-C', root, ...args], { encoding: 'utf8', maxBuffer: BUFFER });
  if (r.error || r.status !== 0) {
    const error = new Error(`git ${args.join(' ')} exited ${r.status ?? r.error?.code}: ${String(r.stderr || r.error?.message || '').trim().split('\n')[0]}`);
    error.cure = 'git fetch origin, then push again';
    throw error;
  }
  return String(r.stdout);
}
const lines = text => text.split('\n').map(l => l.trim()).filter(Boolean);
const hasCommit = (spawn, root, sha) => spawn('git', ['-C', root, 'cat-file', '-e', `${sha}^{commit}`], { encoding: 'utf8' }).status === 0;

/** Manifests that differ between this checkout and `sha` (tracked or untracked). */
function manifestDrift(spawn, root, sha) {
  const spec = ['--', ':(glob)**/build-manifest.json'];
  return [...new Set([...lines(git(spawn, root, ['diff', '--name-only', sha, ...spec])),
    ...lines(git(spawn, root, ['ls-files', '--others', '--exclude-standard', ...spec]))])];
}

function judgeRef({ ref, spawn, root, walk, mode, policyWhy }) {
  const deploying = isDeploying(ref.branch);
  const target = ref.branch ?? ref.ref;
  const where = `push to ${target}`;
  const indeterminate = (why, cure) => [row(CHECK, mode === 'warn' || !deploying ? 'WARN' : 'REFUSED', `${where}: ${why} — the check cannot decide, and an unreadable range never reads as "nothing changed"`, cure)];
  let base = ref.remote;
  if (isZero(ref.remote) || !hasCommit(spawn, root, ref.remote)) {
    const why = isZero(ref.remote) ? `this push creates ${target}, so there is no destination tip to read a range from`
      : `the destination tip ${short(ref.remote)} is not in this clone, so this ref's range cannot be read`;
    if (deploying) {
      return indeterminate(why, isZero(ref.remote)
        ? `this check has no range to read for a new ${target}: create it deliberately, outside an ordinary push`
        : `git fetch origin ${target}, then push again`);
    }
    base = git(spawn, root, ['merge-base', 'origin/dev', ref.local]).trim();
  }
  if (deploying) {
    const head = git(spawn, root, ['rev-parse', 'HEAD']).trim();
    if (head !== ref.local) return indeterminate(`the pushed tip ${short(ref.local)} is not this checkout's HEAD ${short(head)}, so the build manifests read here are not the pushed ones`, CHECKOUT_CURE);
    const drift = manifestDrift(spawn, root, ref.local);
    if (drift.length) return indeterminate(`build-manifest.json differs between this checkout and the pushed tip (${drift.slice(0, 3).join(', ')})`, CHECKOUT_CURE);
  }
  const changed = lines(git(spawn, root, ['diff', '--name-only', base, ref.local]));
  // The tip alone carries CI's tags; the declaration is found with a search that returns one commit.
  const tip = git(spawn, root, ['log', '-1', '--format=%B', ref.local]);
  const declaredBy = git(spawn, root, ['log', '-i', '--fixed-strings', `--grep=${ROLL_TAG}`, '-1', '--format=%H', `${base}..${ref.local}`]).trim() || null;
  return evaluateRef({ target, walked: walk(changed), tip, declaredBy, mode, policyWhy });
}

/** A failure to decide: red on a protected ref under refuse, a WARN otherwise; never silent. */
export function failureRows({ error, deploying, mode, policyWhy = null }) {
  if (mode === 'off') return [];
  const first = String(error?.message || error).split('\n')[0];
  const policy = policyWhy ? ` (policy: mode ${mode}, the default, because ${policyWhy})` : '';
  return [row(CHECK, mode === 'refuse' && deploying ? 'REFUSED' : 'WARN', `the fleet-roll check could not decide: ${first}${policy}`,
    error?.cure || 'fix what the error names and push again (a malformed build-manifest.json also stops CI\'s own graph, build-graph.groovy:815-827)')];
}

const words = text => String(text ?? '').split(/\s+/).filter(Boolean);
/** Whether any pushed ref named by the environment is dev or main. */
export function pushDeploys(env) {
  return [...words(env.GATE_PUSH_TARGETS), ...parseRefs(env.GATE_PUSH_REFS).map(r => r.ref)].some(ref => isDeploying(branchOf(ref)));
}

/**
 * GATE_FLEET_ROLL_DONE carries the pid of the pre-push hook that ran the check. It is honoured only
 * when that process is this one or an ancestor of it, read from /proc; anywhere else (an inherited
 * value, no /proc) the check runs again, which costs one more walk (well under a second).
 */
export function doneFlagHonoured(env = process.env, readStat = pid => readFileSync(`/proc/${pid}/stat`, 'utf8')) {
  const done = Number(env.GATE_FLEET_ROLL_DONE);
  if (!Number.isInteger(done) || done <= 1) return false;
  for (let pid = process.pid, hops = 0; pid > 1 && hops < 64; hops += 1) {
    if (pid === done) return true;
    try {
      const stat = readStat(pid); // "pid (comm) state ppid …"; comm may hold spaces or parentheses
      pid = Number(stat.slice(stat.lastIndexOf(')') + 2).split(' ')[1]);
    } catch { return false; }
  }
  return false;
}

/**
 * The preflight entry. Under pre-push: each protected ref judged from its own range (else the
 * first ref, advisory). Outside a push (`just gate`, `just status gates`): the changed list, as if
 * pushed to dev, never red. Returns rows, or null when there is nothing to judge.
 */
export function checkFleetRoll({ env = process.env, changed = null, root = MODULE_ROOT, walk, spawn = spawnSync, policy } = {}) {
  const { mode, why: policyWhy } = policy ?? fleetRollMode(root);
  if (mode === 'off') return null;
  walk ??= ciWalker(root);
  const targets = words(env.GATE_PUSH_TARGETS);
  try {
    if (!targets.length && env.GATE_PUSH_REFS === undefined) {
      if (!Array.isArray(changed)) return null;
      return evaluateRef({ target: null, walked: walk(changed), mode, policyWhy });
    }
    if (env.GATE_PUSH_REFS === undefined) return olderHook({ targets, changed, walk, mode, policyWhy });
    const refs = parseRefs(env.GATE_PUSH_REFS);
    const named = [...new Set(refs.map(r => r.ref))].sort().join(' ');
    if (named !== [...new Set(targets)].sort().join(' ')) {
      return [row(CHECK, mode === 'refuse' && pushDeploys(env) ? 'REFUSED' : 'WARN',
        `the push names refs [${targets.join(' ')}] that its ref list GATE_PUSH_REFS [${refs.map(r => r.ref).join(' ')}] does not match — the check cannot decide`,
        'push again from a plain shell: an inherited GATE_PUSH_REFS or GATE_PUSH_TARGETS is the usual cause')];
    }
    const judged = refs.filter(r => isDeploying(r.branch));
    return (judged.length ? judged : refs.slice(0, 1)).flatMap(ref => judgeRef({ ref, spawn, root, walk, mode, policyWhy }));
  } catch (error) {
    return failureRows({ error, deploying: pushDeploys(env), mode, policyWhy });
  }
}

function olderHook({ targets, changed, walk, mode, policyWhy }) {
  const target = targets.map(branchOf).find(isDeploying) ?? branchOf(targets[0]) ?? targets[0];
  const deploying = isDeploying(target);
  if (!Array.isArray(changed)) {
    return failureRows({ error: Object.assign(new Error('the running pre-push hook handed over no changed-path list'), { cure: OLDER_HOOK_CURE }), deploying, mode, policyWhy });
  }
  const p = predict(walk(changed), '');
  if (!p.selects) return [];
  const why = `${reachText(p, `push to ${target}`)}: ${edgeText(p)} — and the running pre-push hook hands over no ref list (GATE_PUSH_REFS), so neither this ref's own range nor a declaration can be read`;
  const r = row(CHECK, mode === 'warn' || !deploying ? 'WARN' : 'REFUSED', why, OLDER_HOOK_CURE);
  return [policyWhy ? { ...r, why: `${r.why} (policy: mode ${mode}, the default, because ${policyWhy})` } : r];
}

const OLDER_HOOK_CURE = 'check the launcher first: test -x <worktree>/.husky/_/pre-push (pnpm install creates it; with it missing, git runs no hook at all, silently); then push with -c core.hooksPath=<worktree>/.husky/_ so the current hook reads each ref\'s range and messages, or bring the main checkout up to date (its .husky/pre-push.bash predates the fleet-roll block)';
