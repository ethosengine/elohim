/**
 * waitForDrain — poll elohim-storage /p2p/status until its publish drain
 * queue is empty.
 *
 * Contract (see Phase E1 of the seeder DHT drain plan):
 *
 *   GET /p2p/status returns a JSON object whose `drain` field is either
 *   null (the storage node cannot compute drain state right now — DB pool
 *   exhausted, count query failed, startup not finished) or a shape of
 *   { total, published, pending }.
 *
 *   The seeder MUST treat `drain === null` as "keep waiting", never as
 *   "caught up". Otherwise a broken node would produce a false success
 *   signal for the pipeline.
 *
 * Termination condition:
 *   drain !== null
 *   && drain.total >= expectedMinTotal   (guards zero-content seeds)
 *   && drain.pending === 0
 *
 * All three conditions must hold on the same poll response.
 *
 * P2P readiness is NOT the seeder's concern — the storage node's
 * bootstrap retry loop (Phase B) handles peer formation, and the drain
 * loop (Phase C) handles actual publish. This helper only waits on the
 * resulting observable state.
 */

interface DrainStatus {
  total: number;
  published: number;
  pending: number;
}

interface P2pStatus {
  // Only the fields the seeder cares about. The full P2PStatusInfo has
  // more (peerId, listenAddresses, natStatus, relayMode, etc.) but those
  // are not load-bearing for drain-completion detection.
  drain: DrainStatus | null;
  connectedPeers?: number;
}

export interface WaitForDrainOptions {
  /** Total wait budget before throwing. Default: 5 minutes. */
  timeoutMs?: number;
  /** Poll interval. Default: 2 seconds. */
  pollIntervalMs?: number;
  /**
   * Expected minimum row count. If provided, also asserts
   * drain.total >= this before declaring complete — catches the
   * "zero content was seeded" edge case where a stale-empty DB would
   * otherwise satisfy pending === 0 instantly.
   * Default: 1.
   */
  expectedMinTotal?: number;
}

/**
 * Poll `GET /p2p/status` on a storage peer until the drain queue is empty.
 *
 * Throws if the timeout elapses or fetches fail 10 times in a row.
 */
export async function waitForDrain(
  baseUrl: string,
  options: WaitForDrainOptions = {},
): Promise<DrainStatus> {
  const timeoutMs = options.timeoutMs ?? 5 * 60_000;
  const pollIntervalMs = options.pollIntervalMs ?? 2_000;
  const expectedMinTotal = options.expectedMinTotal ?? 1;

  const normalizedBase = baseUrl.replace(/\/$/, '');
  const url = `${normalizedBase}/p2p/status`;

  const deadline = Date.now() + timeoutMs;
  let lastLoggedPending = -1;
  let consecutiveFetchErrors = 0;

  console.log(
    `waitForDrain: polling ${url} every ${pollIntervalMs}ms (timeout ${timeoutMs}ms, expectedMinTotal=${expectedMinTotal})`,
  );

  while (Date.now() < deadline) {
    try {
      const resp = await fetch(url);
      if (!resp.ok) {
        consecutiveFetchErrors++;
        console.warn(
          `waitForDrain: /p2p/status returned ${resp.status} (attempt ${consecutiveFetchErrors})`,
        );
        if (consecutiveFetchErrors >= 10) {
          throw new Error(
            `waitForDrain: /p2p/status returned ${resp.status} after 10 attempts`,
          );
        }
      } else {
        consecutiveFetchErrors = 0;
        const status = (await resp.json()) as P2pStatus;

        if (status.drain === null || status.drain === undefined) {
          // Broken node signal — do NOT treat as "done".
          console.log(`waitForDrain: drain=null (peer DB unavailable), waiting...`);
        } else {
          const { total, published, pending } = status.drain;
          // Only log when progress changes, to keep pipeline output readable.
          if (pending !== lastLoggedPending) {
            const peersSuffix =
              status.connectedPeers !== undefined
                ? ` (connectedPeers=${status.connectedPeers})`
                : '';
            console.log(
              `waitForDrain: ${published}/${total} published, ${pending} pending${peersSuffix}`,
            );
            lastLoggedPending = pending;
          }
          if (pending === 0 && total >= expectedMinTotal) {
            console.log(`waitForDrain: complete — ${total} rows drained`);
            return status.drain;
          }
        }
      }
    } catch (err) {
      consecutiveFetchErrors++;
      console.warn(
        `waitForDrain: fetch error (attempt ${consecutiveFetchErrors}): ${err}`,
      );
      if (consecutiveFetchErrors >= 10) {
        throw new Error(
          `waitForDrain: fetch failed 10 times in a row: ${err}`,
        );
      }
    }

    await new Promise(resolve => setTimeout(resolve, pollIntervalMs));
  }

  throw new Error(`waitForDrain: drain did not complete within ${timeoutMs}ms`);
}
