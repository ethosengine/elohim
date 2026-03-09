/**
 * AccountPackageService Tests
 *
 * Tests import/export of account packages via the /account/* API.
 */

import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { AccountPackageService } from './account-package.service';

import type {
  AccountImportResultView,
  AccountPackageInputView,
  AccountPackageView,
} from '@elohim/storage-client/generated';

describe('AccountPackageService', () => {
  let service: AccountPackageService;
  let httpMock: HttpTestingController;

  const mockPackageInput: AccountPackageInputView = {
    schemaVersion: 1,
    identity: {
      humanId: 'susan-spouse',
      displayName: 'Susan Dowell',
      category: 'core-family',
      profileReach: 'neighborhood',
      bio: null,
      location: null,
      affinities: ['education', 'health'],
      organizations: [],
    },
    content: [
      { contentId: 'concept-123', reach: 'local', reason: 'steward', stewardRatio: 0.6 },
      { contentId: 'concept-456', reach: 'neighborhood', reason: 'affinity', stewardRatio: null },
    ],
    relationships: [
      {
        targetId: 'matthew-manager',
        relationshipType: 'spouse',
        intimacyLevel: 'intimate',
        isBidirectional: true,
        reach: 'private',
      },
    ],
    stewardship: [
      { contentCategory: 'education', allocationRatio: 0.6, contributionType: 'inherited' },
    ],
    collectives: [
      { collectiveId: 'household-dowell', roleContext: null, intimacyLevel: 'intimate' },
    ],
    conductorGroup: 0,
    manifest: {
      version: '1.0.0',
      generatedAt: '2026-03-03T00:00:00Z',
      sourceStory: 'genesis',
      contentHash: null,
    },
  };

  const mockImportResult: AccountImportResultView = {
    humanId: 'susan-spouse',
    contentUpdated: 2,
    relationshipsCreated: 1,
    stewardshipCreated: 5,
    collectivesJoined: 1,
    errors: [],
  };

  const mockExportedPackage: AccountPackageView = {
    identity: {
      humanId: 'susan-spouse',
      displayName: 'susan-spouse',
      category: null,
      profileReach: null,
      bio: null,
      location: null,
      affinities: [],
      organizations: [],
    },
    content: [{ contentId: 'concept-123', reach: 'local', reason: null, stewardRatio: null }],
    relationships: [
      {
        targetId: 'matthew-manager',
        relationshipType: 'spouse',
        intimacyLevel: 'intimate',
        isBidirectional: true,
        reach: 'private',
      },
    ],
    stewardship: [
      { contentCategory: 'education', allocationRatio: 0.6, contributionType: 'inherited' },
    ],
    collectives: [
      { collectiveId: 'household-dowell', roleContext: null, intimacyLevel: 'intimate' },
    ],
    manifest: {
      version: '1.0.0',
      generatedAt: '2026-03-03T00:00:00Z',
      sourceStory: 'export',
      contentHash: null,
    },
  };

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [AccountPackageService, provideHttpClient(), provideHttpClientTesting()],
    });

    service = TestBed.inject(AccountPackageService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('importPackage', () => {
    it('should POST account package and return import result', () => {
      service.importPackage(mockPackageInput).subscribe(result => {
        expect(result.humanId).toBe('susan-spouse');
        expect(result.contentUpdated).toBe(2);
        expect(result.relationshipsCreated).toBe(1);
        expect(result.stewardshipCreated).toBe(5);
        expect(result.errors).toEqual([]);
      });

      const req = httpMock.expectOne(request => request.url.endsWith('/account/import'));
      expect(req.request.method).toBe('POST');
      expect(req.request.body).toEqual(mockPackageInput);
      req.flush(mockImportResult);
    });

    it('should propagate HTTP errors', () => {
      service.importPackage(mockPackageInput).subscribe({
        error: (error: unknown) => {
          expect(error).toBeTruthy();
        },
      });

      const req = httpMock.expectOne(request => request.url.endsWith('/account/import'));
      req.flush('Server error', { status: 500, statusText: 'Internal Server Error' });
    });
  });

  describe('exportPackage', () => {
    it('should GET account package for a human', () => {
      service.exportPackage('susan-spouse').subscribe(pkg => {
        expect(pkg.identity.humanId).toBe('susan-spouse');
        expect(pkg.content.length).toBe(1);
        expect(pkg.relationships.length).toBe(1);
        expect(pkg.manifest.sourceStory).toBe('export');
      });

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/account/export/susan-spouse')
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockExportedPackage);
    });

    it('should encode human IDs with special characters', () => {
      service.exportPackage('human with spaces').subscribe();

      const req = httpMock.expectOne(request =>
        request.url.includes('/account/export/human%20with%20spaces')
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockExportedPackage);
    });

    it('should propagate HTTP errors', () => {
      service.exportPackage('nonexistent').subscribe({
        error: (error: unknown) => {
          expect(error).toBeTruthy();
        },
      });

      const req = httpMock.expectOne(request =>
        request.url.endsWith('/account/export/nonexistent')
      );
      req.flush('Not found', { status: 404, statusText: 'Not Found' });
    });
  });
});
