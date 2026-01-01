/**
 * Circuit Breaker Service Tests
 */

import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { CircuitBreakerService, CircuitState, CircuitBreakerResult } from './circuit-breaker.service';
import { LoggerService } from './logger.service';

describe('CircuitBreakerService', () => {
  let service: CircuitBreakerService;
  let mockLogger: jasmine.SpyObj<LoggerService>;
  let mockChildLogger: jasmine.SpyObj<ReturnType<LoggerService['createChild']>>;

  beforeEach(() => {
    mockChildLogger = jasmine.createSpyObj('ChildLogger', ['debug', 'info', 'warn', 'error']);
    mockLogger = jasmine.createSpyObj('LoggerService', ['createChild']);
    mockLogger.createChild.and.returnValue(mockChildLogger);

    TestBed.configureTestingModule({
      providers: [
        CircuitBreakerService,
        { provide: LoggerService, useValue: mockLogger },
      ],
    });

    service = TestBed.inject(CircuitBreakerService);
  });

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
    it('should transition to HALF_OPEN after timeout', fakeAsync(() => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      service.register('halfopen-test', {
        failureThreshold: 2,
        resetTimeoutMs: 1000,
      });

      // Open the circuit
      service.execute('halfopen-test', fn);
      tick(0);
      service.execute('halfopen-test', fn);
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

      // Trigger half-open
      service.getState('reopen-test');

      // One failure in half-open should open circuit again
      await service.execute('reopen-test', failFn);
      await service.execute('reopen-test', failFn);

      expect(service.getState('reopen-test')).toBe('OPEN');
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

    it('should clean old failures from window', fakeAsync(() => {
      const fn = jasmine.createSpy('fn').and.rejectWith(new Error('fail'));

      service.register('window-test', {
        failureThreshold: 10,
        failureWindowMs: 1000,
      });

      service.execute('window-test', fn);
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
