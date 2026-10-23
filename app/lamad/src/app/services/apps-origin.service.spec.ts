import { DOCUMENT } from '@angular/common';
import { TestBed } from '@angular/core/testing';

import { afterEach, describe, expect, it, vi } from 'vitest';

import { AUTH_DISCOVERY_PATH, AppsOriginService } from './apps-origin.service';

/** A window-shaped stand-in: the page's location, a session store and a fetch. */
function fakeWindow(origin: string, body: unknown, status = 200) {
  const url = new URL(origin);
  const store = new Map<string, string>();
  const fetch = vi.fn().mockResolvedValue({
    ok: status >= 200 && status < 300,
    status,
    json: () => Promise.resolve(body),
  });
  return {
    location: { origin: url.origin, host: url.host, protocol: url.protocol },
    sessionStorage: {
      getItem: (k: string) => store.get(k) ?? null,
      setItem: (k: string, v: string) => store.set(k, v),
    },
    fetch,
    setTimeout: (fn: () => void, ms: number) => setTimeout(fn, ms),
    clearTimeout: (id: ReturnType<typeof setTimeout>) => clearTimeout(id),
    store,
  };
}

function serviceFor(win: ReturnType<typeof fakeWindow>): AppsOriginService {
  TestBed.configureTestingModule({
    providers: [{ provide: DOCUMENT, useValue: { defaultView: win } }],
  });
  return TestBed.inject(AppsOriginService);
}

describe('AppsOriginService', () => {
  afterEach(() => TestBed.resetTestingModule());

  it('reads the paired apps origin from discovery, once per session', async () => {
    const win = fakeWindow('http://elohim.localhost:8888', {
      apps: true,
    });
    const service = serviceFor(win);

    expect(await service.resolve()).toBe('http://apps.elohim.localhost:8888');
    expect(await service.resolve()).toBe('http://apps.elohim.localhost:8888');
    expect(win.fetch).toHaveBeenCalledTimes(1);
    expect(win.fetch.mock.calls[0][0]).toBe(AUTH_DISCOVERY_PATH);
    expect(service.appsOrigin()).toBe('http://apps.elohim.localhost:8888');

    // A second bundle on the same origin (the lamad bundle beside the shell)
    // reads the session's answer instead of asking again.
    TestBed.resetTestingModule();
    const again = serviceFor(win);
    expect(again.appsOrigin()).toBe('http://apps.elohim.localhost:8888');
  });

  it('keeps same-origin /apps/ URLs when the name is not apps-paired', async () => {
    const win = fakeWindow('http://elohim.localhost:8888', {
      version: 1,
      portal: '/threshold/',
    });
    const service = serviceFor(win);
    expect(await service.resolve()).toBeNull();
    expect(service.appUrl('quiz', 'index.html', null)).toBe('/apps/quiz/index.html');
    expect(win.store.get('elohim-apps-origin:http://elohim.localhost:8888')).toBe('0');
  });

  it('keeps same-origin URLs when there is no discovery document (peer-native, Tauri)', async () => {
    const service = serviceFor(fakeWindow('http://localhost:8090', null, 404));
    expect(await service.resolve()).toBeNull();
  });

  it('derives the apps origin from the page itself, on https too', async () => {
    const service = serviceFor(fakeWindow('https://alpha.elohim.host', { apps: true }));
    expect(await service.resolve()).toBe('https://apps.alpha.elohim.host');
  });

  it('pairs only on apps === true, and never follows a named origin', async () => {
    for (const doc of [
      { apps: 'true' },
      { apps: 1 },
      { apps: false },
      { appsOrigin: 'https://evil.example' },
    ]) {
      TestBed.resetTestingModule();
      const service = serviceFor(fakeWindow('http://elohim.localhost:8888', doc));
      expect(await service.resolve()).toBeNull();
    }
  });

  it('builds the apps-origin URL for a bundle entry', () => {
    const service = serviceFor(fakeWindow('http://elohim.localhost:8888', {}));
    expect(service.appUrl('quiz', 'index.html', 'http://apps.elohim.localhost:8888')).toBe(
      'http://apps.elohim.localhost:8888/apps/quiz/index.html'
    );
  });
});
