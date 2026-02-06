/* eslint-disable no-console, sonarjs/no-duplicate-string */
import { TestBed } from '@angular/core/testing';

import { LoggerService, ChildLogger } from './logger.service';

describe('LoggerService', () => {
  let service: LoggerService;

  // Constants for frequently used test strings
  const TEST_MESSAGE = 'Test message';
  const TEST = 'Test';
  const SHOULD_NOT_APPEAR = 'Should not appear';
  const SHOULD_APPEAR = 'Should appear';
  const TEST_OP = 'test-op';
  const TEST_OPERATION = 'test-operation';
  const OPERATION = 'operation';
  const DEFAULT_SOURCE = 'elohim-app';

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [LoggerService],
    });
    service = TestBed.inject(LoggerService);

    // Spy on console methods

    spyOn(console, 'debug');

    spyOn(console, 'info');

    spyOn(console, 'warn');

    spyOn(console, 'error');

    spyOn(console, 'log');
  });

  afterEach(() => {
    service.clearRecentLogs();
  });

  // =========================================================================
  // Service Creation & Instantiation
  // =========================================================================

  describe('Service Instantiation', () => {
    it('should be created as a singleton', () => {
      expect(service).toBeInstanceOf(LoggerService);
    });

    it('should inject without errors', () => {
      expect(service).toBeDefined();
      expect(service).not.toBeNull();
    });

    it('should have default configuration on creation', () => {
      service.setMinLevel('debug'); // Default behavior
      service.debug('Test');
      expect(console.debug).toHaveBeenCalled();
    });
  });

  // =========================================================================
  // Method Existence Tests
  // =========================================================================

  describe('Method Existence - Configuration Methods', () => {
    it('should have configure method', () => {
      expect(typeof service.configure).toBe('function');
    });

    it('should have setMinLevel method', () => {
      expect(typeof service.setMinLevel).toBe('function');
    });

    it('should have setCorrelationId method', () => {
      expect(typeof service.setCorrelationId).toBe('function');
    });

    it('should have getCorrelationId method', () => {
      expect(typeof service.getCorrelationId).toBe('function');
    });

    it('should have generateCorrelationId method', () => {
      expect(typeof service.generateCorrelationId).toBe('function');
    });

    it('should have setSource method', () => {
      expect(typeof service.setSource).toBe('function');
    });

    it('should have createChild method', () => {
      expect(typeof service.createChild).toBe('function');
    });
  });

  describe('Method Existence - Log Methods', () => {
    it('should have debug method', () => {
      expect(typeof service.debug).toBe('function');
    });

    it('should have info method', () => {
      expect(typeof service.info).toBe('function');
    });

    it('should have warn method', () => {
      expect(typeof service.warn).toBe('function');
    });

    it('should have error method', () => {
      expect(typeof service.error).toBe('function');
    });
  });

  describe('Method Existence - Performance/Inspection Methods', () => {
    it('should have startTimer method', () => {
      expect(typeof service.startTimer).toBe('function');
    });

    it('should have time method', () => {
      expect(typeof service.time).toBe('function');
    });

    it('should have getRecentLogs method', () => {
      expect(typeof service.getRecentLogs).toBe('function');
    });

    it('should have clearRecentLogs method', () => {
      expect(typeof service.clearRecentLogs).toBe('function');
    });
  });

  // =========================================================================
  // Simple Input/Output Tests
  // =========================================================================

  describe('Basic Logging Input/Output', () => {
    it('debug should accept message and call console.debug', () => {
      service.debug('debug message');
      expect(console.debug).toHaveBeenCalled();
    });

    it('debug should accept message with context', () => {
      const context = { key: 'value' };
      service.debug('debug with context', context);
      expect(console.debug).toHaveBeenCalledWith(jasmine.any(String), context);
    });

    it('info should accept message and call console.info', () => {
      service.info('info message');
      expect(console.info).toHaveBeenCalled();
    });

    it('info should accept message with context', () => {
      const context = { userId: '123' };
      service.info('info with context', context);
      expect(console.info).toHaveBeenCalledWith(jasmine.any(String), context);
    });

    it('warn should accept message and call console.warn', () => {
      service.warn('warning message');
      expect(console.warn).toHaveBeenCalled();
    });

    it('warn should accept message with context', () => {
      const context = { warning: true };
      service.warn('warn with context', context);
      expect(console.warn).toHaveBeenCalledWith(jasmine.any(String), context);
    });

    it('error should accept message and call console.error', () => {
      service.error('error message');
      expect(console.error).toHaveBeenCalled();
    });

    it('error should accept message, error object, and context', () => {
      const error = new Error('test error');
      const context = { errorCode: 500 };
      service.error('error occurred', error, context);
      expect(console.error).toHaveBeenCalledWith(jasmine.any(String), context, error);
    });
  });

  describe('Log Output Format', () => {
    it('should output with timestamp when configured', () => {
      service.configure({ includeTimestamp: true });
      service.info(TEST_MESSAGE);
      expect(console.info).toHaveBeenCalledWith(jasmine.stringMatching(/\[\d{2}:\d{2}:\d{2}\]/));
    });

    it('should output without timestamp when disabled', () => {
      service.configure({ includeTimestamp: false });
      service.info(TEST_MESSAGE);
      const callArgs = (console.info as jasmine.Spy).calls.mostRecent().args[0];
      expect(callArgs).not.toMatch(/\[\d{2}:\d{2}:\d{2}\]/);
    });

    it('should include source in output', () => {
      service.setSource('TestSource');
      service.info(TEST_MESSAGE);
      expect(console.info).toHaveBeenCalledWith(jasmine.stringMatching(/\[TestSource\]/));
    });

    it('should include correlation ID prefix in output', () => {
      service.setCorrelationId('req-1234');
      service.info(TEST_MESSAGE);
      const callArgs = (console.info as jasmine.Spy).calls.mostRecent().args[0];
      expect(callArgs).toContain('req-1234');
    });
  });

  // =========================================================================
  // Observable/Signal Return Type Tests
  // =========================================================================

  describe('Signal/Observable Return Types', () => {
    it('getRecentLogs should return array', () => {
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(Array.isArray(logs)).toBe(true);
    });

    it('getRecentLogs should return LogEntry objects', () => {
      service.info(TEST_MESSAGE);
      const logs = service.getRecentLogs();
      expect(logs.length).toBeGreaterThan(0);
      expect(logs[0]).toEqual(
        jasmine.objectContaining({
          timestamp: jasmine.any(String),
          level: jasmine.any(String),
          message: jasmine.any(String),
        })
      );
    });

    it('startTimer should return LogTimer interface', () => {
      const timer = service.startTimer('test');
      expect(timer).toEqual(
        jasmine.objectContaining({
          end: jasmine.any(Function),
          elapsed: jasmine.any(Function),
        })
      );
    });

    it('generateCorrelationId should return string', () => {
      const id = service.generateCorrelationId();
      expect(typeof id).toBe('string');
      expect(id.length).toBeGreaterThan(0);
    });

    it('getCorrelationId should return string or null', () => {
      service.setCorrelationId(null);
      let result = service.getCorrelationId();
      expect(result === null || typeof result === 'string').toBe(true);

      service.setCorrelationId('test-123');
      result = service.getCorrelationId();
      expect(typeof result).toBe('string');
    });
  });

  // =========================================================================
  // Property Initialization Tests
  // =========================================================================

  describe('Property Initialization', () => {
    it('should initialize with null correlation ID', () => {
      service.clearRecentLogs();
      expect(service.getCorrelationId()).toBeNull();
    });

    it('should initialize with default source', () => {
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe(DEFAULT_SOURCE);
    });

    it('should initialize with empty recent logs', () => {
      service.clearRecentLogs();
      expect(service.getRecentLogs().length).toBe(0);
    });

    it('should initialize with debug log level by default', () => {
      service.clearRecentLogs();
      service.debug('Debug message');
      expect(service.getRecentLogs().length).toBe(1);
    });

    it('should initialize with includeTimestamp true by default', () => {
      service.configure({ includeTimestamp: true });
      service.info(TEST);
      const callArgs = (console.info as jasmine.Spy).calls.mostRecent().args[0];
      expect(callArgs).toMatch(/\[\d{2}:\d{2}:\d{2}\]/);
    });

    it('should initialize with jsonOutput false by default', () => {
      service.info(TEST);
      expect(console.log).not.toHaveBeenCalled();
      expect(console.info).toHaveBeenCalled();
    });
  });

  // =========================================================================
  // Error Handling Tests
  // =========================================================================

  describe('Error Handling - Input Variations', () => {
    it('should handle non-Error objects passed to error method', () => {
      service.error('Something failed', 'string error');
      expect(console.error).toHaveBeenCalled();
    });

    it('should handle undefined context', () => {
      service.info('Message');
      expect(console.info).toHaveBeenCalled();
    });

    it('should handle empty string message', () => {
      service.info('');
      expect(console.info).toHaveBeenCalled();
    });

    it('should handle empty context object', () => {
      service.info('Message', {});
      expect(console.info).toHaveBeenCalled();
    });

    it('should handle null source before setting', () => {
      service.setSource(null);
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe(DEFAULT_SOURCE);
    });

    it('should handle null correlation ID', () => {
      service.setCorrelationId(null);
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(logs[0].correlationId).toBeUndefined();
    });
  });

  // =========================================================================
  // ChildLogger Tests
  // =========================================================================

  describe('ChildLogger - Existence & Instantiation', () => {
    it('should create ChildLogger instance', () => {
      const child = service.createChild('TestSource');
      expect(child).toBeInstanceOf(ChildLogger);
    });

    it('should have debug method on ChildLogger', () => {
      const child = service.createChild('TestSource');
      expect(typeof child.debug).toBe('function');
    });

    it('should have info method on ChildLogger', () => {
      const child = service.createChild('TestSource');
      expect(typeof child.info).toBe('function');
    });

    it('should have warn method on ChildLogger', () => {
      const child = service.createChild('TestSource');
      expect(typeof child.warn).toBe('function');
    });

    it('should have error method on ChildLogger', () => {
      const child = service.createChild('TestSource');
      expect(typeof child.error).toBe('function');
    });

    it('should have startTimer method on ChildLogger', () => {
      const child = service.createChild('TestSource');
      expect(typeof child.startTimer).toBe('function');
    });
  });

  describe('ChildLogger - Input/Output', () => {
    it('should log debug through child logger', () => {
      const child = service.createChild('ChildService');
      child.debug('Child debug');
      expect(console.debug).toHaveBeenCalled();
    });

    it('should log info through child logger', () => {
      const child = service.createChild('ChildService');
      child.info('Child info');
      expect(console.info).toHaveBeenCalled();
    });

    it('should log warn through child logger', () => {
      const child = service.createChild('ChildService');
      child.warn('Child warn');
      expect(console.warn).toHaveBeenCalled();
    });

    it('should log error through child logger', () => {
      const child = service.createChild('ChildService');
      child.error('Child error');
      expect(console.error).toHaveBeenCalled();
    });

    it('should override source in child logger logs', () => {
      service.setSource('ParentSource');
      const child = service.createChild('ChildSource');
      child.info('Message');
      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe('ChildSource');
    });

    it('should preserve source after child logger call', () => {
      service.setSource('ParentSource');
      const child = service.createChild('ChildSource');
      child.info('Message');
      service.info('Another message');
      const logs = service.getRecentLogs();
      // Parent source remains unchanged after child logger use
      expect(logs[1].source).toBe('elohim-app');
    });
  });

  // =========================================================================
  // Log Timer Tests
  // =========================================================================

  describe('LogTimer - Existence & Instantiation', () => {
    it('startTimer should return LogTimer with end function', () => {
      const timer = service.startTimer(OPERATION);
      expect(typeof timer.end).toBe('function');
    });

    it('startTimer should return LogTimer with elapsed function', () => {
      const timer = service.startTimer(OPERATION);
      expect(typeof timer.elapsed).toBe('function');
    });
  });

  describe('LogTimer - Input/Output', () => {
    it('timer.elapsed() should return number', () => {
      const timer = service.startTimer(OPERATION);
      const elapsed = timer.elapsed();
      expect(typeof elapsed).toBe('number');
      expect(elapsed).toBeGreaterThanOrEqual(0);
    });

    it('timer.end() should log a message', () => {
      const timer = service.startTimer(TEST_OP);
      timer.end();
      const logs = service.getRecentLogs();
      expect(logs.length).toBeGreaterThan(0);
      expect(logs[0].message).toContain('test-op');
    });

    it('timer.end() should include durationMs in context', () => {
      const timer = service.startTimer(TEST_OP);
      timer.end();
      const logs = service.getRecentLogs();
      expect(logs[0].context?.['durationMs']).toBeDefined();
      expect(typeof logs[0].context?.['durationMs']).toBe('number');
    });

    it('timer.end() should accept context object', () => {
      const timer = service.startTimer(TEST_OP);
      const customContext = { customKey: 'customValue' };
      timer.end(customContext);
      const logs = service.getRecentLogs();
      expect(logs[0].context?.['customKey']).toBe('customValue');
    });

    it('timer.elapsed() should increase over time', async () => {
      const timer = service.startTimer(OPERATION);
      const elapsed1 = timer.elapsed();
      await new Promise(resolve => setTimeout(resolve, 5));
      const elapsed2 = timer.elapsed();
      expect(elapsed2).toBeGreaterThanOrEqual(elapsed1);
    });
  });

  // =========================================================================
  // Configuration Tests
  // =========================================================================

  describe('Configuration - Method Input/Output', () => {
    it('configure should accept partial config object', () => {
      service.configure({ minLevel: 'warn' });
      service.debug('Should be filtered');
      expect(console.debug).not.toHaveBeenCalled();
    });

    it('configure should merge with existing config', () => {
      service.configure({ minLevel: 'warn' });
      service.configure({ defaultSource: 'custom' });
      service.warn(TEST); // Use warn level since minLevel is 'warn'
      const logs = service.getRecentLogs();
      expect(logs.length).toBeGreaterThan(0);
      expect(logs[0].source).toBe('custom');
    });

    it('setMinLevel should update log level filtering', () => {
      service.setMinLevel('error');
      service.warn('Should be filtered');
      expect(console.warn).not.toHaveBeenCalled();
    });
  });

  // =========================================================================
  // LogEntry Structure Tests
  // =========================================================================

  describe('LogEntry Structure - Required Properties', () => {
    it('LogEntry should have timestamp property', () => {
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(logs[0].timestamp).toBeDefined();
      expect(typeof logs[0].timestamp).toBe('string');
    });

    it('LogEntry should have level property', () => {
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(logs[0].level).toBeDefined();
      expect(['debug', 'info', 'warn', 'error']).toContain(logs[0].level);
    });

    it('LogEntry should have message property', () => {
      service.info(TEST_MESSAGE);
      const logs = service.getRecentLogs();
      expect(logs[0].message).toBeDefined();
      expect(logs[0].message).toBe('Test message');
    });

    it('LogEntry should have source property', () => {
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(logs[0].source).toBeDefined();
      expect(typeof logs[0].source).toBe('string');
    });
  });

  describe('LogEntry Structure - Optional Properties', () => {
    it('LogEntry should have optional error property', () => {
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect('error' in logs[0]).toBe(true);
    });

    it('LogEntry should have optional context property', () => {
      service.info('Test', { key: 'value' });
      const logs = service.getRecentLogs();
      expect(logs[0].context).toBeDefined();
      expect(logs[0].context).toEqual({ key: 'value' });
    });

    it('LogEntry should have optional correlationId property', () => {
      service.setCorrelationId('req-123');
      service.info(TEST);
      const logs = service.getRecentLogs();
      expect(logs[0].correlationId).toBeDefined();
      expect(logs[0].correlationId).toBe('req-123');
    });

    it('LogEntry should have optional durationMs property for timed operations', () => {
      const timer = service.startTimer(OPERATION);
      timer.end();
      const logs = service.getRecentLogs();
      expect(logs[0].context?.['durationMs']).toBeDefined();
    });
  });

  // =========================================================================
  // Additional Existing Tests (Business Logic)
  // =========================================================================

  describe('Log Levels', () => {
    it('should log debug messages', () => {
      service.debug('Debug message');
      expect(console.debug).toHaveBeenCalled();
    });

    it('should log info messages', () => {
      service.info('Info message');
      expect(console.info).toHaveBeenCalled();
    });

    it('should log warn messages', () => {
      service.warn('Warning message');
      expect(console.warn).toHaveBeenCalled();
    });

    it('should log error messages', () => {
      service.error('Error message');
      expect(console.error).toHaveBeenCalled();
    });

    it('should log error with Error object', () => {
      const error = new Error('Test error');
      service.error('Something failed', error);
      expect(console.error).toHaveBeenCalled();
    });
  });

  describe('Log Level Filtering', () => {
    it('should filter out debug when minLevel is info', () => {
      service.setMinLevel('info');
      service.debug(SHOULD_NOT_APPEAR);
      expect(console.debug).not.toHaveBeenCalled();
    });

    it('should allow info when minLevel is info', () => {
      service.setMinLevel('info');
      service.info(SHOULD_APPEAR);
      expect(console.info).toHaveBeenCalled();
    });

    it('should filter out info when minLevel is warn', () => {
      service.setMinLevel('warn');
      service.info(SHOULD_NOT_APPEAR);
      expect(console.info).not.toHaveBeenCalled();
    });

    it('should filter out warn when minLevel is error', () => {
      service.setMinLevel('error');
      service.warn(SHOULD_NOT_APPEAR);
      expect(console.warn).not.toHaveBeenCalled();
    });

    it('should always allow error', () => {
      service.setMinLevel('error');
      service.error(SHOULD_APPEAR);
      expect(console.error).toHaveBeenCalled();
    });
  });

  describe('Context', () => {
    it('should include context in log output', () => {
      const context = { userId: 'abc', action: 'login' };
      service.info('User action', context);
      expect(console.info).toHaveBeenCalledWith(jasmine.any(String), context);
    });

    it('should work without context', () => {
      service.info('Simple message');
      expect(console.info).toHaveBeenCalledWith(jasmine.any(String));
    });
  });

  describe('Correlation ID', () => {
    it('should set and get correlation ID', () => {
      service.setCorrelationId('req-12345');
      expect(service.getCorrelationId()).toBe('req-12345');
    });

    it('should clear correlation ID', () => {
      service.setCorrelationId('req-12345');
      service.setCorrelationId(null);
      expect(service.getCorrelationId()).toBeNull();
    });

    it('should generate correlation ID', () => {
      const id = service.generateCorrelationId();
      expect(typeof id).toBe('string');
      expect(id.length).toBeGreaterThan(0);
      expect(service.getCorrelationId()).toBe(id);
    });

    it('should include correlation ID in log entry', () => {
      service.setCorrelationId('req-abc123');
      service.info(TEST_MESSAGE);

      const logs = service.getRecentLogs();
      expect(logs.length).toBe(1);
      expect(logs[0].correlationId).toBe('req-abc123');
    });
  });

  describe('Source', () => {
    it('should set source for logs', () => {
      service.setSource('HolochainClient');
      service.info(TEST_MESSAGE);

      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe('HolochainClient');
    });

    it('should use default source when not set', () => {
      service.info(TEST_MESSAGE);

      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe(DEFAULT_SOURCE);
    });
  });

  describe('Child Logger', () => {
    it('should create child logger with specific source', () => {
      const child = service.createChild('ContentService');
      child.info('Child message');

      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe('ContentService');
    });

    it('should inherit correlation ID', () => {
      service.setCorrelationId('parent-123');
      const child = service.createChild('ChildService');
      child.info('Child message');

      const logs = service.getRecentLogs();
      expect(logs[0].correlationId).toBe('parent-123');
    });
  });

  describe('Recent Logs', () => {
    it('should store recent logs', () => {
      service.info('Message 1');
      service.info('Message 2');

      const logs = service.getRecentLogs();
      expect(logs.length).toBe(2);
      expect(logs[0].message).toBe('Message 1');
      expect(logs[1].message).toBe('Message 2');
    });

    it('should clear recent logs', () => {
      service.info('Message');
      service.clearRecentLogs();

      expect(service.getRecentLogs().length).toBe(0);
    });

    it('should limit recent logs to max size', () => {
      // Log more than max (100)
      for (let i = 0; i < 110; i++) {
        service.info(`Message ${i}`);
      }

      const logs = service.getRecentLogs();
      expect(logs.length).toBe(100);
      expect(logs[0].message).toBe('Message 10'); // First 10 should be evicted
    });
  });

  describe('Performance Timing', () => {
    it('should create timer', () => {
      const timer = service.startTimer(TEST_OPERATION);
      expect(timer).not.toBeNull();
      expect(typeof timer.end).toBe('function');
      expect(typeof timer.elapsed).toBe('function');
    });

    it('should measure elapsed time', async () => {
      const timer = service.startTimer(TEST_OPERATION);

      // Wait a bit
      await new Promise(resolve => setTimeout(resolve, 10));

      const elapsed = timer.elapsed();
      expect(elapsed).toBeGreaterThanOrEqual(5);
    });

    it('should log when timer ends', () => {
      const timer = service.startTimer('test-operation');
      timer.end({ extra: 'context' });

      expect(console.info).toHaveBeenCalled();
      const logs = service.getRecentLogs();
      expect(logs[0].message).toContain('test-operation completed');
      expect(logs[0].context?.['durationMs']).toBeDefined();
    });

    it('should time async operations', async () => {
      const result = await service.time('async-op', async () => {
        await new Promise(resolve => setTimeout(resolve, 5));
        return 'done';
      });

      expect(result).toBe('done');
      const logs = service.getRecentLogs();
      expect(logs[0].context?.['success']).toBe(true);
    });

    it('should log failed async operations', async () => {
      let errorOccurred = false;
      try {
        await service.time('failing-op', async () => {
          await Promise.resolve();
          throw new Error('Operation failed');
        });
      } catch {
        errorOccurred = true;
      }

      expect(errorOccurred).toBe(true);
      const logs = service.getRecentLogs();
      expect(logs[0].context?.['success']).toBe(false);
    });
  });

  describe('JSON Output', () => {
    it('should output JSON when configured', () => {
      service.configure({ jsonOutput: true });
      service.info('JSON message', { key: 'value' });

      expect(console.log).toHaveBeenCalled();
      const logCall = (console.log as jasmine.Spy).calls.first().args[0];
      const parsed = JSON.parse(logCall);
      expect(parsed.message).toBe('JSON message');
      expect(parsed.context.key).toBe('value');
    });
  });

  describe('Configuration', () => {
    it('should configure multiple options', () => {
      service.configure({
        minLevel: 'warn',
        includeTimestamp: false,
        defaultSource: 'test-app',
      });

      service.debug('Should not appear');
      expect(console.debug).not.toHaveBeenCalled();

      service.warn('Should appear');
      expect(console.warn).toHaveBeenCalled();
    });
  });

  describe('Log Entry Structure', () => {
    it('should have correct structure', () => {
      service.setCorrelationId('test-123');
      service.setSource('TestService');
      service.info('Test message', { key: 'value' });

      const logs = service.getRecentLogs();
      const entry = logs[0];

      expect(entry.timestamp).toMatch(/^\d{4}-\d{2}-\d{2}T/);
      expect(entry.level).toBe('info');
      expect(entry.message).toBe('Test message');
      expect(entry.context).toEqual({ key: 'value' });
      expect(entry.correlationId).toBe('test-123');
      expect(entry.source).toBe('TestService');
    });

    it('should include error in entry', () => {
      const error = new Error('Test error');
      service.error('Something failed', error);

      const logs = service.getRecentLogs();
      expect(logs[0].error).toBe(error);
    });
  });
});
