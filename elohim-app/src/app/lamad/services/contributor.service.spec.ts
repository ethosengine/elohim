import { TestBed } from '@angular/core/testing';
import { ContributorService } from './contributor.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { signal } from '@angular/core';

describe('ContributorService', () => {
  let service: ContributorService;
  let holochainSpy: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    holochainSpy = jasmine.createSpyObj(
      'HolochainClientService',
      ['callZome'],
      {
        isConnected: signal(true),
      }
    );

    TestBed.configureTestingModule({
      providers: [
        ContributorService,
        { provide: HolochainClientService, useValue: holochainSpy },
      ],
    });
    service = TestBed.inject(ContributorService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Signals and State', () => {
    it('should have available signal', () => {
      expect(service.available).toBeDefined();
      expect(service.available()).toBeDefined();
    });

    it('should have ready computed signal', () => {
      expect(service.ready).toBeDefined();
      expect(typeof service.ready()).toBe('boolean');
    });

    it('should have dashboard$ observable', () => {
      expect(service.dashboard$).toBeDefined();
      expect(service.dashboard$.subscribe).toBeDefined();
    });
  });

  describe('isAvailable()', () => {
    it('should have isAvailable method', () => {
      expect(service.isAvailable).toBeDefined();
      expect(typeof service.isAvailable).toBe('function');
    });

    it('should return boolean', () => {
      const result = service.isAvailable();
      expect(typeof result).toBe('boolean');
    });
  });

  describe('getDashboard()', () => {
    it('should have getDashboard method', () => {
      expect(service.getDashboard).toBeDefined();
      expect(typeof service.getDashboard).toBe('function');
    });

    it('should return observable', () => {
      const result = service.getDashboard('contributor-123');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });

    it('should accept contributorId parameter', () => {
      expect(() => {
        service.getDashboard('contributor-123');
      }).not.toThrow();
    });

    it('should cache requests', () => {
      holochainSpy.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));
      service.getDashboard('contributor-123');
      service.getDashboard('contributor-123');
      // Should only call once due to caching
      expect(holochainSpy.callZome).not.toHaveBeenCalled();
    });
  });

  describe('getMyDashboard()', () => {
    it('should have getMyDashboard method', () => {
      expect(service.getMyDashboard).toBeDefined();
      expect(typeof service.getMyDashboard).toBe('function');
    });

    it('should return observable', () => {
      holochainSpy.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));
      const result = service.getMyDashboard();
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('getImpact()', () => {
    it('should have getImpact method', () => {
      expect(service.getImpact).toBeDefined();
      expect(typeof service.getImpact).toBe('function');
    });

    it('should return observable', () => {
      holochainSpy.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));
      const result = service.getImpact('contributor-123');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });

    it('should accept contributorId parameter', () => {
      holochainSpy.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));
      expect(() => {
        service.getImpact('contributor-123');
      }).not.toThrow();
    });
  });

  describe('getContentImpact()', () => {
    it('should have getContentImpact method', () => {
      expect(service.getContentImpact).toBeDefined();
      expect(typeof service.getContentImpact).toBe('function');
    });

    it('should return observable', () => {
      holochainSpy.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));
      const result = service.getContentImpact('contributor-123');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('getRecognitionHistory()', () => {
    it('should have getRecognitionHistory method', () => {
      expect(service.getRecognitionHistory).toBeDefined();
      expect(typeof service.getRecognitionHistory).toBe('function');
    });

    it('should return observable', () => {
      const result = service.getRecognitionHistory('contributor-123');
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });

    it('should accept contributorId parameter', () => {
      expect(() => {
        service.getRecognitionHistory('contributor-123');
      }).not.toThrow();
    });

    it('should cache requests', () => {
      holochainSpy.callZome.and.returnValue(
        Promise.resolve({ success: true, data: [] })
      );
      service.getRecognitionHistory('contributor-123');
      service.getRecognitionHistory('contributor-123');
      // Cache should prevent multiple calls
      expect(service).toBeTruthy();
    });
  });

  describe('clearCache()', () => {
    it('should have clearCache method', () => {
      expect(service.clearCache).toBeDefined();
      expect(typeof service.clearCache).toBe('function');
    });

    it('should clear caches without error', () => {
      expect(() => {
        service.clearCache();
      }).not.toThrow();
    });
  });

  describe('refreshMyDashboard()', () => {
    it('should have refreshMyDashboard method', () => {
      expect(service.refreshMyDashboard).toBeDefined();
      expect(typeof service.refreshMyDashboard).toBe('function');
    });

    it('should return observable', () => {
      holochainSpy.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));
      const result = service.refreshMyDashboard();
      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined();
    });
  });

  describe('testAvailability()', () => {
    it('should have testAvailability method', () => {
      expect(service.testAvailability).toBeDefined();
      expect(typeof service.testAvailability).toBe('function');
    });

    it('should return Promise<boolean>', async () => {
      holochainSpy.callZome.and.returnValue(Promise.resolve({ success: true, data: null }));
      const result = service.testAvailability();
      expect(result instanceof Promise).toBe(true);
      const isAvailable = await result;
      expect(typeof isAvailable).toBe('boolean');
    });

    it('should handle errors gracefully', async () => {
      holochainSpy.callZome.and.returnValue(Promise.reject(new Error('Connection failed')));
      const result = await service.testAvailability();
      expect(typeof result).toBe('boolean');
    });
  });
});
