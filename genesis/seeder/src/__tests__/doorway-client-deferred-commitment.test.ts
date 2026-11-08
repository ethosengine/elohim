import { afterEach, describe, expect, it, vi } from 'vitest';

import { DoorwayClient } from '../doorway-client.js';

class TestClient extends DoorwayClient {
  public createCommitment(): Promise<Response> {
    return this.fetch('/api/v1/commitments', { method: 'POST' });
  }
}

const deferred = () =>
  new Response('REA projection changed during authority read; deferred', { status: 400 });

describe('DoorwayClient deferred commitment-write retry', () => {
  const originalFetch = globalThis.fetch;

  afterEach(() => {
    globalThis.fetch = originalFetch;
    vi.restoreAllMocks();
  });

  it('succeeds after two deferred 400 responses and makes three calls', async () => {
    const answers = [deferred(), deferred(), new Response('{}', { status: 200 })];
    const fetchMock = vi.fn(async () => answers.shift()!);
    const sleeps: number[] = [];
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => {});

    const client = new TestClient({
      baseUrl: 'http://storage.test',
      sleep: async milliseconds => {
        sleeps.push(milliseconds);
      },
    });

    await expect(client.createCommitment()).resolves.toMatchObject({ status: 200 });
    expect(fetchMock).toHaveBeenCalledTimes(3);
    expect(sleeps).toEqual([1_000, 2_000]);
    expect(warn).toHaveBeenNthCalledWith(
      1,
      expect.stringMatching(/attempt 2\/4.*REA projection changed during authority read; deferred/),
    );
    expect(warn).toHaveBeenNthCalledWith(
      2,
      expect.stringMatching(/attempt 3\/4.*REA projection changed during authority read; deferred/),
    );
  });

  it('fails after four deferred 400 responses and names all four attempts', async () => {
    const fetchMock = vi.fn(async () => deferred());
    const sleeps: number[] = [];
    globalThis.fetch = fetchMock as unknown as typeof fetch;
    vi.spyOn(console, 'warn').mockImplementation(() => {});

    const client = new TestClient({
      baseUrl: 'http://storage.test',
      sleep: async milliseconds => {
        sleeps.push(milliseconds);
      },
    });

    await expect(client.createCommitment()).rejects.toThrow(/failed after 4 attempts/);
    expect(fetchMock).toHaveBeenCalledTimes(4);
    expect(sleeps).toEqual([1_000, 2_000, 4_000]);
  });

  it('returns a non-deferred 400 on the first call without retrying', async () => {
    const fetchMock = vi.fn(async () => new Response('invalid commitment shape', { status: 400 }));
    const sleep = vi.fn(async () => {});
    globalThis.fetch = fetchMock as unknown as typeof fetch;

    const client = new TestClient({ baseUrl: 'http://storage.test', sleep });
    const response = await client.createCommitment();

    expect(response.status).toBe(400);
    expect(await response.text()).toBe('invalid commitment shape');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    expect(sleep).not.toHaveBeenCalled();
  });
});
