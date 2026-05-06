import { HttpRequest, HttpHandlerFn, HttpEvent } from '@angular/common/http';
import { Observable, of } from 'rxjs';
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';

import { apiBaseUrlInterceptor } from './api-base-url.interceptor';

vi.mock('../../../environments/environment', () => ({
  environment: {
    client: { doorwayUrl: 'https://doorway-alpha.elohim.host' },
    holochain: { storageUrl: 'http://localhost:8090' },
  },
}));

function captureUrl(): { handler: HttpHandlerFn; received: () => string | undefined } {
  let captured: string | undefined;
  const handler: HttpHandlerFn = (req): Observable<HttpEvent<unknown>> => {
    captured = req.url;
    return of({} as HttpEvent<unknown>);
  };
  return { handler, received: () => captured };
}

function setOrigin(origin: string): void {
  Object.defineProperty(globalThis, 'location', {
    value: { origin } as Location,
    configurable: true,
    writable: true,
  });
}

function setTauri(present: boolean): void {
  if (present) {
    (globalThis as unknown as { __TAURI__?: object }).__TAURI__ = {};
  } else {
    delete (globalThis as unknown as { __TAURI__?: object }).__TAURI__;
  }
}

describe('apiBaseUrlInterceptor', () => {
  const originalLocation = globalThis.location;

  afterEach(() => {
    Object.defineProperty(globalThis, 'location', {
      value: originalLocation,
      configurable: true,
      writable: true,
    });
    setTauri(false);
  });

  describe('cross-origin SPA host (alpha/prod)', () => {
    beforeEach(() => setOrigin('https://alpha.elohim.host'));

    it('prefixes relative /api/v1/* with doorway URL', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('https://doorway-alpha.elohim.host/api/v1/mastery');
    });

    it('prefixes /db/, /blob/, /apps/, /health', () => {
      const cases = ['/db/content/123', '/blob/sha256-abc', '/apps/foo/index.html', '/health'];
      for (const path of cases) {
        const { handler, received } = captureUrl();
        apiBaseUrlInterceptor(new HttpRequest('GET', path), handler).subscribe();
        expect(received()).toBe(`https://doorway-alpha.elohim.host${path}`);
      }
    });

    it('leaves absolute URLs untouched', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(
        new HttpRequest('GET', 'https://other.example.com/api/v1/mastery'),
        handler
      ).subscribe();
      expect(received()).toBe('https://other.example.com/api/v1/mastery');
    });

    it('leaves non-doorway paths untouched', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/assets/icon.svg'), handler).subscribe();
      expect(received()).toBe('/assets/icon.svg');
    });
  });

  describe('Tauri desktop', () => {
    beforeEach(() => {
      setOrigin('https://tauri.localhost');
      setTauri(true);
    });

    it('prefixes /api/v1/* with storage sidecar URL (not doorway)', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('http://localhost:8090/api/v1/mastery');
    });

    it('prefixes /db/, /blob/ with storage sidecar URL', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/db/content/123'), handler).subscribe();
      expect(received()).toBe('http://localhost:8090/db/content/123');
    });

    it('does not use doorway URL even when set in environment', () => {
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/health'), handler).subscribe();
      expect(received()).not.toContain('doorway');
    });
  });

  describe('Eclipse Che environment', () => {
    it('does not prefix when in Che (.devspaces.)', () => {
      setOrigin('https://workspace-angular-dev.devspaces.example.com');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('/api/v1/mastery');
    });

    it('does not prefix when in Che (.code.ethosengine.com)', () => {
      setOrigin('https://workspace-angular-dev.code.ethosengine.com');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('POST', '/api/v1/mastery', {}), handler).subscribe();
      expect(received()).toBe('/api/v1/mastery');
    });
  });

  describe('local dev server', () => {
    it('does not prefix when on localhost', () => {
      setOrigin('http://localhost:4200');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/api/v1/mastery'), handler).subscribe();
      expect(received()).toBe('/api/v1/mastery');
    });

    it('does not prefix when on 127.0.0.1', () => {
      setOrigin('http://127.0.0.1:4200');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/api/v1/mastery'), handler).subscribe();
      expect(received()).toBe('/api/v1/mastery');
    });
  });

  describe('same-origin as doorway', () => {
    it('does not prefix when origin already matches doorway', () => {
      setOrigin('https://doorway-alpha.elohim.host');
      const { handler, received } = captureUrl();
      apiBaseUrlInterceptor(new HttpRequest('GET', '/api/v1/mastery'), handler).subscribe();
      expect(received()).toBe('/api/v1/mastery');
    });
  });
});
