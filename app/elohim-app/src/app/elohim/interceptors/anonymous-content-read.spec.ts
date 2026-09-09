import {
  HttpContext,
  HttpErrorResponse,
  HttpHeaders,
  HttpParams,
  HttpRequest,
  HttpResponse,
} from '@angular/common/http';
import { TestBed } from '@angular/core/testing';
import { firstValueFrom, of, throwError } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { DOORWAY_ADDRESS_RESOLVER, ELOHIM_CLIENT as LIBRARY_CLIENT } from '@elohim/service';
import { FederationPeerResolver } from '@elohim/service/keep';
import { ELOHIM_CLIENT, provideElohimClient } from '../providers/elohim-client.provider';
import { ANONYMOUS_CONTENT_READ } from './anonymous-content-read';
import { apiBaseUrlInterceptor, resetDoorwayFailoverState } from './api-base-url.interceptor';

const primary = 'https://reader.example';
const sibling = 'https://sibling.example';
const context = () => new HttpContext().set(ANONYMOUS_CONTENT_READ, true);

describe('explicit anonymous sibling reads', () => {
  beforeEach(async () => {
    resetDoorwayFailoverState();
    vi.stubGlobal('location', { origin: primary });
    vi.stubGlobal(
      'fetch',
      vi.fn().mockResolvedValue({
        ok: true,
        json: async () => ({
          doorways: [
            { id: 'primary', url: primary },
            { id: 'sibling', url: sibling },
          ],
        }),
      })
    );
    TestBed.configureTestingModule({
      providers: provideElohimClient({
        mode: { type: 'browser', doorway: { url: primary } },
        federatedResolution: true,
      }),
    });
    await (TestBed.inject(DOORWAY_ADDRESS_RESOLVER) as FederationPeerResolver).warm();
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
    TestBed.resetTestingModule();
  });

  async function read(req: HttpRequest<unknown>, failing = primary) {
    const calls: HttpRequest<unknown>[] = [];
    const result = TestBed.runInInjectionContext(() =>
      apiBaseUrlInterceptor(req, request => {
        calls.push(request);
        return request.url.startsWith(failing)
          ? throwError(() => new HttpErrorResponse({ status: 0 }))
          : of(new HttpResponse({ status: 200, body: 'body' }));
      })
    );
    await firstValueFrom(result).catch(() => undefined);
    return calls;
  }

  it('shares the warmed resolver and one SDK client across both tokens', () => {
    expect(TestBed.inject(ELOHIM_CLIENT)).toBe(TestBed.inject(LIBRARY_CLIENT));
    expect(
      (TestBed.inject(DOORWAY_ADDRESS_RESOLVER) as FederationPeerResolver).peers()
    ).toHaveLength(2);
  });

  it('routes the real same-origin read to a discovered sibling with credentials omitted', async () => {
    const calls = await read(
      new HttpRequest('GET', `${primary}/db/content/manifesto`, { context: context() })
    );
    expect(calls.map(r => r.url)).toEqual([
      `${primary}/db/content/manifesto`,
      `${sibling}/db/content/manifesto`,
    ]);
    expect(calls.every(r => !r.withCredentials && r.credentials === 'omit')).toBe(true);
  });

  it.each([
    new HttpRequest('GET', `${primary}/db/content/manifesto`, {
      context: context(),
      params: new HttpParams().set('token', 'secret'),
    }),
    new HttpRequest('GET', 'https://%', { context: context() }),
    new HttpRequest('GET', `${primary}/db/content/manifesto`),
    new HttpRequest('GET', `${primary}/db/content/manifesto`, {
      context: context(),
      headers: new HttpHeaders({ Authorization: 'Bearer secret' }),
    }),
    new HttpRequest('GET', `${primary}/db/content/manifesto`, {
      context: context(),
      withCredentials: true,
    }),
    new HttpRequest('GET', `${primary}/db/private/value`, { context: context() }),
    new HttpRequest('GET', 'https://external.example/db/content/manifesto', { context: context() }),
    new HttpRequest('POST', `${primary}/db/content/manifesto`, {}, { context: context() }),
  ])('never expands an ineligible request to discovered peers', async request => {
    expect((await read(request)).some(r => r.url.startsWith(sibling))).toBe(false);
  });

  it('waits another interval after a failed primary reprobe', async () => {
    let now = 100000;
    vi.spyOn(Date, 'now').mockImplementation(() => now);
    const request = () =>
      new HttpRequest('GET', `${primary}/db/content/manifesto`, { context: context() });
    await read(request());
    now += 31000;
    expect(await read(request())).toHaveLength(2);
    now += 1000;
    expect((await read(request()))[0].url.startsWith(sibling)).toBe(true);
  });

  it('keeps public stickiness separate from writes and reprobes primary despite continued reads', async () => {
    let now = 100000;
    vi.spyOn(Date, 'now').mockImplementation(() => now);
    const request = () =>
      new HttpRequest('GET', `${primary}/blob/sha256-abc`, { context: context() });
    await read(request());
    now += 20000;
    expect((await read(request()))[0].url.startsWith(sibling)).toBe(true);
    expect((await read(new HttpRequest('POST', '/db/content', {})))[0].url).toBe(
      `${primary}/db/content`
    );
    now += 11000;
    expect((await read(request(), 'https://none.example'))[0].url.startsWith(primary)).toBe(true);
  });
});
