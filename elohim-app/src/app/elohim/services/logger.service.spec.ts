import { TestBed } from '@angular/core/testing';
import { LoggerService, LogLevel, LogEntry } from './logger.service';

describe('LoggerService', () => {
  let service: LoggerService;

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

  it('should be created', () => {
    expect(service).toBeInstanceOf(LoggerService);
  });

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
      service.debug('Should not appear');
      expect(console.debug).not.toHaveBeenCalled();
    });

    it('should allow info when minLevel is info', () => {
      service.setMinLevel('info');
      service.info('Should appear');
      expect(console.info).toHaveBeenCalled();
    });

    it('should filter out info when minLevel is warn', () => {
      service.setMinLevel('warn');
      service.info('Should not appear');
      expect(console.info).not.toHaveBeenCalled();
    });

    it('should filter out warn when minLevel is error', () => {
      service.setMinLevel('error');
      service.warn('Should not appear');
      expect(console.warn).not.toHaveBeenCalled();
    });

    it('should always allow error', () => {
      service.setMinLevel('error');
      service.error('Should appear');
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
      service.info('Test message');

      const logs = service.getRecentLogs();
      expect(logs.length).toBe(1);
      expect(logs[0].correlationId).toBe('req-abc123');
    });
  });

  describe('Source', () => {
    it('should set source for logs', () => {
      service.setSource('HolochainClient');
      service.info('Test message');

      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe('HolochainClient');
    });

    it('should use default source when not set', () => {
      service.info('Test message');

      const logs = service.getRecentLogs();
      expect(logs[0].source).toBe('elohim-app');
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
      const timer = service.startTimer('test-operation');
      expect(timer).not.toBeNull();
      expect(typeof timer.end).toBe('function');
      expect(typeof timer.elapsed).toBe('function');
    });

    it('should measure elapsed time', async () => {
      const timer = service.startTimer('test-operation');

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
      try {
        await service.time('failing-op', async () => {
          throw new Error('Operation failed');
        });
      } catch {
        // Expected
      }

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
