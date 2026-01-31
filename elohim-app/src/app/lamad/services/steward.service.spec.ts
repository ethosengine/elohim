import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { StewardService } from './steward.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';

describe('StewardService', () => {
  let service: StewardService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;

  beforeEach(() => {
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', [
      'callZome',
      'isConnected',
    ]);
    mockHolochainClient.isConnected.and.returnValue(false);

    TestBed.configureTestingModule({
      providers: [
        StewardService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(StewardService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Service Status', () => {
    it('should have available signal', () => {
      expect(service.available).toBeDefined();
    });

    it('should have ready computed signal', () => {
      expect(service.ready).toBeDefined();
    });

    it('should have isAvailable method', () => {
      expect(service.isAvailable).toBeDefined();
      expect(typeof service.isAvailable).toBe('function');
    });
  });

  describe('Observable State', () => {
    it('should have myCredentials$ observable', () => {
      expect(service.myCredentials$).toBeDefined();
      expect(service.myCredentials$.subscribe).toBeDefined();
    });

    it('should have myGates$ observable', () => {
      expect(service.myGates$).toBeDefined();
      expect(service.myGates$.subscribe).toBeDefined();
    });

    it('should have myAccessGrants$ observable', () => {
      expect(service.myAccessGrants$).toBeDefined();
      expect(service.myAccessGrants$.subscribe).toBeDefined();
    });
  });

  describe('Credential Methods', () => {
    it('should have createCredential method', () => {
      expect(service.createCredential).toBeDefined();
      expect(typeof service.createCredential).toBe('function');
    });

    it('should have getCredential method', () => {
      expect(service.getCredential).toBeDefined();
      expect(typeof service.getCredential).toBe('function');
    });

    it('should have getMyCredentials method', () => {
      expect(service.getMyCredentials).toBeDefined();
      expect(typeof service.getMyCredentials).toBe('function');
    });

    it('should have getCredentialsForHuman method', () => {
      expect(service.getCredentialsForHuman).toBeDefined();
      expect(typeof service.getCredentialsForHuman).toBe('function');
    });

    it('getCredential should return observable when not available', (done) => {
      service.getCredential('cred-123').subscribe((result) => {
        expect(result).toBeNull();
        done();
      });
    });

    it('getMyCredentials should return empty array when not available', (done) => {
      service.getMyCredentials().subscribe((result) => {
        expect(Array.isArray(result)).toBe(true);
        expect(result.length).toBe(0);
        done();
      });
    });

    it('getCredentialsForHuman should return empty array when not available', (done) => {
      service.getCredentialsForHuman('human-1').subscribe((result) => {
        expect(Array.isArray(result)).toBe(true);
        expect(result.length).toBe(0);
        done();
      });
    });
  });

  describe('Gate Methods', () => {
    it('should have createGate method', () => {
      expect(service.createGate).toBeDefined();
      expect(typeof service.createGate).toBe('function');
    });

    it('should have getGate method', () => {
      expect(service.getGate).toBeDefined();
      expect(typeof service.getGate).toBe('function');
    });

    it('should have getGatesForResource method', () => {
      expect(service.getGatesForResource).toBeDefined();
      expect(typeof service.getGatesForResource).toBe('function');
    });

    it('getGate should return observable when not available', (done) => {
      service.getGate('gate-123').subscribe((result) => {
        expect(result).toBeNull();
        done();
      });
    });

    it('getGatesForResource should return empty array when not available', (done) => {
      service.getGatesForResource('resource-1').subscribe((result) => {
        expect(Array.isArray(result)).toBe(true);
        expect(result.length).toBe(0);
        done();
      });
    });
  });

  describe('Access Control Methods', () => {
    it('should have checkAccess method', () => {
      expect(service.checkAccess).toBeDefined();
      expect(typeof service.checkAccess).toBe('function');
    });

    it('should have grantAccess method', () => {
      expect(service.grantAccess).toBeDefined();
      expect(typeof service.grantAccess).toBe('function');
    });

    it('should have getMyAccessGrants method', () => {
      expect(service.getMyAccessGrants).toBeDefined();
      expect(typeof service.getMyAccessGrants).toBe('function');
    });

    it('checkAccess should return observable when not available', (done) => {
      service.checkAccess('gate-123').subscribe((result) => {
        expect(result).toBeNull();
        done();
      });
    });

    it('getMyAccessGrants should return empty array when not available', (done) => {
      service.getMyAccessGrants().subscribe((result) => {
        expect(Array.isArray(result)).toBe(true);
        expect(result.length).toBe(0);
        done();
      });
    });
  });

  describe('Revenue Methods', () => {
    it('should have getRevenueSummary method', () => {
      expect(service.getRevenueSummary).toBeDefined();
      expect(typeof service.getRevenueSummary).toBe('function');
    });

    it('getRevenueSummary should return observable when not available', (done) => {
      service.getRevenueSummary('steward-1').subscribe((result) => {
        expect(result).toBeNull();
        done();
      });
    });
  });

  describe('Cache Management', () => {
    it('should have clearCache method', () => {
      expect(service.clearCache).toBeDefined();
      expect(typeof service.clearCache).toBe('function');
    });

    it('clearCache should execute without error', () => {
      expect(() => service.clearCache()).not.toThrow();
    });
  });

  describe('Availability Testing', () => {
    it('should have testAvailability method', () => {
      expect(service.testAvailability).toBeDefined();
      expect(typeof service.testAvailability).toBe('function');
    });
  });
});
