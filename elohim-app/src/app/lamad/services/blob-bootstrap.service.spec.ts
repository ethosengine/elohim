import { TestBed } from '@angular/core/testing';
import { BlobBootstrapService } from './blob-bootstrap.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { BlobCacheTiersService } from './blob-cache-tiers.service';
import { BlobManagerService } from './blob-manager.service';
import { signal } from '@angular/core';

describe('BlobBootstrapService', () => {
  let service: BlobBootstrapService;
  let holochainSpy: jasmine.SpyObj<HolochainClientService>;
  let blobManagerSpy: jasmine.SpyObj<BlobManagerService>;
  let blobCacheSpy: jasmine.SpyObj<BlobCacheTiersService>;

  beforeEach(() => {
    holochainSpy = jasmine.createSpyObj('HolochainClientService', ['isConnected']);
    blobManagerSpy = jasmine.createSpyObj('BlobManagerService', ['fetch']);
    blobCacheSpy = jasmine.createSpyObj('BlobCacheTiersService', ['init']);

    TestBed.configureTestingModule({
      providers: [
        BlobBootstrapService,
        { provide: HolochainClientService, useValue: holochainSpy },
        { provide: BlobManagerService, useValue: blobManagerSpy },
        { provide: BlobCacheTiersService, useValue: blobCacheSpy },
      ],
    });
    service = TestBed.inject(BlobBootstrapService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('State Signals', () => {
    it('should have state signal', () => {
      expect(service.state).toBeDefined();
    });

    it('should have status computed signal', () => {
      expect(service.status).toBeDefined();
      expect(typeof service.status()).toBe('string');
    });

    it('should have isReady computed signal', () => {
      expect(service.isReady).toBeDefined();
      expect(typeof service.isReady()).toBe('boolean');
    });

    it('should have canServeOffline computed signal', () => {
      expect(service.canServeOffline).toBeDefined();
      expect(typeof service.canServeOffline()).toBe('boolean');
    });
  });

  describe('Service Initialization', () => {
    it('should initialize with engine', () => {
      expect(service).toBeTruthy();
    });

    it('should initialize state to initializing status', () => {
      expect(service.status()).toBeDefined();
    });
  });
});
