import { TestBed } from '@angular/core/testing';
import { Router } from '@angular/router';
import { HttpClient } from '@angular/common/http';
import { of, throwError } from 'rxjs';
import { vi } from 'vitest';

import { LoggerService, type LogEntry } from './logger.service';
import { DiagnosticCollectorService, type DiagnosticBundle } from './diagnostic-collector.service';

describe('DiagnosticCollectorService', () => {
  let service: DiagnosticCollectorService;
  let loggerMock: { getRecentLogs: ReturnType<typeof vi.fn> };
  let routerMock: { url: string };
  let httpMock: { get: ReturnType<typeof vi.fn> };

  beforeEach(() => {
    loggerMock = {
      getRecentLogs: vi.fn().mockReturnValue([]),
    };
    routerMock = { url: '/learn/path/123/node/456' };
    httpMock = {
      get: vi.fn().mockReturnValue(of({ status: 'ok', blobs: 10, bytes: 1024 })),
    };

    TestBed.configureTestingModule({
      providers: [
        DiagnosticCollectorService,
        { provide: LoggerService, useValue: loggerMock },
        { provide: Router, useValue: routerMock },
        { provide: HttpClient, useValue: httpMock },
      ],
    });

    service = TestBed.inject(DiagnosticCollectorService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should include current route in context', async () => {
    const bundle = await service.collect();
    expect(bundle.context.url).toBe('/learn/path/123/node/456');
  });

  it('should include logs from LoggerService', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'error', message: 'test error' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.logs).toEqual(mockLogs);
  });

  it('should filter logs to warn and error levels', async () => {
    const mockLogs: LogEntry[] = [
      { timestamp: '2026-03-15T10:00:00Z', level: 'debug', message: 'noise' },
      { timestamp: '2026-03-15T10:00:01Z', level: 'info', message: 'info' },
      { timestamp: '2026-03-15T10:00:02Z', level: 'warn', message: 'warning' },
      { timestamp: '2026-03-15T10:00:03Z', level: 'error', message: 'error' },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.logs.length).toBe(2);
    expect(bundle.logs[0].level).toBe('warn');
    expect(bundle.logs[1].level).toBe('error');
  });

  it('should include environment info', async () => {
    const bundle = await service.collect();
    expect(bundle.environment.platform).toBeDefined();
    expect(bundle.environment.userAgent).toBeDefined();
  });

  it('should fetch health snapshot', async () => {
    const bundle = await service.collect();
    expect(httpMock.get).toHaveBeenCalled();
    expect(bundle.environment.storageHealth).toEqual({ status: 'ok', blobs: 10, bytes: 1024 });
  });

  it('should handle health fetch failure gracefully', async () => {
    httpMock.get.mockReturnValue(throwError(() => new Error('network error')));

    const bundle = await service.collect();
    expect(bundle.environment.storageHealth).toBeNull();
  });

  it('should extract unique correlation IDs from logs', async () => {
    const mockLogs: LogEntry[] = [
      {
        timestamp: '2026-03-15T10:00:00Z',
        level: 'error',
        message: 'fail',
        correlationId: 'corr-1',
      },
      {
        timestamp: '2026-03-15T10:00:01Z',
        level: 'error',
        message: 'fail2',
        correlationId: 'corr-1',
      },
      {
        timestamp: '2026-03-15T10:00:02Z',
        level: 'warn',
        message: 'warn',
        correlationId: 'corr-2',
      },
    ];
    loggerMock.getRecentLogs.mockReturnValue(mockLogs);

    const bundle = await service.collect();
    expect(bundle.correlationIds).toEqual(['corr-1', 'corr-2']);
  });

  it('should include collectedAt timestamp', async () => {
    const bundle = await service.collect();
    expect(bundle.collectedAt).toBeDefined();
    expect(new Date(bundle.collectedAt).getTime()).toBeGreaterThan(0);
  });
});
