/**
 * waitForPull — poll elohim-storage /p2p/status until the acquisition pull
 * queue is caught up (spec 2026-06-07-epr-acquisition-pull-queue-design §4.3).
 *
 * Contract (parallel to waitForDrain):
 *
 *   GET /p2p/status returns a JSON object whose `pull` field is either
 *   null (the storage node cannot compute pull state right now — DB pool
 *   exhausted, count query failed, startup not finished) or a shape of
 *   { total, fetched, pending, failed, caughtUp }.
 *
 *   The seeder MUST treat `pull === null` as "keep waiting", never as
 *   "caught up". Otherwise a broken node would produce a false success
 *   signal for the pipeline.
 *
 * Termination condition:
 *   pull !== null
 *   && pull.total >= expectedMinTotal   (guards zero-desired-set seeds)
 *   && pull.pending === 0
 *
 * All three conditions must hold on the same poll response.
 *
 * P2P readiness is NOT the seeder's concern — the acquisition state machine
 * handles peer formation and fetch dispatch. This helper only waits on the
 * resulting observable state.
 */

import type { P2PStatusInfo, PullStatusInfo } from '@elohim/storage-client';

export interface WaitForPullOptions {
  /** Total wait budget before throwing. Default: 5 minutes. */
  timeoutMs?: number;
  /** Poll interval. Default: 2 seconds. */
  pollIntervalMs?: number;
  /**
   * Expected minimum row count. If provided, also asserts
   * pull.total >= this before declaring complete — catches the
   * "zero desired set" edge case where a stale-empty DB would
   * otherwise satisfy pending === 0 instantly.
   * Default: 1.
   */
  expectedMinTotal?: number;
}

/**
 * Poll `GET /p2p/status` on a storage peer until the pull queue is empty.
 *
 * Throws if the timeout elapses or fetches fail 10 times in a row.
 */
export async function waitForPull(
  baseUrl: string,
  options: WaitForPullOptions = {},
): Promise<PullStatusInfo> {
  const timeoutMs = options.timeoutMs ?? 5 * 60_000;
  const pollIntervalMs = options.pollIntervalMs ?? 2_000;
  const expectedMinTotal = options.expectedMinTotal ?? 1;

  const normalizedBase = baseUrl.replace(/\/$/, '');
  const url = `${normalizedBase}/p2p/status`;

  const deadline = Date.now() + timeoutMs;
  let lastLoggedPending = -1;
  let consecutiveFetchErrors = 0;
  let lastSeenPull: PullStatusInfo | null = null;
  let sawNonNullPull = false;

  console.log(
    `waitForPull: polling ${url} every ${pollIntervalMs}ms (timeout ${timeoutMs}ms, expectedMinTotal=${expectedMinTotal})`,
  );

  while (Date.now() < deadline) {
    try {
      const resp = await fetch(url);
      if (!resp.ok) {
        consecutiveFetchErrors++;
        console.warn(
          `waitForPull: /p2p/status returned ${resp.status} (attempt ${consecutiveFetchErrors})`,
        );
        if (consecutiveFetchErrors >= 10) {
          throw new Error(
            `waitForPull: /p2p/status returned ${resp.status} after 10 attempts`,
          );
        }
      } else {
        consecutiveFetchErrors = 0;
        const status = (await resp.json()) as P2PStatusInfo;

        if (status.pull === null || status.pull === undefined) {
          // Broken node signal — do NOT treat as "done".
          console.log(`waitForPull: pull=null (peer DB unavailable), waiting...`);
        } else {
          lastSeenPull = status.pull;
          sawNonNullPull = true;
          const { total, fetched, pending } = status.pull;
          // Only log when progress changes, to keep pipeline output readable.
          if (pending !== lastLoggedPending) {
            const peersSuffix =
              status.connectedPeers !== undefined
                ? ` (connectedPeers=${status.connectedPeers})`
                : '';
            console.log(
              `waitForPull: ${fetched}/${total} fetched, ${pending} pending${peersSuffix}`,
            );
            lastLoggedPending = pending;
          }
          if (pending === 0 && total >= expectedMinTotal) {
            console.log(`waitForPull: complete — ${total} rows pulled`);
            return status.pull;
          }
        }
      }
    } catch (err) {
      consecutiveFetchErrors++;
      console.warn(
        `waitForPull: fetch error (attempt ${consecutiveFetchErrors}): ${err}`,
      );
      if (consecutiveFetchErrors >= 10) {
        throw new Error(
          `waitForPull: fetch failed 10 times in a row: ${err}`,
        );
      }
    }

    await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
  }

  const lastState = sawNonNullPull
    ? `last seen pull: total=${lastSeenPull!.total}, fetched=${lastSeenPull!.fetched}, pending=${lastSeenPull!.pending}, failed=${lastSeenPull!.failed}`
    : 'pull was null throughout (peer DB unavailable)';
  throw new Error(`waitForPull: pull did not complete within ${timeoutMs}ms — ${lastState}`);
}
