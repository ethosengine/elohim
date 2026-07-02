import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting, HttpTestingController } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import { firstValueFrom } from 'rxjs';

import {
  EprResolverService,
  isContentAddress,
  normalizeContentAddress,
  type StepRef,
  type CrossPathMatch,
} from './epr-resolver.service';
import { StorageClientService } from './storage-client.service';
import { BUNDLE_ROUTE_CONTEXT, type EprRef } from '@elohim/service';
import { vi } from 'vitest';

import type { EprHead } from '../models/epr-head.model';

/**
 * Encode an EPR Head as the JSON bytes the head endpoint returns (arraybuffer).
 * Built via a test-realm `ArrayBuffer` (not TextEncoder's buffer) so Angular's
 * HttpTestingController `instanceof ArrayBuffer` check passes — the head JSON is
 * ASCII, so charCode copy is exact.
 */
function headBuffer(head: Partial<EprHead>): ArrayBuffer {
  const json = JSON.stringify(head);
  const buffer = new ArrayBuffer(json.length);
  const view = new Uint8Array(buffer);
  for (let i = 0; i < json.length; i++) view[i] = json.charCodeAt(i);
  return buffer;
}

const sampleHead: Partial<EprHead> = {
  version: 1,
  id: 'manifesto',
  content: '',
  lamad: {
    title: 'Manifesto',
    contentType: 'article',
    description: 'The foundational document',
    contentFormat: 'markdown',
    tags: [],
  },
  shefa: {},
  qahal: { reach: 'public' },
  relationships: [],
};

describe('EprResolverService', () => {
  let service: EprResolverService;
  let storageSpy: any;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    storageSpy = {
      getBlobUrl: vi.fn(),
      getStorageBaseUrl: vi.fn(),
      getContent: vi.fn(),
    };
    storageSpy.getStorageBaseUrl.mockReturnValue('https://doorway.host');
    storageSpy.getBlobUrl.mockImplementation((hash: string) => `https://doorway.host/blob/${hash}`);

    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: StorageClientService, useValue: storageSpy },
        // Shell context: owns /epr universal route, claims no content type
        {
          provide: BUNDLE_ROUTE_CONTEXT,
          useValue: { claims: [], ownsUniversalRoute: true },
        },
      ],
    });
    service = TestBed.inject(EprResolverService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  /** Match + answer the head request the head-first resolver issues. */
  function flushHead(id: string, buffer: ArrayBuffer): void {
    const req = httpMock.expectOne(`https://doorway.host/epr-head/${id}`);
    expect(req.request.method).toBe('GET');
    req.flush(buffer);
  }

  function flushHeadError(id: string, status: number, statusText: string): void {
    const req = httpMock.expectOne(`https://doorway.host/epr-head/${id}`);
    req.flush(new ArrayBuffer(0), { status, statusText });
  }

  // ── isContentAddress ────────────────────────────────────────────────────

  describe('isContentAddress', () => {
    it('recognizes CIDv1 base32', () => {
      expect(isContentAddress('bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku')).toBe(
        true
      );
    });

    it('recognizes CIDv0 base58', () => {
      expect(isContentAddress('QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG')).toBe(true);
    });

    it('recognizes sha256-{hex}', () => {
      expect(
        isContentAddress('sha256-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890')
      ).toBe(true);
    });

    it('recognizes sha256:{hex}', () => {
      expect(
        isContentAddress('sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890')
      ).toBe(true);
    });

    it('recognizes raw 64-char hex', () => {
      expect(
        isContentAddress('abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890')
      ).toBe(true);
    });

    it('rejects empty string', () => {
      expect(isContentAddress('')).toBe(false);
    });

    it('rejects plain text', () => {
      expect(isContentAddress('hello world')).toBe(false);
    });

    it('rejects short hex', () => {
      expect(isContentAddress('abcdef123')).toBe(false);
    });
  });

  // ── normalizeContentAddress ─────────────────────────────────────────────

  describe('normalizeContentAddress', () => {
    it('passes through CIDv1 base32 unchanged', () => {
      const cid = 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku';
      expect(normalizeContentAddress(cid)).toBe(cid);
    });

    it('passes through CIDv0 base58 unchanged', () => {
      const cid = 'QmYwAPJzv5CZsnA625s3Xf2nemtYgPpHdWEz79ojWnPbdG';
      expect(normalizeContentAddress(cid)).toBe(cid);
    });

    it('passes through sha256-{hex} unchanged', () => {
      const hash = 'sha256-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
      expect(normalizeContentAddress(hash)).toBe(hash);
    });

    it('converts sha256:{hex} to sha256-{hex}', () => {
      const input = 'sha256:abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
      const expected = 'sha256-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
      expect(normalizeContentAddress(input)).toBe(expected);
    });

    it('adds sha256- prefix to raw 64-char hex', () => {
      const hex = 'abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
      expect(normalizeContentAddress(hex)).toBe(`sha256-${hex}`);
    });

    it('returns unrecognized input unchanged', () => {
      expect(normalizeContentAddress('some-content-id')).toBe('some-content-id');
    });
  });

  // ── resolveUrl ─────────────────────────────────────────────────────────

  describe('resolveUrl', () => {
    it('returns an HTTP URL and /epr route for a head-tier ref (shell context)', () => {
      const result = service.resolveUrl('epr:manifesto-foundations');
      expect(result.ref.id).toBe('manifesto-foundations');
      expect(result.url).toBe('https://doorway.host/db/content/manifesto-foundations');
      // Shell owns /epr universal route
      expect(result.route).toEqual(['/epr', 'manifesto-foundations']);
      expect(result.href).toBe('/epr/manifesto-foundations');
    });

    it('returns empty url and null route for a blob-tier ref', () => {
      const result = service.resolveUrl('epr:manifesto-foundations/blob');
      expect(result.ref.tier).toBe('blob');
      expect(result.route).toBeNull();
    });

    it('includes blob hash in url when provided', () => {
      const hash = 'sha256-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
      storageSpy.getBlobUrl.mockReturnValue(`https://doorway.host/blob/${hash}`);
      const result = service.resolveUrl('epr:manifesto-foundations/blob', hash);
      expect(result.url).toContain('/blob/');
    });
  });

  // ── resolve (head-first) ──────────────────────────────────────────────────

  describe('resolve', () => {
    it('resolves preview metadata + route/href from the HEAD, never the body', async () => {
      const promise = firstValueFrom(service.resolve('epr:manifesto'));
      flushHead('manifesto', headBuffer(sampleHead));
      // The reach-gated body endpoint is NEVER touched by preview resolution.
      httpMock.expectNone('https://doorway.host/db/content/manifesto');
      const result = await promise;

      expect(result).not.toBeNull();
      expect(result!.ref.id).toBe('manifesto');
      expect(result!.content.title).toBe('Manifesto');
      expect(result!.content.description).toBe('The foundational document');
      expect(result!.content.reach).toBe('public');
      expect(result!.blobUrl).toBeNull();
      // Shell owns /epr — article is unclaimed → universal route
      expect(result!.route).toEqual(['/epr', 'manifesto']);
      expect(result!.href).toBe('/epr/manifesto');
    });

    it('returns null when the HEAD is forbidden (403) — a gated body cannot fail a preview differently', async () => {
      const promise = firstValueFrom(service.resolve('epr:gated'));
      flushHeadError('gated', 403, 'Forbidden');
      expect(await promise).toBeNull();
    });

    it('returns null when the HEAD is missing (404)', async () => {
      const promise = firstValueFrom(service.resolve('epr:missing-content'));
      flushHeadError('missing-content', 404, 'Not Found');
      expect(await promise).toBeNull();
    });

    it('does NOT call the storage body endpoint (getContent) at all', async () => {
      const promise = firstValueFrom(service.resolve('epr:manifesto'));
      flushHead('manifesto', headBuffer(sampleHead));
      await promise;
      expect(storageSpy.getContent).not.toHaveBeenCalled();
    });
  });

  // ── resolvePreview (typed degradation) ────────────────────────────────────

  describe('resolvePreview', () => {
    it('maps a resolved HEAD to preview fields (title, reach, contentType, route/href)', async () => {
      const promise = firstValueFrom(service.resolvePreview('epr:manifesto'));
      flushHead('manifesto', headBuffer(sampleHead));
      const outcome = await promise;

      expect(outcome.state).toBe('resolved');
      if (outcome.state !== 'resolved') throw new Error('expected resolved');
      expect(outcome.preview.title).toBe('Manifesto');
      expect(outcome.preview.description).toBe('The foundational document');
      expect(outcome.preview.reach).toBe('public');
      expect(outcome.preview.contentType).toBe('article');
      expect(outcome.preview.route).toEqual(['/epr', 'manifesto']);
      expect(outcome.preview.href).toBe('/epr/manifesto');
    });

    it('degrades to forbidden on a 403 head (unavailable at your reach)', async () => {
      const promise = firstValueFrom(service.resolvePreview('epr:gated'));
      flushHeadError('gated', 403, 'Forbidden');
      expect((await promise).state).toBe('forbidden');
    });

    it('degrades to forbidden on a 401 head', async () => {
      const promise = firstValueFrom(service.resolvePreview('epr:gated'));
      flushHeadError('gated', 401, 'Unauthorized');
      expect((await promise).state).toBe('forbidden');
    });

    it('degrades to missing on a 404 head', async () => {
      const promise = firstValueFrom(service.resolvePreview('epr:gone'));
      flushHeadError('gone', 404, 'Not Found');
      expect((await promise).state).toBe('missing');
    });

    it('degrades to error on a 5xx / network failure', async () => {
      const promise = firstValueFrom(service.resolvePreview('epr:boom'));
      flushHeadError('boom', 500, 'Server Error');
      expect((await promise).state).toBe('error');
    });

    it('falls back to the epr id as title when the head omits lamad.title', async () => {
      const promise = firstValueFrom(service.resolvePreview('epr:untitled'));
      flushHead('untitled', headBuffer({ ...sampleHead, id: 'untitled', lamad: undefined as any }));
      const outcome = await promise;
      expect(outcome.state).toBe('resolved');
      if (outcome.state !== 'resolved') throw new Error('expected resolved');
      expect(outcome.preview.title).toBe('untitled');
    });
  });

  // ── resolveInContext ────────────────────────────────────────────────────

  describe('resolveInContext', () => {
    const steps: StepRef[] = [
      { resourceId: 'intro-concept', order: 0 },
      { resourceId: 'rea-foundations', order: 1 },
      { resourceId: 'governance-basics', order: 2 },
    ];

    it('resolves to in-path step when target is in current path', () => {
      const result = service.resolveInContext('epr:rea-foundations', 'my-path', steps);
      expect(result.resolution).toBe('in-path');
      expect(result.stepIndex).toBe(1);
      // Shell owns /epr — mints universal step route
      expect(result.route).toEqual(['/epr', 'my-path']);
      expect(result.href).toBe('/epr/my-path#step/1');
    });

    it('resolves to standalone when no path context', () => {
      const result = service.resolveInContext('epr:rea-foundations', null, []);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/epr', 'rea-foundations']); // shell owns /epr
      expect(result.href).toBe('/epr/rea-foundations');
    });

    it('resolves to standalone when target not in current path', () => {
      const result = service.resolveInContext('epr:unknown-content', 'my-path', steps);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/epr', 'unknown-content']);
    });

    it('resolves to cross-path when matches provided', () => {
      const crossMatches: CrossPathMatch[] = [{ pathId: 'other-path', stepIndex: 3 }];
      const result = service.resolveInContext(
        'epr:unknown-content',
        'my-path',
        steps,
        crossMatches
      );
      expect(result.resolution).toBe('cross-path');
      expect(result.crossPath).toEqual({ pathId: 'other-path', stepIndex: 3 });
      // Shell mints universal cross-path step route
      expect(result.route).toEqual(['/epr', 'other-path']);
      expect(result.href).toBe('/epr/other-path#step/3');
    });

    it('prefers in-path over cross-path', () => {
      const crossMatches: CrossPathMatch[] = [{ pathId: 'other-path', stepIndex: 5 }];
      const result = service.resolveInContext(
        'epr:rea-foundations',
        'my-path',
        steps,
        crossMatches
      );
      expect(result.resolution).toBe('in-path');
      expect(result.stepIndex).toBe(1);
    });

    it('falls back to standalone when cross-path matches empty', () => {
      const result = service.resolveInContext('epr:unknown', 'my-path', steps, []);
      expect(result.resolution).toBe('standalone');
    });

    it('handles bare ID without epr: prefix', () => {
      const result = service.resolveInContext('rea-foundations', 'my-path', steps);
      expect(result.resolution).toBe('in-path');
      expect(result.stepIndex).toBe(1);
    });

    it('in-path target degrades to /epr in the shell (no path claim)', () => {
      const result = service.resolveInContext('epr:rea-foundations', 'my-path', steps);
      expect(result.resolution).toBe('in-path');
      expect(result.stepIndex).toBe(1);
      expect(result.route).toEqual(['/epr', 'my-path']); // shell mints universal
      expect(result.href).toBe('/epr/my-path#step/1');
    });

    it('uses eprToRoute for known path-type URIs', () => {
      // epr:elohim-protocol#step/2 has a path fragment — shell has ownsUniversalRoute
      const result = service.resolveInContext('epr:elohim-protocol#step/2', null, []);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/epr', 'elohim-protocol']);
      expect(result.href).toBe('/epr/elohim-protocol#step/2');
    });

    describe('with a path-claiming bundle context (lamad)', () => {
      let lamadService: EprResolverService;

      beforeEach(() => {
        // Re-configure TestBed with a BUNDLE_ROUTE_CONTEXT that claims 'path'
        TestBed.resetTestingModule();
        const lamadStorageSpy = {
          getBlobUrl: vi.fn(),
          getStorageBaseUrl: vi.fn(),
          getContent: vi.fn(),
        };
        lamadStorageSpy.getStorageBaseUrl.mockReturnValue('https://doorway.host');
        lamadStorageSpy.getBlobUrl.mockImplementation(
          (hash: string) => `https://doorway.host/blob/${hash}`
        );

        TestBed.configureTestingModule({
          providers: [
            provideHttpClient(),
            provideHttpClientTesting(),
            { provide: StorageClientService, useValue: lamadStorageSpy },
            {
              provide: BUNDLE_ROUTE_CONTEXT,
              useValue: {
                claims: [
                  {
                    contentType: 'path',
                    commands: (ref: EprRef) =>
                      ref.fragment?.type === 'step'
                        ? ['/path', ref.id, 'step', ref.fragment.value]
                        : ['/path', ref.id],
                  },
                ],
              },
            },
          ],
        });
        lamadService = TestBed.inject(EprResolverService);
      });

      it('mints in-bundle step commands for in-path targets', () => {
        const result = lamadService.resolveInContext('epr:rea-foundations', 'my-path', steps);
        expect(result.route).toEqual(['/path', 'my-path', 'step', '1']);
      });

      it('mints no commands for unclaimed content (cross-bundle href)', () => {
        const result = lamadService.resolveInContext('epr:unknown-content', null, []);
        expect(result.route).toBeNull();
        expect(result.href).toBe('/epr/unknown-content');
      });
    });
  });
});
