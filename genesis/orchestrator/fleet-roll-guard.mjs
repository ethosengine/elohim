// fleet-roll: refuse, in seconds, a push to dev or main that CI would answer by rebuilding the
// Edge pipeline and rolling its fleet, unless a pushed commit declares the roll with `[edge:roll]`.
// The steward's standing rule is that nothing rolls the Edge fleet except deliberately; on
// 2026-11-19 a one-line edit to elohim/sdk/schemas/v1/gate-live-event.schema.json (an input of
// cargo-build-storage) was about to roll all seven alpha peers and was caught only by hand.
// Habit `careless-reds-cost-seconds` (genesis/orchestrator/.epr-meta).
//
// LIMIT: the prediction reads this push's own changed paths and its own commit tags. An Edge step
// left stale by an earlier landing (a range CI has not dispatched yet, or a [skip ci] run that a
// later push supersedes) is not seen. CI's own narrowing filters (the already-built filter, edge's
// closure-keyed skips) are not applied either, so for this push's own paths and tags the error
// lies on the side of predicting a roll that CI then skips.
//
// The CI rule this mirrors (read 2026-11-19 at dev 2080441f5):
// - The orchestrator reads ONE commit message, the tip it checked out: `git log -1 --format=%B`
//   (genesis/orchestrator/Jenkinsfile:2652); elohim/holochain/Jenkinsfile:2552 does the same. A tag
//   in an earlier commit of the range is inert, so forcing and suppressing tags are read on the tip.
// - [skip ci] / [ci skip] / [no ci] (Jenkinsfile:2657) sets SKIP_CI: the plan, health check and
//   Execute Builds stages are skipped (:2758, :2786, :2877, :2927), so nothing is dispatched. The
//   post block still archives baselines and the run ends SUCCESS, so __global__ advances to that
//   tip (:204-:230, :3512): the next run's diff starts past it.
// - [build:edge] / [build:all] / [build:…,edge,…] force elohim-edge in as class build
//   (:2666-:2705, parseBuildTagTokens :907, forced runClasses :1913). elohim-edge is not manualOnly,
//   so [build:all] includes it. commit-tag-parser.mjs parseCommitTags is the JS mirror used here.
// - [deploy-only] (webhook-gated, :2722-:2726; a push is a webhook) dispatches elohim-edge with
//   DEPLOY_ONLY (:2828-:2834, :1331): it skips edge's build stages and runs its deploy stage. It is
//   treated here as a roll; whether pods restart when the image tags are unchanged was not verified.
// - [edge:validate-only] on the tip isolates the dispatch to elohim-edge alone (:2711-:2714,
//   :2841-:2844) and sends it as RUN_CLASS=verify (:1286), which edge runs as validate-only
//   (elohim/holochain/Jenkinsfile:2551-:2568): no build, no deploy. It wins over [build:edge] and
//   [deploy-only]. It also drops every other pipeline from that run, and the run advances
//   __global__, so the other pipelines' changes in that range are not dispatched later either.
// - [run:verify|measure|profile] (:1915-:1925) dispatches only pipelines that declare a step at or
//   below the tagged class, plus [build:*] force-includes. No elohim-edge step declares a class
//   (all read as build), so the tag leaves elohim-edge undispatched unless forced; with
//   [deploy-only] it is still dispatched (:2830) and rolls.
// - With no tag, elohim-edge is dispatched validate-only when every stale step is
//   dataplane-validation (:1858-:1879, a literal rule while that step declares no class), or when
//   its derived run class is below deploy (the manifest-only cap). Any other stale step builds.
// - Only dev and main deploy. Off dev/main (and for a PR build) the orchestrator dispatches edge
//   gate-only, with no publish or deploy (gateOnlyRun :961, applyGateOnlyOverride :963,
//   triggerPipeline :1288; edge isGateOnly elohim/holochain/Jenkinsfile:2598). dev deploys alpha
//   (:3270-:3276); main deploys prod (:3538-:3544) and, being dispatched with FORCE_DEPLOY, alpha
//   too. The staging stage (:3485) needs both !isGateOnly() and a staging branch, so it is
//   unreachable today.
// - Unknown `[edge:…]` tags are inert: the orchestrator matches `[edge:validate-only]` (:2711) and
//   `[edge:gate-only]` (:944) literally, and the edge Jenkinsfile matches `[edge:validate-only]`
//   (:2556, :2562), `[compute-worker:…]` (:2558, :2618) and `[conductor-roll]` (:1930) literally.
//   Neither file has a generic `[edge:` matcher, so `[edge:roll]` changes nothing in CI. It is a
//   record in git history of who declared the roll and why.

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { walkGraph } from './graph-walker.mjs';
import { loadManifests } from './manifest-utils.mjs';
import { parseCommitTags, parseSkipCi } from './commit-tag-parser.mjs';
import { loadPipelineRegistry, parseRunClassTag, runClassRank } from './pipeline-registry.mjs';

export const CHECK = 'fleet-roll';
export const EDGE = 'elohim-edge';
export const VALIDATE_STEP = 'dataplane-validation';
/** Branches whose orchestrator run can deploy the Edge fleet. */
export const DEPLOYING_BRANCHES = Object.freeze(['dev', 'main']);
export const ROLL_TAG = '[edge:roll]';
export const MODES = Object.freeze(['refuse', 'warn', 'off']);

const hasRollTag = message => /\[edge:roll\]/i.test(message ?? '');
const hasValidateOnly = message => /\[edge:validate-only\]/i.test(message ?? '');
const hasDeployOnly = message => /\[deploy[-_ ]only\]/i.test(message ?? '');
const short = sha => (sha ? String(sha).slice(0, 12) : '');

/** `fleet_roll_guard.mode` in pool-policy.json; absent or unreadable reads as refuse. */
export function fleetRollMode(root) {
  try {
    const mode = JSON.parse(readFileSync(join(root, 'genesis/agentic/pool-policy.json'), 'utf8')).fleet_roll_guard?.mode;
    return MODES.includes(mode) ? mode : 'refuse';
  } catch { return 'refuse'; }
}

/** The CI walk of this tree: walkGraph over every build-manifest.json, no .ci-ignore (CI applies none). */
export function treeWalker(root) {
  let manifests = null;
  return changed => walkGraph(manifests ??= loadManifests(root), changed);
}

/**
 * The tip's [build:*] tag(s) that force elohim-edge, or null. `[build:all]` reads the registry's
 * manualOnly flags, so the registry is loaded only when a [build:*] tag is present.
 */
function forcingBuildTag(tip, registry) {
  const tags = tip.match(/\[build:[a-z,-]+\]/gi);
  if (!tags || !parseCommitTags(tip, registry()).includes(EDGE)) return null;
  return tags.join(' ');
}

const row = (level, why = '', cure = '') => ({ check: CHECK, level, why, cure });

function cureFor(prediction) {
  const parts = [];
  if (prediction.forcedBy) parts.push(`drop ${prediction.forcedBy} from the tip commit`);
  if (prediction.paths.length) {
    parts.push('take the change out of the Edge build inputs (split it onto its own push)');
    parts.push('or land it with the next deliberate roll');
  }
  parts.push(`or declare the roll: add ${ROLL_TAG} and the reason to a pushed commit message`);
  return `${parts.join('; ')}. [edge:validate-only] on the tip also stops the roll, but it drops every other pipeline from that run and the next run's diff starts past it, so their changes in this range are never dispatched: not a cure for a mixed push.`;
}

function describe(prediction) {
  const steps = prediction.steps.join(', ');
  const paths = prediction.paths.slice(0, 3).join(', ');
  const more = prediction.paths.length > 3 ? ` and ${prediction.paths.length - 3} more` : '';
  if (!prediction.forcedBy) return `${EDGE} builds ${steps}, selected by ${paths}${more}`;
  return `the tip's ${prediction.forcedBy} forces a full ${EDGE} run${steps ? ` (stale steps: ${steps}${paths ? `, from ${paths}${more}` : ''})` : ''}`;
}

/**
 * Predict whether this push rolls the Edge fleet, and the preflight row that says so.
 *
 * @param {object} args
 * @param {string[]} args.changed  this push's changed paths (union over its refs)
 * @param {string[]|Error} args.messages  the pushed commits' messages, tip first; an Error when
 *   they could not be read. `[]` for a run that is not a push (`just gate`).
 * @param {string|null} args.target  the pushed branch (`dev`, `self/x`), or null when not a push
 * @param {'refuse'|'warn'|'off'} args.mode  pool-policy.json fleet_roll_guard.mode
 * @param {(changed: string[]) => {pipelines: Array<{name: string, reasons: string[], steps: Array<{name: string, class: string}>, runClass: string}>}} args.walk
 * @param {() => Map<string, object>} [args.registry]  pipeline registry, read only for [build:all]
 * @returns {{prediction: object|null, row: object|null}}
 */
export function evaluateFleetRoll({ changed, messages, target, mode = 'refuse', walk, registry = () => new Map() }) {
  if (mode === 'off') return { prediction: null, row: null };
  const deploying = DEPLOYING_BRANCHES.includes(target);
  const pushed = target !== null && target !== undefined;
  const where = pushed ? `push to ${target}` : 'run, which is not a push';
  const severity = level => (level === 'REFUSED' && (mode === 'warn' || !deploying) ? 'WARN' : level);

  if (messages instanceof Error || !Array.isArray(changed)) {
    const what = messages instanceof Error ? `the pushed commits' messages could not be read (${messages.message})` : 'the changed paths could not be read';
    return {
      prediction: null,
      row: row(severity('REFUSED'), `${where}: ${what} — a roll cannot be ruled out, and an unreadable range never reads as "nothing changed"`,
        'git fetch origin, then push again (the hook hands the range over as GATE_PUSH_RANGES; a worktree push running an older checkout\'s hook needs -c core.hooksPath=<worktree>/.husky/_)'),
    };
  }

  const tip = messages[0] ?? '';
  const declaredIn = messages.findIndex(hasRollTag);
  const edge = changed.length ? walk(changed).pipelines.find(p => p.name === EDGE) : undefined;
  const paths = [...new Set((edge?.reasons ?? []).map(r => r.replace(/^(source|buildProcess): /, '')))];
  const staleSteps = (edge?.steps ?? []).map(s => s.name);
  const buildSteps = staleSteps.filter(s => s !== VALIDATE_STEP);
  const prediction = { roll: false, deploying, target: target ?? null, steps: buildSteps, paths, forcedBy: null, suppressedBy: null, declaredBy: null };

  // Precedence mirrors the orchestrator (header): skip > validate-only > force > run-cap > graph.
  const buildTag = parseSkipCi(tip) || hasValidateOnly(tip) ? null : forcingBuildTag(tip, registry);
  if (parseSkipCi(tip)) prediction.suppressedBy = '[skip ci]';
  else if (hasValidateOnly(tip)) prediction.suppressedBy = '[edge:validate-only]';
  else if (buildTag) prediction.forcedBy = buildTag;
  else if (parseRunClassTag(tip) && !hasDeployOnly(tip)) prediction.suppressedBy = `[run:${parseRunClassTag(tip)}]`;
  else if (hasDeployOnly(tip)) prediction.forcedBy = '[deploy-only]';

  if (prediction.forcedBy) {
    prediction.roll = true;
    if (!prediction.steps.length) prediction.steps = staleSteps;
  } else if (!prediction.suppressedBy && edge) {
    const belowDeploy = runClassRank(edge.runClass) < runClassRank('deploy');
    prediction.roll = buildSteps.length > 0 && !belowDeploy;
  }

  if (prediction.suppressedBy) {
    if (!edge) return { prediction, row: null };
    return { prediction, row: row('ok', `the tip's ${prediction.suppressedBy} keeps ${EDGE} from building or deploying for this ${where}`) };
  }
  if (!prediction.roll) {
    if (!edge) return { prediction, row: null };
    return { prediction, row: row('ok', `${EDGE} validates only (${staleSteps.join(', ') || VALIDATE_STEP}); no build, no fleet roll`) };
  }
  if (declaredIn >= 0) {
    prediction.declaredBy = declaredIn === 0 ? 'the tip commit' : 'a pushed commit';
    return { prediction, row: row('ok', `roll declared by ${ROLL_TAG} in ${prediction.declaredBy}: ${describe(prediction)}`) };
  }
  const reach = deploying ? `this push to ${target} would rebuild and roll the Edge fleet` : `this change would roll the Edge fleet if pushed to dev (${pushed ? `target ${target} does not deploy` : 'not a push'})`;
  return {
    prediction,
    row: row(severity('REFUSED'),
      `${reach}: ${describe(prediction)} — and no pushed commit declares ${ROLL_TAG}. Read from this push's own paths and tags only; an Edge step left stale by an earlier landing is not seen`,
      cureFor(prediction)),
  };
}

/** `GATE_PUSH_RANGES`: one `<remote ref> <base>..<pushed sha>` per line (pre-push.bash). */
export function parseRanges(text) {
  return String(text ?? '').split('\n').map(line => line.trim()).filter(Boolean).map(line => {
    const [ref, range] = line.split(/\s+/);
    const sha = range?.includes('..') ? range.slice(range.indexOf('..') + 2) : '';
    return { ref, range, sha, branch: ref?.startsWith('refs/heads/') ? ref.slice('refs/heads/'.length) : null };
  });
}

/** Messages of a pushed range, tip first. A failed `git` call is an Error, never an empty list. */
export function readMessages({ root, range, sha, spawn = spawnSync }) {
  const log = args => {
    const r = spawn('git', ['-C', root, 'log', '--format=%B%x1e', ...args], { encoding: 'utf8' });
    if (r.error || r.status !== 0) {
      throw new Error(`git log ${args.join(' ')} exited ${r.status ?? r.error?.code}: ${String(r.stderr || r.error?.message || '').trim().split('\n')[0]}`);
    }
    const parts = String(r.stdout).split('\x1e');
    parts.pop(); // what follows the last separator is not a message
    return parts.map(m => m.trim());
  };
  try {
    // The tip is read on its own (CI reads exactly it, and a range already upstream is empty);
    // the whole range follows it, the tip again among them.
    const tip = log(['-1', sha]);
    if (tip.length !== 1) throw new Error(`git log -1 ${sha} returned ${tip.length} messages`);
    return [tip[0], ...log([range])];
  } catch (error) { return error; }
}

/**
 * The preflight entry. Under pre-push, one row per deploying ref (else one for the first ref);
 * outside a push (`just gate`, `just status gates`), one row as if the change were pushed to dev,
 * never REFUSED. Silent when no changed-path list exists outside a push.
 */
export function checkFleetRoll({ root, env = process.env, changed, walk = treeWalker(root), spawn = spawnSync, mode = fleetRollMode(root) }) {
  if (mode === 'off') return null;
  const registry = () => loadPipelineRegistry(root);
  const pushing = !!(env.GATE_PUSH_TARGETS || env.GATE_PUSH_RANGES !== undefined);
  if (!pushing) {
    if (!Array.isArray(changed)) return null;
    return evaluateFleetRoll({ changed, messages: [], target: null, mode, walk, registry }).row;
  }
  if (env.GATE_PUSH_RANGES === undefined) {
    return evaluateFleetRoll({ changed, messages: new Error('GATE_PUSH_RANGES is not set by the running pre-push hook'), target: 'dev', mode, walk, registry }).row;
  }
  const ranges = parseRanges(env.GATE_PUSH_RANGES);
  if (!ranges.length) return null;
  const deploying = ranges.filter(r => DEPLOYING_BRANCHES.includes(r.branch));
  const rows = [];
  for (const r of deploying.length ? deploying : ranges.slice(0, 1)) {
    const messages = r.range && r.sha ? readMessages({ root, range: r.range, sha: r.sha, spawn }) : new Error(`malformed range line for ${r.ref}`);
    const { row: out } = evaluateFleetRoll({ changed, messages, target: r.branch ?? r.ref, mode, walk, registry });
    if (out) rows.push(out);
  }
  return rows;
}
