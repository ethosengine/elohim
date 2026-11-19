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
// and that Edge runs its deploy stage. Whether pods restart is decided later by Edge itself, which
// skips a peer's storage restart when nothing its pod consumes moved
// (elohim/holochain/Jenkinsfile:2380-2393). What `[edge:roll]` establishes is only that a pushed
// commit message carries it; a reason beside it is asked for and not checked.
//
// WHAT IT READS. Under a push, git objects only, never the working tree: each pushed ref's range
// (destination tip..pushed tip), the pushed tip's message, and the build manifests of the pushed
// commit's own superproject tree (`git ls-tree`, contents through `git cat-file --batch`). That
// is what CI's checkout holds: it initialises no submodule (orchestrator Jenkinsfile:2587-2609),
// so the verdict does not depend on which submodules, ignored files or sparse paths a developer
// has. Build-process files are read from the same tree. The policy in force is the DESTINATION
// tip's genesis/agentic/pool-policy.json, so a push cannot switch off its own check and an
// uncommitted edit changes nothing; outside a push (`just gate`) the working tree is read and the
// check can only warn.
//
// LIMITS. Not seen: an Edge step left stale by an earlier landing (CI diffs from its stored
// baseline, which a failed run holds back, Jenkinsfile:217-224, :1450); CI's stored build-process
// hashes (a referenced file in the range reads as stale even when the hashed function did not
// change); CI's already-built filter (:1725-1764), which can drop a pipeline, including a
// validate-only Edge that [deploy-only] then re-adds as build. A pre-push hook older than the
// fleet-roll block hands over no ref list: then HEAD's tree and HEAD's message are read with the
// union of changed paths, which is right only when HEAD is the pushed commit (a non-HEAD push
// through an older hook is not read). If node cannot start at all, the hook block records a red
// on a push that names dev or main, in every mode, because the mode cannot be read without node;
// otherwise it prints a WARN. A tree listing or batch read larger than 64 MiB cannot be judged
// and is said so. [skip ci] or [edge:validate-only] on the tip suppress the request for a
// declaration (CI then dispatches nothing, or validates only); their WARN rows say what that
// costs. In a shallow clone, a range that reaches the shallow boundary cannot prove that no
// pushed commit carries the declaration, so "none found" there cannot be decided.
//
// THE CI RULE (read 2026-11-19 at dev 2080441f5; orchestrator = genesis/orchestrator/Jenkinsfile):
// - The graph: graph-walker.mjs walkGraphCi, a port of build-graph.groovy walkBuildGraph (its
//   comment names the lines): CI's glob, structural validation, build-process reads, propagation
//   over `depends` to a fixed point, manual-only steps not dispatched, the manifest-only cap.
//   A pipeline with a blank or missing jenkinsPath, Edge included, is never dispatched (:1882,
//   :1892, :1269); the orchestrator never dispatches itself (:1957-1961).
// - Tags: the orchestrator and Edge read ONE message, the tip's (orchestrator :2652, Edge :2552).
//   Effective order, highest first:
//   1. [skip ci] / [ci skip] / [no ci] (:2657) sets SKIP_CI; nothing is dispatched (:2758, :2786,
//      :2877, :2927). The post block still archives baselines and a green run advances __global__
//      to that tip (:204-230, :3512), so the next run's diff starts past the push. A later change
//      that selects the same pipeline still builds its bytes; a failed or aborted run keeps the
//      old baseline (:217-224).
//   2. [edge:validate-only] (:2711) dispatches elohim-edge alone (:2841-2844) as verify (:1286),
//      over [build:edge] and [deploy-only]; Edge runs it validate-only (Edge :2551-2568).
//   3. [deploy-only] (webhook-gated, :2722-2726): the dispatch becomes Edge (and Genesis)
//      (:2828-2834) with DEPLOY_ONLY, so Edge skips its build stages and runs its deploy stage
//      (:1331, Edge :2586-2595), unless its class from PIPELINE_RUN_CLASSES (:1962-1964) is below
//      deploy; an Edge absent from that list reads as build (:931-934), and the already-built
//      filter (:1912) can remove a validate-only Edge before the list is written. The guard
//      cannot see that filter, so a tip with [deploy-only] always asks for the declaration.
//   4. [build:…] tokens (:907): `edge` forces elohim-edge in as build (:1886-1896, :1913). `all`
//      is expanded at Checkout (:2688-2693) against PIPELINE_REGISTRY_JSON, which runBuildGraph
//      fills only later (:2413, :1855), so it forces nothing on a fresh run; the guard still asks
//      for the declaration when a protected tip carries it (steward ruling 2026-11-19).
//   5. With no forcing tag, an Edge whose only stale step is dataplane-validation is verify
//      (:1874-1876); a class below deploy (the manifest-only cap) is validate-only too (:1877).
//   6. [run:verify|measure|profile] (:1915-1925) keeps only pipelines declaring a step at or below
//      the class, plus [build:*] force-includes; no Edge step declares a class, so Edge leaves.
// - Branches: only dev and main deploy. Off them, and for a PR build, Edge is dispatched
//   gate-only (:961-967, :1288; Edge isGateOnly :2598). dev deploys alpha (Edge :3270-3276); main
//   deploys prod (Edge :3538) and, dispatched with FORCE_DEPLOY, alpha too.
// - Submodule pins: a dependency on a pipeline whose manifest CI cannot see is pruned
//   (build-graph.groovy:104-119), so a brit or rakia pin move selects no Edge step in CI today; a
//   WARN row says so, decided from the pushed tree alone (the gitlink, its .gitmodules name and the
//   pruned pipeline names; never a local checkout), and names the Edge steps reached through those
//   pruned dependencies.
// - Unknown `[edge:…]` tags are inert: the orchestrator matches `[edge:validate-only]` (:2711) and
//   `[edge:gate-only]` (:944) literally; Edge matches `[edge:validate-only]`, `[compute-worker:…]`
//   and `[conductor-roll]` literally (Edge :2556, :2558, :2618, :1930).

import { spawnSync } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { basename, dirname, join, posix, resolve } from 'node:path';
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
const POLICY_PATH = 'genesis/agentic/pool-policy.json';
const MANIFEST = 'build-manifest.json';
// build-graph.groovy:27 — `find … -not -path '*/<dir>/*'` for each of these.
const CI_SKIPPED_DIRS = new Set(['node_modules', '.superpowers', '.claude', 'local-dev']);
const BUFFER = 64 * 1024 * 1024;
const BASELINE = "once the run ends green, the next run's diff starts past this push; a later change that selects the same pipeline still builds these bytes; a failed or aborted run keeps the old baseline";
const short = sha => String(sha ?? '').slice(0, 12);
const isZero = sha => /^0+$/.test(String(sha ?? ''));
const isDeploying = branch => DEPLOYING_BRANCHES.includes(branch);
const branchOf = ref => (String(ref).startsWith('refs/heads/') ? String(ref).slice('refs/heads/'.length) : null);
const row = (check, level, why, cure = '') => ({ check, level, why, cure });
const words = text => String(text ?? '').split(/\s+/).filter(Boolean);
const lines = text => String(text).split('\n').map(l => l.trim()).filter(Boolean);

// ── Policy ──────────────────────────────────────────────────────────────────

/** `fleet_roll_guard.mode` from one policy file's text; `where` names it in a default's reason. */
export function parsePolicy(text, where) {
  if (text == null) return { mode: 'refuse', why: `${where} has no ${POLICY_PATH}` };
  let policy;
  try { policy = JSON.parse(text); } catch { return { mode: 'refuse', why: `${where}'s ${POLICY_PATH} is not valid JSON` }; }
  const mode = policy?.fleet_roll_guard?.mode;
  if (mode === undefined) return { mode: 'refuse', why: `${where}'s ${POLICY_PATH} declares no fleet_roll_guard.mode` };
  if (!MODES.includes(mode)) return { mode: 'refuse', why: `fleet_roll_guard.mode ${JSON.stringify(mode)} is not refuse, warn or off` };
  return { mode, why: null, source: `${where}'s ${POLICY_PATH}` };
}

/** The working tree's policy: what `just gate` and other runs outside a push read. */
export function fleetRollMode(root = MODULE_ROOT) {
  let text;
  try { text = readFileSync(join(root, POLICY_PATH), 'utf8'); }
  catch (error) { return { mode: 'refuse', why: `${POLICY_PATH} could not be read (${error.code || error.message})` }; }
  return parsePolicy(text, 'this checkout');
}

function policyNote({ mode, why, source }) {
  if (why) return ` (policy: mode ${mode}, the default, because ${why})`;
  if (mode !== 'refuse' && source) return ` (policy: mode ${mode}, declared in ${source})`;
  return '';
}

// ── Git objects ─────────────────────────────────────────────────────────────

function gitError(message, cure = 'the git error above names what failed; correct it and push again') {
  return Object.assign(new Error(message), { cure });
}
function git(spawn, root, args) {
  const r = spawn('git', ['-C', root, ...args], { encoding: 'utf8', maxBuffer: BUFFER });
  if (r.error || r.status !== 0) throw gitError(`git ${args.join(' ')} exited ${r.status ?? r.error?.code}: ${String(r.stderr || r.error?.message || '').trim().split('\n')[0]}`);
  return String(r.stdout);
}
const hasCommit = (spawn, root, sha) => spawn('git', ['-C', root, 'cat-file', '-e', `${sha}^{commit}`], { encoding: 'utf8' }).status === 0;

const TOO_LARGE = 'larger than 64 MiB; this check cannot judge a tree that large';
function spawnFailure(r, what) {
  if (r.error?.code === 'ENOBUFS') return gitError(`${what}: the output is ${TOO_LARGE}`);
  return gitError(`${what} exited ${r.status ?? r.error?.code}: ${String(r.stderr || r.error?.message || '').trim().split('\n')[0]}`);
}

/**
 * `git cat-file --batch` by object id only (a path is never sent, so no path can shift another's
 * response). Each response is checked: the header names the requested id, its type and size, the
 * body is followed by a newline, and nothing follows the last response. `<id> missing` is null; a
 * non-blob is null.
 */
export function catBlobs(spawn, root, oids) {
  const out = new Map();
  const wanted = [...new Set(oids)];
  if (!wanted.length) return out;
  for (const oid of wanted) if (!/^([0-9a-f]{40}|[0-9a-f]{64})$/.test(oid)) throw gitError(`not an object id: ${JSON.stringify(oid)}`);
  const r = spawn('git', ['-C', root, 'cat-file', '--batch'], { input: `${wanted.join('\n')}\n`, maxBuffer: BUFFER });
  if (r.error || r.status !== 0) throw spawnFailure(r, 'git cat-file --batch');
  const buf = Buffer.isBuffer(r.stdout) ? r.stdout : Buffer.from(String(r.stdout));
  let pos = 0;
  for (const oid of wanted) {
    const nl = buf.indexOf(0x0a, pos);
    if (nl < 0) throw gitError(`git cat-file --batch gave no response for ${oid}`);
    const header = buf.subarray(pos, nl).toString('utf8');
    pos = nl + 1;
    if (header === `${oid} missing`) { out.set(oid, null); continue; }
    const m = /^([0-9a-f]{40}|[0-9a-f]{64}) (\S+) (\d+)$/.exec(header);
    if (!m || m[1] !== oid) throw gitError(`git cat-file --batch answered "${header}" for ${oid}`);
    const size = Number(m[3]);
    if (pos + size >= buf.length || buf[pos + size] !== 0x0a) throw gitError(`git cat-file --batch returned a short object for ${oid}`);
    out.set(oid, m[2] === 'blob' ? buf.subarray(pos, pos + size).toString('utf8') : null);
    pos += size + 1;
  }
  if (pos !== buf.length) throw gitError(`git cat-file --batch left ${buf.length - pos} bytes after the last response`);
  return out;
}

/** One file's text at a commit-ish, its id found with ls-tree; null when the path or ref is absent. */
function blobAt(spawn, root, commitish, path) {
  const r = spawn('git', ['-C', root, 'ls-tree', '-z', commitish, '--', path], { encoding: 'utf8' });
  if (r.error || r.status !== 0) return null;
  const entry = String(r.stdout).split('\0').find(e => e.endsWith(`\t${path}`));
  const oid = entry?.slice(0, entry.indexOf('\t')).split(' ')[2];
  return oid ? catBlobs(spawn, root, [oid]).get(oid) ?? null : null;
}

class Indeterminate extends Error {}

/**
 * The build graph CI's checkout would hold at `sha`: every build-manifest.json in the commit's own
 * superproject tree outside the directories CI's find skips, parsed; the gitlinks; the .gitmodules
 * names; and a reader of build-process files from the same tree, every read by object id from the
 * listing. A symlinked manifest cannot be decided here. A symlinked build-process file is read
 * through its target once, inside the tree; a target outside the tree, absent, or itself a link
 * reads as unreadable, with that reason.
 */
export function loadTree(spawn, root, sha) {
  const r = spawn('git', ['-C', root, 'ls-tree', '-r', '-z', '--full-tree', sha], { encoding: 'utf8', maxBuffer: BUFFER });
  if (r.error || r.status !== 0) throw spawnFailure(r, `git ls-tree ${sha}`);
  const index = new Map();
  const gitlinks = new Set();
  const blobs = [];
  for (const entry of String(r.stdout).split('\0').filter(Boolean)) {
    const tab = entry.indexOf('\t');
    const [mode, type, oid] = entry.slice(0, tab).split(' ');
    const path = entry.slice(tab + 1);
    if (type === 'commit') { gitlinks.add(path); continue; }
    index.set(path, { mode, type, oid });
    if (basename(path) !== MANIFEST || path.split('/').slice(0, -1).some(d => CI_SKIPPED_DIRS.has(d))) continue;
    if (mode === '120000') throw new Indeterminate(`${path} is a symlink in the pushed tree; CI's find lists it and its readFile follows it, which this check does not do`);
    blobs.push({ path, oid });
  }
  const contents = catBlobs(spawn, root, blobs.map(b => b.oid));
  const manifests = [];
  const errors = [];
  for (const { path, oid } of blobs.sort((a, b) => a.path.localeCompare(b.path))) {
    try { manifests.push({ path: `./${path}`, content: JSON.parse(contents.get(oid)) }); }
    catch (error) { errors.push(`./${path}: ${String(error.message).split('\n')[0]}`); }
  }
  const norm = file => posix.normalize(String(file)).replace(/^\.\//, '');
  const processFiles = [...new Set(manifests.flatMap(m => Object.values(m.content.steps || {})
    .flatMap(s => (s.inputs?.buildProcess || []).map(ref => norm(String(ref).split('@')[0])))))];
  const first = catBlobs(spawn, root, [...processFiles, '.gitmodules'].map(f => index.get(f)?.oid).filter(Boolean));
  const targets = new Map();
  for (const file of processFiles) {
    const entry = index.get(file);
    if (entry?.mode !== '120000') continue;
    const link = first.get(entry.oid) ?? '';
    const resolved = posix.normalize(posix.join(posix.dirname(file), link));
    if (posix.isAbsolute(link) || resolved.startsWith('../') || resolved === '..') targets.set(file, { why: `a symlink to ${link}, outside the tree` });
    else if (!index.has(resolved)) targets.set(file, { why: `a symlink to ${resolved}, which is not in the tree` });
    else if (index.get(resolved).mode === '120000') targets.set(file, { why: `a symlink to ${resolved}, itself a symlink` });
    else targets.set(file, { oid: index.get(resolved).oid });
  }
  const second = catBlobs(spawn, root, [...targets.values()].map(t => t.oid).filter(Boolean));
  const readFile = file => {
    const path = norm(file);
    const entry = index.get(path);
    if (!entry) return null;
    if (entry.mode !== '120000') return first.get(entry.oid) ?? null;
    const target = targets.get(path);
    return target.why ? { why: target.why } : (second.get(target.oid) ?? null);
  };
  const gitmodules = new Map();
  let name = null;
  for (const line of String(first.get(index.get('.gitmodules')?.oid) ?? '').split('\n')) {
    const section = /^\s*\[submodule\s+"(.+)"\]\s*$/.exec(line);
    if (section) { name = section[1]; continue; }
    const path = /^\s*path\s*=\s*(.+?)\s*$/.exec(line);
    if (path && name) gitmodules.set(path[1], name);
  }
  return { manifests, errors, gitlinks, gitmodules, readFile };
}

// ── Prediction ──────────────────────────────────────────────────────────────

function buildTokens(tip) {
  const tokens = [];
  for (const m of tip.matchAll(/\[build:([a-z,-]+)\]/gi)) tokens.push(...m[1].split(',').map(t => t.trim().toLowerCase()));
  return tokens;
}

const dispatchable = (meta, name) => name !== SELF_PIPELINE && !!meta?.[name]?.jenkinsPath && !meta[name].manualOnly;

/** What CI does with elohim-edge for this walk and tip (header, "THE CI RULE"). Pure. */
export function predict(walked, tip = '') {
  const meta = walked.meta || {};
  // CI dispatches Edge on a jenkinsPath alone (:1886-1893, :1269); manual-only steps already left
  // the graph walk, so a manual-only Edge is reached only by a forcing or mode tag.
  const edgeOk = !!meta?.[EDGE]?.jenkinsPath;
  const edge = edgeOk ? walked.pipelines.find(p => p.name === EDGE) : undefined;
  const buildSteps = (edge?.steps ?? []).filter(s => s.name !== VALIDATE_STEP);
  const others = walked.pipelines.map(p => p.name).filter(name => name !== EDGE && dispatchable(meta, name));
  const skipTag = /\[(skip ci|ci skip|no ci)\]/i.exec(tip)?.[0] ?? null;
  const validateOnly = /\[edge:validate-only\]/i.test(tip);
  const tokens = buildTokens(tip);
  const forcedEdge = edgeOk && tokens.includes('edge');
  const buildAll = tokens.includes('all');
  const runTag = parseRunClassTag(tip);
  const deployOnly = /\[deploy[-_ ]only\]/i.test(tip);
  let graphClass = edge ? edge.runClass : null;
  if (edge && edge.steps.every(s => s.name === VALIDATE_STEP)) graphClass = 'verify';
  const pathSelects = graphClass !== null && runClassRank(graphClass) >= runClassRank('deploy');
  let edgeClass = null;
  if (!skipTag && validateOnly && edgeOk) edgeClass = 'verify';
  else if (!skipTag && !validateOnly) {
    edgeClass = forcedEdge ? 'build' : graphClass;
    if (runTag && !forcedEdge) {
      const declares = Object.values(meta[EDGE]?.stepClasses || {}).some(c => runClassRank(c) <= runClassRank(runTag));
      edgeClass = declares && edge ? runTag : null;
    }
  }
  const deployAsk = deployOnly && !skipTag && !validateOnly && edgeOk;
  const selects = deployAsk || (edgeClass !== null && runClassRank(edgeClass) >= runClassRank('deploy'));
  const asks = buildAll && !selects && !skipTag && !validateOnly;
  const forcedBy = forcedEdge ? (tip.match(/\[build:[a-z,-]+\]/gi) || []).filter(t => buildTokens(t).includes('edge')).join(' ') : null;
  return { edge, buildSteps, others, skipTag, validateOnly, forcedBy, buildAll, runTag, deployOnly, deployAsk, edgeClass, pathSelects, selects, asks };
}

function edgeText(p) {
  if (!p.buildSteps.length) return '';
  const causes = [...new Set(p.buildSteps.map(s => s.cause))];
  const through = [...new Set(p.buildSteps.map(s => s.via).filter(v => v && !v.startsWith(`${EDGE}:`)))];
  const more = causes.length > 3 ? ` and ${causes.length - 3} more` : '';
  return `${EDGE} stale steps ${p.buildSteps.map(s => s.name).join(', ')}, from ${causes.slice(0, 3).join(', ')}${more}${through.length ? `, through ${through.join(', ')}` : ''}`;
}

function reachText(p, where) {
  if (p.deployAsk && p.forcedBy) return `the tip's ${p.forcedBy} and [deploy-only] make CI dispatch ${EDGE} with DEPLOY_ONLY: Edge skips its build stages and runs its deploy step on this ${where}`;
  if (p.deployAsk) {
    return `the tip's [deploy-only] asks CI for the Edge deploy step on this ${where}: CI runs it unless it dispatches Edge validate-only, and CI's already-built filter, which this check cannot see, can drop a validate-only Edge that [deploy-only] then re-adds as build and deploys (orchestrator Jenkinsfile:1912, :1962-1964, :2828-2834, :931-934)`;
  }
  if (p.forcedBy) return `the tip's ${p.forcedBy} makes CI select an Edge build and its deploy step for this ${where}`;
  if (p.selects) return `CI would select an Edge build and its deploy step for this ${where}`;
  return `the tip's [build:all] asks for every pipeline, Edge included, on this ${where}: CI forces nothing with it today (it expands [build:all] at Checkout against a pipeline registry filled only later, orchestrator Jenkinsfile:2688-2693, :2413), but it is read here as asking, so the selection is asked to be declared`;
}

function cureFor(p) {
  const parts = [];
  const tags = [p.forcedBy, p.deployAsk ? '[deploy-only]' : null, p.buildAll && !p.skipTag && !p.validateOnly ? '[build:all]' : null].filter(Boolean);
  if (tags.length) {
    const requests = tags.length + (p.pathSelects ? 1 : 0);
    parts.push(`drop ${tags.join(' and ')} from the tip commit${p.pathSelects ? ' and take the Edge build inputs out of this push' : ''}${requests > 1 ? ' (each one left in place still asks)' : ''}`);
  } else if (p.buildSteps.length) {
    parts.push('take the Edge build inputs out of this push, or land them with the next deliberate roll');
  }
  parts.push(`or declare the selection: add ${ROLL_TAG} to a pushed commit message, with the reason (asked for, not checked)`);
  return `${parts.join('; ')}. [edge:validate-only] on the tip also stops the selection, but it drops every other pipeline from that run, and once the run ends green the next run's diff starts past this push: not a cure for a mixed push.`;
}

/**
 * The rows for one ref: `fleet-roll` and, on a protected ref with a skip tag, `skip-ci`. Pure.
 * @param {{target: string|null, walked: object, tip?: string, declaredBy?: string|null,
 *   mode?: string, policyWhy?: string|null, policySource?: string|null}} args
 */
export function evaluateRef({ target, walked, tip = '', declaredBy = null, mode = 'refuse', policyWhy = null, policySource = null, historyCut = null }) {
  if (mode === 'off') return [];
  const deploying = isDeploying(target);
  const where = deploying ? `push to ${target}` : `change, were it pushed to dev (${target ? `target ${target} does not deploy` : 'not a push'})`;
  const level = l => (l === 'REFUSED' && (mode === 'warn' || !deploying) ? 'WARN' : l);
  const note = policyNote({ mode, why: policyWhy, source: policySource });
  const rows = [];
  const add = r => rows.push({ ...r, why: `${r.why}${note}` });
  if (walked.errors?.length) {
    add(row(CHECK, level('REFUSED'), `the build graph at the pushed commit is not valid: ${walked.errors[0]}; CI stops there too (build-graph.groovy:815-827), so no selection is predicted`,
      'correct the build manifests named above; CI cannot plan this push either'));
    return rows;
  }
  const p = predict(walked, tip);
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
      add(row(CHECK, 'WARN', `the tip's [edge:validate-only] makes ${EDGE} validate only for this ${where}: ${parts.join(', and ')}; ${BASELINE}`,
        'push those changes under a tip without [edge:validate-only], and keep the measuring commit on its own push'));
    } else if (p.edge) {
      add(row(CHECK, 'ok', `the tip's [edge:validate-only] makes ${EDGE} validate only for this ${where}; no build, no deploy`));
    }
  } else if (p.selects || p.asks) {
    const what = `${reachText(p, where)}${inputs ? `: ${inputs}` : ''}`;
    if (declaredBy) add(row(CHECK, 'ok', `${what} — declared by ${ROLL_TAG} in ${short(declaredBy)}`));
    else if (historyCut) {
      add(row(CHECK, level('REFUSED'),
        `${what} — and whether a pushed commit carries ${ROLL_TAG} cannot be decided: this clone is shallow and the range reaches its boundary at ${short(historyCut.commit)}, so a declaration further back is not visible`,
        `fetch enough history (git fetch --deepen=<n> ${historyCut.remote} or git fetch --unshallow ${historyCut.remote}), then push again`));
    } else {
      add(row(CHECK, level('REFUSED'),
        `${what} — and no pushed commit carries ${ROLL_TAG}. Read from ${target ? "this ref's own range and its tip's tags" : 'the changed paths given, with no commit read'}; an Edge step left stale by an earlier landing is not seen`,
        cureFor(p)));
    }
  } else if (p.edge) {
    const what = p.runTag ? `the tip's [run:${p.runTag}] keeps ${EDGE} out of the dispatch` : `${EDGE} validates only (${p.edge.steps.map(s => s.name).join(', ')})`;
    add(row(CHECK, 'ok', `${what}; CI selects no Edge build or deploy for this ${where}`));
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

/**
 * One WARN per moved gitlink P, decided from the pushed tree alone: an Edge build step depends on
 * a pipeline no manifest in that tree defines (CI prunes it) and whose name P's path or its
 * .gitmodules name gives (`<name>` or `elohim-<last segment>`). The hypothetical steps are those
 * reached from P through those pruned dependencies only.
 */
function pinRows({ tree, changed }) {
  const edge = tree.manifests.find(m => m.content.pipeline === EDGE);
  if (!edge) return [];
  const defined = new Set(tree.manifests.map(m => m.content.pipeline));
  const pruned = Object.values(edge.content.steps || {}).flatMap(s => s.depends ?? [])
    .filter(d => String(d).includes(':') && !defined.has(String(d).split(':')[0]));
  const rows = [];
  for (const path of changed.filter(p => tree.gitlinks.has(p))) {
    const names = new Set([path, tree.gitmodules.get(path)].filter(Boolean).flatMap(n => [basename(n), `elohim-${basename(n)}`]));
    const hits = pruned.filter(d => names.has(d.split(':')[0]));
    if (!hits.length) continue;
    const pipelines = [...new Set(hits.map(d => d.split(':')[0]))];
    const synthetic = pipelines.map(pipeline => ({ path: `./(pruned)/${pipeline}`, content: { pipeline,
      steps: Object.fromEntries(hits.filter(d => d.startsWith(`${pipeline}:`)).map(d => [d.split(':')[1], { inputs: { sources: [path] }, depends: [] }])) } }));
    const would = walkGraph([...tree.manifests, ...synthetic], [path], { ci: true, readFile: tree.readFile });
    const selected = (would.pipelines.find(p => p.name === EDGE)?.steps ?? []).filter(s => s.name !== VALIDATE_STEP).map(s => s.name);
    if (!selected.length) continue;
    rows.push(row(CHECK, 'WARN',
      `this push moves the ${path} pin, a submodule whose manifest CI's checkout cannot see (it initialises no submodule, orchestrator Jenkinsfile:2587-2609); ${EDGE}'s build steps depend on ${pipelines.join(', ')}, a pipeline no manifest in the pushed tree defines, which CI prunes (build-graph.groovy:104-119), so this pin move selects no Edge step today; if CI ever saw that manifest watching ${path}, this push would also select ${selected.join(', ')}`,
      `nothing to do for today's CI; a pin move meant to reach the fleet needs a deliberate Edge build with ${ROLL_TAG}`));
  }
  return rows;
}

/**
 * The rows for `changed` judged against the build graph at commit `sha` (git objects). Used for
 * every pushed ref, and directly by the real-manifest probes.
 */
export function evaluatePaths({ root = MODULE_ROOT, sha, changed, target, tip = '', declaredBy = null, historyCut = null, spawn = spawnSync, policy = { mode: 'refuse', why: null } }) {
  if (policy.mode === 'off') return [];
  const tree = loadTree(spawn, root, sha);
  const walked = tree.errors.length ? { pipelines: [], meta: {}, errors: tree.errors } : walkGraph(tree.manifests, changed, { ci: true, readFile: tree.readFile });
  const args = { target, tip, declaredBy, historyCut, mode: policy.mode, policyWhy: policy.why, policySource: policy.source };
  const rows = evaluateRef({ walked, ...args });
  if (walked.errors?.length) return rows;
  const note = policyNote({ mode: policy.mode, why: policy.why, source: policy.source });
  return [...rows, ...pinRows({ tree, changed }).map(r => ({ ...r, why: `${r.why}${note}` }))];
}

// ── Pushes ──────────────────────────────────────────────────────────────────

/** `GATE_PUSH_REFS`: one `<remote ref> <pushed sha> <destination sha>` per line (pre-push.bash). */
export function parseRefs(text) {
  return String(text ?? '').split('\n').map(line => line.trim()).filter(Boolean).map(line => {
    const [ref, local, remote] = line.split(/\s+/);
    return { ref, local, remote, branch: branchOf(ref) };
  });
}

/** A failure to decide: red on a protected ref under refuse, a WARN otherwise; never silent. */
export function failureRows({ error, deploying, mode = 'refuse', policyWhy = null, policySource = null }) {
  if (mode === 'off') return [];
  const first = String(error?.message || error).split('\n')[0];
  return [row(CHECK, mode === 'refuse' && deploying ? 'REFUSED' : 'WARN',
    `the fleet-roll check could not decide: ${first}${policyNote({ mode, why: policyWhy, source: policySource })}`,
    error?.cure || 'fix what the error names and push again (a malformed build-manifest.json also stops CI\'s own graph, build-graph.groovy:815-827)')];
}

/**
 * In a shallow clone, `git log base..tip` stops at the boundary and still exits 0. The range
 * reaches the boundary when one of its commits is a shallow root; then a declaration further back
 * is not visible. Returns { commit, remote } or null.
 */
function shallowCut(spawn, root, base, tip, remote) {
  if (git(spawn, root, ['rev-parse', '--is-shallow-repository']).trim() !== 'true') return null;
  const shallowFile = resolve(root, git(spawn, root, ['rev-parse', '--git-path', 'shallow']).trim());
  let roots;
  try { roots = new Set(lines(readFileSync(shallowFile, 'utf8'))); } catch { return null; }
  const hit = lines(git(spawn, root, ['rev-list', `${base}..${tip}`])).find(c => roots.has(c));
  return hit ? { commit: hit, remote } : null;
}

function judgeRef({ ref, spawn, root, remoteName }) {
  const deploying = isDeploying(ref.branch);
  const target = ref.branch ?? ref.ref;
  const where = `push to ${target}`;
  let policy = { mode: 'refuse', why: null };
  try {
    let base = ref.remote;
    if (isZero(ref.remote) || !hasCommit(spawn, root, ref.remote)) {
      if (deploying) {
        throw Object.assign(new Indeterminate(isZero(ref.remote) ? `this push creates ${target}, so there is no destination tip to read a range from`
          : `the destination tip ${short(ref.remote)} is not in this clone, so this ref's range cannot be read`),
        { cure: isZero(ref.remote) ? `this check has no range to read for a new ${target}: create it deliberately, outside an ordinary push` : `git fetch ${remoteName} ${target}, then push again` });
      }
      base = git(spawn, root, ['merge-base', `${remoteName}/dev`, ref.local]).trim();
    } else {
      policy = parsePolicy(blobAt(spawn, root, ref.remote, POLICY_PATH), `${target}'s tip ${short(ref.remote)}`);
      if (!policy.why) policy.source = `${target}'s ${POLICY_PATH} at ${short(ref.remote)}`;
    }
    if (policy.mode === 'off') return [];
    // Explicit options and NUL separators: no local diff setting (ignoreSubmodules, renames,
    // relative, quotePath) changes which paths this reads.
    const changed = git(spawn, root, ['diff', '--name-only', '-z', '--no-renames', '--ignore-submodules=none', '--no-relative', '--no-ext-diff', base, ref.local]).split('\0').filter(Boolean);
    // The tip alone carries CI's tags; the declaration is found with a search that returns one commit.
    const tip = git(spawn, root, ['log', '-1', '--no-show-signature', '--format=%B', ref.local]);
    const declaredBy = git(spawn, root, ['log', '--no-show-signature', '-i', '--fixed-strings', `--grep=${ROLL_TAG}`, '-1', '--format=%H', `${base}..${ref.local}`]).trim() || null;
    const historyCut = declaredBy ? null : shallowCut(spawn, root, base, ref.local, remoteName);
    return evaluatePaths({ root, sha: ref.local, changed, target, tip, declaredBy, historyCut, spawn, policy });
  } catch (error) {
    if (error instanceof Indeterminate) {
      return [row(CHECK, policy.mode === 'warn' || !deploying ? 'WARN' : 'REFUSED',
        `${where}: ${error.message} — the check cannot decide, and an unreadable range never reads as "nothing changed"`,
        error.cure || 'replace the symlinked build-manifest.json with a regular file in the pushed commit; this check cannot load such a tree, so a declaration cannot be honoured either')];
    }
    return failureRows({ error, deploying, mode: policy.mode, policyWhy: policy.why, policySource: policy.source });
  }
}

/** Whether any pushed ref named by the environment is dev or main. */
export function pushDeploys(env) {
  return [...words(env.GATE_PUSH_TARGETS), ...parseRefs(env.GATE_PUSH_REFS).map(r => r.ref)].some(ref => isDeploying(branchOf(ref)));
}

/**
 * The early hook block's mark: the --fleet-roll run writes the exact ref list it judged to the
 * file GATE_FLEET_ROLL_MARK names (a temporary file the hook removes on exit). The ordinary
 * preflight skips the check only when that file holds this push's ref list.
 */
export function fleetRollMarkHonoured(env = process.env) {
  if (!env.GATE_FLEET_ROLL_MARK || !env.GATE_PUSH_REFS) return false;
  try { return readFileSync(env.GATE_FLEET_ROLL_MARK, 'utf8') === env.GATE_PUSH_REFS; } catch { return false; }
}

const OLDER_HOOK_CURE = 'check the launcher first: test -x <worktree>/.husky/_/pre-push (pnpm install creates it; with it missing, git runs no hook at all, silently); then push with -c core.hooksPath=<worktree>/.husky/_ so the current hook reads each ref\'s range and messages, or bring the main checkout up to date (its .husky/pre-push.bash predates the fleet-roll block)';

function olderHook({ targets, changed, root, spawn, remoteName }) {
  const target = targets.map(branchOf).find(isDeploying) ?? branchOf(targets[0]) ?? targets[0];
  const deploying = isDeploying(target);
  let policy = { mode: 'refuse', why: null };
  try {
    policy = parsePolicy(blobAt(spawn, root, `${remoteName}/${target}`, POLICY_PATH), `${remoteName}/${target}`);
    if (policy.mode === 'off') return [];
    if (!Array.isArray(changed)) throw Object.assign(new Error('the running pre-push hook handed over no changed-path list'), { cure: OLDER_HOOK_CURE });
    const tip = git(spawn, root, ['log', '-1', '--no-show-signature', '--format=%B', 'HEAD']);
    const tree = loadTree(spawn, root, 'HEAD');
    const walked = tree.errors.length ? { pipelines: [], meta: {}, errors: tree.errors } : walkGraph(tree.manifests, changed, { ci: true, readFile: tree.readFile });
    if (walked.errors?.length) return evaluateRef({ target, walked, mode: policy.mode, policyWhy: policy.why, policySource: policy.source });
    const p = predict(walked, tip);
    if (!p.selects && !p.asks) return [];
    const inputs = edgeText(p);
    return [row(CHECK, policy.mode === 'warn' || !deploying ? 'WARN' : 'REFUSED',
      `${reachText(p, `push to ${target}`)}${inputs ? `: ${inputs}` : ''} — and the running pre-push hook hands over no ref list (GATE_PUSH_REFS), so this ref's range and any declaration cannot be read; HEAD's message was read for CI's tags, which is right only when HEAD is the pushed commit${policyNote(policy)}`,
      OLDER_HOOK_CURE)];
  } catch (error) {
    return failureRows({ error, deploying, mode: policy.mode, policyWhy: policy.why, policySource: policy.source });
  }
}

// ── Outside a push ──────────────────────────────────────────────────────────

/**
 * This checkout's working-tree manifests, as CI's find would list them in a checkout without
 * submodules: no manifest inside a directory that holds its own `.git`, none under the
 * directories CI skips. A file that does not parse is named in the error.
 */
function workingTreeGraph(root) {
  const inSubmodule = rel => {
    for (let dir = dirname(rel); dir && dir !== '.'; dir = dirname(dir)) if (existsSync(join(root, dir, '.git'))) return true;
    return false;
  };
  const manifests = [];
  const errors = [];
  for (const path of discoverManifests(root)) {
    const rel = path.replace(/^\.[/\\]/, '');
    if (rel.split('/').slice(0, -1).some(d => CI_SKIPPED_DIRS.has(d)) || inSubmodule(rel)) continue;
    try { manifests.push({ path, content: JSON.parse(readFileSync(resolve(root, path), 'utf8')) }); }
    catch (error) { errors.push(`${path}: ${String(error.message).split('\n')[0]}`); }
  }
  const readFile = file => { try { return readFileSync(join(root, file), 'utf8'); } catch { return null; } };
  return { manifests, errors, readFile };
}

/**
 * The preflight entry. Under pre-push: each protected ref judged from its own range and objects
 * (else the first ref, advisory). Outside a push (`just gate`, `just status gates`): the changed
 * list against this checkout's working tree, as if pushed to dev, never red. Returns rows, or null.
 */
export function checkFleetRoll({ env = process.env, changed = null, root = MODULE_ROOT, spawn = spawnSync } = {}) {
  const targets = words(env.GATE_PUSH_TARGETS);
  const remoteName = env.GATE_PUSH_REMOTE || 'origin';
  if (!targets.length && env.GATE_PUSH_REFS === undefined) {
    if (!Array.isArray(changed)) return null;
    const policy = fleetRollMode(root);
    if (policy.mode === 'off') return null;
    try {
      const tree = workingTreeGraph(root);
      const walked = tree.errors.length ? { pipelines: [], meta: {}, errors: tree.errors } : walkGraph(tree.manifests, changed, { ci: true, readFile: tree.readFile });
      return evaluateRef({ target: null, walked, mode: policy.mode, policyWhy: policy.why, policySource: policy.source });
    } catch (error) {
      return failureRows({ error, deploying: false, mode: policy.mode, policyWhy: policy.why });
    }
  }
  if (env.GATE_PUSH_REFS === undefined) return olderHook({ targets, changed, root, spawn, remoteName });
  const refs = parseRefs(env.GATE_PUSH_REFS);
  const named = [...new Set(refs.map(r => r.ref))].sort().join(' ');
  if (named !== [...new Set(targets)].sort().join(' ')) {
    return [row(CHECK, pushDeploys(env) ? 'REFUSED' : 'WARN',
      `the push names refs [${targets.join(' ')}] that its ref list GATE_PUSH_REFS [${refs.map(r => r.ref).join(' ')}] does not match — the check cannot decide`,
      'push again from a plain shell: an inherited GATE_PUSH_REFS or GATE_PUSH_TARGETS is the usual cause')];
  }
  const judged = refs.filter(r => isDeploying(r.branch));
  return (judged.length ? judged : refs.slice(0, 1)).flatMap(ref => judgeRef({ ref, spawn, root, remoteName }));
}
