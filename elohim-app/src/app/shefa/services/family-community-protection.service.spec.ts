/**
 * Family-community-protection Service Tests
 */

import { TestBed } from '@angular/core/testing';

import { FamilyCommunityProtectionService } from './family-community-protection.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { of, take } from 'rxjs';

describe('FamilyCommunityProtectionService', () => {
  let service: FamilyCommunityProtectionService;
  let mockHolochain: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    mockHolochain = jasmine.createSpyObj('HolochainClientService', ['callZome']);
    mockHolochain.callZome.and.returnValue(Promise.resolve({ success: false, data: [] }));

    TestBed.configureTestingModule({
      providers: [
        FamilyCommunityProtectionService,
        { provide: HolochainClientService, useValue: mockHolochain },
      ],
    });
    service = TestBed.inject(FamilyCommunityProtectionService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  describe('initializeProtectionMonitoring', () => {
    it('should have initializeProtectionMonitoring method', () => {
      expect(service.initializeProtectionMonitoring).toBeDefined();
      expect(typeof service.initializeProtectionMonitoring).toBe('function');
    });

    it('should return observable', (done) => {
      const result = service.initializeProtectionMonitoring('operator-1');
      expect(result).toBeDefined();
      result.subscribe(() => {
        done();
      });
    });

    it('should return protection status observable', (done) => {
      const result = service.initializeProtectionMonitoring('operator-1', 100);
      result.pipe(take(1)).subscribe((status) => {
        expect(status).toBeDefined();
        expect(status.custodians).toEqual([]);
        done();
      });
    });
  });

  describe('getProtectionStatus', () => {
    it('should have getProtectionStatus method', () => {
      expect(service.getProtectionStatus).toBeDefined();
      expect(typeof service.getProtectionStatus).toBe('function');
    });

    it('should return null initially', () => {
      const result = service.getProtectionStatus();
      expect(result).toBeNull();
    });
  });

  describe('getProtectionStatus$', () => {
    it('should have getProtectionStatus$ method', () => {
      expect(service.getProtectionStatus$).toBeDefined();
      expect(typeof service.getProtectionStatus$).toBe('function');
    });

    it('should return observable', (done) => {
      const result = service.getProtectionStatus$();
      expect(result).toBeDefined();
      result.subscribe((status) => {
        expect(status).toBeNull();
        done();
      });
    });
  });

  describe('getCustodiansByType', () => {
    it('should have getCustodiansByType method', () => {
      expect(service.getCustodiansByType).toBeDefined();
      expect(typeof service.getCustodiansByType).toBe('function');
    });

    it('should return empty array when no status', () => {
      const result = service.getCustodiansByType('family');
      expect(result).toEqual([]);
    });

    it('should return custodians of type family', () => {
      const result = service.getCustodiansByType('family');
      expect(result).toEqual(jasmine.any(Array));
    });

    it('should return custodians of type friend', () => {
      const result = service.getCustodiansByType('friend');
      expect(result).toEqual(jasmine.any(Array));
    });

    it('should return custodians of type community', () => {
      const result = service.getCustodiansByType('community');
      expect(result).toEqual(jasmine.any(Array));
    });

    it('should return custodians of type professional', () => {
      const result = service.getCustodiansByType('professional');
      expect(result).toEqual(jasmine.any(Array));
    });

    it('should return custodians of type institution', () => {
      const result = service.getCustodiansByType('institution');
      expect(result).toEqual(jasmine.any(Array));
    });
  });

  describe('getHighRiskRegions', () => {
    it('should have getHighRiskRegions method', () => {
      expect(service.getHighRiskRegions).toBeDefined();
      expect(typeof service.getHighRiskRegions).toBe('function');
    });

    it('should return empty array when no status', () => {
      const result = service.getHighRiskRegions();
      expect(result).toEqual([]);
    });

    it('should return array of regions', () => {
      const result = service.getHighRiskRegions();
      expect(result).toEqual(jasmine.any(Array));
    });
  });

  describe('isCustodianHealthy', () => {
    it('should have isCustodianHealthy method', () => {
      expect(service.isCustodianHealthy).toBeDefined();
      expect(typeof service.isCustodianHealthy).toBe('function');
    });

    it('should return false when no status', () => {
      const result = service.isCustodianHealthy('custodian-1');
      expect(result).toBeFalse();
    });

    it('should return boolean', () => {
      const result = service.isCustodianHealthy('custodian-1');
      expect(typeof result).toBe('boolean');
    });
  });

  describe('getAverageUptime', () => {
    it('should have getAverageUptime method', () => {
      expect(service.getAverageUptime).toBeDefined();
      expect(typeof service.getAverageUptime).toBe('function');
    });

    it('should return 0 when no custodians', () => {
      const result = service.getAverageUptime();
      expect(result).toBe(0);
    });

    it('should return number', () => {
      const result = service.getAverageUptime();
      expect(typeof result).toBe('number');
    });
  });

  describe('getProtectionAlerts', () => {
    it('should have getProtectionAlerts method', () => {
      expect(service.getProtectionAlerts).toBeDefined();
      expect(typeof service.getProtectionAlerts).toBe('function');
    });

    it('should return empty array initially', () => {
      const result = service.getProtectionAlerts();
      expect(result).toEqual([]);
    });

    it('should return array of strings', () => {
      const result = service.getProtectionAlerts();
      expect(result).toEqual(jasmine.any(Array));
    });
  });
});
