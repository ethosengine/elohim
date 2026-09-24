/**
 * Already-built dispatch filter — "this pipeline's own baseline is newer than
 * the global diff base, nothing it watches has changed since, and its artifact
 * is actually there, so building it again buys no information".
 *
 * WHY THIS EXISTS (2026-09-22). The orchestrator computes ONE changeset,
 * `git diff <__global__>..HEAD`, and feeds it to the graph walker. But
 * `archivePipelineBaselines` deliberately refuses to advance `__global__` on a
 * FAILURE/ABORTED orchestrator run, while `recordPipelineResult` DOES advance
 * a per-pipeline baseline whenever that downstream finished non-red. So a
 * string of red orchestrator runs freezes the global diff base while the
 * per-pipeline record keeps moving — and every later run re-diffs a range that
 * has already been built and re-dispatches the whole cascade.
 *
 * Lived, measured twice:
 *   - #1886 (TIMER, 2026-09-21 09:00Z) loaded `__global__: 82dcb5c8` beside
 *     `elohim-edge: bc81e107` — the edge baseline already EQUALLED that run's
 *     own HEAD — and still dispatched elohim-edge/dev #1471, rebuilding the
 *     images edge #1470 had shipped SUCCESS the day before: 3h04m of fleet roll.
 *   - #1888 (WEBHOOK, HEAD 7d27b04c, 2026-09-22 04:51Z) loaded
 *     `__global__: 82dcb5c8` beside `elohim-holochain: 86e7c220` (frozen
 *     global: #1884–#1887 all died on the 4h timeout). `86e7c220..7d27b04c`
 *     touches NOTHING under `elohim/holochain`, but `82dcb5c8..7d27b04c` does,
 *     so the ~53-minute DNA pipeline was dispatched with FORCE_BUILD=true for
 *     zero information. Not an exact-sha match, not a TIMER.
 *
 * Every redundant DNA/edge dispatch rolls the alpha fleet, and every roll
 * restarts the conductor pods and leaves storage cells answering CellDisabled
 * for 1–6 h per peer, during which no app build can author heads. The cost of
 * a false dispatch is a write outage; the cost of a false SKIP is a missing
 * build — or, worse, a consumer built against an artifact that was never
 * published. Hence the asymmetries below.
 *
 * ── THIS MODULE DOES NO GLOB MATCHING. ──────────────────────────────────────
 * The narrow walk MUST have the plan stage's semantics by construction, so the
 * decision is split and the WALKER STAYS IN GROOVY:
 *
 *   1. `groups <state>` (here) — pure git + manifest reads. For every pipeline
 *      whose baseline is a strict descendant of `__global__`, resolvable and
 *      unforced, emit its `git diff --name-only <B_P>..HEAD` file list, grouped
 *      by baseline sha, plus each pipeline's baseline PROVENANCE.
 *   2. The Jenkinsfile (`walkNarrowGroups`) then calls the SAME
 *      `build-graph.groovy::walkBuildGraph(changedFiles)` the plan stage
 *      called, once per group, and writes each group's dispatch set out.
 *   3. `decide <state> <groups> <walks>` (here) — a pipeline may be skipped
 *      only when THAT GROOVY RESULT excludes it, and only after the
 *      producer-readiness rule below.
 *
 * An earlier cut re-implemented matching in JS over `picomatch`. Two defects
 * killed it: picomatch excludes dot segments by default while Groovy's
 * `matchesGlob()` includes them (a changed `doorway/doorway-service/src/.hidden.rs`
 * selected edge in the plan and vanished from the narrow walk, so edge was
 * suppressed WITH changed source), and `picomatch` is a node_modules dependency
 * that the orchestrator's clean checkout never installs — every non-exact walk
 * would have thrown into the silent fall-through. Neither is a bug to patch
 * with `{dot: true}`: a second matcher is a second source of truth. There is
 * now exactly one. **Nothing on this module's import graph resolves outside the
 * Node standard library** (`node:child_process`, `node:fs`, `node:path`,
 * `node:url`, plus `./manifest-utils.mjs`, itself only `fs` + `path`), so
 * planning needs no install. Keep it that way.
 *
 * ── THE RULE ────────────────────────────────────────────────────────────────
 * Pipeline P is skipped iff ALL hold:
 *   (a) P has its own full-40-hex baseline B_P in pipeline-baselines.json;
 *   (b) B_P differs from `__global__` AND `__global__` is an ANCESTOR of B_P
 *       — B_P is strictly newer than the diff base actually used, so skipping
 *       can only ever drop a range a completed build already covered;
 *   (c) B_P still exists in this checkout (`git cat-file -e`);
 *   (d) the Groovy walker, over `git diff --name-only B_P..HEAD`, does not
 *       return P — dependency propagation, buildProcess hashes, manualOnly
 *       exclusion and glob semantics all exactly as the plan stage has them;
 *   (e) P was not force-included by `[build:*]` / FORCE_BUILD_PIPELINES;
 *   (f) P's baseline provenance is determinable (see below);
 *   (g) if that provenance is DISPATCH-ONLY, NO pipeline surviving in this
 *       wave reaches P through `dependsOn` (transitively).
 * Trigger type is not a condition.
 *
 * ── WHAT A BASELINE ACTUALLY PROVES — and why (f)/(g) exist ─────────────────
 * Two provenances hide behind one number in pipeline-baselines.json:
 *   - VERDICT-BACKED. recordPipelineResult advances the baseline after a
 *     SUCCESS or UNSTABLE downstream result, and HOLDS it on ABORTED/FAILURE
 *     (Jenkinsfile:530-535 / :536-540). B_P really was built green.
 *   - DISPATCH-ONLY (optimistic). A `longRunning: true` manifest makes
 *     triggerPipeline fire-and-forget — `shouldWait = !(config.longRunning)`
 *     (Jenkinsfile:750) — so dispatchResult returns `dispatched: true` with no
 *     result yet (Jenkinsfile:622-628) and recordPipelineResult advances the
 *     baseline anyway, before any verdict (Jenkinsfile:521-529). For these,
 *     B_P means only "B_P was dispatched": the build may still be running, or
 *     may have gone red.
 * `longRunning` is the deciding field, read from the same build-manifest.json
 * the Jenkinsfile reads at :121. Today: elohim-holochain, elohim-steward,
 * elohim-library.
 *
 * Naming that in the log is not enough, because dropping an optimistic
 * producer removes a CONSUMER'S COMPLETION GUARANTEE. groupByDependencyLevel
 * orders only SELECTED pipelines, and needsDetachedDependencyBarrier waits
 * only for a producer that is in the selected set (Jenkinsfile:699-705) — an
 * absent dependency reads as satisfied. Drop DNA while keeping edge and edge
 * fetches the floating `dev-latest` hApp tag (elohim/holochain/Jenkinsfile:109),
 * which may not be the bytes DNA's baseline names.
 *
 * So (g): a dispatch-only producer that ANY surviving pipeline reaches through
 * `dependsOn` is KEPT. Full stop — there is no evidence exception, and this
 * module holds no controller client of any kind.
 *
 * An earlier cut allowed the skip when the controller reported a SUCCESS build
 * at exactly B_P. That is unsound, and the unsoundness is in the checked-in
 * pipeline: `dev-latest` is a FLOATING tag that the DNA job overwrites
 * (elohim/holochain/dna/Jenkinsfile:1057) BEFORE its later publication steps,
 * and other non-main branches publish it too. So DNA #10 can succeed at B_P,
 * #11 can overwrite the tag with different bytes and then fail or be aborted,
 * and `lastSuccessfulBuild` still returns #10. The probe would accept #10's sha
 * and drop DNA while edge fetches #11's bytes. A historical build query cannot
 * establish what a floating tag names NOW; only an immutable artifact
 * reference handed to the consumer could, and no such reference exists today.
 * When one does, it belongs in the dispatch parameters, not in this filter.
 *
 * Survivors are traversal ROOTS and traversal continues THROUGH skipped
 * intermediates: if app survives, app -> edge -> DNA keeps DNA even when edge
 * is itself skipped, because a skipped edge contributes no fresh artifact
 * binding either.
 *
 * ── DIRECTION OF SAFETY ─────────────────────────────────────────────────────
 * Every uncertainty dispatches. A missing/short/unparseable baseline; a missing
 * `__global__`; a commit git cannot resolve; a non-ancestor or equal baseline;
 * any git failure; a pipeline with no manifest in this checkout (provenance
 * unknown — e.g. elohim-sophia, whose manifest lives in an uninitialized
 * submodule); a missing or malformed walk result; a needed optimistic
 * producer — all fall through to today's behaviour for that pipeline, and one
 * pipeline's failure never suppresses another's skip. Genesis auto-include
 * still runs AFTER the filter.
 *
 * The record this reads is Jenkins' own `pipeline-baselines.json`. No new
 * state is introduced.
 *
 * Used by:
 *   - genesis/orchestrator/Jenkinsfile (applyAlreadyBuiltFilter, walkNarrowGroups)
 */

import { execFileSync } from 'node:child_process';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const FULL_SHA = /^[0-9a-f]{40}$/;

/** Baseline keys that are bookkeeping, never dispatchable pipeline names. */
const RESERVED_BASELINE_KEYS = new Set(['__global__']);

/** genesis/orchestrator/ -> repo root. cwd-independent on purpose. */
const REPO_ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/**
 * Real git, real manifests. Every method may throw; every caller treats a
 * throw as "no answer" and therefore as "dispatch". There is deliberately NO
 * controller client here — see the header on why a historical build query
 * cannot prove what a floating tag names now.
 */
export function defaultDeps(root = REPO_ROOT) {
  const git = (...args) =>
    execFileSync('git', ['-C', root, ...args], {
      encoding: 'utf8',
      stdio: ['ignore', 'pipe', 'pipe'],
    });

  let manifests = null;
  const loadOnce = async () => {
    if (manifests === null) {
      // manifest-utils.mjs is deliberately dependency-free (fs + path only):
      // planning must never need node_modules. Do not add an import here that
      // breaks that.
      const { loadManifests } = await import('./manifest-utils.mjs');
      manifests = loadManifests(root);
    }
    return manifests;
  };
  const metaOf = async (name) => {
    for (const { content } of await loadOnce()) {
      if (content.pipeline === name) return content;
    }
    return null;
  };

  return {
    hasCommit(sha) {
      // `cat-file -e <sha>^{commit}` is the exists-AND-is-a-commit test.
      try {
        git('cat-file', '-e', `${sha}^{commit}`);
        return true;
      } catch {
        return false;
      }
    },
    isAncestor(ancestor, descendant) {
      // exit 0 = yes, exit 1 = no, anything else = a real git error (throws,
      // and the caller dispatches).
      try {
        git('merge-base', '--is-ancestor', ancestor, descendant);
        return true;
      } catch (err) {
        if (err && err.status === 1) return false;
        throw err;
      }
    },
    changedFiles(from, to) {
      return git('diff', '--name-only', `${from}..${to}`).split('\n').filter(Boolean);
    },
    /**
     * 'verdict' | 'dispatch-only' | null (no manifest in this checkout).
     * `longRunning` is the same field Jenkinsfile:121 reads.
     */
    async provenanceOf(name) {
      const content = await metaOf(name);
      if (content === null) return null;
      return content.longRunning === true ? 'dispatch-only' : 'verdict';
    },
    /** dependsOn exactly as needsDetachedDependencyBarrier reads it. */
    async dependsOn(name) {
      const content = await metaOf(name);
      return Array.isArray(content?.dependsOn) ? content.dependsOn : [];
    },
  };
}

/**
 * PHASE 1 — git + manifest facts only. No matching, no policy.
 *
 * Emits, for every pipeline that clears (a)(b)(c)(e)(f), the narrow file list
 * its baseline implies, grouped by baseline sha so the Groovy walker runs once
 * per DISTINCT baseline rather than once per pipeline.
 *
 * Never throws.
 *
 * @param {object} state
 * @param {string} [state.trigger]     env.BUILD_TRIGGER — log label only
 * @param {string} [state.headSha]     env.GIT_COMMIT_FULL
 * @param {string[]} [state.pipelines] the dispatch set, in plan order
 * @param {Record<string,string>} [state.baselines] pipeline-baselines.json
 * @param {string[]} [state.forced]    env.FORCE_BUILD_PIPELINES, split
 * @param {object} [deps]
 * @returns {Promise<{groups: object, provenance: object, notes: object}>}
 */
export async function planNarrowGroups(state = {}, deps = defaultDeps()) {
  const empty = { groups: {}, provenance: {}, notes: {} };

  const pipelines = Array.isArray(state.pipelines) ? state.pipelines : [];
  const headSha = String(state.headSha ?? '');
  if (!FULL_SHA.test(headSha)) return empty;

  const baselines =
    state.baselines && typeof state.baselines === 'object' ? state.baselines : {};
  const globalBaseline = String(baselines.__global__ ?? '');
  // No usable global baseline means there is no "diff base actually used" to
  // be newer than — condition (b) is unprovable, so nothing is a candidate.
  if (!FULL_SHA.test(globalBaseline)) return empty;

  const forced = new Set(Array.isArray(state.forced) ? state.forced : []);
  const groups = {};
  const provenance = {};
  const notes = {};

  for (const name of pipelines) {
    if (RESERVED_BASELINE_KEYS.has(name)) continue;
    if (forced.has(name)) {
      notes[name] = 'force-included by [build:*]';
      continue;
    }
    const baseline = baselines[name];
    if (typeof baseline !== 'string' || !FULL_SHA.test(baseline)) {
      notes[name] = 'no full-40-hex baseline of its own';
      continue;
    }
    if (baseline === globalBaseline) {
      notes[name] = 'baseline equals the global diff base — nothing newer proven';
      continue;
    }

    try {
      if (!deps.hasCommit(baseline)) {
        notes[name] = `baseline ${baseline.slice(0, 8)} is not resolvable in this checkout`;
        continue;
      }
      if (!deps.isAncestor(globalBaseline, baseline)) {
        notes[name] = `baseline ${baseline.slice(0, 8)} is not a descendant of the global diff base`;
        continue;
      }
      const prov = await deps.provenanceOf(name);
      if (prov === null) {
        notes[name] = 'no build-manifest.json in this checkout — baseline provenance unknown';
        continue;
      }
      provenance[name] = prov;
      if (!groups[baseline]) {
        groups[baseline] = { changedFiles: deps.changedFiles(baseline, headSha), pipelines: [] };
      }
      groups[baseline].pipelines.push(name);
    } catch (err) {
      notes[name] = `git/manifest read failed: ${err?.message ?? err}`;
      delete provenance[name];
    }
  }

  return { groups, provenance, notes };
}

/**
 * Transitive `dependsOn` closure: does any pipeline in `survivors` reach
 * `producer`? Same edges needsDetachedDependencyBarrier uses, but transitive
 * (strictly more conservative than its direct `.contains(producer)`).
 */
async function consumersNeeding(producer, survivors, deps) {
  const found = [];
  for (const candidate of survivors) {
    if (candidate === producer) continue;
    const seen = new Set([candidate]);
    const queue = [candidate];
    let reaches = false;
    while (queue.length > 0 && !reaches) {
      for (const dep of await deps.dependsOn(queue.shift())) {
        if (dep === producer) {
          reaches = true;
          break;
        }
        if (!seen.has(dep)) {
          seen.add(dep);
          queue.push(dep);
        }
      }
    }
    if (reaches) found.push(candidate);
  }
  return found;
}

/**
 * PHASE 3 — turn the Groovy walker's narrow results into a dispatch decision.
 *
 * `walks` is `{<baselineSha>: [<pipeline>…]}` as written by the Jenkinsfile's
 * walkNarrowGroups from `build-graph.groovy::walkBuildGraph`. A group with no
 * entry, or a non-array entry, is treated as "the walker did not answer" and
 * every pipeline in it dispatches.
 *
 * Never throws.
 *
 * @returns {Promise<{dispatch: string[], skipped: string[], logLines: string[]}>}
 */
export async function decideDispatch(state = {}, plan = {}, walks = {}, deps = defaultDeps()) {
  const pipelines = Array.isArray(state.pipelines) ? [...state.pipelines] : [];
  const unchanged = { dispatch: pipelines, skipped: [], logLines: [] };
  if (pipelines.length === 0) return unchanged;

  const groups = plan?.groups && typeof plan.groups === 'object' ? plan.groups : {};
  const provenance = plan?.provenance && typeof plan.provenance === 'object' ? plan.provenance : {};
  const trigger = String(state.trigger || 'UNKNOWN');

  // Which baseline does each candidate belong to, and did the Groovy walker
  // exclude it there?
  const baselineOf = new Map();
  const skip = new Set();
  for (const [baseline, group] of Object.entries(groups)) {
    const members = Array.isArray(group?.pipelines) ? group.pipelines : [];
    const walked = walks?.[baseline];
    if (!Array.isArray(walked)) continue; // walker did not answer → dispatch all
    for (const name of members) {
      if (!pipelines.includes(name)) continue;
      baselineOf.set(name, baseline);
      if (!walked.includes(name)) skip.add(name);
    }
  }

  // (g) Producer readiness. A dispatch-only producer any survivor reaches is
  // KEPT, unconditionally. Un-skipping only GROWS the survivor set, so the
  // fixed point converges, and it cascades through a chain of producers.
  const kept = [];
  const decided = new Set();
  let settled = false;
  while (!settled) {
    settled = true;
    const survivors = pipelines.filter((name) => !skip.has(name));
    for (const name of [...skip]) {
      if (provenance[name] !== 'dispatch-only') continue;
      if (decided.has(name)) continue;
      // A manifest read that throws leaves the consumer set UNKNOWN, and an
      // unknown consumer set is not "no consumers": keep the producer.
      let consumers;
      try {
        consumers = await consumersNeeding(name, survivors, deps);
      } catch {
        consumers = ['(dependsOn unreadable)'];
      }
      if (consumers.length === 0) continue;
      decided.add(name);
      skip.delete(name);
      kept.push({ name, baseline: baselineOf.get(name), consumers });
      settled = false;
    }
  }

  const dispatch = [];
  const skipped = [];
  const logLines = [];
  for (const entry of kept) {
    logLines.push(
      `▶️  ${trigger} already-built filter: ${entry.name} KEPT — baseline ` +
        `${entry.baseline.slice(0, 8)} only records a dispatch (longRunning: verdict never ` +
        `recorded) and surviving pipeline(s) ${entry.consumers.join(', ')} reach it through ` +
        `dependsOn; a floating artifact tag cannot be proven from a baseline sha`,
    );
  }
  for (const name of pipelines) {
    if (!skip.has(name)) {
      dispatch.push(name);
      continue;
    }
    skipped.push(name);
    const proof =
      provenance[name] === 'dispatch-only'
        ? 'dispatched (longRunning: verdict never recorded), nothing surviving needs it'
        : 'built green';
    logLines.push(
      `⏭️  ${trigger} already-built filter: ${name} skipped — baseline ` +
        `${baselineOf.get(name).slice(0, 8)} ${proof}, no watched input changed since ` +
        '(force with [build:*])',
    );
  }

  return { dispatch, skipped, logLines };
}

/**
 * Is {dispatch, skipped} an EXACT PARTITION of `planned`?
 *
 * Returns null when it is, or a one-line reason when it is not. Both this and
 * the Jenkinsfile's decodeAlreadyBuiltDecision enforce it, because a decision
 * that merely "looks like JSON" can still shrink the wave: `{"dispatch":[],
 * "skipped":["invented","invented"]}` empties the plan, and
 * `{"dispatch":["edge"],"skipped":["edge"]}` silently loses every other
 * pipeline. Membership alone is not enough — uniqueness and total coverage are
 * what make the two lists a partition.
 *
 * @param {string[]} planned
 * @param {unknown} dispatch
 * @param {unknown} skipped
 * @returns {string|null}
 */
export function partitionViolation(planned, dispatch, skipped) {
  if (!Array.isArray(dispatch) || !Array.isArray(skipped)) {
    return 'dispatch and skipped must both be arrays';
  }
  const want = new Set(planned);
  const seen = new Set();
  for (const [label, list] of [['dispatch', dispatch], ['skipped', skipped]]) {
    for (const name of list) {
      if (typeof name !== 'string') return `${label} contains a non-string entry`;
      if (!want.has(name)) return `${label} names '${name}', which was never planned`;
      if (seen.has(name)) return `'${name}' appears more than once across dispatch/skipped`;
      seen.add(name);
    }
  }
  if (seen.size !== want.size) {
    const missing = planned.filter((name) => !seen.has(name));
    return `decision omits ${missing.join(', ')}`;
  }
  return null;
}

// ── CLI ──────────────────────────────────────────────────────────────────────
// `node timer-dispatch.mjs groups <state.json>`                  → phase 1 JSON
// `node timer-dispatch.mjs decide <state.json> <plan> <walks>`   → phase 3 JSON
// State always travels by FILE, never argv — same contract as
// commit-tag-parser.mjs, so untrusted text and large maps stay out of the shell
// command line.
if (import.meta.url === `file://${process.argv[1]}`) {
  const [verb, ...paths] = process.argv.slice(2);
  const { readFileSync } = await import('node:fs');
  const read = (p) => JSON.parse(readFileSync(p, 'utf8'));

  if (verb === 'groups' && paths.length === 1) {
    process.stdout.write(JSON.stringify(await planNarrowGroups(read(paths[0]))));
  } else if (verb === 'decide' && paths.length === 3) {
    const [state, plan, walks] = paths.map(read);
    const out = await decideDispatch(state, plan, walks);
    // Self-check before printing. A decision that is not an exact partition of
    // the planned set must never reach the Jenkinsfile as a success: exiting
    // non-zero puts it on the rc guard's fall-through path as well as the
    // decoder's, so neither side can be the only thing standing between a bug
    // here and a silently emptied wave.
    const planned = Array.isArray(state?.pipelines) ? state.pipelines : [];
    const violation = partitionViolation(planned, out.dispatch, out.skipped);
    if (violation !== null) {
      process.stdout.write(JSON.stringify({ error: violation }));
      process.exit(3);
    }
    process.stdout.write(JSON.stringify(out));
  } else {
    console.error(
      'usage: timer-dispatch.mjs groups <state.json>\n' +
        '       timer-dispatch.mjs decide <state.json> <plan.json> <walks.json>',
    );
    process.exit(2);
  }
}
