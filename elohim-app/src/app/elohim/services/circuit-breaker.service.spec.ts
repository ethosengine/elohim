/**
 * Circuit Breaker Service Tests - Mechanical Coverage
 *
 * Mechanical tests cover:
 * - Service creation and instantiation
 * - All public method existence and accessibility
 * - Basic input/output validation
 * - Observable/Signal return type tests
 * - Property initialization tests
 */

import { TestBed, fakeAsync, tick } from '@angular/core/testing';

import { CircuitBreakerService, CircuitBreakerConfig } from './circuit-breaker.service';
import { LoggerService } from './logger.service';

describe('CircuitBreakerService', () => {
  let service: CircuitBreakerService;
  let mockLogger: jasmine.SpyObj<LoggerService>;
  // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
  let mockChildLogger: jasmine.SpyObj<ReturnType<LoggerService['createChild']>>;

  beforeEach(() => {
    mockChildLogger = jasmine.createSpyObj('ChildLogger', ['debug', 'info', 'warn', 'error']);
    mockLogger = jasmine.createSpyObj('LoggerService', ['createChild']);
    // eslint-disable-next-line @typescript-eslint/no-unsafe-assignment
    mockLogger.createChild.and.returnValue(mockChildLogger);

    TestBed.configureTestingModule({
      providers: [CircuitBreakerService, { provide: LoggerService, useValue: mockLogger }],
    });

    service = TestBed.inject(CircuitBreakerService);
  });

  // ===========================================================================
  // MECHANICAL TESTS - Service Creation and Instantiation
  // ===========================================================================

  describe('Mechanical: Service Creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should be injectable from root', () => {
      const injectedService = TestBed.inject(CircuitBreakerService);
      expect(injectedService).toBeTruthy();
      expect(injectedService).toBe(service);
    });

    it('should instantiate with LoggerService dependency', () => {
      expect(mockLogger.createChild).toHaveBeenCalledWith('CircuitBreaker');
    });
  });

  // ===========================================================================
  // MECHANICAL TESTS - Public Method Existence
  // ===========================================================================

  describe('Mechanical: Public Methods Existence', () => {
    it('should have register method', () => {
      expect(typeof service.register).toBe('function');
    });

    it('should have execute method', () => {
      expect(typeof service.execute).toBe('function');
    });

    it('should have getState method', () => {
      expect(typeof service.getState).toBe('function');
    });

    it('should have getStats method', () => {
      expect(typeof service.getStats).toBe('function');
    });

    it('should have reset method', () => {
      expect(typeof service.reset).toBe('function');
    });

    it('should have getCircuitNames method', () => {
      expect(typeof service.getCircuitNames).toBe('function');
    });

    it('should have circuitStates property', () => {
      expect(service.circuitStates).toBeDefined();
    });
  });

  // ===========================================================================
  // MECHANICAL TESTS - Property Initialization
  // ===========================================================================

  describe('Mechanical: Property Initialization', () => {
    it('should initialize circuitStates as a signal', () => {
      const states = service.circuitStates;
      expect(typeof states).toBe('function'); // Signals are callable
    });

    it('should initialize circuitStates with empty Map', () => {
      const states = service.circuitStates();
      expect(states instanceof Map).toBe(true);
      expect(states.size).toBe(0);
    });

    it('should initialize circuitStates as read-only', () => {
      const states = service.circuitStates;
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      expect((states as any).set as any).toBeUndefined();
    });

    it('should have no circuits registered initially', () => {
      const names = service.getCircuitNames();
      expect(names).toEqual([]);
    });
  });

  // ===========================================================================
  // MECHANICAL TESTS - Observable/Signal Return Types
  // ===========================================================================

  describe('Mechanical: Signal Return Types', () => {
    it('circuitStates should return a signal callable', () => {
      const states = service.circuitStates;
      expect(typeof states).toBe('function');
      expect(typeof states()).toBe('object');
    });

    it('circuitStates() should return a Map instance', () => {
      const states = service.circuitStates();
      expect(states instanceof Map).toBe(true);
    });

    it('circuitStates should update reactively', fakeAsync(() => {
      const states1 = service.circuitStates();
      const initialSize = states1.size;

      service.register('reactive');
      tick(0);

      const states2 = service.circuitStates();
      expect(states2.size).toBeGreaterThan(initialSize);
    }));
  });

  // ===========================================================================
  // MECHANICAL TESTS - Basic Input/Output
  // ===========================================================================

  describe('Mechanical: register() Input/Output', () => {
    it('should accept circuit name and register it', () => {
      service.register('mech-test-1');
      expect(service.getCircuitNames()).toContain('mech-test-1');
    });

    it('should accept optional config partial', () => {
      const config: Partial<CircuitBreakerConfig> = { failureThreshold: 10 };
      service.register('mech-test-2', config);
      expect(service.getCircuitNames()).toContain('mech-test-2');
    });

    it('should return void', () => {
      const result = service.register('mech-test-3');
      expect(result).toBeUndefined();
    });

    it('should initialize state to CLOSED', () => {
      service.register('mech-test-4');
      expect(service.getState('mech-test-4')).toBe('CLOSED');
    });
  });

  describe('Mechanical: execute() Input/Output', () => {
    it('should accept circuit name and async function', async () => {
      const fn = jasmine.createSpy('fn').and.resolveTo('data');
      const result = await service.execute('mech-exec-1', fn);
      expect(result).toBeTruthy();
    });

    it('should accept optional config partial', async () => {
      const fn = jasmine.createSpy('fn').and.resolveTo('data');
      const result = await service.execute('mech-exec-2', fn, { failureThreshold: 3 });
      expect(result).toBeTruthy();
    });

    it('should return CircuitBreakerResult with required properties', async () => {
      const fn = jasmine.createSpy('fn').and.resolveTo('test-data');
      const result = await service.execute('mech-exec-3', fn);

      expect(result.success).toBeDefined();
      expect(typeof result.success).toBe('boolean');
      expect(result.circuitOpen).toBeDefined();
      expect(typeof result.circuitOpen).toBe('boolean');
      expect(result.state).toBeDefined();
      expect(['CLOSED', 'OPEN', 'HALF_OPEN']).toContain(result.state);
    });

    it('should be async and return Promise', () => {
      const fn = jasmine.createSpy('fn').and.resolveTo('data');
      const result = service.execute('mech-exec-4', fn);
      expect(result instanceof Promise).toBe(true);
    });
  });

  describe('Mechanical: getState() Input/Output', () => {
    it('should accept circuit name', () => {
      service.register('mech-state-1');
      const result = service.getState('mech-state-1');
      expect(result).not.toBeNull();
    });

    it('should return CircuitState or null', () => {
      service.register('mech-state-2');
      const result = service.getState('mech-state-2');
      expect(result === null || ['CLOSED', 'OPEN', 'HALF_OPEN'].includes(result!)).toBe(true);
    });
  });

  describe('Mechanical: getStats() Input/Output', () => {
    it('should accept circuit name', () => {
      service.register('mech-stats-1');
      const result = service.getStats('mech-stats-1');
      expect(result).toBeTruthy();
    });

    it('should return stats object with required properties', () => {
      service.register('mech-stats-2');
      const stats = service.getStats('mech-stats-2');

      expect(stats!.state).toBeDefined();
      expect(stats!.recentFailures).toBeDefined();
      expect(typeof stats!.recentFailures).toBe('number');
      expect(stats!.consecutiveSuccesses).toBeDefined();
      expect(typeof stats!.consecutiveSuccesses).toBe('number');
      expect(stats!.timeSinceLastFailure).toBeDefined();
      expect(stats!.timeSinceStateChange).toBeDefined();
      expect(typeof stats!.timeSinceStateChange).toBe('number');
    });
  });

  describe('Mechanical: reset() Input/Output', () => {
    it('should accept circuit name', () => {
      service.register('mech-reset-1');
      const result = service.reset('mech-reset-1');
      expect(result).toBeUndefined();
    });

    it('should handle non-existent circuit without error', () => {
      expect(() => service.reset('nonexistent')).not.toThrow();
    });
  });

  describe('Mechanical: getCircuitNames() Input/Output', () => {
    it('should return string array', () => {
      const result = service.getCircuitNames();
      expect(Array.isArray(result)).toBe(true);
    });

    it('should return registered circuit names', () => {
      service.register('mech-name-1');
      service.register('mech-name-2');
      const result = service.getCircuitNames();
      expect(result).toContain('mech-name-1');
      expect(result).toContain('mech-name-2');
    });
  });

  // ===========================================================================
  // Existing Behavior Tests (Original)
  // ===========================================================================

  describe('register', () => {
    it('should register a new circuit with default config', () => {
      service.register('test-circuit');

      expect(service.getState('test-circuit')).toBe('CLOSED');
      expect(service.getCircuitNames()).toContain('test-circuit');
    });

    it('should register with custom config', () => {
      service.register('custom-circuit', {
        failureThreshold: 3,
        resetTimeoutMs: 10000,
      });

      const stats = service.getStats('custom-circuit');
      expect(stats).not.toBeNull();
      expect(stats!.state).toBe('CLOSED');
    });

    it('should not re-register existing circuit', () => {
      service.register('existing', { failureThreshold: 5 });
      service.register('existing', { failureThreshold: 10 });

      // Should still have original config (first registration wins)
      expect(service.getCircuitNames().filter(n => n === 'existing').length).toBe(1);
    });
  });

  describe('execute - CLOSED state', () => {
    it('should execute function successfully', async () => {
      const fn = jasmine.createSpy('fn').and.resolveTo('success');

      const result = await service.execute('test', fn);

      expect(result.success).toBe(true);
      expect(result.data).toBe('success');
      expect(result.circuitOpen).toBe(false);
      expect(result.state).toBe('CLOSED');
      expect(fn).toHaveBeenCalled();
    });

    it('should record failure on error', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('test error'));

      const result = await service.execute('test', fn);

      expect(result.success).toBe(false);
      expect(result.error).toBe('test error');
      expect(result.circuitOpen).toBe(false);

      const stats = service.getStats('test');
      expect(stats!.recentFailures).toBe(1);
    });

    it('should open circuit after reaching failure threshold', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      // Default threshold is 5
      for (let i = 0; i < 5; i++) {
        await service.execute('threshold-test', fn);
      }

      expect(service.getState('threshold-test')).toBe('OPEN');
      expect(mockChildLogger.warn).toHaveBeenCalled();
    });

    it('should clear failures on success', async () => {
      const failFn = jasmine.createSpy('fail').and.rejectWith(new Error('fail'));
      const successFn = jasmine.createSpy('success').and.resolveTo('ok');

      // Add some failures
      await service.execute('clear-test', failFn);
      await service.execute('clear-test', failFn);

      let stats = service.getStats('clear-test');
      expect(stats!.recentFailures).toBe(2);

      // Success should clear failures
      await service.execute('clear-test', successFn);

      stats = service.getStats('clear-test');
      expect(stats!.recentFailures).toBe(0);
    });
  });

  describe('execute - OPEN state', () => {
    it('should fail fast when circuit is open', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      // Open the circuit
      service.register('open-test', { failureThreshold: 2 });
      await service.execute('open-test', fn);
      await service.execute('open-test', fn);

      expect(service.getState('open-test')).toBe('OPEN');

      // Should fail fast without calling fn
      fn.calls.reset();
      const result = await service.execute('open-test', fn);

      expect(result.success).toBe(false);
      expect(result.circuitOpen).toBe(true);
      expect(result.state).toBe('OPEN');
      expect(result.error).toContain('Circuit breaker');
      expect(fn).not.toHaveBeenCalled();
    });
  });

  describe('execute - HALF_OPEN state', () => {
    it('should transition to HALF_OPEN after timeout', fakeAsync(async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      service.register('halfopen-test', {
        failureThreshold: 2,
        resetTimeoutMs: 1000,
      });

      // Open the circuit
      // eslint-disable-next-line @typescript-eslint/no-floating-promises
      void service.execute('halfopen-test', fn);
      tick(0);
      // eslint-disable-next-line @typescript-eslint/no-floating-promises
      void service.execute('halfopen-test', fn);
      tick(0);

      expect(service.getState('halfopen-test')).toBe('OPEN');

      // Wait for reset timeout
      tick(1100);

      // Check state should trigger transition
      expect(service.getState('halfopen-test')).toBe('HALF_OPEN');
    }));

    it('should close circuit after success threshold in HALF_OPEN', async () => {
      const failFn = jasmine.createSpy('fail').and.rejectWith(new Error('fail'));
      const successFn = jasmine.createSpy('success').and.resolveTo('ok');

      service.register('recovery-test', {
        failureThreshold: 2,
        resetTimeoutMs: 0, // Immediate transition for testing
        successThreshold: 2,
      });

      // Open circuit
      await service.execute('recovery-test', failFn);
      await service.execute('recovery-test', failFn);

      // Manually trigger half-open by calling getState (will check timeout)
      // Since resetTimeoutMs is 0, it should transition immediately
      service.getState('recovery-test');

      // Success calls should close circuit
      await service.execute('recovery-test', successFn);
      await service.execute('recovery-test', successFn);

      expect(service.getState('recovery-test')).toBe('CLOSED');
    });

    it('should re-open circuit on failure in HALF_OPEN', async () => {
      const failFn = jasmine.createSpy('fail').and.rejectWith(new Error('fail'));

      service.register('reopen-test', {
        failureThreshold: 2,
        resetTimeoutMs: 0,
      });

      // Open circuit
      await service.execute('reopen-test', failFn);
      await service.execute('reopen-test', failFn);

      // Trigger half-open (resetTimeoutMs=0 means immediate transition)
      service.getState('reopen-test');

      // Failure in half-open should re-open circuit
      await service.execute('reopen-test', failFn);

      // Use getStats to check state without triggering auto-transition
      // (getState would transition OPEN→HALF_OPEN due to resetTimeoutMs=0)
      expect(service.getStats('reopen-test')?.state).toBe('OPEN');
    });
  });

  describe('getStats', () => {
    it('should return null for unknown circuit', () => {
      expect(service.getStats('nonexistent')).toBeNull();
    });

    it('should return accurate statistics', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      await service.execute('stats-test', fn);
      await service.execute('stats-test', fn);

      const stats = service.getStats('stats-test');

      expect(stats).not.toBeNull();
      expect(stats!.state).toBe('CLOSED');
      expect(stats!.recentFailures).toBe(2);
      expect(stats!.consecutiveSuccesses).toBe(0);
      expect(stats!.timeSinceLastFailure).toBeLessThan(1000);
      expect(stats!.timeSinceStateChange).toBeDefined();
    });

    it('should clean old failures from window', fakeAsync(async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      service.register('window-test', {
        failureThreshold: 10,
        failureWindowMs: 1000,
      });

      // eslint-disable-next-line @typescript-eslint/no-floating-promises
      void service.execute('window-test', fn);
      tick(0);

      let stats = service.getStats('window-test');
      expect(stats!.recentFailures).toBe(1);

      // Wait for failure to age out of window
      tick(1100);

      stats = service.getStats('window-test');
      expect(stats!.recentFailures).toBe(0);
    }));
  });

  describe('reset', () => {
    it('should reset circuit to CLOSED state', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      service.register('reset-test', { failureThreshold: 2 });

      // Open circuit
      await service.execute('reset-test', fn);
      await service.execute('reset-test', fn);

      expect(service.getState('reset-test')).toBe('OPEN');

      // Reset
      service.reset('reset-test');

      expect(service.getState('reset-test')).toBe('CLOSED');
      const stats = service.getStats('reset-test');
      expect(stats!.recentFailures).toBe(0);
    });

    it('should do nothing for unknown circuit', () => {
      // Should not throw
      expect(() => service.reset('nonexistent')).not.toThrow();
    });
  });

  describe('circuitStates signal', () => {
    it('should update state signal on registration', () => {
      service.register('signal-test');

      const states = service.circuitStates();
      expect(states.get('signal-test')).toBe('CLOSED');
    });

    it('should update state signal on transitions', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      service.register('transition-test', { failureThreshold: 2 });

      await service.execute('transition-test', fn);
      await service.execute('transition-test', fn);

      const states = service.circuitStates();
      expect(states.get('transition-test')).toBe('OPEN');
    });
  });

  describe('auto-registration', () => {
    it('should auto-register circuit on first execute', async () => {
      const fn = jasmine.createSpy('fn').and.resolveTo('ok');

      await service.execute('auto-registered', fn);

      expect(service.getCircuitNames()).toContain('auto-registered');
    });

    it('should use provided config on auto-registration', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      await service.execute('auto-config', fn, { failureThreshold: 1 });

      // Should be OPEN after just 1 failure with custom threshold
      expect(service.getState('auto-config')).toBe('OPEN');
    });
  });

  describe('error handling', () => {
    it('should handle non-Error throws', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith('string error');

      const result = await service.execute('error-test', fn);

      expect(result.success).toBe(false);
      expect(result.error).toBe('string error');
    });

    it('should handle undefined throws', async () => {
      const fn = jasmine.createSpy('fn').and.rejectWith(undefined);

      const result = await service.execute('undefined-test', fn);

      expect(result.success).toBe(false);
      expect(result.error).toBe('undefined');
    });
  });
});
