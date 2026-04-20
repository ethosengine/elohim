import { HttpClient } from '@angular/common/http';
import { Injectable, inject } from '@angular/core';
import { Router } from '@angular/router';

import { firstValueFrom, timeout, catchError, of } from 'rxjs';

import { LoggerService, type LogEntry } from './logger.service';

export interface DiagnosticBundle {
  logs: LogEntry[];
  environment: {
    platform: 'browser' | 'tauri';
    userAgent: string;
    appVersion: string;
    storageHealth: Record<string, unknown> | null;
  };
  context: {
    url: string;
    eprId: string | null;
    avodahProject: string | null;
    avodahStory: string | null;
  };
  correlationIds: string[];
  collectedAt: string;
}

@Injectable({ providedIn: 'root' })
export class DiagnosticCollectorService {
  private readonly logger = inject(LoggerService);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);

  async collect(): Promise<DiagnosticBundle> {
    const allLogs = this.logger.getRecentLogs();
    const logs = allLogs.filter(l => l.level === 'warn' || l.level === 'error');

    const correlationIds = [
      ...new Set(logs.map(l => l.correlationId).filter((id): id is string => id != null)),
    ];

    const isTauri =
      'window' in globalThis && '__TAURI__' in (globalThis as Record<string, unknown>);

    let storageHealth: Record<string, unknown> | null = null;
    try {
      storageHealth = await firstValueFrom(
        this.http.get<Record<string, unknown>>('/health').pipe(
          timeout(5000),
          catchError(() => of(null))
        )
      );
    } catch {
      storageHealth = null;
    }

    return {
      logs,
      environment: {
        platform: isTauri ? 'tauri' : 'browser',
        userAgent: navigator.userAgent,
        appVersion: '0.1.0',
        storageHealth,
      },
      context: {
        url: this.router.url,
        eprId: null,
        avodahProject: null,
        avodahStory: null,
      },
      correlationIds,
      collectedAt: new Date().toISOString(),
    };
  }
}
