import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';
import { HouseholdResilienceService } from './household-resilience.service';
import { StorageClientService } from '@app/elohim/services/storage-client.service';
import type { HouseholdResilienceView } from '../../generated/household-resilience-view';

describe('HouseholdResilienceService', () => {
  let service: HouseholdResilienceService;
  let httpMock: HttpTestingController;
  let storageClientSpy: { getStorageBaseUrl: ReturnType<typeof vi.fn> };

  const mockView: HouseholdResilienceView = {
    contentId: 'bafk-test-cid',
    householdsStewarding: 3,
    householdsReciprocated: 1,
    protectionStatus: 'protected',
    details: {
      stewardHouseholds: ['household-a', 'household-b', 'household-c'],
      onlinePeerCount: 5,
      healthScore: 0.92,
    },
  };

  beforeEach(() => {
    storageClientSpy = { getStorageBaseUrl: vi.fn().mockReturnValue('http://localhost:8090') };

    TestBed.configureTestingModule({
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        HouseholdResilienceService,
        { provide: StorageClientService, useValue: storageClientSpy },
      ],
    });

    service = TestBed.inject(HouseholdResilienceService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('get()', () => {
    it('calls the correct household resilience URL without viewerHouseholdId', () => {
      const contentId = 'bafk-test-cid';
      service.get(contentId).subscribe();

      const req = httpMock.expectOne(
        'http://localhost:8090/api/v1/resilience/bafk-test-cid/household',
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockView);
    });

    it('appends viewerHouseholdId as a query param when provided', () => {
      service.get('bafk-test-cid', 'household-xyz').subscribe();

      const req = httpMock.expectOne(
        'http://localhost:8090/api/v1/resilience/bafk-test-cid/household?viewerHouseholdId=household-xyz',
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockView);
    });

    it('URL-encodes contentId with special characters', () => {
      service.get('content/with spaces').subscribe();

      const req = httpMock.expectOne(req =>
        req.url.includes('content%2Fwith%20spaces'),
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockView);
    });

    it('URL-encodes viewerHouseholdId with special characters', () => {
      service.get('bafk-test-cid', 'household with spaces').subscribe();

      const req = httpMock.expectOne(req =>
        req.url.includes('viewerHouseholdId=household%20with%20spaces'),
      );
      expect(req.request.method).toBe('GET');
      req.flush(mockView);
    });

    it('returns a HouseholdResilienceView on success', () => {
      let result: HouseholdResilienceView | undefined;
      service.get('bafk-test-cid').subscribe(v => (result = v));

      const req = httpMock.expectOne(
        'http://localhost:8090/api/v1/resilience/bafk-test-cid/household',
      );
      req.flush(mockView);

      expect(result).toEqual(mockView);
    });

    it('uses the base URL from StorageClientService', () => {
      storageClientSpy.getStorageBaseUrl.mockReturnValue('https://doorway.elohim.host');
      service.get('bafk-test-cid').subscribe();

      const req = httpMock.expectOne(
        'https://doorway.elohim.host/api/v1/resilience/bafk-test-cid/household',
      );
      req.flush(mockView);

      expect(storageClientSpy.getStorageBaseUrl).toHaveBeenCalled();
    });
  });
});
