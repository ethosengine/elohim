import { of } from 'rxjs';
import { describe, expect, it, vi } from 'vitest';

import type { ILamadStorageClient } from '../interfaces/storage.interface';

import {
  currentBlobUrlContext,
  resolveBlobHref,
  withOriginRelativeBlobUrls,
} from './blob-url';

const SIDECAR = 'http://localhost:8090';

/** A delegate that behaves like the direct-mode (native sidecar) client. */
function sidecarDelegate(): ILamadStorageClient {
  return {
    getBlobUrl: (hash: string) => `${SIDECAR}/blob/${hash}`,
    getStorageBaseUrl: () => SIDECAR,
    getContentEngagement: vi.fn(() => of({} as never)),
  };
}

describe('resolveBlobHref', () => {
  it('returns an origin-relative blob URL in doorway mode, never the local sidecar address', () => {
    const url = resolveBlobHref('sha256-abc', sidecarDelegate(), {
      mode: 'doorway',
      origin: 'https://example.test',
    });

    expect(url).toBe('https://example.test/blob/sha256-abc');
    expect(url).not.toContain('localhost:8090');
  });

  it('falls back to a root-relative path when the serving origin is unknown (SSR)', () => {
    const url = resolveBlobHref('sha256-abc', sidecarDelegate(), {
      mode: 'doorway',
      origin: undefined,
    });

    // Root-relative resolves against whichever doorway served the page.
    expect(url).toBe('/blob/sha256-abc');
    expect(url).not.toContain('localhost:8090');
  });

  it('never emits the sidecar address for a doorway page, whatever the delegate says', () => {
    const url = resolveBlobHref('sha256-c5dcc24c', sidecarDelegate(), {
      mode: 'doorway',
      origin: 'https://alpha.elohim.host',
    });

    expect(url).toBe('https://alpha.elohim.host/blob/sha256-c5dcc24c');
  });

  it('keeps the configured sidecar URL in direct mode (Tauri / native)', () => {
    const url = resolveBlobHref('sha256-abc', sidecarDelegate(), {
      mode: 'direct',
      origin: 'https://example.test',
    });

    expect(url).toBe(`${SIDECAR}/blob/sha256-abc`);
  });

  it('returns an empty string for an empty hash', () => {
    expect(resolveBlobHref('', sidecarDelegate(), { mode: 'doorway', origin: 'https://x.test' })).toBe(
      ''
    );
  });
});

describe('currentBlobUrlContext', () => {
  it('reports doorway mode with the browser origin when no native runtime is present', () => {
    const ctx = currentBlobUrlContext();

    expect(ctx.mode).toBe('doorway');
    expect(ctx.origin).toBe(globalThis.location?.origin);
  });

  it('reports direct mode when a Tauri runtime is present', () => {
    (globalThis as Record<string, unknown>)['__TAURI__'] = {};
    try {
      expect(currentBlobUrlContext().mode).toBe('direct');
    } finally {
      delete (globalThis as Record<string, unknown>)['__TAURI__'];
    }
  });

  it('reports direct mode when the runtime env pins connectionMode=direct', () => {
    (globalThis as Record<string, unknown>)['__env'] = { connectionMode: 'direct' };
    try {
      expect(currentBlobUrlContext().mode).toBe('direct');
    } finally {
      delete (globalThis as Record<string, unknown>)['__env'];
    }
  });
});

describe('withOriginRelativeBlobUrls', () => {
  it('overrides getBlobUrl so lamad consumers never see the sidecar address', () => {
    const client = withOriginRelativeBlobUrls(sidecarDelegate(), () => ({
      mode: 'doorway',
      origin: 'https://example.test',
    }));

    expect(client.getBlobUrl('sha256-abc')).toBe('https://example.test/blob/sha256-abc');
  });

  it('leaves the API base URL and engagement projection on the delegate', () => {
    const delegate = sidecarDelegate();
    const client = withOriginRelativeBlobUrls(delegate, () => ({
      mode: 'doorway',
      origin: 'https://example.test',
    }));

    // getStorageBaseUrl drives /db and /api calls — unchanged by this wrapper.
    expect(client.getStorageBaseUrl()).toBe(SIDECAR);

    client.getContentEngagement('some-content');
    expect(delegate.getContentEngagement).toHaveBeenCalledWith('some-content');
  });
});
