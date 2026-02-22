/**
 * Retry-poll assertions for eventual consistency of content across doorways.
 */

import { DoorwayClient } from '../api/doorway-client.js';
import { retry, type RetryOptions } from '../utils/retry.js';

/**
 * Poll until content with the given ID is visible through a doorway client.
 * Throws if not found within the retry window.
 */
export async function waitForContent(
  client: DoorwayClient,
  contentId: string,
  opts: RetryOptions = {},
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
  opts: RetryOptions = {},
): Promise<Record<string, unknown>[]> {
  return retry(async () => {
    const results = await client.searchContent(tags);
    if (!results || results.length === 0) {
      throw new Error(`No content with tags [${tags.join(', ')}] on ${client.url}`);
    }
    return results;
  }, opts);
}
