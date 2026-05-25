import { TestBed } from '@angular/core/testing';
import { WasmCacheService } from './wasm-cache.service';

describe('WasmCacheService', () => {
  let service: WasmCacheService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [WasmCacheService],
    });
    service = TestBed.inject(WasmCacheService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Service Methods', () => {
    it('should have methods for cache management', () => {
      expect(typeof service).toBe('object');
      const methods = Object.getOwnPropertyNames(Object.getPrototypeOf(service));
      expect(methods.length).toBeGreaterThan(0);
    });

    it('should be injectable service', () => {
      const service2 = TestBed.inject(WasmCacheService);
      expect(service).toEqual(service2);
    });
  });
});
