/**
 * Timer already-built filter — the ONE home for "this pipeline already built
 * this exact commit, so a cron run has nothing to learn from building it again".
 *
 * WHY THIS EXISTS (2026-09-22). The orchestrator's `cron('0 9 * * *')` exists
 * for one stage: the Gate #6 iroh parity soak, which runs `cargo nextest`
 * inside the orchestrator's own container and needs neither a fresh edge roll
 * nor a running fleet. But the cron fires the WHOLE pipeline, and the dispatch
 * set is computed from the `__global__` baseline — which `archivePipelineBaselines`
 * deliberately refuses to advance on a FAILURE/ABORTED run. So a string of red
 * orchestrator runs freezes `__global__`, and every subsequent nightly re-diffs
 * the same range and re-dispatches the same cascade.
 *
 * Lived, measured: orchestrator/dev #1886 (timer, 2026-09-21 09:00Z) loaded
 * `__global__: 82dcb5c8` alongside `elohim-edge: bc81e107` — the edge baseline
 * already equalled the run's own HEAD — and still dispatched elohim-edge/dev
 * #1471, which rebuilt the images edge #1470 had shipped SUCCESS on 2026-09-20
 * for the same sha bc81e107, and rolled all seven alpha peers for 3h04m. Each
 * fleet roll restarts the conductor pods and leaves storage cells answering
 * CellDisabled for hours (first recovery +108min; the authoring hosts' lamad
 * cells still down +9h), during which no app build can author heads. A daily
 * timer roll of an unchanged commit is therefore a daily WRITE OUTAGE bought
 * with zero new information — and, per AP-TIMER-WEBHOOK-COLLISION, it also
 * abortPrevious's any push that happens to be in flight at 09:00Z.
 *
 * The record this reads is Jenkins' own: the `pipeline-baselines.json` artifact
 * that `recordPipelineResult` advances to `env.GIT_COMMIT_FULL` only on a
 * non-failing downstream outcome. No new state is introduced.
 *
 * DIRECTION OF SAFETY. The predicate is deliberately one-sided: it fires ONLY
 * on an exact 40-hex match between a pipeline's own baseline and HEAD, only on
 * a TIMER-triggered run, and never against a `[build:*]` force-include. A new
 * commit, a cold start, a short sha, an unparseable record, or any other
 * trigger type all fall through to today's behaviour unchanged.
 *
 * Used by:
 *   - genesis/orchestrator/Jenkinsfile (applyTimerAlreadyBuiltFilter)
 */

const FULL_SHA = /^[0-9a-f]{40}$/;

/** Baseline keys that are bookkeeping, never dispatchable pipeline names. */
const RESERVED_BASELINE_KEYS = new Set(['__global__']);

/**
 * Is `recorded` proof that some pipeline already built `headSha`?
 *
 * Only an exact full-sha match counts. A short sha is not proof: the baseline
 * record stores 40-hex `env.GIT_COMMIT_FULL`, and accepting a prefix would let
 * an 8-char collision suppress a real build.
 *
 * @param {string|null|undefined} recorded baseline commit for one pipeline
 * @param {string|null|undefined} headSha this run's HEAD commit
 * @returns {boolean}
 */
export function isAlreadyBuiltAt(recorded, headSha) {
  if (typeof recorded !== 'string' || typeof headSha !== 'string') return false;
  if (!FULL_SHA.test(recorded) || !FULL_SHA.test(headSha)) return false;
  return recorded === headSha;
}

/**
 * Drop, from a TIMER run's dispatch set, every pipeline whose most recent
 * build for THIS commit already succeeded — unless a `[build:*]` tag forces it.
 *
 * @param {object} input
 * @param {string} [input.trigger]   env.BUILD_TRIGGER — only 'TIMER' filters
 * @param {string} [input.headSha]   env.GIT_COMMIT_FULL
 * @param {string[]} [input.pipelines] the dispatch set, in plan order
 * @param {Record<string,string>} [input.baselines] pipeline-baselines.json
 * @param {string[]} [input.forced]  env.FORCE_BUILD_PIPELINES, split
 * @returns {{dispatch: string[], skipped: string[], filtered: boolean}}
 */
export function filterTimerDispatch(input = {}) {
  const pipelines = Array.isArray(input.pipelines) ? [...input.pipelines] : [];
  const unchanged = { dispatch: pipelines, skipped: [], filtered: false };

  if (input.trigger !== 'TIMER') return unchanged;
  if (!FULL_SHA.test(String(input.headSha ?? ''))) return unchanged;

  const baselines = input.baselines && typeof input.baselines === 'object' ? input.baselines : {};
  const forced = new Set(Array.isArray(input.forced) ? input.forced : []);

  const dispatch = [];
  const skipped = [];
  for (const name of pipelines) {
    const alreadyBuilt =
      !RESERVED_BASELINE_KEYS.has(name) &&
      !forced.has(name) &&
      isAlreadyBuiltAt(baselines[name], input.headSha);
    (alreadyBuilt ? skipped : dispatch).push(name);
  }
  return { dispatch, skipped, filtered: true };
}

// CLI: `node timer-dispatch.mjs filter <state.json>` → JSON on stdout.
// The Jenkinsfile passes state by FILE, never argv — same contract as
// commit-tag-parser.mjs, so untrusted text and large maps stay out of the
// shell command line.
if (import.meta.url === `file://${process.argv[1]}`) {
  const [verb, statePath] = process.argv.slice(2);
  if (verb !== 'filter' || !statePath) {
    console.error('usage: timer-dispatch.mjs filter <state.json>');
    process.exit(2);
  }
  const { readFileSync } = await import('node:fs');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  process.stdout.write(JSON.stringify(filterTimerDispatch(state)));
}
