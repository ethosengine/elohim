import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { firstValueFrom } from 'rxjs';

import { NativeBackend } from './native-backend';

describe('NativeBackend', () => {
  let backend: NativeBackend;
  let fetchSpy: ReturnType<typeof vi.spyOn>;

  beforeEach(() => {
    backend = new NativeBackend(() => 'mock-jwt');
  });

  afterEach(() => {
    fetchSpy?.mockRestore();
  });

  it('should handle fulfilled response', async () => {
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        requestId: 'req-1',
        elohimId: 'e-1',
        status: 'fulfilled',
        constitutionalReasoning: {
          primaryPrinciple: 'test',
          interpretation: 'test',
          valuesWeighed: [],
          confidence: 1,
        },
        payload: { text: 'hello' },
        respondedAt: new Date().toISOString(),
      }),
    } as Response);

    const response = await firstValueFrom(
      backend.invoke({ requestId: 'req-1', capability: 'test', params: {} } as any, { id: 'e-1' } as any),
    );

    expect(response.status).toBe('fulfilled');
  });

  it('should handle deferred response', async () => {
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        requestId: 'req-1',
        elohimId: 'e-1',
        status: 'deferred',
        constitutionalReasoning: {
          primaryPrinciple: 'capacity',
          interpretation: 'budget exhausted',
          valuesWeighed: [],
          confidence: 1,
        },
        deferReason: 'BudgetExhausted',
        meshHints: [],
        retryAfterMs: 10000,
        respondedAt: new Date().toISOString(),
      }),
    } as Response);

    const response = await firstValueFrom(
      backend.invoke({ requestId: 'req-1', capability: 'path-recommendation', params: {} } as any, { id: 'e-1' } as any),
    );

    expect(response.status).toBe('deferred');
    expect(response.deferReason).toBe('BudgetExhausted');
    expect(response.meshHints).toEqual([]);
    expect(response.retryAfterMs).toBe(10000);
  });

  it('should include auth header when token available', async () => {
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        requestId: 'req-1',
        elohimId: 'e-1',
        status: 'fulfilled',
        constitutionalReasoning: {
          primaryPrinciple: 'test',
          interpretation: 'test',
          valuesWeighed: [],
          confidence: 1,
        },
        respondedAt: new Date().toISOString(),
      }),
    } as Response);

    await firstValueFrom(
      backend.invoke({ requestId: 'req-1', capability: 'test', params: {} } as any, { id: 'e-1' } as any),
    );

    const [, options] = fetchSpy.mock.calls[0];
    expect((options as RequestInit).headers).toHaveProperty('Authorization', 'Bearer mock-jwt');
  });

  it('should not include auth header when no token', async () => {
    const noAuthBackend = new NativeBackend();

    fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
      json: async () => ({
        requestId: 'req-1',
        elohimId: 'e-1',
        status: 'fulfilled',
        constitutionalReasoning: {
          primaryPrinciple: 'test',
          interpretation: 'test',
          valuesWeighed: [],
          confidence: 1,
        },
        respondedAt: new Date().toISOString(),
      }),
    } as Response);

    await firstValueFrom(
      noAuthBackend.invoke({ requestId: 'req-1', capability: 'test', params: {} } as any, { id: 'e-1' } as any),
    );

    const [, options] = fetchSpy.mock.calls[0];
    expect((options as RequestInit).headers).not.toHaveProperty('Authorization');
  });

  it('should return declined on HTTP error', async () => {
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: false,
      status: 503,
      text: async () => 'Service unavailable',
    } as Response);

    const response = await firstValueFrom(
      backend.invoke({ requestId: 'req-1', capability: 'test', params: {} } as any, { id: 'e-1' } as any),
    );

    expect(response.status).toBe('declined');
    expect(response.declineReason).toContain('503');
  });

  it('should return declined on network error', async () => {
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockRejectedValueOnce(new Error('Network error'));

    const response = await firstValueFrom(
      backend.invoke({ requestId: 'req-1', capability: 'test', params: {} } as any, { id: 'e-1' } as any),
    );

    expect(response.status).toBe('declined');
    expect(response.declineReason).toContain('Network error');
  });

  it('should probe health endpoint for availability', async () => {
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockResolvedValueOnce({
      ok: true,
    } as Response);

    const available = await backend.isAvailable();

    expect(available).toBe(true);
    expect(fetchSpy).toHaveBeenCalledWith(
      '/api/v1/elohim/invoke/health',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('should return false when health probe fails', async () => {
    fetchSpy = vi.spyOn(globalThis, 'fetch').mockRejectedValueOnce(new Error('Connection refused'));

    const available = await backend.isAvailable();

    expect(available).toBe(false);
  });
});
