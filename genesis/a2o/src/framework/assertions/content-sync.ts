/**
 * Retry-poll assertions for eventual consistency of content across doorways.
 */

import { DoorwayClient } from '../api/doorway-client.js';
import { retry, type RetryOptions } from '../utils/retry.js';

/**
 * How long an external read may legitimately answer 404 after `POST /db/content`
 * has returned 201 — the provenance-publish window.
 *
 * elohim-storage's provenance gate hides any content row lacking BOTH
 * `dht_anchor_hash` and `p2p_published_at` (`db/content_diesel.rs` — external
 * HTTP handlers MUST pass the gate, "as if the row did not exist"). The publish
 * drain loop (`p2p/mod.rs::drain_publish_queue`) is the SOLE writer of
 * `p2p_published_at` and ticks on `DRAIN_INTERVAL_SECS = 15`. So an
 * API-authored row is invisible for up to one whole drain tick PLUS the publish
 * itself: measured 8 s / 10 s / 15 s / 15 s / 19 s across five authored nodes on
 * the household mesh (2026-08-21). The 404 is the gate working, not a defect —
 * a poll budget that expires inside this window measures the drain phase, not
 * the system.
 *
 * `maxAttempts` is deliberately large enough that `timeoutMs` is the binding
 * constraint. That coupling is the trap this constant exists to close: the
 * previous call sites asked for `timeoutMs: 15_000` but also `maxAttempts: 5`,
 * and with `initialDelayMs: 1000` × `backoffFactor: 1.5` five attempts exhaust
 * after ~8.1 s of backoff — the poll gave up at roughly half the window it
 * declared, and failed 3 runs in 4.
 */
export const PROVENANCE_VISIBILITY_BUDGET: RetryOptions = {
  maxAttempts: 12,
  initialDelayMs: 1000,
  backoffFactor: 1.5,
  timeoutMs: 45_000,
};

/**
 * Poll until content with the given ID is visible through a doorway client.
 * Throws if not found within the retry window.
 */
export async function waitForContent(
  client: DoorwayClient,
  contentId: string,
  opts: RetryOptions = {}
): Promise<Record<string, unknown>> {
  return retry(async () => {
    const content = await client.getContent(contentId);
    if (!content) throw new Error(`Content ${contentId} not yet visible on ${client.url}`);
    return content;
  }, opts);
}

/**
 * Poll until content matching the given tags appears on a doorway.
 */
export async function waitForContentByTags(
  client: DoorwayClient,
  tags: string[],
  opts: RetryOptions = {}
): Promise<Record<string, unknown>[]> {
  return retry(async () => {
    const results = await client.searchContent(tags);
    if (!results || results.length === 0) {
      throw new Error(`No content with tags [${tags.join(', ')}] on ${client.url}`);
    }
    return results;
  }, opts);
}
