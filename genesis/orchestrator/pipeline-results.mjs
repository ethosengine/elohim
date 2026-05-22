/**
 * Pipeline result classification — single source of truth for what counts
 * as success, failure, or waste across all CI/CD tooling.
 *
 * Why three buckets:
 *   - success: SUCCESS + UNSTABLE — the user's work passed enough gates to be
 *     useful. UNSTABLE means non-blocking issues; we don't pretend it's clean
 *     but we also don't pretend it's broken.
 *   - failure: FAILURE only. ABORTED is NOT a failure of the work; it's a
 *     failure of CI orchestration (we asked it to stop) and should be counted
 *     as waste so concurrency tuning can act on it.
 *   - wasted: ABORTED — the build never got a chance to verdict on the work.
 *     Persistent waste signals supersede-thrash or operator-aborts.
 *
 * Used by:
 *   - pipeline-trajectory.mjs (pattern detector)
 *   - reconcile-build-graph.mjs (success-set check)
 *   - count-pipeline-failures.sh (via pipeline-list.json side channel)
 */

export const SUCCESSFUL_RESULTS = new Set(['SUCCESS', 'UNSTABLE']);
export const TERMINAL_FAILURE_RESULTS = new Set(['FAILURE']);
export const WASTED_RESULTS = new Set(['ABORTED']);
export const SKIPPED_RESULTS = new Set(['NOT_BUILT']);

/**
 * @param {string|null|undefined} result Jenkins build result string
 * @returns {'success'|'failure'|'wasted'|'skipped'|'pending'}
 */
export function classifyResult(result) {
  if (result == null) return 'pending';
  if (SUCCESSFUL_RESULTS.has(result)) return 'success';
  if (TERMINAL_FAILURE_RESULTS.has(result)) return 'failure';
  if (WASTED_RESULTS.has(result)) return 'wasted';
  if (SKIPPED_RESULTS.has(result)) return 'skipped';
  return 'pending';
}

export function isSuccess(result) { return classifyResult(result) === 'success'; }
export function isFailure(result) { return classifyResult(result) === 'failure'; }
export function isWasted(result)  { return classifyResult(result) === 'wasted'; }
