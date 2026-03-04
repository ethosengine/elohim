import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { TestBed } from '@angular/core/testing';

import {
  EprResolverService,
  isContentAddress,
  normalizeContentAddress,
  type StepRef,
  type CrossPathMatch,
} from './epr-resolver.service';
import { StorageClientService } from './storage-client.service';
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
      expect(result.route).toEqual(['/lamad/path', 'my-path', 'step', '1']);
    });

    it('resolves to standalone when no path context', () => {
      const result = service.resolveInContext('epr:rea-foundations', null, []);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/resource', 'rea-foundations']);
    });

    it('resolves to standalone when target not in current path', () => {
      const result = service.resolveInContext('epr:unknown-content', 'my-path', steps);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/resource', 'unknown-content']);
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
      expect(result.route).toEqual(['/lamad/path', 'other-path', 'step', '3']);
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

    it('uses eprToRoute for known path-type URIs', () => {
      // epr:elohim-protocol#step/2 has a path fragment
      const result = service.resolveInContext('epr:elohim-protocol#step/2', null, []);
      expect(result.resolution).toBe('standalone');
      expect(result.route).toEqual(['/lamad/path', 'elohim-protocol', 'step', '2']);
    });
  });
});
