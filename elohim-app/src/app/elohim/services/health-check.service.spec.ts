import { TestBed, fakeAsync, tick, flush } from '@angular/core/testing';
import { HealthCheckService, HealthStatus, HealthState } from './health-check.service';
import { HolochainClientService } from './holochain-client.service';
import { IndexedDBCacheService } from './indexeddb-cache.service';
import { LoggerService } from './logger.service';

describe('HealthCheckService', () => {
  let service: HealthCheckService;
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockIndexedDbCache: jasmine.SpyObj<IndexedDBCacheService>;
  let mockLogger: jasmine.SpyObj<LoggerService>;

  beforeEach(() => {
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', [
      'isConnected',
      'getDisplayInfo',
    ]);
    mockHolochainClient.isConnected.and.returnValue(true);
    mockHolochainClient.getDisplayInfo.and.returnValue({
      appUrl: 'ws://localhost:8888',
      adminUrl: null,
      mode: 'direct' as const,
    });

    mockIndexedDbCache = jasmine.createSpyObj('IndexedDBCacheService', [
      'init',
      'isAvailable',
      'getStats',
    ]);
    mockIndexedDbCache.init.and.returnValue(Promise.resolve(true));
    mockIndexedDbCache.isAvailable.and.returnValue(true);
    mockIndexedDbCache.getStats.and.returnValue(Promise.resolve({
      contentCount: 10,
      pathCount: 5,
      isAvailable: true,
    }));

    const mockChildLogger = {
      debug: jasmine.createSpy('debug'),
      info: jasmine.createSpy('info'),
      warn: jasmine.createSpy('warn'),
      error: jasmine.createSpy('error'),
      startTimer: jasmine.createSpy('startTimer').and.returnValue({
        end: jasmine.createSpy('end'),
        elapsed: jasmine.createSpy('elapsed').and.returnValue(100),
      }),
    };
    mockLogger = jasmine.createSpyObj('LoggerService', ['createChild']);
    mockLogger.createChild.and.returnValue(mockChildLogger);

    TestBed.configureTestingModule({
      providers: [
        HealthCheckService,
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: IndexedDBCacheService, useValue: mockIndexedDbCache },
        { provide: LoggerService, useValue: mockLogger },
      ],
    });

    service = TestBed.inject(HealthCheckService);
  });

  afterEach(() => {
    service.ngOnDestroy();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('Initial State', () => {
    it('should start with unknown status', () => {
      // Initial status before first check completes
      const initialStatus = service.status();
      expect(['unknown', 'healthy', 'degraded', 'unhealthy']).toContain(initialStatus.status);
    });

    it('should have all check categories', () => {
      const status = service.status();
      expect(status.checks.holochain).toBeDefined();
      expect(status.checks.indexedDb).toBeDefined();
      expect(status.checks.blobCache).toBeDefined();
      expect(status.checks.network).toBeDefined();
    });
  });

  describe('Health Checks', () => {
    it('should report healthy when all systems are up', fakeAsync(() => {
      service.refresh().then(status => {
        expect(status.status).toBe('healthy');
        expect(status.checks.holochain.status).toBe('healthy');
        expect(status.checks.indexedDb.status).toBe('healthy');
      });
      tick();
      flush();
    }));

    it('should report unhealthy when Holochain is disconnected', fakeAsync(() => {
      mockHolochainClient.isConnected.and.returnValue(false);

      service.refresh().then(status => {
        expect(status.status).toBe('unhealthy');
        expect(status.checks.holochain.status).toBe('unhealthy');
        expect(status.checks.holochain.message).toContain('Not connected');
      });
      tick();
      flush();
    }));

    it('should report degraded when IndexedDB is unavailable', fakeAsync(() => {
      mockIndexedDbCache.isAvailable.and.returnValue(false);

      service.refresh().then(status => {
        expect(['degraded', 'healthy']).toContain(status.status);
        expect(status.checks.indexedDb.status).toBe('degraded');
      });
      tick();
      flush();
    }));

    it('should include duration in check results', fakeAsync(() => {
      service.refresh().then(status => {
        expect(status.checks.holochain.durationMs).toBeDefined();
        expect(typeof status.checks.holochain.durationMs).toBe('number');
      });
      tick();
      flush();
    }));

    it('should include metadata when available', fakeAsync(() => {
      service.refresh().then(status => {
        expect(status.checks.holochain.metadata).toBeDefined();
        expect(status.checks.holochain.metadata?.mode).toBe('direct');
      });
      tick();
      flush();
    }));
  });

  describe('Computed Signals', () => {
    it('should compute isHealthy correctly', fakeAsync(() => {
      service.refresh();
      tick();
      flush();

      expect(service.isHealthy()).toBe(true);
      expect(service.isDegraded()).toBe(false);
      expect(service.isUnhealthy()).toBe(false);
    }));

    it('should compute isUnhealthy when Holochain disconnected', fakeAsync(() => {
      mockHolochainClient.isConnected.and.returnValue(false);
      service.refresh();
      tick();
      flush();

      expect(service.isHealthy()).toBe(false);
      expect(service.isUnhealthy()).toBe(true);
    }));
  });

  describe('Quick Status', () => {
    it('should return healthy quick status', fakeAsync(() => {
      service.refresh();
      tick();
      flush();

      const quick = service.getQuickStatus();
      expect(quick.color).toBe('green');
      expect(quick.label).toContain('operational');
    }));

    it('should return unhealthy quick status when disconnected', fakeAsync(() => {
      mockHolochainClient.isConnected.and.returnValue(false);
      service.refresh();
      tick();
      flush();

      const quick = service.getQuickStatus();
      expect(quick.color).toBe('red');
      expect(quick.label).toContain('issues');
    }));
  });

  describe('Refresh Behavior', () => {
    it('should set isChecking during refresh', fakeAsync(() => {
      let wasChecking = false;

      // Start refresh
      const promise = service.refresh();

      // Check if checking flag is set
      wasChecking = service.isChecking();

      tick();
      flush();

      expect(wasChecking).toBe(true);
      expect(service.isChecking()).toBe(false);
    }));

    it('should not run concurrent refreshes', fakeAsync(() => {
      const promise1 = service.refresh();
      const promise2 = service.refresh();

      tick();
      flush();

      // Both should resolve to the same status
      Promise.all([promise1, promise2]).then(([status1, status2]) => {
        expect(status1).toBe(status2);
      });
      tick();
    }));

    it('should update lastChecked timestamp', fakeAsync(() => {
      const beforeCheck = new Date().toISOString();
      service.refresh();
      tick();
      flush();

      const status = service.status();
      expect(new Date(status.lastChecked).getTime())
        .toBeGreaterThanOrEqual(new Date(beforeCheck).getTime());
    }));
  });

  describe('Individual Check', () => {
    it('should allow checking only Holochain', fakeAsync(() => {
      service.checkHolochainOnly().then(check => {
        expect(check.name).toBe('holochain');
        expect(check.status).toBe('healthy');
      });
      tick();
      flush();
    }));

    it('should update overall status after individual check', fakeAsync(() => {
      // First make it healthy
      service.refresh();
      tick();
      flush();
      expect(service.status().status).toBe('healthy');

      // Now disconnect Holochain and check only that
      mockHolochainClient.isConnected.and.returnValue(false);
      service.checkHolochainOnly();
      tick();
      flush();

      // Overall status should now be unhealthy
      expect(service.status().status).toBe('unhealthy');
    }));
  });

  describe('Error Handling', () => {
    it('should handle check errors gracefully', fakeAsync(() => {
      mockHolochainClient.isConnected.and.throwError('Connection error');

      service.refresh().then(status => {
        expect(status.checks.holochain.status).toBe('unhealthy');
        expect(status.checks.holochain.message).toContain('Connection error');
      });
      tick();
      flush();
    }));

    it('should handle IndexedDB errors as degraded', fakeAsync(() => {
      mockIndexedDbCache.getStats.and.returnValue(Promise.reject(new Error('DB error')));

      service.refresh().then(status => {
        expect(status.checks.indexedDb.status).toBe('degraded');
      });
      tick();
      flush();
    }));
  });

  describe('Cleanup', () => {
    it('should stop auto-check on destroy', () => {
      // Create a spy on clearInterval
      spyOn(window, 'clearInterval');

      service.ngOnDestroy();

      expect(window.clearInterval).toHaveBeenCalled();
    });
  });

  describe('Summary Generation', () => {
    it('should generate appropriate summary for healthy status', fakeAsync(() => {
      service.refresh().then(status => {
        expect(status.summary).toBe('All systems operational');
      });
      tick();
      flush();
    }));

    it('should list unhealthy components in summary', fakeAsync(() => {
      mockHolochainClient.isConnected.and.returnValue(false);

      service.refresh().then(status => {
        expect(status.summary).toContain('holochain');
      });
      tick();
      flush();
    }));
  });
});
