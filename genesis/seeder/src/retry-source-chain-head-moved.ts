/**
 * Bounded retry for a direct-conductor zome call that can legitimately lose
 * Holochain's optimistic-concurrency race on a source chain.
 *
 * A Holochain source chain is single-writer with optimistic concurrency.
 * When a fixture in this package writes directly to a household conductor's
 * app websocket (bypassing storage's own reconciliation path), that same
 * conductor's storage peer can commit to the identical chain concurrently —
 * so the fixture's commit can lose the race with exactly one error:
 * "Attempted to commit a bundle to the source chain, but the source chain
 * head has moved since the bundle began." The documented client behaviour
 * for this exact error is to retry; a failed commit of this shape writes
 * NOTHING to the chain, so re-issuing the identical call afterward is safe.
 *
 * This is a small package-local equivalent of
 * `genesis/a2o/src/framework/dataplane/carried-election.ts`'s
 * `isSourceChainHeadMovedError` / `retryOnSourceChainHeadMoved` (landed in
 * commit 1630ad305). `genesis/seeder` (package `holochain-seeder`) has no
 * workspace dependency on `@elohim/a2o` and this helper is intentionally too
 * small to justify creating one — the matcher and retry semantics are kept
 * IDENTICAL on purpose so the two copies never silently diverge; if the a2o
 * original's matcher or bounds change, mirror the change here too.
 */

/**
 * Exact match for Holochain's optimistic-concurrency source-chain conflict —
 * "Attempted to commit a bundle to the source chain, but the source chain
 * head has moved since the bundle began" (or its short `HeadMoved` form).
 * A near-miss like `NotHeadMovedPermanent` must NOT match.
 */
export function isSourceChainHeadMovedError(message: string): boolean {
  return (
    message.includes('source chain head has moved') ||
    /(?:^|[:(\s])HeadMoved(?:$|[:,)\s])/.test(message)
  );
}

// Jittered backoff ladder: 200ms up to 3s, 5 steps => 6 attempts total.
const HEAD_MOVED_RETRY_BACKOFFS_MS = [200, 400, 800, 1600, 3000] as const;

/**
 * Retry `attempt()` only when it throws the exact source-chain-head-moved
 * error; any other error propagates on the first try. Up to 6 attempts total
 * (5 jittered backoffs, 200ms-3s each) — well under this package's leg
 * timeouts. On exhaustion, throws naming the attempt count plus the
 * bundle/current head from the last failure (when present in its message).
 */
export async function retryOnSourceChainHeadMoved<T>(
  label: string,
  attempt: () => Promise<T>,
  sleep: (milliseconds: number) => Promise<void> = async milliseconds => {
    await new Promise(resolve => setTimeout(resolve, milliseconds));
  }
): Promise<T> {
  const maxAttempts = HEAD_MOVED_RETRY_BACKOFFS_MS.length + 1;
  let lastMessage = '';
  for (let i = 0; i < maxAttempts; i++) {
    try {
      return await attempt();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      if (!isSourceChainHeadMovedError(message)) throw error;
      lastMessage = message;
      if (i === maxAttempts - 1) break;
      const base = HEAD_MOVED_RETRY_BACKOFFS_MS[i];
      // eslint-disable-next-line sonarjs/pseudo-random -- backoff jitter, not security
      const jitter = Math.floor(base * 0.25 * Math.random());
      console.warn(`${label}: source chain head moved, retrying (attempt ${i + 2}/${maxAttempts})`);
      await sleep(base + jitter);
    }
  }
  const detail = /Bundle head:.*$/s.exec(lastMessage)?.[0] ?? lastMessage;
  throw new Error(
    `${label}: gave up after ${maxAttempts} attempts against a moving source chain head — ${detail}`
  );
}
