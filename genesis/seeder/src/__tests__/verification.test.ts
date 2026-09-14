import { afterEach, describe, expect, it, vi } from 'vitest';

import { SeedingVerification } from '../verification.js';

describe('seeding post-flight verification', () => {
  afterEach(() => vi.unstubAllEnvs());

  it('uses complete exact-ID reachability when author-local counts cannot see adopted rows', async () => {
    const calls: string[] = [];
    const appWs = {
      callZome: vi.fn(
        async ({
          fn_name,
          payload,
        }: {
          fn_name: string;
          payload?: { id?: string };
        }) => {
          calls.push(`${fn_name}:${payload?.id ?? ''}`);
          if (fn_name === 'get_content_by_id') return { id: payload?.id };
          if (fn_name === 'get_content_stats') return { total_count: 3, by_type: { concept: 3 } };
          throw new Error(`unexpected zome function ${fn_name}`);
        },
      ),
    };
    const verification = new SeedingVerification(appWs as never, [] as never);

    const result = await verification.runPostflightVerification(
      { content: 2, paths: 2 },
      ['one', 'two'],
      ['path-one', 'path-two'],
    );

    expect(result.success).toBe(true);
    expect(result.finalCounts).toEqual({ content: 3, paths: 0 });
    expect(calls).toEqual([
      'get_content_by_id:one',
      'get_content_by_id:two',
      'get_content_by_id:path-one',
      'get_content_by_id:path-two',
      'get_content_stats:',
    ]);
    expect(appWs.callZome).not.toHaveBeenCalledWith(
      expect.objectContaining({ fn_name: 'get_all_paths' }),
    );
  });

  it('refuses an idempotent path set when even one requested path is unreadable', async () => {
    vi.stubEnv('SEED_VERIFY_WAIT_MS', '0');
    const pathIds = Array.from({ length: 8 }, (_, index) => `path-${index + 1}`);
    const appWs = {
      callZome: vi.fn(async ({ fn_name, payload }: { fn_name: string; payload?: { id?: string } }) => {
        if (fn_name === 'get_content_by_id') {
          return payload?.id === 'path-8' ? null : { id: payload?.id };
        }
        if (fn_name === 'get_content_stats') return { total_count: 3, by_type: { concept: 3 } };
        throw new Error(`unexpected zome function ${fn_name}`);
      }),
    };
    const verification = new SeedingVerification(appWs as never, [] as never);

    const result = await verification.runPostflightVerification(
      { content: 0, paths: 8, pathsExisting: 8 },
      [],
      pathIds,
    );

    expect(result.success).toBe(false);
    expect(result.errors).toEqual([expect.stringContaining('7/8 sample entries found')]);
  });
});
