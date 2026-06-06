import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

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

describe('EprResolverService', () => {
  let service: EprResolverService;
  let storageSpy: any;

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
  });

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

  // ── resolve ─────────────────────────────────────────────────────────────

  describe('resolve', () => {
    it('returns null when content is not found', done => {
      storageSpy.getContent.mockReturnValue(of(null));
      service.resolve('epr:missing-content').subscribe(result => {
        expect(result).toBeNull();
        done();
      });
    });

    it('resolves content with route and href (shell context)', done => {
      const mockContent = {
        id: 'manifesto',
        title: 'Manifesto',
        contentType: 'article',
        contentFormat: 'markdown',
        contentBody: 'body text',
        reach: 'public',
        tags: [],
      };
      storageSpy.getContent.mockReturnValue(of(mockContent));
      service.resolve('epr:manifesto').subscribe(result => {
        expect(result).not.toBeNull();
        expect(result!.ref.id).toBe('manifesto');
        expect(result!.content).toBe(mockContent);
        expect(result!.blobUrl).toBeNull();
        // Shell owns /epr — article is unclaimed → universal route
        expect(result!.route).toEqual(['/epr', 'manifesto']);
        expect(result!.href).toBe('/epr/manifesto');
        done();
      });
    });

    it('resolves blob URL when contentBody is a content address', done => {
      const hash = 'sha256-abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890';
      storageSpy.getContent.mockReturnValue(
        of({ id: 'blob-content', contentType: 'article', contentBody: hash, reach: 'public' })
      );
      storageSpy.getBlobUrl.mockReturnValue(`https://doorway.host/blob/${hash}`);
      service.resolve('epr:blob-content').subscribe(result => {
        expect(result!.blobUrl).toBe(`https://doorway.host/blob/${hash}`);
        done();
      });
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
