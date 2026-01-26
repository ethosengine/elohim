/**
 * Doorway Registry Service Tests
 *
 * Tests gateway discovery, selection, and health checking.
 */

import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { DoorwayRegistryService } from './doorway-registry.service';
import { HolochainClientService } from '../../elohim/services/holochain-client.service';
import { signal } from '@angular/core';
import type { DoorwayInfo } from '../models/doorway.model';

describe('DoorwayRegistryService', () => {
  let service: DoorwayRegistryService;
  let httpMock: HttpTestingController;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let localStorageMock: { [key: string]: string };

  const mockDoorway: DoorwayInfo = {
    id: 'doorway-1',
    name: 'Test Doorway',
    url: 'https://doorway.example.com',
    description: 'A test doorway',
    region: 'north-america',
    operator: 'Test Operator',
    features: ['premium-content', 'high-availability'],
    status: 'online',
    registrationOpen: true,
    vouchCount: 10,
  };

  const mockDoorway2: DoorwayInfo = {
    id: 'doorway-2',
    name: 'Secondary Doorway',
    url: 'https://doorway2.example.com',
    description: 'Another test doorway',
    region: 'europe',
    operator: 'EU Operator',
    features: ['premium-content'],
    status: 'online',
    registrationOpen: true,
    vouchCount: 5,
  };

  beforeEach(() => {
    // Setup localStorage mock
    localStorageMock = {};
    spyOn(localStorage, 'getItem').and.callFake(
      (key: string) => localStorageMock[key] || null
    );
    spyOn(localStorage, 'setItem').and.callFake(
      (key: string, value: string) => {
        localStorageMock[key] = value;
      }
    );
    spyOn(localStorage, 'removeItem').and.callFake((key: string) => {
      delete localStorageMock[key];
    });

    // Create mock Holochain client
    const isConnectedSignal = signal(false);
    mockHolochainClient = jasmine.createSpyObj(
      'HolochainClientService',
      ['callZome'],
      {
        isConnected: jasmine.createSpy().and.callFake(() => isConnectedSignal()),
      }
    );
    mockHolochainClient.callZome.and.returnValue(
      Promise.resolve({ success: false, error: 'Not connected' })
    );

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [
        DoorwayRegistryService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
      ],
    });

    service = TestBed.inject(DoorwayRegistryService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  // ==========================================================================
  // Initial State Tests
  // ==========================================================================

  describe('initial state', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should start with empty doorways list', () => {
      expect(service.doorways().length).toBe(0);
    });

    it('should start with no selection', () => {
      expect(service.selected()).toBeNull();
      expect(service.hasSelection()).toBe(false);
    });

    it('should start not loading', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should start without error', () => {
      expect(service.error()).toBeNull();
    });

    it('should expose computed signals', () => {
      expect(service.selectedUrl()).toBeNull();
      expect(service.doorwaysWithHealth()).toEqual([]);
    });
  });

  // ==========================================================================
  // Selection Tests
  // ==========================================================================

  describe('selectDoorway', () => {
    it('should set selected doorway', () => {
      service.selectDoorway(mockDoorway);

      expect(service.selected()).not.toBeNull();
      expect(service.selected()?.doorway).toEqual(mockDoorway);
      expect(service.hasSelection()).toBe(true);
    });

    it('should expose selected URL', () => {
      service.selectDoorway(mockDoorway);

      expect(service.selectedUrl()).toBe('https://doorway.example.com');
    });

    it('should persist selection to localStorage', () => {
      service.selectDoorway(mockDoorway);

      expect(localStorage.setItem).toHaveBeenCalled();
    });

    it('should mark explicit selection', () => {
      service.selectDoorway(mockDoorway, true);

      expect(service.selected()?.isExplicit).toBe(true);
    });

    it('should mark implicit selection', () => {
      service.selectDoorway(mockDoorway, false);

      expect(service.selected()?.isExplicit).toBe(false);
    });

    it('should record selection timestamp', () => {
      const before = new Date().toISOString();
      service.selectDoorway(mockDoorway);
      const after = new Date().toISOString();

      const selection = service.selected();
      expect(selection?.selectedAt).toBeDefined();
      // Verify timestamp is between before and after
      expect(selection?.selectedAt! >= before).toBe(true);
      expect(selection?.selectedAt! <= after).toBe(true);
    });
  });

  describe('clearSelection', () => {
    it('should clear selected doorway', () => {
      service.selectDoorway(mockDoorway);
      expect(service.hasSelection()).toBe(true);

      service.clearSelection();

      expect(service.selected()).toBeNull();
      expect(service.hasSelection()).toBe(false);
      expect(service.selectedUrl()).toBeNull();
    });

    it('should remove from localStorage', () => {
      service.selectDoorway(mockDoorway);
      service.clearSelection();

      expect(localStorage.removeItem).toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Lookup Tests
  // ==========================================================================

  describe('getDoorwayById', () => {
    it('should return doorway by ID', async () => {
      // Manually set doorways via loadDoorways fallback
      const loadPromise = service.loadDoorways();

      // Let it fall through to bootstrap list
      const requests = httpMock.match(() => true);
      requests.forEach(r => r.error(new ProgressEvent('error')));

      await loadPromise;

      // Bootstrap doorways should be available
      const doorways = service.doorways();
      if (doorways.length > 0) {
        const found = service.getDoorwayById(doorways[0].id);
        expect(found).toBeDefined();
      }
    });

    it('should return undefined for unknown ID', () => {
      const found = service.getDoorwayById('unknown-id');
      expect(found).toBeUndefined();
    });
  });

  describe('getDoorwayByUrl', () => {
    it('should return doorway by URL', async () => {
      const loadPromise = service.loadDoorways();

      const requests = httpMock.match(() => true);
      requests.forEach(r => r.error(new ProgressEvent('error')));

      await loadPromise;

      const doorways = service.doorways();
      if (doorways.length > 0) {
        const found = service.getDoorwayByUrl(doorways[0].url);
        expect(found).toBeDefined();
      }
    });

    it('should return undefined for unknown URL', () => {
      const found = service.getDoorwayByUrl('https://unknown.example.com');
      expect(found).toBeUndefined();
    });
  });

  // ==========================================================================
  // Load Doorways Tests
  // ==========================================================================

  describe('loadDoorways', () => {
    it('should set loading state while loading', async () => {
      const loadPromise = service.loadDoorways();

      // Flush all pending requests
      const requests = httpMock.match(() => true);
      requests.forEach(r => r.error(new ProgressEvent('error')));

      await loadPromise;
      expect(service.isLoading()).toBe(false);
    });

    it('should try DHT first when Holochain connected', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [mockDoorway, mockDoorway2] })
      );

      const result = await service.loadDoorways();

      expect(mockHolochainClient.callZome).toHaveBeenCalledWith(
        jasmine.objectContaining({
          zomeName: 'infrastructure',
          fnName: 'get_doorways_by_region',
        })
      );
      expect(result.length).toBe(2);
    });

    it('should fall back to REST API when DHT fails', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: false, error: 'DHT error' })
      );

      const loadPromise = service.loadDoorways();

      // Wait for async operations to settle, then flush pending requests
      await new Promise(resolve => setTimeout(resolve, 0));

      // Flush all pending registry requests
      const requests = httpMock.match(req => req.url.includes('/registry/doorways'));
      requests.forEach(r => r.flush([mockDoorway]));

      // Flush any remaining requests
      const remaining = httpMock.match(() => true);
      remaining.forEach(r => r.error(new ProgressEvent('error')));

      const result = await loadPromise;
      expect(result.length).toBeGreaterThan(0);
    });

    it('should use bootstrap list as last resort', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(false);

      const loadPromise = service.loadDoorways();

      // Fail all HTTP requests
      const requests = httpMock.match(() => true);
      requests.forEach(r => r.error(new ProgressEvent('error')));

      const result = await loadPromise;

      // Should get bootstrap doorways
      expect(result.length).toBeGreaterThan(0);
    });

    it('should cache results', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [mockDoorway] })
      );

      await service.loadDoorways();

      expect(localStorage.setItem).toHaveBeenCalled();
    });

    it('should clear error on successful load', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [mockDoorway] })
      );

      await service.loadDoorways();

      expect(service.error()).toBeNull();
    });
  });

  // ==========================================================================
  // Health Check Tests
  // ==========================================================================

  describe('refreshHealth', () => {
    it('should do nothing if no doorways loaded', async () => {
      await service.refreshHealth();

      expect(service.doorwaysWithHealth().length).toBe(0);
    });

    it('should check health of all doorways', async () => {
      // Load doorways first
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [mockDoorway, mockDoorway2] })
      );

      await service.loadDoorways();

      // Refresh health
      const healthPromise = service.refreshHealth();

      // Respond to health checks
      const healthRequests = httpMock.match(req => req.url.includes('/health'));
      healthRequests.forEach(r =>
        r.flush({ status: 'online', registrationOpen: true })
      );

      await healthPromise;

      const withHealth = service.doorwaysWithHealth();
      expect(withHealth.length).toBe(2);
    });
  });

  // ==========================================================================
  // Validation Tests
  // ==========================================================================

  describe('validateDoorway', () => {
    it('should validate a working doorway URL', async () => {
      const validatePromise = service.validateDoorway('https://new-doorway.example.com');

      const req = httpMock.expectOne('https://new-doorway.example.com/health');
      req.flush({ status: 'online', registrationOpen: true, userCount: 100 });

      const result = await validatePromise;

      expect(result.isValid).toBe(true);
      expect(result.doorway).toBeDefined();
      expect(result.doorway?.status).toBe('online');
    });

    it('should return error for unreachable doorway', async () => {
      const validatePromise = service.validateDoorway('https://bad-doorway.example.com');

      const req = httpMock.expectOne('https://bad-doorway.example.com/health');
      req.error(new ProgressEvent('error'));

      const result = await validatePromise;

      expect(result.isValid).toBe(false);
      expect(result.error).toBeDefined();
    });

    it('should normalize URL (remove trailing slash)', async () => {
      const validatePromise = service.validateDoorway('https://doorway.example.com/');

      const req = httpMock.expectOne('https://doorway.example.com/health');
      req.flush({ status: 'online' });

      const result = await validatePromise;
      expect(result.doorway?.url).toBe('https://doorway.example.com');
    });
  });

  // ==========================================================================
  // Session Restoration Tests
  // ==========================================================================

  describe('session restoration', () => {
    it('should restore selection from localStorage on init', () => {
      // Set up localStorage before creating service
      const storedSelection = JSON.stringify({
        doorway: mockDoorway,
        selectedAt: new Date().toISOString(),
        isExplicit: true,
      });
      localStorageMock['elohim_doorway_url'] = storedSelection;

      // Re-create service to trigger restoration
      service = TestBed.inject(DoorwayRegistryService);

      // Check localStorage.getItem was called
      expect(localStorage.getItem).toHaveBeenCalled();
    });
  });

  // ==========================================================================
  // Computed Signals Tests
  // ==========================================================================

  describe('computed signals', () => {
    it('doorwaysWithHealth should include health info', async () => {
      (mockHolochainClient.isConnected as jasmine.Spy).and.returnValue(true);
      mockHolochainClient.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [mockDoorway] })
      );

      await service.loadDoorways();

      const withHealth = service.doorwaysWithHealth();
      expect(withHealth.length).toBe(1);
      expect(withHealth[0]).toEqual(
        jasmine.objectContaining({
          id: 'doorway-1',
          name: 'Test Doorway',
        })
      );
    });

    it('hasSelection should be true when selected', () => {
      expect(service.hasSelection()).toBe(false);

      service.selectDoorway(mockDoorway);

      expect(service.hasSelection()).toBe(true);
    });
  });
});
